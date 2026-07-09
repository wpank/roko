# Roko CLI: Developer Experience & UX Improvements

**Date**: 2026-04-21
**Scope**: Shell completions, modern CLI patterns, TUI enhancements, error UX, and general DX upgrades for the `roko` CLI binary.

---

## Background

Roko is a Rust CLI tool (`crates/roko-cli/`) built with **clap v4** (derive macros). It has 35+ top-level subcommands, many with nested sub-subcommands (e.g. `roko plan run`, `roko prd draft new`). It already has:

- An interactive ratatui TUI dashboard (`roko dashboard`)
- A `roko completions bash|zsh|fish` subcommand with hand-rolled completion scripts
- Dynamic completions that scan the filesystem for plan names and PRD slugs
- A `--json` global flag for machine-readable output
- A `--quiet` flag to suppress non-essential output
- `--high-contrast` and `--reduced-motion` accessibility flags on the dashboard

What follows is everything that can be added to bring the CLI to the level of best-in-class tools like `cargo`, `gh`, `mise`, `starship`, and `zoxide`.

---

## 1. Shell Completions

### Current State

`roko completions <shell>` generates completion scripts for bash, zsh, and fish. The implementation is hand-rolled (not using `clap_complete`) and lives at `crates/roko-cli/src/main.rs:7553-7730`. It already does:

- Top-level subcommand completion
- Nested subcommand completion (2 levels deep)
- Dynamic completions from filesystem (plan names from `plans/`, PRD slugs from `.roko/prd/`)

### What To Add

#### 1a. PowerShell and Nushell support

The current `CompletionShell` enum only covers bash/zsh/fish. Add PowerShell (for Windows/cross-platform users) and Nushell (increasingly popular in the Rust ecosystem).

**Approach**: Use `clap_complete` (v4.6+) for these two shells specifically, since hand-rolling PowerShell and Nushell completion scripts is complex and error-prone. Keep the existing hand-rolled bash/zsh/fish scripts since they already support dynamic completions that `clap_complete` doesn't provide out of the box.

```toml
# Cargo.toml (workspace)
clap_complete = "4"
clap_complete_nushell = "4"  # community-maintained
```

```rust
// Extend the enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Nushell,
    Elvish,
}
```

For PowerShell and Nushell, delegate to `clap_complete::generate()`:

```rust
use clap_complete::{generate, shells::PowerShell};

CompletionShell::PowerShell => {
    let mut cmd = Cli::command();
    generate(PowerShell, &mut cmd, "roko", &mut std::io::stdout());
}
```

#### 1b. Deeper dynamic completions

The current dynamic completions only work at 2 levels. Extend to complete:

- `roko plan run <TAB>` → plan directory names
- `roko prd plan <TAB>` → PRD slugs
- `roko prd draft promote <TAB>` → draft PRD slugs (filter by status)
- `roko chat --agent <TAB>` → running agent IDs (from `.roko/state/`)
- `roko replay <TAB>` → recent signal hashes (from `.roko/signals.jsonl`, last N)
- `roko config set <TAB>` → known config keys from the schema
- `roko explain <TAB>` → known topic names (gates, routing, cognitive, etc.)
- `roko new <TAB>` → scaffold types (gate, scorer, router, policy, etc.)
- `roko tune <TAB>` → subsystem names (gates, routing, budget)
- `roko learn <TAB>` → section names (router, experiments, efficiency, episodes, all)
- `roko plugin install <TAB>` → available plugin names from registry

This requires extending `dynamic_completion_words()` to scan more directories and parse known value sets.

#### 1c. Flag completions

The hand-rolled scripts don't complete flags (`--config`, `--role`, `--model`, etc.). Add:

```bash
# In the bash completion function, when cur starts with -
if [[ "$cur" == -* ]]; then
    COMPREPLY=( $(compgen -W "--config --role --model --repo --resume --effort --json --log-format --quiet --no-replan --headless --help --version" -- "$cur") )
    return 0
fi
```

Same pattern for zsh (`_arguments`) and fish (`complete -l`).

#### 1d. User-facing setup instructions

Add a `roko completions --setup` flag that prints the exact line the user should add to their shell config:

```
$ roko completions zsh --setup
# Add this line to your ~/.zshrc:
#   eval "$(roko completions zsh)"
# Then restart your shell or run:
#   source ~/.zshrc
```

---

## 2. `roko init <shell>` — Unified Shell Integration

### Concept

The starship/mise/zoxide pattern: a single `eval` line that sets up everything — completions, aliases, environment, and shell hooks. This is the single highest-leverage DX improvement because it replaces manual setup with one line.

### Design

```bash
# User adds one line to ~/.zshrc:
eval "$(roko init zsh)"
```

This command outputs a shell script that includes:

1. **Shell completions** (the same output as `roko completions zsh`)
2. **Aliases** for common workflows:
   ```bash
   alias rr='roko run'
   alias rp='roko plan run'
   alias rs='roko status'
   alias rd='roko dashboard'
   alias rprd='roko prd'
   ```
3. **Environment setup**:
   ```bash
   export ROKO_ROOT="$(roko config path 2>/dev/null || echo .)"
   ```
