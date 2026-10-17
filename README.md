# 刺繡小幫手（embroider）

在手機瀏覽器裡打字或選一張圖，轉成可以放進 USB 隨身碟、給縫紉機繡的檔案。
為一位懂刺繡與拼布、但不熟電腦的長輩設計：全程繁體中文、大按鈕、不出現技術術語、可以離線使用。

目前的目標機器是 **bernette B70**。

## 現在能做什麼

- **打字刺繡**：繁體中文、英文、數字；四種字型（黑體、宋體、圓體、楷體）；大小與 16 種線色；即時預覽。
  長句子一行放不下時，自動分成幾行。
- **選圖刺繡**：從相簿選圖或拍照，色塊分明的圖案（卡通、標誌）效果最好；可調顏色多寡與簡化程度。
- **繡框**：內建 B70 原廠三種；可以新增自己的框（方形、圓形、橢圓），重開後仍在。
  設計超出框的範圍時，自動縮小或擋下並說明，**絕不輸出會讓針打到框的檔案**。
- **輸出**：一次存出多種檔案格式（因為還不確定 B70 讀得到哪一種），
  檔名只用大寫英數字（例如 `AMY20001.DST`），第一次先每一種都存，試出能用的之後記住預設。
- **離線可用**、可加到手機主畫面；圖片與設計只在手機上處理，不會上傳。

## 還沒有確認的事（需要在 B70 實機上試）

1. **B70 讀得到哪一種檔案格式**：網路上查不到明確資料，程式裡也沒有猜。
   介面裡有「試試看機器讀得到哪一種」的說明與按鈕，步驟見
   [`specs/001-embroidery-text-tool/quickstart.md`](specs/001-embroidery-text-tool/quickstart.md) 第五節。
2. 輸出檔名與資料夾是否被機器接受（目前採最保守的規則）。
3. 最小字高、筆畫寬度、安全邊距等數值是否適合（目前是經驗值，可調整）。
4. 手機把檔案存進 USB 的實際流程（Android Chrome 的下載位置、OTG 隨身碟）。

自動化測試與獨立的讀檔工具（pyembroidery）能證明「檔案格式本身正確、內容與預覽一致」，
但**不能證明機器讀得到**，這一步只能靠實機。

## 開發

### 需要

- Rust stable（1.94 以上）、`wasm-pack`、`wasm32-unknown-unknown` 目標
- Node.js 24、npm
- `uv`（用來執行 Python 的字型子集化與 pyembroidery 交叉驗證）

### 建置與執行

```bash
cd web
npm install
npm run build:wasm      # 把 Rust 核心編成 WASM
npm run dev             # 開發伺服器（會自動複製字型）
npm run build           # 正式版（含離線快取）
npm run preview         # 預覽正式版
```

`web/dist` 是純靜態檔、網址全部相對，可以放在任何網站的任何子路徑（例如 GitHub Pages）。
「加到主畫面」與離線只在 HTTPS（或 localhost）網址下有效。

### 驗證

```bash
./scripts/check.sh      # Rust：格式、靜態檢查（無警告）、全部測試、體積預算
./scripts/verify.sh     # 以獨立實作（pyembroidery）讀回輸出檔，逐點比對
cd web
npx svelte-check --tsconfig ./tsconfig.app.json
npm test                # 前端單元測試
npm run test:e2e        # 手機視窗的端對端測試（第一次需要 npx playwright install chromium）
```

開發方式是**測試先行（TDD）**，見 [`.specify/memory/constitution.md`](.specify/memory/constitution.md)。

### 專案結構

```
crates/
  stitch-core/      型別、繡框與安全範圍、計畫驗證、線色色票
  stitch-geom/      多邊形、曲線展平、掃描線、偏移
  stitch-gen/       針跡生成：填充、底針、輪廓、跳針與固定針、最小尺寸、縮放適配
  stitch-text/      字型、排版（單行／自動分行）、文字 → 針跡計畫
  stitch-image/     色彩分群、像素區域 → 多邊形、圖片 → 針跡計畫
  stitch-formats/   檔案格式註冊表、DST、EXP、檔名規則
  stitch-wasm/      給瀏覽器的薄綁定層
  stitch-cli/       開發用：產生樣本檔供交叉驗證（不隨產品發佈）
web/                手機優先的 PWA（Svelte 5 + Vite）
fonts/              子集化後的開源字型與授權
tools/              字型子集化、圖示產生、pyembroidery 交叉驗證
specs/              規格、計畫、任務（Spec Kit）
knowledge/          專案的「為什麼」（knowie）
```

核心是純函式：相同輸入與設定永遠得到相同輸出；座標單位統一為 0.1mm 整數。

### 新增一種輸出格式

實作 `stitch_formats::Format` 並在 `Registry::default()` 註冊即可，其他程式與使用者的設定都不用改。
務必同時加上往返測試，並把它納入 `scripts/verify.sh` 的交叉驗證。

## 授權

程式碼：**MIT OR Apache-2.0**，擇一即可，授權文字見 [LICENSE-MIT](LICENSE-MIT) 與 [LICENSE-APACHE](LICENSE-APACHE)。

字型：四個內嵌字型皆為 SIL Open Font License 1.1，授權文字在 [`fonts/LICENSES/`](fonts/LICENSES/)，說明見 [`fonts/README.md`](fonts/README.md)。
