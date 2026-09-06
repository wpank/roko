//! Fuzz target: TOML config parsing via the raw `toml` crate.
//!
//! This is a lightweight complement to `config_toml_parse.rs`: it exercises
//! the TOML parser at the `toml::Value` level without going through the full
//! serde schema, which helps the fuzzer explore TOML grammar edges that the
//! schema-aware path would reject early.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // Parse as an untyped TOML value — must not panic.
    let _: Result<toml::Value, _> = toml::from_str(text);
});
