//! Pack findings into a scannable report. Both renderers — `human` (terminal, for a person) and
//! `markdown` (a fileable note body, for the consumer to store) — share one packing:
//!
//! 1. a per-domain debt scoreboard (triage: which area owes the most),
//! 2. a "most leveraged" callout (creating one target resolves many references),
//! 3. findings grouped by domain and sorted by blast radius (fan-in), with identical issues folded,
//! 4. a folded count of the tracked forward-references (planned concepts) and external paths, which
//!    are `Info` and hidden from the actionable body but surfaced as a count (markdown expands them).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::Report;
use crate::graph::normalize;
use crate::model::{Finding, Severity};

/// Infer a finding's domain from its vault path (the folder convention); `(other)` when the path
/// carries no knowledge domain (Sources, Maps, Writing/articles, …).
fn domain_of(path: &str) -> &str {
    for prefix in ["Concepts/", "Writing/lessons/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return rest.split('/').next().unwrap_or("(other)");
        }
    }
    "(other)"
}

/// One folded group of identical findings (same rule and target) within a domain.
struct Item {
    severity: Severity,
    message: String,
    blast: usize,
    sample_path: String,
}

/// One high-leverage target: creating it resolves `count` references.
struct Leverage {
    target: String,
    count: usize,
    planned: bool,
    domains: Vec<String>,
}

/// The packed report, ready to render.
struct Packed {
    errors: usize,
    warns: usize,
    scoreboard: Vec<(String, usize, usize)>, // domain, errors, warns (sorted by debt)
    leverage: Vec<Leverage>,                 // top targets by fan-in
    domains: Vec<(String, Vec<Item>)>,       // domain -> blast-sorted folded items
    planned: usize,                          // tracked forward-references (info)
    external: usize,                         // external paths (info)
}

fn pack(report: &Report) -> Packed {
    let findings = &report.findings;
    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warns = findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();

    // Scoreboard: error/warn counts per domain.
    let mut board: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for f in findings {
        let slot = board.entry(domain_of(&f.path)).or_default();
        match f.severity {
            Severity::Error => slot.0 += 1,
            Severity::Warn => slot.1 += 1,
            Severity::Info => {}
        }
    }
    let mut scoreboard: Vec<(String, usize, usize)> = board
        .into_iter()
        .filter(|(_, (e, w))| *e + *w > 0)
        .map(|(d, (e, w))| (d.to_owned(), e, w))
        .collect();
    scoreboard.sort_by(|a, b| {
        (b.1 + b.2)
            .cmp(&(a.1 + a.2))
            .then(b.1.cmp(&a.1))
            .then(a.0.cmp(&b.0))
    });

    // Every Info finding is "hidden"; external paths are one slice, the rest are forward-references.
    // Deriving planned from the total (not a second rule filter) keeps planned + external == hidden,
    // so the count line can never under-report a hidden finding (e.g. a planned Direction-A link).
    let external = findings
        .iter()
        .filter(|f| f.rule_id == "link.broken.path" && f.severity == Severity::Info)
        .count();
    let hidden = findings.len() - errors - warns;
    Packed {
        errors,
        warns,
        scoreboard,
        leverage: leverage(findings),
        domains: domain_sections(findings),
        planned: hidden - external,
        external,
    }
}

/// Targets whose creation resolves more than one reference — including planned concepts (a broken
/// link is broken regardless of severity, so this aggregates every link finding by target).
fn leverage(findings: &[Finding]) -> Vec<Leverage> {
    let mut by_target: BTreeMap<String, (String, usize, bool, Vec<String>)> = BTreeMap::new();
    for f in findings {
        if f.rule_id != "link.broken" && f.rule_id != "link.title_not_alias" {
            continue;
        }
        let Some(target) = &f.target else { continue };
        let entry = by_target
            .entry(normalize(target))
            .or_insert_with(|| (target.clone(), 0, true, Vec::new()));
        entry.1 += 1;
        entry.2 &= f.severity == Severity::Info; // planned only if every reference is planned
        let domain = domain_of(&f.path).to_owned();
        if !entry.3.contains(&domain) {
            entry.3.push(domain);
        }
    }
    let mut out: Vec<Leverage> = by_target
        .into_values()
        .filter(|(_, count, _, _)| *count > 1)
        .map(|(target, count, planned, mut domains)| {
            domains.sort();
            Leverage {
                target,
                count,
                planned,
                domains,
            }
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.target.cmp(&b.target)));
    out
}

