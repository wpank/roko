# 10: Terminal Output Corruption

## Problem

The CLI progress output uses carriage returns, spinners, emoji, and ANSI escape codes that corrupt the terminal when:
- The command runs for minutes (opus took 6+ minutes across 3 attempts)
- The terminal scrolls during output
- zsh tries to interpret output fragments as commands after completion
- Unicode characters (✓, ⏳, 📋, ⚠️) cause rendering issues in some terminal states

### What Happened

After a 7-minute `roko prd plan` run with opus (3 attempts), the terminal was completely trashed:
- `✓ Agen` fragments repeated hundreds of times
- zsh interpreted output lines as commands: `zsh: command not found: 📋`
- Previous output lines from BEFORE the command were re-rendered
- The prompt became unusable

### Root Cause

1. **Carriage return abuse**: Progress spinners use `\r` to overwrite lines, but when the terminal scrolls or the output exceeds one line, `\r` causes garbled overlapping text
2. **No newline termination**: Progress updates don't end with `\n`, leaving partial lines that zsh later interprets
3. **Unicode in progress bars**: Emoji characters are multi-byte and interact poorly with `\r` positioning
4. **Long-running process + scroll**: When a command runs for 6+ minutes and scrolls past the visible area, carriage returns point to wrong positions

---

## Codebase Inventory: Where Output Is Written

### InlineTerminal (the ratatui-based path)

**`crates/roko-cli/src/inline/terminal.rs`**

The `InlineTerminal` struct owns a ratatui `Viewport::Inline` backend. It is the proper path for interactive terminal output. Key behaviors:

- `InlineTerminal::new()` at line 77: enables raw mode via `enable_raw_mode()`, creates a ratatui `Viewport::Inline(height)` (default 10 lines via `DEFAULT_VIEWPORT_HEIGHT`).
- `push_lines()` at line 133: uses `terminal.insert_before()` — pushes completed blocks into scrollback **without** using `\r`. This is the correct API.
- `should_use_inline()` at line 228: returns `io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none()`. This is the TTY detection gate — it uses the stdlib `IsTerminal` trait (imported at line 10), not the `atty` crate.
- `Drop` impl at line 219: calls `self.restore()` which calls `disable_raw_mode()` and flushes stdout. RAII-based cleanup.
- Panic hook at line 87: installed in `new()` — disables raw mode and shows cursor on panic.
- `ROKO_VIEWPORT_HEIGHT` env var controls viewport height.

**`crates/roko-cli/src/inline/symbols.rs`**

No emoji — only Unicode box-drawing and braille characters. Braille spinner frames (`⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧`) at line 57, rendered via `spinner_frame(tick: u64)`. Progress bar uses `━` (filled) and `░` (empty). These are all single-width Unicode, not multi-byte emoji, so they do not cause `\r` positioning issues.

**`crates/roko-cli/src/inline/styled.rs`**

All progress output is assembled as ratatui `Line<'static>` via `Span` builders — never raw `\r` or `eprint!`. Key builders:

- `section_start()` (line 21): `◆ label  value · detail`
- `continuation()` (line 47): `│ label     value · detail`
- `section_end()` (line 67): `└ label     value`
- `spinner_line()` (line 143): `│ ⠋ message... (elapsed)` — the live spinner
- `status_bar()` (line 158): `$0.0310  ·  4821 in / 1203 out  ·  haiku  ·  ━━━━░░ 62%`

None of these use raw `\r`. They all go through `InlineTerminal.push_lines()` → `insert_before()`.

**`crates/roko-cli/src/inline/primitives/streaming.rs`**

`StreamingState` (line 23): live streaming block rendered into the viewport. The `render()` method draws the braille spinner when the buffer is empty (the "Thinking..." phase), then switches to showing live text with a cursor. Also never uses `\r`.

### RunnerInlineTerminal (runner-local wrapper)

**`crates/roko-cli/src/runner/inline_output.rs`**

`RunnerInlineTerminal` wraps `InlineTerminal` and falls back to `InlineTarget::Plain` if the terminal is unavailable. The fallback path calls `write_plain()` (line 322) which writes to `io::stderr().lock()` — plain text with `\n`.

When `InlineTerminal::new()` fails (not a TTY, raw mode unavailable), the debug log says "inline terminal unavailable; using structured plain output". This is the correct fallback.

### FormattedStderrSink (the non-TTY / CI path)

**`crates/roko-cli/src/runner/output_sink.rs`, lines 402–781**

