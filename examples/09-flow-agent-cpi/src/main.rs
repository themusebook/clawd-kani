//! End-to-end flow #2: agent CPI flow (feature `solana-agent`).
//!
//! A full agent operation cycle: the agent activates (FSM), moves
//! lamports from its treasury under a conservation snapshot, performs
//! a guarded CPI to an allowlisted program under a reentrancy guard,
//! checks oracle consensus before acting, pauses and finally closes
//! (FSM again). Every step is asserted with the clawd-kani agent
//! helpers.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/09-flow-agent-cpi/Cargo.toml
//! ```
//! Expected: the harness VERIFIES.

fn main() {
    println!("This is a Kani end-to-end flow example. Run it with:");
    println!("  cargo kani --manifest-path examples/09-flow-agent-cpi/Cargo.toml");
}

#[cfg(kani)]
mod flow {
    use clawd_kani::agent::*;
    use clawd_kani::generators::*;

    const TOKEN_PROGRAM_ID: [u8; 32] = [7u8; 32];
    const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

    clawd_kani::cpi_contract! {
        name: pay_invoice,
        program: TOKEN_PROGRAM_ID,
        allowlist: [TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID],
        args: |amount: u64|,
        requires: { assert!(amount > 0); },
        body: { let _fee = amount / 100; },
        ensures: { assert!(amount <= 1_000_000_000); },
        record: { lamports_transferred: amount, accounts_touched: 2 },
    }

    #[kani::proof]
    #[kani::unwind(20)]
    fn proof_agent_cpi_flow_end_to_end() {
        // 1. The agent and its treasury appear, idle and funded.
        let mut agent = any_agent_account(
            AgentConfig::new()
                .signer()
                .writable()
                .lamports_range(0, 1u64 << 62)
                .state(AgentState::Idle),
        );
        let treasury = any_agent_account(
            AgentConfig::new()
                .writable()
                .lamports_range(0, 1u64 << 62)
                .state(AgentState::Active),
        );
        assert_account_invariants(&agent);

        // 2. FSM: Idle -> Active is the only way in.
        assert_fsm_transition_guard(AgentState::Idle, AgentState::Active);
        agent.state = AgentState::Active;

        // 3. Oracle consensus before moving funds: two feeds at the
        //    reference outvote one outlier.
        let reference = any_u128_up_to(1u128 << 56);
        kani::assume(reference > 0);
        assert_oracle_consensus([reference, reference, reference * 10], reference, 50);

        // 4. Move lamports treasury -> agent under conservation.
        let before = [agent.clone(), treasury.clone()];
        let amount = any_u64_range(0, treasury.lamports);
        let mut treasury_after = treasury.clone();
        treasury_after.lamports -= amount;
        let mut agent_after = agent.clone();
        agent_after.lamports = agent_after.lamports.checked_add(amount).unwrap();
        let after = [agent_after.clone(), treasury_after];
        assert_lamport_conservation(&before, &after);
        assert_no_reentrancy(&after);

        // 5. Guarded CPI under a reentrancy guard: the call is
        //    allowlisted, recorded, and cannot be nested.
        let mut guard = CallGuard::new();
        let mut log = CpiLog::new();
        assert!(guard.enter());
        assert!(!guard.enter(), "nested call must be rejected");
        let invoice = any_u64_range(1, 1_000_000_000);
        pay_invoice(invoice, &mut log);
        guard.exit();
        assert_eq!(log.count(), 1);
        assert_cpi_authorized(&log, &[TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID]);

        // 6. PDA derivation for the receipt account stays in limits.
        assert_pda_seeds_valid(&[32, 8, 4]);

        // 7. FSM wind-down: Active -> Paused -> Active -> Closed, and
        //    Closed is terminal.
        assert_fsm_transition_guard(AgentState::Active, AgentState::Paused);
        assert_fsm_transition_guard(AgentState::Paused, AgentState::Active);
        assert_fsm_transition_guard(AgentState::Active, AgentState::Closed);
        assert!(!can_transition(AgentState::Closed, AgentState::Active));
    }
}
