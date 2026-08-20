# 86 — Gate Compile Rung Silently Passes When Build Tool Is Missing

**Priority**: P0 — correctness (gate returns "pass" for code that was never compiled)
**Size**: XS (30 minutes)
**Crates**: `crates/roko-gate` (`src/compile.rs`)
**Depends on**: None

---

## Background

The `CompileGate` is Rung 1 of the 7-rung gate pipeline: it verifies that agent-generated code actually compiles. When an agent produces Rust code and the gate runs `cargo check`, a pass verdict means "this code compiles cleanly." This gate is the first line of defense against non-compiling changes being accepted into the codebase.

There is a check at the top of `CompileGate::verify` that asks: "is the required build tool available on PATH?" If `cargo` is not in `$PATH` (e.g., on a fresh CI runner, inside a minimal Docker container, or on a machine without Rust installed), the build tool is not found. The current code logs a warning and returns `Verdict::pass`. This is wrong: the code was never compiled, so we do not know whether it compiles. The correct verdict is `Verdict::fail` — fail closed.

The fix is a one-line change: replace `Verdict::pass` with `Verdict::fail` at the point where the build tool is not found.

## Current State

1. File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile.rs`

2. Lines 118-128 contain the build tool availability check:

```rust
if !self.build_system.is_available() {
    let reason = format!(
        "{} not available: '{}' not found on PATH",
        self.build_system.name(),
        self.build_system.program()
    );
    tracing::warn!(gate = %self.name, "{reason}");
    return Verdict::pass(&self.name)               // ← LINE 125: WRONG
        .with_detail(format!("skipped: {reason}"))
        .with_duration(started.elapsed().as_millis() as u64);
}
```

3. The `is_available` function is in `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/payload.rs` at lines 289-300. It walks `$PATH` entries checking for the build system binary. The six build systems are `Cargo` (`cargo`), `Npm` (`npm`), `Go` (`go`), `Python` (`python3`), `Forge` (`forge`), and `Make` (`make`).

4. The `Verdict::pass` function signature in `roko-core` accepts `(&self.name)` and optionally a detail string via `.with_detail()` and duration via `.with_duration()`. The `Verdict::fail` function has the same signature. The only difference is `Verdict::pass` sets `passed: true` and `Verdict::fail` sets `passed: false`.

5. The existing test file for this gate is at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile.rs` lines 230-271. It contains tests for `summarize_errors` and a `cargo_shortcut_names` test. There is currently no test that mocks the PATH to verify the availability check behavior.

## Implementation Plan

### Step 1: Change `Verdict::pass` to `Verdict::fail` on line 125

In `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile.rs`, find the block at lines 118-128 and change line 125 from:

```rust
    return Verdict::pass(&self.name)
        .with_detail(format!("skipped: {reason}"))
```

to:

```rust
    return Verdict::fail(&self.name, format!("build tool not available: {reason}"))
        .with_duration(started.elapsed().as_millis() as u64);
```

Note: check the exact `Verdict::fail` signature in `roko-core` — it may take the detail string as a second argument rather than via `.with_detail()`. Look at how other callers in `compile.rs` use `Verdict::fail` (for example the "spawn failed" case at approximately line 160) to match the calling convention.

Remove the `.with_detail(format!("skipped: {reason}"))` call from the pass path and do not add it separately; the detail is now conveyed through the fail message itself.

### Step 2: Add a test that exercises the "tool not found" path

Add a test in the `mod tests` block at line 231 that temporarily removes the build system binary from the test process's view. The cleanest way to do this without actually uninstalling tools is to set `PATH` to an empty string for the duration of the test.

```rust
#[test]
fn compile_gate_fails_when_cargo_not_on_path() {
    // Override PATH so cargo is not found.
    let original_path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: tests are single-threaded in this module; PATH is restored.
    unsafe {
        std::env::set_var("PATH", "");
    }

    let gate = CompileGate::cargo();
    let available = gate.build_system.is_available();

    unsafe {
        std::env::set_var("PATH", &original_path);
    }

    assert!(
        !available,
        "cargo should not be found when PATH is empty"
    );
    // The gate verdict path itself requires a Signal input and an async
    // runtime, so we verify the is_available() branch outcome here.
    // The integration test in tests/ exercises the full verdict.
}
```

Note: if the module already uses `tokio::test` for async tests, write this one as a synchronous `#[test]` since `is_available` is synchronous.

## Acceptance Criteria

1. Line 125 of `compile.rs` changed from `Verdict::pass` to `Verdict::fail`.
2. The fail detail message includes the build system name and the "not found on PATH" reason.
3. The `.with_detail(format!("skipped: {reason}"))` call is removed (or replaced by the fail message).
4. `cargo test -p roko-gate` passes, including existing `summarize_errors` and `cargo_shortcut_names` tests.
5. A new test confirms `is_available()` returns `false` when `PATH` is cleared.
6. `cargo clippy -p roko-gate -- -D warnings` passes.

## Verification Checklist

- [ ] Read lines 118-128 of `compile.rs` to confirm the exact current code before editing
- [ ] Check how `Verdict::fail` is called elsewhere in `compile.rs` (search for `Verdict::fail` in that file) to match the exact calling convention
- [ ] Make the one-line change (pass → fail)
- [ ] Run `cargo test -p roko-gate`
- [ ] Run `cargo clippy -p roko-gate -- -D warnings`
- [ ] Verify the new test does not permanently clear PATH (restore it before assertions)

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile.rs` | Line 125: change `Verdict::pass` to `Verdict::fail`; remove `.with_detail("skipped: ...")` call; add test for unavailable build tool |
