# 43 — Clippy Blanket Suppression Removal

**Priority**: P2 — Real warnings including latent bugs are silently discarded for the largest crate
**Size**: L (2-4 days, volume not complexity)
**Crates**: `roko-cli` (`crates/roko-cli/`)
**Depends on**: None (but work in `tmp/backlog/20-event-loop-decomposition.md` and `tmp/backlog/22-chat-inline-decomposition.md` will reduce the total warning count once done)

---

## Background

Clippy is Rust's official linter. When `cargo clippy -- -D warnings` runs, it treats any lint warning as a compile error, ensuring the codebase stays clean. Most crates in this workspace follow this rule strictly. The single exception is `roko-cli`, which is the largest crate in the workspace and contains the runner, TUI, interactive chat loop, plan dispatch, the gate pipeline adapter, and the merge queue — all real production paths.

At the top of `crates/roko-cli/src/lib.rs`, two blanket suppressions disable lint checking entirely for the whole crate:

1. A `#![allow(dead_code, unused_imports, unused_variables)]` at line 6 — suppresses warnings that often signal wired-but-unreachable code paths.
2. A `#![cfg_attr(clippy, allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, missing_docs))]` block at lines 14-23 — suppresses every clippy lint category.

The practical effect: `cargo clippy --workspace --no-deps -- -D warnings` passes today, but only because `roko-cli` is excluded from lint enforcement. The suppressions were added during rapid development as a shortcut to keep CI green and were never removed.

Real warnings — including ones that signal latent bugs such as unused `Result` values, needless `.clone()` calls on types that implement `Copy`, or unreachable match arms — are silently discarded. The cleanup is not architecturally complex; it is volume work that requires methodically fixing or suppressing each warning with a justification comment.

## Current State

1. **`crates/roko-cli/src/lib.rs` line 6:** `#![allow(dead_code, unused_imports, unused_variables)]` — crate-level blanket that predates the `cfg_attr` block.

2. **`crates/roko-cli/src/lib.rs` lines 14-23:** The `cfg_attr` block:
   ```rust
   #![cfg_attr(
       clippy,
       allow(
           clippy::all,
           clippy::pedantic,
           clippy::nursery,
           clippy::restriction,
           missing_docs
       )
   )]
   ```

3. **`crates/roko-cli/src/lib.rs` line 12:** `#![allow(clippy::module_name_repetitions)]` — this one is legitimate and should be kept. Many types like `RunnerEvent`, `RunnerConfig`, `DispatchOutcome` live in modules named `runner` or `dispatch`, making repetition unavoidable.

4. **`crates/roko-cli/src/lib.rs` line 13:** `#![allow(missing_docs)]` — keep at crate level; internal binary crates do not require public doc comments.

5. **`crates/roko-cli/src/runner/event_loop.rs`** is 23,154 lines — the primary lint surface. It will produce the most warnings, predominantly `clippy::too_many_lines`, `clippy::too_many_arguments`, and `clippy::cognitive_complexity`.

6. **`crates/roko-cli/src/chat_inline.rs`** is 5,698 lines — the second largest file.

7. The rest of the workspace is already clean under `cargo clippy --workspace --no-deps -- -D warnings`.

## Implementation Plan

### Step 1: Remove the blanket suppressions and capture the warning list

Edit `crates/roko-cli/src/lib.rs`:
- Delete line 6 (`#![allow(dead_code, unused_imports, unused_variables)]`)
- Delete lines 14-23 (the `cfg_attr` block)
- Keep line 12 (`#![allow(clippy::module_name_repetitions)]`)
- Keep line 13 (`#![allow(missing_docs)]`)

Then run:
```bash
cargo clippy -p roko-cli -- -D warnings 2>&1 | tee /tmp/roko-cli-lints.txt
wc -l /tmp/roko-cli-lints.txt
```

