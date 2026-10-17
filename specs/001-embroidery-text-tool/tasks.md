---

description: "手機刺繡工具（文字與簡單圖片）任務清單"
---

# Tasks: 手機刺繡工具（文字與簡單圖片）

**Input**: Design documents from `/specs/001-embroidery-text-tool/`

**Prerequisites**: plan.md、spec.md、research.md、data-model.md、contracts/、quickstart.md

**Tests**: 本專案憲章原則一（TDD，NON-NEGOTIABLE）要求**測試先行**。每個實作任務之前都有對應的測試任務；
測試任務 MUST 先寫、先執行並**確認失敗（紅）**，再做實作使其通過（綠），最後重構。

**Organization**: 依使用者故事分組，每個故事可獨立實作與測試。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可平行（不同檔案、無未完成的相依）
- **[Story]**: 所屬使用者故事（US1～US5）
- 所有長度單位為 0.1mm 整數（例如 160mm = `1600`）

## Path Conventions

Rust workspace 加獨立前端（見 plan.md）：`crates/<名稱>/{src,tests}`、`web/src`、`web/tests`、`tools/`、`fonts/`。

---

## Phase 1: Setup（共用基礎）

**Purpose**: 專案初始化與基本結構

- [x] T001 建立 Rust workspace：根目錄 `Cargo.toml`，與 `crates/stitch-core`、`stitch-formats`、`stitch-geom`、`stitch-gen`、`stitch-text`、`stitch-image`、`stitch-wasm`、`stitch-cli` 各自的 `Cargo.toml` 與空的 `src/lib.rs`（cli 為 `src/main.rs`），依賴方向：core ← geom ← gen ← text／image ← wasm；formats 只依賴 core
- [x] T002 [P] 設定格式與靜態檢查：`rustfmt.toml`、`scripts/check.sh`（依序執行 `cargo fmt --check`、`cargo clippy --workspace -- -D warnings`、`cargo test --workspace`），與 `.gitignore`（忽略 `target/`、`out/`、`web/node_modules/`、`web/dist/`、`.claude/`）
- [x] T003 [P] 建立前端骨架：在 `web/` 以 Vite 建立 Svelte 5 加 TypeScript 專案，設定 `vitest`（`web/vitest.config.ts`）與 Playwright（`web/playwright.config.ts`，使用 Pixel 7 手機視窗）
- [x] T004 [P] 建立交叉驗證工具骨架 `tools/verify_outputs.py`（以 `uv run --no-project --with pyembroidery python -I` 執行，輸入資料夾，讀回每個 `.DST`／`.EXP` 並輸出針數、座標範圍、換色數的 JSON）
- [x] T005 [P] 建立字型資料夾 `fonts/README.md`，列出四個候選字型（Noto Sans TC、Noto Serif TC、粉圓體 jf-openhuninn、霞鶩文楷 TC）與授權確認欄位

---

## Phase 2: Foundational（阻擋所有故事的先決條件）

**Purpose**: 型別、繡框安全範圍、位移拆分、格式註冊表、DST 與 EXP、基本幾何、WASM 骨架

**⚠️ CRITICAL**: 此階段完成前，不得開始任何使用者故事

### 核心型別與繡框

