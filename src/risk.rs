//! Percolator-style risk primitives and Kani proofs.
//!
//! Reference: <https://github.com/aeyakovenko/percolator>
//!
//! The six functions here are the *specification* implementations used
//! by the Clawd percolator wrapper: they favour obviously-correct
//! checked/saturating arithmetic over speed, and each ships with proofs
//! of conservation, bounds and monotonicity in the `proofs` module
//! below. All amounts are token atoms (`u128`); ratios are returned as
//! `(num, den)` pairs so no precision is lost to a fixed-point scale.

/// Global haircut ratio `h` as used in Percolator.
///
/// Returns `(h_num, h_den)` where `h = h_num / h_den`.
///
/// The vault must first cover senior obligations — total principal plus
/// the insurance fund's committed balance. Only the residual backs
/// positive PnL, so profitable accounts are haircut pro-rata when the
/// residual cannot cover them in full. When `pnl_pos_total == 0` there
/// are no profitable accounts, so `h = 1`.
#[must_use]
pub fn haircut_ratio(
    vault: u128,
    principal_total: u128,
    insurance: u128,
    pnl_pos_total: u128,
) -> (u128, u128) {
    let owed = principal_total.saturating_add(insurance);
    let residual = vault.saturating_sub(owed);
    if pnl_pos_total == 0 {
        (1, 1)
    } else {
        (residual.min(pnl_pos_total), pnl_pos_total)
    }
}

/// Effective positive PnL for one account after the haircut.
///
/// Negative PnL is untouched (returned as 0); positive PnL is scaled
/// by `h = h_num / h_den` (floored).
///
/// Contract:
/// - Callers should pass `h_den > 0` and `h_num <= h_den` (as produced
///   by [`haircut_ratio`]).
/// - If `h_den == 0`, this returns 0 to avoid division-by-zero.
/// - If `h_num > h_den`, it is clamped to `h_den`.
#[must_use]
pub fn effective_pnl(pnl_i: i128, h_num: u128, h_den: u128) -> u128 {
    let pos = pnl_i.max(0) as u128;
    if pos == 0 || h_den == 0 {
        return 0;
    }
    let h_num = h_num.min(h_den);
    if h_num == h_den {
        return pos;
    }
    crate::math::mul_div_floor(pos, h_num, h_den)
}

/// Linear warmup slope: the fraction of `gross_profit` that has become
/// withdrawable after `elapsed` of `warmup_period` time units.
///
/// `warmed = gross_profit * min(elapsed, warmup_period) / warmup_period`
/// (floored). A zero warmup period means profits are immediately
/// withdrawable.
#[must_use]
pub fn warmup_slope(gross_profit: u128, elapsed: u64, warmup_period: u64) -> u128 {
    if warmup_period == 0 || elapsed >= warmup_period {
        gross_profit
    } else {
        crate::math::mul_div_floor(gross_profit, elapsed as u128, warmup_period as u128)
    }
}

/// Fee-debt sweep: given an account's accumulated fee debt and its
/// available balance, compute how much is swept (paid) and the debt
/// that remains.
///
/// Returns `(swept, remaining_debt)`. Conservation:
/// `swept + remaining_debt == fee_debt` and `swept <= available`.
#[must_use]
pub fn fee_debt_sweep(fee_debt: u128, available: u128) -> (u128, u128) {
    let swept = fee_debt.min(available);
    (swept, fee_debt - swept)
}

/// Funding payment for one account.
///
/// `payment = position * rate_num / rate_den`, signed by the rate and
/// flipped for shorts: with a positive rate, longs pay and shorts
/// receive (positive return = the account pays). Returns 0 when the
/// position, the rate, or the denominator is zero, and saturates the
/// magnitude at `i128::MAX` on overflow rather than wrapping.
#[must_use]
pub fn funding_payment(position: u128, rate_num: i128, rate_den: u128, is_long: bool) -> i128 {
    if position == 0 || rate_num == 0 || rate_den == 0 {
        return 0;
    }

    let abs_num: u128 = rate_num.unsigned_abs();
    let magnitude: i128 = match position.checked_mul(abs_num) {
        Some(prod) => {
            let mag_u128 = prod / rate_den;
            if mag_u128 > i128::MAX as u128 {
                i128::MAX
            } else {
                mag_u128 as i128
            }
        }
        None => i128::MAX,
    };

    let raw = if rate_num < 0 { -magnitude } else { magnitude };
    if is_long {
        raw
    } else {
        -raw
    }
}

