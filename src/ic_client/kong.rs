// どこで: Kong canister へのクエリを扱うクライアント
// 何を: pools メソッドから残高を取得し (ICP, SNS) を返す
// なぜ: アービトラージ計算の基準価格として利用するため

use candid::types::Label;
use candid::{types::value::IDLField, types::value::IDLValue, Encode, IDLArgs, Nat};
use thiserror::Error;

use super::agent::IcClient;

#[derive(Debug, Clone)]
pub struct KongPoolSnapshot {
    pub icp_balance: f64,
    pub sns_balance: f64,
    pub icp_lp_fee: f64,
    pub sns_lp_fee: f64,
    pub icp_raw: u128,
    pub sns_raw: u128,
    pub icp_lp_raw: u128,
    pub sns_lp_raw: u128,
    pub price_icp_per_sns: f64,
    pub lp_fee_bps: u32,
}

#[derive(Debug, Error)]
pub enum KongError {
    #[error("IC クライアントエラー: {0}")]
    Client(String),
    #[error("candid デコード失敗: {0}")]
    Decode(String),
    #[error("pools から必要なフィールドを取得できませんでした")]
    MissingFields,
}

pub async fn fetch_pool_snapshot(
    client: &IcClient,
    kong_canister: &str,
    ticker: &str,
) -> Result<KongPoolSnapshot, KongError> {
    // Opt<Text> で ticker を指定
    let args = Encode!(&Some(ticker.to_string())).map_err(|e| KongError::Decode(e.to_string()))?;

    let raw = client
        .query_raw(kong_canister, "pools", args)
        .await
        .map_err(|e| KongError::Client(e.to_string()))?;

    parse_pools(&raw, ticker)
}

fn parse_pools(raw: &[u8], ticker: &str) -> Result<KongPoolSnapshot, KongError> {
    let args = IDLArgs::from_bytes(raw).map_err(|e| KongError::Decode(e.to_string()))?;
    let first = args.args.first().ok_or(KongError::MissingFields)?;

    // 期待形: variant { 17724 = vec { record { ... } } }
    let pools_vec = match first {
        IDLValue::Variant(v) => {
            let field = v.0.as_ref();
            let is_ok = match &field.id {
                Label::Named(name) if name == "ok" => true,
                Label::Id(id) if *id == 17_724u32 => true,
                _ => false,
            };
            if is_ok {
                match &field.val {
                    IDLValue::Vec(v) => v.as_slice(),
                    _ => return Err(KongError::MissingFields),
                }
            } else {
                return Err(KongError::MissingFields);
            }
        }
        _ => return Err(KongError::MissingFields),
    };

    let entry = pools_vec.first().ok_or(KongError::MissingFields)?;
    let entry_record = match entry {
        IDLValue::Record(entries) => entries,
        _ => return Err(KongError::MissingFields),
    };

    // ticker は 4_007_505_752 または 486_076_665 に入っているケースを許容
    let ticker_val = extract_text_any(entry_record, &[4_007_505_752u32, 486_076_665u32])
        .ok_or(KongError::MissingFields)?;
    if ticker_val != ticker {
        return Err(KongError::Decode("ticker mismatch".to_string()));
    }

    let sns = extract_nat(entry_record, 1_476_685_581u32).ok_or(KongError::MissingFields)?;
    let icp = extract_nat(entry_record, 1_476_685_582u32).ok_or(KongError::MissingFields)?;
    let sns_lp_fee =
        extract_nat(entry_record, 1_283_592_060u32).ok_or(KongError::MissingFields)?;
    let icp_lp_fee =
        extract_nat(entry_record, 1_283_592_061u32).ok_or(KongError::MissingFields)?;
    let price_icp_per_sns =
        extract_float(entry_record, 3_364_572_809u32).ok_or(KongError::MissingFields)?;
    let lp_fee_bps_nat =
        extract_nat(entry_record, 4_243_077_425u32).ok_or(KongError::MissingFields)?;

    let sns_f = nat_to_f64(&sns)?;
    let icp_f = nat_to_f64(&icp)?;
    let sns_lp_fee_f = nat_to_f64(&sns_lp_fee)?;
    let icp_lp_fee_f = nat_to_f64(&icp_lp_fee)?;
    let sns_raw = nat_to_u128(&sns)?;
    let icp_raw = nat_to_u128(&icp)?;
    let sns_lp_raw = nat_to_u128(&sns_lp_fee)?;
    let icp_lp_raw = nat_to_u128(&icp_lp_fee)?;
    let lp_fee_bps = lp_fee_bps_nat
        .0
        .to_string()
        .parse::<u32>()
        .map_err(|e| KongError::Decode(e.to_string()))?;

    Ok(KongPoolSnapshot {
        icp_balance: icp_f,
        sns_balance: sns_f,
        icp_lp_fee: icp_lp_fee_f,
        sns_lp_fee: sns_lp_fee_f,
        icp_raw,
        sns_raw,
        icp_lp_raw,
        sns_lp_raw,
        price_icp_per_sns,
        lp_fee_bps,
    })
}

fn extract_text_any(entries: &[IDLField], ids: &[u32]) -> Option<String> {
    for field in entries {
        for id in ids {
            if field.id == Label::Id(*id) {
                if let IDLValue::Text(s) = &field.val {
                    return Some(s.clone());
                }
            }
        }
    }
    None
}

fn extract_nat(entries: &[IDLField], id: u32) -> Option<Nat> {
    for field in entries {
        if field.id == Label::Id(id) {
            return match &field.val {
                IDLValue::Nat(n) => Some(n.clone()),
                IDLValue::Nat8(v) => Some(Nat::from(*v)),
                IDLValue::Nat16(v) => Some(Nat::from(*v)),
                IDLValue::Nat32(v) => Some(Nat::from(*v)),
                IDLValue::Nat64(v) => Some(Nat::from(*v)),
                _ => None,
            };
        }
    }
    None
}

fn nat_to_f64(n: &Nat) -> Result<f64, KongError> {
    let s = n.0.to_string();
    s.parse::<f64>()
        .map_err(|e| KongError::Decode(e.to_string()))
}

fn nat_to_u128(n: &Nat) -> Result<u128, KongError> {
    n.0.to_string()
        .parse::<u128>()
        .map_err(|e| KongError::Decode(e.to_string()))
}

fn extract_float(entries: &[IDLField], id: u32) -> Option<f64> {
    for field in entries {
        if field.id == Label::Id(id) {
            if let IDLValue::Float64(v) = &field.val {
                return Some(*v);
            }
        }
    }
    None
}
