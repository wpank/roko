# Mori Workflow and UX Analysis

Comprehensive analysis of how a user actually interacts with Mori day-to-day,
based on source-level inspection of the 108K LOC codebase at
`/Users/will/dev/uniswap/bardo/apps/mori/`.

---

## 1. How Does a User Start Mori?

### Primary launch: `./mori.sh`

The canonical entry point is the `mori.sh` shell script at the repository root.
It is **not** a simple `cargo run` wrapper -- it is a full bootstrap harness that:

1. **Loads `.env`** from the repo root (API keys, gateway config).
2. **Auto-detects or configures the inference gateway:**
   - `--embedded-gateway`: Start Mori's built-in gateway process.
   - `--direct`: Skip gateway entirely, agents talk to providers directly.
   - Default (`auto`): Check if an external gateway is running on `:4000`.
     If yes, set `ANTHROPIC_BASE_URL` to route through it. If no, still set
     the URL so agents connect once it starts.
3. **Validates the repo** (checks for `Cargo.toml`, `apps/mori/`).
4. **Creates required directories:**
   ```
   .mori/
   .mori/runs/
   plans/context/briefs/
   plans/context/reviews/
   plans/context/tasks/
   plans/context/docs/
   plans/context/summaries/
   plans/context/archive/
   ```
5. **Runs a context sync script** (`scripts/bardo-sync-context.sh`) if present.
6. **Checks if a release build is needed.** Compares timestamps of
   `Cargo.toml`, `Cargo.lock`, and source files against the release binary.
   If stale, rebuilds with `cargo build -p mori -p mori-mcp --release`.
   During rebuild, writes a heartbeat to `.mori/runs/status.json` every 5
   seconds so external monitors know it is rebuilding, not hung.
7. **Launches the binary:**
   ```bash
   exec "$RELEASE_BIN" --repo-root "$SCRIPT_DIR" --parallel --batch-id current --no-gateway "$@"
   ```

The default flags always include `--parallel` (DAG-based concurrent execution)
and a `--batch-id` (defaults to `current` or the `MORI_BATCH_ID` env var).

### Alternative launchers

| Script | Purpose |
|--------|---------|
| `./mori-supervisor.sh` | Self-healing supervisor. Catches panics, feeds crash reports to Claude/Cursor for auto-fix, rebuilds, restarts. Circuit breaker after 10 failures or 3 of the same error. |
| `./mori-gateway.sh` | Start the standalone inference gateway on `:4000`. |
| `./mori-claude.sh` | Launch Claude Code routed through the gateway. Sets `ANTHROPIC_BASE_URL` and `ANTHROPIC_API_KEY` so all Claude CLI calls go through the gateway's caching/routing layer. |

### Direct binary invocation

```bash
# Build and run directly
cargo build -p mori --release
./target/release/mori --repo-root . --parallel 01-09

# Or via cargo
cargo run -p mori --release -- --parallel 01-09
```

---

## 2. Complete CLI Command Set

Mori uses a flat subcommand model. The first argument determines the mode.
When no recognized subcommand is given, the default mode is **plan execution
with TUI**.

### Core execution (default -- no subcommand)

```
mori [plan_specs...] [flags]
```

Plan specs are numeric identifiers or ranges: `01`, `01-09`, `08a-08d`.
If omitted, reads from `.mori/queue.toml`.

**Execution flags:**

