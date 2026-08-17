# 09: Unified CLI UX — Stop Making New Commands

## Problem

The CLI has **42 top-level commands** plus ~100 sub-subcommands. Many overlap or are
indistinguishable to a user.

### Complete Current Command Tree

**Top-level (42 commands):**

```
roko
  Core workflow:
    init              Create .roko/ and roko.toml
    do                Task from prompt (progressive formality: trivial/simple/standard/complex)
    run               Universal loop (compose → agent → gate → persist)
    status            Signal counts, latest episode, gate pass/fail
    show              Inspect workspace state (costs, agents, knowledge, plans, learning)
    doctor            Diagnose workspace bootstrap state
    layer-check       Check workspace layer dependency rules

  Planning & PRDs:
    plan              (7 subcommands: list, show, create, validate, run, generate, regenerate)
    prd               (6 subcommands: idea, list, status, draft[4], plan, consolidate)

  Agents:
    agent             (8 subcommands: create, delete, list, start, stop, status, serve, chat)

  Research:
    research          (7 subcommands: topic, enhance-prd, enhance-plan, enhance-tasks, analyze, list, search)
    think             Read-only research without execution

  Tuning:
    tune              (4 subcommands: routing, gates, budget, model)

  Knowledge:
    knowledge         (9 subcommands: query, stats, gc, backup, restore, sync,
                       dream[5], custody[3], archive)

  Learning:
    learn             (6 subcommands: all, route, experiments, efficiency, episodes, tune)

  Jobs:
    job               (6 subcommands: list, create, match, show, execute, cancel)

  Benchmarks:
    bench             (2 subcommands: demo, swe)

  Demos:
    demo              (2 subcommands: setup, warm)

  Configuration:
    config            (19 subcommands: init, show, path, doctor, edit, set, set-secret,
                       check-secrets, validate, migrate, export,
                       providers[4], models[2], subscriptions[5], events,
                       experiments, plugins[4], secrets, mcp[3])

  Code intelligence:
    index             (4 subcommands: build, rebuild, search, stats)

  Graph execution:
    graph             (3 subcommands: run, validate, show)

  Data feeds:
    isfr              (3 subcommands: start, status, sources)
    feed              (2 subcommands: list, status)

  Server & deployment:
    dev               Start serve + demo frontend
    up                Start serve + all agents
    serve             HTTP API server
    acp               ACP JSON-RPC server
    daemon            (8 subcommands: start, stop, status, logs, reload, restart, install, uninstall)
    deploy            (3 subcommands: railway, fly, docker)
    worker            Run as deployed worker

  Interactive:
    dashboard         Launch ratatui TUI

  Authentication:
    login             Authenticate
    logout            Remove credentials
    whoami            Show auth status

  Utilities:
    vision-loop       Iterative vision-guided UI refinement
    resume            Resume plan from last checkpoint
    replay            Walk signal DAG by hash
    history           List/show past chat session summaries
    inject            Inject signal into running session
    completions       Shell completion scripts
    new               Generate boilerplate
    explain           Explain a roko concept
```

**Total: 42 top-level + ~100 sub-subcommands = ~142 addressable commands.**

### Confusion Matrix

| Command | What it does | Confused with |
|---------|-------------|----------------|
| `roko do "..."` | Universal entry point — classify + execute | `run`, `plan run` |
| `roko run "..."` | Universal loop (compose→agent→gate→persist) | `do` |
| `roko plan run <dir>` | Execute a task plan | `do --complexity medium` |
| `roko prd plan <slug>` | Generate tasks from PRD | `plan generate` |
| `roko plan generate` | Also generates plans | `prd plan` |
| `roko prd idea "..."` | Capture idea | `do`? `think`? |
| `roko prd draft new` | Draft a PRD | When vs `do`? |
| `roko think "..."` | Research without acting | `research topic`? |
| `roko research topic` | Also research | `think`? |
| `roko tune` | Write roko.toml | `config set`? |
| `roko learn tune` | Also tune thresholds | `tune`? |
| `roko show` | View workspace state | `status`? `dashboard`? |
| `roko status` | Also view workspace state | `show`? |

## Core Insight

Users have exactly **3 intents**:

1. **"Remember this"** → capture an idea/thought for later
2. **"Figure this out"** → plan/research/break down a problem
3. **"Do this"** → execute something (with whatever planning is needed)

Everything else is plumbing that should be automatic or hidden behind flags.

## Proposal: 3 Primary Verbs

### `roko note` — capture (replaces `prd idea`)

