# Mori-Old Analysis: New Gaps Not in Backlog

> **Source**: All 17 comparison docs + INDEX/SYNTHESIS/IMPLEMENTATION-CHECKLIST/CONTEXT in
> `/Users/will/dev/nunchi/roko/roko/tmp/mori-old/`
>
> **Cross-reference**: Existing backlog at `tmp/backlog/00-INDEX.md` (items 01-110)
>
> **Date**: 2026-08-19
>
> **Method**: Read every mori-old document, extracted every concrete gap/task/recommendation,
> checked each against the backlog index. Items that map to existing backlog entries are noted.
> Items that are new are numbered with an "MO-" prefix and include priority/size estimates.

---

## How to Read This Document

Each gap has:
- **Title**: Short descriptive name
- **Source**: Which mori-old file(s) identified it
- **Description**: What needs to be done, concretely
- **Backlog mapping**: Which existing item covers it (if any), or "NEW"
- **Priority/Size** (for new items only): P0-P3 / XS-XL

---

## Part 1: Gaps That Map to Existing Backlog Items

These were found in the mori-old analysis but are already captured in the active backlog.
Listed here for completeness.

| Mori-Old Finding | Source Doc | Existing Item |
|---|---|---|
| Event loop decomposition (god object at ~23K LOC) | 12, 03, 02 | #20 Event Loop Decomposition |
| Express mode (skip reviews, auto-fix on gate failure) | 03, 05 | #05 Express Mode |
| Post-gate LLM reflection generating playbook candidates | 09, 03 | #15 Post-Gate Reflection |
| Warm agent pre-spawning (process pool, not just prompt cache) | 04, 03 | #16 Warm Agent Spawning |
| ACP/serve experiment injection parity (context vs receipt protocol) | 12, 09 | #39 ACP Learning-Pipeline Parity |
| Cascade router task category awareness | 09, 12 | #84 Cascade Router Task Category Awareness |
| UX34 force_backend override learning isolation | 12 | #90 UX34 Override Learning Isolation |
| Hindsight causal inference refinement | 09 | #92 Hindsight Causal Inference |
| Named-surface TUI rendering completion | 11, 02 | #41 TUI Push-Mode Panel Data |
| TUI design system alignment | 02 | #71 TUI Design System Alignment |
| Doctor/onboarding diagnostics coverage | 06 | #104 Doctor Diagnostic Coverage |
| CLI UX consistency (--json flags, verb consistency) | 05, 06 | #77 CLI UX Consistency |
| Cross-crate code duplication | 13 | #102 Cross-Crate Code Duplication |
| Plan execution resilience (resume, stale snapshots) | 03, 05 | #103 Plan Execution Resilience |
| GitHub workflow robustness | 07 | #98 GitHub Workflow Robustness |
| Zero-config onboarding wizard | 05 | #63 Zero-Config Onboarding |
| HDC prompt assembly wiring | 09, 11 | #67 HDC Prompt Assembly |
| CLI error message quality | 05, 06 | #100 CLI Error Message Quality |

---

## Part 2: New Gaps — Not in Any Existing Backlog Item

Concrete actionable items that are not represented in the backlog.

---

### MO-01: TUI Headless Snapshot Mode

**Source**: `IMPLEMENTATION-CHECKLIST.md` (section 0.1, 0.2)

**Description**: Add `roko dashboard --snapshot <dir>` and `roko screenshot` commands that
render all TUI tabs and sub-views to text files (and optionally PNG) using ratatui's
`TestBackend` without requiring a real terminal. This is the foundation for automated visual
assessment — Claude or an agent can invoke `roko screenshot`, then read the manifest and
per-tab text files to understand TUI state without a live terminal session.

The infrastructure already exists: `TestBackend` is used in unit tests, `rendered_text()`
helper exists, `Tab::ALL` provides all 10 tabs. The gap is a CLI-accessible snapshot mode
that serializes all rendered views to disk.

Implementation path:
- New `crates/roko-cli/src/tui/snapshot.rs` module
- Add `--snapshot <dir>` flag to `Dashboard` CLI variant
- Add `roko screenshot` as top-level command in `main.rs`
- For each of 10 tabs: render with TestBackend, write `.txt`, optionally write `.png`
- Write `manifest.json` listing all captured files with tab/sub-view metadata

**Backlog mapping**: NEW

**Priority**: P1 — directly enables Claude self-monitoring of the TUI; nothing else
requires a real terminal session to assess TUI output.

**Size**: M (2-3d)

---

### MO-02: Continuous Screenshot Collection During Plan Runs

**Source**: `IMPLEMENTATION-CHECKLIST.md` (section 0.3)

**Description**: Add `--screenshots` flag to `roko plan run` that captures TUI snapshots
at configurable intervals and on significant events (plan state changes, gate results, wave
transitions, errors, completion). Produces a visual timeline at
`.roko/screenshots/run-<timestamp>/` with a `manifest.json` linking events to snapshot dirs.

This enables reviewing the complete visual evolution of a run after the fact. Smart capture
only renders tabs relevant to each event type (gate events → F1+F2+F10; agent events →
F1+F3; interval → all tabs).

Depends on MO-01 for the snapshot engine.

**Backlog mapping**: NEW

**Priority**: P2 — useful for dogfood verification and debugging but not blocking

**Size**: S (1d)

---

### MO-03: `roko diagnose <plan-id>` Command

**Source**: `IMPLEMENTATION-CHECKLIST.md` (section 0.5)

**Description**: A single command that aggregates everything needed to understand why a
plan failed: task status, gate results, classified errors (file/line/type/message from cargo
output), git worktree state (branch, dirty files, commits ahead), suggested recovery actions,
and relevant episode history. Always outputs JSON for machine consumption.

This is the #1 missing debugging tool. Currently diagnosing a failed plan requires reading
multiple files, running git commands, and correlating across JSONL logs. `roko diagnose`
collapses this into one structured output that Claude or an agent can act on.

New file: `crates/roko-cli/src/commands/diagnose.rs`

**Backlog mapping**: NEW (distinct from #104 Doctor Diagnostic Coverage which focuses on
workspace-level checks, not per-plan failure analysis)

**Priority**: P1 — directly unblocks the self-hosting debugging loop

**Size**: M (2-3d)

---

### MO-04: `--json` Output for All Core CLI Commands

**Source**: `IMPLEMENTATION-CHECKLIST.md` (section 0.4), `05-MORI-WORKFLOW-UX.md`

**Description**: Add machine-readable `--json` output to commands that don't have it:
`roko status`, `roko doctor`, `roko learn all`, `roko plan list`, `roko plan show <id>`,
`roko agent list`. Each should have a documented stable JSON schema.

