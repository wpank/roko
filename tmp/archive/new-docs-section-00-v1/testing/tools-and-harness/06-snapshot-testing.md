# Snapshot Testing

> Golden file infrastructure for regression testing: the `assert_golden!` macro, update workflow, and diff output.

**Status**: Shipping
**Crate**: `roko-test`
**Depends on**: [01-test-harness.md](01-test-harness.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Snapshot testing captures the exact serialized output of a computation and stores it as a golden file. On subsequent runs, the current output is compared to the golden file. A diff is a test failure. This is the infrastructure behind [regression tests](../tiers/04-regression-tests.md).

---

## The `assert_golden!` Macro

```rust
/// Compare `value` against the golden file at `path`.
/// In normal mode: fails with a diff if they differ.
/// In UPDATE_GOLDEN=1 mode: writes `value` to `path` and passes.
macro_rules! assert_golden {
    ($path:expr, $value:expr) => { ... };
    ($path:expr, $value:expr, $msg:expr) => { ... };
}
```

Usage:
```rust
#[test]
fn compile_gate_verdict_golden() {
    let input = GateInput::fixture_clean_rust();
    let verdict = CompileGate::default().evaluate(&input, 1.0).unwrap();

    assert_golden!(
        "tests/golden/gate/compile_gate/pass_clean_rust.verdict.json",
        &verdict,
        "CompileGate verdict for clean Rust changed"
    );
}
```

---

## Serialization Format

Golden files use `serde_json::to_string_pretty` with keys sorted alphabetically. This ensures:
- Deterministic output across platforms.
- Meaningful diffs (changes appear on specific lines, not as key reorderings).
- Human-readable format for code review.

For types that do not implement `Serialize`, the golden file stores the `Debug` output (with a `# debug-format` comment at the top to signal this).

---

## Diff Output

On failure, `assert_golden!` prints a unified diff:

```
thread 'compile_gate_verdict_golden' panicked: CompileGate verdict for clean Rust changed
--- tests/golden/gate/compile_gate/pass_clean_rust.verdict.json
+++ current output
@@ -2,6 +2,6 @@
   "gate": "CompileGate",
-  "outcome": "Pass",
+  "outcome": "Fail",
   "details": {
```

The diff is printed to stderr so it appears in CI logs.

---

## Update Workflow

When a behaviour change is intentional:

```bash
# Update a single golden file
UPDATE_GOLDEN=1 cargo test -p roko-gate -- compile_gate_verdict_golden

# Update all golden files in a crate
UPDATE_GOLDEN=1 cargo test -p roko-gate

# Update all golden files in the workspace
UPDATE_GOLDEN=1 cargo test --workspace
```

After updating:
1. `git diff tests/golden/` to review all changes.
2. Verify each change is intentional.
3. Add a `# golden-update: <reason>` note to the commit message.

---

## Golden File Naming

```
tests/golden/<crate-name>/<module>/<test-name>.<format>
```

Format extensions:
- `.json` — JSON output (most common)
- `.txt` — plain text output
- `.debug` — Rust Debug output

Examples:
```
tests/golden/gate/compile_gate/pass_clean_rust.verdict.json
tests/golden/serialization/engram/minimal.json
tests/golden/serialization/score/all_axes.json
```

---

## Invariants

- Golden files are committed to the repository.
- `UPDATE_GOLDEN=1` is never set in CI without explicit override.
- A PR that changes golden files must include a justification.

---

## See also

- [../tiers/04-regression-tests.md](../tiers/04-regression-tests.md) — regression tests use this infrastructure
