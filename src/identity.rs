// どこで: Rust 化した kong-ics の署名鍵管理
// 何を: PEM から BasicIdentity を構築し、Agent に渡す
// なぜ: 署名方法を一箇所に集約し再利用性を高めるため

use ic_agent::identity::{AnonymousIdentity, BasicIdentity, Secp256k1Identity};
use ic_agent::Identity;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("PEM 読み込みに失敗しました: {0}")]
    ReadFailed(String),
    #[error("PEM から identity を生成できませんでした: {0}")]
    BuildFailed(String),
}

pub fn load_identity(pem_path: &Path) -> Result<Arc<dyn Identity + Send + Sync>, IdentityError> {
    // パスが空文字の場合は匿名で返す
    if pem_path.as_os_str().is_empty() {
        return Ok(Arc::new(AnonymousIdentity {}));
    }

    let pem_str =
        fs::read_to_string(pem_path).map_err(|e| IdentityError::ReadFailed(e.to_string()))?;

    // EC (secp256k1) 鍵か PKCS8 (Ed25519) かで分岐
    if pem_str.contains("EC PRIVATE KEY") {
        let id = Secp256k1Identity::from_pem(pem_str.as_bytes())
            .map_err(|e| IdentityError::BuildFailed(e.to_string()))?;
        Ok(Arc::new(id))
    } else {
        let id = BasicIdentity::from_pem(pem_str.as_bytes())
            .map_err(|e| IdentityError::BuildFailed(e.to_string()))?;
        Ok(Arc::new(id))
    }
}
