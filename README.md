# kura 蔵

Read-only knowledge-graph guardian / indexer for the Koopa Obsidian vault.

A 蔵 (kura) is a traditional Japanese storehouse that protects what it holds without
altering it — which is exactly this tool's contract: it scans the whole note corpus as a
graph, reports what's broken / orphaned / colliding, and **never modifies a file**.

It closes the gap between "every file's frontmatter is valid" (what the vault's per-file
`schema_lint.py` checks) and "the whole knowledge graph is sound" — the corpus-level checks
that need a whole-vault symbol table and link graph, which per-file lint and ad-hoc grep cannot do.

## Highest-value checks

- **`link.title_not_alias`** — a `[[link]]` that matches a note's *title* but the title isn't in
  that note's `aliases`, so Obsidian silently fails to resolve it while `schema_lint` reports green.
  Invisible to the eye and to per-file lint; only a whole-corpus symbol table catches it.
- **symbol-table recall** (`coverage`) — a deterministic, alias-aware "does a note on X already exist?"
  oracle, so an agent can trust "not written yet" before creating a duplicate.

## Usage

```bash
kura check [PATH...] [--all] [--format json|human] [--deny error] [--root <dir>]
kura coverage [--format json|human]
```

Design & rationale: `~/obsidian/System/vault-guard-spec.md`. Schema source of truth:
`~/obsidian/System/schemas/vault-schema.toml`. Rust harness: `rust-spec`.

Status: **P0 scaffold** (lib + thin CLI, builds and runs end-to-end). P1 parse+graph → P2 rules →
P3 coverage. See vault-guard-spec.md §14.
