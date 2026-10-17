# 字型

本專案內嵌的字型必須是授權允許**嵌入、子集化、散布**的開源字型。每個字型的授權文字放在 `LICENSES/`。

| 字型 | 用途 | 來源 | 授權 | 已確認 |
|---|---|---|---|---|
| Noto Sans TC | 黑體 | Google Fonts（google/fonts） | SIL OFL 1.1 | 2026-10-17 |
| Noto Serif TC | 宋體 | Google Fonts（google/fonts） | SIL OFL 1.1 | 2026-10-17 |
| jf-openhuninn 2.1（粉圓體） | 圓體 | justfont/open-huninn-font | SIL OFL 1.1 | 2026-10-17 |
| LXGW WenKai TC 1.522（霞鶩文楷 TC） | 楷體 | lxgw/LxgwWenkaiTC | SIL OFL 1.1 | 2026-10-17 |

確認方式：各字型內嵌的授權欄位（name 表 ID 13／14）皆為 SIL OFL 1.1，並與來源倉庫的授權文字一致。

## 使用條件（OFL 1.1 摘要）

- 可以內嵌、子集化（修改）與散布；必須隨附授權文字（`LICENSES/`）。
- 字型本身不可單獨販售。
- 修改過的版本不得使用原字型的「保留字型名稱」（例如 Noto Sans TC 的 `Source`）。
  本專案的子集檔以 `*-subset.ttf` 命名，僅供本應用程式內部使用。

## 子集化

`*-subset.ttf` 由 `tools/subset_fonts.py` 產生：

- 範圍：Big5 第一級常用漢字（5401 字）加 ASCII、全形與中文標點。不在範圍內的字會退回其他字型，
  都沒有時提示使用者。
- 可變字型（Noto 系列）固定在字重 500（筆畫較粗，對刺繡較有利）。
- 驗收：`tools/tests/test_subset.py`（涵蓋度與單檔上限 4MB）。

重新產生：

```bash
uv run --no-project --with fonttools --with brotli python -I tools/subset_fonts.py <來源字型資料夾>
uv run --no-project --with fonttools python -I tools/tests/test_subset.py
```
