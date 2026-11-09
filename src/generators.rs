//! Constrained symbolic generators for common Solana domains.
//!
//! Each generator produces a fully symbolic value and then narrows it
//! with `kani::assume`, so harnesses state their input domain in one
//! place and CBMC only explores the reachable, meaningful part of it.

/// Arbitrary `u8` in `[0, max]` inclusive.
pub fn any_u8_up_to(max: u8) -> u8 {
    let v: u8 = kani::any();
    kani::assume(v <= max);
    v
}

/// Arbitrary `u16` in `[min, max]` inclusive.
pub fn any_u16_range(min: u16, max: u16) -> u16 {
    let v: u16 = kani::any();
    kani::assume(v >= min && v <= max);
    v
}

/// Arbitrary `u64` in `[min, max]` inclusive.
pub fn any_u64_range(min: u64, max: u64) -> u64 {
    let v: u64 = kani::any();
    kani::assume(v >= min && v <= max);
    v
}

/// Arbitrary `i64` in `[min, max]` inclusive.
pub fn any_i64_range(min: i64, max: i64) -> i64 {
    let v: i64 = kani::any();
    kani::assume(v >= min && v <= max);
    v
}

/// Arbitrary `u128` in `[0, max]` inclusive.
pub fn any_u128_up_to(max: u128) -> u128 {
    let v: u128 = kani::any();
    kani::assume(v <= max);
    v
}

/// Quality/percentage score in `[0, 100]`.
pub fn any_score() -> u8 {
    any_u8_up_to(100)
}

/// Pool/governance weight in `[0, 1_000]`.
pub fn any_weight() -> u16 {
    any_u16_range(0, 1_000)
}

/// Basis points in `[0, 10_000]` (0%..=100%).
pub fn any_bps() -> u64 {
    any_u64_range(0, 10_000)
}

/// Lamports bounded by a plausible on-chain balance (<= 2^63) so that
/// sums of a few balances cannot overflow `u64`/`u128` accumulators.
pub fn any_lamports_bounded() -> u64 {
    let v: u64 = kani::any();
    kani::assume(v <= (1u64 << 63));
    v
}

/// Unix timestamp (seconds) in a plausible post-genesis range.
pub fn any_timestamp() -> i64 {
    any_i64_range(0, i64::MAX / 2)
}

/// A pair of timestamps `(t0, t1)` with `t0 <= t1`, e.g. a deposit time
/// and a later withdrawal time.
pub fn any_ordered_timestamps() -> (i64, i64) {
    let t0 = any_timestamp();
    let t1 = any_timestamp();
    kani::assume(t1 >= t0);
    (t0, t1)
}

/// A ratio `(num, den)` with `0 <= num <= den` and `den > 0`, the shape
/// produced by [`crate::risk::haircut_ratio`].
pub fn any_ratio() -> (u128, u128) {
    let den = any_u128_up_to(1u128 << 64);
    kani::assume(den > 0);
    let num = any_u128_up_to(den);
    (num, den)
}
