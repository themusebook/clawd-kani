//! Proof helpers for staking multipliers and reward accrual.
//!
//! A staking multiplier maps stake (or time-weighted stake) to a reward
//! weight. The helpers prove the properties a sound multiplier must
//! have: monotonicity (more stake never earns a smaller multiplier),
//! membership in an expected discrete set, and sane behaviour at zero.

/// Proves `multiplier_fn` is monotone non-decreasing in the stake:
/// for all symbolic `a <= b`, `multiplier_fn(a) <= multiplier_fn(b)`.
pub fn assert_multiplier_monotonic<F>(multiplier_fn: F)
where
    F: Fn(u64) -> u64,
{
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    kani::assume(a <= b);
    assert!(
        multiplier_fn(a) <= multiplier_fn(b),
        "multiplier decreases as stake grows"
    );
}

/// Proves `multiplier_fn(x)` always lands in `expected_values`.
pub fn assert_multiplier_in_set<F>(multiplier_fn: F, expected_values: &[u64])
where
    F: Fn(u64) -> u64,
{
    let x: u64 = kani::any();
    let m = multiplier_fn(x);
    assert!(
        expected_values.contains(&m),
        "multiplier outside the expected set"
    );
}

/// Proves a rewards function pays exactly zero when nothing is staked.
pub fn assert_zero_rewards_when_unstaked<F>(rewards_fn: F)
where
    F: Fn(u64) -> u128,
{
    assert_eq!(rewards_fn(0), 0, "rewards paid with zero stake");
}

/// Proves `rewards_fn(staked)` never overflows and stays within
/// `max_accumulated` for every `staked <= max_staked`, by evaluating the
/// real implementation symbolically (a panic is a proof failure).
pub fn assert_rewards_no_overflow<F>(rewards_fn: F, max_staked: u64, max_accumulated: u128)
where
    F: Fn(u64) -> u128,
{
    let staked = crate::generators::any_u64_range(0, max_staked);
    let rewards = rewards_fn(staked);
    assert!(rewards <= max_accumulated, "rewards exceed the accrual cap");
}

/// Proves the multiplier applied to a stake never loses principal:
/// for a multiplier expressed in basis points (`10_000 == 1.0x`),
/// `stake * mult_bps / 10_000 >= stake` whenever `mult_bps >= 10_000`,
/// over stakes bounded by `max_stake`.
pub fn assert_multiplier_never_below_principal<F>(multiplier_bps_fn: F, max_stake: u64)
where
    F: Fn(u64) -> u64,
{
    let stake = crate::generators::any_u64_range(0, max_stake);
    let mult_bps = multiplier_bps_fn(stake);
    kani::assume(mult_bps >= 10_000);
    let value = crate::math::mul_div_floor(stake as u128, mult_bps as u128, 10_000);
    assert!(value >= stake as u128, "multiplier values stake below principal");
}
