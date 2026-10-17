# 資料模型

所有長度單位為 **0.1mm 的整數**（`i32`）。例如 160mm 以 `1600` 表示。

## 實體

### Hoop（繡框）

| 欄位 | 型別 | 規則 |
|---|---|---|
| id | 字串 | 唯一；內建框使用固定 id（`b70-large`、`b70-medium`、`b70-small`） |
| name | 字串 | 非空，≤ 20 字 |
| width | 正整數 | 可繡寬，> 0；上限 3000（300mm） |
| height | 正整數 | 可繡高，> 0；上限 3000 |
| shape | 列舉 | `rect`、`circle`、`ellipse`；圓形時 width = height |
| margin | 非負整數 | 安全邊距，預設 50（5mm）；必須 < min(width, height) / 2 |
| builtin | 布林 | 內建為 true，不可刪除或修改尺寸 |

**衍生**：`safe_area()` = 可繡區內縮 margin 後的區域。**驗證**：設計所有針點必須落在 `safe_area()` 內。

### Design（設計）

| 欄位 | 型別 | 規則 |
|---|---|---|
| id | 字串 | 唯一 |
| name | 字串 | 使用者看到的名稱（可含中文） |
| elements | 元素陣列 | 至少一個才能輸出 |
| hoop_id | 字串 | 目前選用的繡框 |
| scale | 浮點 | 自動縮放比例，≤ 1 為縮小 |
| updated_at | 時間 | 排序最近設計 |

### TextElement（文字元素）

| 欄位 | 型別 | 規則 |
|---|---|---|
| text | 字串 | 非空白；繁中、英文、數字、常見標點 |
| font_id | 字串 | 必須存在於字型清單 |
| height | 正整數 | 字高；低於最小可繡字高時自動放大 |
| color | 線色 | 見 ThreadColor |
| layout | 列舉 | `single_line`、`wrap` |
| position | 點 | 相對設計中心 |

### ImageElement（圖片元素）

| 欄位 | 型別 | 規則 |
|---|---|---|
| source | 位元組 | 解碼後 RGBA，僅存本機 |
| width, height | 正整數 | 像素 |
| color_count | 整數 | 2～12；預設依圖片內容建議 |
| simplify | 整數 | 0～100，簡化程度 |
| shapes | 色塊陣列 | 轉換結果：每塊含多邊形（含洞）與 ThreadColor |

### ThreadColor（線色）

| 欄位 | 型別 | 規則 |
|---|---|---|
| rgb | 三個 u8 | 預覽用顏色 |
| name | 字串 | 對使用者顯示的中文色名（如「正紅」「米白」） |

### OutputFormat（輸出格式）

| 欄位 | 型別 | 規則 |
|---|---|---|
| id | 字串 | `dst`、`exp`；唯一 |
| extension | 字串 | 小寫副檔名，不含點 |
| label | 字串 | 對使用者顯示的易懂名稱（如「通用格式 A」） |
| max_step | 整數 | 單針最大位移（0.1mm 單位）；DST 為 121，EXP 為 127 |

格式清單由註冊表提供；新增格式不改動其他實體。

### Preferences（使用者設定）

| 欄位 | 型別 | 規則 |
|---|---|---|
| default_formats | 格式 id 陣列 | 空代表「尚未選定，全部輸出」 |
| last_hoop_id | 字串 | 最近使用的框 |
| min_text_height | 整數 | 最小可繡字高，預設 40（4mm） |
| min_stroke_width | 整數 | 最小筆畫寬，預設 8（0.8mm） |

### StitchPlan（針跡計畫）

| 欄位 | 型別 | 規則 |
|---|---|---|
| stitches | 針點陣列 | 每個含 x、y（絕對座標）與類型 |
| colors | 線色陣列 | 依換色順序 |

針點類型：`Stitch`（一般針）、`Jump`（跳針）、`Trim`（剪線）、`ColorChange`（換色）、`End`（結束）。

**不變式**：
1. 所有 `Stitch` 與 `Jump` 的相鄰位移，輸出時會依格式 `max_step` 拆分；
2. 全部針點落在所選框的 `safe_area()` 內；
3. 以 `End` 結尾；顏色數與 `ColorChange` 次數一致。

## 關係

```
Design 1 ──* Element（Text 或 Image）
Design * ── 1 Hoop
Design ──生成──> StitchPlan ──經 OutputFormat 編碼──> 輸出檔案（每種格式一個）
Preferences ── 指向 ── OutputFormat、Hoop
```

## 狀態轉換

- **Design**：草稿（編輯中）→ 可輸出（驗證通過）→ 已輸出（記錄輸出時間）；任何編輯回到草稿並重新驗證。
- **Preferences.default_formats**：空（首次，全部輸出）→ 使用者選定（只輸出所選）→ 可隨時改回空。

## 驗證規則總表

| 規則 | 來源需求 |
|---|---|
| 設計必須完全落在安全範圍內，否則自動縮放或阻止 | FR-008、FR-009、SC-004 |
| 低於最小字高或筆畫寬時自動放大或提示 | FR-005 |
| 內建框不可刪除或修改尺寸 | FR-007 |
| 框的尺寸須在合理範圍內 | 規格邊界情況 |
| 空白文字不可輸出 | 規格邊界情況 |
