//! Coverage report: per-domain concept counts and each concept's mount state.
//!
//! A concept is classified by the source type of its inbound edges, not by its own frontmatter
//! (status is not discriminative — concepts sit at `seedling` whether mapped or not). An edge from a
//! map (MOC / topic-map / source-map) means the concept is mounted; an edge only from a lesson or
//! another concept means it is in the corpus but not yet on a map (pending_mount, advisory); no
//! inbound edge at all is a true orphan. Only the last is a real problem.

use std::collections::{HashMap, HashSet};

use crate::graph::{Graph, Resolution};

/// Per-domain concept counts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DomainCoverage {
    pub domain: String,
    pub concepts: usize,
    pub mounted: usize,
    pub pending_mount: usize,
    pub orphan: usize,
}

/// The coverage report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Coverage {
    pub total_concepts: usize,
    pub domains: Vec<DomainCoverage>,
    /// Concepts reached only by a lesson/concept edge, not yet mounted on a map (advisory).
    pub pending_mount: Vec<String>,
    /// Concepts with no inbound edge at all.
    pub orphans: Vec<String>,
}

/// Compute coverage over the graph.
#[must_use]
pub fn compute(graph: &Graph) -> Coverage {
    // Reverse edges: which concepts are reached, and whether by a map.
    let mut mapped: HashSet<&str> = HashSet::new();
    let mut referenced: HashSet<&str> = HashSet::new();
    for note in &graph.notes {
        // System/ holds templates and specs, not knowledge content; it is out of scope for coverage.
        if note.path.starts_with("System/") {
            continue;
        }
        let from_map = matches!(
            note.note_type.as_deref(),
            Some("moc" | "topic-map" | "source-map")
        );
        let body = note.wikilinks.iter().map(|w| w.target.as_str());
        let provenance = note
            .based_on
            .iter()
            .chain(&note.related)
            .map(String::as_str);
        for target in body.chain(provenance) {
            for path in resolve_targets(graph, target) {
                referenced.insert(path);
                if from_map {
                    mapped.insert(path);
                }
            }
        }
    }

    // Classify each concept.
    let mut domains: HashMap<&str, DomainCoverage> = HashMap::new();
    let mut pending = Vec::new();
    let mut orphans = Vec::new();
    let mut total = 0;
    for note in &graph.notes {
        if note.note_type.as_deref() != Some("concept") || note.path.starts_with("System/") {
            continue;
        }
        total += 1;
        let domain = note.domain.as_deref().unwrap_or("(none)");
        let row = domains.entry(domain).or_insert_with(|| DomainCoverage {
            domain: domain.to_owned(),
            concepts: 0,
            mounted: 0,
            pending_mount: 0,
            orphan: 0,
        });
        row.concepts += 1;
        if mapped.contains(note.path.as_str()) {
            row.mounted += 1;
        } else if referenced.contains(note.path.as_str()) {
            row.pending_mount += 1;
            pending.push(note.path.clone());
        } else {
            row.orphan += 1;
            orphans.push(note.path.clone());
        }
    }

    let mut domains: Vec<DomainCoverage> = domains.into_values().collect();
    domains.sort_by(|a, b| a.domain.cmp(&b.domain));
    pending.sort();
    orphans.sort();
    Coverage {
        total_concepts: total,
        domains,
        pending_mount: pending,
        orphans,
    }
}

/// Resolve a body-link or provenance value to the note path(s) it reaches. An ambiguous name
/// contributes an edge to every candidate, so an ambiguously-referenced concept is never miscounted
/// as a true orphan.
fn resolve_targets<'a>(graph: &'a Graph, value: &str) -> Vec<&'a str> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix("[[")
        .and_then(|s| s.strip_suffix("]]"))
        .unwrap_or(trimmed);
    let Some(target) = crate::wikilink::strip_target(inner) else {
        return Vec::new();
    };
    match graph.symbols.resolve(&target) {
        Resolution::One(path) => vec![path],
        Resolution::Ambiguous(members) => members.iter().map(String::as_str).collect(),
        Resolution::Unresolved => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    // unwrap on a known-present fixture is the assertion itself.
    #![allow(clippy::unwrap_used)]

    use super::compute;
    use crate::Graph;
    use crate::model::Note;

    fn graph(notes: &[(&str, &str)]) -> Graph {
        Graph::build(
            notes
                .iter()
                .map(|(p, c)| Note::from_markdown(p, c))
                .collect(),
            &[],
        )
    }

    #[test]
    fn classifies_concepts_by_inbound_edge_source() {
        let g = graph(&[
            (
                "Concepts/golang/Mounted.md",
                "---\ntype: concept\ndomain: golang\n---\n",
            ),
            (
                "Concepts/golang/Pending.md",
                "---\ntype: concept\ndomain: golang\n---\n",
            ),
            (
                "Concepts/golang/Orphan.md",
                "---\ntype: concept\ndomain: golang\n---\n",
            ),
            (
                "Maps/topics/Go MOC.md",
                "---\ntype: topic-map\ndomain: golang\n---\n[[Mounted]]\n",
            ),
            (
                "Writing/lessons/golang/L1.md",
                "---\ntype: lesson\ndomain: golang\nbased_on:\n  - \"[[Pending]]\"\n---\n",
            ),
        ]);
        let cov = compute(&g);
        assert_eq!(cov.total_concepts, 3);
        assert_eq!(cov.orphans, ["Concepts/golang/Orphan.md"]);
        assert_eq!(cov.pending_mount, ["Concepts/golang/Pending.md"]);
        let golang = cov.domains.iter().find(|d| d.domain == "golang").unwrap();
        assert_eq!(
            (golang.mounted, golang.pending_mount, golang.orphan),
            (1, 1, 1)
        );
    }
}