Note: `roko doctor --json` already has partial implementation per `06-ROKO-E2E-WIRING-AUDIT.md`
(it mentions JSON format with `--json`). The gap is that `roko learn all`, `roko plan list`,
and `roko agent list` do not have JSON output.

**Backlog mapping**: NEW (partially overlaps with #77 CLI UX Consistency but that item
focuses on verb consistency, not JSON output modes)

**Priority**: P2 — agents need structured output; plain-text parsing is fragile

**Size**: S (1d)

---

### MO-05: Queue Manifest (`roko.queue.toml` / Milestone System)

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 2, R1)

**Description**: Add a `.roko/queue.toml` or `plans/queue.toml` manifest that supports:
- Named milestones grouping plans into logical batches with descriptions and tags
- Sequential milestone progression (complete milestone N before advancing to N+1)
- Maintenance plan declarations (refactor/QA/docs/integration/audit) that run after their
  implementation plans complete and are injected as DAG edges automatically
- Per-run session settings (`[run]` section) overriding `roko.toml` for just this run

This is the single most impactful operator UX gap vs Mori. Without milestones, a user
running `roko plan run plans/` gets all plans in DAG order with no way to say "these 5
plans are the MVP, then these 3 are the demo." The existing `plans/INDEX.md` has plan
metadata; queue.toml would formalize ordering and grouping.

Implementation path:
- Port `QueueConfig` struct from Mori's `orchestrator/queue.rs`
- Wire into `plan_loader::load_plans()` for milestone-ordered plan selection
- Add `roko plan queue show/edit/validate` CLI commands
- Runner reads queue config for milestone-based selection

**Backlog mapping**: NEW (related to #20 Event Loop Decomposition but distinct — this is
operator-facing workflow config, not internal architecture)

**Priority**: P2 — significantly improves multi-plan operator UX

**Size**: L (5-7d)

---

### MO-06: Plan-Level Wave Computation (Cross-Plan DAG)

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 3, R2)

**Description**: Add Kahn's algorithm for plan-level waves using cross-plan dependencies.
Roko already has per-plan task DAGs with `depends_on_plan` fields; the missing piece is
computing which plans can run in parallel (a wave) vs which must sequence.

Wave computation enables:
- Parallel plans within a wave with bounded concurrency
- Sequential wave progression (wave 0 completes before wave 1 starts)
- Wave progress visualization in the TUI (see MO-07)
- Critical path analysis (longest sequential chain = ETA lower bound)
- File/crate overlap detection to warn about conflicting parallel plans

Port `PlanDag::compute_waves()` from Mori's `orchestrator/dag.rs`. The algorithm is
straightforward Kahn's and Roko's task definitions already carry the dependency data.

**Backlog mapping**: NEW

**Priority**: P2 — prerequisite for MO-07 (wave visualization) and MO-05 (milestones)

**Size**: M (2-3d)

---

### MO-07: Wave Progress Visualization in TUI

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (sections 7, R7), `02-ROKO-TUI-ARCHITECTURE.md`

**Description**: Add wave progress display to the TUI dashboard. Mori's F2:plans tab showed
a hierarchical wave/plan/task tree with wave grouping, progress bars per wave, and estimated
time remaining per wave. Roko's F2 shows plans but not wave grouping.

Concrete additions:
- `wave_progress.rs` widget: proportional segments per wave with ocean gradient fill
- Wave grouping in the `plans_view.rs` left panel (group by wave, show wave header)
- Wave indicator in the header bar showing "Wave 2/5 active"
- Queue overview modal (`queue_overview.rs`) showing milestone roadmap

Depends on MO-06 for wave computation data.

**Backlog mapping**: NEW (partial overlap with #107 Plan Run UX Friction, #108 TUI Live
Feedback Gaps, and #109 TUI Realtime Streaming Parity, but those items didn't exist when
the backlog was created on 2026-08-18 and these are distinct concerns)

**Priority**: P3 — UX polish for multi-plan runs

**Size**: M (2-3d)

---

### MO-08: Conductor Supervisor Loop (Live Intervention)

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 10, R4), `04-AGENT-SYSTEM-COMPARISON.md`

**Description**: Wire the existing `roko-conductor` watchers into a live supervision loop
in the runner event loop. The infrastructure exists: `roko-conductor` has 12 watchers and
a circuit breaker, `ConductorRingSink` collects signals, but there is no periodic tick that
actually reads the ring buffer and dispatches interventions.

What needs wiring:
- Add a periodic conductor tick to the `tokio::select!` loop in `event_loop.rs`
- Read from `ConductorRingSink`
- Map conductor signals to intervention actions: nudge (SendMessage), restart (RestartAgent),
  force-advance, skip-reviews
- Expose conductor state (watcher health, recent interventions) in TUI

Mori's conductor monitored: silence timeout (180s), compile fail threshold (3), task stall
(300s), context pressure (80%), phase timeout (1800s). All of these have analogues in
Roko's existing watcher implementations.

**Backlog mapping**: NEW (related to conductor concepts but no backlog item captures the
"wire the dispatch loop" gap specifically)

**Priority**: P2 — prevents stuck agents from burning tokens indefinitely; live Roko runs
without this have no safety net when agents stall

**Size**: M (2-3d)

---

### MO-09: Batch Controller (Pause After N Plans)

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 12, R6), `05-MORI-WORKFLOW-UX.md`

**Description**: Add "pause after N plan completions" for human oversight checkpoints.
Mori's `BatchController` was 35 lines: track `completed_since_pause` counter, when it
reaches `batch_size`, pause the event loop, resume on operator signal.

Implementation:
- Add `batch_size: Option<usize>` to `RunConfig`
- Add `--batch-size N` flag to `roko plan run`
- In event loop: count completions, emit a pause event when threshold hit
- TUI shows "Batch pause — press Enter to continue" modal
- CLI (headless) mode: wait for stdin Enter or a `--batch-continue` API call

**Backlog mapping**: NEW

**Priority**: P3 — operator quality-of-life for long runs

**Size**: XS (2-4h)

---

### MO-10: Per-Plan and Per-Task Config Overrides in TOML

**Source**: `10-CONFIG-SYSTEM-COMPARISON.md`, `05-MORI-WORKFLOW-UX.md`

**Description**: Mori supported per-plan config overrides (`[plan_overrides."09"]` with
`model`, `provider`) and per-task routing metadata (`complexity_band`, `category`,
`reasoning_level`, `speed_priority`, `preferred_model`, `preferred_provider`) directly in
the task TOML. Roko's task schema has `complexity_band` but lacks the full per-task
routing override set.

Additionally, Mori had execution presets ("quality", "balanced", "cost", "speed") that
adjusted multiple config fields at once. Roko has domain profiles (coding/research/review)
but not these cost/quality-tradeoff presets.

