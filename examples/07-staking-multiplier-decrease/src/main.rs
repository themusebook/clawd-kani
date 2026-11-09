//! Fail-to-fix #7: staking multiplier that decreases with stake.
//!
//! A multiplier derived from a *share of the current pool*
//! (`SCALE / (SCALE + stake)`) shrinks as stake grows: the biggest
//! stakers get the worst rate, and a whale can dilute everyone else's
//! multiplier by staking. Multipliers must be monotone non-decreasing
//! in stake.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/07-staking-multiplier-decrease/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_multiplier_monotone` FAILS;
//! `fixed::proof_multiplier_monotone` VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/07-staking-multiplier-decrease/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. Pool-share style multiplier (bps,
/// 10_000 = 1.0x) that *decreases* as stake grows.
#[cfg(kani)]
mod vulnerable {
    pub fn multiplier_bps(stake: u64) -> u64 {
        const SCALE: u128 = 1_000_000;
        ((10_000u128 * SCALE) / (SCALE + stake as u128)) as u64
    }

    #[kani::proof]
    fn proof_multiplier_monotone() {
        clawd_kani::staking::assert_multiplier_monotonic(multiplier_bps);
    }
}

/// FIXED: a tiered bonus that grows with stake and is capped —
/// monotone non-decreasing by construction.
#[cfg(kani)]
mod fixed {
    pub fn multiplier_bps(stake: u64) -> u64 {
        // +1 bps per 1_000 staked, capped at +5_000 bps (1.5x).
        10_000 + (stake / 1_000).min(5_000)
    }

    #[kani::proof]
    fn proof_multiplier_monotone() {
        clawd_kani::staking::assert_multiplier_monotonic(multiplier_bps);
    }

    #[kani::proof]
    fn proof_multiplier_in_expected_range() {
        clawd_kani::bounds::assert_output_bounded(multiplier_bps, 10_000, 15_000);
    }
}
