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

## 工作紀律(只記 rust-spec 與 `.claude/rules` 未覆蓋的 delta)

> Verify(fmt/clippy/nextest/doc + L1/L2)→ `.claude/rules/development-lifecycle.md`;comprehend-first
> → `.claude/rules/agents.md`;commit 格式 → `.claude/rules/git-workflow.md`;debug → `/debug`;crate
> 選擇 → spec §7 + `rust-crate-registry`。**這些是 canonical,本節不重述,只補它們沒寫的。**

- **動手前先想、先講**:寫 code 前說假設(「我假設 X,錯的話 10 秒糾正」)、點出 tradeoff、>1 條路給
  2-3 選項+推薦。看不懂就停下問,別用「聽起來合理的 code」填補不確定。100% 不確定某 API/簽名存在,
  就說、就查(對抗 Knowledge Hallucination)。
- **外科手術式改動**:diff 最小化。沒被要求碰的別碰(別人的怪命名/typo/import 順序留著)。**match 既有
  file style**(引號、命名、排版),**絕不 reformat / 重排 import**。只清「你這次改動造成」的 dead code,
  既存死碼不歸你。每一行 changed 都要能連回「被要求做的事」——否則 revert。
- **別過度設計(convergent over expansionary)**:寫「現在這個問題」的最小正解,不是「理論上能解」的。
  - 一個 trait 只有一個 impl / 泛型只被一種型別實例化 → 直接用具體 struct(dead flexibility)。
  - 為自己上游已驗的值再加 `Option`/防呆 → 刪(speculative error handling;只處理真會發生的)。
  - 閾值/batch size 做成參數或 env 但只有一個呼叫者 → hardcode 到有第二個理由(unnecessary config)。
  - 「以後可能要…」不是需求,是對未來的猜。複製貼上兩次再抽象,wrong abstraction 比 duplication 貴。
- **failure modes 速查**(抓到自己在做就停):Kitchen Sink(順手重構半個庫)、Wrong Abstraction、
  Invisible Decision(默默定 schema/CLI/退出碼這類難回頭的選擇——要明講)、Optimistic Path(只寫 happy
  path,沒想壞 root/壞 UTF-8/空輸入)、Knowledge Hallucination、Style Drift、Runaway Refactor(改動
  級聯到失控 → 停、說明、取得同意再續)。

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
