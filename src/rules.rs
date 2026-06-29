//! Corpus-level rules: each turns the link graph into findings.

use std::collections::HashMap;

use crate::graph::{Graph, Resolution, normalize};
use crate::model::{Finding, Note, Severity, WikiLink, fingerprint};

/// Run every rule over `graph`, returning unsorted findings.
#[must_use]
pub fn run(graph: &Graph) -> Vec<Finding> {
    let titles = title_index(graph);
    let slugs = slug_index(graph);
    let mut findings = Vec::new();
    link_health(graph, &titles, &mut findings);
    collision_alias(graph, &mut findings);
    provenance_unresolved(graph, &slugs, &mut findings);
    findings
}

// --- shared indices ---------------------------------------------------------

/// Normalized title -> note paths that declare it. Titles are not resolution keys, so a link to a
/// title that is neither a filename nor an alias does not resolve; this index recovers which note the
/// author meant (the `link.title_not_alias` case).
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

/// Lesson slug -> note path. Supersession links (`evolution_*`) reference a slug, not a filename, so
/// they resolve here rather than through the wikilink resolver.
fn slug_index(graph: &Graph) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for note in &graph.notes {
        if let Some(slug) = &note.slug {
            index.insert(slug.clone(), note.path.clone());
        }
    }
    index
}

// --- link.title_not_alias + link.broken -------------------------------------

