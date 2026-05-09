# CLI Output Prettification

## Problem

The demo app's terminal panels show ugly raw output from `roko do` commands:
- WARN/INFO tracing lines (`2026-05-08T04:00:01.758Z WARN roko_core::config::loader: ...`)
  flood the visible output alongside actual user-facing progress
- The `do_cmd.rs` paths use raw `eprintln!("▸ ...")` — no colors, no structure, no spinners
- Plan completion summary uses unformatted `eprintln!` with manual column alignment
- No loading animation during agent execution (just silence until done)
- The inline terminal system (`ratatui Viewport::Inline`) and 11 primitives exist but
  `do_cmd.rs` doesn't use them at all

## What Already Exists (Don't Rebuild)

The CLI has a complete output system that `do_cmd.rs` simply isn't wired to:

| Component | File | Status |
|-----------|------|--------|
| Symbol language (◆◇│├└✔✖⚠▸⠋) | `inline/symbols.rs` | Ready |
| ANSI color helpers (green/red/yellow/cyan/magenta/dim/bold) | `output_format.rs` | Ready |
| Clack-style primitives (intro/step/bar/success/error/warning/branch/end) | `output_format.rs` | Ready |
| Inline ratatui viewport | `inline/terminal.rs` | Ready |
| Plaintext fallback (strips ANSI for pipes/CI) | `inline/plaintext.rs` | Ready |
| Braille spinner (indicatif) | `spinner.rs` | Ready |
| 11 reusable rendering primitives | `inline/primitives/*.rs` | Ready |
| session_summary primitive | `inline/primitives/session_summary.rs` | Ready |
| cost_meter/waterfall primitives | `inline/primitives/cost_meter.rs`, `cost_waterfall.rs` | Ready |
| gate_block primitive | `inline/primitives/gate_block.rs` | Ready |
| progress_tree primitive | `inline/primitives/progress_tree.rs` | Ready |

The demo app (xterm.js + WebGL) renders ANSI colors natively — any colors/styling
we add to CLI output will render correctly in the demo terminal panels.

## Fix Plan

### Fix 1: Suppress tracing logs from user-visible output

**Problem:** `WARN roko_core::config::loader: config warning: duplicate model slug...` lines
dominate the terminal. These are tracing subscriber stderr output.

**File:** `crates/roko-cli/src/main.rs` (~line 2070-2145)

The logging setup already has a file layer (`.roko/roko.log`) and a conditional stderr layer.
But the stderr layer fires on default `roko=info` level. For `roko do` specifically:

```rust
// In the do command path, raise stderr filter to suppress noise:
// - config::loader warnings → file only
// - agent::provider info → file only
// - agent::tool_loop info → file only
// Only show errors on stderr for do command
if is_do_command {
    stderr_filter = "roko=error,roko_runtime::effect_driver=info";
}
```

Alternative: Add `--verbose` / `-v` to `roko do` that lowers the filter. Silent by default.

**Config-level fix:** The "duplicate model slug" warnings should also be deduplicated or
downgraded to `debug` in `config::loader` — they fire on every single command invocation.

### Fix 2: Replace raw `eprintln!("▸ ...")` with output_format primitives

**File:** `crates/roko-cli/src/commands/do_cmd.rs`

Current (every `▸` line):
```rust
eprintln!("\u{25b8} Complexity: {} (auto-detected)", complexity_label(complexity));
eprintln!("\u{25b8} Running single agent...");
```

Replace with the existing clack primitives:
```rust
// Simple path
output_format::intro("roko do");
output_format::step("prompt", &output_format::dim(&truncate(prompt, 60)));
output_format::step("complexity", &format!("{} (auto-detected)", complexity_label(complexity)));
output_format::step("model", &output_format::cyan(&model_name));
output_format::divider();

// Standard path
output_format::step("step 1/2", "Generating plan...");
// ... then after:
output_format::step("step 2/2", &format!("Executing plan ({total_tasks} tasks)..."));

// Complex path
output_format::step("step 1/4", "Creating PRD...");
output_format::step("step 2/4", "Drafting PRD...");
output_format::step("step 3/4", "Generating plan...");
output_format::step("step 4/4", &format!("Executing plan ({total_tasks} tasks)..."));

// Errors
output_format::error(&format!("Plan generation failed (exit {exit_code})"));
output_format::warning("Falling back to plan-from-prompt path");
```

