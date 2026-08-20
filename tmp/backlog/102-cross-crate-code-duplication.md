# 102 — Cross-Crate Utility Duplication (parse_duration, constant_time_eq, truncate_to_budget)

**Priority**: P2 — maintainability; each duplicate has diverged in behavior, making bug fixes error-prone
**Size**: M (2-3 days)
**Crates**: `crates/roko-core/` (consolidation target), `crates/roko-runtime/`, `crates/roko-cli/`, `crates/roko-compose/`, `crates/roko-serve/`, `crates/roko-plugin/`, `apps/agent-relay/`, `apps/mirage-rs/`
**Depends on**: None

---

## Background

Several utility functions are independently implemented multiple times across crates. Each
copy has slightly different behavior: different supported units, different return types, and
different error handling. This means that fixing a bug or extending behavior in one copy
does not fix the others, and callers cannot rely on consistent semantics.

The `parse_duration` family is the most severe case: six independent implementations exist
across five crates and one app, with incompatible signatures and different supported unit
sets. For example, the `knowledge.rs` version supports the `d` (day) unit but silently
returns `None` for `ms`. The TUI versions support `ms` but silently return 0 for `d`. The
`roko-core` version supports all units and is the most correct.

The `constant_time_eq` function is independently implemented three times in crates (a fourth
instance in `apps/agent-relay/` has the same body). Only the version in `webhooks.rs` uses
`core::hint::black_box` to prevent the compiler from optimizing out the constant-time XOR
loop. The `team.rs` version operates on `&str` instead of `&[u8]`, giving slightly different
semantics.

Within `roko-compose`, two functions both named `truncate_to_budget` exist in different files
with the same name but different semantics: one is character-based (raw byte count), the
other is token-budget-aware (converts a token budget to an approximate character limit at 3.5
chars/token). Callers must know which file they imported from.

## Current State

### 1. `parse_duration` — 6 independent implementations in the main tree

| File | Line | Function name | Return type | Supported units |
|---|---|---|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/serve.rs` | 36 | `fn parse_duration(value: &str)` | `Result<Duration, String>` | ms, s, m, h, d |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/builtin_lenses_derived.rs` | 1073 | `fn parse_duration_ms(value: &str)` | `Result<u64>` (milliseconds) | ms, s, m, h, d |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/builtin_lenses_health.rs` | 853 | `fn parse_duration_ms(value: &str)` | `Result<u64>` (milliseconds) | ms, s, m, h (no `d`) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/knowledge.rs` | 334 | `fn parse_duration_to_ms(s: &str)` | `Option<i64>` (milliseconds) | d, h, m, s (no `ms`) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` | 3682 | `fn parse_duration_to_secs(duration: &str)` | `f64` (seconds, 0.0 on error) | ms, s, m (no `h`, no `d`) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` | 1403 | `fn parse_duration_secs(duration: &str)` | `Option<f64>` (seconds) | ms, s, m, h (no `d`) |

The `apps/mirage-rs/src/scenario.rs:203` version supports ms, s, m but not h or d, and
returns `Duration` directly. It is in a separate app crate and can be updated separately
after the core consolidation.

The canonical `roko-core` version at serve.rs:36 supports all units (ms, s, m, h, d),
returns `Result<Duration, String>`, and handles zero-value rejection and overflow correctly.
It is the best candidate for promotion.

### 2. `constant_time_eq` — 3 implementations in crates, 1 in app

