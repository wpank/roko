//! Fuzz target: Signal (Engram) JSONL line parsing.
//!
//! Exercises the `serde_json::from_str::<Signal>` path that every JSONL
//! reader in the workspace hits when processing `.roko/engrams.jsonl`.
//! The goal is to find panics, stack overflows, or excessive allocations
//! on adversarial input rather than expecting successful parsing.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_core::Signal;

fuzz_target!(|data: &[u8]| {
    // Try to interpret the bytes as UTF-8 first; non-UTF-8 input is
    // rejected by serde_json before any parsing happens.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Attempt to deserialize a Signal from the fuzzed input.
    // We do not care whether parsing succeeds -- we care that it
    // does not panic, hang, or allocate unboundedly.
    let _ = serde_json::from_str::<Signal>(text);

    // Also exercise TaskMetric parsing (same JSONL path).
    let _ = serde_json::from_str::<roko_core::TaskMetric>(text);
});
