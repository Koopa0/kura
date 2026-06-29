# kura 設計訊號 — 挖掘 vault 翻出的 verified 事實(待併入 spec)

> 來源:2026-06-29 平行挖掘 `~/obsidian`(65 個 mined 痛點)+ kura session 親手覆查。
> 性質:這些是 **verified 事實**(每條附 file:line,已逐條重查),不是訪談意見。它們直接動到
> `vault-guard-spec.md` 已鎖的決策。**決策方向**仍待 obsidian-side Claude 訪談回覆(見 `.claude/kura-interview.md`)。
> 用法:訪談答案回來後,連同本檔一次更新 spec + 重畫 build plan。每條標了「事實 vs 待裁」。
>
> 標記:✅=kura session 親手 verified;⚠️=我先前口頭講錯、已修正;❓=待訪談裁定方向。

## 一覽

| # | 訊號 | 動到的鎖定決策 | 狀態 |
|---|---|---|---|
| A | concept 層 `title_en` 0/117 全空 | spec 跨語言 dedup「首選靠 title_en」 | ✅ 事實確定,❓方向待裁 |
| B | 缺口帳是純文字表格,非 `[[X]]` | spec §13 決策2「缺口 heading 下 `[[X]]`→planned」 | ✅ 事實確定,❓方向待裁 |
| C | `evolution_*` 是 lesson-only,concept 用 `based_on`/`related` | spec §10 `provenance.unresolved` 欄位集合 | ⚠️ 我先前講錯,已修正 |
| D | `links.py` 把 title 當解析鍵(:64-66)→ title_not_alias 報綠 | kura killer 價值坐實;`vault-link-report` 不能當 golden | ✅ 事實確定 |
| E | 兩條 gate 都有「正常情況被誤擋」風險 | spec §10 gate = collision.alias + map.disk_mismatch | ✅ 事實確定,❓方向待裁 |
| F | spec 檔仍名 `vault-guard-spec.md`,工具已改名 kura | 命名一致性 | ✅ 事實確定 |

---

## A — concept 層 `title_en` 全空,跨語言 dedup 首選方案地基是空的 ✅❓

**事實(verified)**:`Concepts/` 117 檔,有 `title_en:` 鍵的 **0 檔**。`title_en` 在 schema 是
`lesson_only` 欄位(`vault-schema.toml:59-60`),concept 結構上根本不帶它。

**衝擊**:我在對話中對「Go Slice 中文 vs 英文 怎麼判重複」給的首選答案是「靠既有 `title_en` 交叉撞名收斂」。
**這前提對 concept 不成立**——concept 沒有也不該有 title_en。所以跨語言 concept dedup 不能依賴 title_en。

**已知真實案例**:`Go 工程知識體系.md:677`「Go Slice 內部結構 / Go Slice Internals(4,同一概念中英兩名,
收斂成一篇)」——中英雙名重複是 verified 真實痛點,但發現它的是人工 MOC 附錄,不是 title_en。

**待裁(→ 訪談 §2.2)**:(a) 寫 concept 時強制填 title_en(風險:淪為沒人填的儀式欄位,正如現在 0/117);
(b) `similar` 對 concept 退回純詞彙相似 + domain/topic 共現,不依賴 title_en。
**我的傾向**:(b)。強制新欄位違反「每個欄位自證必要」且現況已證沒人填;`similar` 本就 advisory + recall-biased,
詞彙+共現足夠把候選窄到 LLM 能判的數量。

---

## B — 缺口帳是純文字表格,不是 `[[X]]`;planned 偵測 heuristic 直接失效 ✅❓

**事實(verified)**:`Go 工程知識體系.md:377-383` 的缺口帳是 **markdown 表格**,待建概念欄是純文字名
(「Go Interface 多型機制」「Go Method Set 規則」…),**不是 wikilink**。:664-682 的變體名收斂清單同樣純文字
(頓號分隔)。spec §13 決策2 假設「偵測缺口 heading 下的 `[[X]]` 當 planned→info」——對純文字直接無效,
因為根本沒有 `[[X]]` 可抓。

**衝擊**:planned vs 真斷的分類做不出來。若硬套 heuristic,缺口表格裡的計畫概念不會被認成 planned。
但反過來也沒有誤報風險(它們不是 wikilink,本來就不進斷鏈判定)——所以這比較像「heuristic 覆蓋不到」
而非「會誤判」。

