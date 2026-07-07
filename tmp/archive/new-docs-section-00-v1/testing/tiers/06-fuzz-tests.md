# Fuzz Tests

> Continuous fuzz testing of parser and serialization boundaries using `cargo-fuzz` + libFuzzer.

**Status**: Built (targets defined; not running in production CI loop yet)
**Crate**: `roko-fuzz` (fuzz harness crate)
**Depends on**: [01-unit-tests.md](01-unit-tests.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Fuzz tests target the parsing and deserialization boundaries of Roko's data types where malformed input could cause panics, incorrect state, or security issues. They use `cargo-fuzz` (libFuzzer backend) with corpus management. Fuzz targets are defined; continuous fuzzing is a roadmap item pending CI infrastructure.

---

## Fuzz Targets

| Target | Boundary | Invariant tested |
|---|---|---|
| `fuzz_engram_deserialize` | `Engram` deserialization | Must not panic on any input bytes |
| `fuzz_gate_input_parse` | `GateInput` construction | Must not panic; invalid inputs return `Err` |
| `fuzz_plan_dag_parse` | Plan DAG JSON parsing | Must not panic; cycles detected |
| `fuzz_content_hash_bytes` | `ContentHash::from_bytes` | Must not panic on any byte slice |
| `fuzz_score_from_json` | `Score` JSON deserialization | Must not panic; out-of-range values return `Err` |
| `fuzz_tape_replay_parse` | LLM tape file parsing | Must not panic on malformed tape |

---

## Running Fuzz Tests

```bash
# Install cargo-fuzz (one time)
cargo install cargo-fuzz

# Run a specific target for 60 seconds
cargo fuzz run fuzz_engram_deserialize -- -max_total_time=60

# Run with an existing corpus
cargo fuzz run fuzz_engram_deserialize fuzz/corpus/engram_deserialize/

# Reproduce a specific crash
cargo fuzz reproduce fuzz_engram_deserialize fuzz/artifacts/engram_deserialize/crash-<hash>
```

---

## Corpus Management

Fuzz corpora live in `fuzz/corpus/<target>/`. Each file is a seed input that guided discovery of interesting code paths. The corpus is committed to the repo for reproducibility.

When a fuzz run discovers new coverage paths, it generates new corpus files. The developer reviews the new files, prunes duplicates, and commits the net additions.

Crash artifacts from live fuzzing live in `fuzz/artifacts/<target>/`. A crash artifact means a found bug. Every crash artifact must have an associated issue tracking:
1. Root cause analysis.
2. Fix PR.
3. Regression test added to the unit suite.

---

## Sanitizer Configuration

Fuzz targets run with AddressSanitizer by default:

```toml
# fuzz/Cargo.toml
[profile.release]
sanitize = "address"
```

To run with other sanitizers:
```bash
# Thread sanitizer
RUSTFLAGS="-Zsanitizer=thread" cargo fuzz run fuzz_engram_deserialize

# Memory sanitizer
RUSTFLAGS="-Zsanitizer=memory" cargo fuzz run fuzz_engram_deserialize
```

---

## Fuzz Target Template

```rust
// fuzz/fuzz_targets/fuzz_engram_deserialize.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use roko_core::Engram;

fuzz_target!(|data: &[u8]| {
    // Must not panic for any input
    let _ = serde_json::from_slice::<Engram>(data);
    // If it parses, the result must be valid
    if let Ok(engram) = serde_json::from_slice::<Engram>(data) {
        // Content hash must be computable
        let _ = engram.content_hash();
        // Round-trip must succeed
        let reserialized = serde_json::to_vec(&engram).unwrap();
        let _ = serde_json::from_slice::<Engram>(&reserialized).unwrap();
    }
});
```

---

## Integration With the Test Suite

Fuzz targets do not run in the normal `cargo test` pass. They require:
1. `cargo-fuzz` to be installed.
2. Nightly Rust (for sanitizer support).
3. Explicit invocation.

Crash reproductions are extracted as unit test regression fixtures when found.

---

## Invariants

- Fuzz targets must not call `unwrap()` on fuzz-controlled input paths.
- A found crash must be tracked as a filed issue within 24 hours.
- Corpus files from fuzz runs are committed, not gitignored.

---

## Roadmap

- [ ] Run `fuzz_engram_deserialize` continuously in a dedicated CI job (Phase 2).
- [ ] Add a `fuzz_gate_verdict_replay` target for the full gate pipeline.
- [ ] Integrate coverage-guided fuzzing results into the coverage report.

## Open Questions

- Should the fuzz CI job use `clusterfuzz` or `oss-fuzz` infrastructure for continuous operation?

## See also

- [04-regression-tests.md](04-regression-tests.md) — converting crash artifacts to regression tests
- [../by-property/engram-serialization-roundtrip.md](../by-property/engram-serialization-roundtrip.md) — the serialization invariant being fuzzed
