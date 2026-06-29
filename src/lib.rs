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
}

/// Scan `root` for corpus-level findings.
///
/// # Errors
/// Returns [`Error`] if walking or reading a file fails.
pub fn check(root: &std::path::Path, _paths: &[String]) -> Result<Report> {
    // For now this only builds the graph (walk -> parse -> symbol table) to prove the pipeline runs
    // on the real tree without panicking; finding emission is not wired yet, so the graph is dropped.
    let walk = vault::load(root)?;
    let _ = Graph::build(walk.notes, &walk.resources);
    Ok(Report::default())
}
