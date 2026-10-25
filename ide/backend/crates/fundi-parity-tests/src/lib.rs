//! # fundi-parity-tests
//!
//! Week 0 **parity proof chain**: JSON fixtures (recorded from or verified against
//! Python `research/fundi-agent` — baseline tag tracked in `PYTHON_BASELINE.txt`)
//! plus a small harness that runs the **Rust** implementation and compares outputs.
//!
//! - Fixture layout: `fixtures/<logical_module>/*.json`
//! - Schema: see [`harness::ParityFixtureFile`]
//! - Fixtures were captured from the legacy Python reference (one-off); MTECHOS runs Rust only

pub mod harness;
pub mod recorder;

pub use harness::{
    checkpoint_shadow_dir_id, dispatch_case, load_fixture_file, run_all_active_fixtures,
    run_fixture_file, run_fixtures_in_dir, ParityCase, ParityError, ParityFixtureFile,
};
