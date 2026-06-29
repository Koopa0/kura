//! `link.broken.path`: the one check that resolves against the filesystem, not the graph.
//!
//! The five rules in `rules` only see `[[wikilinks]]` and frontmatter. Markdown `[text](path)` links
//! and backticked `path.md` tokens are structurally invisible to them, yet they rot too. This check
//! stats them, with a load-bearing determinism boundary that protects the never-false-positive
//! invariant: a relative path that stays inside `--root` but has no file is a real dead link (`Warn`);
//! a path that escapes `--root` cannot be checked the same way on every machine, so it is reported
//! `Info` ("external, not stat'd") and never as broken.

use std::path::Path;

use crate::model::{Finding, Note, PathRef, Severity, fingerprint};

/// Stat every note's path references and emit findings. Needs the vault root for resolution.
#[must_use]
pub fn check(notes: &[Note], root: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    for note in notes {
        let note_dir = note.path.rsplit_once('/').map_or("", |(dir, _)| dir);
        for pref in &note.path_refs {
            if let Some(finding) = classify(note, note_dir, pref, root) {
                out.push(finding);
            }
        }
    }
    out
}

fn classify(note: &Note, note_dir: &str, pref: &PathRef, root: &Path) -> Option<Finding> {
    if pref.code {
        // A backticked token is written vault-root-relative; accept it if it exists either
        // root-relative or note-relative before calling it dead.
        let root_rel = resolve_within_root("", &pref.target);
        let note_rel = resolve_within_root(note_dir, &pref.target);
        let exists = [&root_rel, &note_rel]
            .into_iter()
            .flatten()
            .any(|rel| root.join(rel).exists());
        if exists {
            return None;
        }
        Some(dead_in_root(note, pref, root_rel.as_deref()?))
    } else {
        // A markdown link is note-relative.
        match resolve_within_root(note_dir, &pref.target) {
            None => Some(external(note, pref)), // escapes the root — not stat'd
            Some(rel) if root.join(&rel).exists() => None,
            Some(rel) => Some(dead_in_root(note, pref, &rel)),
        }
    }
}

/// Resolve `dest` against `base_dir` (both vault-relative, `/`-separated), collapsing `.` and `..`.
/// Returns the normalized vault-relative path, or `None` if it escapes above the root.
fn resolve_within_root(base_dir: &str, dest: &str) -> Option<String> {
    let mut comps: Vec<&str> = base_dir.split('/').filter(|c| !c.is_empty()).collect();
    for part in dest.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                comps.pop()?;
            }
            other => comps.push(other),
        }
    }
    Some(comps.join("/"))
}

/// A relative path that stays inside the vault but has no file (`Warn`).
fn dead_in_root(note: &Note, pref: &PathRef, resolved: &str) -> Finding {
    Finding {
        rule_id: "link.broken.path".to_owned(),
        severity: Severity::Warn,
        path: note.path.clone(),
        line: Some(pref.line),
        field: None,
        message: format!("link to {} resolves to no file", pref.target),
        evidence: format!("{resolved} does not exist in the vault"),
        suggested_action: "fix the path, restore the file, or remove the reference".to_owned(),
        source_rule: "vault-schema.toml#rules".to_owned(),
        target: Some(pref.target.clone()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("link.broken.path", &note.path, &pref.target),
    }
}

/// A path that escapes the vault root — reported but not stat'd, to stay deterministic (`Info`).
fn external(note: &Note, pref: &PathRef) -> Finding {
    Finding {
        rule_id: "link.broken.path".to_owned(),
        severity: Severity::Info,
        path: note.path.clone(),
        line: Some(pref.line),
        field: None,
        message: format!("link to {} points outside the vault root", pref.target),
        evidence: "external path, not stat'd (existence varies by environment)".to_owned(),
        suggested_action: "if it should be in the vault, fix the path; otherwise informational"
            .to_owned(),
        source_rule: "vault-schema.toml#rules".to_owned(),
        target: Some(pref.target.clone()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("link.broken.path", &note.path, &pref.target),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_within_root;

    #[test]
    fn resolve_collapses_dot_segments() {
        assert_eq!(
            resolve_within_root("Concepts/golang", "../rust/X.md").as_deref(),
            Some("Concepts/rust/X.md")
        );
        assert_eq!(
            resolve_within_root("Maps/topics", "./a/b.md").as_deref(),
            Some("Maps/topics/a/b.md")
        );
        // root-relative (empty base)
        assert_eq!(
            resolve_within_root("", "System/reports/x.md").as_deref(),
            Some("System/reports/x.md")
        );
    }

    #[test]
    fn resolve_returns_none_when_it_escapes_root() {
        // five `..` from a three-deep dir climbs above the vault root
        assert_eq!(
            resolve_within_root("Writing/lessons/golang", "../../../../../exam/go/x.md"),
            None
        );
    }
}
