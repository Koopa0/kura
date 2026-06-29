# kura

Read-only knowledge-graph guardian and indexer for an Obsidian vault.

A *kura* (蔵) is a traditional Japanese storehouse that protects what it holds without altering it —
which is exactly this tool's contract. It scans the whole note corpus as a graph, reports what is
broken / orphaned / colliding, and **never modifies a file**.

It closes the gap between "every file's frontmatter is valid" (what a per-file linter checks) and
"the whole knowledge graph is sound" — the corpus-level checks that need a whole-vault symbol table
and link graph, which per-file lint and ad-hoc grep cannot do.

## Invariants

- **Read-only.** It reports problems; a human or agent decides what to change. It never edits,
  moves, renames, or deletes.
- **Never a false "broken".** A wrong "this link is broken" permanently destroys trust, so the
  resolver is biased to false-negative and pinned to Obsidian's real link behavior: it matches by
  filename and alias (never the title), case-insensitively, with NFC normalization for CJK, and
  resolves `[[X#heading]]` by file existence only.
- **Deterministic, stateless.** A full scan every time, pure JSONL out, no database. Output uses a
  stable fingerprint (not tied to line numbers) and a total ordering, so two scans diff cleanly.

## Highest-value check

`link.title_not_alias` — a `[[link]]` that matches a note's *title* when the title is not in that
note's `aliases`. Obsidian silently fails to resolve it while a per-file linter reports green; it is
invisible to the eye and to single-file checks. Only a whole-corpus symbol table catches it.

## Usage

```bash
kura check [PATH...] [--all] [--format json|human] [--deny <rule>] [--root <dir>]
kura coverage [--format json|human]
```

`check` builds the link graph from the entire `--root` tree; path arguments only filter which
findings are printed. Exit codes: `0` clean, `1` gate-hit, `2` tool-error.

## Build

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo nextest run    # or: cargo test
cargo fmt --check
```

## Status

Parsing and the link graph are in place: the walker, frontmatter and wikilink extraction, the symbol
table, and the Obsidian-faithful resolver, with conformance tests over the real vault's known cases.
Rule emission, the JSONL output contract, and `coverage` are next.

## License

MIT.
