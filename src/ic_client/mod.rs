// どこで: IC 呼び出しのクライアントをまとめるモジュール
// 何を: agent 初期化、ICS/Kong への query/update、swap 呼び出し
// なぜ: 外部依存をここに閉じ込め、上位ロジックを簡潔にするため

pub mod agent;
pub mod ics;
pub mod kong;
pub mod swap;
