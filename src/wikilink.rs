//! Extract `[[wikilink]]`s from a markdown body via `pulldown-cmark`.
//!
//! Wikilinks are not CommonMark syntax, so the parser is used only to locate code spans/blocks
//! (to skip) and headings (for gap-section context); the `[[...]]` targets are scanned from the
//! raw text so the brackets are never mangled by link parsing. Obsidian `%%...%%` comments are
//! excluded from the link graph, so a `[[X]]` inside them is not a live link and is skipped too.
//! Targets keep their original case and Unicode; the resolver normalizes at lookup time.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::model::WikiLink;

/// A heading whose text contains one of these marks the following section as planned: wikilinks
/// under it are forward-references, not broken links.
const GAP_MARKERS: [&str; 5] = ["缺口", "待補", "待寫", "待整理", "待建"];

/// Extract every `[[target]]` in `body`, skipping code and comment zones, with 1-based line
/// numbers (offset by `body_start_line`) and gap-section context.
#[must_use]
pub fn extract(body: &str, body_start_line: usize) -> Vec<WikiLink> {
    let (mut skip_zones, headings) = structure(body);
    let comments = comment_zones(body, &skip_zones);
    skip_zones.extend(comments);
    let mut links = Vec::new();
    for (offset, inner) in raw_wikilinks(body) {
        if skip_zones.iter().any(|z| z.contains(&offset)) {
            continue;
        }
        let Some(target) = strip_target(inner) else {
            continue; // a bare anchor like [[#heading]] — same-file jump
        };
        links.push(WikiLink {
            target,
            line: body_start_line + body[..offset].bytes().filter(|&b| b == b'\n').count(),
            under_gap_heading: in_gap_section(&headings, offset),
        });
    }
    links
}

/// A heading's parsed facts: start byte offset, level (used only for relative ordering), and
/// whether its text contains a gap marker.
struct Heading {
    start: usize,
    level: usize,
    gap: bool,
}

/// Code span/block byte ranges (to skip) and the headings, in document order.
fn structure(body: &str) -> (Vec<Range<usize>>, Vec<Heading>) {
    let mut code_zones = Vec::new();
    let mut headings = Vec::new();
    let mut code_block_start = None;
    let mut heading: Option<(usize, usize, String)> = None;
    for (event, range) in Parser::new_ext(body, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_block_start.take() {
                    code_zones.push(start..range.end);
                }
            }
            Event::Code(_) => code_zones.push(range),
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((range.start, level as usize, String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((start, level, text)) = heading.take() {
                    let gap = GAP_MARKERS.iter().any(|m| text.contains(m));
                    headings.push(Heading { start, level, gap });
                }
            }
            Event::Text(t) => {
                if let Some((_, _, text)) = heading.as_mut() {
                    text.push_str(&t);
                }
            }
            _ => {}
        }
    }
    (code_zones, headings)
}

/// Byte ranges of Obsidian `%%...%%` comments (inline and multi-line). `%%` inside code is ignored
/// first (a stray `%%` in a code sample must not shift the pairing of real comments); the remaining
/// `%%` are paired in order and an unpaired trailing one is dropped.
fn comment_zones(body: &str, code_zones: &[Range<usize>]) -> Vec<Range<usize>> {
    let marks: Vec<usize> = body
        .match_indices("%%")
        .map(|(i, _)| i)
        .filter(|i| !code_zones.iter().any(|z| z.contains(i)))
        .collect();
    marks.chunks_exact(2).map(|p| p[0]..p[1] + 2).collect()
}

/// Raw scan for `[[...]]` pairs: `(byte offset of the [[, inner text)`. The inner text must not
/// span a newline (wikilinks are single-line).
fn raw_wikilinks(body: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = body[i..].find("[[") {
        let open = i + rel;
        let after = open + 2;
        let Some(rel_end) = body[after..].find("]]") else {
            break;
        };
        let inner = &body[after..after + rel_end];
        if !inner.contains('\n') {
            out.push((open, inner));
        }
        i = after + rel_end + 2;
    }
    out
}

/// Strip `|display`, `#heading`, `^block` from a wikilink's inner text, leaving the note-name
/// target. `None` if nothing remains (a same-file anchor link). Shared with provenance resolution so
/// frontmatter references are stripped identically to body links.
pub(crate) fn strip_target(inner: &str) -> Option<String> {
    // A wikilink in a table cell escapes the display pipe as `\|`; drop the trailing escape backslash.
    let before_display = inner
        .split('|')
        .next()
        .unwrap_or(inner)
        .trim_end_matches('\\');
    let before_heading = before_display.split('#').next().unwrap_or(before_display);
    let target = before_heading
        .split('^')
        .next()
        .unwrap_or(before_heading)
        .trim();
    (!target.is_empty()).then(|| target.to_owned())
}

/// Whether `offset` falls in a section opened by a gap heading and not yet closed by a heading at
/// the same or a higher level.
fn in_gap_section(headings: &[Heading], offset: usize) -> bool {
    let mut gap_level: Option<usize> = None;
    for h in headings {
        if h.start > offset {
            break;
        }
        if gap_level.is_some_and(|g| h.level <= g) {
            gap_level = None;
        }
        if h.gap {
            gap_level = Some(h.level);
        }
    }
    gap_level.is_some()
}

#[cfg(test)]
mod tests {
    // unwrap on a known-present fixture is the assertion itself.
    #![allow(clippy::unwrap_used)]

    use super::{comment_zones, raw_wikilinks, strip_target};

    #[test]
    fn strips_display_heading_and_block() {
        assert_eq!(strip_target("X|disp").as_deref(), Some("X"));
        assert_eq!(strip_target("X#Heading").as_deref(), Some("X"));
        assert_eq!(strip_target("X^block").as_deref(), Some("X"));
        assert_eq!(strip_target("X#H|disp").as_deref(), Some("X"));
        // A table cell escapes the display pipe as `\|`.
        assert_eq!(strip_target("X\\|disp").as_deref(), Some("X"));
    }

    #[test]
    fn pure_anchor_strips_to_none() {
        assert_eq!(strip_target("#Heading"), None);
    }

    #[test]
    fn raw_scan_ignores_unterminated_and_multiline() {
        assert_eq!(raw_wikilinks("[[a]] [[b]]"), vec![(0, "a"), (6, "b")]);
        assert_eq!(raw_wikilinks("[[open with no close"), vec![]);
        assert_eq!(raw_wikilinks("[[line one\nline two]]"), vec![]);
    }

    #[test]
    fn comment_zones_pair_double_percent() {
        let zones = comment_zones("a %%c%% b", &[]);
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0], 2..7);
        assert!(comment_zones("none here", &[]).is_empty());
    }

    #[test]
    fn double_percent_inside_code_does_not_shift_pairing() {
        // A stray `%%` inside a code zone must be ignored so it cannot mis-pair a real comment.
        let body = "x %% y %%[[Real]]%%";
        // mark the first `%%` as inside a code zone
        let code: Vec<std::ops::Range<usize>> = std::iter::once(2usize..4).collect();
        let zones = comment_zones(body, &code);
        assert_eq!(zones.len(), 1);
        // the surviving pair wraps the real comment, not the stray
        assert!(zones[0].contains(&body.find("[[Real]]").unwrap()));
    }
}
