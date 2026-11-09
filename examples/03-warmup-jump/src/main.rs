//! Fail-to-fix #3: warmup jump.
//!
//! PnL warmup exists so a just-opened position cannot instantly
//! withdraw profits it may not keep (mark prices move; the profit may
//! evaporate before settlement). Skipping the ramp lets an account
//! withdraw 100% of its profit at `elapsed == 0`.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/03-warmup-jump/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_nothing_withdrawable_at_start` FAILS;
//! `fixed::proof_nothing_withdrawable_at_start` VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/03-warmup-jump/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. Profit is fully withdrawable immediately.
#[cfg(kani)]
mod vulnerable {
    pub fn warmed(gross_profit: u128, _elapsed: u64, _warmup_period: u64) -> u128 {
        gross_profit
    }

    #[kani::proof]
    fn proof_nothing_withdrawable_at_start() {
        let gross = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let period = clawd_kani::generators::any_u64_range(1, 1u64 << 32);
        assert_eq!(warmed(gross, 0, period), 0);
    }
}

/// FIXED: the linear warmup slope — nothing at `elapsed == 0`, the
/// full amount once the period has passed, monotone in between.
#[cfg(kani)]
mod fixed {
    #[kani::proof]
    fn proof_nothing_withdrawable_at_start() {
        let gross = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let period = clawd_kani::generators::any_u64_range(1, 1u64 << 32);
        assert_eq!(clawd_kani::risk::warmup_slope(gross, 0, period), 0);
    }

    #[kani::proof]
    fn proof_full_after_period() {
        let gross = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let period = clawd_kani::generators::any_u64_range(0, 1u64 << 32);
        let elapsed = clawd_kani::generators::any_u64_range(0, 1u64 << 33);
        kani::assume(elapsed >= period);
        assert_eq!(clawd_kani::risk::warmup_slope(gross, elapsed, period), gross);
    }
}