- [x] T006 [P] 先寫測試（確認失敗）：針點、針跡計畫、線色、使用者訊息型別的建構與序列化，於 `crates/stitch-core/tests/types.rs`；針點類型 MUST 為 `Stitch`、`Jump`、`Trim`、`ColorChange`、`End`
- [x] T007 實作 `crates/stitch-core/src/types.rs` 與 `message.rs`：`Point{x:i32,y:i32}`、`StitchKind`、`Stitch`、`StitchPlan{stitches, colors}`、`ThreadColor{r,g,b,name}`、`UserMessage{code,text,hint}`（讓 T006 通過）
- [x] T008 [P] 先寫測試（確認失敗）：繡框驗證與安全範圍，於 `crates/stitch-core/tests/hoop.rs`。須涵蓋 data-model 規則：`name` 非空且 ≤ 20 字；`width`、`height` > 0 且上限 3000；`margin` ≥ 0 且 < min(width, height) / 2，預設 50；圓形時 width = height；內建框不可刪除或修改尺寸；方形、圓形、橢圓的 `contains(point)` 判斷；安全範圍 = 可繡區內縮 margin
- [x] T009 實作 `crates/stitch-core/src/hoop.rs`：`Hoop`、`HoopShape{Rect,Circle,Ellipse}`、`validate_hoop`、`safe_contains`，與三個內建框常數（`b70-large` 1600×2600、`b70-medium` 1200×1800、`b70-small` 500×750）（讓 T008 通過）
- [x] T010 [P] 先寫測試（確認失敗）：針跡計畫驗證，於 `crates/stitch-core/tests/validate.rs`。不變式：全部針點在所選框安全範圍內；以 `End` 結尾；`ColorChange` 次數與顏色數一致；空計畫被拒絕
- [x] T011 實作 `crates/stitch-core/src/validate.rs`：`validate_plan(plan, hoop) -> Result<(), UserMessage>`，訊息為繁體中文白話（讓 T010 通過）

### 位移拆分與格式註冊表

- [x] T012 [P] 先寫測試（確認失敗）：單針位移拆分，於 `crates/stitch-formats/tests/split.rs`，含 `proptest` 屬性測試：任意位移拆分後每段 ≤ `max_step`，且總和等於原位移（DST 上限 121、EXP 上限 127）
- [x] T013 實作 `crates/stitch-formats/src/split.rs`：`split_move(dx, dy, max_step)`（讓 T012 通過）
- [x] T014 [P] 先寫測試（確認失敗）：格式註冊表，於 `crates/stitch-formats/tests/registry.rs`。須驗證：可列出已註冊格式；以 id 找格式；註冊一個測試用假格式後，既有格式與其輸出完全不變（對應 FR-011）
- [x] T015 實作 `crates/stitch-formats/src/format.rs` 與 `lib.rs`：`trait Format{id, extension, label, description, max_step, encode, decode}` 與 `Registry`，預設註冊 `dst` 與 `exp`（讓 T014 通過）

### DST 與 EXP

- [x] T016 [P] 先寫測試（確認失敗）：DST，於 `crates/stitch-formats/tests/dst.rs`。含：已知位移的三位元組編碼 golden 位元組；512 位元組表頭（`LA:`、`ST:`、`CO:`、`+X/-X/+Y/-Y` 範圍）；跳針、換色、結束碼；Y 軸方向；寫出後讀回的往返等價
- [x] T017 實作 `crates/stitch-formats/src/dst.rs`（讓 T016 通過）
- [x] T018 [P] 先寫測試（確認失敗）：EXP，於 `crates/stitch-formats/tests/exp.rs`。含：兩位元組位移、`0x80` 控制碼（跳針、換色）golden 位元組，與往返等價
- [x] T019 實作 `crates/stitch-formats/src/exp.rs`（讓 T018 通過）

### 開發用 CLI 與交叉驗證

- [x] T020 [P] 先寫測試（確認失敗）：`stitch-cli sample --shape square --hoop b70-small --out <dir>` 會產生 `.DST` 與 `.EXP`，於 `crates/stitch-cli/tests/sample.rs`
- [x] T021 實作 `crates/stitch-cli/src/main.rs` 的 `sample` 子命令（內建方形測試圖樣；之後任務會加入 `--text`）
- [x] T022 實作 `tools/verify_outputs.py` 的比對邏輯，並建立 `scripts/verify.sh`（先跑 CLI 產生樣本，再用 pyembroidery 讀回；針數、座標範圍、換色數須與核心一致，否則非零結束）。首次執行 MUST 通過，通過才算 DST／EXP 格式獨立驗證完成

