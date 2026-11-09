//! Symbolic agent accounts and agent-protocol invariants.
//!
//! Feature: `solana-agent`.
//!
//! Models an agent's on-chain accounts abstractly (`[u8; 32]` program
//! and account ids, `u64` lamports) so protocol-level properties —
//! lamport conservation, reentrancy, CPI allowlists, PDA seed limits,
//! oracle consensus and FSM transition guards — can be proved without
//! pulling in the full Solana runtime. For proofs over real
//! `solana_program::AccountInfo` values, see the `solana-account-info`
//! feature and [`crate::account_info`].

use crate::generators::*;

// ---------------------------------------------------------------------------
// Agent state machine
// ---------------------------------------------------------------------------

/// Lifecycle states for an agent account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentState {
    /// Created but not yet activated.
    Idle,
    /// Operating normally.
    Active,
    /// Temporarily suspended; can resume or close.
    Paused,
    /// Terminal: no outgoing transitions.
    Closed,
}

/// Symbolic agent state (uniform over the four variants).
pub fn any_agent_state() -> AgentState {
    let tag = any_u8_up_to(3);
    match tag {
        0 => AgentState::Idle,
        1 => AgentState::Active,
        2 => AgentState::Paused,
        _ => AgentState::Closed,
    }
}

/// The legal transition table for [`AgentState`].
#[must_use]
pub fn can_transition(from: AgentState, to: AgentState) -> bool {
    use AgentState::*;
    matches!(
        (from, to),
        (Idle, Active) | (Active, Paused) | (Paused, Active) | (Active, Closed) | (Paused, Closed)
    )
}

/// Asserts that the `from -> to` transition is legal for [`AgentState`].
pub fn assert_fsm_transition_guard(from: AgentState, to: AgentState) {
    assert!(
        can_transition(from, to),
        "illegal agent state transition {from:?} -> {to:?}"
    );
}

/// Generic FSM guard: asserts `(from, to)` is a member of an explicit
/// allowlist of transitions, for any state type.
pub fn assert_transition_allowed<S: Copy + PartialEq>(from: S, to: S, allowed: &[(S, S)]) {
    assert!(
        allowed.contains(&(from, to)),
        "state transition not in the allowlist"
    );
}

// ---------------------------------------------------------------------------
// Symbolic agent accounts
// ---------------------------------------------------------------------------

/// An abstract agent account: just the fields protocol invariants
/// reason about.
#[derive(Clone, Debug)]
pub struct AgentAccount {
    pub pubkey: [u8; 32],
    pub owner: [u8; 32],
    pub lamports: u64,
    pub is_signer: bool,
    pub is_writable: bool,
    pub executable: bool,
    pub state: AgentState,
    /// True while a (modeled) CPI from this account is on the stack.
    pub in_call: bool,
}

/// Builder-style constraints for [`any_agent_account`].
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub owner: Option<[u8; 32]>,
    pub is_signer: bool,
    pub is_writable: bool,
    pub executable: bool,
    pub lamports_min: u64,
    pub lamports_max: u64,
    pub state: Option<AgentState>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            owner: None,
            is_signer: false,
            is_writable: false,
            executable: false,
            lamports_min: 0,
            lamports_max: 1u64 << 63,
            state: None,
        }
    }
}

impl AgentConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn signer(mut self) -> Self {
        self.is_signer = true;
        self
    }

    pub fn writable(mut self) -> Self {
        self.is_writable = true;
        self
    }

    pub fn executable(mut self) -> Self {
        self.executable = true;
        self
    }

    pub fn owner(mut self, owner: [u8; 32]) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn lamports_range(mut self, min: u64, max: u64) -> Self {
        self.lamports_min = min;
        self.lamports_max = max;
        self
    }

    pub fn state(mut self, state: AgentState) -> Self {
        self.state = Some(state);
        self
    }
}

/// Generates a symbolic [`AgentAccount`] satisfying `config`.
pub fn any_agent_account(config: AgentConfig) -> AgentAccount {
    AgentAccount {
        pubkey: kani::any(),
        owner: config.owner.unwrap_or_else(|| kani::any()),
        lamports: any_u64_range(config.lamports_min, config.lamports_max),
        is_signer: config.is_signer,
        is_writable: config.is_writable,
        executable: config.executable,
        state: config.state.unwrap_or_else(any_agent_state),
        in_call: false,
    }
}

