//! Core types: `Note` (a corpus node), `Finding` (a diagnostic), `Severity`.
//! These define the shape of the output contract; the data model is fixed before the logic.

use std::collections::BTreeMap;

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

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        })
    }
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
    /// `source_kind` frontmatter (e.g. `curriculum-gap` marks a planned, not-yet-written lesson).
    pub source_kind: Option<String>,
    pub topics: Vec<String>,
    pub slug: Option<String>,
    /// English title (lesson frontmatter); a writer may search a concept by its English name.
    pub title_en: Option<String>,
    /// Raw values of the provenance / relation fields (not yet resolved).
    pub based_on: Vec<String>,
    pub related: Vec<String>,
    /// Supersession links (lesson-only): the slug this note supersedes, and the slugs that
    /// supersede it.
    pub evolution_predecessor: Option<String>,
    pub evolution_successors: Vec<String>,
    /// Wikilinks appearing in the body (raw text, not yet resolved).
    pub wikilinks: Vec<WikiLink>,
    /// Non-wikilink file references in the body (markdown links, backticked path tokens).
    pub path_refs: Vec<PathRef>,
    /// Plain-text concept names this note lists under a gap heading — concepts planned but not yet
    /// written. A `[[X]]` elsewhere whose target is one of these is a tracked forward-reference.
    pub planned_names: Vec<String>,
    /// True when the file has no frontmatter (a raw transcript).
    pub no_frontmatter: bool,
    /// True when a `---`-fenced block is present but its YAML failed to parse. Distinguished from
    /// `no_frontmatter` so the schema check reports one "invalid frontmatter" finding instead of a
    /// false cascade of "required field missing" (the fields may be present above a syntax error).
    pub bad_frontmatter: bool,
    /// Every frontmatter key and its raw value — the schema checks need all keys (for unknown-key
    /// detection) and the raw enum values, beyond the typed fields above.
    pub frontmatter: BTreeMap<String, FmValue>,
}

/// A raw frontmatter value, kept generic so the schema checks see exactly what the author wrote.
#[derive(Debug, Clone)]
pub enum FmValue {
    /// A scalar (string, number, bool, or empty/null), as a string.
    Scalar(String),
    /// A YAML list, as its string items.
    List(Vec<String>),
}

impl FmValue {
    /// The scalar text, or `None` for a list.
    #[must_use]
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            FmValue::Scalar(s) => Some(s),
            FmValue::List(_) => None,
        }
    }

    /// Whether the value is present and non-empty (a non-empty scalar or a non-empty list).
    #[must_use]
    pub fn is_present(&self) -> bool {
        match self {
            FmValue::Scalar(s) => !s.is_empty(),
            FmValue::List(items) => !items.is_empty(),
        }
    }
}

/// A non-wikilink reference to a file: a markdown `[text](path)` link, or a backticked `path.md`
/// token in prose. The link.broken.path check resolves these against the filesystem, not the graph.
#[derive(Debug, Clone)]
pub struct PathRef {
    /// The raw destination path (a markdown link's `(...)`, or a backtick token's text).
    pub target: String,
    /// 1-based line where it appears.
    pub line: usize,
    /// True for a backticked path token (resolved vault-root-relative); false for a markdown link
    /// (resolved relative to the citing note).
    pub code: bool,
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

/// Stable fingerprint for a finding: FNV-1a over (rule_id, path, target), hex-encoded. The path is
/// the note's already-normalized vault-relative path; the target is the rule's structured key as
/// given (a link target, an alias, or a provenance value). Deterministic across runs and machines
/// (unlike `DefaultHasher`) and not tied to line numbers, so a consumer can set-diff two stateless
/// scans to find a branch's delta.
#[must_use]
pub(crate) fn fingerprint(rule_id: &str, path: &str, target: &str) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for part in [rule_id, path, target] {
        for &byte in part.as_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
        }
        // A separator so ("a", "b") and ("ab", "") cannot collide.
        hash = (hash ^ u64::from(b'\x1f')).wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::fingerprint;

    #[test]
    fn fingerprint_is_stable_and_field_sensitive() {
        assert_eq!(
            fingerprint("link.broken", "a/b.md", "X"),
            fingerprint("link.broken", "a/b.md", "X"),
        );
        assert_ne!(
            fingerprint("link.broken", "a/b.md", "X"),
            fingerprint("link.broken", "a/b.md", "Y"),
        );
        // The separator prevents field-boundary collisions.
        assert_ne!(fingerprint("ab", "", "X"), fingerprint("a", "b", "X"),);
    }
}
