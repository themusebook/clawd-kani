//! Proof helpers for bounded outputs.
//!
//! Wrap a computation and prove its result always stays inside an
//! expected range or set, for every symbolic input in the domain.

/// Proves `min <= compute_fn(x) <= max` for every symbolic `u64` input.
pub fn assert_output_bounded<F>(compute_fn: F, min: u64, max: u64)
where
    F: Fn(u64) -> u64,
{
    let x: u64 = kani::any();
    let out = compute_fn(x);
    assert!(out >= min && out <= max, "output outside [{min}, {max}]");
}

/// Proves `min <= compute_fn(x) <= max` for every symbolic `u16` input.
pub fn assert_output_bounded_u16<F>(compute_fn: F, min: u16, max: u16)
where
    F: Fn(u16) -> u16,
{
    let x: u16 = kani::any();
    let out = compute_fn(x);
    assert!(out >= min && out <= max, "output outside [{min}, {max}]");
}

/// Proves `min <= compute_fn(x) <= max` for every symbolic `u8` input.
pub fn assert_output_bounded_u8<F>(compute_fn: F, min: u8, max: u8)
where
    F: Fn(u8) -> u8,
{
    let x: u8 = kani::any();
    let out = compute_fn(x);
    assert!(out >= min && out <= max, "output outside [{min}, {max}]");
}

/// Proves `compute_fn(x)` always lands in `expected`.
pub fn assert_output_in_set<F>(compute_fn: F, expected: &[u8])
where
    F: Fn(u8) -> u8,
{
    let x: u8 = kani::any();
    let out = compute_fn(x);
    assert!(expected.contains(&out), "output outside the expected set");
}

/// Proves a cost/fee function stays within `[floor, ceiling]` over
/// symbolic inputs bounded by `max_input`.
pub fn assert_cost_bounded<F>(compute_fn: F, max_input: u64, floor: u64, ceiling: u64)
where
    F: Fn(u64) -> u64,
{
    let x = crate::generators::any_u64_range(0, max_input);
    let out = compute_fn(x);
    assert!(
        out >= floor && out <= ceiling,
        "cost outside [{floor}, {ceiling}]"
    );
}

/// Proves `compute_fn(x) == expected_default` whenever `condition_fn(x)`
/// holds, over symbolic inputs.
pub fn assert_default_on_condition<C, F>(condition_fn: C, compute_fn: F, expected_default: u64)
where
    C: Fn(u64) -> bool,
    F: Fn(u64) -> u64,
{
    let x: u64 = kani::any();
    kani::assume(condition_fn(x));
    assert_eq!(
        compute_fn(x),
        expected_default,
        "output is not the default when the condition holds"
    );
}

/// Proves `compute_fn(x) <= bound` for symbolic `u128` inputs up to
/// `max_input` — the wide-math analogue of [`assert_output_bounded`].
pub fn assert_output_bounded_u128<F>(compute_fn: F, max_input: u128, bound: u128)
where
    F: Fn(u128) -> u128,
{
    let x = crate::generators::any_u128_up_to(max_input);
    assert!(compute_fn(x) <= bound, "output exceeds bound");
}
