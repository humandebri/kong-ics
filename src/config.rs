// どこで: Rust 化した kong-ics バイナリの設定モジュール
// 何を: 各種エンドポイント・鍵パス・取引ペア設定を集中管理
// なぜ: マジックナンバーを避け、環境変更を安全に行うため

use serde::{Deserialize, Serialize};
use std::env;

pub const ICP_LEDGER_RAW: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const ICP_LEDGER_IC: &str = "IC.ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const KONG_CANISTER: &str = "2ipq2-uqaaa-aaaar-qailq-cai";
pub const ICP_TRANSFER_FEE_E8: u128 = 10_000;

fn e8(amount: f64) -> u128 {
    // 小数を許容しつつ e8 精度に丸める
    (amount * 1e8f64).round() as u128
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub api_url: String,
    pub fetch_root_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub pem_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineNotifyConfig {
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWebhookConfig {
    /// .env に定義するキー名（例: DISCORD_WEBHOOK_URL）
    pub env_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDefinition {
    pub name: String,
    pub icpswap_lp: String,
    pub sns_canister: String,
    pub transfer_fee_e8: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveTokenConfig {
    pub name: String,
    pub icpswap: String,
    pub sns: String,
    pub sns_threshold_e8: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveConfig {
    pub tokens: Vec<ApproveTokenConfig>,
    pub icp_canister: String,
    pub kong_canister: String,
    pub icp_amount_e8: u128,
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairConfig {
    pub token_icp: String,
    pub token_sns: String,
    pub kong_canister: String,
    pub icpswap_lp: String,
    pub symbol: String,
    pub ikiti_e8: u128,
    pub sns_fee_e8: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeParams {
    /// fee_rate: 0.003 なら 0.3%
    pub fee_rate: f64,
    /// 最低受取に掛ける係数（例: 0.99）
    pub min_receive_factor: f64,
    /// 利益判定しきい値（e8 単位）
    pub profit_threshold_e8: f64,
    /// ループ間隔 (ms)
    pub loop_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub network: NetworkConfig,
    pub identity: IdentityConfig,
    pub line: LineNotifyConfig,
    pub discord: DiscordWebhookConfig,
    pub tokens: Vec<TokenDefinition>,
    pub trade: TradeParams,
    pub approve: ApproveConfig,
    pub pairs: Vec<PairConfig>,
}

impl AppConfig {
    pub fn load_default() -> Self {
        // 環境変数を使って上書きできるようにする（指定がなければデフォルト）
        let fee_rate = env::var("TRADE_FEE_RATE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.003);
        let min_receive_factor = env::var("TRADE_MIN_RECEIVE_FACTOR")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.99);
        let profit_threshold_e8 = env::var("TRADE_PROFIT_THRESHOLD_E8")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.1e8);
        let loop_interval_ms = env::var("TRADE_LOOP_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(200);
        let discord_env_key = env::var("DISCORD_WEBHOOK_ENV_KEY")
            .ok()
            .unwrap_or_else(|| "DISCORD_WEBHOOK_URL".to_string());

        // トークン定義を一元化
        let tokens = vec![
            TokenDefinition {
                name: "bob".to_string(),
                icpswap_lp: "ybilh-nqaaa-aaaag-qkhzq-cai".to_string(),
                sns_canister: "7pail-xaaaa-aaaas-aabmq-cai".to_string(),
                transfer_fee_e8: 1_000_000,
            },
            TokenDefinition {
                name: "kong".to_string(),
                icpswap_lp: "ye4fx-gqaaa-aaaag-qnara-cai".to_string(),
                sns_canister: "o7oak-iyaaa-aaaaq-aadzq-cai".to_string(),
                transfer_fee_e8: 10_000,
            },
            TokenDefinition {
                name: "nicp".to_string(),
                icpswap_lp: "e5a7x-pqaaa-aaaag-qkcga-cai".to_string(),
                sns_canister: "buwm7-7yaaa-aaaar-qagva-cai".to_string(),
                transfer_fee_e8: 1_000_000,
            },
            TokenDefinition {
                name: "usdc".to_string(),
                icpswap_lp: "mohjv-bqaaa-aaaag-qjyia-cai".to_string(),
                sns_canister: "xevnm-gaaaa-aaaar-qafnq-cai".to_string(),
                transfer_fee_e8: 10_000,
            },
            TokenDefinition {
                name: "usdt".to_string(),
                icpswap_lp: "hkstf-6iaaa-aaaag-qkcoq-cai".to_string(),
                sns_canister: "cngnf-vqaaa-aaaar-qag4q-cai".to_string(),
                transfer_fee_e8: 10_000,
            },
            TokenDefinition {
                name: "dkp".to_string(),
                icpswap_lp: "ijd5l-jyaaa-aaaag-qdjga-cai".to_string(),
                sns_canister: "zfcdd-tqaaa-aaaaq-aaaga-cai".to_string(),
                transfer_fee_e8: 100_000,
            },
            TokenDefinition {
                name: "exe".to_string(),
                icpswap_lp: "dlfvj-eqaaa-aaaag-qcs3a-cai".to_string(),
                sns_canister: "rh2pm-ryaaa-aaaan-qeniq-cai".to_string(),
                transfer_fee_e8: 100_000,
            },
            TokenDefinition {
                name: "panda".to_string(),
                icpswap_lp: "5fq4w-lyaaa-aaaag-qjqta-cai".to_string(),
                sns_canister: "druyg-tyaaa-aaaaq-aactq-cai".to_string(),
                transfer_fee_e8: 10_000,
            },
        ];

        let find_token =
            |name: &str| -> Option<&TokenDefinition> { tokens.iter().find(|t| t.name == name) };

        // 承認用しきい値
        let approve_specs = vec![("kong", e8(10_000.0)), ("bob", e8(100.1))];

        // アービトラージ対象ペア（symbol, token_name, ikiti）
        let pair_specs = vec![("BOB_ICP", "bob", e8(10.0)), ("KONG_ICP", "kong", e8(10.0))];

        let approve_tokens: Vec<ApproveTokenConfig> = approve_specs
            .iter()
            .filter_map(|(name, threshold)| {
                find_token(name).map(|t| ApproveTokenConfig {
                    name: t.name.clone(),
                    icpswap: t.icpswap_lp.clone(),
                    sns: t.sns_canister.clone(),
                    sns_threshold_e8: *threshold,
                })
            })
            .collect();

        let pairs: Vec<PairConfig> = pair_specs
            .iter()
            .filter_map(|(symbol, token_name, ikiti)| {
                find_token(token_name).map(|t| PairConfig {
                    token_icp: ICP_LEDGER_IC.to_string(),
                    token_sns: t.sns_canister.clone(),
                    kong_canister: KONG_CANISTER.to_string(),
                    icpswap_lp: t.icpswap_lp.clone(),
                    symbol: symbol.to_string(),
                    ikiti_e8: *ikiti,
                    sns_fee_e8: t.transfer_fee_e8,
                })
            })
            .collect();

        AppConfig {
            network: NetworkConfig {
                api_url: "https://icp-api.io".to_string(),
                fetch_root_key: false,
            },
            identity: IdentityConfig {
                pem_path: "infinity_identity.pem".to_string(),
            },
            line: LineNotifyConfig { token: None },
            discord: DiscordWebhookConfig {
                env_key: discord_env_key,
            },
            tokens,
            trade: TradeParams {
                fee_rate,
                min_receive_factor,
                profit_threshold_e8,
                loop_interval_ms,
            },
            approve: ApproveConfig {
                tokens: approve_tokens,
                icp_canister: ICP_LEDGER_RAW.to_string(),
                kong_canister: KONG_CANISTER.to_string(),
                icp_amount_e8: e8(10.0),
                interval_secs: 100,
            },
            pairs,
        }
    }
}