4. **Shell hooks** (optional, controlled by config):
   ```bash
   # Auto-show project status when cd-ing into a roko project
   _roko_chpwd() {
     if [[ -f "./roko.toml" ]]; then
       roko status --quiet 2>/dev/null
     fi
   }
   chpwd_functions+=(_roko_chpwd)
   ```

### Implementation

Add a new `Init` variant to the shell integration (separate from the existing `Init` subcommand that creates `.roko/`). Options:

**Option A**: New subcommand `roko shell-init zsh` (avoids collision with existing `roko init`).

**Option B**: Flag on existing init: `roko init --shell zsh` (but `roko init` currently creates `.roko/`, so this is confusing).

**Option C**: Overload completions: `roko completions zsh --full` (includes aliases + hooks).

**Recommended**: Option A (`roko shell-init <shell>`). Clean separation, no ambiguity.

```rust
/// Generate shell integration script (completions + aliases + hooks).
ShellInit {
    /// Shell to generate integration for.
    #[arg(value_enum)]
    shell: CompletionShell,
    /// Skip alias definitions.
    #[arg(long)]
    no_aliases: bool,
    /// Skip directory-change hooks.
    #[arg(long)]
    no_hooks: bool,
}
```

---

## 3. Interactive Fuzzy Fallbacks

### Concept

When a user runs a command that requires an argument but doesn't provide one, instead of printing a usage error, show an interactive picker. This is what `gh` does for repo/branch selection and what `mise` does for tool versions.

### Where To Apply

| Command | Missing arg | Picker shows |
|---|---|---|
| `roko plan run` | no plan dir | List of discovered plan directories |
| `roko plan show` | no plan id | List of plan IDs with titles |
| `roko prd plan` | no slug | List of PRD slugs with titles |
| `roko prd draft promote` | no slug | List of draft PRDs |
| `roko replay` | no hash | Last 20 signal hashes with timestamps |
| `roko chat --agent` | default agent | List of running agents |
| `roko explain` | no topic | List of available topics |
| `roko config set` | no key | List of config keys with current values |

### Implementation

Add `dialoguer` as a dependency:

```toml
# crates/roko-cli/Cargo.toml
dialoguer = { version = "0.11", features = ["fuzzy-select"] }
```

Pattern for each command handler:

```rust
use dialoguer::FuzzySelect;

async fn cmd_plan_run(cli: &Cli, dir: Option<PathBuf>) -> Result<i32> {
    let dir = match dir {
        Some(d) => d,
        None if std::io::stdin().is_terminal() => {
            // Interactive mode: show picker
            let plans = discover_plans(&resolve_workdir(cli))?;
            if plans.is_empty() {
                anyhow::bail!("No plans found. Create one with `roko plan create`.");
            }
            let labels: Vec<&str> = plans.iter().map(|p| p.name.as_str()).collect();
            let idx = FuzzySelect::new()
                .with_prompt("Select a plan to run")
                .items(&labels)
                .interact()?;
            plans[idx].path.clone()
        }
        None => {
            anyhow::bail!("No plan directory specified. Usage: roko plan run <dir>");
        }
    };
    // ... rest of handler
}
```

Key detail: only show the picker when stdin is a terminal (`std::io::stdin().is_terminal()`). In non-interactive contexts (pipes, CI), fall back to the normal error message. This prevents breaking scripts.

---

## 4. Man Page Generation

### Concept

Generate ROFF man pages from the clap command definition so users can run `man roko`, `man roko-plan`, etc. Single source of truth — the same `#[command]` / `#[arg]` annotations that drive `--help` also drive the man pages.

### Implementation

Add `clap_mangen` to build dependencies:

```toml
# crates/roko-cli/Cargo.toml
[build-dependencies]
clap = { workspace = true, features = ["derive"] }
clap_mangen = "0.2"
```

Create `crates/roko-cli/build.rs`:

```rust
use clap::CommandFactory;
use clap_mangen::Man;
use std::fs;
use std::path::PathBuf;

// Include the CLI struct definition (or reference it)
include!("src/cli_types.rs"); // factor out Cli struct for build.rs access

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(
        std::env::var_os("OUT_DIR").unwrap_or_else(|| "target/man".into()),
    );
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir)?;

    let cmd = Cli::command();
    // Generate main man page
    Man::new(cmd.clone()).render(&mut fs::File::create(man_dir.join("roko.1"))?)?;

    // Generate per-subcommand man pages
    for sub in cmd.get_subcommands() {
        let name = format!("roko-{}", sub.get_name());
        let path = man_dir.join(format!("{name}.1"));
        Man::new(sub.clone()).render(&mut fs::File::create(path)?)?;
    }

    Ok(())
}
```

Add a CLI command to install them:

```rust
/// Install man pages to the system man directory.
ManPages {
    /// Output directory (default: /usr/local/share/man/man1/).
    #[arg(long, default_value = "/usr/local/share/man/man1")]
    dir: PathBuf,
}
```

Or simpler: `roko completions --man` outputs the man page to stdout, and the user redirects:

```bash
roko man roko > /usr/local/share/man/man1/roko.1
roko man roko-plan > /usr/local/share/man/man1/roko-plan.1
```

---

## 5. Rich Error Diagnostics with `miette`

### Current State

The codebase uses `anyhow::Result` for error handling and `thiserror` for error type definitions. Errors are printed as plain text strings.

### What `miette` Adds