Status: **NOT YET IMPLEMENTED.** No `note` command, no `NoteCmd`, no `cmd_note` exists in
the codebase as of the current branch.

```bash
roko note "cursor composer 2 support — spawn agent subprocess, JSON-RPC over stdio"
roko note "feed agents should publish to relay topics"
roko note "knowledge store should inform model routing"
```

Append-only. No LLM call. Instant. Captures timestamped thoughts into `.roko/notes/`.

Optionally tag:
```bash
roko note --tag cursor "ACP protocol uses session/new + session/prompt"
roko note --tag feeds "agents should subscribe to each other via relay"
```

Implementation: trivial wrapper over `prd::cmd_idea` with tag support added to the storage
format. Notes live at `.roko/notes/<timestamp>-<slug>.md` (separate from PRD ideas).

### `roko plan` — synthesize + break down

Status: **PARTIALLY IMPLEMENTED.** `roko plan` exists with subcommands `list`, `show`,
`create`, `validate`, `run`, `generate`, `regenerate`. The design here adds new behavior
to `roko plan` when called **without** a subcommand (just with a prompt string):

```bash
# From a specific topic/prompt (NEW behavior — currently requires prd + plan subcommands):
roko plan "implement cursor composer 2 support"

# From accumulated notes (NEW — synthesis from notes):
roko plan --from-notes
roko plan --from-notes --tag cursor

# Existing subcommands remain:
roko plan list
roko plan show cursor-composer-2
roko plan run .roko/plans/cursor-composer-2/
```

What `roko plan "prompt"` does internally (delegating to existing code):
1. Calls `repo_context::build_repo_context` (already exists)
2. Calls `plan_generate::build_generation_prompt` (already exists)
3. Dispatches strategist agent via `run_agent_capture_silent` (already exists)
4. Shows plan approval screen (new — same as `roko develop`)
5. Saves to `.roko/plans/<slug>/tasks.toml`

`roko plan --from-notes` is the new piece:
1. Reads all notes from `.roko/notes/` (or filtered by `--tag`)
2. Clusters related notes by keyword similarity
3. For each cluster, generates a separate plan
4. Shows interactive selection: generate all? pick one?

### `roko do` — execute (already implemented, refine behavior)

Status: **FULLY IMPLEMENTED.** Located at
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/do_cmd.rs`.

The design refinement: `roko do <slug>` should look for an existing plan by name before
treating the argument as a prompt. Currently `roko do` only accepts prompt strings;
executing an existing plan requires `roko plan run .roko/plans/<slug>/`.

```bash
# NEW: Execute a plan by name (looks up .roko/plans/cursor-composer-2/)
roko do cursor-composer-2

# Existing: Execute from prompt (auto-classifies complexity)
roko do "implement cursor support"

# Existing: Simple one-shot tasks (no plan needed)
roko do "fix the login bug"
```

The key change: detect whether the first argument is a plan slug or a prompt. If it matches
a directory under `.roko/plans/`, run that plan. Otherwise, treat as a prompt and classify.

## Command Surface (proposed redesign)

### Primary (daily use — 3 commands)

| Command | Intent | Example | Status |
|---------|--------|---------|--------|
| `roko note` | Remember | `roko note "cursor ACP uses JSON-RPC"` | NOT IMPLEMENTED |
| `roko plan "..."` | Figure out | `roko plan "implement cursor support"` | PARTIAL (subcommands exist) |
| `roko do` | Execute | `roko do cursor-composer-2` | IMPLEMENTED (prompt-only) |
| `roko develop` | Plan+do | `roko develop "implement X"` | NOT IMPLEMENTED (wrapper) |

### Secondary (monitoring — already exist)

| Command | Intent | Status |
|---------|--------|--------|
| `roko status` | What's happening? | IMPLEMENTED |
| `roko show` | Detailed state | IMPLEMENTED |
| `roko dashboard` | Watch it work (TUI) | IMPLEMENTED |

### Configuration (occasional — already exist)

| Command | Intent | Status |
|---------|--------|--------|
| `roko init` | First-time config | IMPLEMENTED |
| `roko config init` | Interactive wizard | IMPLEMENTED |
| `roko tune` | Tune routing/gates/budget | IMPLEMENTED |
| `roko config` | Everything else | IMPLEMENTED |

### Power user (rare — already exist)

| Command | Intent |
|---------|--------|
| `roko serve` | HTTP API |
| `roko agent` | Agent management |
| `roko knowledge` | Knowledge store |
| `roko research` | Deep research |
| `roko learn` | Inspect learning state |
| `roko bench` | Benchmarks |
| `roko deploy` | Cloud deployment |
| `roko daemon` | Background service |

## Current Overlap Map

Commands that do substantially the same thing and should be consolidated:

```
roko do "prompt"            ←→  roko run "prompt"
  WINNER: roko do (already has complexity routing)
  ACTION: deprecate run, print hint

