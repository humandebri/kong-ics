// どこで: IC への低レベル呼び出しをまとめる Agent ラッパ
// 何を: Agent 初期化、query/update のエラーハンドリング一元化
// なぜ: agent-rs フォーク差分を吸収し、上位を安定させるため

use ic_agent::export::Principal;
use ic_agent::{Agent, AgentError, Identity};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IcClientError {
    #[error("Agent 初期化に失敗しました: {0}")]
    Init(String),
    #[error("query 失敗: {0}")]
    Query(String),
    #[error("update 失敗: {0}")]
    Update(String),
}

#[derive(Clone)]
pub struct IcClient {
    pub agent: Arc<Agent>,
}

impl IcClient {
    pub async fn new(
        url: &str,
        identity: Arc<dyn Identity + Send + Sync>,
        fetch_root_key: bool,
    ) -> Result<Self, IcClientError> {
        let agent = Agent::builder()
            .with_url(url)
            .with_arc_identity(identity)
            .build()
            .map_err(|e| IcClientError::Init(e.to_string()))?;
        if fetch_root_key {
            agent
                .fetch_root_key()
                .await
                .map_err(|e| IcClientError::Init(format!("root key: {}", e)))?;
        }
        Ok(IcClient {
            agent: Arc::new(agent),
        })
    }

    pub async fn query_raw(
        &self,
        canister: &str,
        method: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, IcClientError> {
        let canister_id =
            Principal::from_text(canister).map_err(|e| IcClientError::Query(e.to_string()))?;
        self.agent
            .query(&canister_id, method)
            .with_arg(args)
            .call()
            .await
            .map_err(|e| IcClientError::Query(render_agent_error(e)))
    }

    pub async fn update_raw(
        &self,
        canister: &str,
        method: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, IcClientError> {
        let canister_id =
            Principal::from_text(canister).map_err(|e| IcClientError::Update(e.to_string()))?;
        self.agent
            .update(&canister_id, method)
            .with_arg(args)
            .call_and_wait()
            .await
            .map_err(|e| IcClientError::Update(render_agent_error(e)))
    }
}

fn render_agent_error(err: AgentError) -> String {
    err.to_string()
}
