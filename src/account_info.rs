//! Symbolic `solana_program::AccountInfo` generators and lamport
//! conservation snapshots.
//!
//! Feature: `solana-account-info` (pulls `solana-program` 2.x).
//!
//! [`any_account_info`] builds a *real* `AccountInfo<'static>` whose
//! fields are symbolic but constrained by an [`AccountConfig`]: key and
//! owner are arbitrary 32-byte arrays, the data buffer is fully
//! symbolic, and lamports are symbolic within an exact value or range
//! (optionally floored at the rent-exempt minimum). Storage is
//! `Box::leak`ed to obtain the `'static` lifetime `AccountInfo` wants;
//! under Kani the leak is irrelevant, and these generators are only
//! ever compiled under `cfg(kani)` anyway.

use solana_program::{account_info::AccountInfo, pubkey::Pubkey, rent::Rent};
use std::ops::RangeInclusive;

use crate::generators::*;

#[derive(Clone, Debug)]
enum Lamports {
    Any,
    Exact(u64),
    Range(RangeInclusive<u64>),
}

/// Constraints for [`any_account_info`].
#[derive(Clone, Debug)]
pub struct AccountConfig {
    key: Option<Pubkey>,
    owner: Option<Pubkey>,
    lamports: Lamports,
    rent_exempt: bool,
    pub is_signer: bool,
    pub is_writable: bool,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            key: None,
            owner: None,
            lamports: Lamports::Any,
            rent_exempt: true,
            is_signer: false,
            is_writable: false,
            executable: false,
            rent_epoch: 0,
        }
    }
}

impl AccountConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key(mut self, key: Pubkey) -> Self {
        self.key = Some(key);
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn signer(mut self) -> Self {
        self.is_signer = true;
        self
    }

    pub fn writable(mut self) -> Self {
        self.is_writable = true;
        self
    }

    pub fn payer(mut self) -> Self {
        self.is_signer = true;
        self.is_writable = true;
        self
    }

    pub fn executable(mut self) -> Self {
        self.executable = true;
        self
    }

    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exact(lamports);
        self
    }

    pub fn lamports_range(mut self, range: RangeInclusive<u64>) -> Self {
        self.lamports = Lamports::Range(range);
        self
    }

    /// When enabled (the default), the generated balance is at least
    /// the rent-exempt minimum for the data length; this raises the
    /// floor of a configured range rather than replacing it.
    pub fn rent_exempt(mut self, enabled: bool) -> Self {
        self.rent_exempt = enabled;
        self
    }

    pub fn rent_epoch(mut self, epoch: u64) -> Self {
        self.rent_epoch = epoch;
        self
    }
}

fn any_pubkey() -> Pubkey {
    Pubkey::new_from_array(kani::any())
}

/// Generates a symbolic `AccountInfo` with a `DATA_LEN`-byte data
/// buffer, satisfying `cfg`.
pub fn any_account_info<const DATA_LEN: usize>(cfg: AccountConfig) -> AccountInfo<'static> {
    let key: &Pubkey = Box::leak(Box::new(cfg.key.unwrap_or_else(any_pubkey)));
    let owner: &Pubkey = Box::leak(Box::new(cfg.owner.unwrap_or_else(any_pubkey)));

    let mut lamports_value: u64 = match &cfg.lamports {
        Lamports::Any => kani::any(),
        Lamports::Exact(v) => *v,
        Lamports::Range(range) => any_u64_range(*range.start(), *range.end()),
    };
    if cfg.rent_exempt {
        let minimum = Rent::default().minimum_balance(DATA_LEN);
        if lamports_value < minimum {
            lamports_value = minimum;
        }
    }

    let lamports: &'static mut u64 = Box::leak(Box::new(lamports_value));
    let data: &'static mut [u8; DATA_LEN] = Box::leak(Box::new(kani::any()));

    // solana-account-info 2.3 `AccountInfo::new` takes the raw
    // `&mut` references and wraps them in `Rc<RefCell<..>>` itself.
    AccountInfo::new(
        key,
        cfg.is_signer,
        cfg.is_writable,
        lamports,
        data,
        owner,
        cfg.executable,
        cfg.rent_epoch,
    )
}

