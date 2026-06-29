# kura

A read-only knowledge-graph guardian and indexer for an Obsidian vault. It scans the whole note
corpus once, builds a link graph and a symbol table, reports what is broken / duplicated / orphaned /
drifting — and **never modifies a file**.

## The problem

A vault that agents and humans both write into rots in ways that are invisible to per-file checks and
to the naked eye:

- **Silent broken links.** A link written with a note's *title* when that title is not the note's
  filename and not one of its aliases: Obsidian fails to resolve it and shows it broken, but a
  per-file frontmatter linter reports green and the link *looks* fine in the source. It just quietly
  dies. (Worse: some tools index the title as a resolution key and report it *resolved*, hiding the
  break entirely.)
- **Duplicate-before-write.** An agent is about to create a note on a concept that already exists
  under an alias. A full-text search returns nothing, so it writes a second copy. "Zero search hits"
  is not "doesn't exist".
- **Alias collisions.** Two notes declare the same alias; the wikilink for it resolves to only one of
  them, and the other silently loses every inbound link.
- **Provenance rot.** A note's `based_on` / `related` points at a note that was renamed or never
  written.
- **Map ↔ disk drift.** A syllabus or index lists a note that isn't on disk, or a note exists on disk
  that the index never lists.

None of these are visible to a linter that sees one file at a time. Catching them needs a whole-vault
symbol table and link graph — which is what kura is.

## Use cases

- **Pre-merge / CI gate.** An agent (or a human) writes a batch of notes on a branch; kura runs
  before merge and fails only on damage the branch *newly introduced*, so the gate stays trustworthy
  instead of drowning in pre-existing noise.
- **Dedup-before-write oracle.** Before an agent writes a new concept, kura answers deterministically
  whether a note for that name — by filename, alias, or title — already exists, so it never creates a
  duplicate on a false "not found".
- **Coverage and curation.** Which domains and concepts exist, what is orphaned (in no map), and what
  is a tracked gap versus a real break.
- **Faithful link audit.** A resolver that matches Obsidian's real behavior, so its findings can be
  trusted as a gate rather than second-guessed.

It is built to be called by tools — an agent runner, CI — that read the JSONL output, and by a human
who reads the summary.

## Invariants

- **Read-only.** It reports problems; a human or agent decides what to change. It never edits, moves,
  renames, or deletes.
- **Never a false "broken".** A wrong "this link is broken" permanently destroys trust, so the
  resolver is biased to false-negative and pinned to Obsidian's real link behavior: it matches by
  filename and alias (never the title), case-insensitively, with NFC normalization for CJK, indexes
  non-markdown link targets (canvas, attachments) so a live `[[file.canvas]]` never reads as broken,
  and resolves `[[note#heading]]` by file existence only.
- **Deterministic and stateless.** A full scan every time, pure JSONL out, no database. Every finding
  carries a stable fingerprint (not tied to line numbers) under a total ordering, so two scans diff
  cleanly and a consumer can see exactly what a branch changed.

## Usage

```bash
kura check [PATH...] [--all] [--format json|human] [--deny <rule>] [--baseline <prev.jsonl>] [--root <dir>]
kura coverage [--format json|human] [--root <dir>]
kura exists <name> [--format json|human] [--root <dir>]
```

`check` always builds the link graph from the entire `--root` tree; path arguments only filter which
findings are printed. Output is pure JSONL on stdout in json mode (everything else on stderr) so it
pipes cleanly. `--deny <rule>` (repeatable) fails the run when a finding for that rule exists;
`--baseline` reads a prior run's JSONL and reports/gates only on findings this run newly introduced.
Exit codes: `0` clean, `1` gate-hit, `2` tool-error.

`coverage` classifies each concept as mounted (on a map), pending-mount (in the corpus but not yet
mapped), or orphan (nothing links it). `exists` answers whether a note for a name already exists —
by filename, title, alias, or English title — exiting `0` if it does and `1` if not, for a dedup
check before writing.

## Build

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo nextest run    # or: cargo test
cargo fmt --check
```

## Status

The full v1 surface is in place, with conformance tests over real cases: the Obsidian-faithful
resolver; the five checks (`link.title_not_alias`, `link.broken`, `collision.alias`,
`provenance.unresolved`, `map.disk_mismatch`); the JSONL output contract with stable fingerprints,
deterministic ordering, scope, and delta gating; and the `coverage` and `exists` commands. Absorbing
the per-file frontmatter linter, supersession tracking, and an MCP server are future work.

## License

MIT.
