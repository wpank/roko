# Clippy Blanket Suppression Removal

**Priority**: P2
**Size**: L (2–4 days, volume not complexity)

---

## Problem

`crates/roko-cli/src/lib.rs` lines 14–23 carry a blanket `#![cfg_attr(clippy, …)]`
that suppresses every clippy lint category — `all`, `pedantic`, `nursery`, and
`restriction` — for essentially the entire `roko-cli` crate:

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

`roko-cli` is the largest crate in the workspace. `event_loop.rs` alone is the largest
source file (~23K lines). The `chat_inline.rs` file is the second largest (~5,700 lines).
The crate contains the runner, the TUI, the interactive chat loop, plan dispatch, the
gate pipeline adapter, and the merge queue — all real production paths.

Because the blanket suppression is on, `cargo clippy --workspace --no-deps -- -D warnings`
passes today, but only because `roko-cli` is excluded from lint enforcement. The rest of
the workspace gets strict lint checking; `roko-cli` does not. Real warnings — including
ones that signal latent bugs (unused results, needless clones, unreachable patterns) —
are silently discarded.

The suppression was added during rapid development as a shortcut to keep CI green. It
was never removed.

There is also a separate `#![allow(dead_code, unused_imports, unused_variables)]` on
line 6 and `#![allow(clippy::module_name_repetitions)]` on line 12. Line 6 is a second
blanket that predates the `cfg_attr` block.

### What already exists

| Component | Location | Status |
|---|---|---|
| Blanket `cfg_attr` suppression | `crates/roko-cli/src/lib.rs:14–23` | EXISTS (to be removed) |
| `dead_code` / `unused_*` suppression | `crates/roko-cli/src/lib.rs:6` | EXISTS (to be tightened) |
| `clippy::module_name_repetitions` allow | `crates/roko-cli/src/lib.rs:12` | EXISTS (keep or inline) |
| Rest of workspace clippy gate | CI + `cargo clippy --workspace --no-deps -- -D warnings` | EXISTS (enforced) |

### What is missing

1. **Removal of the blanket suppression.** Once removed, `cargo clippy -p roko-cli --
   -D warnings` will emit (estimated) 50–200 warnings — many of them real signal.

2. **Systematic fix-or-suppress pass.** Each resulting warning must be either fixed
   (preferred) or suppressed with a targeted `#[allow(clippy::foo)]` on the specific
   item, with a one-line comment explaining why the lint is wrong in that case.

3. **Removal of the `dead_code` / `unused_*` crate-level allow.** Unused code in the
   runner is a sign of wired-but-unreachable paths. Each case should be addressed at
   the item level, not suppressed globally.

---

## Proposed approach

### Step 1: remove the blanket, capture the damage

```bash
# Remove the cfg_attr block and the dead_code line from lib.rs, then:
cargo clippy -p roko-cli -- -D warnings 2>&1 | tee /tmp/roko-cli-lints.txt
wc -l /tmp/roko-cli-lints.txt
```

Triage the output by lint category. Expect heavy volume in:
- `clippy::too_many_arguments` — acceptable to allow at function level
- `clippy::too_many_lines` — acceptable to allow at function level (see backlog/20)
- `clippy::missing_errors_doc` / `missing_panics_doc` — fix or allow with comment
- `clippy::must_use_candidate` — fix by adding `#[must_use]` or `let _ =`
- `clippy::cognitive_complexity` — allow at function level pending decomp (backlog/20)
- `clippy::module_name_repetitions` — keep the existing crate-level allow

### Step 2: fix the actionable lints

Work through categories from easiest to hardest:
- Unused results (`let _ =` or propagate with `?`)
- Needless clones (remove where `.clone()` is unnecessary)
- Unreachable patterns (remove dead match arms)
- Missing `#[must_use]` annotations

### Step 3: add targeted `#[allow]` for acceptable suppressions

For lints that are valid but acceptable in context (e.g., a 500-line orchestration
function that cannot be decomposed yet), add the suppression at the item level:

```rust
// This function is the primary runner event loop and cannot be decomposed
// without the work tracked in backlog/20-event-loop-decomposition.md.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub async fn run_plan(...) { ... }
```

Never use a module-level or crate-level allow except for lints that genuinely apply
everywhere (e.g., `clippy::module_name_repetitions` is already handled correctly).

### Step 4: restore CI parity

After the pass completes, `cargo clippy --workspace --no-deps -- -D warnings` must pass
with no blanket suppressions in `roko-cli/src/lib.rs`.

---

## Acceptance criteria

1. `crates/roko-cli/src/lib.rs` contains no `cfg_attr(clippy, allow(clippy::all, …))`
   block.
2. `crates/roko-cli/src/lib.rs` contains no crate-level `#![allow(dead_code)]` or
   `#![allow(unused_imports)]` or `#![allow(unused_variables)]`.
3. `cargo clippy --workspace --no-deps -- -D warnings` passes with zero errors.
4. Every remaining `#[allow(clippy::…)]` in `crates/roko-cli/src/` has a one-line
   comment explaining why the suppression is justified.
5. `cargo test -p roko-cli` passes with zero failures after each batch of fixes.

---

## References

- `crates/roko-cli/src/lib.rs` — lines 6, 12, 14–23 contain the suppressions
- `crates/roko-cli/src/runner/event_loop.rs` — ~23K lines, primary lint surface
- `crates/roko-cli/src/chat_inline.rs` — ~5,700 lines, second largest
- `tmp/backlog/20-event-loop-decomposition.md` — decomp work that will reduce the
  surface area of cognitive-complexity and too-many-lines lints
- `tmp/backlog/22-chat-inline-decomposition.md` — same for the chat module