| Flag | Default | Effect |
|------|---------|--------|
| `--parallel` | off | Enable DAG-based parallel plan execution |
| `--max-agents N` | 8 | Maximum concurrent agent processes |
| `--max-parallel-plans N` | 1 | Maximum plans running simultaneously |
| `--express` | off | Single-pass implementer, no strategist/reviews, up to 20 agents |
| `--no-review` | off | Skip strategist + review loop |
| `--skip-tests` | off | Skip `cargo test` gate |
| `--no-docs` | off | Skip Scribe + Critic phases |
| `--model MODEL` | config | Override the model for all agents |
| `--fallback-model MODEL` | none | Retry failed spawns with this model |
| `--fast` | off | Codex fast mode (1.5x speed, 2x credits) |
| `--max-iterations N` | 8 | Maximum review iterations before halting |
| `--batch-size N` | none | Pause after every N plans for review |
| `--batch-id ID` | "current" | Override batch branch suffix |
| `--headless` | off | Run without TUI (log to stdout) |
| `--pre-plan` | off | Speculative pre-planning for upcoming waves |
| `--refactor` | off | Enable post-plan refactoring passes |
| `--paused` | off | Launch TUI in paused state (no agents until unpause) |
| `--queue` | off | Force reading `.mori/queue.toml` even with CLI specs |
| `--milestone NAME` | none | Run only plans from a specific milestone |
| `--preset PRESET` | none | Execution preset: quality, balanced, cost, speed |
| `--gateway / --no-gateway` | auto | Enable/disable embedded inference gateway |
| `--gateway-port N` | 4000 | Port for embedded gateway |
| `--gateway-url URL` | none | Route agents through external gateway |
| `--validate` | off | Validate plan files and show parallelism stats, no execution |
| `--dry-run` | off | Build DAG, print execution plan, exit |
| `--cleanup` | off | Clean up merged plan branches and prune worktrees |

### Subcommands

```
mori bootstrap [root]            Create a Mori-ready workspace
mori init [root]                 Lightweight init (.mori/ structure only)
mori setup [--scope global|repo] Configure provider and model preferences
mori prd draft [slug]            Create a starter PRD document
mori prd ingest [sources...]     Copy existing PRD files into managed folders
mori plan draft [plan_id]        Create a starter plan directory
mori plan prepare [plan_id]      Rebuild companion artifacts for existing plan
mori enrich routing [plans...]   Classify tasks into routing metadata
mori enrich all [plans...]       Full enrichment (routing + support artifacts)
mori enrich artifacts <step>     Generate specific support artifact for a plan
mori learn [--write-playbook]    Inspect or refresh learned execution memory
mori server [--port 8080]        Run as HTTP server (headless)
mori deploy up <target>          Deploy gateway/mori to cloud (Fly/Railway)
mori deploy list                 List deployed instances
mori deploy status <app>         Show deployed instance status
mori deploy logs <app>           Show deployed instance logs
mori deploy destroy <app>        Destroy deployed instance
mori ingest "<directive>"        Inject a directive into running orchestrator
mori refresh                     Refresh context artifacts
```

### Enrichment artifact types

The `mori enrich artifacts` command supports 13 distinct artifact steps:

- `briefs` -- Plan brief summaries
- `tasks` -- Task decomposition
- `verify` -- Verification criteria
- `review` -- Review rubrics
- `prd` -- PRD context extraction
- `decompose` -- Task decomposition
- `tests` -- Test generation
- `invariants` -- Invariant extraction
- `scribe` -- Citation/diagram tasks
- `research` -- Research packs
- `dependencies` -- Dependency manifests
- `fixtures` -- Fixture manifests
- `integration` -- Integration context
- `all` -- Run all of the above

---

## 3. Workflow: Idea to Plan to Execute to Verify to Merge

### Phase 1: Specification

```
prd/
  inbox/     -- Raw PRD files dropped here
  active/    -- PRDs being worked on
  archive/   -- Completed PRDs
```

```bash
mori prd draft --title "Wire auth middleware" --from-readme
mori prd ingest ~/specs/auth-spec.md --target active
```

### Phase 2: Plan creation

Each plan is a numbered markdown file in `plans/`:

```
plans/
  01-workspace-scaffold.md
  02-core-types.md
  ...
  plans/context/         -- Generated support artifacts
    briefs/              -- Plan brief summaries
    reviews/             -- Review rubrics
    tasks/               -- Task decompositions (TOML)
    docs/                -- Generated documentation
    summaries/           -- Completion summaries
    prd-chunks/          -- PRD context extracts
    verify-chains/       -- Verification chain specs
```

