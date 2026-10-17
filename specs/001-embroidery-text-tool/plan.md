# Implementation Plan: 手機刺繡工具（文字與簡單圖片）

**Branch**: `001-embroidery-text-tool` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-embroidery-text-tool/spec.md`

## Summary

以 Rust 實作刺繡核心（文字轉輪廓、針跡生成、DST/EXP 讀寫、繡框安全範圍、圖片轉色塊），編成 WASM，
由行動優先的 PWA 前端呼叫。核心全為純函式，以 TDD 開發。輸出格式以「格式註冊表」管理，
一次可產生多種格式，因 B70 實機支援格式未知（`knowledge/draft/2026-10-17-B70-檔案格式.md`）。
輸出檔以獨立的讀檔工具（pyembroidery）交叉驗證，作為沒有實機時的最佳替代。

## Technical Context

**Language/Version**: Rust 1.94（stable，edition 2021）；前端 TypeScript 5.x

**Primary Dependencies**:
- Rust：`ttf-parser`（字形輪廓）、`kurbo`（曲線展平）、`i_overlay`（多邊形布林與偏移）、
  `visioncortex`（圖片分群與輪廓追蹤）、`wasm-bindgen`、`serde`、`serde-wasm-bindgen`
- 前端：Vite、Svelte 5、`vite-plugin-pwa`、`idb-keyval`
- 授權：上述皆為 MIT 或 Apache-2.0（已於 crates.io 查證）

**Storage**: 瀏覽器本機：`localStorage`（繡框、偏好）、IndexedDB（最近設計）；無伺服器

**Testing**: `cargo test`（單元、golden、屬性測試 `proptest`）；前端 `vitest` 與 Playwright（手機視窗模擬）；
輸出檔交叉驗證用 Python `pyembroidery`（僅測試工具，不隨產品發佈）

**Target Platform**: Android Chrome（PWA，可離線）；桌面瀏覽器僅作開發用

**Project Type**: web-application（Rust workspace 加前端）

**Performance Goals**: 20 字內文字預覽更新 < 1 秒（中階 Android）；簡單圖片轉換 < 10 秒（SC-002、SC-007）

**Constraints**: 離線可用；WASM 與字型體積需控制（首次載入後快取）；圖片與設計不離開裝置；
內部座標單位 0.1mm 整數

**Scale/Scope**: 單一使用者；單一設計約 5,000～60,000 針；字型 3～4 種

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原則 | 檢查 | 結果 |
|---|---|---|
| I. 測試驅動開發 | 任務清單中每個實作任務前必有失敗測試任務；golden、往返、屬性測試皆列入 | 通過 |
| II. 繁體中文 | 規格、計畫、研究、資料模型、任務皆繁體中文；程式識別字與技術名詞維持原文 | 通過 |

設計後複查（Phase 1 完成）：通過。無需填寫 Complexity Tracking。

## Project Structure

### Documentation (this feature)

```text
specs/001-embroidery-text-tool/
├── plan.md              # 本檔
├── research.md          # Phase 0：技術決策與理由
├── data-model.md        # Phase 1：資料模型
├── quickstart.md        # Phase 1：驗證指南
├── contracts/
│   ├── wasm-api.md      # 核心與前端之間的介面
│   └── output-files.md  # 輸出檔案與檔名契約
└── tasks.md             # Phase 2（/speckit-tasks 產生，本指令不建立）
```

### Source Code (repository root)

```text
Cargo.toml                    # workspace
crates/
├── stitch-core/              # 型別、單位、繡框模型、安全範圍與縮放判斷
├── stitch-formats/           # 格式註冊表；dst、exp 讀寫；單針位移拆分
├── stitch-geom/              # 多邊形、曲線展平、偏移、掃描線求交
├── stitch-gen/               # 針跡生成：填充、底針、輪廓針、固定針、最小尺寸
├── stitch-text/              # 字型載入、排版、字形轉多邊形
├── stitch-image/             # 色彩量化、分群、輪廓追蹤、簡化
├── stitch-wasm/              # wasm-bindgen 薄綁定層（無業務邏輯）
└── stitch-cli/               # 開發用命令列：產生樣本檔供交叉驗證與除錯（不隨產品發佈）
fonts/                        # 子集化後的開源字型與授權檔
web/
├── src/
│   ├── lib/                  # 與核心溝通的包裝、儲存、檔案輸出
│   ├── components/           # 預覽畫布、繡框選擇、格式測試說明
│   └── routes/               # 輸入文字、圖片、輸出、設定
├── tests/                    # vitest 與 Playwright
└── public/                   # PWA manifest、圖示
tools/
└── verify_outputs.py         # 以 pyembroidery 讀回並比對輸出檔
```

**Structure Decision**: Rust workspace 加獨立的 `web/` 前端。核心 crate 依賴方向由下而上
（`core` ← `geom` ← `gen` ← `text`／`image` ← `wasm`；`formats` 只依賴 `core`），
底層不依賴上層。`stitch-wasm` 只做型別轉接。

## Complexity Tracking

無違反事項，不需填寫。
