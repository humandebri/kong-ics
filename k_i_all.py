import asyncio
import time
import numpy as np
import requests
from functools import lru_cache

# ===== IC関連の import =====
from ic.identity import Identity
from ic.client import Client
from ic.agent import Agent
from ic.candid import encode, Types

def tuti(message):
    payload = {'message': message}
    headers = {'Authorization': 'Bearer ' + 'TyCDlSWL9A9wgsGywpAAIkUqxgmGCE9PcbTjpZzWO0j'}
    requests.post('https://notify-api.line.me/api/notify', data=payload, headers=headers)
    return


@lru_cache(maxsize=None)
def swap_icp_to_ckusdc(result, token0, token1, fee_rate=0.003):
    return int((token1/(result + token0)) * result * (1 - fee_rate))

@lru_cache(maxsize=None)
def cal_amount(dex1_token0, dex1_token1, dex2_token0, dex2_token1):
    a2 = np.float64(dex1_token0) * 10000
    a3 = 10000 - 30
    a1 = a3 * np.float64(dex1_token1)

    b2 = np.float64(dex2_token0) * 10000
    b3 = 10000 - 30
    b1 = b3 * np.float64(dex2_token1)

    A = a1*a1*b3*b3 + 2*a1*a3*b2*b3 + a3*a3*b2*b2
    B = 2*a1*a2*b2*b3 + 2*a2*a3*b2*b2
    C = a2*a2*b2*b2 - a1*a2*b1*b2

    result = (-B + np.sqrt(B * B - 4*A*C)) / (2*A)
    return result

async def get_icsinfo(agent, canistar):
    """
    ICPSwapのcanisterから pool metadata を取得し、
    (ics_lp_token0_k, ics_lp_token1_k) を返す
    """
    respo = await agent.query_raw_async(canistar, "metadata", encode([]))
    sqrtPriceX96 = respo[0]["value"]["_24860"]["_1161096524"]
    L = respo[0]["value"]["_24860"]["_1304432370"]
    price_icsk = (sqrtPriceX96 ** 2) / ((2 ** 96) ** 2)

    ics_lp_token0_k = L / np.sqrt(price_icsk)  # sneed
    ics_lp_token1_k = L * np.sqrt(price_icsk)  # icp
    return ics_lp_token0_k, ics_lp_token1_k

async def get_konginfo(agent, kong, tiker):
    """
    Kongのcanisterからプールデータを取得し、
    (ICP残高, bob残高) = (balance_1, balance_0) を返す
    """
    params = [{'type': Types.Opt(Types.Text), 'value': [tiker]}]
    pool = await agent.query_raw_async(kong, 'pools', encode(params))
    entry = pool[0]['value']['_17724']['_3331604503'][0]
    if entry['_4007505752'] == tiker:
        balance_0 = float(entry['_1476685581'])  # bob
        balance_1 = float(entry['_1476685582'])  # icp
    else:
        print("poolがありません")
        balance_0 = 0.0
        balance_1 = 0.0

    return balance_1, balance_0  # (ICP, bob)

