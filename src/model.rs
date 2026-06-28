//! 核心型別:Note(語料節點)、Finding(診斷)、Severity。
//! 這些是 output contract 的形狀,實作前先把資料模型定死。

use serde::Serialize;

/// 一筆診斷的嚴重度。對齊 spec §11 與 schema_lint 0/1/2 退出碼模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 列管缺口、forward-reference、格式建議:不 gate。
    Info,
    /// 未列管斷鏈、撞名、未追蹤 supersession:advisory(部分可 gate)。
    Warn,
    /// schema/enum 違規、archived 入課綱:gate。
    Error,
}

/// 一個語料節點(一篇筆記)。symbol table 與 link graph 都建在這上面。
#[derive(Debug, Clone)]
pub struct Note {
    /// vault 相對路徑(正規化、forward-slash)。
    pub path: String,
    pub title: Option<String>,
    pub aliases: Vec<String>,
    pub note_type: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub topics: Vec<String>,
    pub slug: Option<String>,
    /// provenance / 關聯欄位的原始值(尚未解析)。
    pub based_on: Vec<String>,
    pub related: Vec<String>,
    /// body 內出現的 wikilink(原始文字,未解析)。
    pub wikilinks: Vec<WikiLink>,
    /// 無 frontmatter(raw 逐字稿)= true。
    pub no_frontmatter: bool,
}

/// body 內一個 `[[target|display]]` 連結。
#[derive(Debug, Clone)]
pub struct WikiLink {
    /// `[[...]]` 內剝掉 `#`/`|`/`^` 後的目標文字。
    pub target: String,
    /// 連結所在行(1-based)。
    pub line: usize,
    /// 是否落在「含 缺口/待補/待寫 的 heading」下(planned forward-reference)。
    pub under_gap_heading: bool,
}

/// 一筆診斷。一行一筆(JSONL),欄位形狀 = spec §5 + §11(硬化後)。
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    /// 引用此 finding 的 SOURCE 檔(我要 cite 的那個)。
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub message: String,
    pub evidence: String,
    pub suggested_action: String,
    /// 指回治理來源(vault-schema.toml#... 或 Note-Schema.md#...)。
    pub source_rule: String,
    /// 原始 link 文字(結構化,不必解析 prose)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// 解析到的路徑;None = 解不到。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_to: Option<String>,
    /// collision 的全部成員路徑(一筆列全部,不拆 N 個半 finding)。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collision_members: Vec<String>,
    /// 穩定指紋:hash(rule_id + 正規化 path + target),不綁行號。
    /// 讓消費者 set-diff 兩次 stateless 掃出 branch delta。
    pub fingerprint: String,
}

impl Finding {
    /// 確定性全序排序鍵:path → line → rule_id。兩次掃可乾淨 diff、snapshot 不 churn。
    #[must_use]
    pub fn sort_key(&self) -> (&str, usize, &str) {
        (&self.path, self.line.unwrap_or(0), &self.rule_id)
    }
}
