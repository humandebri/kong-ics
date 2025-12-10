// どこで: スワップ系 update 呼び出し
// 何を: Kong の swap_async と ICPSwap の swap を叩く
// なぜ: 取引実行を Rust から完結させるため

use candid::Encode;
use serde::Serialize;
use thiserror::Error;

use super::agent::IcClient;

#[derive(Debug, Error)]
pub enum SwapError {
    #[error("IC クライアントエラー: {0}")]
    Client(String),
    #[error("candid エンコード失敗: {0}")]
    Encode(String),
}

pub async fn swap_kong(
    client: &IcClient,
    kong_canister: &str,
    pay_token: &str,
    receive_token: &str,
    pay_amount: u128,
    min_receive_amount: u128,
) -> Result<Vec<u8>, SwapError> {
    #[derive(candid::CandidType)]
    struct SwapParams {
        receive_token: String,
        pay_amount: u128,
        receive_amount: Option<u128>,
        pay_token: String,
    }

    let params = SwapParams {
        receive_token: receive_token.to_string(),
        pay_amount,
        receive_amount: Some(min_receive_amount),
        pay_token: pay_token.to_string(),
    };

    let args = Encode!(&params).map_err(|e| SwapError::Encode(e.to_string()))?;

    client
        .update_raw(kong_canister, "swap_async", args)
        .await
        .map_err(|e| SwapError::Client(e.to_string()))
}

pub async fn swap_icps(
    client: &IcClient,
    lp_canister: &str,
    amount_in: u128,
    min_amount_out: u128,
    zero_for_one: bool,
) -> Result<Vec<u8>, SwapError> {
    #[derive(candid::CandidType, Serialize)]
    struct SwapParams {
        #[serde(rename = "amountIn")]
        amount_in: String,
        #[serde(rename = "zeroForOne")]
        zero_for_one: bool,
        #[serde(rename = "amountOutMinimum")]
        amount_out_minimum: String,
    }

    let params = SwapParams {
        amount_in: amount_in.to_string(),
        zero_for_one,
        amount_out_minimum: min_amount_out.to_string(),
    };

    let args = Encode!(&params).map_err(|e| SwapError::Encode(e.to_string()))?;

    client
        .update_raw(lp_canister, "swap", args)
        .await
        .map_err(|e| SwapError::Client(e.to_string()))
}
