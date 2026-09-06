//! Fuzz target: gate context and verify-step deserialization in roko-gate.
//!
//! `GateTaskContextSpec` and `VerifyStepSpec` are deserialized from JSON when
//! the HTTP control plane or CLI dispatch layer submits a gate request.
//! `GatePayload` is deserialized at the gate service boundary.  All three
//! carry user-controlled strings and must not panic on adversarial JSON.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_gate::{GatePayload, GateTaskContextSpec, VerifyStepSpec};

fuzz_target!(|data: &[u8]| {
    // All three types deserialize from JSON strings.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Task context: title, description, symbols, acceptance criteria.
    let _ = serde_json::from_str::<GateTaskContextSpec>(text);

    // Verify step: phase, command, fail_msg, timeout_ms.
    let _ = serde_json::from_str::<VerifyStepSpec>(text);

    // Gate payload: the full execution context sent to the gate pipeline.
    let _ = serde_json::from_str::<GatePayload>(text);
});
