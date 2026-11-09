#!/usr/bin/env bash
# clawd-kani FULL profile: every harness in the library (including the
# wide-domain `kani-full` variants) plus both end-to-end flow examples.
#
# Prerequisite (once): cargo install --locked kani-verifier && cargo kani setup
#
# The seven fail-to-fix examples are NOT run here: their `vulnerable`
# harnesses are expected to FAIL (that is the lesson). Run them one at a
# time via the command in each example's doc comment.
#
# Usage: scripts/kani-full.sh [extra cargo-kani args...]
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$CRATE_DIR"

echo "== clawd-kani: library harnesses (all features) =="
cargo kani --features kani-full,solana-agent,solana-account-info "$@"

echo "== clawd-kani: flow example 08 (vault settlement) =="
cargo kani --manifest-path examples/08-flow-vault-settlement/Cargo.toml

echo "== clawd-kani: flow example 09 (agent CPI) =="
cargo kani --manifest-path examples/09-flow-agent-cpi/Cargo.toml

echo "== clawd-kani FULL profile: all green =="
