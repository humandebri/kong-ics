// どこで: スワップ系 update 呼び出し
// 何を: Kong の swap_async と ICPSwap の swap を叩く
// なぜ: 取引実行を Rust から完結させるため

use candid::{Encode, IDLArgs, IDLValue};
use serde::Serialize;
use thiserror::Error;

use super::agent::IcClient;

#[derive(Debug, Error)]
pub enum SwapError {
    #[error("IC クライアントエラー: {0}")]
    Client(String),
    #[error("candid エンコード失敗: {0}")]
    Encode(String),
    #[error("swap エラー: {0}")]
    Swap(String),
}

pub async fn swap_kong(
    client: &IcClient,
    kong_canister: &str,
    pay_token: &str,
    receive_token: &str,
    pay_amount: u128,
    min_receive_amount: u128,
) -> Result<String, SwapError> {
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

    let raw = client
        .update_raw(kong_canister, "swap_async", args)
        .await
        .map_err(|e| SwapError::Client(e.to_string()))?;

    let decoded_args = IDLArgs::from_bytes(&raw).map_err(|e| SwapError::Client(e.to_string()))?;
    let mut is_err = false;
    let mut err_msg = String::new();
    if let Some(first) = decoded_args.args.get(0) {
        if let IDLValue::Variant(var) = first {
            let label = &var.0.id;
            // Err が Named/Id のどちらでも検出する
            if label == &candid::types::Label::Named("err".to_string())
                || label == &candid::types::Label::Id(3_456_837u32)
            {
                is_err = true;
                err_msg = format!("{}", var.1);
            }
        }
    }
    let decoded = decoded_args.to_string();
    if is_err {
        Err(SwapError::Swap(err_msg))
    } else {
        Ok(decoded)
    }
}

pub async fn swap_icps(
    client: &IcClient,
    lp_canister: &str,
    amount_in: u128,
    min_amount_out: u128,
    zero_for_one: bool,
) -> Result<String, SwapError> {
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

    let raw = client
        .update_raw(lp_canister, "swap", args)
        .await
        .map_err(|e| SwapError::Client(e.to_string()))?;

    let decoded = IDLArgs::from_bytes(&raw)
        .map(|v| v.to_string())
        .unwrap_or_else(|e| format!("decode err: {}", e));
    Ok(decoded)
}

pub async fn swap_icps_deposit(
    client: &IcClient,
    lp_canister: &str,
    amount_in: u128,
    min_amount_out: u128,
    zero_for_one: bool,
    token_in_fee: u128,
    token_out_fee: u128,
) -> Result<String, SwapError> {
    #[derive(candid::CandidType, Serialize)]
    struct SwapParams {
        #[serde(rename = "tokenInFee")]
        token_in_fee: u128,
        #[serde(rename = "amountIn")]
        amount_in: String,
        #[serde(rename = "zeroForOne")]
        zero_for_one: bool,
        #[serde(rename = "amountOutMinimum")]
        amount_out_minimum: String,
        #[serde(rename = "tokenOutFee")]
        token_out_fee: u128,
    }

    let params = SwapParams {
        token_in_fee,
        amount_in: amount_in.to_string(),
        zero_for_one,
        amount_out_minimum: min_amount_out.to_string(),
        token_out_fee,
    };

    let args = Encode!(&params).map_err(|e| SwapError::Encode(e.to_string()))?;

    let raw = client
        .update_raw(lp_canister, "depositFromAndSwap", args)
        .await
        .map_err(|e| SwapError::Client(e.to_string()))?;

    let decoded = IDLArgs::from_bytes(&raw)
        .map(|v| v.to_string())
        .unwrap_or_else(|e| format!("decode err: {}", e));
    Ok(decoded)
}
