# 147 Raw eprintln! Calls Across Command Files

## Problem

The CLI has a well-designed `output_format.rs` module with ANSI colors, clack-style
primitives (intro, step, bar, success, error), and spinner support. But only `run.rs`
uses it. All other command files use raw `eprintln!` for user-facing output, resulting in:

- No colors or formatting in most commands
- Log noise (WARN/INFO from tracing) mixed with user output
- No spinners during long operations
- `--quiet` flag not respected by prd/research/develop commands

## Scope

### Files using raw eprintln! (partial list):

| File | Count | Examples |
|------|-------|---------|
| `commands/do_cmd.rs` | ~25 | Step progress, timing, errors |
| `commands/prd.rs` | ~30 | Draft progress, validation, listing |
| `commands/develop.rs` | ~15 | Plan generation, approval prompts |
| `commands/plan.rs` | ~20 | Run progress, task status |
| `research.rs` | ~15 | Research progress, agent output |
| `orchestrate.rs` | ~20 | Agent dispatch, gate results |
| `commands/knowledge.rs` | ~10 | Query results, stats |
| `commands/config.rs` | ~12 | Config display, validation |
| **Total** | ~147 | |

### What `output_format.rs` provides but isn't used:

```rust
pub fn intro(title: &str) { /* ◆ title with color */ }
pub fn step(msg: &str) { /* │ step indicator */ }
pub fn success(msg: &str) { /* ✔ green success */ }
pub fn error(msg: &str) { /* ✖ red error */ }
pub fn bar() { /* │ continuation bar */ }
pub fn spinner(msg: &str) -> Spinner { /* ⣷ animated spinner */ }
```

### What `inline/symbols.rs` provides:

```rust
pub const CHECK: &str = "✔";
pub const CROSS: &str = "✖";
pub const WARNING: &str = "⚠";
pub const ARROW: &str = "▸";
pub const SPINNER_FRAMES: &[&str] = &["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"];
```

## Fix

### Approach: Create a `CliOutput` wrapper (~30 min)

**File:** `crates/roko-cli/src/cli_output.rs` (new)

```rust
use crate::output_format;

pub struct CliOutput {
    quiet: bool,
    format: OutputFormat,  // text, json, minimal
}

impl CliOutput {
    pub fn step(&self, msg: &str) {
        if !self.quiet { output_format::step(msg); }
    }
    pub fn success(&self, msg: &str) {
        if !self.quiet { output_format::success(msg); }
    }
    pub fn error(&self, msg: &str) {
        output_format::error(msg);  // always show errors
    }
    pub fn spinner(&self, msg: &str) -> Option<Spinner> {
        if self.quiet { None } else { Some(output_format::spinner(msg)) }
    }
    pub fn timing(&self, label: &str, duration: Duration) {
        if !self.quiet {
            output_format::step(&format!("{label}: {:.1}s", duration.as_secs_f64()));
        }
    }
}
```

Then replace `eprintln!` calls in each command file with `out.step()`, `out.success()`, etc.

### Migration order (by impact):

1. `do_cmd.rs` — most visible to users (the primary command)
2. `prd.rs` — PRD workflow output
3. `plan.rs` — plan execution output
4. `research.rs` — research output
5. `orchestrate.rs` — execution monitoring
6. Remaining files

### Log noise suppression

**File:** `crates/roko-cli/src/main.rs`

Set tracing filter to suppress WARN/INFO from dependencies:
```rust
tracing_subscriber::fmt()
    .with_env_filter("roko=info,warn")  // only roko crate logs
    .init();
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/cli_output.rs` | New: CliOutput wrapper |
| `crates/roko-cli/src/commands/do_cmd.rs` | Replace ~25 eprintln! |
| `crates/roko-cli/src/commands/prd.rs` | Replace ~30 eprintln! |
| `crates/roko-cli/src/commands/plan.rs` | Replace ~20 eprintln! |
| + ~6 more command files | Replace remaining eprintln! |

## Priority

**P1** — This is the most visible UX issue. Every CLI interaction looks unpolished because
the formatting primitives exist but aren't used.