### 基本幾何

- [x] T023 [P] 先寫測試（確認失敗）：多邊形（含洞）、曲線展平（以 `kurbo`）、面積與點在多邊形內、掃描線求交，於 `crates/stitch-geom/tests/geom.rs`
- [x] T024 實作 `crates/stitch-geom/src/{polygon,flatten,scanline}.rs`（讓 T023 通過）；多邊形偏移與布林運算使用 `i_overlay`，包裝於 `offset.rs`，並於 T023 補上偏移測試

### WASM 骨架

- [x] T025 [P] 先寫測試（確認失敗）：`list_builtin_hoops`、`list_formats`、`validate_hoop`，於 `crates/stitch-wasm/tests/api.rs`（以 `wasm-pack test --node` 執行），對應 contracts/wasm-api.md（實作偏差：邏輯放在可原生測試的 `crates/stitch-wasm/src/api.rs`，測試改以 `cargo test` 執行；WASM 實際載入由 T027 的前端測試涵蓋）
- [x] T026 實作 `crates/stitch-wasm/src/lib.rs`：僅做型別轉接（`serde-wasm-bindgen`），無業務邏輯（讓 T025 通過）
- [x] T027 [P] 建立前端核心載入層：先寫測試 `web/tests/core.test.ts`，再實作 `web/src/lib/core.ts`（載入 WASM、提供型別化包裝）與 `web/package.json` 的 `build:wasm` 指令（`wasm-pack build crates/stitch-wasm --target web --out-dir ../../web/src/lib/wasm`）

**Checkpoint**: 型別、繡框、DST／EXP、註冊表、基本幾何、WASM 骨架就緒；`scripts/check.sh` 與 `scripts/verify.sh` 全部通過

---

## Phase 3: User Story 1 - 輸入文字並產生刺繡檔 (Priority: P1) 🎯 MVP

**Goal**: 輸入文字、選字型大小線色，看到接近真實的預覽，選內建繡框，存出 DST／EXP 檔。

**Independent Test**: 輸入「王小明 2026」，選中框、一種字型與顏色，預覽出現且尺寸正確；輸出檔能被 pyembroidery 讀回且內容與預覽一致。

### 字型

- [x] T028 [US1] 確認四個候選字型的授權允許嵌入、子集化與散布：逐一取得官方授權文字，存入 `fonts/LICENSES/<字型名>.txt`，更新 `fonts/README.md`；授權不允許者從清單移除
- [x] T029 [P] [US1] 先寫測試（確認失敗）：`tools/subset_fonts.py` 產生的子集字型須涵蓋 Big5 第一級 5401 常用繁體字加英數與常見標點，且單一檔案大小有上限（每個 ≤ 4MB；實作偏差：原訂 2MB，實測第一級 5401 字的子集為 1.8～3.6MB，總計約 9.9MB），於 `tools/tests/test_subset.py`
- [x] T030 [US1] 實作 `tools/subset_fonts.py`（以 `fonttools` 子集化），產生 `fonts/*.ttf`（讓 T029 通過）

### 文字轉多邊形

- [x] T031 [P] [US1] 先寫測試（確認失敗）：字型載入、單一字形輪廓（`A`、`8`、`王`、`龍`）轉多邊形（含洞）、`single_line` 與 `wrap` 排版、字形缺漏時退回備用字型並回報，於 `crates/stitch-text/tests/text.rs`；`TextElement.text` MUST 非空白，空白輸入回傳繁體中文訊息
- [x] T032 [US1] 實作 `crates/stitch-text/src/{font,layout,glyph}.rs`（以 `ttf-parser` 與 `kurbo`）（讓 T031 通過）

### 針跡生成