roko prd idea "text"        ←→  roko note "text"  (proposed)
  WINNER: roko note (simpler, no baggage)
  ACTION: note wraps prd idea; prd idea prints hint

roko prd plan <slug>        ←→  roko plan generate --from-file <prd>
  WINNER: merge into roko plan "prompt" / roko plan --from-prd <slug>
  ACTION: keep prd plan for backward compat; add plan --from-prd

roko think "question"       ←→  roko research topic "question"
  WINNER: roko think (simpler name, local-only)
  roko research (network-enabled, Perplexity)
  ACTION: keep both, clarify think=local research=external

roko tune <subsystem>       ←→  roko learn tune <subsystem>
  WINNER: roko tune (top-level, shorter)
  ACTION: learn tune prints hint to use roko tune

roko show                   ←→  roko status
  roko show: rich multi-pane view (costs, agents, knowledge, plans)
  roko status: compact 3-line health summary
  ACTION: both stay; clarify the distinction in help text
```

## Rust Code Sketch: New CLI Structure

### Adding `note` to the Command enum

```rust
// crates/roko-cli/src/main.rs — add to Command enum

/// Capture a thought, idea, or observation for later synthesis.
/// Instant, no LLM call. Stored in .roko/notes/.
#[command(after_help = "\
Examples:
  roko note \"cursor ACP uses JSON-RPC over stdio\"
  roko note --tag cursor \"startup lock prevents concurrent spawns\"
  roko note --tag feeds \"relay should forward feed ticks as topic messages\"")]
Note {
    /// The thought or idea to capture.
    #[arg(value_name = "TEXT")]
    text: Vec<String>,
    /// Tag this note for later synthesis filtering.
    #[arg(long)]
    tag: Option<String>,
    /// Working directory (default: cwd).
    #[arg(long)]
    workdir: Option<PathBuf>,
},
```

### Adding `develop` to the Command enum

```rust
/// Generate a plan from a prompt, review it, then execute it.
/// Equivalent to: roko do --plan --approve --tui
#[command(after_help = "\
Examples:
  roko develop \"implement cursor composer 2 support\"
  roko develop \"add auth to the API\" --dry-run
  roko develop --from-notes --tag cursor")]
Develop {
    /// Natural language description of what to build.
    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
    /// Generate plan but don't execute.
    #[arg(long)]
    dry_run: bool,
    /// Skip the approval screen and execute immediately.
    #[arg(long)]
    yes: bool,
    /// Synthesize from accumulated notes instead of a prompt.
    #[arg(long, conflicts_with = "prompt")]
    from_notes: bool,
    /// Filter notes by tag when using --from-notes.
    #[arg(long)]
    tag: Option<String>,
    /// Working directory (default: cwd).
    #[arg(long)]
    workdir: Option<PathBuf>,
    /// Override the provider.
    #[arg(long)]
    provider: Option<String>,
    /// Resume previous interrupted session.
    #[arg(long = "continue", value_name = "WORK_ID", num_args = 0..=1)]
    r#continue: Option<Option<String>>,
},
```

### Plan slug lookup in `do`

```rust
// In cmd_do(), before ScopeResolver classification:
if prompt_args.len() == 1 {
    let maybe_slug = &prompt_args[0];
    let plan_path = workdir.join(".roko").join("plans").join(maybe_slug);
    if plan_path.is_dir() && plan_path.join("tasks.toml").is_file() {
        return run_plan_execution(cli, &workdir, &plan_path, no_cascade, provider).await;
    }
}
// Otherwise: proceed with existing prompt classification
```

### `roko note` implementation

```rust
// crates/roko-cli/src/commands/note.rs

use std::path::PathBuf;
use chrono::Utc;

