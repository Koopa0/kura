//! kura — a read-only knowledge-graph guardian / indexer for the Koopa Obsidian vault.
//!
//! It scans the whole corpus once, builds a link graph + symbol table, runs corpus-level checks,
//! and emits JSONL + a summary. Read-only: it never modifies files. The enum source of truth is the
//! vault's `System/schemas/vault-schema.toml` (kura never carries a second copy).
//!
//! All real logic lives in this library; `main.rs` is a thin shell. A future MCP layer would wrap
//! the same library.

#![forbid(unsafe_code)]

pub mod graph;
pub mod model;
mod note;
pub mod rules;
pub mod vault;
mod wikilink;

pub use graph::{Graph, Resolution, SymbolTable};
pub use model::{Finding, Note, Severity, WikiLink};

/// Tool error (the library uses concrete `thiserror` types; the binary boundary uses `anyhow`).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read vault: {0}")]
    Walk(String),
    #[error("failed to load schema {path}: {source}")]
    Schema {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The result of one check: findings (deterministically sorted) and per-severity counts.
#[derive(Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
}

impl Report {
    /// Deterministic total-order sort (path -> line -> rule_id). Always call before emit.
    pub fn sort(&mut self) {
        self.findings
            .sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    #[must_use]
    pub fn count(&self, sev: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == sev).count()
    }

    /// Whether any finding reaches the deny threshold (used for gating).
    #[must_use]
    pub fn has_at_least(&self, deny: Severity) -> bool {
        self.findings.iter().any(|f| f.severity >= deny)
    }

    /// Render findings as JSONL (one JSON object per line) — pure data for stdout in json mode.
    ///
    /// # Errors
    /// Returns a `serde_json` error if a finding fails to serialize.
    pub fn to_jsonl(&self) -> serde_json::Result<String> {
        let mut out = String::new();
        for f in &self.findings {
            out.push_str(&serde_json::to_string(f)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// A human summary: a count line, then every non-info finding (info is hidden by default).
    #[must_use]
    pub fn summary(&self) -> String {
        use std::fmt::Write as _;
        let mut s = String::new();
        let _ = writeln!(
            s,
            "{} findings: {} error, {} warn, {} info",
            self.findings.len(),
            self.count(Severity::Error),
            self.count(Severity::Warn),
            self.count(Severity::Info),
        );
        for f in self
            .findings
            .iter()
            .filter(|f| f.severity != Severity::Info)
        {
            let _ = writeln!(
                s,
                "  [{:?}] {}:{} {}",
                f.severity,
                f.path,
                f.line.unwrap_or(0),
                f.message
            );
        }
        s
    }
}

/// Scan `root` for corpus-level findings.
///
/// # Errors
/// Returns [`Error`] if walking or reading a file fails.
pub fn check(root: &std::path::Path, paths: &[String], all: bool) -> Result<Report> {
    let walk = vault::load(root)?;
    let graph = Graph::build(walk.notes, &walk.resources);
    let mut findings = rules::run(&graph);
    // The graph is always built whole-tree; these only filter which findings are printed.
    if !all {
        // Default scope skips System/: those files cite reports and specs, not live links.
        findings.retain(|f| !f.path.starts_with("System/"));
    }
    if !paths.is_empty() {
        let wanted: Vec<String> = paths.iter().map(|p| p.replace('\\', "/")).collect();
        findings.retain(|f| {
            wanted
                .iter()
                .any(|w| f.path == *w || f.path.starts_with(&format!("{w}/")))
        });
    }
    let mut report = Report { findings };
    report.sort();
    Ok(report)
}
