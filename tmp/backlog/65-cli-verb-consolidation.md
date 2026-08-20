# 65 — CLI Verb Consolidation

**Priority**: P3 — `roko --help` outputs 46+ top-level verbs with no visual grouping, overwhelming new users
**Size**: L (5–8 days)
**Crates**: `crates/roko-cli` (primary: `src/main.rs`, `src/agent_serve.rs`, `src/commands/util.rs`)
**Depends on**: None

---

## Background

The roko CLI has grown organically to include over 46 top-level subcommands. When a new user types `roko help`, they see a wall of verbs that scrolls past a single terminal screen with no indication of which five they actually need. The `COMMAND GROUPS` text that organizes them into categories only appears on `roko --help` (the long form with `after_long_help`), not on the default `roko help` output. Related operations are scattered across different top-level verbs: `tune` vs `learn tune`, `dev` vs `up` vs `serve`, `show` vs `status` vs `dashboard`. Shell completions only handle two nesting levels; deep paths like `roko config plugins audit` do not complete the third word.

The work here is purely organizational: reduce the top-level verb count, add visual help grouping, improve shell completion depth, and preserve all existing paths as hidden deprecated aliases. No functionality is removed or changed.

## Current State

1. **`enum Command`** — `crates/roko-cli/src/main.rs` line 324. The `after_long_help` at line 229 groups verbs into 12 categories but this text is only shown with `roko --help`, not with `roko help` (short form).

2. **Top-level variants in `Command`** (from main.rs lines 334–831):
   `Init`, `Do`, `Develop`, `Run`, `Status`, `Github`, `Show`, `Doctor`, `Setup`, `Diagnose`, `LayerCheck`, `Plan`, `Prd`, `Agent`, `Research`, `Think`, `Note`, `Tune`, `Knowledge`, `Learn`, `Job`, `Market`, `Bench`, `Demo`, `Config`, `Index`, `Graph`, `Feed`, `Recipe`, `Trigger`, `Dev`, `Up`, `Serve`, `Acp`, `Daemon`, `Deploy`, `Worker`, `Dashboard`, `Screenshot`, `Login`, `Logout`, `Whoami`, `VisionLoop`, and more — approximately 46 variants.

3. **`TuneCmd`** — `crates/roko-cli/src/main.rs` line 1722. `Tune(TuneCmd)` at line 606 is a top-level verb. A duplicate `learn tune` path also exists under `LearnCmd`.

4. **Nested subcommand enums** (all defined in `main.rs` unless noted):

   | Parent | Enum | File:Line |
   |---|---|---|
   | `plan` | `PlanCmd` | `main.rs:1431` |
   | `prd` | `PrdCmd` | `main.rs:1628` |
   | `research` | `ResearchCmd` | `main.rs:1680` |
   | `knowledge` | `KnowledgeCmd` | `main.rs:936` |
   | `learn` | `LearnCmd` | `main.rs:1149` |
   | `config` | `ConfigCmd` | `main.rs:1971` |
   | `config providers` | `ConfigProviderCmd` | `main.rs:2145` |
   | `config models` | `ConfigModelCmd` | `main.rs:2202` |
   | `config subscriptions` | `ConfigSubscriptionCmd` | `main.rs:2230` |
   | `config experiments model` | `ModelExperimentCmd` | `commands/experiment.rs:26` |
   | `config plugins` | `PluginCmd` | `main.rs:1273` |
   | `config mcp` | `ConfigMcpCmd` | `main.rs:2232` |
   | `agent` | `AgentCmd` | `agent_serve.rs:33` |
   | `daemon` | `DaemonCmd` | `main.rs:1361` |
   | `graph` | `GraphCmd` | `commands/graph.rs:22` |
   | `feed` | `FeedCmd` | `commands/feed.rs:14` |
   | `recipe` | `RecipeCmd` | `commands/recipe.rs:14` |
   | `trigger` | `TriggerCmd` | `commands/trigger.rs:19` |

