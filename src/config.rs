// どこで: Rust 化した kong-ics バイナリの設定モジュール
// 何を: 各種エンドポイント・鍵パス・取引ペア設定を集中管理
// なぜ: マジックナンバーを避け、環境変更を安全に行うため

use serde::{Deserialize, Serialize};
use std::env;

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
pub struct PairConfig {
    pub token_icp: String,
    pub token_sns: String,
    pub kong_canister: String,
    pub icpswap_lp: String,
    pub symbol: String,
    pub ikiti_e8: u128,
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
    pub trade: TradeParams,
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
            trade: TradeParams {
                fee_rate,
                min_receive_factor,
                profit_threshold_e8,
                loop_interval_ms,
            },
            pairs: vec![
                PairConfig {
                    token_icp: "IC.ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    token_sns: "IC.7pail-xaaaa-aaaas-aabmq-cai".to_string(),
                    kong_canister: "2ipq2-uqaaa-aaaar-qailq-cai".to_string(),
                    icpswap_lp: "ybilh-nqaaa-aaaag-qkhzq-cai".to_string(),
                    symbol: "BOB_ICP".to_string(),
                    ikiti_e8: 30_000_000_000,
                },
                PairConfig {
                    token_icp: "IC.ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    token_sns: "IC.2ouva-viaaa-aaaaq-aaamq-cai".to_string(),
                    kong_canister: "2ipq2-uqaaa-aaaar-qailq-cai".to_string(),
                    icpswap_lp: "ne2vj-6yaaa-aaaag-qb3ia-cai".to_string(),
                    symbol: "CHAT_ICP".to_string(),
                    ikiti_e8: 50_000_000_000,
                },
                PairConfig {
                    token_icp: "IC.ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    token_sns: "IC.o7oak-iyaaa-aaaaq-aadzq-cai".to_string(),
                    kong_canister: "2ipq2-uqaaa-aaaar-qailq-cai".to_string(),
                    icpswap_lp: "ye4fx-gqaaa-aaaag-qnara-cai".to_string(),
                    symbol: "KONG_ICP".to_string(),
                    ikiti_e8: 60_000_000_000,
                },
                PairConfig {
                    token_icp: "IC.ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    token_sns: "IC.jcmow-hyaaa-aaaaq-aadlq-cai".to_string(),
                    kong_canister: "2ipq2-uqaaa-aaaar-qailq-cai".to_string(),
                    icpswap_lp: "oqn67-kaaaa-aaaag-qj72q-cai".to_string(),
                    symbol: "WTN_ICP".to_string(),
                    ikiti_e8: 30_000_000_000,
                },
                PairConfig {
                    token_icp: "IC.ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    token_sns: "IC.buwm7-7yaaa-aaaar-qagva-cai".to_string(),
                    kong_canister: "2ipq2-uqaaa-aaaar-qailq-cai".to_string(),
                    icpswap_lp: "e5a7x-pqaaa-aaaag-qkcga-cai".to_string(),
                    symbol: "nICP_ICP".to_string(),
                    ikiti_e8: 40_000_000_000,
                },
            ],
        }
    }
}