/// Actionable findings (`Warn` + `Error`) grouped by domain, identical issues folded, each domain
/// sorted error-first then by blast radius.
fn domain_sections(findings: &[Finding]) -> Vec<(String, Vec<Item>)> {
    // key (domain \x1f rule \x1f target) -> folded item carrying its domain.
    let mut groups: BTreeMap<String, (String, Item)> = BTreeMap::new();
    for f in findings.iter().filter(|f| f.severity != Severity::Info) {
        let domain = domain_of(&f.path).to_owned();
        let target = f.target.as_deref().unwrap_or(&f.path);
        let key = format!("{domain}\u{1f}{}\u{1f}{target}", f.rule_id);
        let blast = f.collision_members.len().max(1);
        let entry = groups.entry(key).or_insert_with(|| {
            (
                domain.clone(),
                Item {
                    severity: f.severity,
                    message: f.message.clone(),
                    blast: 0,
                    sample_path: f.path.clone(),
                },
            )
        });
        entry.1.blast += blast;
    }
    let mut by_domain: BTreeMap<String, Vec<Item>> = BTreeMap::new();
    for (_, (domain, item)) in groups {
        by_domain.entry(domain).or_default().push(item);
    }
    let mut out: Vec<(String, Vec<Item>)> = by_domain.into_iter().collect();
    // domains ordered by total blast (most debt first)
    out.sort_by(|a, b| {
        let sum = |items: &[Item]| items.iter().map(|i| i.blast).sum::<usize>();
        sum(&b.1).cmp(&sum(&a.1)).then(a.0.cmp(&b.0))
    });
    for (_, items) in &mut out {
        items.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then(b.blast.cmp(&a.blast))
                .then(a.message.cmp(&b.message))
        });
    }
    out
}

/// Render the packed report for a terminal reader.
#[must_use]
pub fn human(report: &Report) -> String {
    let p = pack(report);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{} findings: {} error, {} warn, {} hidden ({} planned forward-refs, {} external paths)",
        report.findings.len(),
        p.errors,
        p.warns,
        p.planned + p.external,
        p.planned,
        p.external,
    );

    if !p.scoreboard.is_empty() {
        let _ = writeln!(s, "\ndebt by domain:");
        for (domain, e, w) in &p.scoreboard {
            let _ = writeln!(s, "  {domain:<20} {e} error · {w} warn");
        }
    }

    if !p.leverage.is_empty() {
        let _ = writeln!(s, "\nmost leveraged (create one, resolve many):");
        for l in p.leverage.iter().take(5) {
            let tag = if l.planned { "planned" } else { "broken" };
            let _ = writeln!(
                s,
                "  ×{} [[{}]] ({tag}) — {}",
                l.count,
                l.target,
                l.domains.join(", ")
            );
        }
    }

    for (domain, items) in &p.domains {
        let _ = writeln!(s, "\n▌ {domain}");
        for i in items {
            let n = if i.blast > 1 {
                format!("×{} ", i.blast)
            } else {
                String::new()
            };
            let _ = writeln!(
                s,
                "  [{}] {n}{}  ({})",
                i.severity, i.message, i.sample_path
            );
        }
    }
    s
}

/// Neutralize characters that would break a markdown table (`|`), HTML/`<details>` (`<`, `>`), or
/// turn the report's own text into live wikilinks that pollute the graph (`[[`, `]]`).
fn escape_md(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace("[[", "[\\[")
        .replace("]]", "]\\]")
}