class Trade:
    def __init__(
        self,
        token_icp: str,
        token_sns: str,
        kong_canister: str,
        icpswap_lp: str,
        symbol: str,
        ikiti: float
    ):
        """
        異なるペアをトレードしたい場合、上記パラメータを変えて
        本クラスをインスタンス化すればOK。
        """
        # 従来どおりAgentを作成
        ano = Identity()
        client = Client(url="https://icp-api.io")
        self.agent = Agent(ano, client)

        self.myprincipal = "xxol4-pnhff-brpzi-lxpfe-dy4hs-d2kzd-mif4k-aatnp-ltnhs-ydkcn-qqe"

        # キャッシュ変数
        self.kong_cache = None  # (R_ICP, R_snstoken)
        self.ics_cache = None   # (ics_lp_token0_k, ics_lp_token1_k)

        # コンストラクタ引数を保存
        self.token_icp = token_icp
        self.token_sns = token_sns
        self.kong_canister = kong_canister
        self.icpswap_lp = icpswap_lp
        self.symbol = symbol
        self.ikiti = ikiti

    async def update_konginfo_cache(self):
        """
        Kongのcanisterに問い合わせ、最新データをキャッシュに入れる。
        """
        try:
            R_ICP, R_snstoken = await get_konginfo(self.agent, self.kong_canister, self.symbol)
            self.kong_cache = (R_ICP, R_snstoken)
        except Exception as e:
            print(self.symbol,"update_konginfo_cache 失敗:", e)

    async def update_icsinfo_cache(self):
        """
        ICSのcanisterに問い合わせ、最新データをキャッシュに入れる。
        """
        try:
            ics_lp_token0_k, ics_lp_token1_k = await get_icsinfo(self.agent, self.icpswap_lp)
            self.ics_cache = (ics_lp_token0_k, ics_lp_token1_k)
        except Exception as e:
            print("update_icsinfo_cache 失敗:", e)

    async def check(self):
        """
        アービトラージの計算を行い、条件があえばswapを実行する。
        ただし「Kongがまだ更新されていない」場合は前回のキャッシュに頼る。
        """
        start = time.time()

        # 1) ICS情報は毎回更新
        await self.update_icsinfo_cache()
        if not self.ics_cache:
            print(self.symbol,"ICSのキャッシュがありません。スキップします。")
            return

        # 2) Kongのキャッシュがない場合はスキップ
        if not self.kong_cache:
            print(self.symbol,"Kongのキャッシュがありません。今回の計算はスキップします。")
            return

        R_ICP, R_snstoken = self.kong_cache
        ics_lp_token0_k, ics_lp_token1_k = self.ics_cache
        ikiti = self.ikiti

        # 3) アービトラージ計算
        result = cal_amount(R_ICP, R_snstoken, ics_lp_token0_k, ics_lp_token1_k)
        result_abs = abs(result)
        if result_abs > ikiti:
            result_abs = ikiti

        if result < 0:
            output_icpswap = swap_icp_to_ckusdc(result_abs, ics_lp_token1_k, ics_lp_token0_k)
            output_kong = swap_icp_to_ckusdc(output_icpswap, R_snstoken, R_ICP, fee_rate=0.003)
            kekka = output_kong - result_abs
            flag = 1
        else:
            output_kong = swap_icp_to_ckusdc(result_abs, R_ICP, R_snstoken, fee_rate=0.003)
            output_icpswap = swap_icp_to_ckusdc(output_kong, ics_lp_token0_k, ics_lp_token1_k)
            kekka = output_icpswap - result_abs
            flag = 2

        # 4) スワップが利益になるか判定
        if kekka > 0.1e8:
            if flag == 1:
                print("icpswap→kong 利益が出ました", kekka / 1e8)
                ics, son = await asyncio.gather(
                    self.swap_icps(result_abs, output_icpswap, False, result_abs, ikiti, kekka, self.icpswap_lp),
                    self.swap_kong(self.token_sns, self.token_icp, int(output_icpswap), int(output_kong),
                                   result_abs, ikiti, kekka),
                    return_exceptions=True
                )

                print(result_abs / 1e8, output_icpswap / 1e8, output_kong / 1e8, kekka / 1e8, flag)

                if isinstance(ics, Exception):
                    print("swap_icpsでエラー:", ics)
                if isinstance(son, Exception):
                    print("swap_kongでエラー:", son)

                message = f'{self.symbol}::kong_icsがswapしました！ics→inf{round(result/1e8,3)}を{round(output_kong/1e8,3)}にswapして{round(kekka/1e8,4)}ICP増えました!!（想像）'
                tuti(message)

            else:
                print("kong→icpswap 利益が出ました", kekka / 1e8)
                ics, son = await asyncio.gather(
                    self.swap_kong(self.token_icp, self.token_sns, int(result_abs), int(output_kong),
                                   result_abs, ikiti, kekka),
                    self.swap_icps(int(output_kong), int(output_icpswap), True, result_abs,
                                   ikiti, kekka, self.icpswap_lp),
                    return_exceptions=True
                )

                print(result_abs / 1e8, output_kong / 1e8, output_icpswap / 1e8, kekka / 1e8, flag)

                if isinstance(ics, Exception):
                    print("swap_kongでエラー:", ics)
                if isinstance(son, Exception):
                    print("swap_icpsでエラー:", son)

                message = f'{self.symbol}::kong_icsがswapしました！ics→inf{round(result/1e8,3)}を{round(output_icpswap/1e8,3)}にswapして{round(kekka/1e8,4)}ICP増えました!!（想像）'
                tuti(message)

        end = time.time()
        #print("check()所要時間:", end - start, "秒")

    async def swap_kong(self, token1, token2, amount, amount2, result, ikiti, kekka):
        types_s = Types.Record({
            'receive_token': Types.Text,
            'pay_amount': Types.Nat,
            'receive_amount': Types.Opt(Types.Nat),
            'pay_token': Types.Text,
        })

        values_s = {
            'receive_token': token2,
            'pay_amount': amount,
            'receive_amount': [int(amount2*0.99)],
            'pay_token': token1,
        }

        params = [{'type': types_s, 'value': values_s}]

        with open('infinity_identity.pem','r') as f:
            private_key_1 = f.read()
        plug = Identity.from_pem(private_key_1)
        client = Client(url="https://ic0.app")
        agent = Agent(plug, client)

        kong_canister = self.kong_canister
        respo  = await agent.update_raw_async(kong_canister, "swap_async", encode(params))
        print("swap_kong : ", amount, amount2, result)

        if "Slippage exceeded" in str(respo):
            print("slipage over")
            await asyncio.sleep(3)
        elif "enough funds" in str(respo):
            print("資金不足_bob_kong")
            await asyncio.sleep(3)
        else:
            value = respo[0]["value"]["_17724"]
            print("swap_kong : ", value)
            return value
        return

    async def swap_icps(self, amount, amount2, flag, result, ikiti, kekka, icpswap_lp):
        types_s = Types.Record({
            'amountIn': Types.Text,
            'zeroForOne': Types.Bool,
            'amountOutMinimum': Types.Text
        })

        values_s = {
            'amountIn': str(amount),
            'zeroForOne': flag,
            'amountOutMinimum': str(int(amount2*0.99))
        }

        params_s = [{'type': types_s, 'value': values_s}]

        with open('infinity_identity.pem','r') as f:
            private_key_1 = f.read()
        plug = Identity.from_pem(private_key_1)
        client = Client(url="https://ic0.app")
        agent = Agent(plug, client)

        respo_s = await agent.update_raw_async(icpswap_lp, "swap", encode(params_s))

        print("swap_ics:", amount, amount2, flag, result)
        print("swap_ics response:", respo_s)
        return respo_s


