# kura — 開發接手(NEXT)

> 給接手 kura 的 session。P0 scaffold 已完成(build/clippy/run 綠,commit `b91f01d`)。
> 此檔 = 完整工具定位 + 情境 + P1 kickoff。權威設計在 `~/obsidian/System/vault-guard-spec.md`(§10-15),
> 由 Claude 在 obsidian session 2026-06-28 起草,情境均 grounded 於真實 vault 案例(已實測)。
> Rust 工程紀律以 `~/rust/github.com/koopa0/rust-spec` 為準(harness 裝好後 active)。

## kura 是什麼:agent 的 vault 工具箱,不只一個 linter

kura 讀 Obsidian vault(`~/obsidian`),一次掃全語料 → 自己抽取(frontmatter + wikilink)→ 建
**link graph + symbol table** → 在這之上提供多個命令。它是 read-only,不改任何檔。

它的價值地基是**抽取層 + symbol table + graph**。一旦建好,它支撐的不只 gate,是一整套 agent / 人會用的
vault 命令。設計時把每個命令都當「第一公民」,不是把 check 做完才想其他——抽取層要為全部命令服務。

| 命令 | 誰用 | 解決什麼 | 靠哪層 |
|---|---|---|---|
| `kura check [PATH...]` | QA gate / hermes 批次自檢 / pre-merge | 抓 branch 新引入的語料層損壞(斷鏈/撞名/provenance/map↔disk) | 抽取+解析+檢查 |
| `kura coverage` | 你 curation / MOC 維護 | 生成覆蓋率 + domain 清單(取代手數),校對 Vault-Index 漏的 domain | 抽取+symbol table |
| `kura exists <X>` | 檢索 agent 寫新概念前(dedup-before-write) | 確定性回答「X 寫過沒、在哪」——比 search_knowledge 可信(它「零命中不等於沒寫過」) | symbol table |
| `kura backlinks <note>` | 改寫前看 blast radius / 找相關 | 誰連到 Y、解不解析得到 | graph(反向邊) |
| `kura graph` | dashboard / 檢索料(之後) | dump link graph(薄 debug,不過度雕) | graph |

> v1 先做 `check` + `coverage`(spec §11 定的);`exists`/`backlinks`/`graph` 是抽取層+symbol table
> 建好後的近距離延伸(同一份資料、不同投影),設計抽取層時把它們的需求一起考慮進去,別只為 check 設計。

## 自己抽取(不吞第三方 Obsidian crate)

kura **自己做抽取**——撈 frontmatter + `[[link]]` 文字 + 行號 + heading 脈絡。這是 kura 的輸入層,
自己寫、自己掌握。**不用** turbovault-parser / gray_matter / obsidian-export 這類(用的人少、不確定可靠、
不在 rust-crate-registry,且 `[[ ]]` 抽取對 kura 不苦)。

但**用 `pulldown-cmark`** 當底層 markdown tokenizer(registry 線上、生態標準)——它給「這是 heading /
文字 / code block」事件,`[[ ]]` 的辨識、行號、heading 脈絡追蹤是 kura 在事件流上自己做。
frontmatter 切分(`---`...`---`)自己做幾行 + registry 的 YAML crate 解析。
crate 一律對 rust-crate-registry;registry 沒明列但 kura 真需要的(走訪 `ignore`、NFC `unicode-normalization`)
照 rust-spec「需要新 crate 留對話告訴使用者、不自己加」處理。

## resolver fidelity = #1 命脈(永不 false-positive)

全 5-消費者 panel 一致:**誤報「斷」永久毀信任 > 漏報一條可活 → 一律偏 false-negative。**
這是 kura 最該做對的單一件事。釘 Obsidian 實際行為(`~/obsidian/.obsidian/app.json`:
`alwaysUpdateLinks:true`、無自訂 newLinkFormat → 預設 shortest-path、大小寫不敏感):
- 比對:檔名 + alias(**標題不是解析鍵**)、大小寫不敏感、**NFC 正規化中文**(combining marks)、剝 `#`/`|`/`^`。
- `[[X#heading]]`/`[[X^block]]`:**只看檔案存在**,不驗 heading/block 存在(最多 info,絕不報斷)。
- 同名不同資料夾(歧義):emit `ambiguous` 或 collision,**不猜、不報斷**。
- conformance fixtures **對真 Obsidian 行為抓**(非重實作猜)——insta eval 的承重案例。

