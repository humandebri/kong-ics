// どこで: クイックデバッグ用バイナリ
// 何を: ICS metadata と Kong pools を query し、Candid のラベルIDごとに表示
// なぜ: .did が手元にない状態でフィールドIDを確認するため

use candid::{types::value::IDLValue, Encode, IDLArgs};
use kong_ics::config::AppConfig;
use kong_ics::ic_client::agent::IcClient;
use kong_ics::identity::load_identity;
use std::error::Error;
use std::path::Path;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();
    let cfg = AppConfig::load_default();
    let pair = cfg.pairs.first().expect("少なくとも1ペア必要です (config)");

    let identity = load_identity(Path::new(&cfg.identity.pem_path))?;
    let client = IcClient::new(
        &cfg.network.api_url,
        identity.clone(),
        cfg.network.fetch_root_key,
    )
    .await?;

    // ICS metadata
    let meta_args = candid::Encode!()?;
    let meta_raw = client
        .query_raw(&pair.icpswap_lp, "metadata", meta_args)
        .await?;
    println!("=== ICS metadata ===");
    print_raw("metadata raw", &meta_raw);
    print_idl(&meta_raw);

    // Kong pools
    let pools_args = Encode!(&Some(pair.symbol.clone()))?;
    let pools_raw = client
        .query_raw(&pair.kong_canister, "pools", pools_args)
        .await?;
    println!("=== Kong pools ({}) ===", pair.symbol);
    print_raw("pools raw", &pools_raw);
    print_idl(&pools_raw);

    Ok(())
}

fn print_idl(bytes: &[u8]) {
    match IDLArgs::from_bytes(bytes) {
        Ok(args) => {
            for (i, v) in args.args.iter().enumerate() {
                println!("[{}] {:?}", i, v);
                if let IDLValue::Record(fields) = v {
                    for field in fields {
                        println!("  - {:?}: {:?}", field.id, field.val);
                    }
                }
            }
        }
        Err(e) => {
            println!("デコード失敗: {}", e);
        }
    }
}

fn print_raw(label: &str, bytes: &[u8]) {
    print!("{label} ({} bytes): 0x", bytes.len());
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