// ---------------------------------------------------------------------------
// Lamport conservation + account invariants
// ---------------------------------------------------------------------------

/// Total lamports across accounts, in `u128` so sums cannot overflow.
#[must_use]
pub fn total_lamports(accounts: &[AgentAccount]) -> u128 {
    accounts.iter().map(|a| a.lamports as u128).sum()
}

/// Asserts two account snapshots hold the same accounts (by count) and
/// the same total lamports: value moved, but none was created or lost.
pub fn assert_lamport_conservation(before: &[AgentAccount], after: &[AgentAccount]) {
    assert_eq!(before.len(), after.len(), "account count changed");
    assert_eq!(
        total_lamports(before),
        total_lamports(after),
        "lamports were created or destroyed"
    );
}

/// Asserts no account is in the middle of a call — the quiescent-point
/// check that rules out reentrancy in the modeled call graph.
pub fn assert_no_reentrancy(accounts: &[AgentAccount]) {
    for account in accounts {
        assert!(!account.in_call, "account is reentered");
    }
}

/// Baseline per-account invariants:
/// - an account can only be `in_call` while [`AgentState::Active`];
/// - an executable (program) account is never a signer;
/// - a [`AgentState::Closed`] account is never in a call.
pub fn assert_account_invariants(account: &AgentAccount) {
    if account.in_call {
        assert_eq!(account.state, AgentState::Active, "in_call outside Active");
    }
    if account.executable {
        assert!(!account.is_signer, "executable account cannot sign");
    }
    if account.state == AgentState::Closed {
        assert!(!account.in_call, "closed account is in a call");
    }
}

/// A reentrancy guard for modeled CPI: the first `enter` succeeds,
/// nested entries fail until `exit`.
#[derive(Clone, Debug, Default)]
pub struct CallGuard {
    entered: bool,
}

impl CallGuard {
    pub const fn new() -> Self {
        Self { entered: false }
    }

    /// Returns `true` if the call may proceed, `false` if reentrant.
    pub fn enter(&mut self) -> bool {
        if self.entered {
            false
        } else {
            self.entered = true;
            true
        }
    }

    pub fn exit(&mut self) {
        self.entered = false;
    }

    #[must_use]
    pub fn is_entered(&self) -> bool {
        self.entered
    }
}

// ---------------------------------------------------------------------------
// CPI records, allowlists, and the cpi_stub! / cpi_contract! macros
// ---------------------------------------------------------------------------

/// A single recorded (modeled) CPI call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpiRecord {
    pub target_program: [u8; 32],
    pub instruction_name: &'static str,
    pub lamports_transferred: u64,
    pub accounts_touched: u8,
}

/// Fixed-size CPI call log. No heap allocation.
pub struct CpiLog {
    records: [Option<CpiRecord>; 16],
    count: usize,
}

/// Helper: a `[None; 16]` array without nightly const tricks.
const fn none_array() -> [Option<CpiRecord>; 16] {
    [
        None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        None,
    ]
}

impl CpiLog {
    pub const fn new() -> Self {
        Self {
            records: none_array(),
            count: 0,
        }
    }