Concrete additions:
- Extend `TaskDefinition` in `tasks.toml` schema to accept `preferred_model`,
  `preferred_provider`, `reasoning_level`, `speed_priority` routing hints
- Add `[plan_overrides.<plan_id>]` support to `roko.toml`
- Add execution presets to `roko.toml` `[runner]` section (or as CLI `--preset` flag)
- Wire per-task hints into the cascade router dispatch

**Backlog mapping**: NEW

**Priority**: P2 — enables fine-grained control that operators need for long runs

**Size**: M (2-3d)

---

### MO-11: Per-Role Context Limits and Effort Overrides

**Source**: `10-CONFIG-SYSTEM-COMPARISON.md` (sections 4, 5)

**Description**: Roko has a single global `context_limit_k` and `default_effort`. Mori had
per-role context window limits (`role_context_k` HashMap, 27 entries) and per-role effort
levels (`role_effort` HashMap, with hardcoded tier defaults: implementer/strategist/architect
= High; auditor/critic/researcher/conductor = Medium; scribe/reviewer = Low).

Per-role effort defaults are immediately actionable and prevent over-spending on lightweight
roles (scribe at High effort is wasteful) or under-serving complex roles (implementer at Low
effort misses edge cases).

Implementation:
- Add `[agent.roles]` section to `roko.toml` schema supporting per-role model, effort, and
  context_limit_k overrides
- Wire role-specific lookups into `AgentDispatchRequest` assembly in the runner
- Default tiers: inherit Mori's production-tested defaults (High for creation roles, Medium
  for review/research, Low for bookkeeping roles)

**Backlog mapping**: NEW

**Priority**: P2 — directly affects cost and quality on multi-role plan runs

**Size**: S (1d)

---

### MO-12: Dedicated Lightweight Status File (`status.json`)

**Source**: `14-STATE-PERSISTENCE-COMPARISON.md`, `05-MORI-WORKFLOW-UX.md`

**Description**: Mori wrote a lightweight `status.json` to `.mori/runs/status.json` on
every tick (250ms in TUI mode, 5s during builds). This file is the interface for external
monitors: shell scripts, `mori-supervisor.sh`, cron jobs, and status bar integrations. It
contains only: plan counts, current plan/phase/iteration, started_at, last_activity, pid,
and hang_threshold_seconds.

Roko's equivalent state is embedded in the large `state-snapshot.json` (which includes
full task lists, episode history, etc.). External monitors can't cheaply poll it. The
`roko status --quick` command exists but is a CLI invocation, not a file-based interface.

Implementation:
- Write `.roko/state/status.json` on every runner tick with fields matching Mori's schema
- Keep it minimal: `{"run_id", "plans_total", "plans_completed", "current_plan",
  "current_phase", "started_at", "last_activity", "pid", "hang_threshold_seconds"}`
- Also write during `roko serve` periodic sampling
- Document as the canonical lightweight status interface

**Backlog mapping**: NEW

**Priority**: P2 — enables external monitoring, babysitting scripts, and status integrations

**Size**: XS (2-4h)

---

### MO-13: Crash Report File (`crash-report.json`)

**Source**: `05-MORI-WORKFLOW-UX.md` (section 6)

**Description**: On panic or fatal error, write a structured `crash-report.json` to
`.roko/state/crash-report.json` containing: error message and location, full backtrace,
application state at crash time (current plan/phase/active agents/recent logs), config
summary, environment info (Rust version, OS, terminal size), and an error signature
(SHA-256 hash for dedup).

Mori's `mori-supervisor.sh` read this report and fed it to Claude/Cursor for automated
fix attempts. The roko equivalent would allow `roko diagnose --last-crash` to surface what
happened without requiring the operator to parse raw tracing output.

Implementation:
- Add a panic hook in `main.rs` that writes the crash report before the process exits
- Use the existing `tracing` infrastructure to capture recent log buffer
- Include enough state from `StateHub` to understand what was running at crash time

**Backlog mapping**: NEW

**Priority**: P3 — significantly improves debuggability but not blocking

**Size**: S (1d)

---

### MO-14: Support Artifact Freshness Checking

**Source**: `09-LEARNING-METRICS-COMPARISON.md` (section 14, item 5)

**Description**: Mori checked the mtime of support artifacts (research.md, dependency-
manifest.toml, integration.md, verify-chains/) against the plan.md and tasks.toml that
generated them. If a plan was edited after its artifacts were generated, Mori warned that
artifacts might be stale.

Roko's `plan validate` checks TOML structure but not artifact freshness. The enrichment
pipeline (`runner/enrichment.rs`) exists but doesn't surface a staleness warning.

Implementation:
- In `plan validate` and the runner preflight check, compare mtime of:
  - `plans/<id>/tasks.toml` (source of truth)
  - Any enrichment artifacts in `.roko/prd/`, `.roko/research/`, etc.
- Emit a warning (not failure) when artifacts are older than their source plan
- Add a `roko plan freshen <id>` command to re-run enrichment for stale plans

**Backlog mapping**: NEW (distinct from #85 Plan Generation TOML Reliability which focuses
on generation correctness)

**Priority**: P3 — helpful but not blocking

**Size**: XS (2-4h)

---

### MO-15: MCP Result Caching in `roko-mcp-code`

**Source**: `08-MCP-TOOL-COMPARISON.md` (sections 2, 10)

**Description**: Mori's `mori-mcp` server had a 5-minute TTL result cache keyed by
tool name + arguments. At 500 entries, it evicted LRU at every 50 calls. Roko's
`roko-mcp-code` has no caching — every call to `search_code`, `get_symbol_context`, etc.
re-runs the search.

For the self-hosting use case, the same symbol lookups are requested by multiple agents
working on related tasks. Caching reduces latency and CPU for repeated tool calls.

Implementation:
- Add a `HashMap<(String, serde_json::Value), (serde_json::Value, Instant)>` cache inside
  `roko-mcp-code/src/lib.rs`
- TTL: 5 minutes (configurable via MCP server CLI flag)
- Eviction: LRU at 500 entries, triggered every 50 calls
- Cache bypass: add `_no_cache: bool` field to tool argument schemas

**Backlog mapping**: NEW

**Priority**: P3 — performance improvement, not correctness

**Size**: XS (2-4h)

---

### MO-16: Token Savings Tracking per MCP Tool Call

**Source**: `08-MCP-TOOL-COMPARISON.md` (sections 2, 6)

**Description**: Mori's `mori-mcp` tracked estimated token savings per tool call:
`search_code` = 500 tokens saved, `get_symbol_context` = 2000, `find_references` = 3000,
`get_callers` = 5000, `workspace_map` = 5000. The `get_mcp_savings` tool exposed cumulative
savings. This was visible in the F7:inspect TUI panel.