5. **Shell completions** — `crates/roko-cli/src/commands/util.rs`. `print_completions()` at line 1466, `completion_words()` at line 1477, `nested_subcommand_words()` at line 1490. The nested function introspects the clap command tree to one level. Third-level subcommands (e.g., `roko config plugins audit`) are not completed. `dynamic_completion_words()` at line 1508 currently only adds plan names and PRD slugs to the `plan` and `prd` parent contexts.

6. **Help output** — `roko --help` shows the `COMMAND GROUPS` from `after_long_help` (line 229). `roko help` (the default short-form) does not. Clap 4.x supports `help_heading` on individual variants to group them in both help modes.

7. **Duplicate verbs** identified:
   - `roko status` vs `roko show` (both inspect workspace state)
   - `roko dashboard` and `roko show --live` (both open TUI)
   - `roko tune` and `roko learn tune` (both tune adaptive thresholds)
   - `roko dev` and `roko up` (both start server variants)
   - `roko config set-secret` and `roko config secrets set`

## Implementation Plan

### Step 1: Add `help_heading` to top-level Command variants

Clap 4.x supports `#[command(help_heading = "Category")]` on individual variants of a `#[derive(Subcommand)]` enum. This groups subcommands under headings in the default short help output — no `after_long_help` hack required.

In `crates/roko-cli/src/main.rs`, add `#[command(help_heading = "...")]` attributes to groups of variants in the `Command` enum. Proposed grouping (matching the existing `COMMAND GROUPS` text at line 230):

```rust
// ── Core workflow ──────────────────────────────────────────────────
#[command(help_heading = "Core workflow")]
Init { ... }
#[command(help_heading = "Core workflow")]
Do { ... }
// etc.

// ── Planning ───────────────────────────────────────────────────────
#[command(help_heading = "Planning")]
Plan { ... }
#[command(help_heading = "Planning")]
Prd { ... }
```

This does not remove any commands. It makes `roko --help` display them in organized groups without needing to read the `after_long_help` appendix.

### Step 2: Add a quick-start line to the top of help output

Change the `about` field in `Cli` from `"Minimal CLI for the Roko universal loop"` (line 228) to include a quick-start line:

```rust
about = "Roko — agent toolkit\n\nQuick start: roko setup, roko do \"task\", roko status\nRun roko help <command> for details."
```

### Step 3: Hide redundant top-level verbs with deprecation warnings

For each duplicate verb, add `#[command(hide = true)]` to the old position and emit a one-time deprecation warning when it is invoked. The command still executes — users with existing scripts are not broken.

Priority duplicates to hide:

| Old verb | Canonical path | Deprecation message |
|---|---|---|
| `roko tune` | `roko learn tune` | `warning: 'roko tune' is deprecated, use 'roko learn tune'` |
| `roko dev` | `roko serve dev` or just `roko serve` | `warning: 'roko dev' is deprecated, use 'roko serve'` |
| `roko up` | `roko serve` | `warning: 'roko up' is deprecated, use 'roko serve'` |
| `roko layer-check` | `roko doctor --check layers` or `roko doctor` | `warning: 'roko layer-check' is deprecated, use 'roko doctor'` |

Implementation: in the match arm for the old command in `main.rs`, add `eprintln!("warning: ...")` before delegating to the canonical handler. Add `#[command(hide = true)]` to the variant definition so it does not appear in help output.

### Step 4: Add `visible_alias` abbreviations for most-used verbs

Clap 4.x `#[command(visible_alias = "...")]` adds an alias that appears in help output. Add these to the most-used core verbs:

```rust
#[command(visible_alias = "s")]   // Status
#[command(visible_alias = "d")]   // Do
#[command(visible_alias = "p")]   // Plan
```

These are short-form aliases that work in shell but appear in help. Do NOT add them to anything that would create ambiguity (verify no existing single-letter subcommands conflict by checking `completion_words()`).

### Step 5: Fix shell completion depth for three-level commands

Replace the `nested_subcommand_words()` function in `crates/roko-cli/src/commands/util.rs` (line 1490) with a recursive version that collects two levels of nesting:

