# kura(蔵)

Koopa Obsidian vault 的 **read-only 知識圖守衛 / 索引器**。一次掃全語料 → 建 link graph +
symbol table → 跑語料層檢查 → 出 JSONL + 摘要。**read-only,不改檔。**

## 權威來源(開工前讀)

- **設計 spec**:`~/obsidian/System/vault-guard-spec.md`(§10-15 = v1 定稿;5-消費者 panel 收斂)。
- **enum 單一來源**:`~/obsidian/System/schemas/vault-schema.toml`(本工具載入它,**不自帶第二份 enum**)。
- **方向**:`~/obsidian/System/Koopa-Knowledge-Compiler.md`(Part II)。

## Rust 工程 harness:`rust-spec` 為 canonical

本專案 drop-in 套 `~/rust/github.com/koopa0/rust-spec` 的標準(rules / skills / clippy / rustfmt /
verify chain)。code 風格、error handling、testing、crate 選擇以 rust-spec 為準。本檔只記 kura 專屬。

## v1 範圍(精核,每多一條規則要有真實案例)

5 條規則:`link.title_not_alias`(killer)、`link.broken`(advisory 不 gate)、`collision.alias`(gate)、
`provenance.unresolved`、`map.disk_mismatch`(gate)。`coverage` 子命令出覆蓋率 + domain + symbol table +
缺口 + orphan。CLI = `check` + `coverage`。frontmatter 驗證留 vault `schema_lint.py`(Stage 3 才吸收)。

## 不變量(死守)

- **read-only**:不改檔/移動/改名/刪。報問題,人/Claude 決定。
- **resolver 永不 false-positive**(誤報「斷」永久毀信任 > 漏報):偏 false-negative;釘真 Obsidian
  config(filename+alias、大小寫不敏感、**NFC 正規化中文**、剝 `#`/`|`/`^`);heading/block 只看檔案存在。
- **output contract**:stable fingerprint(不綁行號)+ 確定性全序排序 + 結構化欄位(target/resolved_to/
  collision_members,不靠解析中文 prose);json mode **stdout 純 JSONL**、其餘進 stderr;退出碼 0/1/2。
- **stateless 全掃**,純 JSONL,不碰 SQLite/DB。enum 從 `vault-schema.toml` 載入,絕不 hardcode 第二份。

## Build / verify

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo nextest run    # 或 cargo test
cargo fmt --check
```

## 狀態

P0 scaffold 完成(lib + 薄 CLI,空殼端到端跑通)。P1 = parse + graph;P2 = 5 條規則 + output contract;
P3 = coverage + gate 整合。見 vault-guard-spec.md §14 build plan。
