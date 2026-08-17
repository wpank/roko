# 03: CLI Output Noise

## Problem

Every roko command prints irrelevant warnings that obscure real information. The signal-to-noise ratio is terrible.

### Example (every single command)

```
model: gpt54-mini via openai (source: project default)
warning: Empty api_key_env for provider 'ollama'. Configure it in roko.toml [providers.ollama].
warning: Missing GEMINI_API_KEY for provider 'gemini'. Export it in your shell or in a .env file.
📋 Generating plans from PRD: cursor-composer-backend
warning: repo context build timed out while collecting symbol matches; returning partial results
warning: repository context not verified for keywords ["backend", "composer", "cursor"]; generated plan may reference nonexistent code.
```

3 of 5 lines are irrelevant warnings about providers the user isn't using.

## Root Causes

1. **Provider health checks run eagerly for ALL providers**, not just the one being used.
2. **Warnings have no severity levels.** An unconfigured provider you don't use gets the same prominence as a real problem.
3. **"model: X via Y (source: Z)"** is debug info printed at INFO level.
4. **No distinction between first-run noise and persistent misconfiguration.**

---

## Codebase Inventory: Where Progress Output Is Emitted

### FormattedStderrSink — the default for `roko do` and `roko plan run`

**`crates/roko-cli/src/runner/output_sink.rs`, lines 402–781**

This is the primary progress sink for interactive CLI commands. All output goes to stderr via `writeln!`. Output lines use the format `[plan/task] ICON message`. There is no "noise" here — it only emits structured events that correspond to actual plan runner state changes.

Icons:
- `>` (yellow): in-progress / starting
- `+` (green): success / pass
- `x` (red): failure / error
- `|` (dim): agent text content

**No emoji.** `FormattedStderrSink` uses ASCII icons only. The emoji in the problem example (`📋`) came from a previous implementation.

### symbols.rs — no emoji

**`crates/roko-cli/src/inline/symbols.rs`**

All symbols are Unicode box-drawing / braille / geometric shapes — no emoji:
- `◆` (START), `│` (BAR), `└` (END), `✔` (PASS), `✖` (FAIL), `⚠` (WARN)
- `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧` (spinner frames)
- `━` / `░` (progress bar fill / empty)
- `⏳` (PENDING) — this one could cause issues on older terminals

The `⏳` symbol at line 44 (PENDING constant) is the one emoji-like character. It is not actually emoji (U+23F3, HOURGLASS WITH FLOWING SAND) but it can render inconsistently. It is currently unused in `FormattedStderrSink`.

### Sink selection — where `--quiet` and `--json` suppress output

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

`--quiet` (`-q`) is already wired for both commands. It silences all sink output. `--json` also silences it. These are the correct hooks for "clean output" mode.

### The actual noise sources

The `model: X via Y (source: Z)` line and provider warnings are NOT emitted by `FormattedStderrSink`. They come from elsewhere — most likely from the provider resolution code in `roko-core` or `roko-agent` that runs before the plan executor. The config loading path in `roko-core/src/config/` contains provider validation logic that fires on startup.

The warnings about unused providers (`ollama`, `gemini`) are from provider health checks that run eagerly on startup for ALL configured providers. This is the core noise source.

### `clean_output` config flag

A search for `clean_output` across the codebase returns no matches. **This flag does not exist** in the current code. It was mentioned in earlier planning docs but was not implemented. The mechanism for clean output is `--quiet` (`-q`) on CLI commands.

### Color detection

**`crates/roko-cli/src/inline/terminal.rs`, line 228:**

```rust
pub fn should_use_inline() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}
```

Uses `std::io::IsTerminal` (stdlib, stable since 1.70). `FormattedStderrSink::new(color: bool)` receives the color decision from the command, which calls `cli.color.should_color()`.

---

## Proposed Solutions

### S1: Only warn about the provider being used (highest impact)

The noise from unconfigured providers is the biggest problem. Fix: validate only the resolved provider, not all providers.

