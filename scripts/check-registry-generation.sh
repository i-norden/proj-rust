#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --manifest-path gen-reference/Cargo.toml --bin gen-registry
cargo run --manifest-path gen-reference/Cargo.toml --bin gen-registry -- --check