**待裁(→ 訪談 §5.2/§5.3)**:(a) 把缺口條目改成 `[[X]]`(動 vault,違反「不強推新慣例」);
(b) kura 容忍純文字——純文字缺口名一律當「未引用計畫」、不參與斷鏈分類。
**我的傾向**:(b)。NEXT.md 明寫「不要強推新慣例,偵測既有 heading 文字就好」;純文字缺口名既然不是
wikilink、就不該進 link.broken 判定,容忍即可。另注意 spec 提的「含 缺口/待補/待寫 的 heading」關鍵字法
要對「第四種寫法」保持開放(訪談 §5.2a)。

---

## C — `evolution_*` 是 lesson-only,concept provenance 只有 `based_on`/`related`(我先前講錯)⚠️

**修正**:我上一則訊息說「`evolution_*` 在 schema 裡不存在、spec 寫超前」——**這是錯的**。覆查 `vault-schema.toml`:
- `based_on`(:54)、`related`(:55)在 **closed-schema known keys**(任何筆記可用,含 concept)。
- `evolution_predecessor` / `evolution_successors`(:60)在 **`lesson_only`** 區——只有 `type=lesson` 可用。
- `:163-165` 確認:`predecessor_field`/`successor_field` 註明 `# lesson`;`related` 是 `# 非 lesson` 的通用連結欄。

**事實(verified)**:所以 spec §10 `provenance.unresolved` 寫的 `based_on`/`related`/`evolution_*` 三者都存在,
但**分層**:concept 該驗 `based_on`+`related`;lesson 才加驗 `evolution_predecessor`+`evolution_successors`。
`evolution_*` 不是 glob,就這兩個具名欄位。

**衝擊**:provenance.unresolved 的欄位集合要**按 type 分**,不是一套通用清單。對 concept 驗 evolution_* 是驗一個
它結構上不該有的欄位。這是 schema-aware 的實作細節,不是 spec 錯——但 spec §10 的扁平寫法會誤導實作。

**訪談 §8.1 仍有效**(確認 v1 認哪些欄位),但前提已由我覆查釐清:按 type 分層。

---

## D — `links.py` 把 title 當解析鍵(:64-66),正是 title_not_alias 報綠的根因 ✅

**事實(verified)**:hermes 的 `pylib/hermes/vault/links.py` 建可解析索引時,把 frontmatter `title` 加進去
(`:64-66` `title = _TITLE.search(fm); if title: names.append(title.group(1))`;docstring :9 明列「相對路徑、
basename、frontmatter title、aliases」)。**Obsidian 不把 title 當解析鍵**——所以 `[[Go Slice 內部結構]]`
(Go Slice.md 的 title,不在它 aliases)在 links.py **解得到→報綠**,在真 Obsidian **靜默斷**。

**雙重意義**:
1. **kura 的 killer 價值坐實**:title_not_alias 不只沒被現行工具抓,現行工具還**方向相反地報綠**。kura 抓得到
   = 補上一個確定性盲區。這是 spec §15「最高價值兩件」之一,verified 成立。
2. **`vault-link-report.md` 不能直接當 golden**(訪談 §9.3):它由 links.py 生成,焊了這個 bug。拿它當基準
   會把 bug 一起搬進 kura。

**好 prior art(verified,值得照搬而非重造)**:links.py **已經**做對了——NFC 正規化(:39-45,註解明寫 macOS
NFD vs Obsidian NFC 的 CJK 陷阱)、大小寫不敏感(`.lower()`)、跳 frontmatter + code fence、剝 `|`/`#`/`^`、
純錨點 `[[#heading]]` 跳過。**kura resolver 應對齊 links.py 這些行為,唯一差異:解析索引不放 title**
(title_not_alias 改由獨立 finding 浮現,而非靜默解析掉)。這把 spec §12 fidelity 從抽象規格變成有現成參考實作。

---

## E — 兩條 gate 都有「正常情況被誤擋」風險 ✅❓

**E1 — map.disk_mismatch:bits-bytes-words 是 `status:draft`(verified)。**
`Writing/lessons/golang/Bits, Bytes, and Words.md:7` `status: draft`,slug `bits-bytes-words`。spec 把
「課文在硬碟、課綱沒列」當 gate-worthy mismatch——但**沒寫完的 draft 課本來就不該掛課綱**。gate 它 =
擋正常 WIP。**待裁(→ 訪談 §6.3)**:map.disk_mismatch 是否排除 `status:draft`。
**我的傾向**:排除 draft(或至少 draft→info 不 gate)。draft 不在課綱是預期狀態,不是漂移。