```rust
// Before (current): validate ALL providers at startup
for (name, provider) in &config.providers {
    if provider.api_key_env.is_empty() { warn!(...) }
}

// After: validate only the resolved provider
let resolved = resolve_provider(model)?;
if resolved.needs_key() && !resolved.has_key() {
    // This is actually actionable — show it
    eprintln!("error: {} requires {} (not set)", resolved.name, resolved.key_env);
}
```

The provider validation loop is in the config loading path. Search for it in `crates/roko-core/src/config/` and `crates/roko-agent/src/` to find the exact location.

### S2: Severity tiers for output

| Level | When | Format |
|-------|------|--------|
| Error | Blocks execution | `✖ error: ...` (red) |
| Warning | Degraded but continuing | `⚠ ...` (yellow) — only if relevant to current op |
| Info | Progress updates | `  Generating plan...` / `✔ Done` |
| Debug | Only with `--verbose` | Everything else |

This maps naturally to `FormattedStderrSink`'s existing `emit_fail` / `emit_progress` / `emit_pass` / `emit_event` methods. The gap is that provider warnings bypass `FormattedStderrSink` entirely.

### S3: Suppress repeat warnings

If the same warning has been shown in the last N commands (stored in `.roko/state/warnings.json`), suppress it. Or: show it once per session, not once per command.

### S4: `--quiet` / `-q` flag (already implemented)

```
$ roko prd plan cursor-composer-backend -q
✔ Plan generated: .roko/prd/plans/cursor-composer-backend/tasks.toml (6 tasks)
```

`--quiet` already exists and is wired in `do_cmd.rs` and `plan.rs` — it switches from `FormattedStderrSink` to `NoopSink`. The gap is that provider warnings bypass the sink entirely (they use `warn!` tracing macros or direct `eprintln!` before the sink is constructed).

### S5: Move model/source line to `--verbose`

```
# Current (always shown):
model: gpt54-mini via openai (source: project default)

# Proposed: only with --verbose or on first use
# Normal output just starts with the action:
[plan/task] > Agent starting: "Add rate limiting" [architect]
```

The model/source line comes from the agent dispatcher, not the sink. It would need to be moved to `tracing::debug!` or gated on a `--verbose` flag.

### S6: Emoji audit

The `📋` emoji in the old problem example came from a previous code path. Current `FormattedStderrSink` uses no emoji. However, `symbols.rs` defines `PENDING: &str = "⏳"` (line 44) which is an emoji-like character. It is not currently used in `FormattedStderrSink` but could be in future. Recommendation: replace `⏳` with `○` or `◌` (hollow circle) for consistency with the rest of the symbol set.

### S7: Structured progress events for ACP (new — different noise problem)

When slash commands run in Zed via ACP, the `FormattedStderrSink` output becomes noise in the ACP text stream. The correct fix is structured progress events — see doc 18 for details.

---

## Specific Fixes

| Current output | Problem | Fix |
|---|---|---|
| `warning: Empty api_key_env for provider 'ollama'` | Not using ollama | Don't validate unused providers (S1) |
| `warning: Missing GEMINI_API_KEY` | Not using gemini | Same (S1) |
| `model: gpt54-mini via openai (source: project default)` | Debug info at INFO | Move to `--verbose` (S5) |
| `warning: repo context build timed out` | Unclear impact | Either fix timeout or say "⚠ Generating without repo context (timeout)." |
| `warning: repository context not verified for keywords` | Unclear action | Remove or make `--verbose` note |

## Priority

S1 (only warn about active provider) is trivial and eliminates 60% of the noise. S4 (`-q`) already exists and gives power users an escape hatch. Both are easy wins.

S6 (emoji audit of `⏳`) is a one-line fix. Do it.

The more fundamental fix (S2 severity tiers) requires routing provider warnings through the same output sink used by plan runner events, which means restructuring how config loading emits warnings. That is the right long-term architecture.
