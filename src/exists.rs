//! A deterministic "does a note for this name already exist?" oracle, for dedup-before-write.
//!
//! Deliberately wider than the resolver: a writer searches by filename, title, alias, or English
//! title, so all four are matched (the resolver excludes the title because Obsidian does not resolve
//! by it — but a human asking "have I written this?" thinks in titles). Each hit reports which field
//! matched, so a caller can tell a real duplicate from a merely-similar name. The error-cost is the
//! opposite of the resolver's: a false "no" makes an agent write a duplicate, so this over-recalls.

use crate::graph::{Graph, filename_stem, normalize};

/// One note that exposes the queried name, and the field it matched on.
#[derive(Debug, serde::Serialize)]
pub struct Match {
    pub path: String,
    pub field: &'static str,
    pub value: String,
}

/// The result of an existence query.
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub query: String,
    pub matches: Vec<Match>,
}

impl Report {
    /// Whether any note exposes the queried name.
    #[must_use]
    pub fn found(&self) -> bool {
        !self.matches.is_empty()
    }
}

/// Look `query` up across every note's filename, title, aliases, and English title.
#[must_use]
pub fn lookup(graph: &Graph, query: &str) -> Report {
    let key = normalize(query);
    let mut matches = Vec::new();
    for note in &graph.notes {
        let stem = filename_stem(&note.path);
        if normalize(stem) == key {
            matches.push(hit(note, "filename", stem));
        }
        if let Some(title) = &note.title {
            if normalize(title) == key {
                matches.push(hit(note, "title", title));
            }
        }
        for alias in &note.aliases {
            if normalize(alias) == key {
                matches.push(hit(note, "alias", alias));
            }
        }
        if let Some(title_en) = &note.title_en {
            if normalize(title_en) == key {
                matches.push(hit(note, "title_en", title_en));
            }
        }
    }
    matches.sort_by(|a, b| (a.path.as_str(), a.field).cmp(&(b.path.as_str(), b.field)));
    Report {
        query: query.to_owned(),
        matches,
    }
}

fn hit(note: &crate::model::Note, field: &'static str, value: &str) -> Match {
    Match {
        path: note.path.clone(),
        field,
        value: value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
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
    fn finds_by_title_even_though_resolver_would_not() {
        // The resolver excludes the title, but exists must find a note by its title.
        let g = graph(&[(
            "Concepts/golang/Go Slice.md",
            "---\ntitle: \"Go Slice 內部結構\"\naliases:\n  - Slice Header\n---\n",
        )]);
        let r = super::lookup(&g, "Go Slice 內部結構");
        assert!(r.found());
        assert_eq!(r.matches[0].field, "title");
        // and by filename, alias, case-insensitively
        assert_eq!(super::lookup(&g, "go slice").matches[0].field, "filename");
        assert_eq!(super::lookup(&g, "Slice Header").matches[0].field, "alias");
    }

    #[test]
    fn reports_not_found_for_an_unwritten_name() {
        let g = graph(&[("a.md", "body")]);
        assert!(!super::lookup(&g, "Never Written").found());
    }
}