A plan file has YAML frontmatter declaring dependencies, touched crates,
parallelism, and estimated effort:

```yaml
---
plan: 09-chain-layer
depends_on: ["01", "02"]
crates_touched: ["golem-chain", "golem-core"]
estimated_tasks: 12
estimated_parallel_width: 4
estimated_minutes: 180
parallel_safe: true
---
```

Each plan has a companion TOML task file (e.g., `plans/context/tasks/09-tasks.toml`)
with individual task definitions:

```toml
[meta]
plan = "09-chain-layer"
iteration = 1
total = 12

[[tasks]]
id = "T1"
title = "Create ChainClient trait"
files = ["crates/golem-chain/src/client.rs"]
depends_on = []
status = "pending"
complexity_band = "standard"
category = "implementation"
```

```bash
mori plan draft 09 --title "Chain Layer" --prd prd/active/chain.md --depends-on 01,02
mori plan prepare 09 --force
mori enrich all 09
```

### Phase 3: Queue configuration

`.mori/queue.toml` organizes plans into milestones:

```toml
[run]
mode = "express"
max_agents = 15
max_parallel_plans = 5

[[milestone]]
name = "Audit Remediation"
description = "Re-implement plans that failed verification"
tags = ["audit", "remediation", "critical"]
plans = ["01", "10", "11", "12", "14a", "14b", ...]

[[milestone]]
name = "Runnable Golem"
plans = ["02", "03", "09", "12", "13a", ...]
```

Milestones run sequentially: Mori completes all plans in the first incomplete
milestone before advancing to the next. Within a milestone, plans execute
according to their DAG dependencies.

### Phase 4: Execution

```bash
./mori.sh                    # Run current milestone from queue.toml
./mori.sh 01-09              # Run specific plans
./mori.sh --milestone "Runnable Golem"  # Run a specific milestone
```

For each plan, the orchestrator follows this state machine:

```
Initializing
  |
  v
PlanReady --> Preflight (pre-checks)
  |
  v
Implementer (agent writes code in isolated worktree)
  |
  v
CompileGate (cargo check / cargo build)
  |         \
  | pass     \ fail --> back to Implementer (or AutoFixer in express mode)
  v
TestGate (cargo test)
  |         \
  | pass     \ fail --> back to Implementer
  v
Reviewing (Architect + Auditor + Scribe in parallel)
  |
  v
CriticReview (Critic reviews docs)
  |
  v
Verdict (Approve / Revise)
  |         \
  | approve  \ revise --> back to Implementer (with review feedback)
  v
Committing (merge to batch branch)
  |
  v
Complete
```

In **express mode**, the pipeline is shortened:
```
Implementer --> CompileGate --> TestGate --> AutoFixer (if fail) --> Complete
```
No strategist, no reviewers, no docs phases.

### Phase 5: Git workflow

Each plan executes in an **isolated git worktree**:

```
.mori/worktrees/
  plan-01/              -- worktree for plan 01
  plan-02/              -- worktree for plan 02
  ...
```

Worktrees branch from `main-fresh` (configurable via `fresh_base_branch`).
Each worktree gets:
- Its own `target/` directory (scoped cargo builds)
- A `.cursor/cli.json` for Cursor ACP agent permissions
- A `mcp-config.local.json` for MCP tool server configuration
- Isolated environment variables (`CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`)

On success, plan branches merge into the **batch branch** (e.g., `batch/current`).
The batch branch accumulates completed plans and can be merged to `main` via
the TUI.

### Phase 6: Verification and merge

After gate pass:
1. Plan worktree work is committed.
2. Plan branch merges into batch branch (dependency-ordered merge queue).
3. Post-merge verification runs on the batch branch.
4. Worktree is cleaned up.

Batch-to-main merge is a manual TUI action (`m` key on Plans tab) with a
confirmation dialog showing plan count, failed count, and last commit hash.

---