Roko has no equivalent. Token savings tracking answers "is the MCP system saving us money?"
and guides decisions about which tools to promote in system prompts.

Implementation:
- Add a per-tool token savings estimate table to `roko-mcp-code/src/lib.rs`
- Track cumulative savings in an in-memory counter
- Expose via a `get_mcp_savings` tool (hidden from `tools/list` but callable)
- Feed into E33 telemetry as an `ObservableEvent::McpTokenSavings` variant

**Backlog mapping**: NEW

**Priority**: P3 — observability for cost optimization

**Size**: XS (2-4h)

---

### MO-17: Per-Worktree MCP Config Auto-Generation

**Source**: `08-MCP-TOOL-COMPARISON.md` (sections 4, 7)

**Description**: Mori auto-generated MCP config files into every worktree at creation time:
`.cursor/mcp.json`, `.mori/mcp-config.local.json`, `.codex/config.toml`. Each pointed to
the MCP server with `--root` scoped to the worktree path, so agents in that worktree got
code intelligence scoped to their branch's file tree.

Roko's worktree manager does not write MCP configs. The `.mcp.json` walk-up discovery finds
the repo-root config from any worktree, but the MCP server root is not scoped to the
worktree. This means agents in worktree X might see symbols from the main tree that don't
exist yet in their branch.

Implementation:
- In `WorktreeManager::create()`, after the worktree is established, write:
  - `.roko/mcp.json` (local override) pointing to `roko-mcp-code` binary with `--root <worktree_path>`
  - For Claude CLI dispatches: a worktree-local MCP config override
- Binary resolution: try `target/release/roko-mcp-code`, `target/debug/roko-mcp-code`,
  fall back to `cargo run -p roko-mcp-code --`

**Backlog mapping**: NEW (related to #66 Context Sources & Editor Integration but that item
is about editor integration broadly)

**Priority**: P3 — correctness improvement for multi-worktree code intelligence

**Size**: S (1d)

---

### MO-18: TUI Interactive Config Editing

**Source**: `10-CONFIG-SYSTEM-COMPARISON.md` (section "TUI config view"), `05-MORI-WORKFLOW-UX.md`

**Description**: Mori's F6:cfg tab was fully interactive: `j/k` navigated settings,
`h/l` cycled enum values or decreased/increased numbers, `Enter`/`Space` toggled booleans,
and pressing `s` wrote changes to `.mori/config.toml` immediately.

Roko's F6:config renders config fields and supports cursor movement and field type cycling
(per `config_view.rs`), but the persistence path (`ConfigSave` action → `roko.toml`) is
noted as unverified in `02-ROKO-TUI-ARCHITECTURE.md`. The round-trip back to `roko.toml`
needs to be implemented and tested.

Implementation:
- In `config_view.rs`, wire the `ConfigSave` action to call `roko config set <key> <val>`
  (which already exists via the config subsystem) or directly write to the config file
  using the existing `ConfigMigrator` + `LoadOptions` infrastructure
- Add a confirmation toast on successful save ("Config saved to roko.toml")
- Add undo: keep the pre-edit value in memory, `Ctrl-Z` reverts
- Verify with an integration test that editing a bool field in the TUI persists to disk

**Backlog mapping**: NEW (the F6 config view exists but its persistence path is unverified)

**Priority**: P2 — operators need to adjust config during runs without leaving the TUI

**Size**: S (1d)

---

### MO-19: Agent Status Panel in TUI (Role × Model × Context)

**Source**: `10-CONFIG-SYSTEM-COMPARISON.md` (section 4), `04-AGENT-SYSTEM-COMPARISON.md`

**Description**: Mori's F6:cfg tab right-side panel showed a dense agent status table:
one row per agent role (27 roles), showing status icon (active/has-tokens/idle), short
model name, token usage (used/context_window), context percentage, turn count, and effort
level. This made it immediately visible whether agents were stalled, which models were
being used, and where context pressure was building.

Roko's F3:agents tab shows the active roster but not the full 27-role status grid with
context gauges. The F1:dashboard "Agents" sub-tab shows a roster but without the
context window utilization column.

Implementation:
- Add a `context_gauge` column to the agents roster in `agents_view.rs` and the F1 agents
  sub-tab
- Show `input_tokens / context_limit_k * 1000` as a gauge (existing `context_gauge.rs`
  widget can be reused)
- Add turn count column
- Add effort level column (derived from role defaults + config overrides)
- The data is already available in `DashboardSnapshot` agent entries — this is a rendering
  gap, not a data gap

**Backlog mapping**: NEW

**Priority**: P2 — context pressure awareness prevents token overspend and stall blindness

**Size**: S (1d)

---

### MO-20: F7:inspect "Single Pane of Glass" for Learning + MCP

**Source**: `09-LEARNING-METRICS-COMPARISON.md` (section 12), `08-MCP-TOOL-COMPARISON.md`

**Description**: Mori's F7:inspect was a three-column production TUI panel showing
everything in one view: (1) MCP server config paths, backend status, worktree routing;
(2) AST index file/symbol/ref counts and resolution percentage; (3) episodes, playbook
rules, routing coverage, route hints, prompt stats, registries, knowledge utilization,
tool call counts.

Roko's F7:inspect exists (`context_view.rs`, 1,110 LOC) but has four sections: system
health, token burn per role, cost per model, and cascade router + alerts. The MCP server
status, playbook rule count, routing coverage percentage, and AST index stats are NOT
visible in the F7 view.

The data exists — efficiency events, cascade router JSON, knowledge store, MCP tool call
counts — it just hasn't been assembled into the F7 panel.

Concrete additions to `context_view.rs`:
- Add a "MCP Runtime" sub-panel: server config path (exists or missing), tool count,
  recent tool call count, index file/symbol counts if available
- Add a "Learning" sub-panel: episode count (pass/fail), playbook rule count (learned vs
  manual), routing coverage %, cascade router stage
- Add a "Prompt Stats" sub-panel: avg prompt tokens per role (from efficiency events),
  avg context window utilization

**Backlog mapping**: NEW (partially addressed by #108 TUI Live Feedback Gaps but that item
is broader; this is specifically the F7 pane composition problem)

**Priority**: P2 — the TUI's primary observability panel is significantly below Mori's
production quality

**Size**: M (2-3d)

---

### MO-21: CorticalState / Cognitive Autonomy Wiring

**Source**: `11-CYBERNETIC-FEATURES-AUDIT.md` (feature 3: Cognitive Autonomy — PARTIAL)

**Description**: `CorticalState` and `heartbeat.rs` (2,717 LOC) are built but never
instantiated by any production code path. The cognitive autonomy subsystem (E23 10/10
accepted) has lifecycle type-state, behavioral vitality, energy accounting, and EFE routing
built, but no caller creates a `CorticalState` instance at runtime.

