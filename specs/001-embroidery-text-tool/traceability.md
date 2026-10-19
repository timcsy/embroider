# 需求對照表（Traceability）

每個需求由哪個自動化測試保證。**沒有自動化覆蓋的，明確標示「需實機」或「需媽媽試用」**，不假裝已驗證。
最後更新：2026-10-17。

圖例：✅ 已由自動化測試保證　🟡 部分（見備註）　🧑 需要真人／實機

測試位置縮寫：`core`＝`crates/stitch-core/tests`、`gen`＝`crates/stitch-gen/tests`、`text`＝`crates/stitch-text/tests`、
`image`＝`crates/stitch-image/tests`、`fmt`＝`crates/stitch-formats/tests`、`wasm`＝`crates/stitch-wasm/tests`、
`web`＝`web/tests`、`e2e`＝`web/tests/e2e`、`verify`＝`scripts/verify.sh`（pyembroidery 交叉驗證）。

## 功能需求

| 需求 | 狀態 | 由什麼保證 | 備註 |
|---|---|---|---|
| FR-001 文字輸入與即時預覽 | ✅ | `text/text.rs`、`text/pipeline.rs`、`e2e/text-flow` | 輸入後 150ms 去抖再重算 |
| FR-002 字型授權 | ✅ | `fonts/LICENSES/`、`tools/tests/test_subset.py` | 四個字型皆 SIL OFL 1.1，已逐一確認 |
| FR-003 調整大小與線色 | ✅ | `e2e/text-flow`（拖大小、換線色）、`text/pipeline.rs` | |
| FR-004 接近真實的預覽 | 🟡 | `e2e`（預覽畫布確實畫出對應線色）、`web/preview.test.ts` | 「像不像真的」是主觀判斷，已人工看過截圖；真實觀感要媽媽確認 🧑 |
| FR-005 過小細節偵測 | ✅ | `gen/min_size.rs`、`gen/design.rs`、`image/pipeline.rs` | 預設字高 ≥ 4mm、筆畫 ≥ 0.8mm，是**假設值** 🧑 |
| FR-006 內建 B70 三種框 | ✅ | `core/hoop.rs`、`web/hoops.test.ts`、`e2e/hoop-flow` | 尺寸取自零售商公開資訊 |
| FR-007 新增／編輯／刪除自己的框 | ✅ | `web/hoops.test.ts`、`web/hoop-form.test.ts`、`e2e/hoop-flow` | 原廠框不可刪除或改尺寸 |
| FR-008 安全邊距 | ✅ | `core/hoop.rs`、`core/validate.rs`、`gen/fit*.rs`（含屬性測試） | 預設 5mm |
| FR-009 超出範圍自動縮放或阻止 | ✅ | `gen/design.rs`、`gen/fit_shapes.rs`、`text/pipeline.rs`、`e2e/hoop-flow` | |
| FR-010 輸出刺繡檔 | ✅ | `wasm/text_preview.rs`、`e2e/text-flow`、`verify` | 下載到手機；進 USB 的步驟 🧑 |
| FR-011 DST、EXP 且可擴充 | ✅ | `fmt/dst.rs`、`fmt/exp.rs`、`fmt/registry.rs`（註冊假格式不影響既有輸出） | |
| FR-012 一次輸出多格式、安全檔名 | ✅ | `fmt/filename.rs`（屬性測試）、`wasm/text_preview.rs`、`e2e/formats-flow` | 機器是否接受這個檔名規則 🧑 |
| FR-013 記住預設格式 | ✅ | `web/prefs.test.ts`、`e2e/formats-flow` | |
| FR-014 實機測試說明與一鍵設定 | ✅ | `e2e/formats-flow`、`routes/TryOnMachine.svelte` | 說明文字是否淺白 🧑 |
| FR-015 選圖（相簿或拍照） | 🟡 | `web/image-input.test.ts`、`e2e/image-flow`（模擬選檔） | 「拍照」按鈕未在真手機上試過 🧑 |
| FR-016 調整顏色多寡與簡化程度 | ✅ | `image/*`、`e2e/image-flow`（拖顏色、簡化） | 細線走平針、窄柱走緞面由 `gen/stroke.rs` 守著；圖片預設大小 `web/fit-size.test.ts`；圈範圍 `web/crop.test.ts`、`e2e/image-flow` |
| FR-017 不暴露技術術語 | ✅ | `web/strings.test.ts`（掃過所有介面字串） | 檔案副檔名只出現在「試試看機器讀得到哪一種」頁（來自核心的格式說明） |
| FR-018 白話錯誤訊息 | ✅ | 各層 `*.rs`／`web/*.test.ts` 對錯誤碼與中文訊息的斷言、`e2e` 的 alert 檢查 | |
| FR-019 可復原 | ✅ | `web/history.test.ts`、`e2e/text-flow`（上一步／下一步） | 選圖不在復原範圍內（刻意，像素太大） |
| FR-020 離線與加到主畫面 | 🟡 | `e2e/offline`（真的斷網後重整仍可用、有 manifest 與圖示） | 實際「加到主畫面」要在真手機上做 🧑 |
| FR-021 本機保存設定 | ✅ | `web/hoops.test.ts`、`web/prefs.test.ts`、`web/designs.test.ts`、`e2e/offline` | 儲存空間壞掉時退回記憶體，不當機 |
| FR-022 資料不離開裝置 | ✅ | `e2e/text-flow`（全程沒有任何對外連線） | |
| FR-023 輸出檔可被獨立程式讀回且一致 | ✅ | `verify`（pyembroidery 讀回 DST 與 EXP，逐點比對；故意改錯能被抓到） | 只證明格式正確，不證明機器讀得到 |
| FR-024 長運算顯示進度、不凍結 | 🟡 | 運算在 Web Worker；畫面顯示「正在更新…」；`web/perf.test.ts` 量運算時間（三次取最快，避免並行時誤報） | 沒有自動測「主執行緒不卡」；目前運算只要幾十毫秒 |