`miette` is a diagnostic library that renders errors with:

- **Source spans** — point to the exact line/column in config files or plan TOML that caused the error
- **Help text** — actionable suggestions ("did you mean X?", "run Y to fix")
- **Error codes** — machine-readable codes for documentation lookup
- **Related errors** — group multiple issues into one diagnostic
- **Colored output** — respects `NO_COLOR`

### Example: Before vs After

**Before (anyhow)**:
```
Error: invalid gate configuration in roko.toml: unknown gate type "comiple"
```

**After (miette)**:
```
  x Error parsing roko.toml
   ,----
 14 | gate_type = "comiple"
   :               ^^^^^^^^ unknown gate type
   `----
  help: Did you mean "compile"? Valid gate types: compile, test, clippy,
        diff, lint, format, custom
```

### Implementation

```toml
# Cargo.toml (workspace)
miette = { version = "7", features = ["fancy"] }
```

For user-facing errors (config parsing, plan validation, gate failures), wrap in miette diagnostics:

```rust
use miette::{Diagnostic, SourceSpan};

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("invalid gate configuration")]
#[diagnostic(code(roko::config::invalid_gate), help("Valid gate types: compile, test, clippy, diff, lint, format, custom"))]
struct InvalidGateConfig {
    #[source_code]
    src: String,
    #[label("unknown gate type")]
    span: SourceSpan,
}
```

### Where To Apply (highest value)

1. **Config parsing** (`roko.toml`) — show the exact line with the error
2. **Plan/task TOML parsing** — show which task definition is malformed
3. **Gate failures** — show what the gate expected vs what it got, with a help suggestion
4. **Missing dependencies** — "roko-agent requires `claude` CLI. Install: brew install claude"
5. **Permission errors** — "Cannot write to .roko/. Check directory permissions."

### Migration Path

miette is compatible with anyhow. You don't need to replace anyhow everywhere — just wrap user-facing errors at the CLI boundary:

```rust
fn main() -> miette::Result<()> {
    // Inner code still uses anyhow
    // Convert at the boundary
    run().map_err(|e| miette::miette!("{e:#}"))?;
    Ok(())
}
```

---

## 6. `NO_COLOR` / `CLICOLOR` Standards Compliance

### What

Two environment variable standards that all modern CLIs should respect:

- `NO_COLOR` (https://no-color.org/) — when set to any value, disable all ANSI color/styling
- `CLICOLOR=0` — disable colors
- `CLICOLOR_FORCE=1` — force colors even when not a terminal (for CI log viewers)

### Implementation

Check at startup in `main()`:

```rust
fn should_use_color() -> bool {
    // NO_COLOR takes absolute priority (https://no-color.org/)
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    // CLICOLOR_FORCE=1 forces color even in non-TTY
    if std::env::var("CLICOLOR_FORCE").ok().as_deref() == Some("1") {
        return true;
    }
    // CLICOLOR=0 disables color
    if std::env::var("CLICOLOR").ok().as_deref() == Some("0") {
        return false;
    }
    // Default: color if stdout is a terminal
    std::io::stdout().is_terminal()
}
```

Apply globally via a `--color=auto|always|never` flag (add `colorchoice-clap` for integration with clap's built-in color handling):

```toml
colorchoice-clap = "1"
```

```rust
#[derive(Debug, Parser)]
#[command(name = "roko", version, about)]
struct Cli {
    #[command(flatten)]
    color: colorchoice_clap::Color,
    // ... rest of fields
}
```

This also makes clap's own `--help` output respect the color choice.

---

## 7. Styled Help Output

### Current State

clap v4 enables styled help by default (bold headers, underlined sections). However, there's no custom branding or help organization beyond what clap provides.

### Enhancements

#### 7a. Custom help template with grouped commands

Instead of a flat list of 35+ subcommands, group them by category in `--help` output:

```rust
#[derive(Debug, Parser)]
#[command(
    name = "roko",
    version,
    about = "Toolkit for agents that build themselves",
    // Custom help template with categories
    help_template = "\
{before-help}{name} {version}
{about-with-newline}
{usage-heading} {usage}

Core:
  run           Seed a prompt and run the universal loop
  status        Print signal counts and gate results
  dashboard     Launch the interactive TUI
  chat          Interactive chat with an agent

Planning:
  plan          Manage plans (list, show, create, run)
  prd           Manage PRDs (idea, draft, publish, plan)
  research      Research topics and enhance documents

Learning:
  learn         Show learning subsystem state
  tune          Tune thresholds and routing
  experiment    Manage model experiments
  explain       Explain a roko concept

Infrastructure:
  init          Create .roko/ and default config
  config        Manage configuration
  serve         Start HTTP API server
  daemon        Manage daemon mode
  deploy        Cloud deployment targets

Advanced:
  agent         Manage agent runtimes
  neuro         Search durable knowledge
  index         Code intelligence
  archive       Cold storage management
  plugin        Manage plugins
  new           Generate scaffolding

{all-args}{after-help}"
)]
struct Cli { /* ... */ }
```

#### 7b. Subcommand aliases in help

Show aliases inline so users discover shortcuts:

```
Core:
  run (do)          Seed a prompt and run the universal loop
  dashboard (watch) Launch the interactive TUI
```

#### 7c. Examples in help

Add `#[command(after_help = "...")]` with usage examples for key subcommands:

