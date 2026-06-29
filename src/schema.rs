//! Load `vault-schema.toml` — the single source of truth for frontmatter enums, required and known
//! fields, and structural rules. kura carries no second copy; the schema checks read from here.

use std::path::Path;

use serde::Deserialize;

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
known = ["title", "type", "domain", "status"]
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
}
