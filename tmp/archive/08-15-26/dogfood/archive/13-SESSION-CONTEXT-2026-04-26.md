# Session Context — 2026-04-26 (Full Day)

> Written for someone with zero prior context. This captures an entire day of
> dogfooding, debugging, architecture analysis, and refactoring on roko.

## What Roko Is

Roko is a Rust toolkit (18 crates, ~177K LOC) that builds agents which build themselves.
The core loop: read PRDs → generate plans → execute tasks via LLM agents → verify with
gates → persist results → learn from outcomes. The plan runner (`roko plan run`) is the
heart — it takes a `tasks.toml` file, dispatches agent tasks, runs verification gates,
and persists state.

## What Happened Today

We tried to dogfood `roko plan run` on a real plan. It was broken in dozens of ways.
We spent the day debugging, patching, analyzing, and ultimately deciding the glue layer
(`orchestrate.rs`, 21K lines) needs replacement with a cleaner architecture.

### Timeline

1. **Morning**: Applied 4 initial fixes (skip_enrichment, Ctrl+C kill, config warning dedup, health endpoint). Ran the plan. TUI crashed — `ws_client.rs` tried `tokio::spawn` from a non-tokio thread. Fixed.

2. **Run 2**: Enrichment still ran despite `skip_enrichment = true`. Root cause: `plans_dir()` resolves to `plans/` (top-level) instead of `.roko/plans/` — task tracker never loaded. Fixed with fallback path.

3. **Run 3**: 6 phantom plans appeared — `discover_plans()` scans `.md` files and found enrichment artifacts (`brief.md`, `research.md`, etc.). Fixed by detecting `tasks.toml` and skipping discovery.

4. **Run 4**: `from_plans_dir` double-nested the path (`plans_dir.join(base)` when plans_dir already WAS the plan). Enrichment overwrote `tasks.toml`. Fixed by rewriting to parent directory.

5. **Deep analysis**: Every fix revealed another broken assumption. Analyzed `orchestrate.rs` structure (21K lines, 250 methods, god object). Compared with mori's architecture (direct RunState, no indirection). Wrote spec for runner v2.

6. **Afternoon**: Runner v2 session completed (2,180 lines in `runner/` module). Parallel refactoring session did config/schema decomposition + main.rs decomposition + cascade_router split. Also applied C1-C8 TUI data field fixes.

7. **Testing**: Calculator test plan in fresh repo — failed because no git repo (auto-init fix added). Failed again — TUI still shows "plan plan" and "-" for model because the old `orchestrate.rs` path was still being used despite runner v2 existing.

8. **Final fix**: Wired runner v2 as the default for `plan run --approval`. This bypasses the entire broken DashboardEvent indirection pipeline.

## The Root Architectural Problem

**Mori** (the predecessor): Single-threaded event loop. `RunState` is a flat struct mutated directly. When a task starts: `state.task_title = task.title`. TUI renders `&RunState` each frame. 1 layer. Never breaks.

