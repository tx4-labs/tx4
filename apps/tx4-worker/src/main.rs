//! TX4 worker binary.
//!
//! Phase-1A: bootstrap skeleton only. No outbox claim/lease/fencing loops.

fn main() {
    let _ = (
        tx4_worker::crate_name(),
        tx4_config::crate_name(),
        tx4_observability::crate_name(),
    );
}
