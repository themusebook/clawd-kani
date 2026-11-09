//! Proof helpers for value-conserving token splits.
//!
//! Each helper takes the split function under test as a closure and
//! proves the conservation property for every symbolic total: parts
//! never exceed the total and always sum back to it exactly (any
//! rounding residue must be assigned to one of the parts, e.g. a sink).

/// Proves that `split_fn(total)` conserves value for all symbolic totals.
pub fn assert_two_way_split_conserves<F>(split_fn: F)
where
    F: Fn(u64) -> (u64, u64),
{
    let total: u64 = kani::any();
    let (a, b) = split_fn(total);
    assert!(a <= total, "part_a exceeds total");
    assert!(b <= total, "part_b exceeds total");
    let sum = (a as u128) + (b as u128);
    assert_eq!(sum, total as u128, "split does not conserve value");
}

/// Proves that `split_fn(total)` conserves value for all symbolic totals.
pub fn assert_three_way_split_conserves<F>(split_fn: F)
where
    F: Fn(u64) -> (u64, u64, u64),
{
    let total: u64 = kani::any();
    let (a, b, c) = split_fn(total);
    assert!(a <= total, "part_a exceeds total");
    assert!(b <= total, "part_b exceeds total");
    assert!(c <= total, "part_c exceeds total");
    let sum = (a as u128) + (b as u128) + (c as u128);
    assert_eq!(sum, total as u128, "split does not conserve value");
}

/// Proves that `split_fn(total, rate_bps)` conserves value for
/// `rate_bps <= 10_000`.
pub fn assert_bps_split_conserves<F>(split_fn: F)
where
    F: Fn(u64, u64) -> (u64, u64),
{
    let total: u64 = kani::any();
    let rate_bps = crate::generators::any_bps();
    let (a, b) = split_fn(total, rate_bps);
    assert!(a <= total, "part_a exceeds total");
    assert!(b <= total, "part_b exceeds total");
    let sum = (a as u128) + (b as u128);
    assert_eq!(sum, total as u128, "bps split does not conserve value");
}

/// Proves that `split_fn(total)` conserves value over `u128` totals
/// bounded by `max_total` (keeps the symbolic domain tractable).
pub fn assert_two_way_split_conserves_u128<F>(split_fn: F, max_total: u128)
where
    F: Fn(u128) -> (u128, u128),
{
    let total = crate::generators::any_u128_up_to(max_total);
    let (a, b) = split_fn(total);
    assert!(a <= total, "part_a exceeds total");
    assert!(b <= total, "part_b exceeds total");
    assert_eq!(a + b, total, "split does not conserve value");
}

/// Proves a two-way split sends its rounding residue to the sink part:
/// `sink >= total - first` is false in general; instead we check the
/// exact identity `first + sink == total` *and* that `first` equals the
/// floored share `total * bps / 10_000`, so the sink provably absorbs
/// the residue rather than new value being minted.
pub fn assert_bps_split_residue_to_sink<F>(split_fn: F)
where
    F: Fn(u64, u64) -> (u64, u64),
{
    let total: u64 = kani::any();
    let rate_bps = crate::generators::any_bps();
    let (first, sink) = split_fn(total, rate_bps);
    let expected_first = ((total as u128) * (rate_bps as u128) / 10_000) as u64;
    assert_eq!(first, expected_first, "first part is not the floored bps share");
    assert_eq!(
        (first as u128) + (sink as u128),
        total as u128,
        "residue did not land in the sink"
    );
}
