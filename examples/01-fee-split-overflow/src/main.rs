//! Fail-to-fix #1: fee split overflow.
//!
//! A fee computed as `total * bps / 10_000` in `u64` overflows for
//! large totals (a balance over ~1.8e15 lamports at 100% fee, less at
//! realistic supplies once other math is chained). Kani finds the
//! overflowing input instantly; the fix routes through `u128` math.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/01-fee-split-overflow/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_fee_never_exceeds_total` FAILS with a
//! counterexample (arithmetic overflow); `fixed::proof_split_conserves`
//! VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/01-fee-split-overflow/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. `u64` multiplication before the division.
#[cfg(kani)]
mod vulnerable {
    pub fn fee_split(total: u64, bps: u64) -> (u64, u64) {
        let fee = total * bps / 10_000; // overflows for large totals
        (fee, total - fee)
    }

    #[kani::proof]
    fn proof_fee_never_exceeds_total() {
        // The fee rate is pinned to a constant (99.99%) so CBMC only
        // multiplies by a constant; the overflow is still reachable for
        // any total above ~u64::MAX / 9_999.
        let total: u64 = kani::any();
        let (fee, rest) = fee_split(total, 9_999);
        assert!(fee <= total);
        assert_eq!(fee as u128 + rest as u128, total as u128);
    }
}

/// FIXED: widen to `u128` before multiplying; the residue lands in the
/// second part, so the split conserves value by construction.
#[cfg(kani)]
mod fixed {
    use clawd_kani::math::mul_div_floor;

    pub fn fee_split(total: u64, bps: u64) -> (u64, u64) {
        let fee = mul_div_floor(total as u128, bps as u128, 10_000) as u64;
        (fee, total - fee)
    }

    // Bounded demonstration domain (totals up to 2^56, covering every
    // realistic lamport balance). The unbounded statement is what the
    // library helpers `token::assert_bps_split_conserves` and
    // `token::assert_bps_split_residue_to_sink` express; they are kept
    // out of this example so it verifies in seconds.
    #[kani::proof]
    fn proof_split_conserves() {
        let total = clawd_kani::generators::any_u64_range(0, 1u64 << 56);
        let bps = clawd_kani::generators::any_bps();
        let (fee, rest) = fee_split(total, bps);
        assert_eq!(fee as u128, (total as u128 * bps as u128) / 10_000);
        assert!(fee <= total);
        assert_eq!(fee + rest, total);
    }
}