```rust
/// Execute plans (the main orchestration loop).
#[command(after_help = "\
Examples:
  roko plan run plans/              Run all plans in directory
  roko plan run plans/ --resume     Resume from last checkpoint
  roko plan run plans/ --no-replan  Disable automatic replanning
")]
Run { /* ... */ }
```

---

## 8. OSC 8 Terminal Hyperlinks

### What

Modern terminals (iTerm2, Kitty, WezTerm, Windows Terminal, Ghostty, Alacritty 0.14+) support clickable hyperlinks via the OSC 8 escape sequence. When roko outputs a file path, it can be clickable — clicking opens the file in the user's editor.

### Where To Use

- **Gate failure output** — click to open the file that failed compilation/linting
- **Status output** — click to open `.roko/episodes.jsonl`, plan files, etc.
- **Error messages** — click to open the config file with the error
- **`roko plan show`** — click task file paths

### Implementation

```rust
/// Wrap text in an OSC 8 hyperlink if the terminal supports it.
fn hyperlink(url: &str, text: &str) -> String {
    if should_use_hyperlinks() {
        format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
    } else {
        text.to_string()
    }
}

fn file_link(path: &std::path::Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let url = format!("file://{}", abs.display());
    let display = abs.display().to_string();
    hyperlink(&url, &display)
}

fn should_use_hyperlinks() -> bool {
    // Detect terminal support
    // TERM_PROGRAM is set by most modern terminals
    let term = std::env::var("TERM_PROGRAM").unwrap_or_default();
    matches!(
        term.as_str(),
        "iTerm.app" | "WezTerm" | "ghostty" | "vscode"
    ) || std::env::var("KONSOLE_VERSION").is_ok()
}
```

Optionally use the `anstyle-hyperlink` or `terminal-link` crate instead of hand-rolling.

---

## 9. Progress Indicators

### Current State

The CLI likely uses basic `println!` for progress. The workspace doesn't currently depend on `indicatif`.

### What To Add

`indicatif` is the standard Rust crate for terminal progress bars and spinners.

```toml
# crates/roko-cli/Cargo.toml
indicatif = "0.17"
```

#### 9a. Spinners for indeterminate work

Use for operations where you don't know how long they'll take:

- Agent dispatch (waiting for Claude response)
- Research enhancement
- PRD generation
- Plan generation

```rust
use indicatif::{ProgressBar, ProgressStyle};

let spinner = ProgressBar::new_spinner();
spinner.set_style(
    ProgressStyle::default_spinner()
        .template("{spinner:.cyan} {msg}")
        .unwrap()
);
spinner.set_message("Generating plan from PRD...");
spinner.enable_steady_tick(std::time::Duration::from_millis(80));

// ... do work ...

spinner.finish_with_message("Plan generated (12 tasks)");
```

#### 9b. Progress bars for batch operations

Use for operations with known item counts:

- `roko plan run` — X of Y tasks completed
- `roko archive` — archiving N engrams
- `roko index build` — indexing N files
- `roko prd consolidate` — processing N PRDs

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