## 4. Intermediate Files and State

### `.mori/` directory structure

```
.mori/
  config.toml              -- Per-repo configuration (models, agents, toggles)
  config.toml.example      -- Example configuration
  queue.toml               -- Milestone/plan execution order
  mcp-config.json          -- MCP server configuration for agents
  costs.db                 -- SQLite cost tracking database
  index.db                 -- Code intelligence index

  runs/                    -- Active run state
    status.json            -- Current run status (plan, phase, iteration, PID)
    events.jsonl           -- Append-only event log
    task-state.json        -- Task-level state for crash recovery
    run.pid                -- Current process PID
    mori.log               -- Structured JSON log (rotated per run)
    discovered-patterns.json  -- Error patterns shared between parallel agents
    output/                -- Per-plan gate output files
    costs/                 -- Per-run cost breakdowns
    recovery/              -- Recovery snapshots and patches
    fixtures/              -- Running fixture state

  memory/                  -- Learning state
    playbook.toml          -- Learned execution rules (when/then patterns)
    episodes.jsonl         -- Agent turn recordings
    efficiency.json        -- Efficiency metrics
    efficiency-history.jsonl -- Historical efficiency data
    context-packs/         -- Pre-computed context packs
    dependencies.toml      -- Dependency registry
    fixtures.toml          -- Fixture registry
    prompt-logs/           -- Prompt/response logs per task
    refresh-state.json     -- Downstream refresh tracking

  plans/                   -- Plan metadata copies
  tools/                   -- MCP tool definitions
  runtime/                 -- Runtime artifacts
```

### `plans/context/` directory structure

```
plans/context/
  briefs/                  -- Plan brief summaries (per-plan)
  tasks/                   -- Task decomposition TOML files
  reviews/                 -- Review rubrics and results
  docs/                    -- Generated documentation
  summaries/               -- Plan completion summaries
  archive/                 -- Completed plan artifacts
  prd-chunks/              -- Extracted PRD sections per plan
  prd2-extracts/           -- PRD2 context extractions
  verify-chains/           -- Verification chain specifications
  decompositions/          -- Task decomposition artifacts
  registry/                -- Dependency/fixture registries
  enrichment-status.md     -- Enrichment pipeline status
  workspace-map.md         -- Auto-generated workspace structure
  golden-path-index.json   -- Golden path test index
  type-registry.json       -- Crate type registry
```

### Key state files

**`status.json`** -- Heartbeat file read by external monitors:
```json
{
  "version": 2,
  "run_id": "20260815-143022",
  "batch_id": "current",
  "plans_total": 15,
  "plans_completed": 7,
  "plans_remaining": 8,
  "current_plan": "09-chain-layer",
  "current_phase": "implementer",
  "current_iteration": 2,
  "started_at": "2026-08-15T14:30:22Z",
  "last_activity": "2026-08-15T15:12:44Z",
  "pid": 67197,
  "hang_threshold_seconds": 600
}
```

**`task-state.json`** -- Crash recovery checkpoint:
```json
{
  "version": 2,
  "run_id": "20260815-143022",
  "batch_branch": "batch/current",
  "completed_tasks": ["09:T1", "09:T2", "09:T3"],
  "in_flight": {"09:T4": "impl-cx-09-4"},
  "completed_plans": ["01", "02", "03"],
  "total_tokens": {"input": 1234567, "output": 234567},
  "plan_iterations": {"09": 2},
  "merge_queue": ["09"],
  "active_worktrees": {"09": "/path/to/worktree"},
  "plan_phases": {"09": "implementer"},
  "task_failure_counts": {"09:T4": 1},
  "skipped_tasks": []
}
```

---

## 5. User Interaction During Execution

### TUI tabs and navigation

The TUI has 7 tabs accessed via `F1`-`F7` or number keys `1`-`7`:

