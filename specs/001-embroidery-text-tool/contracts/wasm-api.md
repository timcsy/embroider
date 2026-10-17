# 契約：核心（WASM）與前端的介面

`stitch-wasm` 對前端暴露的函式。所有函式為**純函式**：相同輸入得到相同輸出，不持有隱藏狀態。
資料以 JSON 相容的結構傳遞（`serde`），長度單位一律為 0.1mm 整數。錯誤以結構化的 `UserMessage`
回傳，內容已是繁體中文白話，前端直接顯示。

## 共用型別

```text
UserMessage { code: string, text: string, hint: string | null }
  // code 供程式判斷；text 是給使用者的白話；hint 是下一步建議

Point { x: i32, y: i32 }
Hoop  { id, name, width, height, shape, margin, builtin }       // 見 data-model.md
ThreadColor { r, g, b, name }
```

## 函式

### `list_fonts() -> FontInfo[]`
回傳可用字型：`{ id, display_name, covers_chars_note }`。

### `list_builtin_hoops() -> Hoop[]`
回傳內建三種框。

### `validate_hoop(hoop: Hoop) -> { ok: bool, message: UserMessage | null }`
檢查新增或編輯的框是否合理（尺寸範圍、形狀規則、邊距）。

### `build_text_preview(req: TextRequest) -> PreviewResult`
```text
TextRequest  { text, font_id, height, color: ThreadColor, layout, hoop: Hoop, settings: Settings }
PreviewResult {
  plan: StitchPlan,          // 針跡，前端用來畫預覽
  fitted_scale: f32,         // 為了放進框而套用的縮放，1.0 為不變
  adjustments: UserMessage[],// 自動調整說明（例如「字太小，已放大」）
  blocked: UserMessage | null// 非空代表不可輸出（例如放不進框）
}
```
前置條件：`text` 非空白。後置條件：若 `blocked` 為空，`plan` 的全部針點落在框的安全範圍內。

### `build_image_preview(req: ImageRequest) -> PreviewResult`
```text
ImageRequest { rgba: bytes, width, height, color_count, simplify, hoop: Hoop, settings: Settings }
```
長時間運算：前端必須在 Web Worker 中呼叫，避免凍結介面。

### `list_formats() -> OutputFormatInfo[]`
回傳註冊的輸出格式：`{ id, extension, label, description }`。

### `encode(plan: StitchPlan, format_id: string, design_label: string) -> EncodedFile`
```text
EncodedFile { file_name: string, bytes: bytes }
```
前置條件：`plan` 已通過安全範圍驗證（函式內再次驗證，不信任前端）。
失敗：回傳 `UserMessage`（例如超出範圍）。檔名規則見 [output-files.md](./output-files.md)。

### `encode_all(plan, format_ids: string[], design_label) -> EncodedFile[]`
一次產生多種格式。`format_ids` 為空代表使用所有已註冊格式。

### `decode(format_id, bytes) -> StitchPlan`
僅供測試與驗證使用（往返測試），產品介面不暴露給使用者。

## 契約測試

每個函式至少有：
1. 正常輸入的 golden 測試；
2. 邊界輸入（空白、超大、超出範圍）的錯誤測試；
3. 屬性測試：任意合法輸入下，輸出不超出安全範圍、單針位移不超過格式上限。

這些測試先於實作撰寫（憲章 I）。