/// Current lamport balance of an account.
#[must_use]
pub fn lamports(account: &AccountInfo<'_>) -> u64 {
    **account.lamports.borrow()
}

/// Total lamports across accounts, in `u128` so sums cannot overflow.
#[must_use]
pub fn sum_lamports(accounts: &[&AccountInfo<'_>]) -> u128 {
    accounts.iter().map(|a| lamports(a) as u128).sum()
}

/// A point-in-time total-lamports snapshot for conservation checks:
/// snapshot before a modeled operation, then assert the total is
/// unchanged afterwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LamportSnapshot(pub u128);

impl LamportSnapshot {
    pub fn new(accounts: &[&AccountInfo<'_>]) -> Self {
        Self(sum_lamports(accounts))
    }

    /// True if the accounts still hold exactly the snapshotted total.
    #[must_use]
    pub fn unchanged(&self, accounts: &[&AccountInfo<'_>]) -> bool {
        sum_lamports(accounts) == self.0
    }

    /// Asserts the accounts still hold exactly the snapshotted total.
    pub fn assert_conserved(&self, accounts: &[&AccountInfo<'_>]) {
        assert!(
            self.unchanged(accounts),
            "lamport total changed: snapshot {}, now {}",
            self.0,
            sum_lamports(accounts)
        );
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn proof_generator_respects_config() {
        let cfg = AccountConfig::new()
            .signer()
            .lamports_range(1_000..=2_000)
            .rent_exempt(false);
        let info = any_account_info::<64>(cfg);
        assert!(info.is_signer);
        assert!(!info.is_writable);
        assert!(!info.executable);
        let balance = lamports(&info);
        assert!(balance >= 1_000 && balance <= 2_000);
        assert_eq!(info.data_len(), 64);
    }

    #[kani::proof]
    fn proof_generator_rent_exempt_floor() {
        let cfg = AccountConfig::new().rent_exempt(true);
        let info = any_account_info::<32>(cfg);
        let minimum = Rent::default().minimum_balance(32);
        assert!(lamports(&info) >= minimum);
    }

    #[kani::proof]
    fn proof_transfer_conserves_lamports() {
        // Two symbolic accounts with balances bounded so the credit
        // side cannot overflow.
        let from = any_account_info::<32>(
            AccountConfig::new()
                .payer()
                .lamports_range(0..=(1u64 << 62))
                .rent_exempt(false),
        );
        let to = any_account_info::<32>(
            AccountConfig::new()
                .writable()
                .lamports_range(0..=(1u64 << 62))
                .rent_exempt(false),
        );
        let accounts: [&AccountInfo<'_>; 2] = [&from, &to];
        let snapshot = LamportSnapshot::new(&accounts);

        let amount = any_u64_range(0, lamports(&from));
        **from.lamports.borrow_mut() -= amount;
        **to.lamports.borrow_mut() += amount;

        assert!(snapshot.unchanged(&accounts));
        snapshot.assert_conserved(&accounts);
        assert_eq!(sum_lamports(&accounts), snapshot.0);
    }

    // The unwind bound keeps CBMC from unrolling the generic
    // iterator fold in `sum_lamports` indefinitely; the slice here is
    // literally empty, so a bound of 4 is exact, not approximate.
    #[kani::proof]
    #[kani::unwind(4)]
    fn proof_snapshot_of_empty_is_zero() {
        let accounts: [&AccountInfo<'_>; 0] = [];
        let snapshot = LamportSnapshot::new(&accounts);
        assert_eq!(snapshot.0, 0);
        assert!(snapshot.unchanged(&accounts));
    }
}