Triage the output by lint category. The expected high-volume categories:
- `clippy::too_many_arguments` — acceptable to allow at function level with a comment
- `clippy::too_many_lines` — acceptable to allow at function level (pending `backlog/20`)
- `clippy::cognitive_complexity` — acceptable to allow at function level (pending `backlog/20`)
- `clippy::missing_errors_doc` / `clippy::missing_panics_doc` — fix by adding doc section or move to `#![allow(missing_docs)]` scope
- `clippy::must_use_candidate` — fix by adding `#[must_use]` or changing callers
- Unused results from `let _ =` — fix by propagating with `?` or explicitly ignoring
- Needless clones on `Copy` types — fix by removing `.clone()`
- Dead code — investigate each case; remove or re-wire

### Step 2: Fix the actionable warnings

Work through categories from lowest risk to highest. For each warning either:
- **Fix it** (preferred): remove the `.clone()`, propagate the `Result`, add the attribute
- **Suppress it at the item level** (acceptable): add `#[allow(clippy::foo)]` directly on the function or struct with a one-line comment explaining why the lint is incorrect in that context

Example of an acceptable item-level suppression:
```rust
// This function orchestrates the full runner DAG; decomposition is tracked in
// tmp/backlog/20-event-loop-decomposition.md.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity, clippy::too_many_arguments)]
pub async fn run_plan_event_loop(...) { ... }
```

Never add suppressions at module level (`#![allow(...)]` inside a `mod {}`) or at file level (re-adding to `lib.rs`). Every remaining `#[allow]` must be on the specific item it covers.

### Step 3: Restore the `dead_code` / `unused` coverage

The `#![allow(dead_code)]` at line 6 masks wired-but-unreachable paths. After removing it, each `dead_code` warning should be investigated:
- If the code is genuinely unused: remove it or open a separate backlog item
- If the code is reachable but the compiler cannot see it (e.g., used only via trait object dispatch): add `#[allow(dead_code)]` at the item level with a comment
- If the code is temporarily unused pending wiring: add `#[allow(dead_code)]` with a `// TODO: wire in <backlog-item>` comment

### Step 4: Verify CI parity

After all changes:
```bash
cargo clippy --workspace --no-deps -- -D warnings
```
Must pass with zero errors. No blanket crate-level `#![allow(clippy::...)]` blocks may remain in `lib.rs` other than `module_name_repetitions` and `missing_docs`.

## Acceptance Criteria

1. `crates/roko-cli/src/lib.rs` contains no `cfg_attr(clippy, allow(clippy::all, …))` block.
2. `crates/roko-cli/src/lib.rs` contains no `#![allow(dead_code)]`, `#![allow(unused_imports)]`, or `#![allow(unused_variables)]` at crate level.
3. `cargo clippy --workspace --no-deps -- -D warnings` passes with zero errors.
4. Every remaining `#[allow(clippy::…)]` in `crates/roko-cli/src/` has a one-line comment explaining why the suppression is justified.
5. `cargo test -p roko-cli` passes with zero failures after each batch of fixes.

## Verification Checklist

- [ ] `grep 'cfg_attr(clippy' crates/roko-cli/src/lib.rs` returns no output
- [ ] `grep '#!\[allow(dead_code' crates/roko-cli/src/lib.rs` returns no output
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes with zero errors
- [ ] Every `#[allow(clippy::` in `crates/roko-cli/src/` is on a specific item (not module/file level) and has a comment
- [ ] `cargo test -p roko-cli` passes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/lib.rs` | Remove line 6 (`#![allow(dead_code, ...)]`) and lines 14-23 (the `cfg_attr` block); keep `module_name_repetitions` and `missing_docs` allows |
| `crates/roko-cli/src/runner/event_loop.rs` | Add item-level `#[allow(...)]` on large functions; fix actionable lints |
| `crates/roko-cli/src/chat_inline.rs` | Fix actionable lints; item-level allows for functions that cannot be decomposed yet |
| Various `crates/roko-cli/src/**/*.rs` | Fix unused results, needless clones, dead code, and other actionable lints per category |