**E2 — collision.alias:verified 是 3-way 跨層撞名(含 lesson)。**
`Concepts/golang/Go 連續記憶體與 CPU Cache.md` 與 `Concepts/golang/薄抽象與硬體導向設計.md` 兩 concept 的
aliases **都含「Mechanical Sympathy」與「機械同理心」**;另 `Writing/lessons/golang/Arrays- Mechanical Sympathy.md`
也以小寫形式撞同名。**P1 resolver 對真 vault 跑,`resolve("Mechanical Sympathy")` 回 `Ambiguous` 列這 3 個成員
(2026-06-29 verified)** —— 證實 subagent 原宣稱的「跨 concept↔lesson 3-way、大小寫不敏感」。其餘出現此字的檔
(Slices、Prepare Your Mind)aliases 為空,是 prose 提及不算撞名。**待裁(→ 訪談 §6.2)**:這種共享 alias 有無正當用途、
v1 collision 範圍(alias↔alias / 跨欄 / slug 唯一性 / 跨層)。
**我的傾向**:2-concept 真撞名 = 真實 gate 案例成立;但 gate 前要確認「同一概念的兩個面向刻意共享框架名」
不是正當用途(這兩檔講的都是 mechanical sympathy 的不同切面,有可能是刻意)。需 Koopa/訪談判意圖。

---

## F — spec 檔名仍是 `vault-guard-spec.md`,工具已改名 kura ✅

**事實(verified)**:工具/repo 已是 kura,但 vault 端 spec 檔仍名 `~/obsidian/System/vault-guard-spec.md`,
且 repo 內 `README.md:28,32`、`CLAUDE.md:8,44`、`NEXT.md:4`、`src/lib.rs:5` 都引用這個舊名檔。schema toml
內部也用 `vault-guard`(:81,160「resolve 由 vault-guard 驗」「vault-guard 的 ledger」)。

**衝擊**:不是 code identifier drift(沒有 `vault_guard` 型別/crate 名混用——已查 src/lib.rs 只是註解引用),
純粹是**權威文件與 schema 內部還用舊名**。低風險,但 P1 動工前該決定:spec 檔改名 `kura-spec.md`(要動 vault +
所有引用),還是接受「spec 檔保留歷史名、工具叫 kura」。**我的傾向**:小事,留到有空再改名;現在記錄即可,
別為改名動一票檔案。但 schema toml 裡的 `vault-guard` 字樣(:81,160)指的是「驗 resolve 的那支工具」=
現在的 kura,語意正確只是名舊,改不改都不影響功能。

---

## 對 spec 的淨影響(訪談前的暫定)

| spec 條目 | 暫定修正 | 待訪談確認 |
|---|---|---|
| 跨語言 dedup 首選 title_en | 改為 similar 走詞彙+共現,title_en 不當 concept 依賴 | §2.2 方向 |
| §13 決策2 planned 偵測 | 補「容忍純文字缺口名,不進斷鏈判定」 | §5.2/§5.3 方向 |
| §10 provenance.unresolved | 欄位按 type 分層(concept: based_on/related;lesson: +evolution_*) | §8.1 確認 |
| §12 resolver fidelity | 加註「對齊 links.py 行為,唯一差異:解析索引不含 title」 | — (已是事實) |
| §9.3 golden 基準 | vault-link-report 不可直接當 golden(焊了 title-as-key bug) | §9.3 確認 |
| §10 gate 集合 | map.disk_mismatch 排除 draft;collision.alias 待確認正當用途/範圍 | §6.2/§6.3 方向 |
| 命名 | 記錄 vault-guard→kura 檔名債,暫不動 | — |

---

# 訪談裁決(obsidian-side Claude,2026-06-29)

> 來源:`kura-interview.md` 34 題的消費者回覆(全 verified、附 file:line)。這一節是 A–F 的**答案** +
> 新增訊號 G–N。**所有「待裁」到此定案**。下一輪改 spec + 重畫 build plan 以本節為準。

## A–F 結案