## 真實情境(都已在 vault 實測,當 fixtures + 驗收標的)

抽取/解析/檢查要能正確處理下面每一個(這些是 kura 存在的理由,不是假想):

1. **title_not_alias(killer)**:`Concepts/golang/Go Slice.md` title 是「Go Slice 內部結構」,
   但該字串不在它 aliases;**3 個檔**連 `[[Go Slice 內部結構]]` → Obsidian 靜默解不到、schema_lint 報綠。
   kura 必須抓到並說「把標題加進 Go Slice.md aliases,或改連 [[Go Slice]]」。
2. **缺口 heading 三種真實格式**(resolver 要認的,不是單一格式):`## 缺口 / 待補`、
   `### 缺口清單(待寫…`、`## forward-reference 缺口`。**含「缺口/待補/待寫」的 heading 下的 `[[X]]`** 當
   planned→info;其他未解析→warn。**不要**強推新慣例(如 `- [ ] [[X]]`)——偵測既有 heading 文字就好。
3. **collision.alias**:`Mechanical Sympathy` 重複 alias(2 檔)→ `[[Mechanical Sympathy]]` 不確定解析。
   一筆 finding 列**全部成員路徑**(別拆 N 個半 finding)。
4. **map↔disk**:`Go 課綱.md` 把 `bits-bytes-words` 列為「★ 待補模組(無現有課)」,但檔案**已存在硬碟**。
   雙向 reconcile:disk 有 map 無、map 有 disk 無都要報。
5. **provenance.unresolved**:`based_on`/`related`/`evolution_*` 指向不存在的筆記。
6. **coverage 校對**:`Vault-Index.md` **完全沒提 japanese**,但 japanese 是最大 domain(**62 概念**,
   golang 才 34)。`kura coverage` 掃 domain 要能生成清單揭穿這個漏。
7. **exists/dedup 規模**:golang 34 / japanese 62 / system-design 12 / rust 5 概念。agent 寫新概念前
   `kura exists <X>` 要在這個語料上確定性回答「寫過沒」。

## output contract(consumer 死活,v1 必做;見 spec §11)

- **stable fingerprint**(hash rule_id + 正規化 path + target,**不綁行號**)→ 消費者 set-diff 兩次 stateless 掃出 branch delta。
- **確定性全序排序**(path→line→rule_id)→ 乾淨 diff、insta 不 churn。
- **結構化欄位不靠解析中文 prose**:`target` / `resolved_to` / `collision_members`。
- json mode **stdout 純 JSONL**,其餘進 stderr;`--format json` 不靠 tty。
- **退出碼 0/1/2**(乾淨/gate-hit/tool-error,對齊 schema_lint)。
- human 摘要:**blast-radius 排序** + domain 分組 + **預設藏 info**(缺口收成一行計數)。
- 預設 link 檢查**排除 System/**(那些報告/spec 是「引用連結」不是真連結);`--all` 才含。

## P1 kickoff(parse + graph;只做這些)

1. parse:`ignore` 走訪 vault → 每檔切 frontmatter + YAML crate 解析成 `Note`(model.rs 已定型別)。
2. wikilink 抽取:`pulldown-cmark` 事件追 heading 脈絡(缺口 heading 標記)+ 跳 code block + 抽
   `[[target|display]]` 的 target 與行號 → `WikiLink`(under_gap_heading 已在型別裡)。
3. symbol table:slug/title/alias → path multimap(撞名在此浮現)。
4. link graph:每條 wikilink 按上面 fidelity 規則解析;歧義不猜。
5. eval:insta conformance fixtures 對真 Obsidian 行為,至少蓋上面情境 1/2/3 + 大小寫 + 同名不同資料夾。

**P1 不做**(留 P2/P3):5 條規則的實際 emit、JSONL 格式、coverage。P1 只要「圖建對 + 解析忠於 Obsidian」。
驗收:`cargo build` + `cargo clippy --all-targets -- -D warnings` + `cargo nextest run` 綠;對真 vault 跑能
正確認出 Go Slice 那條解不到。read-only,絕不改 vault 任何檔;不碰 git push。

## 不變量(死守,見 CLAUDE.md)
read-only · resolver 永不 false-positive · enum 從 `~/obsidian/System/schemas/vault-schema.toml` 載入不自帶第二份 ·
stateless 全掃純 JSONL 不碰 DB · output contract 硬化 · 每多一條規則要有真實案例。
