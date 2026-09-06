//! Fuzz target: verify-chain output parsers in roko-gate.
//!
//! The verify-chain gate runs a sequence of user-authored shell commands and
//! parses their combined stdout to decide pass/fail.  Three pure-function
//! parsers operate on the raw output string:
//!
//! * `parse_verify_chain_counts` — count `[PASS]`/`[FAIL]` markers.
//! * `parse_zero_test_steps`     — detect steps with zero test output.
//! * `parse_verify_chain_failure`— extract a digest of failing lines.
//!
//! All three are exercised here because they share the same input surface
//! (multi-line command output) and have distinct branching logic that an
//! adversarial corpus can explore independently.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_gate::verify_chain_gate::{
    parse_verify_chain_counts, parse_verify_chain_failure, parse_zero_test_steps,
};

fuzz_target!(|data: &[u8]| {
    // All three parsers accept &str; non-UTF-8 input is silently skipped.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Count [PASS]/[FAIL] markers — must not panic.
    let (_pass, _fail) = parse_verify_chain_counts(text);

    // Detect zero-test steps — must not panic or loop infinitely.
    let _ = parse_zero_test_steps(text);

    // Extract failure digest — must not panic or grow without bound.
    let digest = parse_verify_chain_failure(text);
    // The function promises a 2000-byte cap; verify it holds.
    debug_assert!(
        digest.len() <= 2016,
        "parse_verify_chain_failure exceeded 2000-byte cap: {} bytes",
        digest.len()
    );
});
