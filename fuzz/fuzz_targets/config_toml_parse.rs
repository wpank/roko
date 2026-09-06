//! Fuzz target: TOML config parsing in roko-core (RokoConfig).
//!
//! `RokoConfig::from_toml` is the entry point that all roko binaries use when
//! loading `roko.toml`.  It calls `toml::from_str` on the file contents and
//! must never panic or corrupt internal state on malformed input; returning an
//! error is the correct behaviour for invalid TOML.
//!
//! We also exercise the raw `toml::from_str` path directly so that the fuzzer
//! can find panics in the TOML library itself when fed adversarial byte
//! sequences that happen to be valid UTF-8.

#![no_main]

use libfuzzer_sys::fuzz_target;
use roko_core::config::RokoConfig;

fuzz_target!(|data: &[u8]| {
    // TOML parsing operates on &str; non-UTF-8 input is an immediate error.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Primary surface: the unified config entry point used by all binaries.
    let _ = RokoConfig::from_toml(text);

    // Secondary surface: raw TOML value parse (exercises the TOML parser
    // independent of the serde schema).
    let _: Result<toml::Value, _> = toml::from_str(text);
});