- [x] T033 [P] [US1] 先寫測試（確認失敗）：填充針（tatami），於 `crates/stitch-gen/tests/fill.rs`。固定輸入的 golden 針跡序列；行距預設 4（0.4mm）；單針長度上限 30（3mm）；行間錯位圖案；洞不被填；所有針點在多邊形內（`proptest`）
- [x] T034 [US1] 實作 `crates/stitch-gen/src/fill.rs`（讓 T033 通過）
- [x] T035 [P] [US1] 先寫測試（確認失敗）：輪廓走針與底針，於 `crates/stitch-gen/tests/outline_underlay.rs`
- [x] T036 [US1] 實作 `crates/stitch-gen/src/{outline,underlay}.rs`（讓 T035 通過）
- [x] T037 [P] [US1] 先寫測試（確認失敗）：固定針（起針與收針）、跳針與剪線規則（位移超過 30 使用跳針；超過 80 加剪線，數值可設定），與按顏色排序，於 `crates/stitch-gen/tests/travel.rs`
- [x] T038 [US1] 實作 `crates/stitch-gen/src/{lockstitch,travel,order}.rs`（讓 T037 通過）
- [x] T039 [P] [US1] 先寫測試（確認失敗）：最小可繡尺寸，於 `crates/stitch-gen/tests/min_size.rs`。`min_text_height` 預設 40（4mm）、`min_stroke_width` 預設 8（0.8mm）；低於下限自動放大並產生調整說明；放大後仍超出安全範圍則 `blocked`
- [x] T040 [US1] 實作 `crates/stitch-gen/src/min_size.rs`（讓 T039 通過）
- [x] T041 [P] [US1] 先寫測試（確認失敗）：縮放適配，於 `crates/stitch-gen/tests/fit.rs`。設計超出方形安全範圍時等比縮小至剛好放入；縮小後低於最小尺寸則阻止並給出建議（改選較大的框）
- [x] T042 [US1] 實作 `crates/stitch-gen/src/fit.rs`（讓 T041 通過）

### 組裝與介面

- [x] T043 [P] [US1] 先寫測試（確認失敗）：`build_text_preview`，於 `crates/stitch-wasm/tests/text_preview.rs`，對應 contracts/wasm-api.md 的 `TextRequest`／`PreviewResult`；後置條件：`blocked` 為空時所有針點落在安全範圍；相同輸入得到相同輸出
- [x] T044 [US1] 實作 `crates/stitch-text/src/pipeline.rs` 與 `crates/stitch-wasm/src/text.rs` 的 `build_text_preview`、`encode`、`encode_all`、`list_fonts`（讓 T043 通過）；`encode` 內 MUST 再次驗證安全範圍，不信任前端
- [x] T045 [P] [US1] 先寫測試（確認失敗）：`stitch-cli sample --text "Amy 2026"` 與 `scripts/verify.sh` 以文字樣本通過 pyembroidery 讀回比對，於 `crates/stitch-cli/tests/text_sample.rs`
- [x] T046 [US1] 擴充 `crates/stitch-cli/src/main.rs` 的 `sample --text`，並將文字樣本納入 `scripts/verify.sh`（讓 T045 通過）
- [x] T047 [P] [US1] 先寫測試（確認失敗）：預覽繪製的座標換算與線條方向（不依賴 DOM 的純函式），於 `web/tests/preview.test.ts`
- [x] T048 [US1] 實作 `web/src/components/PreviewCanvas.svelte` 與 `web/src/lib/preview.ts`：Canvas 2D 逐針繪製，帶方向與微光澤、布料底色、顯示實際尺寸（讓 T047 通過）
- [x] T049 [P] [US1] 先寫測試（確認失敗）：繁體中文介面字串與術語規範，於 `web/tests/strings.test.ts`。使用者可見字串 MUST NOT 含 quantization、解析度等技術術語（格式名稱僅限測試說明與設定）
- [x] T050 [US1] 建立 `web/src/lib/strings.zh-TW.ts`（所有使用者可見文字）與色票 `web/src/lib/threads.ts`（附中文色名，如「正紅」「米白」）（讓 T049 通過）
- [x] T051 [P] [US1] 先寫測試（確認失敗）：可復原的編輯狀態（FR-019），於 `web/tests/history.test.ts`
- [x] T052 [US1] 實作 `web/src/lib/store.ts`（設計狀態加復原與重做）（讓 T051 通過）
- [x] T053 [US1] 實作文字輸入頁 `web/src/routes/TextPage.svelte`：文字框、字型選擇、大小、線色、內建框選擇、即時預覽（輸入時不需按確定），並在 `web/src/components/` 加入大按鈕（觸控目標 ≥ 48px）；使用 Web Worker `web/src/lib/worker.ts` 呼叫核心避免凍結
- [x] T054 [US1] 實作「存到 USB」：`web/src/lib/export.ts`（呼叫 `encode_all`，逐檔觸發下載）與按鈕；輸出前顯示白話說明
- [x] T055 [US1] Playwright 端對端（手機視窗）：輸入文字 → 預覽 → 選框 → 存出，於 `web/tests/e2e/text-flow.spec.ts`；先寫並確認失敗，完成上述任務後通過

