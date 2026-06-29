//! Parse one markdown file into a [`Note`]: split frontmatter, read known YAML fields, extract
//! body wikilinks. Unknown or extra frontmatter keys are ignored here; frontmatter schema
//! validation lives elsewhere.

use yaml_rust2::{Yaml, YamlLoader};

use crate::model::Note;
use crate::wikilink;

impl Note {
    /// Build a [`Note`] from a vault-relative `path` and raw file `content`.
    #[must_use]
    pub fn from_markdown(path: &str, content: &str) -> Self {
        let (frontmatter, body, body_line) = split_frontmatter(content);
        let mut note = Note {
            path: path.to_owned(),
            title: None,
            aliases: Vec::new(),
            note_type: None,
            domain: None,
            status: None,
            topics: Vec::new(),
            slug: None,
            based_on: Vec::new(),
            related: Vec::new(),
            wikilinks: wikilink::extract(body, body_line),
            no_frontmatter: frontmatter.is_none(),
        };
        if let Some(fm) = frontmatter {
            if let Ok(docs) = YamlLoader::load_from_str(fm) {
                if let Some(doc) = docs.first() {
                    note.title = str_field(doc, "title");
                    note.aliases = list_field(doc, "aliases");
                    note.note_type = str_field(doc, "type");
                    note.domain = str_field(doc, "domain");
                    note.status = str_field(doc, "status");
                    note.topics = list_field(doc, "topics");
                    note.slug = str_field(doc, "slug");
                    note.based_on = list_field(doc, "based_on");
                    note.related = list_field(doc, "related");
                }
            }
            // Malformed YAML keeps the defaults; the resolver biases to false-negative, so a parse
            // error never invents a phantom broken link here.
        }
        note
    }
}

/// Split a leading `---`-fenced YAML frontmatter block.
///
/// Returns `(frontmatter_yaml, body, body_first_line_1based)`. Frontmatter is recognized only at
/// the very start of the file; without it the whole content is the body starting at line 1.
fn split_frontmatter(content: &str) -> (Option<&str>, &str, usize) {
    let Some(rest) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return (None, content, 1);
    };
    let mut offset = 0;
    let mut line = 1; // the opening `---`
    for raw in rest.split_inclusive('\n') {
        line += 1;
        let trimmed = raw.trim_end_matches(['\r', '\n']);
        if trimmed == "---" || trimmed == "..." {
            let yaml = &rest[..offset];
            let body = &rest[offset + raw.len()..];
            return (Some(yaml), body, line + 1);
        }
        offset += raw.len();
    }
    // No closing fence: treat as no frontmatter (conservative).
    (None, content, 1)
}

/// One scalar string field (missing or non-string -> `None`).
fn str_field(doc: &Yaml, key: &str) -> Option<String> {
    doc[key].as_str().map(str::to_owned)
}

/// One string-list field: block or flow list both work; a lone string is a single-element list.
fn list_field(doc: &Yaml, key: &str) -> Vec<String> {
    match &doc[key] {
        Yaml::Array(items) => items
            .iter()
            .filter_map(Yaml::as_str)
            .map(str::to_owned)
            .collect(),
        Yaml::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::split_frontmatter;

    #[test]
    fn no_frontmatter_yields_whole_body_at_line_one() {
        let (fm, body, line) = split_frontmatter("# hello\nbody\n");
        assert!(fm.is_none());
        assert_eq!(body, "# hello\nbody\n");
        assert_eq!(line, 1);
    }

    #[test]
    fn frontmatter_split_reports_body_start_line() {
        let (fm, body, line) = split_frontmatter("---\ntitle: X\n---\nbody\n");
        assert_eq!(fm, Some("title: X\n"));
        assert_eq!(body, "body\n");
        assert_eq!(line, 4);
    }

    #[test]
    fn unterminated_frontmatter_is_treated_as_body() {
        let (fm, _, line) = split_frontmatter("---\ntitle: X\nbody with no closing fence\n");
        assert!(fm.is_none());
        assert_eq!(line, 1);
    }
}
