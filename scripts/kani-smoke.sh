#!/usr/bin/env bash
# clawd-kani SMOKE profile: a fast subset of harnesses that must pass on
# every change. Full coverage lives in kani-full.sh.
#
# Prerequisite (once): cargo install --locked kani-verifier && cargo kani setup
#
# Usage: scripts/kani-smoke.sh [extra cargo-kani args...]
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$CRATE_DIR"

# --harness may be repeated; a harness runs if it matches ANY filter.
cargo kani \
    --harness proof_fee_sweep_conservation \
    --harness proof_writeoff_conservation \
    --harness proof_haircut_ratio_bounds \
    --harness proof_funding_zero_cases \
    --harness proof_warmup_zero_at_start \
    --harness proof_fee_sweep_clears_when_sufficient \
    "$@"
