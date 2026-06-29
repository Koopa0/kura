//! Core types: `Note` (a corpus node), `Finding` (a diagnostic), `Severity`.
//! These define the shape of the output contract; the data model is fixed before the logic.

use serde::Serialize;

/// Severity of one diagnostic. Mirrors the 0/1/2 exit-code model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Listed gaps, forward-references, formatting hints: never gates.
    Info,
    /// Unlisted broken links, collisions, untracked supersession: advisory (some may gate).
    Warn,
    /// Schema/enum violations, an archived note still in a syllabus: gates.
    Error,
}

/// A corpus node (one note). The symbol table and link graph are built on these.
#[derive(Debug, Clone)]
pub struct Note {
    /// Vault-relative path (normalized, forward-slash).
    pub path: String,
    pub title: Option<String>,
    pub aliases: Vec<String>,
    pub note_type: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub topics: Vec<String>,
    pub slug: Option<String>,
    /// Raw values of the provenance / relation fields (not yet resolved).
    pub based_on: Vec<String>,
    pub related: Vec<String>,
    /// Wikilinks appearing in the body (raw text, not yet resolved).
    pub wikilinks: Vec<WikiLink>,
    /// True when the file has no frontmatter (a raw transcript).
    pub no_frontmatter: bool,
}

/// A `[[target|display]]` link in a body.
#[derive(Debug, Clone)]
pub struct WikiLink {
    /// The target text inside `[[...]]` after stripping `#`/`|`/`^`.
    pub target: String,
    /// 1-based line where the link appears.
    pub line: usize,
    /// Whether it sits under a heading whose text contains a gap marker (a planned
    /// forward-reference).
    pub under_gap_heading: bool,
}

/// One diagnostic. One per line (JSONL); the field shape is the output contract.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    /// The source file that carries this finding (the one being cited).
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub message: String,
    pub evidence: String,
    pub suggested_action: String,
    /// Points back to the governing source of the rule.
    pub source_rule: String,
    /// The original link text (structured, so no prose parsing is needed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The path it resolved to; `None` means unresolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_to: Option<String>,
    /// All member paths of a collision (one finding lists them all, not N half-findings).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collision_members: Vec<String>,
    /// Stable fingerprint: hash(rule_id + normalized path + target), not tied to line numbers, so a
    /// consumer can set-diff two stateless scans to find a branch's delta.
    pub fingerprint: String,
}

impl Finding {
    /// Deterministic total-order sort key: path -> line -> rule_id.
    #[must_use]
    pub fn sort_key(&self) -> (&str, usize, &str) {
        (&self.path, self.line.unwrap_or(0), &self.rule_id)
    }
}
