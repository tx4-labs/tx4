//! TX4 server binary.
//!
//! Phase-1A: bootstrap skeleton only. No HTTP listen, routes, or DB.

fn main() {
    // Reference library crates so the dependency boundary is exercised at link time.
    let _ = (
        tx4_api::crate_name(),
        tx4_config::crate_name(),
        tx4_observability::crate_name(),
    );
}