| Tab | Name | Shows |
|-----|------|-------|
| F1 | Dashboard | Overview: plan progress, phase, agent count, cost, ETA |
| F2 | Plans | Hierarchical wave/plan/task tree with drill-in |
| F3 | Agents | Active agent list with output streaming |
| F4 | Git | Branch tree, worktree status, merge state |
| F5 | Logs | Structured log viewer with level filtering |
| F6 | Config | Live-editable configuration (models, toggles, limits) |
| F7 | Processes | Running process list and resource usage |
| F8 | Queue Overview | Milestone roadmap across the full queue |

### Interactive controls

**Global (all tabs):**
- `q` / `Ctrl-C` -- Quit
- `p` -- Toggle pause/resume (agents stop spawning when paused)
- `?` -- Show help overlay
- `Ctrl-R` -- Restart all plans (with confirmation)
- `Ctrl-X` -- Force advance selected plan
- `Ctrl-D` -- Reset selected plan
- `Ctrl-G` -- Git reconcile (commit/merge/prune)
- `Ctrl-A` -- Approve all pending approvals
- `Ctrl-T` -- Open task picker modal

**Plans tab (F2):**
- `j/k` or arrows -- Navigate plan/task tree
- `l/h` or left/right -- Next/previous wave
- `Enter` -- Drill into plan/task detail
- `Esc` -- Drill out / go back
- `m` -- Merge batch to main
- `M` -- Merge selected done plan to batch
- `s` -- Soft retry failed plan
- `z` -- Diagnose plan (inspect state)
- `S` -- Repair plan (preserve work)
- `R` -- Repair plan (clean start)
- `c` -- Re-verify plan
- `/` -- Filter plans by name

**Agents tab (F3):**
- `j/k` -- Navigate agent list
- `End` -- Scroll to bottom of agent output
- backtick -- Toggle between all agents

**Config tab (F6):**
- `j/k` -- Navigate settings
- `h/l` -- Decrease/increase values
- `Enter`/`Space` -- Toggle boolean / cycle enum

### Live injection

While agents are running, the user can inject directives:

```bash
# From a separate terminal:
mori ingest "fix the auth handler to validate tokens"
```

Or via the TUI (type-in mode), or by dropping a file into `.mori/ingest/`.
Directives are classified (agent nudge, plan amendment, new task, context only)
and routed to the appropriate running agent or plan.

### Pause and resume

- `p` in the TUI toggles pause. When paused, no new agents spawn, but running
  agents continue to completion.
- `--paused` flag starts the TUI in paused state.
- On crash, state is preserved in `task-state.json`. Restarting Mori resumes
  from the last checkpoint: completed tasks stay completed, in-flight tasks
  are retried.

---

## 6. Diagnostic and Debugging Tools

### Validation mode

```bash
mori --validate 01-09
```

Outputs:
- Plans discovered with dependency info
- Task files loaded with counts
- Unified task DAG analysis (node count, max parallelism width, critical path minutes)
- Dangling references
- Execution wave breakdown

### Dry run mode

```bash
mori --dry-run 01-09
```

Builds the DAG, prints the execution plan (waves, parallelism, dependencies),
and exits without spawning any agents.

### Learning inspection

```bash
mori learn
```

Outputs a comprehensive snapshot:
- Episode counts (total, success, fail)
- Plans with history/reflections
- Playbook rules (learned vs. manual)
- Routing coverage percentage
- Rich routing (category/speed/quality/context tags)
- Model/provider/strategy distributions
- Prompt density statistics
- Support artifact freshness
- Fixture utilization

```bash
mori learn --write-playbook          # Refresh playbook from history
mori learn --write-playbook --dry-run  # Preview what would change
```

### Crash reports

On panic or fatal error, Mori writes a structured JSON crash report to
`.mori/runs/crash-report.json` containing:

- Error message and location
- Full backtrace
- Application state at crash time (orchestrator state, current plan/phase,
  active agents, recent logs)
- Configuration summary
- Environment info (Rust version, OS, terminal size)
- Error signature (SHA-256 hash for dedup)