### Fix 3: Add spinner during agent execution

**File:** `crates/roko-cli/src/commands/do_cmd.rs` (in `run_simple_path`)

The `spinner.rs` wrapper already exists and uses indicatif:

```rust
use roko_cli::spinner::cli_spinner;

// Before agent dispatch:
let spinner = cli_spinner("Running agent...");

// After dispatch returns:
spinner.finish_and_clear();
output_format::success(&format!("workflow completed ({} agent turns)", report.agent_turns));
```

For the standard/complex paths, use step-aware spinners:
```rust
let spinner = cli_spinner("Generating plan...");
// ... agent call ...
spinner.finish_with_message("Plan generated (3 tasks)");

let spinner = cli_spinner("Executing plan...");
// ... execution ...
spinner.finish_and_clear();
```

### Fix 4: Replace raw plan completion summary with output_format

**File:** `crates/roko-cli/src/commands/do_cmd.rs` (~line 630-680)

Current:
```rust
eprintln!("\n▸ Plan complete: {}/{} tasks, ${:.2}, {}s", ...);
eprintln!("  {:.<24} {:>8} ...", "task", "tok_in", ...);  // manual column alignment
```

Replace with the session_summary + cost_waterfall primitives that already exist:
```rust
// Use output_format for the summary block:
output_format::intro("Plan complete");
output_format::branch(&format!("tasks      {}/{}", completed, total));
output_format::branch(&format!("cost       {}", output_format::cyan(&format!("${:.4}", cost))));
output_format::branch(&format!("duration   {}", output_format::cyan(&format!("{}s", secs))));
output_format::divider();

// Per-plan status with colors:
for p in &v2_report.plans {
    if p.completed {
        output_format::branch(&format!(
            "{} {} -- {}/{} tasks",
            output_format::green("✔"),
            p.plan_id,
            p.tasks_completed,
            p.tasks_total,
        ));
    } else {
        output_format::branch(&format!(
            "{} {} -- {}/{} tasks",
            output_format::red("✖"),
            p.plan_id,
            p.tasks_completed,
            p.tasks_total,
        ));
    }
}

// Per-task cost table with colors:
if !v2_report.task_costs.is_empty() {
    output_format::divider();
    output_format::step("Task costs", "");
    output_format::bar(&output_format::dim(&format!(
        "  {:.<24} {:>8} {:>8} {:>9} {:>6} {:>6}",
        "task", "tok_in", "tok_out", "cost", "calls", "result"
    )));
    for tc in &v2_report.task_costs {
        let outcome_colored = match tc.outcome.as_str() {
            "pass" | "ok" => output_format::green(&tc.outcome),
            "fail" => output_format::red(&tc.outcome),
            _ => output_format::dim(&tc.outcome),
        };
        output_format::bar(&format!(
            "  {:.<24} {:>8} {:>8} ${:>7.4} {:>6} {:>6}",
            tc.task_id, tc.tokens_in, tc.tokens_out, tc.cost_usd,
            tc.agent_calls, outcome_colored,
        ));
    }
}
output_format::end(&output_format::dim(&v2_report.run_id));
```

### Fix 5: Color the model line in the header

**File:** `crates/roko-cli/src/commands/do_cmd.rs` (~line 807-828)

The `print_run_preview` function already exists but uses plain `println!`. Wire it through
`output_format`:
```rust
output_format::intro("roko do");
output_format::step("prompt", &output_format::dim(&truncate(prompt, 60)));
output_format::step("model", &output_format::cyan(&format!("{} via {}", model, provider)));
output_format::step("complexity", &complexity_label(complexity));
output_format::divider();
```

### Fix 6: Failure details with color

**File:** `crates/roko-cli/src/commands/do_cmd.rs` (~line 668-679)

