//! Fail-to-fix #5: loss writeoff overrun.
//!
//! When a liquidated account's loss exceeds the insurance fund, the
//! fund can only absorb what it holds. Subtracting the full loss
//! anyway underflows the fund balance (a panic in checked builds, a
//! wrapped astronomically-large balance in release builds).
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/05-writeoff-overrun/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_writeoff_never_exceeds_insurance`
//! FAILS (underflow counterexample); the `fixed` harness VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/05-writeoff-overrun/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. Subtracts the whole loss from insurance.
#[cfg(kani)]
mod vulnerable {
    pub fn loss_writeoff(negative_equity: u128, insurance: u128) -> (u128, u128) {
        (negative_equity, insurance - negative_equity) // underflows
    }

    #[kani::proof]
    fn proof_writeoff_never_exceeds_insurance() {
        let loss = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let insurance = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let (writeoff, new_insurance) = loss_writeoff(loss, insurance);
        assert!(writeoff <= insurance);
        assert_eq!(writeoff + new_insurance, insurance);
    }
}

/// FIXED: the writeoff is capped at the insurance balance; the rest
/// of the loss remains as the account's (uncovered) residual.
#[cfg(kani)]
mod fixed {
    #[kani::proof]
    fn proof_writeoff_never_exceeds_insurance() {
        let loss = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let insurance = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let (writeoff, new_insurance) = clawd_kani::risk::loss_writeoff(loss, insurance);
        assert!(writeoff <= insurance);
        assert!(writeoff <= loss);
        assert_eq!(writeoff + new_insurance, insurance);
    }
}