The supervisor (`mori-supervisor.sh`) reads this report and feeds it to
Claude or Cursor for automated fix attempts.

### Error pattern discovery

When a gate fails, the error digest is extracted and written to
`.mori/runs/discovered-patterns.json`. Parallel agents read this file and
inject recent patterns into their context, avoiding re-discovery of the
same errors.

### Log analysis

```
.mori/runs/mori.log          -- Structured JSON log (tracing-subscriber)
.mori/runs/events.jsonl       -- Machine-readable event timeline
.mori/memory/episodes.jsonl   -- Agent turn recordings
.mori/memory/prompt-logs/     -- Full prompt/response per task
```

---

## 7. Progress and Problem Reporting

### TUI dashboard (F1)

The dashboard shows real-time:
- Total plans / completed / failed / in-progress
- Current phase per active plan
- Agent count and model distribution
- Cost (input/output tokens)
- Estimated time remaining
- Wave progress (which waves are complete)
- Notification bar for recent events

### Status file

`.mori/runs/status.json` is updated every tick (250ms in TUI mode, 5 seconds
during builds). External tools can poll this file:

```json
{
  "current_plan": "09-chain-layer",
  "current_phase": "test-gate",
  "plans_completed": 7,
  "plans_total": 15,
  "hang_threshold_seconds": 600
}
```

### Event journal

`.mori/runs/events.jsonl` records every state transition:

```json
{"ts":"2026-08-15T15:12:44Z","event":"plan_started","plan":"09","phase":"preflight","iter":1}
{"ts":"2026-08-15T15:13:01Z","event":"task_done","plan":"09","task":"T1","duration_secs":17}
{"ts":"2026-08-15T15:14:22Z","event":"plan_gates_passed","plan":"09","phase":"test-gate","iter":1}
{"ts":"2026-08-15T15:14:30Z","event":"plan_merged","plan":"09","phase":"committing","iter":1}
```

### Notifications

The TUI shows toast-style notifications with TTL and severity levels
(Info, Warn, Error). Examples:
- "Plan 09: compile gate PASS"
- "Review cap hit for 12 after 5 revisions, force-committing"
- "Agent spawn failed, retrying with fallback model"

### Conductor interventions

The Conductor meta-agent monitors all running agents and can:
- **Nudge**: Send a steering message to a stuck agent
- **Restart**: Kill and restart an agent from scratch
- **Force-advance**: Skip reviews and commit what's there
- **Skip validations**: Bypass specific gates
- **Assign additional tasks**: Feed more work to a warm agent

Interventions are logged and visible in the TUI's Logs tab.

---

## 8. Configuration Hierarchy

Configuration follows a **global -> repo -> queue -> CLI** precedence chain,
where later layers override earlier ones.

### Layer 1: Global config

Created by `mori setup --scope global`. Located at a platform-specific path
(e.g., `~/.config/mori/config.toml`). Sets default provider preferences.

### Layer 2: Repo config (`.mori/config.toml`)

Per-repository configuration. This is the primary config file with ~70
settings:

```toml
# Models
codex_default_model = "claude-sonnet-4-6"
cursor_default_model = "composer-2-fast"
claude_default_model = "claude-sonnet-4-6"
conductor_model = "claude-sonnet-4-6"
fast_task_model = "claude-haiku-4-5-20251001"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"

# Provider routing
fast_task_provider = "claude"
standard_task_provider = "claude"
complex_task_provider = "claude"
routing_mode = "auto_override"
optimization_profile = "balanced"
context_strategy = "mcp_first"

# Agent behavior
max_agents = 20
max_parallel_plans = 5
express_mode = true
agent_bare_mode = false
max_auto_fix_attempts = 2
warm_implementers_per_plan = 1

# Review pipeline
architect_enabled = true
auditor_enabled = true
scribe_enabled = true
critic_enabled = true
max_iterations = 5
clippy_enabled = false

# Knowledge injection
knowledge_file_intel = true
knowledge_warnings = true
knowledge_wave_context = true
knowledge_error_patterns = true

# Learning
auto_playbook_refresh = true
auto_research_prepass = true
```