`FormattedStderrSink` writes structured `[plan/task]` prefixed lines to stderr. Used by `roko do` and `roko plan run` when the terminal is interactive but the inline UI is not active, or when TTY detection has not been wired.

Output format:
```
[plan-id/task-id] > Agent starting: "Add rate limiting middleware" [architect]
[plan-id/task-id] > Agent: claude-opus-4-5 (claude_cli pid 12345)
[plan-id/task-id] | Reading crates/roko-agent/src/dispatcher/mod.rs
[plan-id/task-id] + Gate passed: compile (2.3s)
[plan-id/task-id] x Gate failed: test -- 2 test failures
[plan-id/task-id] + Task completed (3/6) in 45.2s
[plan-id] + Plan complete: 6/6 passed in 3m12s
```

Each line ends with `\n` (via `writeln!`). No `\r`. The `emit()` method at line 444 does `writeln!(stderr, "{line}")`.

Icon convention:
- `>` yellow = in-progress/info
- `+` green = pass/success
- `x` red = fail/error
- `|` dim = agent output content
- ` ` = token counts

### StderrSink (rich inline path — currently the default for `roko plan run`)

**`crates/roko-cli/src/runner/output_sink.rs`, lines 160–380**

`StderrSink` owns a `RunnerInlineTerminal` and accumulates agent text in a `Mutex<String>` buffer. It is the default sink when `roko plan run` runs without `--quiet`. It delegates to `RunnerInlineTerminal` which calls `InlineTerminal.push_lines()` — the ratatui path.

**The corruption risk is in this path.** When `InlineTerminal::new()` fails silently (not detected as a TTY by `should_use_inline()`), it falls through to `write_plain()` without informing the caller.

### Sink selection in commands

**`crates/roko-cli/src/commands/do_cmd.rs`, lines 558–566:**

```rust
output_sink: if !cli.quiet && !cli.json {
    Arc::new(FormattedStderrSink::new(cli.color.should_color()))
} else {
    Arc::new(NoopSink)
},
```

**`crates/roko-cli/src/commands/plan.rs`, lines 542–552:**

```rust
output_sink: if !approval && !cli.quiet && !cli.json {
    Arc::new(FormattedStderrSink::new(cli.color.should_color()))
} else {
    Arc::new(NoopSink)
},
```

Both use `FormattedStderrSink` by default. Neither checks `should_use_inline()` at sink selection time — the TTY check is deferred to `RunnerInlineTerminal::new()` inside `StderrSink`. The `StderrSink` itself is not used here — only `FormattedStderrSink` and `NoopSink`.

### Legacy eprintln! in StderrSink

**`crates/roko-cli/src/runner/output_sink.rs`, lines 371–379:**

```rust
fn plan_summary(...) {
    eprintln!("[{plan_id}] summary: ...");
}

fn agent_line(&self, plan_id, task_id, line) {
    eprintln!("[{plan_id}/{task_id}]   {line}");
}
```

These bypass `emit()` and go directly to `eprintln!`. They always end with `\n` so they are not corruption sources, but they are inconsistent with the `writeln!(stderr.lock(), ...)` pattern used everywhere else.

---

## TTY Detection

**`crates/roko-cli/src/inline/terminal.rs`, line 228:**

```rust
pub fn should_use_inline() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}
```

Uses `std::io::IsTerminal` (stable since Rust 1.70, imported at line 10). Returns `false` when:
- stdout is piped/redirected
- `NO_COLOR` env var is set (any value)
- `CLICOLOR=0` is NOT checked (gap — `NO_COLOR` only)

This function exists but is only called from `bench_demo.rs` (line 112). **It is not called when constructing `FormattedStderrSink` or `StderrSink`** — the commands don't pass the TTY state through to the sink constructors.

`InlineTerminal::new()` in terminal.rs will fail if not on a TTY (raw mode will fail or the viewport will not render) — this is how TTY detection is implicitly applied for the ratatui path. But `FormattedStderrSink` has no TTY guard at all, it always writes.

### Approval mode check

**`crates/roko-cli/src/commands/plan.rs`, line 559:**

```rust
if !std::io::stdout().is_terminal() {
    anyhow::bail!("approval mode requires an interactive terminal");
}
```

This explicitly checks `is_terminal()` for approval mode only. The same guard should be applied to the `StderrSink` path.

---

## The Actual Corruption Bug

