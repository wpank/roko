# Mori Parity Audit Status

> **Historical checklist warning (2026-08-31):** DONE/PARTIAL below records the
> original static audit and is not end-to-end completion evidence. Use
> `post-merge-live-audit-2026-08-31.md` for the operational P0-P7 status and
> `visual-density-effects.md` for the Mori-baseline rendering follow-up.

**34 DONE | 17 PARTIAL | 2 NOT STARTED** (53 items, PR #73: 107 files, +10,186/-1,094)

## Full Checklist

| St | B# | Item | Status | Evidence | Gap | BL# |
|----|-----|------|--------|----------|-----|-----|
| [x] | 0.1 | CLI UX consistency | DONE | `main.rs:1194` aliases, `explain.rs:329` signal mapping, `main.rs:1513` plans/ default | | #77 |
| [~] | 0.2 | Legacy page removal | PARTIAL | `tui/app.rs:53-60` dual render paths, `tui/pages/mod.rs:13` 16 PageId variants | PageId/PageScaffold kept for text-mode compat; not dead code per spec | #122 |
| [ ] | 0.3 | Tab content badges | NOT STARTED | `tui/tabs.rs:110` returns `&'static str` only | `Tab` is Copy+const; need dynamic count API accepting TuiState | #130 |
| [x] | 0.4 | Context-sensitive keybind hints | DONE | `tui/widgets/status_bar.rs:163` switches on all 10 tabs, tests at 325-380 | | #157 |
| [x] | 0.5 | CARGO_BUILD_JOBS limit | DONE | `runner/gate_dispatch.rs:45` nproc/2, 6+ call sites | | #206 |
| [x] | 0.6 | default-run = "roko" | DONE | `crates/roko-cli/Cargo.toml:9` | | |
| [x] | 0.7 | TUI theme alignment | DONE | `tui/theme.rs:34-73` 27 named consts, zero inline Color::Rgb outside theme.rs | | #71 |
| [~] | 1.1 | TUI live feedback gaps | PARTIAL | `event_loop.rs:10991` heartbeat, `tui_bridge.rs:302` gate_rung_started | No gate-in-progress elapsed widget; GateRungStarted only in event log | #108 |
| [x] | 1.2 | Header bar mori parity | DONE | `tui/widgets/header_bar.rs` 8 sections: pulsing dot, gradient bar, ETA, cost, CPU/mem, agents, spinner, tabs | | #124 |
| [x] | 1.3 | Error digest widget | DONE | `tui/widgets/error_digest.rs:103` compact+full modes, 4 sources, dedup | | #126 |
| [x] | 1.4 | TUI notification toasts | DONE | `modals/notification.rs` bottom-right stack, TTL, level colors, 5 visible, dismiss with `n` | | #201 |
| [x] | 1.5 | Agent status panel | DONE | `tui/widgets/agent_status_grid.rs` dense grid: icon/role/model/turns/context%/effort | | #189 |
| [x] | 1.6 | Per-model cost stats | DONE | `tui/widgets/cost_by_model.rs` table with Pass%/AvgDur/Cost/$/Task, F7 sub_tab 4 | | #156 |
| [x] | 2.1 | Plan-level wave computation | DONE | `runner/plan_dag.rs:174` Kahn's topo sort, cycle detect, `plan list --waves` | | #117 |
| [x] | 2.2 | Queue manifest | DONE | `runner/queue_manifest.rs` TOML serde, validate 6 issues, `plan queue show/validate/init` | | #116 |
| [x] | 2.3 | Plan tree wave hierarchy widget | DONE | `tui/views/plans_view.rs:180` collapsible wave groups, gradient progress bars | | #125 |
| [~] | 2.4 | Critical path ETA | PARTIAL | `task_dag.rs:682` computation, `header_bar.rs:301` display, `state.rs:1230` field | Field never written; `remaining_eta_minutes()` never called from snapshot path | #196 |
| [x] | 2.5 | File/crate overlap analysis | DONE | `runner/plan_loader.rs:175` compute+warn, `plan_dag.rs` CrateOverlap in DagSummary | | #195 |
| [x] | 2.6 | Plan validate --dag | DONE | `commands/plan.rs:347` CrossPlanDag + DagSummary, text+JSON modes | | #200 |
| [x] | 3.1 | Workspace lock scope reduction | DONE | `plan.rs:1179` generate=no lock, `plan.rs:333` validate=no lock, `plan.rs:514` run=lock | | #226 |
| [x] | 3.2 | --from-backlog plan generation | DONE | `plan_generate.rs:660` BacklogSpec, parse_backlog_ids, per-ID generation+validation | | #227 |
| [~] | 3.3 | Plan run UX friction | PARTIAL | `plan.rs:569` --fresh prune, `plan.rs:187` path stripping, `plan.rs:474` wrong cwd | No cascade router warning for --force-backend (UX34) | #107 |
| [x] | 3.4 | Plan run preflight checks | DONE | `runner/preflight.rs` 7 checks (config/creds/disk/git/plans/rust/lock), --skip-preflight | | #120 |
| [x] | 3.5 | Plan generation TOML reliability | DONE | `plan_generate.rs:154` preamble+counter-example, `prd.rs:1464` fast pre-check | | #85 |
| [~] | 3.6 | Plan generation crash retry | PARTIAL | `prd.rs:1344` classify+retry+escalate on `roko prd plan` path | `roko plan generate` has no retry loop; only PRD path covered | #57 |
| [~] | 4.1 | TUI recovery keybindings | PARTIAL | `input.rs:807` 5 keys (s/z/S/R/c), `app.rs:2031` handlers, confirm modal | Runner never polls ConfirmAction signals; recovery is logged but not dispatched | #119 |
| [x] | 4.2 | Plan CLI control commands | DONE | `main.rs:1660` Pause/Resume/Cancel/Retry, `runner/types.rs:27` ControlCommand, polled every 250ms | | #146 |
| [~] | 4.3 | Batch controller | PARTIAL | `main.rs:1635` --batch-size, `types.rs:2260` RunConfig field, BatchPause/Resume events | `_completed_since_batch_pause` is stub; never incremented or compared | #179 |
| [~] | 4.4 | TUI log search/filter | PARTIAL | `input.rs:607` LogSearch mode, `state.rs:647` LogSearchState with regex | `logs_view.rs` never reads `tui_state.log_search`; no highlight or filter render | #217 |
| [~] | 4.5 | TUI plan tree filter/search | PARTIAL | `input.rs:617` PlanFilter mode, `state.rs:748` PlanTreeFilter with status: prefix | `plan_tree.rs` reads old `state.filter`/`filter_active`; new struct unused | #219 |
| [~] | 5.1 | TUI streaming RC-7 | PARTIAL | RC-1 through RC-6 DONE; `tui_bridge.rs:302` GateRungStarted exists | No "rung X running for Ns" live indicator; only completed results shown | #109 |
| [~] | 5.2 | TUI data model unification | PARTIAL | `state.rs:1826` TuiModel Phase A done, InspectData populated | Phases B+C not started: `App` still uses both TuiState and DashboardData | #121 |
| [x] | 5.3 | TUI push-mode panel data | DONE | `dashboard_view.rs:312` 4-priority source selection, tests at state.rs:5821 | | #41 |
| [x] | 5.4 | F7 inspect view parity | DONE | `context_view.rs:1071` 3-panel: MCP/learning/prompt_stats, 5s refresh | | #127 |
| [x] | 6.1 | Doctor/onboarding diagnostics | DONE | `doctor.rs:812` Claude CLI fix, `doctor.rs:879` Perplexity fix, `setup.rs:64` provider detect | | #79 |
| [~] | 6.2 | CLI error message quality | PARTIAL | `commands/util.rs:360` recovery hints on terminal paths | 285 eprintln! without hints remain across roko-cli/src/ | #100 |
| [x] | 6.3 | CLI JSON output mode | DONE | `main.rs:264` global --json, verified on status/doctor/learn/plan list/plan show/agent list | | #113 |
| [x] | 6.4 | Lightweight status file | DONE | `runner/status_file.rs` 1s debounced atomic write, `plan status` reader | | #182 |
| [x] | 6.5 | Run ID in snapshots and events | DONE | `types.rs:1107` all RunnerEvent variants, `state.rs:326` snapshot, `status_file.rs:22` | | #212 |
| [x] | 7.1 | PRD/cloud-worker runner-v2 migration | DONE | `prd.rs:1047` PRD path, `worker/cloud.rs:455` cloud path, no legacy PlanRunner | | #131 |
| [ ] | 7.2 | Per-plan agent-handle map | NOT STARTED | `agent_stream.rs:110` AgentHandle exists; only `agent_active: bool` in event loop | Need HashMap<TaskId, AgentHandle> for targeted cancel + double-dispatch | #139 |
| [~] | 7.3 | Conductor supervisor loop | PARTIAL | `event_loop.rs:5676` tick handler, `stuck_detection.rs:206` 4 thresholds | Nudge/ForceAdvance only log; no agent injection or DAG mutation; opt-in only | #178 |
| [~] | 7.4 | Replan-on-gate-failure | PARTIAL | `types.rs:731` 4 StructuralReplanStrategy variants, `event_loop.rs:16064` runtime | Only 2 coarse strategies used; no retry-count escalation ladder | #134 |
| [x] | 7.5 | Merge success/conflict proof | DONE | `tests/merge_proof.rs` 6 harnesses, `merge.rs:584` post-merge regression gate | | #140 |
| [x] | 8.1 | Daimon TUI view | DONE | `tui/views/affect_view.rs` PAD gauges, behavioral state, somatic markers, biases | | #10 |
| [x] | 8.2 | Continuous screenshots | DONE | `runner/screenshot_collector.rs` numbered files + manifest.json, --screenshots flag | | #112 |
| [x] | 8.3 | Atmospheric effects default | DONE | Minimal defaults to restrained self-glow plus sparse clearance-aware particles; Full adds a background-only state field | Reduced motion still forces effects off | |
| [x] | 8.4 | ROSEDUST palette port | DONE | `tui/theme.rs` complete v2 palette, Theme::dark(), high_contrast overrides, Gradient | | #123 |
| [x] | 9.1 | CLI verb consolidation | DONE | `main.rs:343,442,545` visible_alias d/s/p, `main.rs:540,605` hide=true on deprecated | | #65 |
| [x] | 9.2 | Provider configuration UX | DONE | `commands/config_cmd.rs:403` discover, `462` catalog, `505` add with pre-filled defaults | | #222 |
| [~] | 9.3 | Interactive setup wizard | PARTIAL | `commands/setup.rs` 5-step stdin wizard, provider detect, auto-write roko.toml | Stdin-interactive, not ratatui TUI; simplified vs spec's 5-phase flow | #223 |
| [x] | 9.4 | Backlog import command | DONE | `commands/backlog.rs` import from tmp/backlog, PRD idea creation, --draft/--plan/--execute | | #147 |
| [~] | 9.5 | Progressive formality (5 verbs) | PARTIAL | do/think/show/tune all wired with ScopeResolver | `undo` verb entirely absent: no enum variant, no handler | |

## Remaining Work (17 PARTIAL + 2 NOT STARTED)

### NOT STARTED

| BL# | Item | What to do | Effort | Detail in |
|-----|------|------------|--------|-----------|
| #130 | Tab content badges | Add `fn label_with_counts(&self, state: &TuiState) -> String` to Tab; plumb agent/error/gate counts into header tab strip render | XS-S | batch-0.md |
| #139 | Per-plan agent-handle map | Add `HashMap<TaskId, AgentHandle>` to runner state; insert on spawn, remove on completion, lookup for targeted cancel, replace `agent_active: bool` and `task_agent_calls: u32` counter | M | batch-7.md |

### PARTIAL -- Render layer missing (5 items, Pattern 1)

| BL# | Item | What to do | Effort | Detail in |
|-----|------|------------|--------|-----------|
| #217 | Log search render | Make `logs_view.rs` read `tui_state.log_search.compiled` for highlight spans and filter-mode exclusion | XS | batch-4.md |
| #219 | Plan filter render | Change `plan_tree.rs` to read `tui_state.plan_tree_filter` instead of old `state.filter`/`filter_active`; update test at line 1004 | XS | batch-4.md |
| #179 | Batch controller | Remove `_` prefix from `_completed_since_batch_pause`, increment on plan completion, compare to `config.batch_size`, emit BatchPause | XS | batch-4.md |
| #119 | Recovery keybindings | Add `ControlCommand::poll`-style pickup for SoftRetryPlan/RepairPlanPreserve/ReverifyPlan signals; runner must act on confirmed TUI actions | S | batch-4.md |
| #196 | Critical path ETA | Call `remaining_eta_minutes()` from snapshot update path, write result to `tui_state.critical_path_eta_minutes` | XS | batch-2.md |

### PARTIAL -- Types defined, runtime not wired (2 items, Pattern 2)

| BL# | Item | What to do | Effort | Detail in |
|-----|------|------------|--------|-----------|
| #134 | Replan escalation ladder | Wire retry-count-based selection of the 4 StructuralReplanStrategy variants (MergeWithNext at attempt 1, SplitTask at 2, etc.) | S | batch-7.md |
| #178 | Conductor supervisor | Make Nudge inject context into agent message channel; make ForceAdvance mutate task DAG state; add phase stall threshold (1800s); change tick to 2s; make always-on | S-M | batch-7.md |

### PARTIAL -- Path-specific coverage (2 items, Pattern 3)

| BL# | Item | What to do | Effort | Detail in |
|-----|------|------------|--------|-----------|
| #57 | Crash retry/escalation | Port retry loop + `classify_agent_crash` + `next_tier_model` escalation from `prd.rs` to `commands/plan.rs` PlanCmd::Generate handler | S | batch-3.md |
| #109 | TUI streaming RC-7 | Add "rung X running for Ns" live indicator widget; GateRungStarted event exists but only lands in event log, not a dedicated current_rung field | S | batch-5.md |

### PARTIAL -- Spec-vs-implementation scope (3 items, Pattern 4)

| BL# | Item | What to do | Effort | Detail in |
|-----|------|------------|--------|-----------|
| #223 | Setup wizard | Rewrite stdin wizard as ratatui TUI with 5-phase flow (Providers/Models/Tools/Backend/Review) | M | batch-9.md |
| #122 | Legacy page removal | Decide: remove PageId/PageScaffold or document as intentional text-mode compat. Currently kept deliberately | XS (decision) | batch-0.md |
| -- | Progressive formality: undo | Implement `roko undo` verb: enum variant, handler, semantics (revert last plan/task?) | S | batch-9.md |

### PARTIAL -- Other (5 items)

| BL# | Item | What to do | Effort | Detail in |
|-----|------|------------|--------|-----------|
| #108 | Gate progress elapsed | Add widget showing "rung X running for Ns" during gate execution; `GateRungStarted` exists but no in-progress display | S | batch-1.md |
| #107 | --force-backend warning | Add cascade router warning when `--force-backend` bypasses learned routing (UX34) | XS | batch-3.md |
| #121 | TUI data model unification B+C | Migrate tabs from TuiState/DashboardData dual reads to single TuiModel; remove DashboardData from App | M-L | batch-5.md |
| #100 | CLI error message quality | Add recovery hints to remaining 285 eprintln! calls across roko-cli/src/ | M | batch-6.md |
| -- | Atmospheric effects default | Change `EffectsPreset::default()` from Off to Minimal | XS | batch-8.md |

## Gap Patterns

**Pattern 1 -- Input/state wired, render missing (5 items):** TUI input handlers, action dispatch, and state models are complete, but view render functions still read old fields or ignore the new state. These are the fastest to close.

**Pattern 2 -- Types defined, runtime not wired (3 items):** Enums, strategies, and struct variants are defined but the runtime dispatch never selects or acts on them. Requires actual behavioral changes.

**Pattern 3 -- Path-specific coverage (2 items):** Feature works on one code path (e.g., `roko prd plan`) but not another (`roko plan generate`). Requires porting logic between handlers.

**Pattern 4 -- Spec-vs-implementation scope (3 items):** Implementation works but doesn't match the spec's ambition level. Some are intentional simplifications (setup wizard), some are deferred scope (undo verb).
