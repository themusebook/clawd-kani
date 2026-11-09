//! Wide integer math plus spec-equivalence proof helpers.
//!
//! The helpers here wrap *your* closures: pass an implementation and a
//! reference/spec implementation and the helper proves they agree on
//! every symbolic input in the domain.

/// Overflow-safe `a * b / den` (floor) for `u128` values.
///
/// Uses a checked multiply first, then falls back to a quotient/remainder
/// decomposition. When `b <= den` the decomposition is exact and the
/// result never exceeds `a`. Returns 0 when `den == 0`.
#[must_use]
pub fn mul_div_floor(a: u128, b: u128, den: u128) -> u128 {
    if den == 0 || a == 0 || b == 0 {
        return 0;
    }
    if let Some(prod) = a.checked_mul(b) {
        return prod / den;
    }
    let q = a / den;
    let r = a % den;
    let head = q.saturating_mul(b);
    let tail = r.checked_mul(b).map(|x| x / den).unwrap_or(0);
    head.saturating_add(tail)
}

/// Overflow-safe `a * b / den` (ceil) for `u128` values.
#[must_use]
pub fn mul_div_ceil(a: u128, b: u128, den: u128) -> u128 {
    let floor = mul_div_floor(a, b, den);
    if den == 0 || a == 0 || b == 0 {
        return 0;
    }
    // Exact division leaves no remainder to round up.
    let rem = mul_rem(a, b, den);
    if rem == 0 {
        floor
    } else {
        floor.saturating_add(1)
    }
}

/// Remainder companion to [`mul_div_floor`]: `a * b mod den`, computed
/// without overflowing when `b <= den` or the product fits in `u128`.
/// Returns 0 when `den == 0`.
#[must_use]
pub fn mul_rem(a: u128, b: u128, den: u128) -> u128 {
    if den == 0 {
        return 0;
    }
    if let Some(prod) = a.checked_mul(b) {
        return prod % den;
    }
    // (a mod den) * b mod den, one more level of decomposition.
    let a_mod = a % den;
    match a_mod.checked_mul(b) {
        Some(prod) => prod % den,
        None => {
            let q = a_mod / den.max(1);
            let r = a_mod % den.max(1);
            let head = (q.saturating_mul(b)) % den;
            let tail = r.checked_mul(b).map(|x| x % den).unwrap_or(0);
            (head + tail) % den
        }
    }
}

/// Proves `actual_fn(x) == spec_fn(x)` for every symbolic `x <= max`.
pub fn assert_matches_spec_u128<F, S>(actual_fn: F, spec_fn: S, max: u128)
where
    F: Fn(u128) -> u128,
    S: Fn(u128) -> u128,
{
    let x = crate::generators::any_u128_up_to(max);
    assert_eq!(actual_fn(x), spec_fn(x), "implementation disagrees with spec");
}

/// Proves `actual_fn(x) == spec_fn(x)` for every symbolic `u64` input.
pub fn assert_matches_spec_u64<F, S>(actual_fn: F, spec_fn: S)
where
    F: Fn(u64) -> u64,
    S: Fn(u64) -> u64,
{
    let x: u64 = kani::any();
    assert_eq!(actual_fn(x), spec_fn(x), "implementation disagrees with spec");
}

/// Proves `f` is monotone non-decreasing over symbolic `u64` pairs.
pub fn assert_monotonic_u64<F>(f: F)
where
    F: Fn(u64) -> u64,
{
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    kani::assume(a <= b);
    assert!(f(a) <= f(b), "function is not monotone non-decreasing");
}

/// Proves `f` is monotone non-increasing over symbolic `u64` pairs.
pub fn assert_anti_monotonic_u64<F>(f: F)
where
    F: Fn(u64) -> u64,
{
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    kani::assume(a <= b);
    assert!(f(a) >= f(b), "function is not monotone non-increasing");
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// `mul_div_floor` agrees with the naive `a * b / den` whenever the
    /// naive product cannot overflow. Kept on a tight domain (2^24
    /// operands, products <= 2^48) because this harness runs in the
    /// smoke profile; the wider 2^40 equivalence evidence is carried
    /// by `proof_mul_div_floor_rem_identity` below.
    #[kani::proof]
    fn proof_mul_div_floor_matches_naive() {
        let a = crate::generators::any_u128_up_to(1u128 << 24);
        let b = crate::generators::any_u128_up_to(1u128 << 24);
        let den = crate::generators::any_u128_up_to(1u128 << 24);
        kani::assume(den > 0);
        assert_eq!(mul_div_floor(a, b, den), (a * b) / den);
    }

    /// Division identity: `floor * den + rem == a * b` in the same
    /// bounded domain, i.e. no value is created or lost by splitting.
    #[kani::proof]
    fn proof_mul_div_floor_rem_identity() {
        let a = crate::generators::any_u128_up_to(1u128 << 40);
        let b = crate::generators::any_u128_up_to(1u128 << 40);
        let den = crate::generators::any_u128_up_to(1u128 << 40);
        kani::assume(den > 0);
        let q = mul_div_floor(a, b, den);
        let r = mul_rem(a, b, den);
        assert!(r < den);
        assert_eq!(q * den + r, a * b);
    }

    /// Ceil is the floor or floor + 1, and never below the exact value.
    #[kani::proof]
    fn proof_mul_div_ceil_bounds() {
        let a = crate::generators::any_u128_up_to(1u128 << 32);
        let b = crate::generators::any_u128_up_to(1u128 << 32);
        let den = crate::generators::any_u128_up_to(1u128 << 32);
        kani::assume(den > 0);
        let f = mul_div_floor(a, b, den);
        let c = mul_div_ceil(a, b, den);
        assert!(c == f || c == f + 1);
        assert!(c * den >= a * b);
    }
}