The original corruption described in this doc (carriage returns, emoji fragments in zsh) predates the current ratatui-based architecture. The current `FormattedStderrSink` writes clean `\n`-terminated lines and does not use `\r` or emoji.

However, the corruption risk is still present in one scenario: if `StderrSink` is used instead of `FormattedStderrSink` (it was previously the default), the ratatui viewport enters raw mode on stdout, and if the process crashes or is killed without calling `restore()`, the terminal is left in raw mode.

The `Drop` impl on `InlineTerminal` calls `restore()`, and the panic hook disables raw mode. But a `SIGKILL` bypasses both. This is the residual corruption risk.

---

## Proposed Solutions

### S1: Use stderr for progress, stdout for results only (already done)

```rust
// Progress goes to stderr (can be suppressed with 2>/dev/null)
eprintln!("  Generating plan from PRD: cursor-composer-backend");

// Final result goes to stdout
println!("✓ Plan generated: .roko/prd/plans/cursor-composer-backend/tasks.toml");
```

`FormattedStderrSink` already does this — all progress goes to stderr.

### S2: Use proper terminal handling (already done)

The current architecture uses ratatui `Viewport::Inline` via `InlineTerminal` — not raw `\r` carriage returns. The `push_lines()` → `insert_before()` pattern is the correct API. The problem was in old code that used `\r` directly; that code has been replaced.

### S3: TTY detection at sink selection (gap — not yet done)

`should_use_inline()` exists but is not used in command handlers. The fix is to call it when selecting the sink:

```rust
use roko_cli::inline::terminal::should_use_inline;

output_sink: if !cli.quiet && !cli.json {
    if should_use_inline() {
        Arc::new(StderrSink::new())  // ratatui path with inline viewport
            as Arc<dyn RunOutputSink>
    } else {
        Arc::new(FormattedStderrSink::new(
            cli.color.should_color() && std::env::var_os("NO_COLOR").is_none()
        )) as Arc<dyn RunOutputSink>
    }
} else {
    Arc::new(NoopSink)
},
```

Files to update:
- `crates/roko-cli/src/commands/do_cmd.rs:558`
- `crates/roko-cli/src/commands/plan.rs:542`

### S4: Simpler progress format (partially done via FormattedStderrSink)

The current `FormattedStderrSink` output is already clean:
```
[plan/task] > Agent starting: "Add rate limiting" [architect]
[plan/task] > Agent: claude-opus-4-5 (claude_cli)
[plan/task] | Reading crates/roko-serve/src/middleware/rate_limit.rs
[plan/task] + Gate passed: compile (2.3s)
[plan/task] + Task completed (3/6) in 45.2s
```

No emoji, no carriage returns, just clean log lines. This is S4 implemented.

### S5: Clean exit on error (already done via RAII)

`InlineTerminal` Drop impl calls `restore()`. Panic hook disables raw mode. Signal handler in `do_cmd.rs` (Ctrl-C cancel token) triggers graceful shutdown.

### S6: Handle CLICOLOR env var (gap)

`should_use_inline()` checks `NO_COLOR` but not `CLICOLOR=0`. Add:

```rust
pub fn should_use_inline() -> bool {
    io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("CLICOLOR").as_deref() != Ok("0")
        && std::env::var("CLICOLOR_FORCE").as_deref() != Ok("1")
}
```

### S7: Structured progress events for ACP (future work)

When the plan runner is invoked via ACP slash commands (e.g. `/plan-generate`), the `FormattedStderrSink` output goes to the subprocess stderr, which is captured after stdout EOF in `run_slash_command()`. This means progress appears late.

The design for a fix: emit structured `ProgressEvent` JSON lines to stdout alongside text output, which `run_slash_command()` parses and converts to `CognitiveEvent::ToolCallStart` / `CognitiveEvent::ToolCallComplete` events for Zed to display.

```rust
// Structured progress line (stdout, parseable by ACP):
// ROKO_PROGRESS: {"type":"task_started","task_id":"T001","title":"..."}
println!("ROKO_PROGRESS: {}", serde_json::to_string(&progress_event)?);
```

---

## Priority

1. **S3 (TTY detection at sink selection)** — immediate fix, prevents ratatui raw mode when piped.
2. **S6 (CLICOLOR support)** — easy two-line fix in `should_use_inline()`.
3. **S7 (structured progress in ACP)** — needed for proper Zed integration.

The core rule: **never leave the terminal in a broken state, no matter what.**