**Checkpoint**: User Story 1 可獨立運作：輸入文字即可得到經 pyembroidery 驗證的 DST／EXP 檔（MVP）

---

## Phase 4: User Story 2 - 選擇繡框並避免超出範圍 (Priority: P1)

**Goal**: 可新增、編輯、刪除自己的繡框（含圓形與橢圓），設計超出範圍時自動縮放或阻止，並用白話說明。

**Independent Test**: 選 50×75mm 小框，輸入很長的文字，被縮小或阻止並顯示白話說明；改選大框後可正常輸出；新增 10×10cm 框後重新整理仍在。

- [x] T056 [P] [US2] 先寫測試（確認失敗）：圓形與橢圓框的適配與阻止，於 `crates/stitch-gen/tests/fit_shapes.rs`（含 `proptest`：任意設計適配後針點皆在安全範圍內，SC-004）
- [x] T057 [US2] 擴充 `crates/stitch-gen/src/fit.rs` 支援圓形與橢圓（讓 T056 通過）（實作偏差：T042 的 fit.rs 已用「頂點是否在凸形安全範圍」的通用寫法，圓形與橢圓當時就涵蓋，T056 的測試因此一寫就過；已另外確認方形與圓形對同一設計給出不同結果，測試並非空洞）
- [x] T058 [P] [US2] 先寫測試（確認失敗）：繡框儲存（新增、編輯、刪除、重新載入），於 `web/tests/hoops.test.ts`。內建框不可刪除或改尺寸；名稱 ≤ 20 字且非空；寬高 > 0 且 ≤ 3000（以 0.1mm 為單位）；錯誤訊息為繁體中文白話
- [x] T059 [US2] 實作 `web/src/lib/hoops.ts`（`localStorage`，所有存取包 `try/catch`，失敗時仍能運作）（讓 T058 通過）
- [x] T060 [P] [US2] 先寫測試（確認失敗）：新增框表單的驗證與「量哪裡」提示，於 `web/tests/hoop-form.test.ts`
- [x] T061 [US2] 實作 `web/src/components/HoopPicker.svelte` 與 `HoopForm.svelte`：選框清單、新增表單（名稱、可繡寬高、形狀、安全邊距預設 5mm）、「請量內框可繡範圍」示意圖 `web/public/hoop-measure.svg`（讓 T060 通過）
- [x] T062 [US2] 在 `web/src/routes/TextPage.svelte` 整合：超出範圍時顯示調整說明；被阻止時禁用「存到 USB」並顯示白話原因與建議，於 `web/src/components/Notice.svelte`
- [x] T063 [US2] Playwright 端對端：小框加長文字被縮小或阻止；新增 10×10cm 框後重新整理仍在；內建框無刪除按鈕，於 `web/tests/e2e/hoop-flow.spec.ts`（先寫、先失敗）

