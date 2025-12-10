// どこで: ICPSwap の metadata を取得するクライアント
// 何を: metadata メソッドを叩き、プールの k 値を計算する
// なぜ: アービトラージ計算の入力となる流動性指標が必要なため

use candid::types::Label;
use candid::{types::value::IDLField, types::value::IDLValue, Encode, IDLArgs, Nat};
use tracing::info;
use thiserror::Error;

use super::agent::IcClient;

#[derive(Debug, Clone)]
pub struct IcsPoolSnapshot {
    pub token0_k: f64,
    pub token1_k: f64,
}

#[derive(Debug, Error)]
pub enum IcsError {
    #[error("IC クライアントエラー: {0}")]
    Client(String),
    #[error("candid デコード失敗: {0}")]
    Decode(String),
    #[error("metadata から必要なフィールドを取得できませんでした")]
    MissingFields,
}

pub async fn fetch_pool_snapshot(
    client: &IcClient,
    canister: &str,
) -> Result<IcsPoolSnapshot, IcsError> {
    let args = Encode!().map_err(|e| IcsError::Decode(e.to_string()))?;
    let raw = client
        .query_raw(canister, "metadata", args)
        .await
        .map_err(|e| IcsError::Client(e.to_string()))?;

    // デバッグ用: レスポンスをデコードして出力
    let decoded = IDLArgs::from_bytes(&raw)
        .map(|v| v.to_string())
        .unwrap_or_else(|e| format!("decode err: {}", e));
    info!("ics metadata decoded: {}", decoded);

    parse_metadata(&raw)
}

fn parse_metadata(raw: &[u8]) -> Result<IcsPoolSnapshot, IcsError> {
    let args = IDLArgs::from_bytes(raw).map_err(|e| IcsError::Decode(e.to_string()))?;
    let first = args.args.first().ok_or(IcsError::MissingFields)?;

    // 期待形: variant { ok = record { sqrtPriceX96; liquidity; ... } }
    let record_fields = match first {
        IDLValue::Variant(variant) => {
            let field = variant.0.as_ref();
            let is_ok = match &field.id {
                Label::Named(name) if name == "ok" => true,
                Label::Id(id) if *id == 24_860u32 => true,
                _ => false,
            };
            if is_ok {
                match &field.val {
                    IDLValue::Record(entries) => entries.as_slice(),
                    _ => return Err(IcsError::MissingFields),
                }
            } else {
                return Err(IcsError::MissingFields);
            }
        }
        _ => return Err(IcsError::MissingFields),
    };

    let sqrt_price_val = extract_nat_named_or_id(record_fields, "sqrtPriceX96", 1_161_096_524u32)
        .ok_or(IcsError::MissingFields)?;
    let l_val = extract_nat_named_or_id(record_fields, "liquidity", 1_304_432_370u32)
        .ok_or(IcsError::MissingFields)?;

    let sqrt_price = nat_to_f64(&sqrt_price_val)?;
    let l = nat_to_f64(&l_val)?;

    // price = (sqrt_price^2)/(2^192)
    let price = (sqrt_price * sqrt_price) / (2f64.powi(192));
    let token0_k = l / price.sqrt();
    let token1_k = l * price.sqrt();

    Ok(IcsPoolSnapshot { token0_k, token1_k })
}

fn extract_nat_named_or_id(entries: &[IDLField], name: &str, id: u32) -> Option<Nat> {
    for field in entries {
        let matched = match &field.id {
            Label::Named(n) if n == name => true,
            Label::Id(i) if *i == id => true,
            _ => false,
        };
        if matched {
            if let IDLValue::Nat(n) = &field.val {
                return Some(n.clone());
            }
        }
    }
    None
}

fn nat_to_f64(n: &Nat) -> Result<f64, IcsError> {
    let s = n.0.to_string();
    s.parse::<f64>()
        .map_err(|e| IcsError::Decode(e.to_string()))
}
