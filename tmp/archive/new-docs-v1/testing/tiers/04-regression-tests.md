# Regression Tests

> Golden-file and verdict-replay tests that prevent reversion of known-correct behaviours.

**Status**: Shipping
**Crate**: all shipping crates with stable output formats
**Depends on**: [02-integration-tests.md](02-integration-tests.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Regression tests capture the exact output (verdict, serialized form, file content) produced by a known-good run and compare it to the current output on every CI pass. A diff is a test failure. Golden files are committed to the repo; a PR that legitimately changes output must update the golden file and justify the change in the commit message.

---

## Two Categories

### 1. Gate Verdict Regression

For each of the 11 gates plus the 7-rung pipeline, a set of canonical inputs is run and the resulting `Verdict` is compared to a stored expectation.

```
tests/golden/gate/compile_gate/
  pass_clean_rust.verdict.json
  fail_syntax_error.verdict.json
  fail_type_error.verdict.json

tests/golden/gate/pipeline/
  full_seven_rungs_pass.verdict.json
  short_circuit_at_rung3.verdict.json
```

If a gate changes its verdict logic and a golden file diff appears, the PR must explain:
- Was this a bug fix (old verdict was wrong)?
- Was this a behaviour change (deliberate)?
- What is the new acceptance criterion?

### 2. Serialization Golden Tests

For Engram, Score, ContentHash, and GateInput, the serialized form (JSON/CBOR) is golden-tested to prevent accidental format changes that would break stored data.

```
tests/golden/serialization/
  engram_minimal.json
  engram_with_all_fields.json
  score_all_axes.json
  content_hash_known_bytes.json
```

Deserialization from the golden file is also tested (the round-trip invariant), but it is the serialization golden test that catches format regressions.

---

## Golden File Format

Golden files use JSON with deterministic key ordering (via `serde_json::to_string_pretty` with sorted keys). Comments are not allowed in JSON; an adjacent `<filename>.md` may document the intent.

Example `pass_clean_rust.verdict.json`:
```json
{
  "gate": "CompileGate",
  "outcome": "Pass",
  "details": {
    "compiler_version": "1.87.0",
    "warnings": 0,
    "errors": 0
  },
  "threshold_used": 1.0,
  "rung": 1
}
```

---

## Writing a Regression Test

```rust
#[test]
fn compile_gate_passes_clean_rust() {
    let input = GateInput::from_fixture("fixtures/clean_rust_project");
    let gate = CompileGate::default();
    let verdict = gate.evaluate(&input).unwrap();

    // Compare to golden file
    assert_golden!(
        "tests/golden/gate/compile_gate/pass_clean_rust.verdict.json",
        &verdict,
        "CompileGate verdict for clean Rust changed unexpectedly"
    );
}
```

The `assert_golden!` macro:
1. Serializes `&verdict` to JSON.
2. Reads the golden file.
3. Compares the two; fails with a diff if they differ.
4. In `UPDATE_GOLDEN=1` mode, writes the current output to the golden file instead of failing.

---

## Updating Golden Files

When a behaviour change is intentional:

```bash
UPDATE_GOLDEN=1 cargo test -p roko-gate -- compile_gate_passes_clean_rust
```

Review the diff, commit the updated file, and add a `# golden-update: <reason>` comment in the commit message.

---

## Diff Output

On failure, the test prints a unified diff:

```diff
--- tests/golden/gate/compile_gate/pass_clean_rust.verdict.json
+++ current output
@@ -3,4 +3,4 @@
   "details": {
-    "warnings": 0,
+    "warnings": 1,
     "errors": 0,
```

---

## Property-Test Regressions vs. Golden Files

| | Property regression (`proptest-regressions/`) | Golden file (`tests/golden/`) |
|---|---|---|
| Created by | proptest on first failure | Developer or `UPDATE_GOLDEN=1` |
| Updated by | Never (delete and re-shrink) | `UPDATE_GOLDEN=1` |
| Format | Proptest seed + input | Canonical JSON |
| Purpose | Prevent re-introduction of specific bugs | Prevent silent output changes |

---

## Invariants

- Golden files are committed with every PR that intentionally changes tested output.
- `UPDATE_GOLDEN=1` is never run in CI without a maintainer explicit action.
- Regression test failure without a corresponding code change is a P0 flakiness bug.

---

## Open Questions

- Should golden files for serialization use a stable CBOR encoding rather than JSON for binary efficiency?
- Should gate verdict golden files include timing information or only structural verdict fields?

## See also

- [../tools-and-harness/06-snapshot-testing.md](../tools-and-harness/06-snapshot-testing.md) — snapshot infrastructure
- [../by-subsystem/subsystem-gate.md](../by-subsystem/subsystem-gate.md) — per-gate test coverage
- [03-property-tests.md](03-property-tests.md) — complement for invariant testing
