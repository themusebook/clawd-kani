//! Fail-to-fix #6: split residue leak.
//!
//! Splitting a pot into equal floored shares (`total / 3` three times)
//! strands the remainder: `total % 3` atoms belong to nobody. Repeated
//! every epoch, the leak compounds — and if the stranded residue sits
//! in a vault account, it silently becomes unclaimable backing. The
//! fix routes the residue to a designated sink share.
//!
//! Run:
//! ```text
//! cargo kani --manifest-path examples/06-split-residue-leak/Cargo.toml
//! ```
//! Expected: `vulnerable::proof_split_conserves` FAILS (e.g. total = 1
//! distributes 0); `fixed::proof_split_conserves` VERIFIES.

fn main() {
    println!("This is a Kani fail-to-fix example. Run it with:");
    println!("  cargo kani --manifest-path examples/06-split-residue-leak/Cargo.toml");
}

/// VULNERABLE — DO NOT USE. The remainder is dropped on the floor.
#[cfg(kani)]
mod vulnerable {
    pub fn split3(total: u64) -> (u64, u64, u64) {
        let share = total / 3;
        (share, share, share)
    }

    #[kani::proof]
    fn proof_split_conserves() {
        clawd_kani::token::assert_three_way_split_conserves(split3);
    }
}

/// FIXED: the last share is the sink — it absorbs the residue, so the
/// parts always sum back to the total.
#[cfg(kani)]
mod fixed {
    pub fn split3(total: u64) -> (u64, u64, u64) {
        let share = total / 3;
        (share, share, total - 2 * share)
    }

    #[kani::proof]
    fn proof_split_conserves() {
        clawd_kani::token::assert_three_way_split_conserves(split3);
    }
}