/// Loss writeoff: when an account's equity goes negative, the insurance
/// fund absorbs the loss up to its balance.
///
/// Returns `(writeoff, new_insurance)`. The writeoff never exceeds the
/// loss or the insurance balance, and
/// `writeoff + new_insurance == insurance` (conservation).
#[must_use]
pub fn loss_writeoff(negative_equity: u128, insurance: u128) -> (u128, u128) {
    let writeoff = negative_equity.min(insurance);
    (writeoff, insurance - writeoff)
}

#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::generators::*;

    /// Moderate bound for amount domains: keeps products of two amounts
    /// comfortably inside `u128` while covering any realistic supply.
    const AMT: u128 = 1u128 << 60;

    fn any_rate_i128() -> i128 {
        let r: i128 = kani::any();
        kani::assume(r >= -(1i128 << 32) && r <= (1i128 << 32));
        r
    }

    // ---- haircut_ratio ----

    #[kani::proof]
    fn proof_haircut_ratio_bounds() {
        let vault = any_u128_up_to(AMT);
        let principal = any_u128_up_to(AMT);
        let insurance = any_u128_up_to(AMT);
        let pnl_pos = any_u128_up_to(AMT);
        let (num, den) = haircut_ratio(vault, principal, insurance, pnl_pos);
        assert!(den > 0, "denominator must be positive");
        assert!(num <= den, "haircut must be in [0, 1]");
        if pnl_pos > 0 {
            assert_eq!(den, pnl_pos);
            assert!(num <= pnl_pos);
        }
    }

    #[kani::proof]
    fn proof_haircut_ratio_one_when_fully_backed() {
        let principal = any_u128_up_to(AMT);
        let insurance = any_u128_up_to(AMT);
        let pnl_pos = any_u128_up_to(AMT);
        kani::assume(pnl_pos > 0);
        // Vault covers principal + insurance + all positive PnL.
        let vault = any_u128_up_to(AMT);
        kani::assume(vault >= principal + insurance + pnl_pos);
        let (num, den) = haircut_ratio(vault, principal, insurance, pnl_pos);
        assert_eq!(num, den, "fully backed PnL must not be haircut");
        assert_eq!(num, pnl_pos);
    }

    #[kani::proof]
    fn proof_haircut_ratio_zero_when_vault_empty() {
        let principal = any_u128_up_to(AMT);
        let insurance = any_u128_up_to(AMT);
        let pnl_pos = any_u128_up_to(AMT);
        kani::assume(pnl_pos > 0);
        let (num, _den) = haircut_ratio(0, principal, insurance, pnl_pos);
        assert_eq!(num, 0, "an empty vault backs no positive PnL");
    }

    // ---- effective_pnl ----

    #[kani::proof]
    fn proof_effective_pnl_bounded_by_profit() {
        let pnl: i128 = kani::any();
        kani::assume(pnl >= -(1i128 << 62) && pnl <= (1i128 << 62));
        let (num, den) = any_ratio();
        let payout = effective_pnl(pnl, num, den);
        assert!(payout <= (pnl.max(0) as u128), "payout exceeds positive PnL");
        if pnl <= 0 {
            assert_eq!(payout, 0, "non-positive PnL pays out nothing");
        }
    }

    #[kani::proof]
    fn proof_effective_pnl_bounded_by_haircut() {
        // Cross-multiplied form of `payout <= pnl * h`, in a domain
        // where both products fit in u128.
        let pos = any_u128_up_to(1u128 << 60);
        let den = any_u128_up_to(1u128 << 60);
        kani::assume(den > 0);
        let num = any_u128_up_to(den);
        let payout = effective_pnl(pos as i128, num, den);
        assert!(payout * den <= pos * num, "payout exceeds the haircut share");
    }

    #[kani::proof]
    fn proof_effective_pnl_monotone_in_pnl() {
        let (num, den) = any_ratio();
        let p1 = any_u128_up_to(1u128 << 40);
        let p2 = any_u128_up_to(1u128 << 40);
        kani::assume(p1 <= p2);
        let out1 = effective_pnl(p1 as i128, num, den);
        let out2 = effective_pnl(p2 as i128, num, den);
        assert!(out1 <= out2, "a larger profit must not pay out less");
    }

    #[kani::proof]
    fn proof_effective_pnl_matches_reference() {
        // Spec equivalence against the naive formula in a domain where
        // the naive product cannot overflow.
        let pos = any_u128_up_to(1u128 << 32);
        let den = any_u128_up_to(1u128 << 32);
        kani::assume(den > 0);
        let num = any_u128_up_to(den);
        let expected = (pos * num) / den;
        assert_eq!(effective_pnl(pos as i128, num, den), expected);
    }

    // ---- fee_debt_sweep ----

    #[kani::proof]
    fn proof_fee_sweep_conservation() {
        let fee_debt = any_u128_up_to(AMT);
        let available = any_u128_up_to(AMT);
        let (swept, remaining) = fee_debt_sweep(fee_debt, available);
        assert_eq!(swept + remaining, fee_debt, "sweep must conserve the debt");
        assert!(swept <= available, "cannot sweep more than is available");
        assert!(swept <= fee_debt, "cannot sweep more than the debt");
    }

    #[kani::proof]
    fn proof_fee_sweep_clears_when_sufficient() {
        let fee_debt = any_u128_up_to(AMT);
        let available = any_u128_up_to(AMT);
        kani::assume(available >= fee_debt);
        let (swept, remaining) = fee_debt_sweep(fee_debt, available);
        assert_eq!(swept, fee_debt);
        assert_eq!(remaining, 0, "debt must clear when funds suffice");
    }

    // ---- funding_payment ----

    #[kani::proof]
    fn proof_funding_long_short_symmetry() {
        let position = any_u128_up_to(1u128 << 64);
        let rate_num = any_rate_i128();
        let rate_den = any_u128_up_to(1u128 << 32);
        kani::assume(rate_den > 0);
        let long_pays = funding_payment(position, rate_num, rate_den, true);
        let short_pays = funding_payment(position, rate_num, rate_den, false);
        assert_eq!(long_pays, -short_pays, "long/short payments must be antisymmetric");
    }

    #[kani::proof]
    fn proof_funding_sign_follows_rate() {
        let position = any_u128_up_to(1u128 << 64);
        kani::assume(position > 0);
        let rate_num = any_rate_i128();
        kani::assume(rate_num > 0);
        let rate_den = any_u128_up_to(1u128 << 32);
        kani::assume(rate_den > 0);
        assert!(funding_payment(position, rate_num, rate_den, true) >= 0);
        assert!(funding_payment(position, rate_num, rate_den, false) <= 0);
    }

    #[kani::proof]
    fn proof_funding_zero_cases() {
        let position = any_u128_up_to(AMT);
        let rate_num = any_rate_i128();
        let rate_den = any_u128_up_to(AMT);
        assert_eq!(funding_payment(0, rate_num, rate_den, true), 0);
        assert_eq!(funding_payment(position, 0, rate_den, true), 0);
        assert_eq!(funding_payment(position, rate_num, 0, true), 0);
    }

    #[kani::proof]
    fn proof_funding_bounded_by_position() {
        // A funding rate of at most 100% (|rate_num| <= rate_den) can
        // never charge an account more than its position size.
        let position = any_u128_up_to(1u128 << 64);
        let rate_den = any_u128_up_to(1u128 << 32);
        kani::assume(rate_den > 0);
        let rate_num = any_rate_i128();
        kani::assume(rate_num.unsigned_abs() <= rate_den);
        let payment = funding_payment(position, rate_num, rate_den, true);
        assert!(payment.unsigned_abs() <= position);
    }

    // ---- loss_writeoff ----

    #[kani::proof]
    fn proof_writeoff_conservation() {
        let negative_equity = any_u128_up_to(AMT);
        let insurance = any_u128_up_to(AMT);
        let (writeoff, new_insurance) = loss_writeoff(negative_equity, insurance);
        assert!(writeoff <= negative_equity, "writeoff exceeds the loss");
        assert!(writeoff <= insurance, "writeoff exceeds the insurance fund");
        assert_eq!(
            writeoff + new_insurance,
            insurance,
            "writeoff must conserve the insurance fund"
        );
        assert_eq!(
            writeoff + (negative_equity - writeoff),
            negative_equity,
            "residual loss must account for the rest"
        );
    }

    #[kani::proof]
    fn proof_writeoff_monotone_in_insurance() {
        let negative_equity = any_u128_up_to(AMT);
        let ins1 = any_u128_up_to(AMT);
        let ins2 = any_u128_up_to(AMT);
        kani::assume(ins1 <= ins2);
        let (w1, _) = loss_writeoff(negative_equity, ins1);
        let (w2, _) = loss_writeoff(negative_equity, ins2);
        assert!(w1 <= w2, "more insurance must not write off less");
    }

    // ---- warmup_slope ----

    #[kani::proof]
    fn proof_warmup_monotonic_in_elapsed() {
        let gross = any_u128_up_to(1u128 << 64);
        let period = any_u64_range(0, 1u64 << 32);
        let e1 = any_u64_range(0, 1u64 << 33);
        let e2 = any_u64_range(0, 1u64 << 33);
        kani::assume(e1 <= e2);
        let s1 = warmup_slope(gross, e1, period);
        let s2 = warmup_slope(gross, e2, period);
        assert!(s1 <= s2, "warmed amount must grow with elapsed time");
    }

    #[kani::proof]
    fn proof_warmup_bounded_by_gross() {
        let gross = any_u128_up_to(1u128 << 64);
        let period = any_u64_range(0, 1u64 << 32);
        let elapsed = any_u64_range(0, 1u64 << 33);
        assert!(warmup_slope(gross, elapsed, period) <= gross);
    }

    #[kani::proof]
    fn proof_warmup_full_after_period() {
        let gross = any_u128_up_to(1u128 << 64);
        let period = any_u64_range(0, 1u64 << 32);
        let elapsed = any_u64_range(0, 1u64 << 33);
        kani::assume(elapsed >= period);
        assert_eq!(warmup_slope(gross, elapsed, period), gross);
    }

    #[kani::proof]
    fn proof_warmup_zero_at_start() {
        let gross = any_u128_up_to(1u128 << 64);
        let period = any_u64_range(1, 1u64 << 32);
        assert_eq!(warmup_slope(gross, 0, period), 0);
    }

    // ---- wide-domain variants (full profile only) ----

    #[cfg(feature = "kani-full")]
    #[kani::proof]
    fn proof_haircut_ratio_bounds_wide() {
        let vault: u128 = kani::any();
        let principal: u128 = kani::any();
        let insurance: u128 = kani::any();
        let pnl_pos: u128 = kani::any();
        let (num, den) = haircut_ratio(vault, principal, insurance, pnl_pos);
        assert!(den > 0);
        assert!(num <= den);
    }

    #[cfg(feature = "kani-full")]
    #[kani::proof]
    fn proof_writeoff_conservation_wide() {
        let negative_equity: u128 = kani::any();
        let insurance: u128 = kani::any();
        let (writeoff, new_insurance) = loss_writeoff(negative_equity, insurance);
        assert!(writeoff <= negative_equity);
        assert!(writeoff <= insurance);
        assert_eq!(writeoff + new_insurance, insurance);
    }

    #[cfg(feature = "kani-full")]
    #[kani::proof]
    fn proof_fee_sweep_conservation_wide() {
        let fee_debt: u128 = kani::any();
        let available: u128 = kani::any();
        let (swept, remaining) = fee_debt_sweep(fee_debt, available);
        assert_eq!(swept + remaining, fee_debt);
        assert!(swept <= available);
    }
}