    /// Appends a record. Panics (a proof failure) if the log is full.
    pub fn record(&mut self, r: CpiRecord) {
        assert!(self.count < 16, "CpiLog overflow: max 16 records");
        self.records[self.count] = Some(r);
        self.count += 1;
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn iter(&self) -> impl Iterator<Item = &CpiRecord> {
        self.records[..self.count]
            .iter()
            .filter_map(|slot| slot.as_ref())
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&CpiRecord> {
        if index >= self.count {
            return None;
        }
        self.records[index].as_ref()
    }
}

impl Default for CpiLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Membership test for a CPI allowlist of program ids.
#[must_use]
pub fn cpi_target_allowed(allowlist: &[[u8; 32]], target: &[u8; 32]) -> bool {
    allowlist.iter().any(|p| p == target)
}

/// Asserts every recorded CPI call targeted an allowlisted program.
pub fn assert_cpi_authorized(log: &CpiLog, allowlist: &[[u8; 32]]) {
    for record in log.iter() {
        assert!(
            cpi_target_allowed(allowlist, &record.target_program),
            "CPI to a program outside the allowlist"
        );
    }
}

/// Generates a CPI contract stub function with pre/post conditions.
///
/// The generated function runs the `pre` assertions, records the call
/// into a [`CpiLog`], then runs the `post` assertions. It models the
/// *interface* of a cross-program call so callers can be verified
/// against the conditions without the real program.
///
/// # Example
/// ```ignore
/// cpi_stub! {
///     name: token_transfer,
///     program: TOKEN_PROGRAM_ID,
///     args: |amount: u64|,
///     pre: { assert!(amount > 0); },
///     post: { assert!(amount <= 1_000_000); },
/// }
/// ```
#[macro_export]
macro_rules! cpi_stub {
    (
        name: $name:ident,
        program: $program:expr,
        args: |$($arg:ident : $arg_ty:ty),* $(,)?|,
        pre: $pre_body:block,
        post: $post_body:block $(,)?
    ) => {
        fn $name(
            $($arg : $arg_ty,)*
            cpi_log: &mut $crate::agent::CpiLog,
        ) {
            $pre_body
            cpi_log.record($crate::agent::CpiRecord {
                target_program: $program,
                instruction_name: stringify!($name),
                lamports_transferred: 0,
                accounts_touched: 0,
            });
            $post_body
        }
    };
}

/// Generates a guarded CPI function with an allowlist, pre/post
/// conditions, and call recording.
///
/// The generated function asserts — in order — that `program` is in
/// `allowlist`, runs the `requires` assertions, records the call into a
/// [`CpiLog`] with the given accounting, runs the modeled `body`, and
/// finally runs the `ensures` assertions.
///
/// # Example
/// ```ignore
/// cpi_contract! {
///     name: guarded_transfer,
///     program: TOKEN_PROGRAM_ID,
///     allowlist: [TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID],
///     args: |amount: u64|,
///     requires: { assert!(amount > 0); },
///     body: { /* modeled lamport movement */ },
///     ensures: { assert!(amount <= MAX_TRANSFER); },
///     record: { lamports_transferred: amount, accounts_touched: 2 },
/// }
/// ```
#[macro_export]
macro_rules! cpi_contract {
    (
        name: $name:ident,
        program: $program:expr,
        allowlist: [$($allowed:expr),+ $(,)?],
        args: |$($arg:ident : $arg_ty:ty),* $(,)?|,
        requires: $requires:block,
        body: $body:block,
        ensures: $ensures:block,
        record: {
            lamports_transferred: $lamports_transferred:expr,
            accounts_touched: $accounts_touched:expr $(,)?
        } $(,)?
    ) => {
        fn $name(
            $($arg : $arg_ty,)*
            cpi_log: &mut $crate::agent::CpiLog,
        ) {
            let __allowlist: [[u8; 32]; _] = [$($allowed),+];
            assert!(
                $crate::agent::cpi_target_allowed(&__allowlist, &$program),
                "CPI target is not in the allowlist"
            );
            $requires
            let __lamports_transferred: u64 = $lamports_transferred;
            let __accounts_touched: u8 = $accounts_touched;
            cpi_log.record($crate::agent::CpiRecord {
                target_program: $program,
                instruction_name: stringify!($name),
                lamports_transferred: __lamports_transferred,
                accounts_touched: __accounts_touched,
            });
            $body
            $ensures
        }
    };
}

// ---------------------------------------------------------------------------
// PDA seed limits (Solana: at most 16 seeds, each at most 32 bytes)
// ---------------------------------------------------------------------------

/// Solana's maximum number of PDA seeds.
pub const MAX_PDA_SEEDS: usize = 16;
/// Solana's maximum length of a single PDA seed.
pub const MAX_PDA_SEED_LEN: usize = 32;

/// Whether a seed layout (given as seed lengths) respects both limits.
#[must_use]
pub fn seed_layout_valid(seed_lens: &[usize]) -> bool {
    seed_lens.len() <= MAX_PDA_SEEDS && seed_lens.iter().all(|&len| len <= MAX_PDA_SEED_LEN)
}

/// Asserts a seed count respects [`MAX_PDA_SEEDS`].
pub fn assert_seed_count_valid(seed_count: usize) {
    assert!(seed_count <= MAX_PDA_SEEDS, "too many PDA seeds");
}

/// Asserts a full seed layout respects both PDA limits.
pub fn assert_pda_seeds_valid(seed_lens: &[usize]) {
    assert!(seed_layout_valid(seed_lens), "PDA seed layout out of limits");
}

// ---------------------------------------------------------------------------
// Oracle consensus (median-of-3)
// ---------------------------------------------------------------------------

/// Median of three values.
#[must_use]
pub fn median3(a: u128, b: u128, c: u128) -> u128 {
    if a <= b {
        if b <= c {
            b
        } else if a <= c {
            c
        } else {
            a
        }
    } else if a <= c {
        a
    } else if b <= c {
        c
    } else {
        b
    }
}

/// Whether the median of `prices` agrees with `reference` within
/// `tolerance_bps` (relative, in basis points).
#[must_use]
pub fn oracle_consensus_ok(prices: [u128; 3], reference: u128, tolerance_bps: u16) -> bool {
    let median = median3(prices[0], prices[1], prices[2]);
    let diff = median.abs_diff(reference);
    diff <= crate::math::mul_div_floor(reference, tolerance_bps as u128, 10_000)
}

/// Asserts median-of-3 oracle consensus against a reference price.
pub fn assert_oracle_consensus(prices: [u128; 3], reference: u128, tolerance_bps: u16) {
    assert!(
        oracle_consensus_ok(prices, reference, tolerance_bps),
        "oracle median disagrees with the reference"
    );
}

// ---------------------------------------------------------------------------
// Proofs
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod proofs {
    use super::*;

    // Modeled program ids used by the CPI proofs.
    const TOKEN_PROGRAM_ID: [u8; 32] = [7u8; 32];
    const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

    crate::cpi_stub! {
        name: token_transfer,
        program: TOKEN_PROGRAM_ID,
        args: |amount: u64|,
        pre: { assert!(amount > 0); },
        post: { assert!(amount <= 1_000_000_000); },
    }

    crate::cpi_contract! {
        name: guarded_transfer,
        program: TOKEN_PROGRAM_ID,
        allowlist: [TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID],
        args: |amount: u64|,
        requires: { assert!(amount > 0); },
        body: { let _modeled_fee = amount / 100; },
        ensures: { assert!(amount <= 1_000_000_000); },
        record: { lamports_transferred: amount, accounts_touched: 2 },
    }

    #[kani::proof]
    fn proof_fsm_closed_is_terminal() {
        let to = any_agent_state();
        assert!(!can_transition(AgentState::Closed, to));
    }

    #[kani::proof]
    fn proof_fsm_guard_matches_allowlist() {
        // The guard must accept exactly the pairs in the explicit table.
        let from = any_agent_state();
        let to = any_agent_state();
        let allowed: [(AgentState, AgentState); 5] = [
            (AgentState::Idle, AgentState::Active),
            (AgentState::Active, AgentState::Paused),
            (AgentState::Paused, AgentState::Active),
            (AgentState::Active, AgentState::Closed),
            (AgentState::Paused, AgentState::Closed),
        ];
        assert_eq!(can_transition(from, to), allowed.contains(&(from, to)));
        if can_transition(from, to) {
            assert_transition_allowed(from, to, &allowed);
            assert_fsm_transition_guard(from, to);
        }
    }

    #[kani::proof]
    fn proof_any_agent_account_respects_config() {
        let owner = [9u8; 32];
        let account = any_agent_account(
            AgentConfig::new()
                .signer()
                .owner(owner)
                .lamports_range(100, 200)
                .state(AgentState::Active),
        );
        assert!(account.is_signer);
        assert_eq!(account.owner, owner);
        assert!(account.lamports >= 100 && account.lamports <= 200);
        assert_eq!(account.state, AgentState::Active);
        assert!(!account.in_call);
        assert_account_invariants(&account);
    }

    #[kani::proof]
    fn proof_lamport_conservation_after_transfer() {
        // Balances capped at 2^62 so the modeled credit leg
        // (b + amount, amount <= a <= 2^62) can never overflow u64.
        let cfg = AgentConfig::new().lamports_range(0, 1u64 << 62);
        let a = any_agent_account(cfg.clone());
        let b = any_agent_account(cfg.clone());
        let c = any_agent_account(cfg);
        let before = [a.clone(), b.clone(), c.clone()];

        let amount = any_u64_range(0, a.lamports);
        let mut a2 = a;
        let mut b2 = b;
        a2.lamports -= amount;
        b2.lamports = b2.lamports.checked_add(amount).unwrap();
        let after = [a2, b2, c];

        assert_lamport_conservation(&before, &after);
        assert_no_reentrancy(&after);
    }

    #[kani::proof]
    fn proof_call_guard_blocks_reentry() {
        let mut guard = CallGuard::new();
        assert!(guard.enter(), "first entry must succeed");
        assert!(guard.is_entered());
        assert!(!guard.enter(), "nested entry must be rejected");
        guard.exit();
        assert!(!guard.is_entered());
        assert!(guard.enter(), "entry after exit must succeed again");
    }

    #[kani::proof]
    #[kani::unwind(20)]
    fn proof_cpi_stub_records_and_authorizes() {
        let amount = any_u64_range(1, 1_000_000_000);
        let mut log = CpiLog::new();
        token_transfer(amount, &mut log);
        assert_eq!(log.count(), 1);
        let record = log.get(0).unwrap();
        assert_eq!(record.target_program, TOKEN_PROGRAM_ID);
        assert_eq!(record.instruction_name, "token_transfer");
        assert_cpi_authorized(&log, &[TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID]);
    }

    #[kani::proof]
    #[kani::unwind(20)]
    fn proof_cpi_contract_enforces_conditions() {
        let amount = any_u64_range(1, 1_000_000_000);
        let mut log = CpiLog::new();
        guarded_transfer(amount, &mut log);
        assert_eq!(log.count(), 1);
        let record = log.get(0).unwrap();
        assert_eq!(record.lamports_transferred, amount);
        assert_eq!(record.accounts_touched, 2);
        assert_cpi_authorized(&log, &[TOKEN_PROGRAM_ID]);
    }

    #[kani::proof]
    fn proof_cpi_allowlist_membership() {
        let target: [u8; 32] = kani::any();
        let allowed = cpi_target_allowed(&[TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID], &target);
        assert_eq!(allowed, target == TOKEN_PROGRAM_ID || target == SYSTEM_PROGRAM_ID);
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn proof_pda_seed_limits() {
        let lens: [usize; 20] = kani::any();
        for &len in lens.iter() {
            kani::assume(len <= 64);
        }
        let n = any_u64_range(0, 20) as usize;
        let slice = &lens[..n];
        // Independent formulation: count and max-length checks.
        let max_len = slice.iter().copied().max().unwrap_or(0);
        let expected = n <= MAX_PDA_SEEDS && max_len <= MAX_PDA_SEED_LEN;
        assert_eq!(seed_layout_valid(slice), expected);
        if expected {
            assert_pda_seeds_valid(slice);
            assert_seed_count_valid(n);
        }
    }

    #[kani::proof]
    fn proof_median3_within_range() {
        let a = any_u128_up_to(1u128 << 64);
        let b = any_u128_up_to(1u128 << 64);
        let c = any_u128_up_to(1u128 << 64);
        let m = median3(a, b, c);
        let lo = a.min(b).min(c);
        let hi = a.max(b).max(c);
        assert!(m >= lo && m <= hi, "median outside the input range");
        assert!(m == a || m == b || m == c, "median must be an input");
    }

    #[kani::proof]
    fn proof_oracle_consensus_majority_rules() {
        // Two feeds at the reference price outvote one wild outlier.
        let reference = any_u128_up_to(1u128 << 60);
        kani::assume(reference > 0);
        let outlier = reference * 10;
        let prices = [reference, reference, outlier];
        assert!(oracle_consensus_ok(prices, reference, 50));
        assert_oracle_consensus(prices, reference, 50);
        // And unanimous agreement always passes, at any tolerance >= 0.
        let flat = [reference, reference, reference];
        assert!(oracle_consensus_ok(flat, reference, 0));
    }
}
