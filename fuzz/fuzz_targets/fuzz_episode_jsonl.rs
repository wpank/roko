//! Fuzz target: Episode JSONL parsing in roko-learn.
//!
//! `EpisodeLogger::read_all` replays `.roko/episodes.jsonl` by deserializing
//! one `Episode` per line.  Adversarial JSONL (truncated records, extra
//! fields, incorrect types) must not cause panics or infinite loops.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_learn::episode_logger::Episode;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Simulate the JSONL reader: one Episode per line.
    for line in text.lines() {
        let _ = serde_json::from_str::<Episode>(line);
    }

    // Also try the whole buffer as a single JSON object.
    let _ = serde_json::from_str::<Episode>(text);
});
