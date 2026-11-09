//! Reusable Kani formal-verification primitives for Solana protocol math.
//!
//! This crate is intentionally a no-op in normal builds: every module is
//! gated behind `cfg(kani)`, so `cargo check` / `cargo test` compile the
//! crate to nothing and there is zero runtime cost. The code only exists
//! when the Kani verifier (`cargo kani`) builds the crate with
//! `--cfg kani`, at which point the `kani` crate — which ships with the
//! verifier and is deliberately not a Cargo dependency — becomes
//! available.
//!
//! Layout:
//! - [`risk`]: Percolator-style risk primitives (haircut ratio, effective
//!   PnL, warmup slope, fee-debt sweep, funding payment, loss writeoff),
//!   each with Kani proofs of conservation, bounds and monotonicity.
//! - [`token`], [`staking`], [`bounds`], [`math`]: reusable assert-style
//!   proof helpers that wrap your own closures.
//! - [`generators`]: constrained symbolic value generators for common
//!   Solana domains (basis points, lamports, scores, weights, timestamps).
//! - [`agent`] (feature `solana-agent`): symbolic agent accounts, lamport
//!   conservation, reentrancy checks, CPI allowlists, PDA seed limits,
//!   oracle consensus and FSM transition guards.
//! - [`account_info`] (feature `solana-account-info`): symbolic
//!   `solana_program::AccountInfo` construction and lamport snapshots.
//!
//! Install Kani once with:
//! `cargo install --locked kani-verifier && cargo kani setup`

#[cfg(kani)]
pub mod generators;

#[cfg(kani)]
pub mod token;

#[cfg(kani)]
pub mod staking;

#[cfg(kani)]
pub mod bounds;

#[cfg(kani)]
pub mod math;

#[cfg(kani)]
pub mod risk;

#[cfg(all(kani, feature = "solana-agent"))]
pub mod agent;

#[cfg(all(kani, feature = "solana-account-info"))]
pub mod account_info;