| File | Line | Operand type | Uses `black_box` |
|---|---|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/webhooks.rs` | 744 | `&[u8]` | Yes |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/team.rs` | 602 | `&str` (compares as bytes internally) | No |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/registry.rs` | 878 | `&[u8]` | No |
| `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/registry.rs` | 620 | `&[u8]` | No |

Only `webhooks.rs` uses `core::hint::black_box(diff) == 0`, which is the correct way to
prevent the compiler from optimizing the loop into a non-constant-time comparison.

### 3. `truncate_to_budget` — 2 functions with the same name in `roko-compose`

| File | Line | Budget unit | Behavior |
|---|---|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/compaction.rs` | 329 | Tokens (converts at 3.5 chars/token) | Appends `"..."` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/prompts.rs` | 16 | Characters (raw byte count) | Appends truncation note |

Both are private (`fn`, not `pub fn`) so there is no cross-file name collision at the import
level, but the semantic confusion is real: `PLAN_BUDGET` and `SUPPORT_BUDGET` are defined as
`usize` constants in `prompts.rs` (as character counts), while `compaction.rs` treats its
argument as a token count.

## Implementation Plan

### Step 1: Create a canonical `parse_duration` in `roko-core`

The existing function at `crates/roko-core/src/config/serve.rs:36` is private to that module.
There is no existing `utils` module in `roko-core`. Add a new file:

**Create** `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/duration.rs`:

```rust
//! Canonical duration string parser for the roko workspace.
//!
//! Supported formats: `<n>ms`, `<n>s`, `<n>m`, `<n>h`, `<n>d`
//! where `<n>` is a positive (non-zero) unsigned integer.

use std::time::Duration;

/// Parse a human-readable duration string into a [`Duration`].
///
/// # Supported units
/// | Suffix | Meaning |
/// |--------|---------|
/// | `ms`   | milliseconds |
/// | `s`    | seconds |
/// | `m`    | minutes |
/// | `h`    | hours |
/// | `d`    | days |
///
/// Returns `Err` if the string is empty, the unit is unrecognized, the
/// numeric part overflows a `u64`, or the value is zero.
///
/// # Examples
/// ```
/// use roko_core::duration::parse_duration;
/// assert_eq!(parse_duration("30s").unwrap(), std::time::Duration::from_secs(30));
/// assert_eq!(parse_duration("7d").unwrap(), std::time::Duration::from_secs(7 * 86400));
/// assert!(parse_duration("0s").is_err());
/// assert!(parse_duration("10x").is_err());
/// ```
pub fn parse_duration(value: &str) -> Result<Duration, String> {
    let value = value.trim();
    let split = value
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| "duration requires a unit suffix: ms, s, m, h, or d".to_string())?;
    let (amount_str, unit) = value.split_at(split);
    if amount_str.is_empty() {
        return Err("duration must start with a positive integer".to_string());
    }
    let amount = amount_str
        .parse::<u64>()
        .map_err(|e| format!("duration numeric part is invalid: {e}"))?;
    if amount == 0 {
        return Err("duration must be greater than zero".to_string());
    }
    let multiplier_ms: u64 = match unit {
        "ms" => 1,
        "s"  => 1_000,
        "m"  => 60_000,
        "h"  => 3_600_000,
        "d"  => 86_400_000,
        other => return Err(format!("unknown duration unit '{other}': use ms, s, m, h, or d")),
    };
    amount
        .checked_mul(multiplier_ms)
        .map(Duration::from_millis)
        .ok_or_else(|| "duration value overflows a u64 millisecond count".to_string())
}

/// Parse a duration string and return the value in milliseconds.
///
/// Convenience wrapper around [`parse_duration`].
pub fn parse_duration_ms(value: &str) -> Result<u64, String> {
    parse_duration(value).map(|d| d.as_millis() as u64)
}
```

**Add** to `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs` (after the
existing `pub mod datum;` line, in alphabetical order):

```rust
/// Canonical duration string parser (supports ms, s, m, h, d).
pub mod duration;
```

### Step 2: Update call sites to use the canonical version

For each call site, the approach is: add `use roko_core::duration::parse_duration;` (or
`parse_duration_ms`) and delete the local function.

**`crates/roko-core/src/config/serve.rs`** — The existing private `parse_duration` at line
36 is already the canonical implementation. After adding `roko_core::duration`, this file
can use `crate::duration::parse_duration` and delete its local copy, or simply keep it as
the source that was moved. Either approach works.

**`crates/roko-runtime/src/builtin_lenses_derived.rs`** — Delete the `parse_duration_ms`
function at line 1073. Add `use roko_core::duration::parse_duration_ms;` at the top of the
file. Verify the `Result<u64>` return type matches (it does: the canonical version returns
`Result<u64, String>` and the crate uses `anyhow::Result`; add `.map_err(|e| anyhow::anyhow!(e))`
at the call site at line 1070 if needed).

**`crates/roko-runtime/src/builtin_lenses_health.rs`** — Delete `parse_duration_ms` at line
853. Add `use roko_core::duration::parse_duration_ms;`. This version was missing the `d`
unit; the canonical version adds it.

**`crates/roko-cli/src/commands/knowledge.rs`** — Delete `parse_duration_to_ms` at line 334.
Replace the call at line 267 with:
```rust
let max_age_ms = roko_core::duration::parse_duration_ms(older_than)
    .ok()
    .map(|ms| ms as i64);