What needs wiring:
- Identify where in the runner or agent factory `CorticalState` should be created
  (likely once per plan run in `event_loop.rs`)
- Wire the heartbeat tick into the event loop's periodic timer
- Feed energy accounting from `AgentEfficiencyEvent` (cost per turn) into the energy fields
- Connect `EfeRouter` decisions to agent dispatch priority ordering

This is not a feature build — the code exists at production quality. It's a wiring task.

**Backlog mapping**: NEW

**Priority**: P3 — architectural wiring, not user-visible UX

**Size**: M (2-3d)

---

### MO-22: EnrichedCell in Main Dispatch Path

**Source**: `11-CYBERNETIC-FEATURES-AUDIT.md` (feature 2: Cross-cut Functors — PARTIAL)

**Description**: `EnrichedCell` (the cross-cut functor wrapper, E44) is built and tested
but NOT used by the main `dispatch_agent_with()` call path in `event_loop.rs`. The runner
dispatches directly through the `SharedAgentFactory` without going through the functor
pipeline.

The E44 cross-cut functors (Memory/Daimon/Dreams/Safety, six transforms, conflict VCG,
and the live non-blocking gate-failure cascade) are accepted at 8/8 but the enrichment
wrapper is bypassed by the primary execution path.

This is a wiring task: change `event_loop.rs`'s dispatch call to route through
`EnrichedCell` rather than directly calling the factory.

**Backlog mapping**: NEW

**Priority**: P3 — all six transforms (memory, daimon, dreams, safety, VCG, cascade) would
become active for every dispatch

**Size**: M (2-3d)

---

### MO-23: Native Agent-to-Telemetry Publication

**Source**: `11-CYBERNETIC-FEATURES-AUDIT.md` (features 6, 8: Telemetry Lens and Agent
Groups)

**Description**: The E33 Telemetry Lens system is complete (9/9) with 39 event variants,
but native Agent publication into the observation ingress boundary is separate scope —
agents don't directly publish `ObservableEvent` values. Instead, the runner publishes on
behalf of agents after each dispatch completes.

What's missing: during a long agent turn (before the turn completes), the agent itself
cannot publish intermediate telemetry (e.g., "I'm about to call tool X", "I found an
error pattern"). This means real-time telemetry lag = turn duration.

The implementation requires: a bidirectional channel from `ToolLoop` / `ClaudeCliAdapter`
back to the `LensExecutor` that allows mid-turn event publication without completing the
turn.

**Backlog mapping**: NEW (GAPS.md notes this as "direct native Agent publication remains
separate product integration scope" but no backlog item tracks it)

**Priority**: P3 — telemetry fidelity improvement

**Size**: L (3-5d)

---

### MO-24: roko-gateway Wired Into Runner-v2

**Source**: `11-CYBERNETIC-FEATURES-AUDIT.md` (feature 8: Inference Gateway — PARTIAL)

**Description**: The E26 inference gateway (`roko-gateway`) is a complete 9-stage pipeline
(routing/fallback, exact and semantic caches, tool/output/thinking controls, convergence,
cost accounting, key rotation, three-level backpressure, handles, batches, events) but
runner-v2 bypasses it entirely. The runner dispatches directly through `roko-agent`
provider adapters.

The gateway sits between the runner and the provider adapters and provides: caching
(avoid duplicate LLM calls for identical prompts), key rotation (distribute load across
API keys), backpressure (prevent rate limit cascades), and cost accounting at the network
level.

Wiring path: change `AgentDispatcherV2::dispatch()` to route through `roko-gateway`
instead of directly calling provider adapters.

**Backlog mapping**: NEW (GAPS.md notes "runner-v2 bypasses roko-gateway" but no backlog
item tracks the wiring task)

**Priority**: P2 — caching alone would reduce costs significantly on repeated similar tasks

**Size**: L (3-5d)

---

### MO-25: Post-Merge Regression Testing

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 11)

**Description**: Mori ran workspace-wide regression tests after merging each plan to the
batch branch. This caught cross-plan compilation errors and test breakage that passed
per-plan gates but failed when combined.

Roko runs gates per-plan (in the plan worktree) but does not run gates on the merged
result. A compile gate can pass in the plan worktree (where the plan's changes are isolated)
but fail after merging (where another plan's changes are present).

Implementation:
- After `PlanMerger::merge()` succeeds and the branch is merged, run a minimal gate set
  (at least `cargo check`) on the target branch
- Integrate with the `RegressionGate` abstraction in `runner/merge.rs`
- On failure: emit a regression event, optionally roll back the merge

**Backlog mapping**: NEW

**Priority**: P2 — prevents silent cross-plan compilation breakage

**Size**: M (2-3d)

---

### MO-26: File Overlap Analysis for Parallel Plans

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 3, R10)

**Description**: Mori's plan DAG had `file_overlap_analysis()` that detected when two
parallel plans (same wave) touched the same crate. Overlapping plans were warned about or
serialized to prevent merge conflicts.

Roko's `MergeQueue` has file-conflict tracking during merge (`ready_batch()` computes
non-conflicting merges) but there is no pre-run analysis during plan loading that warns
"plans X and Y both touch `roko-core` and may conflict."

Implementation:
- During `plan_loader::load_plans()`, compute crate overlaps from `tasks.toml`
  `files` and `crates_touched` metadata
- Warn if two wave-parallel plans touch the same crate
- Optionally serialize conflicting plans (demote them to separate waves)

**Backlog mapping**: NEW

**Priority**: P3 — correctness improvement for multi-plan runs

**Size**: XS (2-4h)

---

### MO-27: Critical Path Analysis and ETA

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 3, R11)

**Description**: Mori computed the critical path (longest weighted dependency chain) through
the plan DAG and used it for ETA estimation. Roko has no ETA display.

The header bar could show "Est. 4h 20m remaining" if we knew the critical path weight.
Wave computation (MO-06) is a prerequisite.

Implementation:
- Port `critical_path()` from Mori's `orchestrator/dag.rs`
- Sum `estimated_minutes` from task definitions along the critical path
- Display in TUI header bar and CLI `roko status` output
- Update dynamically as plans complete

**Backlog mapping**: NEW

**Priority**: P3 — UX improvement for operator planning

**Size**: XS (2-4h)

---

### MO-28: Per-Plan Routing Hints in Task TOML

**Source**: `09-LEARNING-METRICS-COMPARISON.md` (section 3), `05-MORI-WORKFLOW-UX.md`

