// どこで: approve を定期確認・補充するユーティリティ
// 何を: ICRC-2 allowance を監視し、不足していれば approve を発行する
// なぜ: スワップ実行を速くするために事前に許可を張っておく

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use candid::{Encode, IDLArgs, Principal};
use kong_ics::config::AppConfig;
use kong_ics::ic_client::agent::IcClient;
use kong_ics::identity::load_identity;
use kong_ics::notify::DiscordNotifier;
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_tracing();
    let cfg = AppConfig::load_default();

    let identity = match load_identity(Path::new(&cfg.identity.pem_path)) {
        Ok(id) => id,
        Err(e) => {
            error!("Identity 読み込みに失敗: {}", e);
            return;
        }
    };

    let identity_for_client = identity.clone();
    let client = match IcClient::new(
        &cfg.network.api_url,
        identity_for_client,
        cfg.network.fetch_root_key,
    )
    .await
    {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("IcClient 初期化失敗: {}", e);
            return;
        }
    };

    let my_principal = match identity.sender() {
        Ok(p) => p,
        Err(e) => {
            error!("Identity から principal を取得できません: {}", e);
            return;
        }
    };

    let notifier = std::env::var(&cfg.discord.env_key)
        .ok()
        .map(DiscordNotifier::new);

    loop {
        for token in &cfg.approve.tokens {
            // SNS → Kong
            check_and_approve(
                &client,
                &token.sns,
                &cfg.approve.kong_canister,
                &my_principal,
                token.sns_threshold_e8,
                &token.name,
                "kong",
            )
            .await;

            // SNS → icpswap
            check_and_approve(
                &client,
                &token.sns,
                &token.icpswap,
                &my_principal,
                token.sns_threshold_e8,
                &token.name,
                "icpswap",
            )
            .await;

            // ICP → icpswap
            check_and_approve(
                &client,
                &cfg.approve.icp_canister,
                &token.icpswap,
                &my_principal,
                cfg.approve.icp_amount_e8,
                "icp",
                &token.name,
            )
            .await;
        }

        // ICP -> Kong も事前承認しておく
        check_and_approve(
            &client,
            &cfg.approve.icp_canister,
            &cfg.approve.kong_canister,
            &my_principal,
            cfg.approve.icp_amount_e8,
            "icp",
            "kong",
        )
        .await;

        if let Some(notifier) = &notifier {
            let _ = notifier.notify("approve_manager: チェック完了").await;
        }

        sleep(Duration::from_secs(cfg.approve.interval_secs)).await;
    }
}

async fn check_and_approve(
    client: &IcClient,
    token_canister: &str,
    spender_canister: &str,
    owner: &Principal,
    target_allowance: u128,
    token_label: &str,
    spender_label: &str,
) {
    match query_allowance(client, token_canister, owner, spender_canister).await {
        Ok(current) => {
            info!(
                "token:{} -> to:{} | allowance:{} target:{}",
                token_label, spender_label, current, target_allowance
            );
            if current < (target_allowance as f64 * 0.9) as u128 {
                info!(
                    "token:{} -> to:{} | approve send: {}",
                    token_label, spender_label, target_allowance
                );
                if let Err(e) =
                    send_approve(client, token_canister, spender_canister, target_allowance).await
                {
                    warn!(
                        "token:{} -> to:{} | approve failed: {}",
                        token_label, spender_label, e
                    );
                }
            }
        }
        Err(e) => warn!(
            "token:{} -> to:{} | allowance err: {}",
            token_label, spender_label, e
        ),
    }
}

async fn query_allowance(
    client: &IcClient,
    token_canister: &str,
    owner: &Principal,
    spender_canister: &str,
) -> Result<u128, String> {
    #[derive(candid::CandidType)]
    struct Account {
        owner: Principal,
        subaccount: Option<Vec<u8>>,
    }
    #[derive(candid::CandidType)]
    struct AllowanceReq {
        account: Account,
        spender: Account,
    }
    let args = Encode!(&AllowanceReq {
        account: Account {
            owner: owner.clone(),
            subaccount: None,
        },
        spender: Account {
            owner: Principal::from_text(spender_canister)
                .map_err(|e| format!("spender principal parse: {}", e))?,
            subaccount: None,
        },
    })
    .map_err(|e| e.to_string())?;

    let raw = client
        .query_raw(token_canister, "icrc2_allowance", args)
        .await
        .map_err(|e| e.to_string())?;

    let decoded = candid::IDLArgs::from_bytes(&raw).map_err(|e| e.to_string())?;
    let first = decoded
        .args
        .first()
        .ok_or_else(|| "missing allowance result".to_string())?;
    if let candid::types::value::IDLValue::Record(fields) = first {
        for field in fields {
            // 3230440920 が残高フィールド
            if field.id == candid::types::Label::Id(3_230_440_920u32) {
                if let candid::types::value::IDLValue::Nat(n) = &field.val {
                    let v = n.0.to_string().parse::<u128>().map_err(|e| e.to_string())?;
                    return Ok(v);
                }
            }
        }
    }
    Err("allowance field not found".to_string())
}

async fn send_approve(
    client: &IcClient,
    token_canister: &str,
    spender_canister: &str,
    amount: u128,
) -> Result<(), String> {
    #[derive(candid::CandidType)]
    struct Spender {
        owner: Principal,
        subaccount: Option<Vec<u8>>,
    }
    #[derive(candid::CandidType)]
    struct ApproveReq {
        fee: Option<u128>,
        memo: Option<Vec<u8>>,
        from_subaccount: Option<Vec<u8>>,
        created_at_time: Option<u64>,
        amount: u128,
        spender: Spender,
    }
    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;

    let args = Encode!(&ApproveReq {
        fee: None,
        memo: None,
        from_subaccount: None,
        created_at_time: Some(now_nanos),
        amount,
        spender: Spender {
            owner: Principal::from_text(spender_canister)
                .map_err(|e| format!("spender principal parse: {}", e))?,
            subaccount: None,
        },
    })
    .map_err(|e| e.to_string())?;

    let raw = client
        .update_raw(token_canister, "icrc2_approve", args)
        .await
        .map_err(|e| e.to_string())?;

    let decoded = IDLArgs::from_bytes(&raw)
        .map(|v| v.to_string())
        .unwrap_or_else(|e| format!("decode err: {}", e));
    info!(
        "approve response ({} -> {}): decoded={} raw_bytes={:?}",
        token_canister, spender_canister, decoded, raw
    );
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    info!("approve_manager を起動しました");
}