pub async fn cmd_note(
    workdir: &std::path::Path,
    text: Vec<String>,
    tag: Option<String>,
) -> anyhow::Result<i32> {
    let text = text.join(" ").trim().to_string();
    if text.is_empty() {
        anyhow::bail!("provide text to capture");
    }

    let notes_dir = workdir.join(".roko").join("notes");
    std::fs::create_dir_all(&notes_dir)?;

    let timestamp = Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let slug = slugify(&text);
    let filename = format!("{timestamp}-{slug}.md");
    let path = notes_dir.join(&filename);

    let tag_line = tag.as_deref()
        .map(|t| format!("tag: {t}\n"))
        .unwrap_or_default();

    let content = format!(
        "---\ncaptured: {timestamp}\n{tag_line}---\n\n{text}\n"
    );
    std::fs::write(&path, content)?;

    eprintln!("captured: {filename}");
    Ok(0)
}

fn slugify(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect()
}
```

## Deprecation Path

Don't break existing commands. Add deprecation hints:

```rust
// In cmd_prd() PrdCmd::Idea branch:
roko_cli::prd::cmd_idea(&workdir, &joined)?;
if !cli.quiet {
    eprintln!("hint: `roko prd idea` is now `roko note`. The old command still works.");
}

// In cmd_run() dispatch:
eprintln!("hint: `roko run` is now `roko do`. The old command still works.");
// Then delegate to cmd_do

// In learn tune dispatch:
eprintln!("hint: `roko learn tune` is now `roko tune`. The old command still works.");
```

Show each hint only once per week (stored in `.roko/hints.json`).

## The Full Braindump → Execution Flow

```bash
# Day 1: dump ideas as they come
roko note "cursor composer 2 — spawn ACP agent, JSON-RPC over stdio"
roko note "should implement LlmBackend trait like the other backends"
roko note "mori reference: /Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs"
roko note "need streaming notification parsing for deltas and tool calls"
roko note --tag cursor "startup lock to prevent concurrent spawns"

# Day 2: more ideas
roko note "also want the feed agents to publish to relay topics"
roko note --tag feeds "relay should forward feed ticks as topic messages"

# Day 3: synthesize into plans
roko plan --from-notes
```

`roko plan --from-notes` does:
1. Reads `.roko/notes/*.md`
2. Clusters related notes by keyword similarity (TF-IDF or simple overlap)
3. For each cluster, generates a separate plan
   ```
   Found 2 clusters in your notes:

   1. cursor-composer-2 (5 notes)
      → Generating plan...
      → 6 tasks, ~90 min

   2. feed-relay-publishing (2 notes)
      → Generating plan...
      → 3 tasks, ~30 min

   [1] Generate both  [2] Pick one  [q] Quit:
   ```
4. Each approved cluster writes `.roko/plans/<slug>/tasks.toml`

Then execute:
```bash
roko do cursor-composer-2    # runs the plan by slug
roko dashboard               # watch it work
```

Or all-in-one:
```bash
roko develop --from-notes --tag cursor
```

## Implementation Priority

1. `roko note` — trivial, ~50 lines wrapping `prd::cmd_idea` with tag support
2. `roko do <slug>` — add plan lookup before prompt classification (~10 lines)
3. `roko develop` — wrapper over `do_cmd.rs` with approval screen (~150 lines)
4. `roko plan "prompt"` — non-subcommand mode for plan generation (~100 lines)
5. Deprecation hints on `prd idea` and `run`
6. `roko plan --from-notes` — clustering + synthesis (hardest, ~300 lines)

## What NOT to Add

- No `roko work` — that's `roko do`
- No `roko build` — that's `roko do`
- No `roko go` — that's `roko do`
- No new verbs that are aliases for existing verbs
- No additional top-level commands that duplicate subcommand functionality

**One verb per intent. Three primary verbs. Everything else is a flag or subcommand.**

## Preserving Power-User Access

The full command tree stays. All 42 top-level commands and ~100 sub-subcommands remain
functional. The 3-verb design is a **surfacing layer**, not a replacement:

```
Primary surface (daily use):     note, plan, do / develop
Secondary surface (monitoring):  status, show, dashboard
Power surface (full access):     all existing commands still work
```

The difference is discoverability: new users see `note`, `plan`, `do` in the help output
as the first three commands. Power users who know `roko prd draft new` can keep using it.

## Success Criteria

- [ ] New user can go from install to self-development with: `init` → `note` → `plan` → `do`
- [ ] No command requires reading source code or TOML schemas to use
- [ ] Tab completion works for plan names and model names
- [ ] Existing commands still work with deprecation hints
- [ ] Zero warnings about unconfigured providers the user isn't using
- [ ] `roko note "text"` takes <100ms (no LLM call)
- [ ] `roko do cursor-composer-2` finds the plan by slug and runs it
- [ ] `roko develop "prompt"` shows plan approval before executing