**Description**: Mori's task TOML supported per-task routing annotations:
`complexity_band`, `category`, `routing_band`, `routing_source`, `research_before_edit`,
`fixture_keys`, `sidecar_requirements`. These hints were used by `heuristic_routing_band_for_task()`
to classify complexity and route to appropriate models before learned routing kicks in.

Roko's task TOML has `complexity_band` but not `category`, `routing_band`, or `research_before_edit`.
The cascade router uses a 18-dimensional context vector but `category` is one of the
8-dimensional one-hot features — if it's always empty (no per-task category), the router
is working with less signal.

Implementation:
- Extend `TaskDefinition` schema with optional `category: Option<TaskCategory>` and
  `research_before_edit: Option<bool>`
- Map these to the cascade router context vector features
- Document the supported category values: implementation, integration, testing, docs, etc.

**Backlog mapping**: NEW (partially overlaps with #84 Cascade Router Task Category Awareness
but that item focuses on the router's existing data, not on surfacing new signal from TOML)

**Priority**: P2 — improves routing signal quality from the start of a run

**Size**: S (1d)

---

### MO-29: Agent-Level sccache / CARGO_BUILD_JOBS Resource Limits

**Source**: `04-AGENT-SYSTEM-COMPARISON.md` (section 12, item 6)

**Description**: Mori set `CARGO_BUILD_JOBS=2` and `RUSTC_WRAPPER=sccache` in the
environment of each spawned agent subprocess. With 15-20 concurrent agents each running
`cargo check`, the build parallelism multiplied: 20 agents × default 8 build jobs = 160
concurrent rustc processes, causing CPU exhaustion and thrashing.

Roko's agent dispatch (`ClaudeCliAdapter`, `CursorAcpAdapter`) sets environment variables
for API keys and base URLs but does not cap cargo parallelism. With multiple concurrent
tasks in `event_loop.rs`, this can cause system overload during gate execution.

Implementation:
- Add `CARGO_BUILD_JOBS=2` to the environment of all agent subprocess spawns
- If sccache is available on PATH, set `RUSTC_WRAPPER=sccache`
- Make these configurable in `roko.toml` under `[gates]` or `[runner]`

**Backlog mapping**: NEW

**Priority**: P1 — CPU exhaustion with multiple concurrent agents causes real failures

**Size**: XS (1h)

---

### MO-30: Structured JSON Output for Reviewer Roles

**Source**: `04-AGENT-SYSTEM-COMPARISON.md` (section 12, item 3)

**Description**: Mori passed `--json-schema <review_json_schema()>` to the Claude CLI
when spawning reviewer roles (QuickReviewer, Auditor, Critic). This forced structured
JSON output with approve/revise/skip verdicts that could be parsed programmatically.

Roko has `PromptPolicy.output_format` in the role manifest but the wiring from
`output_format` through to the CLI dispatch `--output-format` or `--json-schema` flag
is partial for all provider adapters.

Implementation:
- In `ClaudeCliAdapter::build_command()`, check if the dispatch context's role has
  `PromptPolicy.output_format == OutputFormat::JsonSchema(schema)` and if so, add
  `--output-format json` or `--json-schema <path>` to the command
- Define `review_json_schema()` matching Mori's schema: `{verdict: "approve"|"revise"|"skip", issues: [...]}`
- Parse structured review output in the event loop's gate result handler

**Backlog mapping**: NEW

**Priority**: P2 — enables reliable review verdict parsing instead of LLM output text matching

**Size**: S (1d)

---

### MO-31: Supervisor Auto-Recovery Script

**Source**: `05-MORI-WORKFLOW-UX.md` (section 1, alternatives)

**Description**: Mori had `mori-supervisor.sh`: a self-healing wrapper that caught panics,
read the crash report, fed it to Claude for auto-fix, rebuilt, and restarted. It had a
circuit breaker (stop after 10 total failures or 3 same-error failures) and sent macOS
notifications on circuit-breaker trip.

Roko's equivalent would be a `roko-supervisor.sh` script at the repo root that:
1. Runs `roko plan run` and watches the exit code
2. On failure, reads `.roko/state/crash-report.json` (see MO-13)
3. Feeds the crash summary to `roko run "fix the crash: <context>"` to generate a fix plan
4. If the fix plan succeeds, restarts the original run
5. Circuit breaker: stop after N same-error crashes

**Backlog mapping**: NEW

**Priority**: P3 — convenience for unattended long runs

**Size**: S (1d, mostly shell scripting)

---

### MO-32: TUI Resizable Panes

**Source**: `02-ROKO-TUI-ARCHITECTURE.md` (section 11, "UX Polish Missing")

**Description**: All split ratios in Roko's TUI are fixed percentages (38%/62%, 35%/65%,
32%/68%). There is no drag-to-resize and no keyboard resize (e.g., `<`/`>` to adjust the
left/right pane ratio). Mori also had fixed ratios, but this is a UX improvement both
systems lack.

Implementation:
- Add a `PaneConfig` struct to `TuiState` storing override ratios per tab
- `<` / `>` keys adjust the ratio by 5% increments
- Persist to `.roko/state/tui-prefs.json`
- Clamp to 20%-80% range

**Backlog mapping**: NEW (related to #71 TUI Design System Alignment but that item is about
color/spacing consistency; pane resizing is separate)

**Priority**: P3 — nice-to-have UX

**Size**: S (1d)

---

### MO-33: `mori --validate` Equivalent (`roko plan validate --dag`)

**Source**: `05-MORI-WORKFLOW-UX.md` (section 6), `03-EXECUTION-MODEL-COMPARISON.md`

**Description**: Mori's `--validate` mode printed: plans discovered with dependency info,
task files loaded with counts, unified task DAG analysis (node count, max parallelism width,
critical path minutes), dangling references, and execution wave breakdown. It was the
pre-flight check before committing to a run.

Roko's `roko plan validate <dir>` checks TOML structure but does not show DAG analysis,
parallelism width, wave breakdown, or estimated duration.

Implementation:
- Extend `roko plan validate` to run full DAG analysis and print:
  - Total plans, total tasks, dependency edge count
  - Wave breakdown (which plans in each wave, parallelism width per wave)
  - Critical path (plan sequence + estimated minutes)
  - Dangling `depends_on_plan` references (plans that reference non-existent plans)
  - Plans that could run in parallel but share crates (warning)

Depends on MO-06 (wave computation).

**Backlog mapping**: NEW

**Priority**: P2 — essential pre-run operator check

**Size**: S (1d)

---

### MO-34: Global Key for "Force Advance" and "Reset Plan" in TUI

**Source**: `05-MORI-WORKFLOW-UX.md` (section 5), `03-EXECUTION-MODEL-COMPARISON.md`