/// Walk every note's wikilinks, resolve each, and classify the ones that do not resolve. A resolved
/// or ambiguous link is left to other rules.
fn link_health(graph: &Graph, titles: &HashMap<String, Vec<String>>, out: &mut Vec<Finding>) {
    for note in &graph.notes {
        for link in &note.wikilinks {
            if !matches!(graph.symbols.resolve(&link.target), Resolution::Unresolved) {
                continue;
            }
            if let Some(target_notes) = titles.get(&normalize(&link.target)) {
                out.push(title_not_alias(note, link, target_notes));
            } else {
                out.push(broken(note, link));
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
        message: format!("[[{}]] resolves to no filename or alias", link.target),
        evidence: format!("the target is the title of {target_note} but not one of its aliases"),
        suggested_action: format!(
            "add the title to {target_note}'s aliases, or link an existing filename/alias"
        ),
        source_rule: "Note-Schema.md#aliases".to_owned(),
        target: Some(link.target.clone()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("link.title_not_alias", &source.path, &link.target),
    }
}

/// A link that resolves to nothing. `Info` when it sits under a planned-gap heading (a tracked
/// forward-reference), otherwise `Warn`.
fn broken(source: &Note, link: &WikiLink) -> Finding {
    let planned = link.under_gap_heading;
    Finding {
        rule_id: "link.broken".to_owned(),
        severity: if planned {
            Severity::Info
        } else {
            Severity::Warn
        },
        path: source.path.clone(),
        line: Some(link.line),
        field: None,
        message: format!("[[{}]] resolves to no note", link.target),
        evidence: if planned {
            "under a gap/backlog heading; a tracked forward-reference".to_owned()
        } else {
            "no filename or alias matches the target".to_owned()
        },
        suggested_action: if planned {
            "if it is written, check the filename/alias matches; otherwise leave it tracked"
                .to_owned()
        } else {
            "create the target note, or change the link to an existing filename/alias".to_owned()
        },
        source_rule: "Note-Schema.md#aliases".to_owned(),
        target: Some(link.target.clone()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("link.broken", &source.path, &link.target),
    }
}

// --- collision.alias --------------------------------------------------------

/// An alias declared (in frontmatter) by more than one note. `[[alias]]` then resolves to only one of
/// them and the others silently lose inbound links. Matching is case-insensitive and NFC, across all
/// note kinds; prose mentions do not count (only the `aliases` field).
fn collision_alias(graph: &Graph, out: &mut Vec<Finding>) {
    let mut by_alias: HashMap<String, Vec<String>> = HashMap::new();
    for note in &graph.notes {
        for alias in &note.aliases {
            let key = normalize(alias);
            if key.is_empty() {
                continue;
            }
            let members = by_alias.entry(key).or_default();
            if !members.iter().any(|m| m == &note.path) {
                members.push(note.path.clone());
            }
        }
    }
    let mut collisions: Vec<(String, Vec<String>)> =
        by_alias.into_iter().filter(|(_, m)| m.len() > 1).collect();
    collisions.sort();
    for (alias, mut members) in collisions {
        members.sort();
        out.push(collision(&alias, members));
    }
}

fn collision(alias: &str, members: Vec<String>) -> Finding {
    let count = members.len();
    let path = members.first().cloned().unwrap_or_default();
    let joined = members.join(", ");
    let fp = fingerprint("collision.alias", &path, alias);
    Finding {
        rule_id: "collision.alias".to_owned(),
        severity: Severity::Warn,
        path,
        line: None,
        field: Some("aliases".to_owned()),
        message: format!(
            "alias \"{alias}\" is declared by {count} notes, so [[{alias}]] cannot resolve deterministically"
        ),
        evidence: format!("shared alias across: {joined}"),
        suggested_action: "give the alias a single owner note, or qualify the duplicates"
            .to_owned(),
        source_rule: "vault-schema.toml#rules".to_owned(),
        target: Some(alias.to_owned()),
        resolved_to: None,
        collision_members: members,
        fingerprint: fp,
    }
}

// --- provenance.unresolved --------------------------------------------------

/// A `based_on` / `related` / `evolution_*` value that resolves to no note. `based_on` and `related`
/// are wikilinks (filename/alias); `evolution_*` are lesson slugs. A value is tried both ways before
/// being reported, to stay biased to false-negative.
fn provenance_unresolved(graph: &Graph, slugs: &HashMap<String, String>, out: &mut Vec<Finding>) {
    for note in &graph.notes {
        let predecessor: Vec<String> = note.evolution_predecessor.iter().cloned().collect();
        let fields: [(&str, &[String]); 4] = [
            ("based_on", &note.based_on),
            ("related", &note.related),
            ("evolution_predecessor", &predecessor),
            ("evolution_successors", &note.evolution_successors),
        ];
        for (field, values) in fields {
            for value in values {
                if !provenance_resolves(graph, slugs, value) {
                    out.push(provenance(note, field, value));
                }
            }
        }
    }
}

/// Whether a provenance value resolves either as a wikilink (filename/alias) or as a lesson slug.
/// The value may be a bare slug or a `[[wikilink]]` with `|display` / `#heading` / `^block`, which
/// are stripped the same way a body link is.
fn provenance_resolves(graph: &Graph, slugs: &HashMap<String, String>, value: &str) -> bool {
    let v = value.trim();
    let inner = v
        .strip_prefix("[[")
        .and_then(|s| s.strip_suffix("]]"))
        .unwrap_or(v);
    let Some(target) = crate::wikilink::strip_target(inner) else {
        return true; // nothing to resolve (a bare anchor); do not invent a finding
    };
    if !matches!(graph.symbols.resolve(&target), Resolution::Unresolved) {
        return true;
    }
    slugs.contains_key(target.as_str()) || slugs.contains_key(&normalize(&target))
}

fn provenance(note: &Note, field: &str, value: &str) -> Finding {
    Finding {
        rule_id: "provenance.unresolved".to_owned(),
        severity: Severity::Warn,
        path: note.path.clone(),
        line: None,
        field: Some(field.to_owned()),
        message: format!("{field} -> {value} resolves to nothing"),
        evidence: "no note, alias, or lesson slug matches the reference".to_owned(),
        suggested_action: "fix the reference, or create the target note".to_owned(),
        source_rule: "vault-schema.toml#provenance".to_owned(),
        target: Some(value.to_owned()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint("provenance.unresolved", &note.path, value),
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

    fn of_rule(graph: &Graph, rule: &str) -> Vec<crate::model::Finding> {
        super::run(graph)
            .into_iter()
            .filter(|f| f.rule_id == rule)
            .collect()
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
        let f = of_rule(&g, "link.title_not_alias");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].path, "note.md");
        assert_eq!(f[0].severity, crate::Severity::Warn);
    }

    #[test]
    fn unknown_link_is_warn_planned_link_is_info() {
        let g = graph(&[(
            "n.md",
            "real text [[Ghost]]\n\n## 缺口 / 待補\n[[Planned Note]]\n",
        )]);
        let f = of_rule(&g, "link.broken");
        let by_target = |t: &str| f.iter().find(|x| x.target.as_deref() == Some(t)).unwrap();
        assert_eq!(by_target("Ghost").severity, crate::Severity::Warn);
        assert_eq!(by_target("Planned Note").severity, crate::Severity::Info);
    }

    #[test]
    fn shared_alias_across_notes_is_one_collision() {
        let g = graph(&[
            ("a.md", "---\naliases:\n  - Mechanical Sympathy\n---\n"),
            // different case still collides (case-insensitive)
            ("b.md", "---\naliases:\n  - mechanical sympathy\n---\n"),
        ]);
        let f = of_rule(&g, "collision.alias");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].collision_members, ["a.md", "b.md"]);
    }

    #[test]
    fn unresolved_provenance_is_flagged_resolved_is_not() {
        let g = graph(&[
            ("Lesson.md", "---\nslug: lesson-x\n---\n"),
            ("DDIA.md", "body"),
            (
                "c.md",
                "---\nbased_on:\n  - \"[[DDIA]]\"\n  - \"[[Ghost]]\"\nevolution_predecessor: lesson-x\nevolution_successors:\n  - ghost-slug\n---\n",
            ),
        ]);
        let f = of_rule(&g, "provenance.unresolved");
        let targets: Vec<&str> = f.iter().filter_map(|x| x.target.as_deref()).collect();
        assert_eq!(f.len(), 2);
        assert!(targets.contains(&"[[Ghost]]")); // wikilink that resolves to nothing
        assert!(targets.contains(&"ghost-slug")); // slug that resolves to nothing
        assert!(!targets.iter().any(|t| t.contains("DDIA"))); // resolves -> no finding
    }

    #[test]
    fn provenance_reference_with_display_resolves() {
        // a based_on value carrying |display must be stripped like a body link before resolving
        let g = graph(&[
            ("DDIA.md", "body"),
            ("c.md", "---\nbased_on:\n  - \"[[DDIA|See DDIA]]\"\n---\n"),
        ]);
        assert!(of_rule(&g, "provenance.unresolved").is_empty());
    }
}