**Checkpoint**: User Stories 1 與 2 皆可獨立運作

---

## Phase 5: User Story 3 - 輸出多種檔案格式供實機測試 (Priority: P1)

**Goal**: 一次輸出多種格式、檔名安全易懂、預設格式可記住，並提供給媽媽的測試說明。

**Independent Test**: 選「全部都存，我要試試看」得到每種格式一個檔案；選定預設格式後只輸出該格式，重開仍記得。

- [x] T064 [P] [US3] 先寫測試（確認失敗）：檔名規則，於 `crates/stitch-formats/tests/filename.rs`，對應 contracts/output-files.md。主檔名只含 `A-Z0-9`、1～8 字元；取設計名稱英數字轉大寫最多 4 字元，後接 4 位數序號；無可用字元時前綴 `EMB`；同次輸出共用主檔名；副檔名大寫；`proptest`：任意名稱（含中文、空白、符號、極長字串）皆產生合法檔名
- [x] T065 [US3] 實作 `crates/stitch-formats/src/filename.rs`（讓 T064 通過）
- [x] T066 [P] [US3] 先寫測試（確認失敗）：`encode_all` 的格式選擇（空陣列代表全部已註冊格式）與各檔針數一致性，於 `crates/stitch-wasm/tests/encode_all.rs`
- [x] T067 [US3] 完成 `crates/stitch-wasm/src/encode.rs` 的 `encode_all`（讓 T066 通過）
- [x] T068 [P] [US3] 先寫測試（確認失敗）：偏好設定，於 `web/tests/prefs.test.ts`。`default_formats` 空代表尚未選定，首次輸出全部格式並顯示說明；選定後只輸出所選；重新載入仍記得；可改回空
- [x] T069 [US3] 實作 `web/src/lib/prefs.ts`（讓 T068 通過）
- [x] T070 [P] [US3] 先寫測試（確認失敗）：Web Share 備援的判斷與降級，於 `web/tests/share.test.ts`
- [x] T071 [US3] 實作 `web/src/lib/export.ts` 的分享備援與「畫面名稱 ⇄ 檔名」對照資料（讓 T070 通過）
- [x] T072 [US3] 實作輸出畫面 `web/src/routes/ExportPage.svelte`：「全部都存，我要試試看」、檔名對照表、首次說明文字、下載多檔時提醒按「允許」
- [x] T073 [US3] 實作實機測試說明頁 `web/src/routes/TryOnMachine.svelte`：依 quickstart.md 第五節的步驟，格式以易懂名稱顯示並附副檔名，每個格式一顆「這個機器讀得到，以後就用它」按鈕
- [x] T074 [US3] Playwright 端對端：首次輸出得到全部格式；選定預設後只輸出所選；檔名合法，於 `web/tests/e2e/formats-flow.spec.ts`（先寫、先失敗）

**Checkpoint**: User Stories 1～3 皆可獨立運作，交付給媽媽測試所需的機制齊備

---

## Phase 6: User Story 4 - 簡單圖片轉成刺繡 (Priority: P2)

**Goal**: 選一張色塊明確的圖片，轉成刺繡預覽，可調整顏色多寡與簡化程度。

**Independent Test**: 選一張 3 色圖案，預覽顯示 3 種線色，過小細節被簡化；輸出檔可被讀回。

