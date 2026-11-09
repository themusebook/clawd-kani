//! Fail-to-fix #2: unbounded payout (haircut ignored).
//!
//! Paying every profitable account its full PnL when the vault is
//! under-backed promises more than the vault holds: the first accounts
//! out drain principal that belongs to everyone. Percolator's answer
//! is the haircut: positive PnL is junior and scaled by `h <= 1`.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/02-unbounded-payout/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_payout_never_exceeds_backing` FAILS;
//! `fixed::proof_payout_never_exceeds_backing` VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/02-unbounded-payout/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. Ignores the haircut entirely.
#[cfg(kani)]
mod vulnerable {
    pub fn payout(pnl: u128, _h_num: u128, _h_den: u128) -> u128 {
        pnl
    }

    #[kani::proof]
    fn proof_payout_never_exceeds_backing() {
        // Under-backed: residual covers only half the positive PnL.
        let pnl = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        kani::assume(pnl > 0);
        let paid = payout(pnl, 1, 2);
        assert!(paid * 2 <= pnl, "paid more than the haircut share");
    }
}

/// FIXED: payouts go through `effective_pnl`, which scales positive
/// PnL by the haircut ratio.
#[cfg(kani)]
mod fixed {
    #[kani::proof]
    fn proof_payout_never_exceeds_backing() {
        let pnl = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        kani::assume(pnl > 0);
        let paid = clawd_kani::risk::effective_pnl(pnl as i128, 1, 2);
        assert!(paid * 2 <= pnl, "paid more than the haircut share");
        assert!(paid <= pnl);
    }
}