```
This version previously returned `Option<i64>` and supported `d, h, m, s` (no `ms` suffix).
The canonical version adds `ms` support, which is additive.

**`crates/roko-cli/src/tui/state.rs`** — Delete `parse_duration_to_secs` at line 3682.
Replace the call at line 3655 with:
```rust
let elapsed_secs = roko_core::duration::parse_duration(t.duration.as_str())
    .map(|d| d.as_secs_f64())
    .unwrap_or(0.0);
```

**`crates/roko-cli/src/tui/views/plans_view.rs`** — Delete `parse_duration_secs` at line
1403. Replace the call at line 1214 with:
```rust
if let Some(current_secs) = roko_core::duration::parse_duration(&current_task.duration)
    .map(|d| d.as_secs_f64())
    .ok()
```

### Step 3: Add canonical `constant_time_eq` to `roko-core`

**Add** to `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/hash.rs` (after the
existing `ContentHash` struct and impls, near the end of the file):

```rust
/// Compare two byte slices in constant time to prevent timing side-channels.
///
/// Returns `true` if and only if `a == b`. The comparison takes the same
/// number of operations regardless of where the first difference occurs.
/// Uses `core::hint::black_box` to prevent the compiler from optimizing
/// the XOR loop into an early-exit comparison.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (lhs, rhs) in a.iter().zip(b.iter()) {
        diff |= lhs ^ rhs;
    }
    core::hint::black_box(diff) == 0
}
```

`roko-core/src/hash.rs` is already a public module (`pub mod hash;` in lib.rs at line 115,
with `pub use hash::ContentHash;` at line 285). Add `pub use hash::constant_time_eq;` to
lib.rs so callers can use `roko_core::constant_time_eq`.

**Update call sites:**

`/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/webhooks.rs` — Delete
local `constant_time_eq` at line 744. Add `use roko_core::constant_time_eq;`.

`/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/team.rs` — Delete local
`constant_time_eq` at line 602. The local version compares `&str` arguments. Replace the
call site at line 317 to pass bytes:

Before:
```rust
!invitation.consumed && constant_time_eq(&invitation.invite_token_hash, &supplied_hash)
```
After:
```rust
!invitation.consumed && roko_core::constant_time_eq(
    invitation.invite_token_hash.as_bytes(),
    supplied_hash.as_bytes(),
)
```

`/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/registry.rs` — Delete local
`constant_time_eq` at line 878. Add `use roko_core::constant_time_eq;`. The call site at
line 176 already uses `&[u8]` slices (via `.as_bytes()`), so no call site changes needed.

`/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/registry.rs` — Delete local
`constant_time_eq` at line 620. Add `use roko_core::constant_time_eq;`. The `roko-core`
crate must be a dependency of `agent-relay`; check `apps/agent-relay/Cargo.toml` and add
it if missing.

### Step 4: Disambiguate `truncate_to_budget` in `roko-compose`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/prompts.rs`:

Rename the function at line 16 from `truncate_to_budget` to `truncate_to_char_budget`.
Update all calls in the same file (lines 68, 73, 117, 120, 171, 222, 225, 249, 261, 279,
284, 290, 312, 322, 348, 361, 420, 423, 497, and others) to use the new name.