## 編輯器（階段 2，路線圖見 `knowledge/draft/2026-10-19-編輯器路線圖.md`）

| 能力 | 狀態 | 由什麼保證 | 還不確定 |
|---|---|---|---|
| 多物件（文字、圖片、形狀、手繪）各自位置、旋轉、縮放、翻轉並一起放進繡框 | ✅ | `crates/stitch-compose/tests/compose.rs`（33 個）、`wasm/tests/api.rs`、`verify` 對輸出檔讀回 | 重疊時後加的在上面；文字壓在圖上的提示沒做 |
| 拖移、拉角縮放、轉方向、兩指縮放旋轉、滾輪、方向鍵 | 🟡 | `web/geometry.test.ts`、`e2e/editor-flow`（滑鼠、真的雙指觸控事件） | 真手機上的手感 🧑 |
| 復原、刪除可拿回、複製、上下層、置中 | ✅ | `web/editor.test.ts`、`e2e/editor-flow` | |
| 整排重複、圍成一圈、整組一起動／縮放／旋轉、鏡像、邊框 | ✅ | `web/editor.test.ts`、`e2e/editor-flow` | 整排重疊時的針跡厚度 🧑 |
| 弧形文字 | ✅ | `compose.rs`（拱形與笑臉形兩端高低）、`e2e/editor-flow` | 彎得很大時字的間距看起來是否舒服 🧑 |
| 手繪（抹順、粗細、封閉填滿） | ✅ | `compose.rs`、`web/editor.test.ts`、`e2e/editor-flow` | 平針與緞面的實繡效果 🧑 |
| 畫布放大平移、拖動時即時數字、對齊參考線與吸附、鎖定、多選（Shift 加選、框選、多選模式）、對齊與等距與同樣大小、綁成一組 | ✅ | `web/geometry.test.ts`、`web/editor.test.ts`、`e2e/editor-flow`（含真的雙指觸控事件） | 真機上的手感 🧑 |
| 做到一半自動儲存、重開接著做、全部清掉可用上一步拿回 | ✅ | `web/designs.test.ts`（存、讀、覆蓋、清掉、不混進最近清單）、`e2e/editor-flow`（重新整理後還在、圖片設計也能接著做） | 手機把瀏覽器關掉後的行為與重新整理相同，真機上確認 🧑 |
| 電腦版：上方工具列（加東西、上一步、存檔）、畫布與工具區各自捲動、整頁不捲、鍵盤快捷鍵（Ctrl+Z／Shift+Z／Y／D、Delete）、游標隨位置變化 | ✅ | `e2e/editor-flow`（電腦版 1440×900 四個測試） | 實機的滑鼠手感 🧑 |
| 手機、平板直放、平板橫放、電腦版面與按鈕夠大 | ✅ | `e2e/editor-flow`（四種視窗：無橫向捲動、所有按鈕高度 ≥44px） | 真實裝置 🧑 |

## 成功標準

| 標準 | 狀態 | 由什麼保證 / 目前結果 |
|---|---|---|
| SC-001 媽媽 5 分鐘內完成文字流程 | 🧑 | 需要媽媽實際使用。自動化只能證明流程可走完 |
| SC-002 預覽 1 秒內（中階手機） | 🟡 | `web/perf.test.ts` 預算 300ms（開發機），實測 20～100ms；手機實測 🧑 |
| SC-003 輸出檔 100% 可被讀回且一致 | ✅ | `verify` 對 6 組樣本（文字中英、圖片兩組、方形）、DST 與 EXP |
| SC-004 超出範圍的設計 100% 不輸出 | ✅ | `gen/fit_shapes.rs` 屬性測試、`core/validate.rs`、`wasm` 的 `encode` 內再驗證一次 |
| SC-005 斷網後完整流程可完成 | ✅ | `e2e/offline`（含圖片流程） |
| SC-006 B70 實機讀得到並繡出 | 🧑 | **唯一的驗收就是實機**。見 `quickstart.md` 第五節 |
| SC-007 簡單圖案 10 秒內（中階手機） | 🟡 | `web/perf.test.ts` 預算 1.5 秒（開發機），實測約 30ms；手機實測 🧑 |
| SC-008 媽媽自己完成格式測試與設定預設 | 🧑 | 需要媽媽實際使用 |

## 憲章

| 原則 | 狀態 | 說明 |
|---|---|---|
| I. TDD | ✅ | 每個任務都先寫測試並確認失敗再實作；過程中遇到的失敗記錄在對話與 `tasks.md` 的實作偏差註記 |
| II. 繁體中文 | ✅ | 規格、計畫、任務、README、介面與錯誤訊息皆為繁體中文 |