```rust
if v2_report.tasks_failed > 0 && !cli.quiet && !v2_report.failure_reasons.is_empty() {
    output_format::divider();
    output_format::step("Failures", "");
    for (key, reason) in &v2_report.failure_reasons {
        output_format::branch(&format!("{} {}", output_format::red("✖"), output_format::bold(key)));
        for line in reason.lines().take(5) {
            output_format::bar(&format!("  {}", output_format::dim(line)));
        }
    }
}
```

### Fix 7: Downgrade noisy config warnings

**File:** `crates/roko-core/src/config/loader.rs`

The "duplicate model slug" warnings fire on every command and clutter the demo:
```rust
// Change from:
warn!("config warning: duplicate model slug '{}' ...", slug);
// To:
debug!("config warning: duplicate model slug '{}' ...", slug);
```

Or better: deduplicate at config load time and only emit one summary warning.

## What This Looks Like After

### Before (current)
```
roko --model 'gpt54-mini' do "build a function that checks if a number is prime"
▸ Complexity: simple (auto-detected)
▸ Running single agent...
2026-05-08T04:00:01.758866Z  WARN roko_core::config::loader: config warning: duplicate model slug 'gemini-2.5-flash' ...
2026-05-08T04:00:01.759001Z  WARN roko_core::config::loader: config warning: duplicate model slug 'claude-sonnet-4-6' ...
2026-05-08T04:00:01.759009Z  WARN roko_core::config::loader: config warning: duplicate model slug 'glm-5.1' ...
model: gpt54-mini via openai (source: cli override)
2026-05-08T04:00:01.784895Z  INFO roko_runtime::effect_driver: EffectDriver: calling model_caller ...
2026-05-08T04:00:01.794652Z  INFO roko_agent::provider: creating agent via provider adapter ...
2026-05-08T04:00:02.940009Z  INFO roko_agent::tool_loop: tool_loop: dispatching tool calls iteration=0 ...
... 12 more INFO lines ...
◆ roko run
◇ prompt  build a function that checks if a number is prime
◇ workflow  focused
◇ model  gpt-5.4-mini
│
✔  workflow completed (1 agent turn)
│  Implemented a prime-checking function ...
│
◇ Summary
├  duration   23.3s
├  tokens     127234
├  cost       0.0718
├  gates      (none configured)
└  run_18ad7b26d8af91cb
```

### After (with fixes applied)
```
◆  roko do
◇  prompt     build a function that checks if a number is prime
◇  model      gpt-5.4-mini via openai
◇  complexity simple (auto-detected)
│
⠋ Running agent... (3.2s)          ← live spinner, replaced on completion
│
✔  workflow completed (1 agent turn)
│  Implemented a prime-checking function in `prime.rs`:
│  - `is_prime(n: u64) -> bool`
│  - Efficient primality test using 6k ± 1 optimization
│
◇  Summary
├  duration   23.3s
├  tokens     127 234
├  cost       $0.0718
├  gates      (none configured)
└  run_18ad7b26d8af91cb
```

No WARN/INFO noise. Colors on model, duration, cost, tokens. Spinner during execution.
Same Unicode structure but actually using the output_format helpers that add ANSI codes.

## Files to Modify

| File | Change | Effort |
|------|--------|--------|
| `crates/roko-cli/src/commands/do_cmd.rs` | Replace all `eprintln!("▸ ...")` with output_format primitives + spinners | Medium |
| `crates/roko-cli/src/main.rs` (~2070-2145) | Raise stderr log filter for `do` command path | Small |
| `crates/roko-core/src/config/loader.rs` | Downgrade duplicate slug warnings to `debug!` | Trivial |
| `crates/roko-cli/src/run.rs` | Already uses output_format — minor alignment tweaks | Small |

## Priority

**P1** — This is the first thing anyone sees in the demo. The raw log noise makes roko
look broken even when it's working correctly. The fixes are almost entirely wiring: the
output system exists, it just needs to be called from do_cmd.rs.

## Non-Goals

- Don't touch the inline ratatui viewport for this — that's the `roko dashboard` / TUI path
- Don't add new dependencies — raw ANSI codes + indicatif spinner already cover everything
- Don't change the demo app's terminal renderer — xterm.js already handles all ANSI codes