# --------------------------------------------------
# メインのイベントループ (ペアごとにTradeを動かす)
# --------------------------------------------------

async def main_loop(tr: Trade):
    """
    既存のwhileループ処理を、Tradeのインスタンス単位で実行する関数。
    """
    while True:
        try:
            # 1) Kong情報更新をバックグラウンドで実行 (遅い)
            kong_task = asyncio.create_task(tr.update_konginfo_cache())

            # 2) メインのチェック (ICSはcheck内部で更新される)
            await tr.check()

            # ここでは kong_task を待たずに次へ進む
            # (次回ループ時までにKongキャッシュが更新されていればOK)
            # もし「チェック完了後すぐに最新を使いたい」なら await kong_task する

            # インターバル（必要なら調整）
            # await asyncio.sleep(1)

        except Exception as e:
            print("error:", e)
            message = f'{tr.symbol}が止まりました！！！！ -> {e}'
            tuti(message)
            await asyncio.sleep(1)


async def run_all_pairs():
    """
    複数ペアを一括で並列実行するためのメイン関数。
    """
    trade_pairs = [
        {
            "token_icp": "IC.ryjl3-tyaaa-aaaaa-aaaba-cai",
            "token_sns": "IC.7pail-xaaaa-aaaas-aabmq-cai",
            "kong_canister": "2ipq2-uqaaa-aaaar-qailq-cai",
            "icpswap_lp": "ybilh-nqaaa-aaaag-qkhzq-cai",
            "symbol": "BOB_ICP",
            "ikiti": 30e8
        },
        {
            "token_icp" : "IC.ryjl3-tyaaa-aaaaa-aaaba-cai",
            "token_sns" : "IC.2ouva-viaaa-aaaaq-aaamq-cai",
            "kong_canister": "2ipq2-uqaaa-aaaar-qailq-cai",
            "icpswap_lp" : "ne2vj-6yaaa-aaaag-qb3ia-cai",
            "symbol" : "CHAT_ICP",
            "ikiti" : 50E8
        },
        {
            "token_icp" : "IC.ryjl3-tyaaa-aaaaa-aaaba-cai",
            "token_sns" : "IC.o7oak-iyaaa-aaaaq-aadzq-cai",
            "kong_canister" : "2ipq2-uqaaa-aaaar-qailq-cai",
            "icpswap_lp" : "ye4fx-gqaaa-aaaag-qnara-cai",
            "symbol" : "KONG_ICP",
            "ikiti" : 60E8
        },
        {
            "token_icp" : "IC.ryjl3-tyaaa-aaaaa-aaaba-cai",
            "token_sns" : "IC.jcmow-hyaaa-aaaaq-aadlq-cai",
            "kong_canister" : "2ipq2-uqaaa-aaaar-qailq-cai",
            "icpswap_lp" : "oqn67-kaaaa-aaaag-qj72q-cai",
            "symbol" : "WTN_ICP",
            "ikiti" : 30E8
        },
        {
            "token_icp" : "IC.ryjl3-tyaaa-aaaaa-aaaba-cai",
            "token_sns" : "IC.buwm7-7yaaa-aaaar-qagva-cai",
            "kong_canister" : "2ipq2-uqaaa-aaaar-qailq-cai",
            "icpswap_lp" : "e5a7x-pqaaa-aaaag-qkcga-cai",
            "symbol" : "nICP_ICP",
            "ikiti" : 40E8
        }
    ]

    # インスタンスを作成して並列処理する
    tasks = []
    for pair_conf in trade_pairs:
        tr = Trade(**pair_conf)
        tasks.append(asyncio.create_task(main_loop(tr)))

    # 並列実行
    await asyncio.gather(*tasks)

if __name__ == "__main__":
    asyncio.run(run_all_pairs())

