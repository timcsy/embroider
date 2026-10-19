#!/usr/bin/env bash
# 體積預算：手機第一次載入要下載、之後離線快取，不能無限制變大。
# 超過任何一項就失敗。
set -euo pipefail
cd "$(dirname "$0")/.."

WASM_MAX=$((1024 * 1024))          # 核心 WASM：1MB
FONT_MAX=$((4 * 1024 * 1024))      # 單一字型：4MB
FONTS_TOTAL_MAX=$((12 * 1024 * 1024)) # 字型總量：12MB
JS_MAX=$((300 * 1024))             # 前端程式（未壓縮）：300KB

# GNU 的 stat 用 -c，BSD（macOS）用 -f；GNU 的 -f 是檔案系統資訊（會印出別的東西卻成功），所以要先試 -c
size() { stat -c %s "$1" 2>/dev/null || stat -f %z "$1"; }
fail=0
check() { # 名稱 實際 上限
  local pct=$(( $2 * 100 / $3 ))
  printf '%-22s %9d / %9d bytes (%d%%)\n' "$1" "$2" "$3" "$pct"
  if [ "$2" -gt "$3" ]; then echo "  ✗ 超過預算：$1"; fail=1; fi
}

wasm=web/src/lib/wasm/stitch_wasm_bg.wasm
if [ -f "$wasm" ]; then check "核心 WASM" "$(size "$wasm")" "$WASM_MAX"; else echo "（略過 WASM：尚未建置）"; fi

total=0
for f in fonts/*.ttf; do
  s=$(size "$f"); total=$((total + s))
  check "$(basename "$f")" "$s" "$FONT_MAX"
done
check "字型總量" "$total" "$FONTS_TOTAL_MAX"

if ls web/dist/assets/index-*.js >/dev/null 2>&1; then
  js=0; for f in web/dist/assets/index-*.js; do js=$((js + $(size "$f"))); done
  check "前端程式 (index)" "$js" "$JS_MAX"
else
  echo "（略過前端程式：尚未建置 web/dist）"
fi
exit $fail