### Layer 3: Queue config (`.mori/queue.toml`)

Run-level overrides that apply to the current queue execution:

```toml
[run]
mode = "express"
max_agents = 15
max_parallel_plans = 5
preset = "balanced"
context_strategy = "hybrid"
```

Queue settings override repo config but are overridden by CLI flags.

### Layer 4: CLI flags

CLI flags have the highest precedence. For example, `--max-agents 8` overrides
both queue and config values.

### Layer 5: Per-plan overrides

Both config.toml and queue.toml support per-plan routing overrides:

```toml
[plan_overrides."09"]
model = "claude-opus-4-6"
provider = "claude"
```

### Layer 6: Per-task routing

Each task in a TOML file can declare its own routing:

```toml
[[tasks]]
id = "T5"
complexity_band = "complex"
category = "integration"
reasoning_level = "high"
speed_priority = "accuracy"
preferred_model = "claude-opus-4-6"
preferred_provider = "claude"
```

### MCP config

`.mori/mcp-config.json` (or `.mori/mcp-config.local.json`) configures MCP
tool servers available to agents. Agents spawned in worktrees inherit this
config and use `--mcp-config` and `--strict-mcp-config` to restrict tool access.

---

## 9. Error Handling and Recovery

### Crash recovery

Mori writes `task-state.json` after every significant state change. On
restart, it:

1. Reads `task-state.json` to recover completed tasks, plan iterations,
   merge queue, and active worktrees.
2. Skips completed tasks and plans.
3. Resumes in-flight tasks (kills stale agents, retries from last checkpoint).
4. Restores the merge queue order.
5. Recovers worktree references and validates they still exist.

### Gate failure handling

When a gate (compile, test, clippy) fails:

**Normal mode:**
1. Error digest is extracted from cargo output (unique error blocks with
   file/line references).
2. Discovered patterns are appended to `discovered-patterns.json`.
3. Review feedback is injected into the next implementer prompt.
4. The plan cycles back to Implementer phase with the failure context.
5. After `max_iterations` revisions, force-commits.

**Express mode:**
1. An `AutoFixer` agent is spawned with the error digest.
2. If the fixer succeeds, gates re-run.
3. After `max_auto_fix_attempts` (default 2), the plan is marked failed.

### Agent spawn failure

1. If the primary model fails to spawn, retry with `fallback_model`.
2. If fallback also fails, mark the task as failed and increment
   `task_failure_counts`.
3. After 3 failures for the same task, skip it and move on.
4. Provider health is tracked: disabled providers are bypassed.

### Worktree recovery

If a worktree is corrupted or missing:
- `S` (Repair-Preserve): Saves a recovery patch, cleans the worktree,
  re-applies the patch.
- `R` (Repair-Clean): Removes the worktree entirely and starts fresh.
- `z` (Diagnose): Inspects worktree state without modifying it.

### Merge conflict handling

When merging a plan branch to batch:
1. A `MergeCheckpoint` is written before the merge starts.
2. If the merge fails (conflict), the checkpoint allows retry.
3. The `MergeResolver` agent role can attempt automated conflict resolution.
4. Git reconcile (`Ctrl-G`) commits outstanding work, merges unmerged
   plans, prunes worktrees, and merges batch to staging.

### Supervisor auto-recovery

`mori-supervisor.sh` provides a higher-level recovery loop:

1. Run Mori normally.
2. On crash (non-zero exit), read the crash report.
3. Compute error signature for dedup.
4. Feed crash context to Claude/Cursor agent for auto-fix.
5. Rebuild.
6. If build succeeds, restart Mori.
7. Circuit breaker: stop after 10 total failures or 3 same-error failures.
8. macOS notification on circuit breaker trip.

---

## 10. What Makes the UX Feel "Intuitive"

### Pattern: Everything is a plan number

