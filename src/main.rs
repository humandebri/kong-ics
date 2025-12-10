use std::path::Path;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use kong_ics::arb::Trade;
use kong_ics::config::AppConfig;
use kong_ics::ic_client::agent::IcClient;
use kong_ics::identity::load_identity;
use kong_ics::notify::DiscordNotifier;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_tracing();
    let cfg = AppConfig::load_default();

    let identity = match load_identity(Path::new(&cfg.identity.pem_path)) {
        Ok(id) => id,
        Err(e) => {
            error!("Identity 読み込みに失敗しました: {}", e);
            return;
        }
    };

    let client = match IcClient::new(
        &cfg.network.api_url,
        identity.clone(),
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

    let notifier = std::env::var(&cfg.discord.env_key)
        .ok()
        .map(DiscordNotifier::new);

    let mut tasks = Vec::new();
    for pair in cfg.pairs {
        let trade = Trade::new(
            pair.clone(),
            client.clone(),
            notifier.clone(),
            cfg.trade.fee_rate,
            cfg.trade.min_receive_factor,
            cfg.trade.profit_threshold_e8,
            cfg.trade.loop_interval_ms,
        );
        let handle = tokio::spawn(run_loop(trade));
        tasks.push(handle);
    }

    futures::future::join_all(tasks).await;
}

async fn run_loop(trade: Trade) {
    loop {
        if let Err(e) = trade.tick().await {
            error!("{}: tick エラー {:?}", trade.symbol(), e);
        }
        sleep(Duration::from_millis(trade.loop_interval_ms())).await;
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    info!("kong-ics Rust 版を起動しました");
}
