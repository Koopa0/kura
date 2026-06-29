//! Walk the vault tree: markdown files become notes, other linkable files become resources.
//!
//! The link graph is always built from the whole `--root` tree; path arguments only filter
//! findings, never the graph. The walk skips hidden entries (`.obsidian/`, `.git/`, `.trash/`) but
//! does NOT honor `.gitignore` — Obsidian ignores git, so a gitignored attachment is still a live
//! link target and must be in the index, or a link to it would be falsely reported broken.

use std::path::Path;

use ignore::WalkBuilder;
use unicode_normalization::UnicodeNormalization;

use crate::model::Note;
use crate::{Error, Result};

/// Result of one walk: parsed markdown notes plus the relative paths of non-md linkable files.
///
/// Both feed the symbol table — Obsidian resolves `[[X.canvas]]` to a canvas/attachment on disk,
/// so a resolver that omits them would report a resolvable link as broken.
#[derive(Debug, Default)]
pub struct Walk {
    pub notes: Vec<Note>,
    /// Vault-relative paths of non-md linkable files (canvas, images, pdf, base, ...).
    pub resources: Vec<String>,
}

/// Walk `root`: `.md` files parse into a [`Note`], every other file becomes a linkable resource.
/// Both lists are sorted by path for deterministic output.
///
/// # Errors
/// Returns [`Error`] if walking fails or a file cannot be read (bad root / non-UTF-8).
pub fn load(root: &Path) -> Result<Walk> {
    let mut walk = Walk::default();
    let walker = WalkBuilder::new(root)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .ignore(false)
        .parents(false)
        .build();
    for entry in walker {
        let entry = entry.map_err(|e| Error::Walk(e.to_string()))?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let rel = relative_key(path.strip_prefix(root).unwrap_or(path));
        if path.extension().is_some_and(|ext| ext == "md") {
            let content = std::fs::read_to_string(path)?;
            walk.notes.push(Note::from_markdown(&rel, &content));
        } else {
            walk.resources.push(rel);
        }
    }
    walk.notes.sort_by(|a, b| a.path.cmp(&b.path));
    walk.resources.sort();
    Ok(walk)
}

/// Vault-relative path as a forward-slash, NFC-normalized string (Obsidian's canonical form;
/// stable regardless of the filesystem storing names as NFD on macOS).
fn relative_key(rel: &Path) -> String {
    rel.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
        .nfc()
        .collect()
}
