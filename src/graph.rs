//! Symbol table and resolver.
//!
//! Resolution keys are a note's filename and aliases, normalized to NFC + lowercase (Obsidian is
//! case-insensitive and NFC-matches CJK). The frontmatter title is deliberately not a key: Obsidian
//! does not resolve wikilinks by title, so keying on it would make a link pointing at a file's
//! title — when that title is neither the filename nor an alias — look resolvable when it is not.
//! A name shared by several files is reported as ambiguous, never guessed.

use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

use crate::model::Note;

/// Normalize a name for resolution: trim, NFC, lowercase. The resolver and lookups share this.
#[must_use]
pub fn normalize(name: &str) -> String {
    name.trim().nfc().collect::<String>().to_lowercase()
}

/// Outcome of resolving one wikilink target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution<'a> {
    /// Resolved to exactly one file (its vault-relative path).
    One(&'a str),
    /// The name maps to several files — ambiguous, reported rather than guessed.
    Ambiguous(&'a [String]),
    /// No file exposes this name.
    Unresolved,
}

/// Maps every resolvable name (normalized) to the file paths that expose it.
#[derive(Debug, Default)]
pub struct SymbolTable {
    names: HashMap<String, Vec<String>>,
}

impl SymbolTable {
    /// Build from notes plus non-md resources. A note is keyed by every name Obsidian resolves it by
    /// (filename and vault-relative path, each with and without `.md`) plus its aliases; the title is
    /// excluded. A resource is keyed by its filename and path, both keeping the extension (Obsidian
    /// needs the extension to link a non-note file). Extra keys only ever add resolutions, never
    /// remove one, so they keep the false-negative bias.
    #[must_use]
    pub fn build(notes: &[Note], resources: &[String]) -> Self {
        let mut names: HashMap<String, Vec<String>> = HashMap::new();
        for note in notes {
            for key in note_keys(&note.path) {
                add(&mut names, key, &note.path);
            }
            for alias in &note.aliases {
                add(&mut names, alias, &note.path);
            }
        }
        for resource in resources {
            for key in resource_keys(resource) {
                add(&mut names, key, resource);
            }
        }
        for members in names.values_mut() {
            members.sort();
        }
        Self { names }
    }

    /// Resolve a wikilink `target` (already stripped of `#`/`|`/`^`). Anchors are not verified:
    /// `[[X#heading]]` resolves as long as file `X` exists (the caller stripped the anchor).
    #[must_use]
    pub fn resolve(&self, target: &str) -> Resolution<'_> {
        match self.names.get(&normalize(target)) {
            None => Resolution::Unresolved,
            Some(members) if members.len() == 1 => Resolution::One(&members[0]),
            Some(members) => Resolution::Ambiguous(members),
        }
    }
}

/// The link graph: notes plus the resolver built over them.
#[derive(Debug)]
pub struct Graph {
    pub notes: Vec<Note>,
    pub symbols: SymbolTable,
}

impl Graph {
    #[must_use]
    pub fn build(notes: Vec<Note>, resources: &[String]) -> Self {
        let symbols = SymbolTable::build(&notes, resources);
        Self { notes, symbols }
    }
}

/// Insert `key` (normalized) -> `path`; skip empty keys, dedupe per path.
fn add(names: &mut HashMap<String, Vec<String>>, key: &str, path: &str) {
    let norm = normalize(key);
    if norm.is_empty() {
        return;
    }
    let members = names.entry(norm).or_default();
    if !members.iter().any(|m| m == path) {
        members.push(path.to_owned());
    }
}

/// The names Obsidian resolves a markdown note by: filename and vault-relative path, each with and
/// without the `.md` extension. Duplicates (a note at the vault root) collapse when inserted.
fn note_keys(path: &str) -> [&str; 4] {
    [filename_stem(path), filename(path), path_stem(path), path]
}

/// The names Obsidian resolves a non-markdown file by: filename and vault-relative path, both keeping
/// the extension.
fn resource_keys(path: &str) -> [&str; 2] {
    [filename(path), path]
}

/// Full filename including extension.
fn filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Filename without directory or `.md`.
pub(crate) fn filename_stem(path: &str) -> &str {
    let f = filename(path);
    f.strip_suffix(".md").unwrap_or(f)
}

/// Vault-relative path without the `.md` extension.
fn path_stem(path: &str) -> &str {
    path.strip_suffix(".md").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::{filename_stem, normalize};

    #[test]
    fn normalize_is_trim_nfc_lowercase() {
        assert_eq!(normalize("  Go Slice  "), "go slice");
        // Decomposed (NFD) and composed (NFC) forms of "café" normalize equal.
        assert_eq!(normalize("cafe\u{0301}"), normalize("caf\u{00e9}"));
    }

    #[test]
    fn filename_stem_drops_dir_and_extension() {
        assert_eq!(filename_stem("Concepts/golang/Go Slice.md"), "Go Slice");
        assert_eq!(filename_stem("Foo.md"), "Foo");
    }
}