- **A(title_en)**:✅ 確認 concept title_en = 0/116,且全 git 史 **0 次** concept 合併/刪除/改名(dedup 是「標記未動手」存量,非事件流)。**裁決:title_en 從跨語言 dedup 機制整個拿掉**;`similar` 改走 aliases(NFC+大小寫不敏感)+ domain 相同 + topics 交集,**永遠 advisory**。
- **B(純文字缺口)**:✅ 確認缺口清單絕大多數純文字(非 `[[X]]`)。**裁決:容忍純文字 → 收進「planned names」表,不參與斷鏈/orphan;不全庫改 `[[X]]`(expansionary)**。更關鍵:**權威 planned 訊號不是缺口 heading,而是「concept 內文真實 `[[X]]` 照 Obsidian 預設(檔名/alias,不認 title)解不解得到」**;缺口帳/`（待整理）` 只當 advisory(實測 `（待整理）` 同時黏在活連結與真斷上,不是乾淨判準)。
- **C(evolution_*)**:⚠️ 我先前兩度講錯,**最終正解**:evolution_predecessor/successors **存在**於 toml `lesson_only`(:60),且 vault 有 4 對課文實際在用。**裁決:provenance.unresolved 認 4 欄,但內部兩條 resolve path** —— `based_on`/`related` = wikilink(走 §12 resolver);`evolution_*` = **slug → lesson symbol table**(值是 slug 如 `maps-go124`,**丟進 wikilink resolver 會 false-positive 報斷,破 §12**)。
- **D(links.py title bug)**:✅ 確認。**canonical golden 漏報樣本 = 3 條真連結** `[[Go Slice 內部結構]]`:`Go 記憶體對齊.md:292`、`Go 零值設計.md:273`、`Go String.md:257`(此刻在 Obsidian 斷、工具全報綠)。**`vault-link-report.md` 是 anti-golden,不可當 pass/fail baseline**(焊了 title-bug、綁行號、stale);只能拿它的人工二分當 fixture 來源。
- **E(gate 誤傷)**:✅ 雙確認。map.disk_mismatch 母體此刻 = 4 個 `status:draft` 清一色 Direction B(檔在/課綱無),真正危險的 Direction A(課綱有/檔無)= **0**。collision.alias 存量 **7 個 key 不是 1 個**(見 L)。**裁決見 E→gate 重設(下方 J/L)**。
- **F(命名)**:不變,暫不動。

## G — resolver 索引必須含非-md 可連結檔(🔴 P1 CRITICAL,直接修)

**事實(我 2026-06-29 verified)**:Obsidian `[[ ]]` 可解到非-md 檔(canvas/圖/pdf/base)。vault 實際被連的非-md
= 2 條 canvas:`[[DDIA-Ch1-Overview.canvas|…]]`(`Diagrams/canvas/DDIA-Ch1-Overview.canvas` **硬碟有** → Obsidian 解得到)
與 `[[DDIA-Ch2-Data-Models-Comparison.canvas]]`(硬碟無 → 真斷)。**kura 現況只 index `.md` stem,對前者回 `Unresolved`
= false-positive,P2 會誤報斷 → 破 §12 #1 不變量。** 這是消費者「只做對一件事」的硬 caveat:resolver-done-right
= title 不當鍵(已有)**且** 非-md 檔進索引。
**修法(P1)**:walk **全部**檔;`.md` → 檔名 stem + alias 當鍵;**非-md → 全檔名(含副檔名)當鍵**(Obsidian 對
非-md 要求連結帶副檔名,故只 index full filename、不 index stem,既忠實又不製造假歧義)。

## H — exists 與 resolver 的鍵集**刻意相反**(P2,exists 命令)

resolver 偏 false-negative(永不誤報斷)→ **窄鍵集**(檔名 ∪ alias,**排除 title**)。exists 偏 over-recall
(永不漏報「寫過」)→ **寬鍵集**(檔名 ∪ **title** ∪ alias ∪ title_en,case-insensitive+NFC,**回報哪個欄位命中**)。
實測 **7/116 concept 的 title≠檔名且不在 aliases**(Go Slice/Go String/Go 零拷貝…)——exists 若照抄 resolver 排除
title,正好對這 7 個(最高中英重複風險)false-negative。**resolver 鍵集 ⊊ exists 鍵集,兩命令兩套正規化索引。**

## I — Python lint 吸收範圍**修正**(我先前 over-scope)

我先前說「kura 吸收三支 Python lint」**錯**。spec §2 白紙黑字「不取代 style_lint.py 的 de-AI(留 Claude)」;
§13/§14 只列 **schema_lint** Stage 3 吸收;translator_lint 根本不在 spec。**裁決:只 schema_lint 遷移**
(golden 粒度 = `(path, rule-class, 違規欄位/值)` 集合相等,**非 byte-identical**);**style_lint de-AI 永遠留 LLM**;
**translator_lint 留 Python(YAGNI)**。schema_lint 最怕回歸的分支:System 文件輕規則 early-return、inbox 免 domain、
closed-schema 未知欄位集差、domain==資料夾 off-by-one、concept provenance 的 OR 邏輯。

## J — `--baseline` 進 v1 + gate 重設(消費者最強 pushback)