**Description**: Mori had key bindings for operator recovery actions:
- `Ctrl-X`: Force advance selected plan (commit what's there and proceed)
- `s`: Soft retry failed plan (preserve completed tasks, only retry failures)
- `S`: Repair plan (preserve work, clean worktree)
- `R`: Repair plan (clean start, discard worktree)
- `z`: Diagnose plan (inspect state without modifying)
- `Ctrl-D`: Reset selected plan
- `Ctrl-G`: Git reconcile (commit/merge/prune)
- `Ctrl-A`: Approve all pending approvals
- `m`: Merge batch to main (with confirmation)

Roko's TUI has approval/inject modals but lacks the operator recovery actions. The
underlying operations exist (force advance via config, worktree repair via CLI) but
they are not accessible from the TUI during a live run.

Implementation:
- Add `TuiAction` variants for: ForceAdvancePlan, SoftRetryPlan, ResetPlan, DiagnosePlan,
  GitReconcile, ApproveAll, MergeToMain
- Bind them to keys in `input.rs` (F2 tab for plan-specific, global for approve-all)
- Each action triggers a channel message to the runner event loop
- Add confirmation dialogs for destructive actions (reset, merge)

**Backlog mapping**: NEW

**Priority**: P1 — without these, a stuck agent requires `Ctrl-C` and restart

**Size**: M (2-3d)

---

### MO-35: TUI Notification Toast for Key Events

**Source**: `05-MORI-WORKFLOW-UX.md` (section 7), `02-ROKO-TUI-ARCHITECTURE.md`

**Description**: Mori showed toast-style notifications for key events: "Plan 09: compile
gate PASS", "Review cap hit after 5 revisions, force-committing", "Agent spawn failed,
retrying with fallback model". These were temporary overlays with severity (Info/Warn/Error)
and a TTL that dismissed them automatically.

Roko has a `notification.rs` modal module but the toast system is noted as "underused;
most errors are silent" in the TUI architecture audit.

Implementation:
- Ensure every significant runner event publishes a `DashboardEvent::Notification` with
  severity and message
- Key events to notify: gate pass, gate fail, plan complete, plan failed, agent stall
  detected, spawn failure with fallback, merge success, merge conflict
- The notification overlay already exists; the gap is wiring runtime events to it

**Backlog mapping**: NEW (notification overlay exists but is disconnected from runtime events)

**Priority**: P2 — silent failures make the TUI feel broken

**Size**: S (1d)

---

### MO-36: `mori ingest` Equivalent (Live Directive Injection)

**Source**: `05-MORI-WORKFLOW-UX.md` (sections 5, 7)

**Description**: Mori had `mori ingest "<directive>"` (CLI) and a TUI type-in mode for
injecting directives into running agents. Directives were classified (agent nudge, plan
amendment, new task, context only) and routed to the appropriate running agent or plan.
A file-based drop directory (`.mori/ingest/`) also worked.

Roko has an `Inject` modal in the TUI and `roko inject <session> <payload>` CLI but:
- The TUI inject modal requires the orchestrator approval channel to be connected
- The CLI inject command sends to a session, not to a specific agent
- There is no classification (nudge vs amendment vs new task)

Implementation:
- Add classification to the inject path: pattern-match the directive text to determine
  if it's a nudge (append to agent's next prompt), task amendment (add task to plan),
  or context injection (update knowledge store)
- Wire the TUI inject modal's submission to the runner's `StateHub` approval channel
  even in standalone (non-connected) mode via `roko serve`
- Add drop-directory support: watch `.roko/ingest/` for new files, process and delete

**Backlog mapping**: NEW (the inject infrastructure exists but the classification and
routing are missing)

**Priority**: P2 — steering running agents without stopping the run is essential for
the self-hosting workflow

**Size**: M (2-3d)

---

### MO-37: Structured Error Pattern Sharing Between Parallel Agents

**Source**: `05-MORI-WORKFLOW-UX.md` (sections 6, 7)

**Description**: Mori wrote discovered gate failure patterns to `.mori/runs/discovered-patterns.json`
and parallel agents read this file to avoid re-discovering the same errors. If agent A
found "missing import for `Signal`", agent B (running in parallel) would see it in its
context and avoid the same mistake.

Roko has `ErrorPatternStore` in `roko-learn` that persists discovered error patterns, but
parallel agents running in the same plan run do not read each other's discoveries mid-run.
The pattern store is read at dispatch time (when assembling the system prompt) but only
from patterns discovered in previous runs.

Implementation:
- Add a shared in-memory `ErrorPatternStore` to `SharedAgentFactory` that persists across
  all dispatches within a single run
- When a gate fails, extract error patterns and write to the shared store immediately
  (not just at end-of-run)
- On next dispatch for any task in the same run, read from the shared store and inject
  fresh patterns into the system prompt

**Backlog mapping**: NEW

**Priority**: P2 — directly reduces repeated error costs in parallel agent runs

**Size**: S (1d)

---

### MO-38: Review Cap with Force-Commit

**Source**: `03-EXECUTION-MODEL-COMPARISON.md` (section 5)

**Description**: Mori's `PlanPipeline` had `max_iterations` review cycles. After that many
consecutive revise verdicts, it emitted `ReviewCapHit` and force-committed whatever was
in the worktree. This prevented infinite review loops from consuming unlimited tokens.

Roko has `max_retries` in `DagConfig` that limits task retries, but there is no review-
cycle cap that causes a force-commit after N review rounds.

Implementation:
- Add `max_review_cycles: u32` (default 3) to `RunConfig`
- Track per-plan review cycle count in the event loop
- After `max_review_cycles` consecutive REVISE verdicts, emit `ReviewCapHit` and advance
  to commit regardless of reviewer verdict
- Log and notify via TUI toast

**Backlog mapping**: NEW

**Priority**: P2 — prevents infinite loops that exhaust budgets

**Size**: XS (2-4h)

---

### MO-39: `get_plan_context` MCP Tool

**Source**: `08-MCP-TOOL-COMPARISON.md` (section 10)

**Description**: Mori's `mori-mcp` had a `get_plan_context` tool that replaced injecting
entire PRD documents, decomposition files, verify-tasks, and review-tasks into the system
prompt. Agents called the tool on demand and got targeted context for the specific context
type they needed (11 types: brief, tasks, research, dependencies, fixtures, integration,
decomposition, verify, review, docs, summaries).

Roko's `roko-mcp-code` has `get_context` (assembly by token budget) and `get_index_stats`
but no plan-specific context retrieval. Plan context (the PRD extract, task decomposition,
brief, verification criteria) is injected statically into the system prompt, consuming
context window budget even for tasks that don't need it all.

Implementation:
- Add `get_plan_context` to `roko-mcp-code/src/lib.rs`
- Parameters: `plan_id`, `context_type` (one of brief/tasks/research/brief/etc.)
- Implementation: read from `.roko/` enrichment artifacts for the plan
- Register as a standard tool in the tool inventory

