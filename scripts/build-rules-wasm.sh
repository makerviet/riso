#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../crates/riso-rules"
wasm-pack build --release --target web --out-dir pkg --out-name riso_rules -- --features wasm
ls -la pkg/riso_rules_bg.wasm