§13.6 把 set-diff 外包消費者手搓兩次掃、`--baseline` 延 Next = **v1 最該翻的決策**。沒它,「link.broken 永不
gate、靠 fingerprint delta 只看 branch 新增」兌現不了,消費者第一天就 `--no-verify`。**裁決:`--baseline <prev.jsonl>`
進 v1**(讀上次 JSONL、內部 fingerprint set-diff、只吐**新增** findings、exit 依新增 gated;仍不碰 git、仍 stateless)。
**gate 重設(全 delta-only,只擋 branch 新引入)**:① `link.title_not_alias` 領銜(killer、零誤報、§15 最高價值);
② `collision.alias`(case-insensitive+NFC+跨層,只擋新 key,7 存量先 advisory);③ `map.disk_mismatch` **只 Direction A**
(課綱有/檔無;現 0 實例)+ **draft 永遠豁免**。永遠 advisory:概念層 forward-ref、Direction B/draft、provenance、coverage/orphan、課文斷的存量。

## K — 新規則候選 `link.broken.path`(消費者最大盲區,P2/P3 評估)

五規則 + exists/similar/coverage **全部只走 `[[wikilink]]`+frontmatter**,對 `[text](相對路徑)` 與 backtick `path.md`
**結構上看不見**,而它正在腐爛且**確定性可判**(filesystem stat,零 FP)。實證:6 檔 7 處 `[考試模組](../../../../../exam/go/*.md)`
全死(`Pointers- GC.md:276` 等)。**裁決:評估加 `link.broken.path`**——只 stat「resolve 後仍在 `--root` 內」的相對路徑;
逃出 root 的(exam/go)當 `info` 標 "external, not stat'd"(守永不-false-positive);backtick `.md` 死鏈當 info,**prose 人名引用(rust-batch-1)不碰**(那是 fuzzy NLP)。

## L — collision.alias 範圍定案(P2)

**case-insensitive + NFC + 跨層全域** collision_key(否則連旗艦 Mechanical Sympathy 都抓不全)。實測 **7 個真 alias↔alias key**
(mechanical sympathy 3-way 跨層、pointer/value semantics、pass by value、機械同理心、ています、程度副詞)。
collision 只吃 frontmatter `aliases:`,**不吃 prose 提及**(Prepare Your Mind 等 aliases 空,不算成員)。
範圍:alias↔alias **gate**(只擋新);alias↔filename **掃但只 advisory**(0 實例);**slug 唯一性不做**(0 重複);
**比對不納 title**(§12)。無泛 allowlist(共享 alias 保證掉鏈,無解析安全的正當用途)。

## M — coverage / orphan 定案(P3)

concept-orphan「被收錄」= **union(Maps body `[[wikilink]]` + topic-maps)**;**study-path/課綱課文行不算**(那些連 lesson 檔名,屬 lesson 圖,別混算)。
實測 japanese **32/62 concept 在任何 Maps 無 wikilink**,但屬「批次延後收錄」(學習路徑:90 明寫 L20+ 待補掛)→
**降級 advisory「待補掛」,不報 orphan**(否則第一次跑就對最大 domain 噴 32 假孤兒,= §12 false-positive 換命令重現)。
orphan 報告附「inbound concept refs = N」。`coverage --suggest` 吐 paste-ready MOC tally(人決定貼,**不破 read-only**)。
**symbol table 做成全節點雙向鄰接(forward+reverse)→ backlinks 變 `coverage` 投影,不開新命令**;`graph` 命令確認砍。

## N — 消費者確認的「不要」(全對齊 convergent)

supersession.* 留 Next(最小單檔 warn 歸 schema_lint 不歸 kura);archived-移除候選報告**不要**(破 stateless、0 輸入);
prose-rot 第 6 條**不要**(backtick `.md` 併進 K 的 info);治理文件 enum 掃描器**不要**(改用刪除重複副本;窄版 retired-token denylist 可選低優先);
toml↔doctrine 對賬**歸 gen_fileclasses.py 不歸 kura**;topics 提議器**不要**(topics 刻意開放);MCP warm index **deferred 確認 YAGNI**(425 檔全掃 0.022s)。
lifecycle 轉移/ready owner **永不碰**(連 git-author 推 owner 都不要——本 vault 全 commit 都是 Koopa,零鑑別力)。

## 對 P1 的即時影響(只有一條)

**G = 唯一要改 P1 code 的**:vault walk 全檔、symbol table 加非-md 全檔名鍵。其餘 H–N 全是 P2/P3/spec/未來命令,
P1 不動。`under_gap_heading` 順手補關鍵字 `待整理`/`待建`(B 的 verified 補充;但因 B 已把缺口 heading 降為 advisory,
不投資 bold-heading 偵測)。