/// Render the packed report as a fileable markdown note body (still printed to stdout — kura never
/// writes files). Frontmatter is deterministic (no timestamp); the consumer stamps and routes it.
#[must_use]
pub fn markdown(report: &Report) -> String {
    let p = pack(report);
    let mut s = String::from("---\ntype: report\ntool: kura\n---\n\n# kura check\n\n");
    let _ = writeln!(
        s,
        "{} findings — **{} error**, **{} warn**, {} hidden.\n",
        report.findings.len(),
        p.errors,
        p.warns,
        p.planned + p.external,
    );

    if !p.scoreboard.is_empty() {
        let _ = writeln!(
            s,
            "## Debt by domain\n\n| domain | error | warn |\n|---|--:|--:|"
        );
        for (domain, e, w) in &p.scoreboard {
            let _ = writeln!(s, "| {} | {e} | {w} |", escape_md(domain));
        }
        s.push('\n');
    }

    if !p.leverage.is_empty() {
        let _ = writeln!(s, "## Most leveraged (create one, resolve many)\n");
        for l in p.leverage.iter().take(5) {
            let tag = if l.planned { "planned" } else { "broken" };
            // target stays in a code span (a literal `[[X]]`, not a live wikilink)
            let _ = writeln!(
                s,
                "- **×{}** `[[{}]]` ({tag}) — {}",
                l.count,
                l.target,
                escape_md(&l.domains.join(", "))
            );
        }
        s.push('\n');
    }

    for (domain, items) in &p.domains {
        let _ = writeln!(s, "## {}\n", escape_md(domain));
        for i in items {
            let n = if i.blast > 1 {
                format!("×{} ", i.blast)
            } else {
                String::new()
            };
            let _ = writeln!(
                s,
                "- `{}` {n}{} — {}",
                i.severity,
                escape_md(&i.message),
                escape_md(&i.sample_path)
            );
        }
        s.push('\n');
    }

    if p.planned + p.external > 0 {
        let _ = writeln!(
            s,
            "<details><summary>{} tracked forward-references · {} external paths (info)</summary>\n",
            p.planned, p.external,
        );
        for f in report
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
        {
            let _ = writeln!(
                s,
                "- `{}` {} — {}",
                f.rule_id,
                escape_md(&f.message),
                escape_md(&f.path)
            );
        }
        let _ = writeln!(s, "\n</details>");
    }
    s
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use crate::Report;
    use crate::model::{Finding, Severity, fingerprint};

    fn link(rule: &str, sev: Severity, path: &str, target: &str) -> Finding {
        Finding {
            rule_id: rule.to_owned(),
            severity: sev,
            path: path.to_owned(),
            line: Some(1),
            field: None,
            message: format!("[[{target}]] resolves to no note"),
            evidence: String::new(),
            suggested_action: String::new(),
            source_rule: String::new(),
            target: Some(target.to_owned()),
            resolved_to: None,
            collision_members: Vec::new(),
            fingerprint: fingerprint(rule, path, target),
        }
    }

    #[test]
    fn domain_inferred_from_path() {
        assert_eq!(super::domain_of("Concepts/golang/X.md"), "golang");
        assert_eq!(super::domain_of("Writing/lessons/rust/Y.md"), "rust");
        assert_eq!(super::domain_of("Sources/oreilly/DDIA.md"), "(other)");
    }

    #[test]
    fn leverage_folds_a_target_cited_across_notes() {
        // [[Foo]] cited by two golang notes (warn) -> leverage ×2; a single ref is not leveraged.
        let report = Report {
            findings: vec![
                link("link.broken", Severity::Warn, "Concepts/golang/A.md", "Foo"),
                link("link.broken", Severity::Warn, "Concepts/golang/B.md", "Foo"),
                link("link.broken", Severity::Warn, "Concepts/golang/C.md", "Bar"),
            ],
        };
        let out = super::human(&report);
        assert!(out.contains("×2 [[Foo]] (broken)"), "{out}");
        assert!(
            !out.contains("[[Bar]] ("),
            "single-ref target is not in the leverage callout"
        );
    }

    #[test]
    fn hidden_count_includes_every_info_not_just_link_broken() {
        // a planned Direction-A link (map.disk_mismatch, Info) must be counted as hidden, not lost.
        let report = Report {
            findings: vec![
                link("map.disk_mismatch", Severity::Info, "Maps/x.md", "Lesson"),
                link(
                    "link.broken.path",
                    Severity::Info,
                    "Writing/lessons/golang/a.md",
                    "exam/x.md",
                ),
                link(
                    "link.broken",
                    Severity::Warn,
                    "Concepts/golang/b.md",
                    "Real",
                ),
            ],
        };
        let out = super::human(&report);
        assert!(out.contains("2 hidden"), "{out}");
        assert!(out.contains("1 planned forward-refs"), "{out}"); // the map.disk_mismatch info
        assert!(out.contains("1 external paths"), "{out}");
    }

    #[test]
    fn markdown_escapes_table_and_link_breaking_chars() {
        let mut f = link(
            "collision.alias",
            Severity::Warn,
            "Concepts/golang/a.md",
            "x",
        );
        f.message = "alias has a | pipe, a <tag>, and [[Wiki]]".to_owned();
        let md = super::markdown(&Report { findings: vec![f] });
        assert!(!md.contains("a | pipe"), "a raw pipe breaks a table: {md}");
        assert!(md.contains("\\|") && md.contains("&lt;tag&gt;"));
        assert!(
            !md.contains("[[Wiki]]"),
            "the report must not emit a live wikilink: {md}"
        );
    }

    #[test]
    fn planned_forward_refs_are_counted_not_listed_in_body() {
        let report = Report {
            findings: vec![
                link(
                    "link.broken",
                    Severity::Info,
                    "Concepts/golang/A.md",
                    "Planned",
                ),
                link(
                    "link.broken",
                    Severity::Warn,
                    "Concepts/golang/B.md",
                    "RealBug",
                ),
            ],
        };
        let out = super::human(&report);
        assert!(out.contains("1 planned forward-refs"));
        // the info finding is folded into the count, not printed as an actionable line
        assert!(!out.contains("[[Planned]] resolves"), "{out}");
        assert!(out.contains("[[RealBug]] resolves"));
    }
}
