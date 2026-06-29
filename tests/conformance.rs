//! Conformance fixtures that pin resolver fidelity to real Obsidian behavior, built from in-memory
//! markdown through the public API (no disk access). unwrap/expect on a known-present fixture is the
//! assertion itself, so that lint is relaxed in this file.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use kura::graph::Resolution;
use kura::{Graph, Note};

/// Build a graph from `(path, content)` pairs, with no non-md resources.
fn build_graph(notes: &[(&str, &str)]) -> Graph {
    Graph::build(
        notes
            .iter()
            .map(|(p, c)| Note::from_markdown(p, c))
            .collect(),
        &[],
    )
}

#[test]
fn title_is_not_a_resolution_key() {
    // Go Slice.md: title "Go Slice 內部結構" is not in aliases, so Obsidian silently fails to
    // resolve [[Go Slice 內部結構]]; kura must agree (this is the killer case).
    let g = build_graph(&[(
        "Concepts/golang/Go Slice.md",
        "---\ntitle: \"Go Slice 內部結構\"\naliases:\n  - Slice Header\n---\nbody\n",
    )]);
    let path = "Concepts/golang/Go Slice.md";
    assert_eq!(g.symbols.resolve("Go Slice"), Resolution::One(path)); // filename
    assert_eq!(g.symbols.resolve("Slice Header"), Resolution::One(path)); // alias
    assert_eq!(
        g.symbols.resolve("Go Slice 內部結構"),
        Resolution::Unresolved
    ); // title
}

#[test]
fn resolution_is_case_insensitive() {
    let g = build_graph(&[("a/Go Slice.md", "body")]);
    assert_eq!(
        g.symbols.resolve("go slice"),
        Resolution::One("a/Go Slice.md")
    );
    assert_eq!(
        g.symbols.resolve("GO SLICE"),
        Resolution::One("a/Go Slice.md")
    );
}

#[test]
fn resolution_is_nfc_insensitive() {
    // Alias stored decomposed (NFD), target composed (NFC) — must still resolve.
    let content = "---\naliases:\n  - cafe\u{0301}\n---\n"; // café decomposed
    let g = build_graph(&[("x.md", content)]);
    assert_eq!(g.symbols.resolve("caf\u{00e9}"), Resolution::One("x.md")); // composed
}

#[test]
fn duplicate_alias_is_ambiguous_not_guessed() {
    let g = build_graph(&[
        (
            "Concepts/golang/Go 連續記憶體與 CPU Cache.md",
            "---\naliases:\n  - Mechanical Sympathy\n---\n",
        ),
        (
            "Concepts/golang/薄抽象與硬體導向設計.md",
            "---\naliases:\n  - Mechanical Sympathy\n---\n",
        ),
    ]);
    match g.symbols.resolve("Mechanical Sympathy") {
        Resolution::Ambiguous(members) => assert_eq!(members.len(), 2),
        other => panic!("expected ambiguous, got {other:?}"),
    }
}

#[test]
fn same_filename_different_folder_is_ambiguous() {
    let g = build_graph(&[("golang/Foo.md", "body"), ("rust/Foo.md", "body")]);
    assert!(matches!(g.symbols.resolve("Foo"), Resolution::Ambiguous(_)));
}

#[test]
fn anchor_is_stripped_and_file_resolved() {
    let g = build_graph(&[
        ("Go Slice.md", "body"),
        (
            "note.md",
            "see [[Go Slice#Internals]] and [[Go Slice^abc]]\n",
        ),
    ]);
    let note = g.notes.iter().find(|n| n.path == "note.md").unwrap();
    assert_eq!(note.wikilinks.len(), 2);
    for link in &note.wikilinks {
        assert_eq!(link.target, "Go Slice");
        assert_eq!(
            g.symbols.resolve(&link.target),
            Resolution::One("Go Slice.md")
        );
    }
}

#[test]
fn non_md_resource_resolves_by_full_filename() {
    // Obsidian resolves [[X.canvas]] to a canvas on disk; a resolver that omits non-md files would
    // report a live link as broken. A missing canvas stays Unresolved (a true break).
    let notes = vec![Note::from_markdown(
        "Sources/DDIA.md",
        "see [[DDIA-Ch1-Overview.canvas|x]] and [[DDIA-Ch2.canvas]]\n",
    )];
    let resources = vec!["Diagrams/canvas/DDIA-Ch1-Overview.canvas".to_string()];
    let g = Graph::build(notes, &resources);
    assert_eq!(
        g.symbols.resolve("DDIA-Ch1-Overview.canvas"),
        Resolution::One("Diagrams/canvas/DDIA-Ch1-Overview.canvas"),
    );
    assert_eq!(g.symbols.resolve("DDIA-Ch2.canvas"), Resolution::Unresolved);
}

#[test]
fn path_qualified_link_resolves() {
    // Obsidian resolves [[folder/Note]] (and with .md) to that file; the resolver must too, or it
    // would falsely report a live path-qualified link broken.
    let g = build_graph(&[
        ("Concepts/golang/Go Slice.md", "body"),
        (
            "note.md",
            "[[Concepts/golang/Go Slice]] and [[Concepts/golang/Go Slice.md]]\n",
        ),
    ]);
    let p = "Concepts/golang/Go Slice.md";
    assert_eq!(
        g.symbols.resolve("Concepts/golang/Go Slice"),
        Resolution::One(p)
    );
    assert_eq!(
        g.symbols.resolve("Concepts/golang/Go Slice.md"),
        Resolution::One(p)
    );
    assert_eq!(g.symbols.resolve("Go Slice"), Resolution::One(p)); // bare name still works
}

