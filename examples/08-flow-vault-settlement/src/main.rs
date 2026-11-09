//! End-to-end flow #1: vault settlement.
//!
//! A full settlement round over the clawd-kani risk primitives:
//! the haircut is computed from the vault state, the profitable
//! account is paid through it, its accumulated fee debt is swept from
//! the payout, and the losing account's deficit is written off against
//! insurance. The harness proves the settlement-wide invariants:
//! principal seniority, payout bounded by the residual backing, and
//! conservation in both the sweep and the writeoff.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/08-flow-vault-settlement/Cargo.toml
//! ```
//! Expected: the harness VERIFIES.

fn main() {
    println!("This is a Kani end-to-end flow example. Run it with:");
    println!("  cargo kani --manifest-path examples/08-flow-vault-settlement/Cargo.toml");
}

#[cfg(kani)]
mod flow {
    use clawd_kani::generators::*;
    use clawd_kani::risk::*;

    const AMT: u128 = 1u128 << 48;

    #[kani::proof]
    fn proof_vault_settlement_end_to_end() {
        // Vault state and the two accounts to settle.
        let vault = any_u128_up_to(AMT);
        let principal = any_u128_up_to(AMT);
        let insurance = any_u128_up_to(AMT);
        let pnl_a = any_u128_up_to(AMT); // profitable account
        let loss_b = any_u128_up_to(AMT); // losing account's deficit
        let fee_debt_a = any_u128_up_to(AMT);

        // 1. Haircut from the vault state.
        let (h_num, h_den) = haircut_ratio(vault, principal, insurance, pnl_a);
        assert!(h_num <= h_den && h_den > 0);

        // 2. Pay the profitable account through the haircut.
        let payout_a = effective_pnl(pnl_a as i128, h_num, h_den);
        assert!(payout_a <= pnl_a, "payout exceeds the account's profit");
        if pnl_a > 0 {
            // h_den == pnl_a here, so the payout is also bounded by
            // h_num <= residual: principal + insurance stay covered.
            assert!(payout_a <= h_num, "payout exceeds the residual backing");
        }

        // 3. Sweep the winner's fee debt out of its payout.
        let (swept, remaining_debt) = fee_debt_sweep(fee_debt_a, payout_a);
        let net_a = payout_a - swept;
        assert_eq!(swept + remaining_debt, fee_debt_a);
        assert_eq!(net_a + swept, payout_a);

        // 4. Write the loser's deficit off against insurance.
        let (writeoff, new_insurance) = loss_writeoff(loss_b, insurance);
        assert!(writeoff <= loss_b);
        assert_eq!(writeoff + new_insurance, insurance);

        // 5. Settlement-wide bound: everything that leaves the vault
        //    side (winner payout + insurance writeoff) is backed.
        assert!(
            payout_a + writeoff <= vault,
            "settlement pays out more than the vault holds"
        );
    }
}