let multi = MultiProgress::new();
let overall = multi.add(ProgressBar::new(total_tasks as u64));
overall.set_style(
    ProgressStyle::default_bar()
        .template("{prefix:.bold} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("=> ")
);
overall.set_prefix("Tasks");

for task in tasks {
    overall.set_message(format!("{}", task.name));
    // ... run task ...
    overall.inc(1);
}
overall.finish_with_message("done");
```

#### 9c. Multi-progress for parallel agents

When multiple agents run in parallel during `roko plan run`:

```
Tasks  [===============>                        ] 5/12 wire-gate-pipeline
  agent-1  ⣾ Running compile gate...
  agent-2  ⣾ Generating implementation...
  agent-3  ✓ Completed (2.3s)
```

The `MultiProgress` struct from indicatif supports this natively.

#### 9d. Respect `--quiet` and non-TTY

All progress indicators should check:
- `cli.quiet` flag — suppress all progress output
- `!std::io::stderr().is_terminal()` — disable spinners/bars in pipes/CI (just print line-by-line updates)

```rust
fn make_progress(quiet: bool, total: u64) -> ProgressBar {
    if quiet || !std::io::stderr().is_terminal() {
        ProgressBar::hidden()  // no-op, zero overhead
    } else {
        let pb = ProgressBar::new(total);
        pb.set_style(/* ... */);
        pb
    }
}
```

---

## 10. JSON Output Parity

### Current State

The `--json` global flag exists but not all commands may respect it consistently.

### What To Ensure

Every command that produces output should check `cli.json` and emit structured JSON when set. This enables:

- Piping to `jq` for scripting
- Integration with external dashboards
- Machine-readable CI output
- IDE integrations

### Commands that must support `--json`

| Command | JSON output shape |
|---|---|
| `roko status` | `{ signals: N, episodes: N, gates: { pass: N, fail: N }, ... }` |
| `roko plan list` | `[{ id, name, status, tasks: N, completed: N }]` |
| `roko plan show <id>` | `{ id, name, tasks: [{ id, name, status, agent, ... }] }` |
| `roko prd list` | `[{ slug, title, status, created_at }]` |
| `roko prd status` | `{ total, draft, published, planned, coverage: 0.75 }` |
| `roko learn` | `{ router: {...}, experiments: {...}, efficiency: {...} }` |
| `roko config show` | The full resolved config as JSON |
| `roko explain <topic>` | `{ topic, depth, content: "..." }` |
| `roko plan run` (events) | NDJSON stream of task events: `{ event, task_id, status, ... }` |

### Implementation Pattern

```rust
if cli.json {
    let output = serde_json::to_string_pretty(&data)?;
    println!("{output}");
} else {
    // Human-readable output
    print_human_readable(&data);
}
```

For streaming commands (`plan run`), use NDJSON (newline-delimited JSON):

```rust
if cli.json {
    let line = serde_json::to_string(&event)?;
    println!("{line}");  // one JSON object per line, no pretty-printing
}
```

---

## 11. Self-Update Improvements

### Current State

`roko update` exists with a `--verify` flag for Sigstore/cosign verification.

### What To Add

#### 11a. Version check on startup

Asynchronously check for new versions without blocking the CLI:

```rust
// In main(), spawn a background check
if !cli.quiet && std::io::stderr().is_terminal() {
    tokio::spawn(async {
        if let Some(new_version) = check_for_update().await {
            eprintln!(
                "\n{} Update available: {} -> {} (run `roko update`)",
                "!".yellow(), env!("CARGO_PKG_VERSION"), new_version
            );
        }
    });
}
```

Cache the result to avoid checking on every invocation:

```rust
// Cache in ~/.roko/update-check.json
// { "last_check": "2026-04-21T10:00:00Z", "latest": "0.5.2" }
// Only check once per 24 hours
```

#### 11b. Changelog display

After updating, show what changed:

```
$ roko update
Updating roko v0.5.1 -> v0.5.2...
Done!

What's new in v0.5.2:
  - Fixed gate threshold persistence bug
  - Added PowerShell completions
  - Improved plan run progress display
```

---

## 12. Command Timing

### Concept

Show how long commands take, similar to how `cargo build` shows "Finished in 2.3s". Useful for understanding performance of long-running operations.

### Implementation

```rust
use std::time::Instant;

let start = Instant::now();
let exit_code = run_command(&cli).await?;
let elapsed = start.elapsed();

// Only show timing for commands that take > 1 second
if elapsed.as_secs() >= 1 && !cli.quiet && !cli.json {
    eprintln!("\nCompleted in {}", format_duration(elapsed));
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{:.1}s", d.as_secs_f64())
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
```

---

## 13. Confirmation Prompts for Destructive Operations

### Concept

Before destructive actions, prompt the user for confirmation (unless `--yes` / `-y` is passed).

### Where To Apply

| Command | Destructive action |
|---|---|
| `roko archive` | Moves engrams to cold storage |
| `roko plan run --resume` | Potentially replays already-completed tasks |
| `roko config set` | Changes configuration |
| `roko plugin remove` | Removes a plugin |
| `roko prd draft promote` | Promotes to published (can't undo) |

### Implementation

```rust
use dialoguer::Confirm;

// Add global --yes flag
#[arg(long, short = 'y', global = true)]
yes: bool,

// In handler
if !cli.yes {
    let confirmed = Confirm::new()
        .with_prompt(format!("Archive {} engrams older than {}?", count, older_than))
        .default(false)
        .interact()?;
    if !confirmed {
        eprintln!("Aborted.");
        return Ok(EXIT_SUCCESS);
    }
}
```

---

## 14. Contextual Suggestions on Error

### Concept

When a command fails or the user types something wrong, suggest what they probably meant. Similar to `git`'s "did you mean" suggestions.

### Implementation

Use string similarity (Levenshtein distance) to suggest corrections:

```rust
fn suggest_subcommand(input: &str, commands: &[&str]) -> Option<String> {
    commands
        .iter()
        .map(|cmd| (cmd, strsim::levenshtein(input, cmd)))
        .filter(|(_, dist)| *dist <= 3)
        .min_by_key(|(_, dist)| *dist)
        .map(|(cmd, _)| cmd.to_string())
}
```

```toml
strsim = "0.11"
```

Example output:

```
$ roko plna run plans/
error: unrecognized subcommand 'plna'

  Did you mean 'plan'?

  Usage: roko plan run <dir>
```

Also suggest related commands when something fails:

```
$ roko plan run plans/
error: no plans found in plans/

  Tip: Create a plan first:
    roko prd plan <slug>    Generate plan from a PRD
    roko plan create        Create a plan manually
```

---

## 15. Dry-Run Mode

### Current State

`roko archive --dry-run` exists. But other commands that make changes don't have dry-run.

### What To Add

Add `--dry-run` to commands that modify state:

| Command | What dry-run shows |
|---|---|
| `roko plan run` | Prints the execution plan (task order, agents, gates) without running |
| `roko prd plan` | Shows what the generated plan would look like without saving |
| `roko tune` | Shows what thresholds would change without writing |
| `roko config set` | Shows the diff in config without writing |
| `roko plugin install` | Shows what would be installed without installing |

This is especially valuable for `roko plan run --dry-run` which lets users preview the execution graph before committing to a potentially long-running operation.

---

## 16. `roko doctor` — Environment Diagnostics

### Concept

A single command that checks the user's environment and reports issues. Similar to `brew doctor`, `flutter doctor`, or `rustup check`.

### What It Checks

```
$ roko doctor

Roko v0.5.1 environment check:

  [ok] Rust toolchain: 1.91.0 (minimum: 1.91.0)
  [ok] roko.toml: found at ./roko.toml
  [ok] .roko/ directory: exists, writable
  [ok] Claude CLI: installed (v1.2.3)
  [!!] MCP config: agent.mcp_config not set in roko.toml
  [ok] Git: v2.45.0
  [ok] Shell completions: installed (zsh)
  [--] Nushell completions: not installed
  [ok] Episode log: 1,247 entries
  [ok] Signal log: 3,891 entries
  [!!] Disk usage: .roko/ is 2.3 GB (consider `roko archive`)
  [ok] Latest version: v0.5.1 (up to date)

2 warnings. Run with --fix to auto-fix where possible.
```

### Auto-Fix

`roko doctor --fix` attempts to fix detected issues:

- Creates missing `.roko/` directories
- Initializes missing config files with defaults
- Installs shell completions
- Runs `roko archive` if disk usage is high

---

## 17. `roko --version` Improvements

### Current State

Standard clap version output: `roko 0.5.1`

### Enhanced Version

Show build metadata for debugging:

```
$ roko --version
roko 0.5.1 (abc1234 2026-04-21)
  rustc: 1.91.0
  target: aarch64-apple-darwin
  features: tui,serve,mcp
  profile: release
```

Implementation via clap's `#[command(long_version)]`:

```rust
#[command(
    version,
    long_version = long_version(),
)]
struct Cli { /* ... */ }

fn long_version() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        " (", env!("VERGEN_GIT_SHA_SHORT", "unknown"), " ", env!("VERGEN_BUILD_DATE", "unknown"), ")\n",
        "  rustc: ", env!("VERGEN_RUSTC_SEMVER", "unknown"), "\n",
        "  target: ", env!("VERGEN_CARGO_TARGET_TRIPLE", "unknown"),
    )
}
```

Use the `vergen` crate to embed git/build metadata at compile time:

```toml
[build-dependencies]
vergen = { version = "9", features = ["build", "cargo", "git", "rustc"] }
```

---

## 18. XDG Config Directory Support

### Current State

Config lives at `./roko.toml` (project-local) and `.roko/` (project data).

### What To Add

Support a global user config at the XDG standard location:

```
~/.config/roko/config.toml      # Global defaults (XDG_CONFIG_HOME)
./roko.toml                      # Project overrides (existing)
```

Resolution order (later overrides earlier):

1. Compiled-in defaults
2. `~/.config/roko/config.toml` (global user defaults)
3. `./roko.toml` (project-specific)
4. Environment variables (`ROKO_MODEL=claude-opus-4-6`)
5. CLI flags (`--model claude-opus-4-6`)

```toml
# crates/roko-cli/Cargo.toml
dirs = "6"  # or xdg = "3"
```

```rust
fn global_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("roko")
        .join("config.toml")
}
```

---

## 19. Environment Variable Overrides

### Concept

Allow any config value to be overridden via environment variables. Standard 12-factor app practice.

### Mapping

```
ROKO_MODEL=claude-opus-4-6       → agent.default_model
ROKO_EFFORT=high                  → agent.effort
ROKO_ROLE=architect               → agent.role
ROKO_BACKEND=claude-api           → agent.backend
ROKO_MCP_CONFIG=/path/to/mcp.json → agent.mcp_config
ROKO_LOG_FORMAT=json              → log_format
ROKO_NO_REPLAN=1                  → no_replan
ROKO_QUIET=1                      → quiet
```

Implementation: check env vars in config resolution, between global config and CLI flags:

```rust
fn resolve_model(cli: &Cli, config: &Config) -> String {
    cli.model.clone()
        .or_else(|| std::env::var("ROKO_MODEL").ok())
        .or_else(|| config.agent.default_model.clone())
        .unwrap_or_else(|| "claude-sonnet-4-6".to_string())
}
```

---

## 20. TUI Enhancements

### Current State

The TUI uses ratatui with F1-F7 tabs, file watching, high-contrast and reduced-motion modes.

### What To Add

#### 20a. Command palette (Ctrl+P / Ctrl+K)

A fuzzy-search command palette (like VS Code's Ctrl+Shift+P) that lets users quickly jump to any TUI action:

```
┌─ Command Palette ──────────────────┐
│ > run plan                         │
│                                    │
│   Run Plan          (F3)           │
│   Show Plan List    (F2)           │
│   Create Plan                      │
│   Plan Status                      │
└────────────────────────────────────┘
```

Uses tui-input or tui-textarea for the search field, and fuzzy matching with the `fuzzy-matcher` crate.

#### 20b. Notification toasts

When background events happen (gate failure, agent completion, new signal), show a brief toast notification in the TUI corner:

```
                           ┌──────────────────────┐
                           │ ✓ Task 3 completed   │
                           │   wire-gate-pipeline  │
                           └──────────────────────┘
```

Auto-dismiss after 3-5 seconds.

#### 20c. Keybinding help overlay (Shift+?)

Show all available keybindings for the current tab:

```
┌─ Keybindings ──────────────────────────────┐
│ F1-F7     Switch tabs                      │
│ q / Esc   Quit                             │
│ Enter     Select / expand                  │
│ /         Search                           │
│ r         Refresh                          │
│ Ctrl+P    Command palette                  │
│ ?         This help                        │
│                                            │
│ Press any key to dismiss                   │
└────────────────────────────────────────────┘
```

#### 20d. Mouse support

ratatui + crossterm support mouse events. Add:

- Click to select items in lists
- Scroll wheel for scrolling
- Click on tab bar to switch tabs

#### 20e. Clipboard integration

Copy signal hashes, agent output, error messages to clipboard via Ctrl+C on selected text.

```toml
# crates/roko-cli/Cargo.toml
arboard = "3"  # Cross-platform clipboard
```

#### 20f. Theme customization

Let users configure TUI colors via config:

```toml
# roko.toml
[tui.theme]
primary = "#7C3AED"    # Purple
success = "#10B981"    # Green
error = "#EF4444"      # Red
warning = "#F59E0B"    # Amber
background = "#1E1E2E" # Dark
```

---

## 21. `roko alias` — Custom Command Aliases

### Concept

Let users define custom command aliases in their config:

```toml
# roko.toml or ~/.config/roko/config.toml
[aliases]
pr = "plan run"
ps = "prd status"
qs = "status --quiet"
deploy-prod = "deploy fly --env production"
```

Then:

```bash
$ roko pr plans/    # expands to: roko plan run plans/
$ roko ps           # expands to: roko prd status
```

### Implementation

Before clap parses args, check if the first positional arg matches an alias:

```rust
fn expand_aliases(args: &mut Vec<String>, config: &Config) {
    if args.len() < 2 { return; }
    if let Some(expansion) = config.aliases.get(&args[1]) {
        let expanded: Vec<String> = expansion.split_whitespace().map(String::from).collect();
        args.splice(1..2, expanded);
    }
}
```

---

## 22. Structured Logging with `tracing`

### Current State

Unknown whether the CLI uses `tracing` or `log` or raw `eprintln!`.

### What To Add

If not already using `tracing`, migrate to it for structured, filterable logging:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "fmt"] }
```

```rust
// In main()
let filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "roko=info".parse().unwrap());

