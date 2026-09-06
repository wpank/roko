//! Fuzz target: `parse_test_counts` in roko-gate (test_gate.rs).
//!
//! The gate receives raw stdout from `cargo test`, `go test`, `pytest`,
//! `jest`, and Foundry and must extract pass/fail/ignored counts without
//! panicking on malformed or adversarial tool output.  We exercise every
//! `BuildSystem` variant so that the regex-free line scanners inside each
//! parser branch are all reachable.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_gate::{BuildSystem, parse_test_counts};

fuzz_target!(|data: &[u8]| {
    // The parsers operate on &str.  Skip non-UTF-8 byte sequences.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Exercise every BuildSystem variant.  The function returns Option<TestCount>
    // and must not panic regardless of the input content.
    let _ = parse_test_counts(text, BuildSystem::Cargo);
    let _ = parse_test_counts(text, BuildSystem::Go);
    let _ = parse_test_counts(text, BuildSystem::Npm);
    let _ = parse_test_counts(text, BuildSystem::Python);
    let _ = parse_test_counts(text, BuildSystem::Forge);
    let _ = parse_test_counts(text, BuildSystem::Make);
});