```rust
pub(crate) fn nested_subcommand_words() -> Vec<(String, Vec<String>)> {
    let mut command = Cli::command();
    command.build();
    let mut result = Vec::new();
    for sub in command.get_subcommands() {
        let parent = sub.get_name().to_string();
        // Level 2: children of top-level sub
        let children: Vec<String> = sub.get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        if !children.is_empty() {
            result.push((parent.clone(), children.clone()));
        }
        // Level 3: children of children
        for child_sub in sub.get_subcommands() {
            let child_name = child_sub.get_name().to_string();
            let grandchildren: Vec<String> = child_sub.get_subcommands()
                .map(|s| s.get_name().to_string())
                .collect();
            if !grandchildren.is_empty() {
                // Key is "parent child" for case matching
                result.push((format!("{parent} {child_name}"), grandchildren));
            }
        }
    }
    result
}
```

Update `print_bash_completions`, `print_zsh_completions`, and `print_fish_completions` in `commands/util.rs` to handle the two-word keys (e.g., `"config plugins"`) in their `case "$prev $pprev"` equivalent patterns.

For bash completions specifically, this requires tracking `${COMP_WORDS[COMP_CWORD-2]}` as `pprev` and matching on `"$pprev $prev"`.

### Step 6: Add `ValueHint` to path arguments

For all `--workdir`, `--config`, `--output`, and similar path arguments across all commands in `main.rs`, add:

```rust
#[arg(long, value_hint = clap::ValueHint::DirPath)]
workdir: Option<PathBuf>,

#[arg(long, value_hint = clap::ValueHint::FilePath)]
config: Option<PathBuf>,
```

This enables path tab completion in shells that support `clap_complete` value hints. Affects the global `--config` arg on `Cli` (line 246) and the per-command `--workdir` args throughout.

### Step 7: Add a completion test

In `crates/roko-cli/tests/` (or in `src/commands/util.rs` test module), add a test that calls `completion_words()` and asserts the expected top-level verbs and `nested_subcommand_words()` and asserts that `config plugins` maps to `["audit", "install", "list", "publish", "remove"]` (confirming three-level completion data is present).

## Acceptance Criteria

1. `roko --help` groups subcommands under visual headings (Core workflow, Planning, Agents, etc.) rather than a flat list.
2. `roko help` (short form) also shows grouped headings (not just on `--help`).
3. Top-level verb count visible in help output reduced from ~46 to ≤ 25 (hidden verbs still work, just not listed).
4. Deprecated verbs (`roko tune`, `roko dev`, `roko up`, `roko layer-check`) print a warning message but continue to function.
5. Shell completions handle three nesting levels: `roko config plugins <TAB>` completes `audit`, `install`, `list`, `publish`, `remove`.
6. `--workdir` and path arguments have `ValueHint::DirPath`/`ValueHint::FilePath` annotations.
7. `roko s` works as an alias for `roko status`.
8. `cargo test -p roko-cli` passes with zero regressions.
9. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] Run `roko --help`; confirm subcommands appear under group headings
- [ ] Run `roko help` (short form, no `--help` flag); confirm headings appear
- [ ] Count visible top-level verbs in help output; confirm ≤ 25
- [ ] Run `roko tune routing`; confirm deprecation warning is printed; confirm it still executes
- [ ] Run `roko up`; confirm deprecation warning appears
- [ ] Source `roko completions bash` in a shell; type `roko config plugins <TAB>`; confirm completions
- [ ] Type `roko run --model <TAB>` (if backlog #64 is also done); confirm model names
- [ ] Run `roko s`; confirm it works as `roko status`
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/main.rs` | Add `help_heading` to all `Command` variants; add `visible_alias` to core verbs; add `hide = true` to deprecated variants; add deprecation `eprintln!` in match arms; add `ValueHint` to path args |
| `crates/roko-cli/src/commands/util.rs` | Replace `nested_subcommand_words()` with recursive two-level version; update `print_bash_completions`, `print_zsh_completions`, `print_fish_completions` to handle two-word parent keys |
| `crates/roko-cli/CLAUDE.md` or project `CLAUDE.md` | Update CLI reference table to reflect new canonical verb structure |