**Backlog mapping**: NEW

**Priority**: P3 — context window efficiency improvement

**Size**: S (1d)

---

### MO-40: Prompt Log Files (Full Text Storage)

**Source**: `09-LEARNING-METRICS-COMPARISON.md` (section 5)

**Description**: Mori stored the full prompt text per task invocation in
`.mori/memory/prompt-logs/<id>.json` including: UUID, timestamp, plan, task, role,
context strategy, total tokens (via tiktoken), per-section breakdown, and the full prompt
text.

Roko tracks section metadata (`PromptSectionMeta` with name, tokens, priority,
was_truncated, was_dropped) in `AgentEfficiencyEvent` but does NOT store the full prompt
text. This makes offline analysis hard: "what exactly did the implementer see?" requires
re-assembling from multiple sources.

Implementation:
- Add an optional prompt log mode: `[runner] log_prompts = true` in `roko.toml`
- When enabled, write `.roko/prompt-logs/<task-id>-<attempt>.json` with full prompt text,
  assembled sections, and metadata
- Default off (disk cost at 20+ agents is significant)
- Integrate with `roko diagnose` (MO-03) to include the most recent prompt log

**Backlog mapping**: NEW

**Priority**: P3 — debugging aid for understanding why agents make specific choices

**Size**: S (1d)

---

## Part 3: Summary Table of New Items

| ID | Title | Source | Priority | Size |
|---|---|---|---|---|
| MO-01 | TUI Headless Snapshot Mode | CHECKLIST 0.1 | P1 | M |
| MO-02 | Continuous Screenshot During Runs | CHECKLIST 0.3 | P2 | S |
| MO-03 | `roko diagnose <plan-id>` Command | CHECKLIST 0.5 | P1 | M |
| MO-04 | `--json` Output for Core CLI Commands | CHECKLIST 0.4, 05 | P2 | S |
| MO-05 | Queue Manifest / Milestone System | 03 | P2 | L |
| MO-06 | Plan-Level Wave Computation | 03 | P2 | M |
| MO-07 | Wave Progress Visualization in TUI | 03, 02 | P3 | M |
| MO-08 | Conductor Supervisor Loop (Live Intervention) | 03, 04 | P2 | M |
| MO-09 | Batch Controller (Pause After N Plans) | 03, 05 | P3 | XS |
| MO-10 | Per-Plan and Per-Task Config Overrides | 10, 05 | P2 | M |
| MO-11 | Per-Role Context Limits and Effort Overrides | 10, 04 | P2 | S |
| MO-12 | Dedicated Lightweight Status File | 14, 05 | P2 | XS |
| MO-13 | Crash Report File (`crash-report.json`) | 05 | P3 | S |
| MO-14 | Support Artifact Freshness Checking | 09 | P3 | XS |
| MO-15 | MCP Result Caching in `roko-mcp-code` | 08 | P3 | XS |
| MO-16 | Token Savings Tracking per MCP Tool | 08 | P3 | XS |
| MO-17 | Per-Worktree MCP Config Auto-Generation | 08 | P3 | S |
| MO-18 | TUI Interactive Config Editing (Persistence) | 10, 02 | P2 | S |
| MO-19 | Agent Status Panel (Role × Context × Effort) | 10, 04 | P2 | S |
| MO-20 | F7:inspect "Single Pane of Glass" | 09, 08 | P2 | M |
| MO-21 | CorticalState / Cognitive Autonomy Wiring | 11 | P3 | M |
| MO-22 | EnrichedCell in Main Dispatch Path | 11 | P3 | M |
| MO-23 | Native Agent-to-Telemetry Publication | 11 | P3 | L |
| MO-24 | roko-gateway Wired Into Runner-v2 | 11 | P2 | L |
| MO-25 | Post-Merge Regression Testing | 03 | P2 | M |
| MO-26 | File Overlap Analysis for Parallel Plans | 03 | P3 | XS |
| MO-27 | Critical Path Analysis and ETA | 03 | P3 | XS |
| MO-28 | Per-Plan Routing Hints in Task TOML | 09, 05 | P2 | S |
| MO-29 | sccache / CARGO_BUILD_JOBS Resource Limits | 04 | P1 | XS |
| MO-30 | Structured JSON Output for Reviewer Roles | 04 | P2 | S |
| MO-31 | Supervisor Auto-Recovery Script | 05 | P3 | S |
| MO-32 | TUI Resizable Panes | 02 | P3 | S |
| MO-33 | `roko plan validate --dag` Analysis | 05, 03 | P2 | S |
| MO-34 | Operator Recovery Keys in TUI | 05, 03 | P1 | M |
| MO-35 | TUI Notification Toast Wiring | 05, 02 | P2 | S |
| MO-36 | Live Directive Injection (Classify + Route) | 05 | P2 | M |
| MO-37 | Error Pattern Sharing Between Parallel Agents | 05 | P2 | S |
| MO-38 | Review Cap with Force-Commit | 03 | P2 | XS |
| MO-39 | `get_plan_context` MCP Tool | 08 | P3 | S |
| MO-40 | Prompt Log Files (Full Text Storage) | 09 | P3 | S |

---

## Part 4: Observations About What's Already Good

The mori-old analysis also found areas where Roko **exceeds** Mori's production quality.
These do not need backlog items.

1. **Resume safety**: Roko's `TaskDefFingerprint` + drift detection is strictly better than
   Mori's basic task-level resume. Keep it.

2. **Worktree security**: Roko's creation markers, flock, and compare-and-swap branch creation
   are substantially more robust than Mori's `git worktree add`.

3. **Model routing**: LinUCB CascadeRouter (12K+ LOC) vs Mori's highest-pass-rate heuristic
   (298 LOC). No gap here — Roko is ahead.

4. **Gate pipeline**: 19 gates across 7 rungs with adaptive EMA thresholds vs Mori's 11 gates.
   No gap.

5. **Safety layer**: Mori had per-role CLI flag restrictions. Roko has 21 safety submodules,
   trust-origin IFC, immune graph, corrigibility. No gap.

6. **Prompt assembly**: 9-layer composable SystemPromptBuilder vs Mori's flat ~500-token
   string. No gap.

7. **Learning depth**: roko-learn (123K LOC, 65 modules) vs mori's 5.8K LOC. No gap.

8. **Provider diversity**: 11 provider kinds vs Mori's 3 backends. No gap.

9. **Config evolution**: Field-level merge, schema versioning, migration pipeline, provenance
   tracking. Mori had none of this. No gap.

10. **A/B experiments**: Model + prompt experiments with statistical significance. Mori had
    neither. No gap.