**Roko's orchestrate.rs**: Four layers of indirection:
1. Emit `ServerEvent` (roko-serve/events.rs)
2. Convert to `DashboardEvent` (orchestrate.rs:18094 — LOSSY conversion drops fields)
3. Apply to `DashboardSnapshot` (dashboard_snapshot.rs:800)
4. Convert to `TuiState` (tui/state.rs:1978)
5. Render in view (tui/views/*.rs)

Miss ANY step for ANY field → broken TUI. 15 fields × 5 steps = 75 places to get wrong.
We found and patched ~15 of these today. There are probably more.

**The fix**: Runner v2 (`crates/roko-cli/src/runner/`) uses mori's pattern — direct RunState mutation, one layer, streaming agent output. It's wired as the default for `plan run --approval` as of the last change in this session.

## Current State of the Code

### What's Changed (uncommitted on wp-arch2)

**orchestrate.rs** — many patches:
- `skip_enrichment` support in `handle_enriching()`
- `ensure_task_tracker()` checks both `plans_dir()` and `.roko/plans/` fallback
- `dispatch_agent_with()` same fallback
- `from_plans_dir()` detects plan dir (has tasks.toml) vs parent dir
- `force_shutdown()` sends SIGTERM to process group
- Grace period reduced from 30s to 3s
- `AgentOutput` event emitted after dispatch
- `TaskStarted` event carries title from task tracker
- `AgentSpawned` event carries model
- `EfficiencyEvent` tokens/cost emitted after dispatch
- `ensure_git_repo()` auto-inits git if missing
- `git_changed_files()` returns empty instead of crashing on non-repo
- Agent-to-task binding fixed (was matching `role == phase`, now matches by agent_id prefix)

**runner/ module** (NEW, 2,180 lines):
- `event_loop.rs` — `tokio::select!` loop (like mori's sequential.rs)
- `agent_stream.rs` — `--stream-json` line-by-line parsing
- `agent_events.rs` — direct RunState mutation
- `gate_dispatch.rs` — background gate tasks
- `persist.rs` — atomic writes after every task
- `plan_loader.rs` — just reads tasks.toml (no discovery)
- `tui_bridge.rs` — publishes DashboardEvents to StateHub
- `state.rs` — RunState struct
- `types.rs` — AgentEvent, GateCompletion, RunConfig

**main.rs** — `plan run --approval` now calls runner v2 instead of old PlanRunner

**commands/ module** (NEW) — main.rs decomposed into subcommand files (from parallel refactoring session)

**config/schema.rs** — decomposed into submodules (agent.rs, budget.rs, etc.)

**cascade_router.rs** — split into cascade/ submodules

**dashboard_snapshot.rs** — TaskState.title field added, AgentSpawned.model field added, agent-to-task binding fixed

**task_parser.rs** — TaskMeta.skip_enrichment field added

### What's NOT Changed

- roko-gate (all gates work, not touched)
- roko-agent (backends work, not touched except safety layer)
- roko-learn (split into submodules by refactor session)
- roko-compose (not touched)
- roko-orchestrator (ParallelExecutor is solid, not touched)
- TUI views (rendering code not changed — relies on data flow being correct)

## Files in tmp/ — What's What

### tmp/dogfood/ — Issue Tracking
| File | What | Current? |
|------|------|----------|
| `00-INDEX.md` | Master checklist of all issues (50 items, checkboxes) | YES — source of truth |
| `01-endpoint-audit.md` | HTTP endpoint gaps | Partially resolved |
| `02-plan-runner-gaps.md` | Original plan runner bugs | Mostly resolved |
| `03-resource-management.md` | OOM from zombie processes | Resolved (process group kill) |
| `04-run2-observations.md` | Run 2 findings | Historical |
| `05-mori-vs-roko-agent-wiring.md` | Deep mori comparison | Still relevant — root cause analysis |
| `06-run2-deep-findings.md` | plans_dir bug, routing, memory | plans_dir fixed, others open |
| `07-orchestrate-analysis.md` | 21K-line god object analysis + refactor plan | Superseded by runner v2 |
| `08-statehub-tui-audit.md` | End-to-end StateHub → TUI audit (C1-C9 changes) | Partially implemented; superseded by runner v2 for --approval mode |
| `09-MAY6-DEMO-BUILD.md` | May 6 demo build checklist | Added by user |
| `10-RUNTIME-FIXES.md` | Runtime fixes | Added by user |
| `11-LANDING-PAGE-UPDATES.md` | Landing page | Added by user |
| `12-DECK-AND-MEMO.md` | Deck build | Added by user |
| `archive/resolved-2026-04-26.md` | Resolved issues with details | YES |

### tmp/unified/ — Architecture Spec
| File | What |
|------|------|
| `00-INDEX.md` | Master index for unified architecture (21 spec docs + roadmap) |
| `01-SIGNAL.md` through `21-ROADMAP.md` | Canonical spec for the agent economy |
| `22-PLAN-RUNNER-V2.md` | **Runner v2 architecture spec** (written this session) |

### tmp/unified-migration/ — Migration Phases
| File | What |
|------|------|
| `00-INDEX.md` | 4-phase migration plan |
| `01-PHASE-0-PREP.md` | Pre-migration cleanup (14 items, ~0 done) |
| `02-PHASE-1-KERNEL.md` | Type renames, Cell trait, protocols (~195 items) |
| `03-PHASE-2-ENGINE.md` | Graph engine, agent runtime (~138 items) |
| `04-PHASE-3-ECONOMY.md` | On-chain, arenas (~152 items) |

### tmp/unified-migration-runner/ — Implementation Prompts
| File | What | Status |
|------|------|--------|
| `00-IMPLEMENTATION-INDEX.md` | Index of all implementation plans | Current |
| `RUNNER-V2-IMPLEMENTATION.md` | 45-task runner v2 implementation plan | **Executed** (runner built) |
| `MAIN-RS-DECOMPOSITION.md` | main.rs split plan | **Partially executed** (files created, mod.rs incomplete — fixed) |
| `CASCADE-ROUTER-REFACTOR.md` | cascade_router split plan | **Partially executed** (has compile errors — fixed) |
| `CONFIG-SCHEMA-DECOMPOSITION.md` | config/schema split plan | **Executed** |
| `CELL-TRAIT-AND-RENAMES.md` | Cell trait + 6 protocol renames | **NOT executed** (planned for after merges) |
| `SERVE-ROUTES-CONSOLIDATION.md` | Serve routes split plan | **NOT executed** |
| `DEMURRAGE-AND-TIERS.md` | Knowledge decay system | **NOT executed** |
| `REFACTORING-PROMPT.md` | Sequential refactoring prompt | Superseded by parallel version |
| `REFACTORING-PROMPT-PARALLEL.md` | Parallel refactoring prompt (4 agents + merge) | Used for the refactoring session |
| `POST-REFACTOR-ROADMAP.md` | What to do after runner v2 + refactoring | Current roadmap |

## Open Threads to Track

### Thread 1: Does Runner v2 Actually Work?
The runner v2 was wired as the default for `plan run --approval` in the last change.
It needs testing:
- Does it compile and run? (workspace compiles as of last check)
- Does streaming output show in the TUI?
- Do task titles display correctly?
- Does the model name appear?
- Does persistence work (executor.json, episodes.jsonl)?
- Does Ctrl+C shutdown cleanly?
- Does resume from checkpoint work?

**Test command:**
```bash
cd /Users/will/dev/nunchi/roko/tmp/roko-test
rm -rf .roko/state .roko/learn .roko/traces .roko/engrams.jsonl .roko/custody.jsonl .roko/runtime .roko/daimon .git
env -u CLAUDECODE cargo run -p roko-cli -- plan run .roko/plans/calculator --approval
```

### Thread 2: Refactoring Session Merge Conflicts
The parallel refactoring session (main.rs decomp, config split, cascade router split)
created files but left some incomplete:
- `commands/mod.rs` was missing module declarations — **fixed**
- `cascade_router.rs` had missing `knowledge_advice` fields — **fixed**
- Config module changes came from a worktree and may have stale references

These are all fixed now but test thoroughly before committing.

### Thread 3: Non-Approval Mode Still Uses Old Path
`plan run` WITHOUT `--approval` still calls `PlanRunner::from_plans_dir()` (the old
21K-line orchestrate.rs). This is intentional for now — the old path handles features
the runner v2 doesn't yet (parallel execution, multi-plan, etc.). Eventually the runner
v2 should replace both paths.

### Thread 4: TUI Data for Non-Runner-V2 Paths
The C1-C8 fixes (model on AgentSpawned, title on TaskStarted, etc.) patch the old
DashboardEvent pipeline. These help `roko serve --tui` and `roko dashboard` modes.
But they're band-aids — the systemic issue (4-layer indirection) remains for those modes.

### Thread 5: Uncommitted Changes
ALL changes from today are uncommitted on the `wp-arch2` branch. This includes:
- orchestrate.rs patches
- runner/ module (2,180 lines)
- commands/ module (main.rs decomp)
- config submodule split
- cascade_router split
- dashboard_snapshot field additions
- main.rs wiring of runner v2

These should be committed in logical groups (runner v2 as one commit, refactoring as
another, bugfixes as a third) and reviewed before merging.

### Thread 6: Post-Refactor Roadmap
See `tmp/unified-migration-runner/POST-REFACTOR-ROADMAP.md` for the full plan.
Key waves after current work stabilizes:
- **Wave 1** (1-2 days): Atomic write migration, event bus dedup, println→tracing
- **Wave 2** (5-8 days): Learning loop wiring (lifecycle Pulses, knowledge routing, reflection)
- **Wave 3** (5-6 days): Reliability hardening (health checks, MCP restart, safety)
- **Wave 4** (3-5 days): Demurrage + memory management
- **Wave 5** (8-10 days): EFE routing, context scoping, warm pool
- **Wave 6** (10-14 days): Protocol completion (Observe, Trigger, Connect)

### Thread 7: May 6 Demo Build
User added `09-MAY6-DEMO-BUILD.md` to dogfood — there's a demo deadline. This should
inform priority of what to stabilize first.

## Key Decisions Made Today

1. **Runner v2 replaces orchestrate.rs for `--approval` mode** — eliminates 4-layer
   indirection, uses direct RunState like mori
2. **Enrichment is skippable** via `skip_enrichment = true` in tasks.toml
3. **Plan discovery simplified** — `tasks.toml` presence = plan, no `.md` scanning
4. **Git auto-init** — `roko plan run` auto-creates a git repo if needed
5. **Process group kill** — SIGTERM to entire group on shutdown
6. **Config decomposed** — schema.rs split into section files
7. **Cascade router decomposed** — split into submodules
8. **Cell trait + renames deferred** — touches all 18 crates, needs stable base first

## How to Continue

1. **Test runner v2** with the calculator plan in roko-test
2. **If it works**: commit everything, create PR
3. **If it doesn't**: debug the runner v2 event loop (it's 679 lines, much easier than 21K)
4. **Then**: execute Wave 1 quick wins from POST-REFACTOR-ROADMAP.md
5. **Then**: focus on May 6 demo build items