match cli.log_format {
    LogFormat::Json => {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    }
    LogFormat::Text => {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
    }
}
```

Benefits:

- `RUST_LOG=roko_agent=debug roko plan run` — debug just the agent crate
- `--log-format json` — machine-parseable logs for CI/dashboards
- Spans provide context: which task, which agent, which gate

---

## 23. Shell Hook: Auto-Activate on `cd`

### Concept

Like `nvm`, `direnv`, or `mise` — automatically show project status or activate the roko environment when the user `cd`s into a directory containing `roko.toml`.

### Implementation (part of `roko shell-init`)

```bash
# Generated by `roko shell-init zsh`
_roko_hook() {
  if [[ -f "./roko.toml" ]] && [[ "$_ROKO_LAST_DIR" != "$PWD" ]]; then
    _ROKO_LAST_DIR="$PWD"
    # Show a brief status line
    local status=$(roko status --quiet --json 2>/dev/null)
    if [[ -n "$status" ]]; then
      local tasks=$(echo "$status" | jq -r '.tasks // empty')
      local done=$(echo "$status" | jq -r '.completed // empty')
      if [[ -n "$tasks" ]]; then
        echo "roko: ${done}/${tasks} tasks completed"
      fi
    fi
  fi
}
chpwd_functions+=(_roko_hook)
```

Make this opt-in via config:

```toml
# ~/.config/roko/config.toml
[shell]
auto_status = true   # Show status on cd
```

---

## 24. Performance: Startup Time

### Concern

CLIs should start in <100ms. With 35+ subcommands and dynamic completions, startup can get slow.

### Mitigations

1. **Lazy initialization**: Don't load config, scan filesystem, or connect to services until the specific command needs it
2. **Feature flags**: Gate heavy deps behind cargo features so basic commands don't pull in TUI, HTTP server, etc.
3. **Measure**: Add `--time` hidden flag that prints startup timing breakdown

```rust
#[cfg(debug_assertions)]
{
    let startup = start.elapsed();
    if std::env::var_os("ROKO_TIMING").is_some() {
        eprintln!("[timing] startup: {:?}", startup);
    }
}
```

---

## 25. Carapace Integration

### What

Carapace (https://carapace.sh) is a universal multi-shell completion framework. By providing a carapace spec, roko completions work across any shell carapace supports (30+ shells including exotic ones like oil, xonsh, elvish, etc.).

### Implementation

Export a YAML or JSON spec:

```bash
roko completions --carapace > ~/.config/carapace/specs/roko.yaml
```

Or register as a carapace bridge:

```yaml
# roko.yaml (carapace spec)
name: roko
description: Toolkit for agents that build themselves
commands:
  - name: plan
    description: Manage plans
    commands:
      - name: run
        description: Execute plans
        flags:
          --resume: Resume from checkpoint
        # Dynamic completions
        completion:
          positional:
            - ["$files([plans/])"]
