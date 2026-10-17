#!/usr/bin/env bash
# 依序執行格式、靜態檢查與測試。任何一步失敗即停止。
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/size-budget.sh
