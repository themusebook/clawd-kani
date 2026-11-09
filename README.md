# clawd-kani

Reusable [Kani](https://model-checking.github.io/kani/) verification
primitives and proof harnesses for Solana protocol math — the Clawd
percolator stack's own "Kani primitive": machine-checked invariants for
perps risk math, token splits, staking multipliers, CPI allowlists and
agent accounts, without standing up a formal-methods team.

Modeled on the design of
[kamiyo-kani](https://crates.io/crates/kamiyo-kani) and the Percolator
risk engine (<https://github.com/aeyakovenko/percolator>).

## Features

- **Checked protocol math.** The `risk` module implements
  Percolator-style primitives — `haircut_ratio`, `effective_pnl`,
  `warmup_slope`, `fee_debt_sweep`, `funding_payment`, `loss_writeoff`
  — in checked/saturating `u128`/`i128` arithmetic. Each function ships
  with proofs of conservation, bounds and monotonicity.
- **Reusable proof helpers.** `token`, `staking`, `bounds` and `math`
  expose assert-style helpers that wrap your own closures: split
  conservation, monotone staking multipliers, bounded outputs, spec
  equivalence.
- **Constrained generators.** `generators` produces bounded symbolic
  values for common Solana domains: scores, weights, basis points,
  lamports, ordered timestamps, haircut ratios.
- **Agent invariants** (feature `solana-agent`). Symbolic agent
  accounts, lamport conservation, reentrancy checks, CPI allowlists via
  the `cpi_stub!` and `cpi_contract!` macros, PDA seed limits, oracle
  consensus (median-of-3), and FSM transition guards.
- **Symbolic `AccountInfo`** (feature `solana-account-info`). Generates
  real `solana_program::AccountInfo` values from configurable
  constraints, with a `LamportSnapshot` conservation helper.
- **Zero runtime cost.** Everything is gated behind `cfg(kani)`.
  Normal builds compile the crate to nothing.
- **Fail-to-fix examples.** Seven vulnerable/fixed example pairs plus
  two end-to-end flows, each with a runnable `cargo kani` command in
  its doc comment.

## Install

The crate is used as a dev-dependency (it only exists under
`cfg(kani)`, so it never reaches a production build):

```toml
[dev-dependencies]
clawd-kani = { git = "https://github.com/themusebook/clawd-kani" }
```

(or `clawd-kani = { path = "../clawd-kani" }` against a local checkout).
Once published to crates.io this becomes
`clawd-kani = "0.1.0"`. Features:

```toml
clawd-kani = { version = "0.1.0", features = ["solana-agent", "solana-account-info"] }
```

Install Kani once:

```bash
cargo install --locked kani-verifier
cargo kani setup
```

> Toolchain note: building with `solana-account-info` pulls
> `solana-program` 2.3, whose dependency graph needs Cargo ≥ 1.85.
> Kani verification itself always uses Kani's own pinned toolchain.

## Quick start

```rust
#![cfg(kani)]

use clawd_kani::risk::{effective_pnl, haircut_ratio};

#[kani::proof]
fn payout_is_bounded_by_profit() {
    let vault: u128 = kani::any();
    let principal_total: u128 = kani::any();
    let insurance: u128 = kani::any();
    let pnl_pos_total: u128 = kani::any();
    let my_pnl: i128 = kani::any();

    let (h_num, h_den) = haircut_ratio(vault, principal_total, insurance, pnl_pos_total);
    let payout = effective_pnl(my_pnl, h_num, h_den);

    kani::assert(payout <= my_pnl.max(0) as u128, "payout bounded by positive pnl");
}
```

Run it:

```bash
cargo kani -p clawd-kani
```

## Proof profiles

| Profile | Command | What runs |
|---------|---------|-----------|
| smoke | `scripts/kani-smoke.sh` | Six fast harnesses (fee sweeps, writeoff, haircut bounds, funding zero cases, warmup start) — the always-on CI gate. The symbolic-division harnesses live in the default/full profiles: CBMC's 128-bit divider makes them slow on small CI runners. |
| default | `cargo kani` (repo root) | All default-feature harnesses (risk + math helpers). |
| full | `scripts/kani-full.sh` | Everything: `--features kani-full,solana-agent,solana-account-info` (including wide-domain variants) plus both flow examples. |
| stress | `cargo kani --features kani-stress,...` | `kani-full` plus any stress-gated harnesses; manual/weekly. |

The seven fail-to-fix examples are deliberately *not* part of the
profiles: their `vulnerable` harnesses must FAIL. Run each via the
command in its doc comment and watch Kani produce the counterexample,
then see the `fixed` harness verify.

## Harness coverage

All harnesses are `#[kani::proof]` functions. "Profile" says the
weakest profile that runs the harness.

### Core risk proofs (`src/risk.rs`, default features)

| Harness | Property proved | Profile |
|---------|-----------------|---------|
| `proof_haircut_ratio_bounds` | `0 <= h_num <= h_den`, `h_den > 0` | smoke |
| `proof_haircut_ratio_one_when_fully_backed` | `h = 1` when the vault covers principal + insurance + PnL | default |
| `proof_haircut_ratio_zero_when_vault_empty` | `h_num = 0` when the vault is empty | default |
| `proof_effective_pnl_bounded_by_profit` | payout `<= max(pnl, 0)`; 0 for non-positive PnL | default |
| `proof_effective_pnl_bounded_by_haircut` | `payout * h_den <= pnl * h_num` | default |
| `proof_effective_pnl_monotone_in_pnl` | larger profit never pays less | default |
| `proof_effective_pnl_matches_reference` | equals the naive `pnl * h_num / h_den` where that cannot overflow | default |
| `proof_fee_sweep_conservation` | `swept + remaining = debt`, `swept <= available` | smoke |
| `proof_fee_sweep_clears_when_sufficient` | debt fully clears when funds suffice | smoke |
| `proof_funding_long_short_symmetry` | long payment `= -` short payment | default |
| `proof_funding_sign_follows_rate` | positive rate ⇒ longs pay, shorts receive | default |
| `proof_funding_zero_cases` | zero position / rate / denominator ⇒ 0 | smoke |
| `proof_funding_bounded_by_position` | rate `<= 100%` ⇒ `|payment| <= position` | default |
| `proof_writeoff_conservation` | writeoff `<= loss`, `<= insurance`; fund conserved | smoke |
| `proof_writeoff_monotone_in_insurance` | more insurance never writes off less | default |
| `proof_warmup_monotonic_in_elapsed` | warmed amount grows with elapsed time | default |
| `proof_warmup_bounded_by_gross` | warmed amount `<=` gross profit | default |
| `proof_warmup_full_after_period` | full amount once the period has passed | default |
| `proof_warmup_zero_at_start` | nothing withdrawable at `elapsed = 0` | smoke |
| `proof_haircut_ratio_bounds_wide` | haircut bounds over the full `u128` domain | full |
| `proof_writeoff_conservation_wide` | writeoff conservation over the full `u128` domain | full |
| `proof_fee_sweep_conservation_wide` | sweep conservation over the full `u128` domain | full |

### Wide math proofs (`src/math.rs`, default features)

| Harness | Property proved | Profile |
|---------|-----------------|---------|
| `proof_mul_div_floor_matches_naive` | `mul_div_floor` equals `a * b / den` where the product fits | default |
| `proof_mul_div_floor_rem_identity` | `floor * den + rem = a * b` (nothing created or lost) | default |
| `proof_mul_div_ceil_bounds` | ceil is floor or floor+1 and never below the exact value | default |

### Agent proofs (`src/agent.rs`, feature `solana-agent`)

| Harness | Property proved | Profile |
|---------|-----------------|---------|
| `proof_fsm_closed_is_terminal` | no transition out of `Closed` | full |
| `proof_fsm_guard_matches_allowlist` | the FSM guard accepts exactly the allowlisted pairs | full |
| `proof_any_agent_account_respects_config` | the generator honours signer/owner/range/state constraints | full |
| `proof_lamport_conservation_after_transfer` | a transfer between symbolic accounts conserves the total | full |
| `proof_call_guard_blocks_reentry` | nested `enter` is rejected until `exit` | full |
| `proof_cpi_stub_records_and_authorizes` | a stubbed CPI is recorded and passes the allowlist check | full |
| `proof_cpi_contract_enforces_conditions` | a contract CPI records its accounting and is authorized | full |
| `proof_cpi_allowlist_membership` | allowlist test equals explicit membership | full |
| `proof_pda_seed_limits` | seed-layout validity equals the count/length limit checks | full |
| `proof_median3_within_range` | median is one of the inputs, within `[min, max]` | full |
| `proof_oracle_consensus_majority_rules` | two feeds at the reference outvote one 10x outlier | full |

### AccountInfo proofs (`src/account_info.rs`, feature `solana-account-info`)

| Harness | Property proved | Profile |
|---------|-----------------|---------|
| `proof_generator_respects_config` | signer/range/data-length constraints hold | full |
| `proof_generator_rent_exempt_floor` | balance is at least the rent-exempt minimum | full |
| `proof_transfer_conserves_lamports` | a transfer between symbolic `AccountInfo`s keeps the snapshot total | full |
| `proof_snapshot_of_empty_is_zero` | the empty snapshot is 0 and trivially conserved | full |

## Examples

Each example is a standalone mini-crate under `examples/`; run it with
the `cargo kani --manifest-path ...` command in its doc comment.

| Example | Bug class | Fix |
|---------|-----------|-----|
| `01-fee-split-overflow` | `u64` fee multiplication overflows | `u128` mul/div via `math::mul_div_floor` |
| `02-unbounded-payout` | PnL paid in full, haircut ignored | `risk::effective_pnl` |
| `03-warmup-jump` | profit withdrawable at `elapsed = 0` | `risk::warmup_slope` |
| `04-funding-sign-flip` | shorts pay instead of receiving | `risk::funding_payment` antisymmetry |
| `05-writeoff-overrun` | writeoff subtracts more than insurance holds | `risk::loss_writeoff` cap |
| `06-split-residue-leak` | floored equal shares strand `total % 3` | residue routed to the sink share |
| `07-staking-multiplier-decrease` | pool-share multiplier shrinks as stake grows | monotone tiered multiplier |
| `08-flow-vault-settlement` | — (end-to-end flow) | haircut → payout → fee sweep → writeoff, settlement-wide bounds |
| `09-flow-agent-cpi` | — (end-to-end flow, `solana-agent`) | FSM + conservation + guarded CPI + oracle consensus |

## Design notes

- Generic verification gaps go upstream to Kani; only Solana-focused
  layers live here.
- Proof-critical spec logic stays in-repo: the risk primitives in
  `src/risk.rs` are the specification the wrapper is checked against,
  so there is no external spec-crate trust expansion.
- The `kani` crate is intentionally **not** a Cargo dependency: it
  ships with the Kani verifier and is provided by `cargo kani` when it
  builds this crate with `--cfg kani` (the pattern the Kani book
  documents). `[lints.rust]` declares `cfg(kani)` to `check-cfg` so
  normal builds stay warning-free.
- Helpers assert with plain `assert!`/`assert_eq!`: Kani checks them
  all, and the same helpers double as executable specifications.

## License

MIT