```

This is lower priority than native completions but provides maximum shell coverage.

---

## Summary: Implementation Priority

### Phase 1 — Quick wins (1-2 days each)

1. **`roko shell-init <shell>`** — unified eval integration (Section 2)
2. **`NO_COLOR` / `CLICOLOR` compliance** (Section 6)
3. **Command timing** (Section 12)
4. **Enhanced `--version`** with build metadata (Section 17)
5. **PowerShell + Nushell completions** (Section 1a)
6. **Flag completions** in existing scripts (Section 1c)

### Phase 2 — Medium effort, high value (2-5 days each)

7. **Interactive fuzzy fallbacks** with `dialoguer` (Section 3)
8. **Progress indicators** with `indicatif` (Section 9)
9. **Grouped/styled help** (Section 7)
10. **Contextual error suggestions** (Section 14)
11. **`roko doctor`** environment check (Section 16)
12. **Dry-run mode** for state-changing commands (Section 15)
13. **Deeper dynamic completions** (Section 1b)

### Phase 3 — Polish and advanced (3-7 days each)

14. **Rich error diagnostics** with `miette` (Section 5)
15. **Man page generation** (Section 4)
16. **OSC 8 hyperlinks** (Section 8)
17. **TUI enhancements** — command palette, toasts, mouse, themes (Section 20)
18. **Custom aliases** (Section 21)
19. **XDG config** (Section 18)
20. **Environment variable overrides** (Section 19)
21. **Shell hooks** — auto-status on cd (Section 23)
22. **Carapace spec** (Section 25)

### New Dependencies

| Crate | Version | Purpose | Size impact |
|---|---|---|---|
| `clap_complete` | 4 | PowerShell/Nushell completions | Minimal (compile-time) |
| `clap_mangen` | 0.2 | Man page generation | Build-only |
| `colorchoice-clap` | 1 | `--color` flag integration | Tiny |
| `dialoguer` | 0.11 | Interactive prompts + fuzzy select | ~50KB |
| `indicatif` | 0.17 | Progress bars + spinners | ~80KB |
| `miette` | 7 | Rich error diagnostics | ~200KB |
| `strsim` | 0.11 | "Did you mean?" suggestions | ~10KB |
| `vergen` | 9 | Build metadata embedding | Build-only |
| `dirs` | 6 | XDG directory lookup | ~20KB |
| `arboard` | 3 | Clipboard (TUI) | ~100KB |

All are well-maintained, widely-used crates in the Rust ecosystem.











  How to test the dashboard

  # Terminal 1: Start roko-serve
  cd /Users/will/dev/nunchi/roko/roko
  cargo run -p roko-cli -- serve
  # Starts on http://localhost:6677

  # Terminal 2: Seed some test agents
  curl -s -X POST http://localhost:6677/api/agents/register \
    -H 'Content-Type: application/json' \
    -d '{"agent_id":"agent-rustsmith","label":"rustsmith","sk
  ills":["rust","p2p","eth"],"tier":"Expert","reputation":94,
  "past_jobs_completed":37,"max_concurrent_jobs":5}'

  curl -s -X POST http://localhost:6677/api/agents/register \
    -H 'Content-Type: application/json' \
    -d '{"agent_id":"agent-ethdev","label":"ethdev","skills":
  ["solidity","eth","defi"],"tier":"Trusted","reputation":82,
  "past_jobs_completed":21,"max_concurrent_jobs":3}'

  # Verify match endpoint works
  curl -s -X POST http://localhost:6677/api/jobs/match \
    -H 'Content-Type: application/json' \
    -d '{"title":"build
  relay","skills":["rust"],"reward":"2500 KORAI"}' | jq .

  # Terminal 3: Start dashboard (branch wp-demo-dashboard)
  cd /Users/will/dev/nunchi/nunchi-dashboard
  git checkout wp-demo-dashboard
  npm install
  npm run dev
  # Opens on http://localhost:5173 (Vite default)

  The dashboard's .env already has
  VITE_ROKO_URL=http://127.0.0.1:6677. The Vite dev proxy
  will forward /roko-api/* → http://localhost:6677/api/*.

  To test the matchmaking flow: open the Atelier tab, type
  /coding implement walrus gateway relay in the chat. The
  dashboard calls POST /api/jobs/match, shows the agent quote
   bubble with candidates, and lets you Accept/Deny.


  2. roko run via API tool loop

  ANTHROPIC_API_KEY=sk-ant-... roko run "what is
  2+2"
  # Should print: [roko] using Anthropic API
  (claude-sonnet-4-6)
  # Then per-turn tool output if the model calls
  tools

  Without the key it falls back to CLI:
  unset ANTHROPIC_API_KEY && roko run "what is 2+2"
  # Uses Claude CLI subprocess as before

  3. Dashboard / verdicts panic fix

  roko dashboard
  # Navigate to the gates tab (F5 or whichever has
  verdicts)
  # Previously panicked with "Cannot start a
  runtime from within a runtime"

  4. Via the sidecar you already have running

  # Send a message to the agent sidecar
  curl -s http://127.0.0.1:8081/message \
    -H 'Content-Type: application/json' \
    -d '{"content": "what is 2+2"}' | jq .

  The sidecar uses a different code path (it
  already had the API tool loop), so this tests the
   existing wiring rather than the new roko run
  changes specifically.

  Fastest single test

  The workspace fix is the most immediate to verify
   — just cd .roko && roko status. For the API
  path, the ANTHROPIC_API_KEY=... roko run "what is
   2+2" command is the key one.