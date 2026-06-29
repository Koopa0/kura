//! Load `vault-schema.toml` — the single source of truth for frontmatter enums, required and known
//! fields, and structural rules. kura carries no second copy; the schema checks read from here.

use std::path::Path;

use serde::Deserialize;

use crate::model::{Finding, FmValue, Note, Severity, fingerprint};
use crate::{Error, Result};

/// The machine-readable vault schema. Unknown sections (lifecycle, supersession, version) are
/// ignored — only what the frontmatter checks need is modeled.
#[derive(Debug, Deserialize)]
pub struct Schema {
    pub enums: Enums,
    pub fields: Fields,
    pub rules: Rules,
    pub scan: Scan,
}

#[derive(Debug, Deserialize)]
pub struct Enums {
    #[serde(rename = "type")]
    pub types: Vec<String>,
    pub domain: Vec<String>,
    pub source_kind: Vec<String>,
    pub source_provider: Vec<String>,
    pub level: Vec<String>,
    pub map_kind: Vec<String>,
    pub status: StatusEnums,
}

/// `status` is type-conditional, so it is grouped rather than one flat enum.
#[derive(Debug, Deserialize)]
pub struct StatusEnums {
    pub note: Vec<String>,
    pub system: Vec<String>,
    pub lesson: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Fields {
    pub required: Vec<String>,
    pub required_inbox: Vec<String>,
    pub known: Vec<String>,
    pub lesson_only: Vec<String>,
    pub status_group: StatusGroup,
}

#[derive(Debug, Deserialize)]
pub struct StatusGroup {
    /// Types whose status comes from `enums.status.system` (system / template / guide).
    pub system: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Rules {
    pub domain_equals_folder_under: Vec<String>,
    pub concept_requires_provenance: Vec<String>,
    pub slug_pattern: String,
    #[serde(default)]
    pub forbid_tag_with_slash: bool,
}

#[derive(Debug, Deserialize)]
pub struct Scan {
    pub knowledge_dirs: Vec<String>,
    #[serde(default)]
    pub skip_basenames: Vec<String>,
}

impl Schema {
    /// Load the schema from `<root>/System/schemas/vault-schema.toml`.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if the file cannot be read or [`Error::Schema`] if it does not parse.
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("System/schemas/vault-schema.toml");
        let text = std::fs::read_to_string(&path)?;
        toml::from_str(&text).map_err(|source| Error::Schema {
            path: path.display().to_string(),
            source,
        })
    }
}

/// Validate every knowledge note's frontmatter against the schema (the per-file checks absorbed from
/// schema_lint). Scope mirrors the schema's `knowledge_dirs` and `skip_basenames`. Findings are
/// `Error` — a schema violation gates.
#[must_use]
pub fn check(notes: &[Note], schema: &Schema) -> Vec<Finding> {
    let mut out = Vec::new();
    for note in notes {
        let seg: Vec<&str> = note.path.split('/').collect();
        let in_scope = schema.scan.knowledge_dirs.iter().any(|d| d == seg[0]);
        let skipped = schema
            .scan
            .skip_basenames
            .iter()
            .any(|b| Some(b.as_str()) == seg.last().copied());
        if in_scope && !skipped {
            lint_note(note, schema, &seg, &mut out);
        }
    }
    out
}

#[allow(clippy::too_many_lines)] // a faithful port of one cohesive validator
fn lint_note(note: &Note, schema: &Schema, seg: &[&str], out: &mut Vec<Finding>) {
    if note.no_frontmatter {
        return; // a note with no frontmatter is legal (a raw transcript)
    }
    let fm = &note.frontmatter;
    let scalar = |key: &str| {
        fm.get(key)
            .and_then(FmValue::as_scalar)
            .filter(|s| !s.is_empty())
    };
    let ty = scalar("type");

    if let Some(t) = ty {
        if !schema.enums.types.iter().any(|e| e == t) {
            out.push(finding(
                note,
                "schema.enum",
                Some("type"),
                t,
                "is not an allowed type",
            ));
        }
    }

    let is_lesson = ty == Some("lesson");
    for key in fm.keys() {
        let known = schema.fields.known.iter().any(|k| k == key)
            || (is_lesson && schema.fields.lesson_only.iter().any(|k| k == key));
        if !known {
            out.push(finding(
                note,
                "schema.unknown_key",
                None,
                key,
                "is not a known field",
            ));
        }
    }

    if is_lesson {
        match scalar("slug") {
            None => out.push(finding(
                note,
                "schema.required",
                Some("slug"),
                "",
                "is required for a lesson",
            )),
            Some(s) if !is_kebab(s) => {
                out.push(finding(
                    note,
                    "schema.slug",
                    Some("slug"),
                    s,
                    "is not a valid slug",
                ));
            }
            Some(_) => {}
        }
        if let Some(s) = scalar("status") {
            if !schema.enums.status.lesson.iter().any(|e| e == s) {
                out.push(finding(
                    note,
                    "schema.enum",
                    Some("status"),
                    s,
                    "is not a valid lesson status",
                ));
            }
        }
    }

    // System docs (system / template / guide): light rules only.
    if let Some(t) = ty {
        if schema.fields.status_group.system.iter().any(|s| s == t) {
            if let Some(s) = scalar("status") {
                if !schema.enums.status.system.iter().any(|e| e == s) {
                    out.push(finding(
                        note,
                        "schema.enum",
                        Some("status"),
                        s,
                        "is not a valid system status",
                    ));
                }
            }
            return;
        }
    }

    // Knowledge notes: full rules. inbox is undefined-shape, so domain is not required of it.
    let is_inbox = ty == Some("inbox");
    for key in &schema.fields.required {
        if is_inbox && key == "domain" {
            continue;
        }
        if !fm.get(key).is_some_and(FmValue::is_present) {
            out.push(finding(
                note,
                "schema.required",
                Some(key),
                "",
                "is required",
            ));
        }
    }
    if let Some(s) = scalar("status") {
        if !schema.enums.status.note.iter().any(|e| e == s) {
            out.push(finding(
                note,
                "schema.enum",
                Some("status"),
                s,
                "is not a valid status",
            ));
        }
    }
    for (field, allowed) in [
        ("domain", &schema.enums.domain),
        ("source_kind", &schema.enums.source_kind),
        ("source_provider", &schema.enums.source_provider),
        ("level", &schema.enums.level),
        ("map_kind", &schema.enums.map_kind),
    ] {
        if let Some(v) = scalar(field) {
            if !allowed.iter().any(|e| e == v) {
                out.push(finding(
                    note,
                    "schema.enum",
                    Some(field),
                    v,
                    "is not an allowed value",
                ));
            }
        }
    }
    // domain must equal the first folder under the configured roots (e.g. Concepts/<domain>/…).
    if let Some(d) = scalar("domain") {
        if schema
            .rules
            .domain_equals_folder_under
            .iter()
            .any(|r| r == seg[0])
            && seg.len() >= 3
            && d != seg[1]
        {
            out.push(finding(
                note,
                "schema.domain_folder",
                Some("domain"),
                d,
                &format!("does not match its folder {}", seg[1]),
            ));
        }
    }
    if schema.rules.forbid_tag_with_slash {
        if let Some(FmValue::List(tags)) = fm.get("tags") {
            for tag in tags {
                if tag.contains('/') {
                    out.push(finding(
                        note,
                        "schema.legacy_tag",
                        Some("tags"),
                        tag,
                        "is a legacy tag (use a property)",
                    ));
                }
            }
        }
    }
    if ty == Some("concept")
        && !schema
            .rules
            .concept_requires_provenance
            .iter()
            .any(|k| fm.get(k).is_some_and(FmValue::is_present))
    {
        out.push(finding(
            note,
            "schema.provenance",
            None,
            "",
            "concept has neither based_on nor source_locator",
        ));
    }
}

/// Whether `s` matches the immutable lesson-slug pattern `^[a-z0-9]+(-[a-z0-9]+)*$` (kebab-case).
fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn finding(note: &Note, rule_id: &str, field: Option<&str>, value: &str, why: &str) -> Finding {
    let what = field.unwrap_or("frontmatter");
    Finding {
        rule_id: rule_id.to_owned(),
        severity: Severity::Error,
        path: note.path.clone(),
        line: None,
        field: field.map(str::to_owned),
        message: if value.is_empty() {
            format!("{what} {why}")
        } else {
            format!("{what} \"{value}\" {why}")
        },
        evidence: "frontmatter validated against vault-schema.toml".to_owned(),
        suggested_action: "fix the frontmatter to match the schema".to_owned(),
        source_rule: "vault-schema.toml".to_owned(),
        target: (!value.is_empty()).then(|| value.to_owned()),
        resolved_to: None,
        collision_members: Vec::new(),
        fingerprint: fingerprint(rule_id, &note.path, value),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::Schema;

    const FIXTURE: &str = r#"
schema_version = "1"
[enums]
type = ["concept", "lesson", "system"]
domain = ["golang", "japanese"]
source_kind = ["course"]
source_provider = ["ardanlabs"]
level = ["fundamental"]
map_kind = ["topic"]
[enums.status]
note = ["seedling", "ready"]
system = ["active"]
lesson = ["draft", "ready"]
[fields]
required = ["title", "type", "domain"]
required_inbox = ["title", "type"]
known = ["title", "type", "domain", "status", "based_on", "source_locator", "tags"]
lesson_only = ["slug"]
[fields.status_group]
system = ["system", "guide"]
[rules]
domain_equals_folder_under = ["Concepts"]
concept_requires_provenance = ["based_on", "source_locator"]
slug_pattern = "^[a-z0-9]+(-[a-z0-9]+)*$"
forbid_tag_with_slash = true
[scan]
knowledge_dirs = ["Concepts", "Sources"]
skip_basenames = ["README.md"]
[[lifecycle]]
status = "seedling"
"#;

    #[test]
    fn loads_enums_fields_rules_and_ignores_unknown_sections() {
        let s: Schema = toml::from_str(FIXTURE).unwrap();
        assert!(s.enums.types.contains(&"concept".to_owned()));
        assert_eq!(s.enums.status.lesson, ["draft", "ready"]);
        assert_eq!(s.fields.status_group.system, ["system", "guide"]);
        assert_eq!(s.rules.domain_equals_folder_under, ["Concepts"]);
        assert!(s.rules.forbid_tag_with_slash);
        assert_eq!(s.scan.skip_basenames, ["README.md"]);
    }

    /// Each violation class fires once, on the right field and value — a portable lock on the
    /// behavior the live-vault run proved equivalent to schema_lint.py.
    #[test]
    fn schema_checks_match_each_violation_class() {
        use crate::model::Note;
        let schema: Schema = toml::from_str(FIXTURE).unwrap();
        let notes = [
            // bad type enum + unknown key
            Note::from_markdown(
                "Concepts/golang/A.md",
                "---\ntitle: X\ntype: widget\ndomain: golang\nstatus: seedling\nextra: 1\n---\n",
            ),
            // lesson slug fails the kebab pattern
            Note::from_markdown(
                "Concepts/golang/B.md",
                "---\ntitle: Y\ntype: lesson\ndomain: golang\nslug: Not Kebab\n---\n",
            ),
            // domain valid but != folder, and a concept with no provenance
            Note::from_markdown(
                "Concepts/japanese/C.md",
                "---\ntitle: Z\ntype: concept\ndomain: golang\nstatus: seedling\n---\n",
            ),
            // domain not in enum
            Note::from_markdown(
                "Sources/r/D.md",
                "---\ntitle: T\ntype: concept\ndomain: nim\nbased_on:\n  - \"[[Y]]\"\n---\n",
            ),
            // missing a required field (domain)
            Note::from_markdown(
                "Sources/r/E.md",
                "---\ntitle: E\ntype: concept\nbased_on:\n  - \"[[Z]]\"\n---\n",
            ),
            // skipped: README basename and out-of-scan System/
            Note::from_markdown("Concepts/golang/README.md", "---\ntype: nope\n---\n"),
            Note::from_markdown("System/x.md", "---\ntype: nope\n---\n"),
        ];
        let got: std::collections::BTreeSet<(String, String, String)> =
            super::check(&notes, &schema)
                .into_iter()
                .map(|f| {
                    (
                        f.rule_id,
                        f.field.unwrap_or_default(),
                        f.target.unwrap_or_default(),
                    )
                })
                .collect();
        let want: std::collections::BTreeSet<(String, String, String)> = [
            ("schema.enum", "type", "widget"),
            ("schema.unknown_key", "", "extra"),
            ("schema.slug", "slug", "Not Kebab"),
            ("schema.domain_folder", "domain", "golang"),
            ("schema.provenance", "", ""),
            ("schema.enum", "domain", "nim"),
            ("schema.required", "domain", ""),
        ]
        .into_iter()
        .map(|(r, f, t)| (r.to_owned(), f.to_owned(), t.to_owned()))
        .collect();
        assert_eq!(got, want);
    }
}