- [x] T075 [P] [US4] 先寫測試（確認失敗）：色彩分群與限色，於 `crates/stitch-image/tests/quantize.rs`。`ImageElement.color_count` MUST 為 2～12；合成的 3 色圖得到 3 個群
- [x] T076 [US4] 實作 `crates/stitch-image/src/quantize.rs`（以 `visioncortex`）（讓 T075 通過）
- [x] T077 [P] [US4] 先寫測試（確認失敗）：輪廓追蹤與簡化，於 `crates/stitch-image/tests/trace.rs`。`simplify` MUST 為 0～100；小於最小面積或最小筆畫寬的區塊被合併或丟棄並產生說明；單色與全空白圖不崩潰
- [x] T078 [US4] 實作 `crates/stitch-image/src/{trace,simplify}.rs`（讓 T077 通過）
- [x] T079 [P] [US4] 先寫測試（確認失敗）：`build_image_preview`，於 `crates/stitch-wasm/tests/image_preview.rs`，含複雜漸層圖時回傳「效果有限」提示但允許繼續
- [x] T080 [US4] 實作 `crates/stitch-wasm/src/image.rs`（接 stitch-gen 的填充針與適配）（讓 T079 通過）
- [x] T081 [P] [US4] 先寫測試（確認失敗）：圖片前處理（過大圖縮小、不支援格式的錯誤訊息），於 `web/tests/image-input.test.ts`
- [x] T082 [US4] 實作 `web/src/lib/image.ts` 與 `web/src/routes/ImagePage.svelte`：從相簿或相機選圖、顏色多寡與簡化程度滑桿、在 Web Worker 中轉換並顯示進度（FR-024）
- [x] T083 [US4] Playwright 端對端：上傳 3 色圖、預覽、調整滑桿、輸出，於 `web/tests/e2e/image-flow.spec.ts`（先寫、先失敗）
- [x] T084 [US4] 將圖片樣本納入 `scripts/verify.sh`，確認 pyembroidery 可讀回

**Checkpoint**: User Stories 1～4 皆可獨立運作

---

## Phase 7: User Story 5 - 離線與重開後保留設定 (Priority: P2)

**Goal**: 曾載入一次後可離線使用，可加到主畫面，重開後保留繡框、預設格式與最近設計。

**Independent Test**: 開啟一次後斷網重新整理，文字流程完整可用；自訂框與預設格式仍在。

- [x] T085 [P] [US5] 先寫測試（確認失敗）：最近設計儲存（IndexedDB，以 `idb-keyval`），於 `web/tests/designs.test.ts`；儲存失敗時介面仍可運作
- [x] T086 [US5] 實作 `web/src/lib/designs.ts` 與「最近的設計」清單元件（讓 T085 通過）
- [x] T087 [US5] 設定 PWA：`web/vite.config.ts`（`vite-plugin-pwa` 預先快取 WASM、字型與靜態資源）、`web/public/manifest.webmanifest`、圖示 `web/public/icons/`（`display: standalone`、繁體中文名稱）
- [x] T088 [US5] Playwright 端對端：載入後 `context.setOffline(true)`，重新整理後文字流程（輸入、預覽、選框、輸出）全部可完成，且自訂框與預設格式仍在，於 `web/tests/e2e/offline.spec.ts`（先寫、先失敗）

**Checkpoint**: 所有使用者故事皆可獨立運作

---

## Phase 8: Polish & Cross-Cutting

**Purpose**: 跨故事的收尾