#[test]
fn md_extension_link_resolves() {
    let g = build_graph(&[("Go Slice.md", "body")]);
    assert_eq!(
        g.symbols.resolve("Go Slice.md"),
        Resolution::One("Go Slice.md")
    );
    assert_eq!(
        g.symbols.resolve("Go Slice"),
        Resolution::One("Go Slice.md")
    );
}

#[test]
fn escaped_table_pipe_link_resolves() {
    // In a table cell Obsidian escapes the display pipe as `\|`; the target before it must resolve.
    let g = build_graph(&[
        ("Go Slice.md", "body"),
        ("t.md", "| h |\n| - |\n| [[Go Slice\\|see]] |\n"),
    ]);
    let note = g.notes.iter().find(|n| n.path == "t.md").unwrap();
    assert_eq!(note.wikilinks.len(), 1);
    assert_eq!(note.wikilinks[0].target, "Go Slice");
    assert_eq!(
        g.symbols.resolve("Go Slice"),
        Resolution::One("Go Slice.md")
    );
}

#[test]
fn jsonl_output_is_a_pure_contract() {
    let g = build_graph(&[
        (
            "Concepts/golang/Go Slice.md",
            "---\ntitle: \"Go Slice 內部結構\"\n---\nbody\n",
        ),
        ("note.md", "[[Go Slice 內部結構]] and [[Ghost]]\n"),
    ]);
    let mut report = kura::Report {
        findings: kura::rules::run(&g),
    };
    report.sort();
    let jsonl = report.to_jsonl().unwrap();
    assert!(!jsonl.is_empty());
    for line in jsonl.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        for key in [
            "rule_id",
            "severity",
            "path",
            "message",
            "evidence",
            "suggested_action",
            "source_rule",
            "fingerprint",
        ] {
            assert!(v.get(key).is_some(), "missing {key} in: {line}");
        }
        let sev = v["severity"].as_str().unwrap();
        assert!(
            ["info", "warn", "error"].contains(&sev),
            "bad severity: {sev}"
        );
    }
    // Deterministic: re-rendering the same report is byte-identical.
    assert_eq!(jsonl, report.to_jsonl().unwrap());
}

#[test]
fn wikilinks_in_code_are_skipped() {
    let body = "text [[Real]]\n```\n[[InCodeBlock]]\n```\ninline `[[InCodeSpan]]`\n";
    let note = Note::from_markdown("n.md", body);
    let targets: Vec<&str> = note.wikilinks.iter().map(|w| w.target.as_str()).collect();
    assert_eq!(targets, ["Real"]);
}

#[test]
fn wikilinks_in_obsidian_comments_are_skipped() {
    let note = Note::from_markdown("n.md", "%%[[Commented]]%% and [[Real]]\n");
    let targets: Vec<&str> = note.wikilinks.iter().map(|w| w.target.as_str()).collect();
    assert_eq!(targets, ["Real"]);
}

#[test]
fn pure_anchor_link_is_ignored() {
    let note = Note::from_markdown("n.md", "jump [[#Section]] here\n");
    assert!(note.wikilinks.is_empty());
}

#[test]
fn gap_heading_marks_links_planned() {
    let body = "## Body\n[[Solid]]\n\n## 缺口 / 待補\n[[Planned]]\n\n## 待整理\n[[Loose]]\n";
    let note = Note::from_markdown("n.md", body);
    let find = |t: &str| note.wikilinks.iter().find(|w| w.target == t).unwrap();
    assert!(!find("Solid").under_gap_heading);
    assert!(find("Planned").under_gap_heading);
    assert!(find("Loose").under_gap_heading); // 待整理 is also a gap marker
}

#[test]
fn wikilink_line_number_is_one_based_past_frontmatter() {
    let content = "---\ntitle: X\n---\nline four [[Target]]\n";
    let note = Note::from_markdown("n.md", content);
    assert_eq!(note.wikilinks[0].line, 4);
}

#[test]
fn vault_load_separates_notes_from_resources() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("kura-conformance-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Concepts/golang"))?;
    std::fs::create_dir_all(dir.join("Diagrams"))?;
    std::fs::write(
        dir.join("Concepts/golang/Go Slice.md"),
        "---\naliases:\n  - Slice Header\n---\nbody\n",
    )?;
    std::fs::write(dir.join("Diagrams/x.canvas"), "{}")?;
    std::fs::write(dir.join("README.txt"), "not markdown")?;
    let walk = kura::vault::load(&dir)?;
    std::fs::remove_dir_all(&dir)?;
    assert_eq!(walk.notes.len(), 1);
    assert_eq!(walk.notes[0].path, "Concepts/golang/Go Slice.md");
    assert!(walk.resources.iter().any(|r| r == "Diagrams/x.canvas"));
    assert!(walk.resources.iter().any(|r| r == "README.txt"));
    Ok(())
}

#[test]
fn collision_with_a_non_system_member_survives_default_scope()
-> Result<(), Box<dyn std::error::Error>> {
    // A collision between a System/ note and a knowledge note must not be dropped by the default
    // (System-excluded) scope just because its citing path sorts to the System/ member.
    let dir = std::env::temp_dir().join(format!("kura-scope-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("System"))?;
    std::fs::create_dir_all(dir.join("Writing"))?;
    let fm = "---\naliases:\n  - Shared\n---\n";
    std::fs::write(dir.join("System/A.md"), fm)?; // "System" sorts before "Writing"
    std::fs::write(dir.join("Writing/B.md"), fm)?;
    let report = kura::check(&dir, &[], false)?; // default scope excludes System/
    std::fs::remove_dir_all(&dir)?;
    let collisions = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "collision.alias")
        .count();
    assert_eq!(collisions, 1, "kept because Writing/B.md is in scope");
    Ok(())
}
