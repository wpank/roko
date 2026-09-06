//! Fuzz target: JSONL parsing in roko-fs (FileSubstrate line reader).
//!
//! Exercises the `serde_json::from_str::<Signal>` path used by
//! `FileSubstrate` when replaying `.roko/engrams.jsonl` on startup, plus the
//! `ClassifiedRecord` path used by `read_classified_jsonl` for transcript
//! persistence. Both surfaces process untrusted on-disk bytes and must never
//! panic or allocate unboundedly on adversarial input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_core::Signal;
use roko_fs::ClassifiedRecord;

fuzz_target!(|data: &[u8]| {
    // Both parsers require valid UTF-8; skip non-UTF-8 sequences.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Exercise the primary JSONL reader path: one Signal per line.
    for line in text.lines() {
        let _ = serde_json::from_str::<Signal>(line);
    }

    // Exercise the classified transcript reader path.
    for line in text.lines() {
        let _ = serde_json::from_str::<ClassifiedRecord>(line);
    }

    // Also try treating the entire input as a single JSONL line.
    let _ = serde_json::from_str::<Signal>(text);
    let _ = serde_json::from_str::<ClassifiedRecord>(text);
});