- [x] T089 [P] 效能基準測試：20 字內文字預覽 < 1 秒（SC-002）、簡單圖片轉換 < 10 秒（SC-007），於 `crates/stitch-wasm/benches/` 與 `web/tests/perf.test.ts`；未達標則最佳化
- [x] T090 [P] 體積預算檢查：WASM 與字型總量，並在 `scripts/check.sh` 加上上限警示
- [x] T091 [P] 繁體中文使用說明 `README.md`（專案簡介、開發、驗證、給媽媽的測試清單入口）
- [x] T092 [P] 稽核 `cargo clippy`、`cargo fmt`、前端型別檢查與 lint 全部乾淨
- [x] T093 對照 spec.md 的 FR-001～FR-024 與 SC-001～SC-008，逐條勾稽測試或標明「需實機」（SC-001 與 SC-006、SC-008 需媽媽實測）
- [x] T094 執行 `quickstart.md` 第一至四節全部驗證
- [x] T095 更新知識庫：把實測結論與決策寫入 `knowledge/`（`draft/2026-10-17-B70-檔案格式.md` 標示待媽媽實測；`vision.md` 的階段 1 勾稽進度）

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup（Phase 1）**: 無相依，可立即開始
- **Foundational（Phase 2）**: 相依於 Setup；**阻擋所有使用者故事**
- **US1（Phase 3）**: 相依於 Foundational；是 MVP
- **US2（Phase 4）**: 相依於 Foundational；整合 US1 的適配與頁面（T062 依賴 T053）
- **US3（Phase 5）**: 相依於 Foundational；整合 US1 的匯出（T071 依賴 T054）
- **US4（Phase 6）**: 相依於 Foundational 與 US1 的針跡生成（T034、T036、T038、T042）
- **US5（Phase 7）**: 相依於 US2 與 US3 的儲存機制（T059、T069）
- **Polish（Phase 8）**: 相依於所有想交付的故事

### Within Each Story

- 測試任務 MUST 先寫並確認失敗，才可開始對應的實作任務（憲章原則一）
- 型別與核心 → 針跡生成 → 綁定 → 前端
- 每個故事完成後先通過 Checkpoint 再進入下一個

### Parallel Opportunities

- Setup 的 T002～T005 可平行
- Foundational 中不同 crate 的「測試任務」可平行（T006、T008、T010、T012、T014、T016、T018、T023、T025）
- US1 內文字、針跡生成、前端字串／預覽／復原的測試任務可平行
- US4 的 `stitch-image` 工作可與 US2、US3 的前端工作平行

---

## Parallel Example: Phase 2 的測試任務

```bash
# 同時撰寫並確認失敗（各自不同檔案）：
Task: "T006 stitch-core 型別測試 in crates/stitch-core/tests/types.rs"
Task: "T008 繡框測試 in crates/stitch-core/tests/hoop.rs"
Task: "T016 DST 測試 in crates/stitch-formats/tests/dst.rs"
Task: "T018 EXP 測試 in crates/stitch-formats/tests/exp.rs"
Task: "T023 幾何測試 in crates/stitch-geom/tests/geom.rs"
```

---

## Implementation Strategy

### MVP First（只做 US1）

1. 完成 Phase 1 Setup
2. 完成 Phase 2 Foundational（關鍵，阻擋所有故事）
3. 完成 Phase 3 US1
4. **停下來驗證**：`scripts/check.sh` 與 `scripts/verify.sh` 通過；在桌面瀏覽器手機視窗走完文字流程
5. 此時已有可交給媽媽試用的初版

### Incremental Delivery

1. Setup + Foundational → 基礎就緒
2. US1 → 獨立驗證（MVP）
3. US2 → 獨立驗證（繡框安全，防止撞針）
4. US3 → 獨立驗證（多格式、可實機測試）→ **此時可交給媽媽做 B70 實機測試**
5. US4、US5 → 依實測回饋決定優先順序

### 實機測試交接點

完成 US3（T074）後，即可依 quickstart.md 第五節把成果交給媽媽。她的回報會決定：
- 預設輸出格式（可能新增 PES／JEF 等，只需新增格式實作並註冊，不動既有程式）
- 檔名規則是否過於保守
- 最小字高、筆畫寬、安全邊距等假設值的校正

---

## Notes

- [P] 任務 = 不同檔案、無相依
- 驗證步驟：每個測試任務完成後 MUST 實際執行並確認「因正確的原因失敗」，才開始實作
- 不得以刪除或跳過測試來使其通過（憲章原則一）
- 每個任務或邏輯群組完成後建議提交（專案目前尚未使用 git，可先 `git init`）
- 在每個 Checkpoint 停下來獨立驗證該故事
- 避免：模糊任務、同檔案衝突、破壞故事獨立性的跨故事相依