Plans are identified by simple numbers (`01`, `02`, `09`, `08a`). This
numbering scheme is used consistently everywhere:
- CLI arguments: `mori 01-09`
- Queue config: `plans = ["01", "02", "09"]`
- Task IDs: `09:T1`, `09:T2`
- Branch names: `plan-09`, `batch/current`
- Worktree paths: `.mori/worktrees/plan-09/`
- Context files: `plans/context/briefs/09-brief.md`

The user never has to remember full plan names. Numbers are always valid.

### Pattern: Sensible defaults that don't require configuration

- No arguments: reads from `queue.toml`, runs the current milestone.
- No `queue.toml`: provides a clear error message telling the user what to do.
- No config: first-time setup wizard (`maybe_run_first_time_setup`) runs
  automatically on the first launch.
- Gateway auto-detection: if running, use it; if not, direct API calls work.
- Model routing: tasks are classified by complexity band (fast/standard/complex)
  and automatically routed to appropriate models.

### Pattern: Progressive disclosure

The minimal invocation is just `./mori.sh`. Everything else is optional:
1. Run with queue.toml defaults
2. Override with CLI flags as needed
3. Tune with config.toml for persistent preferences
4. Per-plan overrides for special cases
5. Per-task routing for fine-grained control

### Pattern: Non-destructive operations require confirmation

Every destructive action in the TUI goes through a confirmation dialog:
- Restart all plans
- Force advance
- Reset plan
- Git reconcile
- Merge to main

The confirmation dialog shows the action name, description, and affected
plans. `y` confirms, `n` cancels.

### Pattern: Vim-style navigation

Consistent `j/k/h/l` navigation across all tabs. `Enter` drills in, `Esc`
goes back. `/` starts a filter. Tab cycles focus zones. This works because
the target user is a developer already comfortable with vim keybindings.

### Pattern: Everything is observable

- TUI shows real-time agent output streaming
- Status file is pollable by external tools
- Events are machine-readable JSONL
- Logs are structured JSON
- Crash reports are comprehensive JSON
- The learning system exposes all its state via `mori learn`

### Pattern: Fail forward, not backward

- Express mode: skip reviews, auto-fix on failure, move on
- Force advance: commit whatever is there and proceed
- Soft retry: preserve completed tasks, only retry failures
- Playbook learning: extract patterns from failures to avoid repeating them
- Error pattern sharing: parallel agents learn from each other's failures

### Pattern: The TUI is optional

Every operation has a non-TUI equivalent:
- `--headless` mode for CI/scripts
- `mori server` for HTTP API access
- `status.json` for monitoring
- `events.jsonl` for audit
- CLI subcommands for all plan/PRD operations

### Pattern: Git isolation by default

Every plan runs in its own worktree. This means:
- No plan can corrupt another plan's work
- Parallel execution is safe by default
- Failed plans don't block other plans
- Merge is explicit and ordered
- The user's working tree is never touched

### Pattern: Cost awareness

The gateway tracks cost per request. The TUI shows cumulative token usage.
The batch API routes non-urgent work at 50% cost. Model routing puts fast
tasks on cheap models and complex tasks on expensive ones. Execution presets
(quality/balanced/cost/speed) let the user make an explicit cost-quality
tradeoff.

---

## Summary

Mori's UX is built around a simple mental model: **plans are numbered, plans
have tasks, tasks run in worktrees, gates verify, branches merge**. The
complexity (DAG scheduling, multi-model routing, learning, gateway caching,
crash recovery) is hidden behind sensible defaults. The user's day-to-day
is: edit `queue.toml`, run `./mori.sh`, watch the TUI, merge when done.
The system handles everything else automatically and tells you when
something goes wrong.

The key architectural insight is that the TUI is not just a progress
display -- it is a full control surface. The user can pause, inject
directives, retry plans, change configuration, merge branches, and
diagnose problems without leaving the TUI or opening another terminal.
This makes Mori feel less like a batch script and more like an IDE for
agent-driven development.
