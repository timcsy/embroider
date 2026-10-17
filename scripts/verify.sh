#!/usr/bin/env bash
# 以 CLI 產生樣本，再用獨立實作（pyembroidery）讀回，逐點比對原始計畫。
# 只證明檔案格式正確，不證明 B70 能讀。
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=out
rm -rf "$OUT"
CLI="cargo run -q --release -p stitch-cli --"
$CLI sample --shape square --hoop b70-small --out "$OUT/square" >/dev/null
$CLI sample --text "Amy 2026" --hoop b70-medium --out "$OUT/en" >/dev/null
$CLI sample --text "王小明" --font kai --height 200 --hoop b70-medium --out "$OUT/zh" >/dev/null
$CLI sample --text "媽媽生日快樂" --font round --height 150 --hoop b70-large --out "$OUT/zh2" >/dev/null
$CLI sample --image web/tests/e2e/fixtures/logo.png --hoop b70-medium --size 700 --out "$OUT/img" >/dev/null
$CLI sample --image web/tests/e2e/fixtures/logo.png --colors 6 --simplify 70 --hoop b70-large --size 1200 --out "$OUT/img2" >/dev/null
status=0
for d in "$OUT"/*/; do
  echo "== $d"
  uv run --no-project --with pyembroidery python -I tools/verify_outputs.py "$d" || status=1
done
exit $status
