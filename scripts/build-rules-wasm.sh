#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../crates/raso-rules"
wasm-pack build --release --target web --out-dir pkg --out-name raso_rules -- --features wasm
ls -la pkg/raso_rules_bg.wasm
