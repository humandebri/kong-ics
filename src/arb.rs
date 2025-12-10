// どこで: アービトラージ計算と実行の中心ロジック
// 何を: プール情報のキャッシュ、計算、スワップ実行、通知
// なぜ: 上位(main)から見たときに単一目的で扱えるようにするため

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::PairConfig;
use crate::ic_client::agent::IcClient;
use crate::ic_client::ics::{fetch_pool_snapshot as fetch_ics, IcsPoolSnapshot};
use crate::ic_client::kong::{fetch_pool_snapshot as fetch_kong, KongPoolSnapshot};
use crate::ic_client::swap::{swap_icps, swap_kong};
use crate::notify::DiscordNotifier;

#[derive(Debug)]
pub enum TradeError {
    Client(String),
    Logic(String),
}

pub struct Trade {
    config: PairConfig,
    client: Arc<IcClient>,
    kong_cache: RwLock<Option<KongPoolSnapshot>>,
    ics_cache: RwLock<Option<IcsPoolSnapshot>>,
    notifier: Option<DiscordNotifier>,
    fee_rate: f64,
    min_receive_factor: f64,
    profit_threshold_e8: f64,
    loop_interval_ms: u64,
}

impl Trade {
    pub fn new(
        config: PairConfig,
        client: Arc<IcClient>,
        notifier: Option<DiscordNotifier>,
        fee_rate: f64,
        min_receive_factor: f64,
        profit_threshold_e8: f64,
        loop_interval_ms: u64,
    ) -> Self {
        Trade {
            config,
            client,
            kong_cache: RwLock::new(None),
            ics_cache: RwLock::new(None),
            notifier,
            fee_rate,
            min_receive_factor,
            profit_threshold_e8,
            loop_interval_ms,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.config.symbol
    }

    pub fn loop_interval_ms(&self) -> u64 {
        self.loop_interval_ms
    }

    pub async fn update_kong_cache(&self) {
        match fetch_kong(
            &self.client,
            &self.config.kong_canister,
            &self.config.symbol,
        )
        .await
        {
            Ok(snapshot) => {
                let mut guard = self.kong_cache.write().await;
                *guard = Some(snapshot);
            }
            Err(e) => {
                warn!("{}: Kong 更新失敗: {}", self.config.symbol, e);
            }
        }
    }

    pub async fn update_ics_cache(&self) {
        match fetch_ics(&self.client, &self.config.icpswap_lp).await {
            Ok(snapshot) => {
                let mut guard = self.ics_cache.write().await;
                *guard = Some(snapshot);
            }
            Err(e) => {
                warn!("{}: ICS 更新失敗: {}", self.config.symbol, e);
            }
        }
    }

    pub async fn tick(&self) -> Result<(), TradeError> {
        // Kong/ICS を並列更新
        tokio::join!(self.update_kong_cache(), self.update_ics_cache());

        let kong_cache = { self.kong_cache.read().await.clone() };
        let ics_cache = { self.ics_cache.read().await.clone() };
        if kong_cache.is_none() || ics_cache.is_none() {
            return Ok(());
        }

        let kong = kong_cache.expect("checked above");
        let ics = ics_cache.expect("checked above");
        let ikiti = self.config.ikiti_e8 as f64;

        let result = cal_amount(
            kong.icp_balance,
            kong.sns_balance,
            ics.token0_k,
            ics.token1_k,
        );
        let mut result_abs = result.abs();
        if result_abs > ikiti {
            result_abs = ikiti;
        }

        let fee = self.fee_rate;
        let (kekka, direction, output_a, output_b) = if result < 0f64 {
            let output_icpswap = swap_icp_to_ckusdc(result_abs, ics.token1_k, ics.token0_k, fee);
            let output_kong =
                swap_icp_to_ckusdc(output_icpswap, kong.sns_balance, kong.icp_balance, fee);
            let delta = output_kong - result_abs;
            (delta, SwapDirection::IcsToKong, output_icpswap, output_kong)
        } else {
            let output_kong =
                swap_icp_to_ckusdc(result_abs, kong.icp_balance, kong.sns_balance, fee);
            let output_icpswap = swap_icp_to_ckusdc(output_kong, ics.token0_k, ics.token1_k, fee);
            let delta = output_icpswap - result_abs;
            (delta, SwapDirection::KongToIcs, output_kong, output_icpswap)
        };

        info!(
            "{}: check result_abs {:.4} ICP, output_a {:.4} ICP, output_b {:.4} ICP, profit {:.4} ICP, dir={:?}",
            self.config.symbol,
            result_abs / 1e8f64,
            output_a / 1e8f64,
            output_b / 1e8f64,
            kekka / 1e8f64,
            direction
        );

        if kekka > self.profit_threshold_e8 {
            info!(
                "{}: 利益見込み {:.4} ICP (dir={:?})",
                self.config.symbol,
                kekka / 1e8f64,
                direction
            );
            self.execute_swaps(result_abs, output_a, output_b, direction)
                .await?;
        } else {
            info!(
                "{}: 利益しきい値未達 (profit {:.4} ICP, threshold {:.4} ICP)",
                self.config.symbol,
                kekka / 1e8f64,
                self.profit_threshold_e8 / 1e8f64
            );
        }

        Ok(())
    }

    async fn execute_swaps(
        &self,
        amount_in: f64,
        mid_amount: f64,
        final_amount: f64,
        direction: SwapDirection,
    ) -> Result<(), TradeError> {
        // min_receive_factor を最低受取に設定
        let min_mid = (mid_amount * self.min_receive_factor).round() as u128;
        let min_final = (final_amount * self.min_receive_factor).round() as u128;
        let amount_in_u = amount_in.round() as u128;

        match direction {
            SwapDirection::IcsToKong => {
                let ics_call = swap_icps(
                    &self.client,
                    &self.config.icpswap_lp,
                    amount_in_u,
                    min_mid,
                    false,
                );
                let kong_call = swap_kong(
                    &self.client,
                    &self.config.kong_canister,
                    &self.config.token_sns,
                    &self.config.token_icp,
                    min_mid,
                    min_final,
                );
                let (ics_res, kong_res) = tokio::join!(ics_call, kong_call);
                if let Err(e) = ics_res {
                    return Err(TradeError::Client(format!("swap_icps: {}", e)));
                }
                if let Err(e) = kong_res {
                    return Err(TradeError::Client(format!("swap_kong: {}", e)));
                }
            }
            SwapDirection::KongToIcs => {
                let kong_call = swap_kong(
                    &self.client,
                    &self.config.kong_canister,
                    &self.config.token_icp,
                    &self.config.token_sns,
                    amount_in_u,
                    min_mid,
                );
                let ics_call = swap_icps(
                    &self.client,
                    &self.config.icpswap_lp,
                    min_mid,
                    min_final,
                    true,
                );
                let (kong_res, ics_res) = tokio::join!(kong_call, ics_call);
                if let Err(e) = kong_res {
                    return Err(TradeError::Client(format!("swap_kong: {}", e)));
                }
                if let Err(e) = ics_res {
                    return Err(TradeError::Client(format!("swap_icps: {}", e)));
                }
            }
        }

        if let Some(notifier) = &self.notifier {
            let dir_text = match direction {
                SwapDirection::IcsToKong => "ics→kong",
                SwapDirection::KongToIcs => "kong→ics",
            };
            let message = format!(
                "{} が {} で swap 実行。in {:.4} / out {:.4}",
                self.config.symbol,
                dir_text,
                amount_in / 1e8f64,
                final_amount / 1e8f64
            );
            if let Err(e) = notifier.notify(&message).await {
                warn!("LINE 通知失敗: {}", e);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum SwapDirection {
    IcsToKong,
    KongToIcs,
}

// --- 計算ロジック ---

pub fn swap_icp_to_ckusdc(amount: f64, token0: f64, token1: f64, fee_rate: f64) -> f64 {
    let numerator = token1 / (amount + token0);
    (numerator * amount) * (1f64 - fee_rate)
}

pub fn cal_amount(dex1_token0: f64, dex1_token1: f64, dex2_token0: f64, dex2_token1: f64) -> f64 {
    let a2 = dex1_token0 * 10_000f64;
    let a3 = 10_000f64 - 30f64;
    let a1 = a3 * dex1_token1;

    let b2 = dex2_token0 * 10_000f64;
    let b3 = 10_000f64 - 30f64;
    let b1 = b3 * dex2_token1;

    let a = a1 * a1 * b3 * b3 + 2f64 * a1 * a3 * b2 * b3 + a3 * a3 * b2 * b2;
    let b = 2f64 * a1 * a2 * b2 * b3 + 2f64 * a2 * a3 * b2 * b2;
    let c = a2 * a2 * b2 * b2 - a1 * a2 * b1 * b2;

    let sqrt = (b * b - 4f64 * a * c).sqrt();
    (-b + sqrt) / (2f64 * a)
}
