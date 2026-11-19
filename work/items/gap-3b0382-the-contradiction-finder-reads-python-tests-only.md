+++
id = "gap-3b0382"
kind = "gap"
title = "The contradiction finder reads Python tests only, so a Rust or TypeScript test asserting one call to two values (assert_eq!(round_to_dollars(150), 2) and 3) passes the spec gate's HF6 and the gaming screen"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "M"
subsystem = ["roko-gate/attempt_diff"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-ba2841 (worker, 2026-10-27)"
discovered_from = "gap-ba2841"
anchors = ["crates/roko-gate/src/attempt_diff/gaming.rs::LANGUAGES", "crates/roko-gate/src/attempt_diff/gaming/python.rs::expectations"]
links = { depends_on = [], blocks = [], related = ["gap-ba2841", "find-2ba86d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn test_contradictions_read_rust_and_typescript_tests' crates/roko-gate/src && cargo test -p roko-gate --lib test_contradictions_read_rust_and_typescript_tests"
+++

## Problem

The finder of contradictory tests (`roko_gate::attempt_diff::test_contradictions`) reads Python only. The spec
gate's HF6 (gap-ba2841) and the gaming screen's special cases (find-2ba86d) both use it, so a pinned Rust or
TypeScript acceptance test, or a fenced `rust`/`ts` block in a task's description, that asserts one call to two
values no one result meets passes the spec gate:

```rust
assert_eq!(round_to_dollars(150), 2);
assert_eq!(round_to_dollars(150), 3);
```

```ts
expect(roundToDollars(150)).toBe(2);
expect(roundToDollars(150)).toBe(3);
```

Prose comparisons (`round_to_dollars(150) == 2`) are language-neutral and are caught; test code in another
language is not.

## Why it matters

Goal `truth`. Most of roko's own plans are Rust, and the golden path has TypeScript tasks
(`crates/roko-cli/tests/fixtures/golden_path/plan/tasks.toml`). A contradictory Rust or TypeScript test is as
impossible as a Python one, and invites the same gaming before any check can see it.

## Where

- `crates/roko-gate/src/attempt_diff/gaming.rs`: `LANGUAGES` and the `Language` trait (`covers`, `names`,
  `expectations`): a language plugs in here.
- `crates/roko-gate/src/attempt_diff/gaming/python.rs`: the Python reader's test assertions (`expectations`,
  `assertion`, `call`, `constant_arguments`, `object_names`), the model for another reader.

## Current state

Python only, as find-2ba86d chose ("Python first, pluggable by language"). gap-ba2841 made the finder public and
gave its assertions a scope and a count of the state-changing statements before them, which another language's
reader must fill too.

## Plan

1. A Rust reader of test assertions: `assert_eq!(call, literal)`, `assert_eq!(literal, call)`, `assert!(call)`,
   `assert!(!call)`, with `#[test]` functions as scopes; `let` bindings and other statements count as state
   changes, as Python's do.
2. A TypeScript/JavaScript reader: `expect(call).toBe(literal)` / `toEqual` / `toBeTruthy` / `toBeFalsy`,
   `assert.strictEqual(call, literal)`, with `it`/`test` callbacks as scopes.
3. Only the test assertions are needed for HF6; the gaming screen's product-code findings in those languages are
   separate work.

## Done when

`test_contradictions` finds `round_to_dollars(150)` asserted to both 2 and 3 in Rust and TypeScript test code,
and leaves consistent tests alone; the `[[verify]]` passes.

## Notes

Keep the rules of `test_contradictions`: constant arguments only, no method of an object the test makes, and
nothing between the two assertions that can change state.
