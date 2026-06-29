//! Corpus-level rules: each turns the link graph into findings.
//!
//! Finding messages are Traditional Chinese on purpose — they are product output read in the
//! vault's own language, not code prose.

use std::collections::HashMap;

use crate::graph::{Graph, Resolution, normalize};
use crate::model::{Finding, Severity, fingerprint};
use crate::model::{Note, WikiLink};

/// Run every rule over `graph`, returning unsorted findings.
#[must_use]
pub fn run(graph: &Graph) -> Vec<Finding> {
    let titles = title_index(graph);
    let mut findings = Vec::new();
    link_health(graph, &titles, &mut findings);
    findings
}

/// Normalized title -> note paths that declare it. Titles are not resolution keys, so a link to a
/// title that is neither a filename nor an alias does not resolve; this index recovers which note
/// the author meant (the `link.title_not_alias` case).
fn title_index(graph: &Graph) -> HashMap<String, Vec<String>> {
    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    for note in &graph.notes {
        if let Some(title) = &note.title {
            let key = normalize(title);
            if !key.is_empty() {
                index.entry(key).or_default().push(note.path.clone());
            }
        }
    }
    for paths in index.values_mut() {
        paths.sort();
    }
    index
}

/// `link.title_not_alias` and `link.broken`: walk every note's wikilinks, resolve each, and classify
/// the ones that do not resolve. A resolved or ambiguous link is left to other rules.
fn link_health(graph: &Graph, titles: &HashMap<String, Vec<String>>, out: &mut Vec<Finding>) {
    for note in &graph.notes {
        for link in &note.wikilinks {
            if !matches!(graph.symbols.resolve(&link.target), Resolution::Unresolved) {
                continue;
            }
            if let Some(target_notes) = titles.get(&normalize(&link.target)) {
                out.push(title_not_alias(note, link, target_notes));
            } else if link.under_gap_heading {
                out.push(broken(note, link, Severity::Info));
            } else {
                out.push(broken(note, link, Severity::Warn));
            }
        }
    }
}

/// A link whose target is some note's title but not its filename or alias: Obsidian fails to resolve
/// it silently. The killer case.
fn title_not_alias(source: &Note, link: &WikiLink, target_notes: &[String]) -> Finding {
    let target_note = target_notes.first().map_or("", String::as_str);
    Finding {
        rule_id: "link.title_not_alias".to_owned(),
        severity: Severity::Warn,
        path: source.path.clone(),
        line: Some(link.line),
        field: None,
        message: format!("[[{}]] 解不到檔名或 alias", link.target),
        evidence: format!("目標標題存在於 {target_note},但不在其 aliases"),
        suggested_action: format!("把標題加進 {target_note} 的 aliases,或改連既有檔名/alias"),
        source_rule: "Note-Schema.md#aliases".to_owned(),
        target: Some(link.target.clone()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("link.title_not_alias", &source.path, &link.target),
    }
}

/// A link that resolves to nothing. `Info` when it sits under a planned-gap heading
/// (a forward-reference), otherwise `Warn`.
fn broken(source: &Note, link: &WikiLink, severity: Severity) -> Finding {
    let planned = severity == Severity::Info;
    Finding {
        rule_id: "link.broken".to_owned(),
        severity,
        path: source.path.clone(),
        line: Some(link.line),
        field: None,
        message: format!("[[{}]] 解不到任何筆記", link.target),
        evidence: if planned {
            "位於缺口/待補 heading 下,屬列管的 forward-reference".to_owned()
        } else {
            "目標檔名與 alias 皆無此名".to_owned()
        },
        suggested_action: if planned {
            "若已撰寫,確認檔名/alias 對得上;否則維持列管即可".to_owned()
        } else {
            "建立目標筆記,或把連結改成既有檔名/alias".to_owned()
        },
        source_rule: "Note-Schema.md#aliases".to_owned(),
        target: Some(link.target.clone()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("link.broken", &source.path, &link.target),
    }
}

#[cfg(test)]
mod tests {
    // unwrap on a known-present fixture is the assertion itself.
    #![allow(clippy::unwrap_used)]

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

    fn rule_ids(graph: &Graph) -> Vec<String> {
        super::run(graph).into_iter().map(|f| f.rule_id).collect()
    }

    #[test]
    fn title_link_that_is_not_an_alias_is_flagged() {
        let g = graph(&[
            (
                "Concepts/golang/Go Slice.md",
                "---\ntitle: \"Go Slice 內部結構\"\naliases:\n  - Slice Header\n---\nbody\n",
            ),
            ("note.md", "see [[Go Slice 內部結構]]\n"),
        ]);
        let findings = super::run(&g);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "link.title_not_alias");
        assert_eq!(findings[0].path, "note.md");
        assert_eq!(findings[0].severity, crate::Severity::Warn);
    }

    #[test]
    fn resolvable_link_produces_no_finding() {
        let g = graph(&[
            ("Go Slice.md", "body"),
            ("note.md", "see [[Go Slice]] and [[Slice Header]]\n"),
        ]);
        // Go Slice.md has no alias "Slice Header" here, so that one is a real break; the filename
        // link resolves.
        let ids = rule_ids(&g);
        assert_eq!(ids, ["link.broken"]); // only "Slice Header" is unresolved
    }

    #[test]
    fn unknown_link_is_broken_warn_planned_link_is_info() {
        let g = graph(&[(
            "n.md",
            "real text [[Ghost]]\n\n## 缺口 / 待補\n[[Planned Note]]\n",
        )]);
        let findings = super::run(&g);
        let by_target = |t: &str| {
            findings
                .iter()
                .find(|f| f.target.as_deref() == Some(t))
                .unwrap()
        };
        assert_eq!(by_target("Ghost").severity, crate::Severity::Warn);
        assert_eq!(by_target("Planned Note").severity, crate::Severity::Info);
    }
}
