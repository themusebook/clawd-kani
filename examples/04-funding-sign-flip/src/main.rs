//! Fail-to-fix #4: funding sign flip.
//!
//! Funding is a zero-sum transfer between longs and shorts: with a
//! positive rate, longs pay exactly what shorts receive. Forgetting to
//! flip the sign for shorts makes *both* sides pay — value leaks to
//! (or from) the venue on every funding tick.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/04-funding-sign-flip/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_long_short_antisymmetry` FAILS;
//! `fixed::proof_long_short_antisymmetry` VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/04-funding-sign-flip/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. `is_long` is ignored: shorts pay too.
#[cfg(kani)]
mod vulnerable {
    pub fn funding_payment(position: u128, rate_num: i128, rate_den: u128, _is_long: bool) -> i128 {
        if position == 0 || rate_num == 0 || rate_den == 0 {
            return 0;
        }
        let magnitude = (position * rate_num.unsigned_abs() / rate_den) as i128;
        if rate_num < 0 {
            -magnitude
        } else {
            magnitude
        }
    }

    #[kani::proof]
    fn proof_long_short_antisymmetry() {
        let position = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let rate_num: i128 = 7;
        let rate_den: u128 = 100;
        let long_pays = funding_payment(position, rate_num, rate_den, true);
        let short_pays = funding_payment(position, rate_num, rate_den, false);
        assert_eq!(long_pays, -short_pays);
    }
}

/// FIXED: `funding_payment` flips the sign for shorts, so the two
/// sides are exactly antisymmetric.
#[cfg(kani)]
mod fixed {
    #[kani::proof]
    fn proof_long_short_antisymmetry() {
        let position = clawd_kani::generators::any_u128_up_to(1u128 << 40);
        let rate_num: i128 = 7;
        let rate_den: u128 = 100;
        let long_pays = clawd_kani::risk::funding_payment(position, rate_num, rate_den, true);
        let short_pays = clawd_kani::risk::funding_payment(position, rate_num, rate_den, false);
        assert_eq!(long_pays, -short_pays);
        assert!(long_pays >= 0 && short_pays <= 0);
    }
}