The function in `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/compaction.rs`
at line 329 keeps its name `truncate_to_budget` since it is the token-aware version and is
the more general one.

Update the doc comment on the prompts.rs version to clarify:
```rust
/// Truncate `content` to at most `budget` *characters*, appending a truncation note.
/// For token-budget truncation, see [`compaction::truncate_to_budget`].
fn truncate_to_char_budget(content: &str, budget: usize) -> String {
```

## Acceptance Criteria

1. A single `roko_core::duration::parse_duration` function exists that supports all units:
   ms, s, m, h, d.
2. All six local `parse_duration*` implementations in main-tree crates are deleted and
   replaced with calls to the canonical version.
3. A single `roko_core::constant_time_eq` function exists that uses `core::hint::black_box`.
4. All three local `constant_time_eq` implementations in main-tree crates are deleted and
   replaced with calls to the canonical version; the `team.rs` call site passes `.as_bytes()`.
5. The `agent-relay` app's local `constant_time_eq` at line 620 is replaced with the
   canonical version.
6. The prompts.rs `truncate_to_budget` is renamed to `truncate_to_char_budget` with all
   call sites in that file updated.
7. `cargo test --workspace` passes with no new failures.
8. A new unit test in `roko-core/src/duration.rs` validates all units (ms, s, m, h, d),
   zero rejection, overflow rejection, and unknown-unit rejection.

## Verification Checklist

- [ ] `grep -rn "fn parse_duration" crates/ apps/ --include="*.rs" | grep -v "target/"` shows only the canonical version in `roko-core/src/duration.rs` (plus the serve.rs internal one which can remain private until replaced)
- [ ] `grep -rn "fn constant_time_eq" crates/ apps/ --include="*.rs" | grep -v "target/"` shows only the canonical version in `roko-core/src/hash.rs`
- [ ] `grep -rn "fn truncate_to_budget" crates/ --include="*.rs" | grep -v "target/"` shows the compaction.rs version only; prompts.rs shows `truncate_to_char_budget`
- [ ] `cargo test -p roko-core -- duration` passes the new duration unit tests
- [ ] `cargo test -p roko-runtime` passes (builtin_lenses_derived and builtin_lenses_health updated)
- [ ] `cargo test -p roko-cli` passes (knowledge.rs, tui/state.rs, tui/views/plans_view.rs updated)
- [ ] `cargo test -p roko-serve` passes (webhooks.rs, team.rs updated)
- [ ] `cargo test -p roko-plugin` passes (registry.rs updated)
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` is clean

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/duration.rs` | **Create new file** with canonical `parse_duration` and `parse_duration_ms` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs` | Add `pub mod duration;` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/hash.rs` | Add `constant_time_eq` function |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs` | Add `pub use hash::constant_time_eq;` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/serve.rs` | Remove local `parse_duration` (now superseded by `crate::duration::parse_duration`) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/builtin_lenses_derived.rs` | Delete local `parse_duration_ms`; use `roko_core::duration::parse_duration_ms` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/builtin_lenses_health.rs` | Delete local `parse_duration_ms`; use `roko_core::duration::parse_duration_ms` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/knowledge.rs` | Delete `parse_duration_to_ms`; use `roko_core::duration::parse_duration_ms` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` | Delete `parse_duration_to_secs`; use `roko_core::duration::parse_duration` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` | Delete `parse_duration_secs`; use `roko_core::duration::parse_duration` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/webhooks.rs` | Delete local `constant_time_eq`; use `roko_core::constant_time_eq` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/team.rs` | Delete local `constant_time_eq`; call `roko_core::constant_time_eq` with `.as_bytes()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/registry.rs` | Delete local `constant_time_eq`; use `roko_core::constant_time_eq` |
| `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/registry.rs` | Delete local `constant_time_eq`; use `roko_core::constant_time_eq` (add `roko-core` dep if missing) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/prompts.rs` | Rename `truncate_to_budget` to `truncate_to_char_budget`; update all internal call sites |
