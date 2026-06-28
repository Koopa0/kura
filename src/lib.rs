//! kura(蔵)— Koopa Obsidian vault 的 read-only 知識圖守衛 / 索引器。
//!
//! 一次掃全語料 → 建 link graph + symbol table → 跑語料層檢查 → 出 JSONL + 摘要。
//! read-only,不改檔。enum 來源是 vault 的 `System/schemas/vault-schema.toml`(不自帶第二份)。
//! 設計見 vault `System/vault-guard-spec.md`(§10-15 為 v1 定稿)。
//!
//! lib + 薄 CLI:核心邏輯全在這個 lib;`main.rs` 只是殼。之後若需 MCP,wrap 同一個 lib。

pub mod model;

pub use model::{Finding, Note, Severity, WikiLink};

/// 工具錯誤(lib 用 thiserror 具體型別;bin 邊界才 anyhow)。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("讀取 vault 失敗: {0}")]
    Walk(String),
    #[error("載入 schema {path} 失敗: {source}")]
    Schema {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 一次 check 的結果:findings(已確定性排序)+ 各 severity 計數。
#[derive(Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
}

impl Report {
    /// 確定性全序排序(path → line → rule_id)。emit 前一定呼叫。
    pub fn sort(&mut self) {
        self.findings
            .sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    #[must_use]
    pub fn count(&self, sev: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == sev).count()
    }

    /// 是否有達到 deny 門檻的 finding(gate 用)。
    #[must_use]
    pub fn has_at_least(&self, deny: Severity) -> bool {
        self.findings.iter().any(|f| f.severity >= deny)
    }
}

/// v1 P0:骨架已就位,實際檢查待 P1(parse+graph)/ P2(rules)。
///
/// 回傳空 Report 讓 CLI 端到端跑通(scaffold 驗收:空殼不爆)。
pub fn check(_root: &std::path::Path, _paths: &[String]) -> Result<Report> {
    // TODO(P1): walk → parse frontmatter + wikilinks → build symbol table + graph
    // TODO(P2): link.broken / title_not_alias / collision.alias / provenance.unresolved / map.disk_mismatch
    Ok(Report::default())
}
