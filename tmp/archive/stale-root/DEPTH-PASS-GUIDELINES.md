# Depth Pass Guidelines

> Complete reference for upgrading roko's shallow codex-generated implementations
> to mori-faithful, PRD-grounded, academically-informed depth.
>
> **Rule**: Every depth pass MUST read the listed reference files before writing code.
> The codex script failed because it never read these. Don't repeat that mistake.

---

## How to use this document

Each pass has:
1. **Scope** — what you're upgrading
2. **Reference files** — MUST-READ before writing any code (mori source, PRDs, component specs)
3. **Roko files to modify** — where the shallow implementations live
4. **Gap list** — exactly what's missing, with mori file:line references
5. **Acceptance criteria** — how to know you're done

For each pass, feed the reference files via `--read` flags:
```bash
claude --print --read <ref1> --read <ref2> ... -p "<task prompt>"
```

---

## Pass 1: TUI — Port Mori's Terminal UI

### Scope
Replace roko's text-scaffold TUI (8,858 LOC, 8 files) with a faithful port of mori's interactive TUI (20,678 LOC, 55 files). The goal is visual and behavioral parity: same layout, same theme, same views, same widgets, same modals, same keybindings. Roko-specific pages (efficiency, learning, experiments) are additions on top.

### Reference files (MUST-READ)

**Mori TUI source** (port from these):
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/mod.rs` (55 LOC) — init/restore
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/layout.rs` (362 LOC) — main render dispatcher, per-tab routing, post-processing layers, alert bar, modal overlay stack
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/theme.rs` (266 LOC) — ROSEDUST palette (exact RGB values), 19 style functions, per-role accents (28 roles), per-phase accents, gradients (fire/context/ocean), `brighten()`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/input.rs` (600 LOC) — 136 keybindings, 4 input modes (Normal/Inject/Filter/Confirm), mouse support, focus zone dispatch
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/atmosphere.rs` (284 LOC) — particle system (500 cap), heartbeat, breathing, shimmer, flash
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/postfx.rs` (444 LOC) — bloom, vignette, dim overlay, modal glow, ambient orbs, drop shadow, amber color grade
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/hit_test.rs` (349 LOC) — mouse zone computation per-tab
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/color.rs` (179 LOC) — HSV conversion, screen/additive blend, gradient LUTs
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/bars.rs` (190 LOC) — gradient bar, segmented bar, NERV gauge, semantic bar
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/math.rs` (140 LOC) — Vec2, easing functions, wave combinators
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/vfx.rs` (104 LOC) — field generators (plasma, noise, FBM, voronoi, ripple)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/nerv_viz.rs` (355 LOC) — progress percolation, activity ripples, data rain
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/effects_config.rs` (91 LOC) — toggleable effects, degraded mode
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/tabs.rs` (62 LOC) — Tab enum, F-key labels
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/` (ALL files) — 7 tab views:
  - `dashboard.rs` (434) — master-detail with 7 sub-tabs
  - `plans.rs` (1,516) — hierarchical plan tree with waves
  - `agents.rs` (795) — agent roster + transcript
  - `git_view.rs` (691) — branch tree + commit log + diff
  - `logs.rs` (126) — scrollable log viewer
  - `config.rs` (762) — interactive config editor
  - `context.rs` (1,001) — context/inspect view
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/widgets/` (ALL files) — 26 widgets:
  - `agent_output.rs` (1,679), `plan_tree.rs` (1,077), `task_progress.rs` (601), `header_bar.rs` (432), `phase_compact.rs` (408), `error_digest.rs` (407), `plan_list.rs` (350), `phase_bar.rs` (316), `parallel_pool.rs` (307), `sys_metrics.rs` (295), `agent_pool.rs` (226), `status_bar.rs` (220), `branch_tree.rs` (193), `token_sparkline.rs` (194), `diff_panel.rs` (157), `phase_timeline.rs` (164), `wave_bar.rs` (132), `wave_progress.rs` (124), `agent_grid.rs` (121), `token_bar.rs` (116), `context_gauge.rs` (117), `braille.rs` (77), `status_badge.rs` (69), `scrollbar.rs` (45), `tab_bar.rs` (33)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/modals/` (ALL files) — 13 modals:
  - `task_detail.rs` (628), `confirm.rs` (460), `plan_detail.rs` (458), `queue_overview.rs` (331), `agent_pool_modal.rs` (176), `help.rs` (155), `wave_overview.rs` (148), `task_picker.rs` (140), `approval.rs` (64), `notification.rs` (59), `inject.rs` (54), `quit.rs` (54), `batch_review.rs` (45)

**Component specs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/tui-views.md` (500 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/tui-widgets.md` (688 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/tui-modals.md` (514 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/tui-state.md` (304 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/roko-tui.md` (272 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/efficiency-dashboard.md` (586 LOC)

**Dashboard spec from mori-agents**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/mori-efficiency-dashboard-spec.md` (1,011 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/mori-efficiency-dashboard-prototype.jsx` (740 LOC)

### Roko files to modify
- `crates/roko-cli/src/tui/` — ALL files (rewrite or heavily extend)
- `crates/roko-cli/src/main.rs` — dashboard command handler

### Gap list (68 items)
See `tmp/MORI-PARITY-GAP-ANALYSIS.md` § TUI Dashboard for the complete list. Key items:
- 7 missing views (Git, Processes, Context, interactive Config, Plans tree, Dashboard master-detail, Agents roster)
- 18 missing widgets (agent_output, plan_tree, phase_bar/compact/timeline, wave_bar/progress, diff_panel, error_digest, etc.)
- 11 missing modals (plan_detail, task_detail, confirm, queue_overview, etc.)
- ROSEDUST theme with exact RGB values
- Atmosphere + post-processing effects
- Mouse support + hit testing
- 4 input modes + 136 keybindings

### Acceptance criteria
- [ ] `roko dashboard` launches interactive ratatui TUI with ROSEDUST theme
- [ ] 7 tabs navigable via F1-F7 and number keys
- [ ] Dashboard tab shows master-detail split with plan tree + sub-tabs
- [ ] Plans tab shows hierarchical collapsible plan tree with wave grouping
- [ ] Agents tab shows agent roster with live output scrolling
- [ ] All 13 modals work (help, plan detail, task detail, confirm, etc.)
- [ ] Mouse clicking navigates focus zones
- [ ] Particles + bloom + vignette render on capable terminals
- [ ] `NO_COLOR` disables effects gracefully
- [ ] Roko-specific pages (Health, Learning, Experiments, Signals) preserved as additional tabs

---

## Pass 2: Gate Infrastructure

### Scope
Add the build infrastructure that makes gates performant at scale: affected-crate scoping, cargo semaphore, sccache integration, error digest extraction, pattern sharing, nextest detection.

### Reference files (MUST-READ)

**Mori gates**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/gates.rs` (2,944 LOC) — `affected_crate_args()` (lines 154-213), `cargo_gate_semaphore()`, `apply_gate_env()` (lines 612-630), `extract_error_digest()` (lines 90-136), `append_discovered_pattern()` / `read_discovered_patterns()`, `extract_failing_test_names()` (lines 969-991), `extract_test_failure_snippet()`, `looks_like_shared_target_cache_corruption()`, `clean_retry_target_dir()`, `is_mostly_passing()`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/gates.rs` (1,758 LOC) — `format_gate()`, `cargo_toml_validation_gate()`, `full_loop_gate()`, `golem_lifecycle_gate()`

**Component specs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/gates/gate-pipeline.md` (257 LOC) — `PipelineVerdict` with rung-tagged steps, `StepHook` trait, `GateRegistry` trait, `feedback_for_agent()`, panic isolation
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/gates/gate-compile-refactor.md` (183 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/gates/gate-test.md` (213 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/gates/gate-diff.md` (224 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/gates/6-rung-selector.md` (206 LOC)

### Roko files to modify
- `crates/roko-gate/src/compile.rs` — add affected-crate scoping
- `crates/roko-gate/src/test_gate.rs` — add nextest detection, failing test extraction
- `crates/roko-gate/src/pipeline.rs` — upgrade to `PipelineVerdict` with rung tags, `StepHook`, panic isolation
- `crates/roko-gate/src/lib.rs` — add `GateRegistry` trait, `FormatGate`, `CargoTomlValidationGate`
- `crates/roko-gate/src/error_digest.rs` (NEW) — `extract_error_digest()`, `extract_failing_test_names()`, `extract_test_failure_snippet()`
- `crates/roko-gate/src/pattern_sharing.rs` (NEW) — `append_discovered_pattern()`, `read_discovered_patterns()`
- `crates/roko-gate/src/env.rs` (NEW) — `apply_gate_env()`, `cargo_gate_semaphore()`, shared target dir, sccache, corruption detection

### Gap list (17 items)
See `tmp/MORI-PARITY-GAP-ANALYSIS.md` § Gates for the complete list.

### Acceptance criteria
- [ ] `cargo check -p crate1 -p crate2` (scoped) instead of `--workspace` when possible
- [ ] Global 2-permit semaphore throttles concurrent cargo processes
- [ ] `CARGO_TARGET_DIR`, `RUSTC_WRAPPER=sccache`, `CARGO_BUILD_JOBS` set in gate env
- [ ] Error digests extracted and attached to `Verdict.error_digest`
- [ ] Discovered patterns persisted to `.roko/runs/discovered-patterns.json`
- [ ] `cargo nextest` preferred when available
- [ ] `PipelineVerdict` carries per-rung `(Rung, Verdict)` breakdown
- [ ] `feedback_for_agent()` returns focused feedback string
- [ ] Gate panics caught and surfaced as `Verdict::fail`

---

## Pass 3: Orchestrator — Task-Level Scheduling + Merge Queue

### Scope
Upgrade `ParallelExecutor` from plan-level to task-level awareness. Add merge queue with conflict detection. Enrich crash recovery snapshots.

### Reference files (MUST-READ)

**Mori orchestrator**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/executor.rs` (4,629 LOC) — `ParallelExecutor`, `GlobalTaskId`, `in_flight_tasks`, `completed_tasks`, `skipped_tasks`, `task_failure_counts`, batch spawning, wave execution
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/unified_dag.rs` (1,216 LOC) — `UnifiedTaskDag`, `GlobalTaskId`, cross-plan dependencies
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/queue.rs` (1,005 LOC) — merge queue, conflict detection, FIFO-with-skip
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/tasks.rs` (2,086 LOC) — task schema, lifecycle, execution state
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pipeline.rs` (793 LOC) — `PipelinePhase`, state transitions
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/memory.rs` (3,107 LOC) — iteration memory, failure context
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/autofix.rs` (915 LOC) — automatic remediation
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/context.rs` (813 LOC) — context pressure
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pattern_learning.rs` (487 LOC) — failure pattern learning
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs` (17,999 LOC) — the main runtime loop (read selectively: agent dispatch, event handling, state updates)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` (3,358 LOC) — agent spawn, instance IDs, warm pool, DOA detection, zombie reaping, kill escalation
- `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` (3,225 LOC) — `diagnose_plan_worktree()`, `RecoveryDecision`, disk-space reclamation, ENOSPC handling

**Component specs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/orchestrator/parallel-executor.md` (495 LOC) — 22 action variants, 5 sub-modules, acceptance criteria
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/orchestrator/crash-recovery.md` (264 LOC) — 15-field snapshot, atomic write, .bak rotation
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/orchestrator/merge-queue.md` (204 LOC) — MergeQueue struct, file-overlap detection, FIFO-with-skip
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/orchestrator/worktree-manager.md` (318 LOC) — WorktreeManager, RecoveryDecision, disk budget, index persistence

**Mori-agents docs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/09-multi-agent-orchestration.md` (615 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/16-prd-to-execution-pipeline.md` (920 LOC)

### Roko files to modify
- `crates/roko-orchestrator/src/executor/mod.rs` — add task-level tracking, `GlobalTaskId`, `in_flight_tasks`
- `crates/roko-orchestrator/src/executor/action.rs` — add 17 missing action variants
- `crates/roko-orchestrator/src/executor/snapshot.rs` — upgrade to 15-field snapshots
- `crates/roko-orchestrator/src/merge_queue.rs` (NEW) — `MergeQueue` with conflict detection
- `crates/roko-orchestrator/src/unified_dag.rs` (NEW) — `UnifiedTaskDag`, cross-plan deps
- `crates/roko-orchestrator/src/worktree.rs` — add `RecoveryDecision`, disk reclamation, index persistence
- `crates/roko-cli/src/orchestrate.rs` — wire new actions, agent lifecycle (instance IDs, warm pool, DOA detection)

### Gap list
See `tmp/MORI-PARITY-GAP-ANALYSIS.md` § Orchestrator for complete list (17 missing actions, task scheduling, merge queue, crash recovery, agent lifecycle, worktree management).

### Acceptance criteria
- [ ] `ExecutorAction` has all 22+ variants from component spec
- [ ] `GlobalTaskId` tracks individual tasks across plans
- [ ] `MergeQueue` with file-overlap conflict detection and FIFO-with-skip
- [ ] `ExecutorSnapshot` has 15 fields with atomic write and .bak rotation
- [ ] Agent instance IDs (role-plan-task-iteration format)
- [ ] Warm reviewer pre-spawn during gating
- [ ] Spawn failure tracking with exponential backoff
- [ ] Dead-on-arrival detection (<15s exit with no output)
- [ ] Worktree `RecoveryDecision` enum with 5 variants
- [ ] Disk-space-aware worktree reclamation

---

## Pass 4: Cognitive Layer — Grimoire, Dreams, Daimon

### Scope
Deepen the scaffolds that codex created in `roko-neuro`, `roko-daimon`, `roko-dreams` with the full algorithms and data structures from the PRDs and mori's golem crates.

### Reference files (MUST-READ)

**PRD documents** (academic/research foundation):
- `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/` (12 files) — CLS architecture (McClelland 1995), episodic/semantic stores, knowledge hierarchy
  - `00-overview.md` — CLS dual-system model, memory lifecycle
  - `01-grimoire.md` — 5 knowledge types, decay classes, A-MAC admission, retrieval scoring
  - `01b-grimoire-memetic.md` — replicator dynamics (Dawkins/Taylor-Jonker), Price equation, epistemic parasites, AntiKnowledge
  - `01c-grimoire-hdc.md` — Binary Spatter Codes (D=10,240), episode compression, holographic legacy bundles, controlled forgetting
- `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/` (10 files) — affect/emotion system
  - PAD model (Pleasure-Arousal-Dominance)
  - OCC/Scherer/Chain-of-Emotion appraisal pipeline
  - Somatic markers (Damasio 1994, Bechara 2000)
  - Somatic landscape (k-d tree over strategy space)
  - ALMA three-layer EMA (Gebhard 2005)
  - Learned helplessness (Seligman 1972)
  - Mood-congruent memory (Bower 1981)
  - Negativity bias (Kahneman-Tversky 1.6x)
- `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/` (9 files) — offline learning
  - Three-phase cycle (NREM replay, REM imagination, integration)
  - Mattar-Daw utility-weighted replay (gain × need)
  - Counterfactual reasoning (Pearl's SCM)
  - Creative recombination (Boden 2004: combinational/exploratory/transformational)
  - Threat simulation (Revonsuo 2000)
  - Emotional depotentiation (Walker & van der Helm 2009)
  - Memory triage (Stickgold & Walker 2013: preserve/abstract/forget)
  - Budget allocation by behavioral phase (thriving→terminal)

**Mori golem crates** (reference implementations):
- `/Users/will/dev/uniswap/bardo/crates/golem-grimoire/src/` (13,666 LOC) — full knowledge system:
  - `entry.rs` (848) — `GrimoireEntry`, 5 types, `DecayClass`, `KnowledgePolarity`
  - `admission.rs` (540) — A-MAC five-factor scoring
  - `decay.rs` (931) — Ebbinghaus forgetting curves
  - `retrieval.rs` (1,137) — four-factor retrieval scoring
  - `causal.rs` (1,229) — causal graph with typed edges
  - `hierarchical.rs` (1,702) — two-level RAPTOR retrieval
  - `curator.rs` (907) — 50-tick maintenance cycle
  - `clade.rs` (1,917) — cross-agent knowledge sharing
  - `memetic.rs` (550) — fitness computation, replicator dynamics
  - `store.rs` (421) — `GrimoireStore` facade
  - `reader.rs` (318) — `GrimoireReader` trait
  - `writer.rs` (344) — `GrimoireWriter` trait
  - `substrate/episodic.rs` (714) — episodic store
  - `substrate/semantic.rs` (922) — semantic store
  - `snapshot.rs` (207) — checkpoint/restore
- `/Users/will/dev/uniswap/bardo/crates/golem-daimon/src/` (8,907 LOC) — affect system:
  - `appraisal.rs` (1,691) — 8-step OCC/Scherer pipeline
  - `behavior.rs` (1,318) — emotional risk/probe/escalation/sharing
  - `dream_daimon.rs` (1,111) — REM depotentiation, emotional load
  - `memory.rs` (769) — emotional memory, mood-congruent retrieval
  - `somatic.rs` (612) — k-d tree somatic landscape
  - `emotion.rs` (317) — PAD vectors, Plutchik mapping
  - `alma.rs` (377) — three-layer EMA (emotion→mood→personality)
  - `contagion.rs` (302) — cross-agent affect transfer
  - `runtime_daimon.rs` (773) — `DaimonExtension` runtime hook
  - `mortality_daimon.rs` (533) — mortality-linked affect
  - `death_daimon.rs` (698) — end-of-life narrative + testament
- `/Users/will/dev/uniswap/bardo/crates/golem-dreams/src/` (2,844 LOC) — dream consolidation:
  - `evolution/mod.rs` (607) — `EvolutionEngine`
  - `evolution/correlation.rs` (548) — meme correlation matrices
  - `evolution/strategy.rs` (412) — strategy evolution, fitness
  - `evolution/recombination.rs` (372) — genetic recombination
  - `sleepwalker.rs` (263) — dream trigger config
  - `scheduler.rs` (133) — NREM/REM timing
  - `consolidation/styx.rs` (224) — vault submission

**Component specs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/dream-consolidation.md` (243 LOC) — `DreamRunner`, `DreamConfig`, `DreamReport`, 9-step consolidation cycle
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/episode-logger.md` (314 LOC) — 33-field Episode, `EpisodeLog`, retention, compaction
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/playbook.md` (345 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/skill-library.md` (298 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/pattern-discovery.md` (248 LOC)

**Mori-agents docs (practical learning)**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/07-self-improvement.md` (570 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/19-practical-self-learning.md` (1,155 LOC)

**Mori-refactor docs (theoretical foundation)**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/09-memory-and-knowledge.md` (1,348 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md` (944 LOC)

### Roko files to modify

**Grimoire (roko-neuro)**:
- `crates/roko-neuro/src/lib.rs` — upgrade to full module structure
- `crates/roko-neuro/src/entry.rs` (NEW or upgrade) — `GrimoireEntry` with 5 types, `DecayClass`, confidence, quality scores, bi-temporal metadata
- `crates/roko-neuro/src/admission.rs` (NEW or upgrade) — A-MAC five-factor scoring with hallucination firewall
- `crates/roko-neuro/src/decay.rs` (NEW or upgrade) — Ebbinghaus forgetting curves (`R = e^(-t/S)`) with 4 decay classes
- `crates/roko-neuro/src/retrieval.rs` (NEW or upgrade) — four-factor retrieval (recency × importance × relevance × congruence)
- `crates/roko-neuro/src/causal.rs` (NEW) — causal graph with typed directed edges
- `crates/roko-neuro/src/curator.rs` (NEW) — 50-tick maintenance cycle (validate/prune/compress/cross-reference)
- `crates/roko-neuro/src/memetic.rs` (NEW) — fitness computation, replicator dynamics, Price equation, parasite detection
- `crates/roko-neuro/src/store.rs` (NEW or upgrade) — `GrimoireStore` facade with `Reader`/`Writer` traits

**Daimon (roko-daimon)**:
- `crates/roko-daimon/src/lib.rs` — upgrade to full module structure
- `crates/roko-daimon/src/emotion.rs` (NEW or upgrade) — PAD vectors, Plutchik mapping, `EmotionLabel`
- `crates/roko-daimon/src/appraisal.rs` (NEW or upgrade) — 8-step OCC/Scherer pipeline, `AppraisalEngine`
- `crates/roko-daimon/src/somatic.rs` (NEW) — k-d tree somatic landscape, behavioral bias lookup
- `crates/roko-daimon/src/memory.rs` (NEW) — emotional memory, mood-congruent retrieval (Bower 1981)
- `crates/roko-daimon/src/alma.rs` (NEW) — three-layer EMA (emotion→mood→personality, Gebhard 2005)
- `crates/roko-daimon/src/behavior.rs` (NEW) — behavioral modulation from affect state
- `crates/roko-daimon/src/contagion.rs` (NEW) — cross-agent affect transfer

**Dreams (roko-dreams)**:
- `crates/roko-dreams/src/lib.rs` — upgrade to full module structure
- `crates/roko-dreams/src/runner.rs` (NEW or upgrade) — `DreamRunner` with 9-step consolidation cycle
- `crates/roko-dreams/src/replay.rs` (NEW) — Mattar-Daw utility-weighted episode selection, bidirectional replay
- `crates/roko-dreams/src/imagination.rs` (NEW) — counterfactual generation, creative recombination, threat simulation
- `crates/roko-dreams/src/consolidation.rs` (NEW) — memory triage (preserve/abstract/forget), staging buffer
- `crates/roko-dreams/src/scheduler.rs` (NEW or upgrade) — dream scheduling, budget allocation by behavioral phase

### Key algorithms to implement

**Grimoire**:
- Ebbinghaus decay: `effective_confidence = base × 0.5^(t / half_life)`, floor 0.05
- A-MAC admission: `score = 0.30×accuracy + 0.20×memorability + 0.25×actionability + 0.15×consistency + 0.10×non_redundancy`, threshold 0.45
- Replicator dynamics: `dx_i/dt = x_i × (W_i - W_bar)` where `W = fidelity × fecundity × longevity`
- Price equation: `ΔW_bar = Cov(W_i, x_i) + E[Δw_i]`
- Retrieval scoring: `score = 0.20×recency + 0.25×importance + 0.35×relevance + 0.20×emotional_congruence`

**Daimon**:
- PAD mapping: P = 0.6×desirability + 0.4×PnL_direction (negativity bias 1.6x); A = 0.15×anomaly + 0.50×prediction_error + 0.35×baseline; D = 0.50×coping + 0.30×vitality + 0.20×baseline
- EMA rates: P=0.15, A=0.20, D=0.08
- Mood EMA: `mood = 0.9 × old + 0.1 × current`
- Learned helplessness: D < -0.3 for 200+ ticks
- Mood-congruent retrieval: 5-30% boost (Bower 1981)

**Dreams**:
- Mattar-Daw: `utility = gain × need × (1 - 0.5×spacing_penalty)`
- Gain: `0.4×surprise + 0.3×significance + 0.3×suboptimality`
- Need: `0.4×cosine_sim + 0.3×regime_match + 0.3×recency`
- Replay decay: `gain_after = gain × 0.85^replay_count`
- Budget allocation: thriving (34% NREM, 30% REM, 21% consolidation) → terminal (16% NREM, 12% REM, 57% consolidation)

### Acceptance criteria
- [ ] `GrimoireEntry` with 5 knowledge types, decay classes, confidence dynamics
- [ ] A-MAC admission gate with 5-factor scoring (threshold 0.45)
- [ ] Ebbinghaus decay with 4 decay classes and floor 0.05
- [ ] Curator cycle runs every 50 ticks (validate/prune/compress)
- [ ] Causal graph with typed directed edges
- [ ] PAD vectors with OCC/Scherer appraisal pipeline
- [ ] Somatic landscape (k-d tree over parameter space)
- [ ] Three-layer EMA (ALMA: emotion→mood→personality)
- [ ] Learned helplessness detection (D < -0.3 for 200+ ticks)
- [ ] DreamRunner with Mattar-Daw replay selection
- [ ] Counterfactual generation via causal model
- [ ] Memory triage (preserve/abstract/forget)
- [ ] Dream journal entries persisted to `.roko/dream/`
- [ ] All algorithms have unit tests matching PRD formulas

---

## Pass 5: Conductor Depth

### Scope
Add LLM conductor agent, graduated action vocabulary, cooldown dedup, rate limiter, missing watchers.

### Reference files (MUST-READ)

**Mori conductor**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/mod.rs` (~10,786 LOC)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/watchers.rs` (967 LOC) — AgentSilence, TaskStall, TaskContinuation
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/llm.rs` (~27,430 LOC) — LLM conductor agent, `conductor_system_prompt()`, `state_snapshot()`, `parse_directive()`, 17+ directive types

**Component spec**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/roko-conductor.md` (311 LOC) — 7 intervention levels, circuit breaker, 10 watchers

### Roko files to modify
- `crates/roko-conductor/src/` — add LLM conductor, graduated actions, missing watchers, cooldown, rate limiter

### Acceptance criteria
- [ ] LLM conductor agent with `state_snapshot()` and `parse_directive()` (17+ directives)
- [ ] 7 intervention levels (Nudge/Suggest/Enforce/Pause/Rollback/Abort/Escalate)
- [ ] `AgentSilence`, `TaskStall`, `TaskContinuation` watchers
- [ ] Cooldown deduplication (120s per watcher:plan:role)
- [ ] Agent spawn rate limiter with priority queue
- [ ] Graduated escalation on compile failures (3→nudge, 5→restart, 7→force-advance)

---

## Pass 6: Agent Infrastructure

### Scope
Add missing agent backends, warm pool, per-turn budget, skill injection, streaming events.

### Reference files (MUST-READ)

**Mori agent**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` (3,358 LOC) — `ClaudeConnection`, `CursorAcpConnection`, `AppServerConnection`, warm pool, kill escalation, DOA detection
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/roles.rs` (297 LOC) — 28 roles, `ModelSpec`, `AgentBackend`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/events.rs` (170 LOC) — `AgentEvent` enum

**Mori prompts**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompts.rs` (5,914 LOC) — 40+ prompt functions, skill injection, iteration feedback compression, context caching, verification isolation

**Component specs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/agents/agent-pool.md` (246 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/agents/multi-agent-pool.md` (278 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/agents/process-mgmt.md` (301 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/agents/tool-permissions-matrix.md` (222 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/system-prompt-builder.md` (258 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/enrichment-pipeline.md` (282 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/context-pack-cache.md` (265 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/prompt-budget-per-role.md` (237 LOC)

**Mori-agents docs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/02-connection-backends.md` (697 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/04-context-engineering.md` (548 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/05-prompt-engineering.md` (540 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/24-prompt-budget-engineering.md` (352 LOC)

### Roko files to modify
- `crates/roko-agent/src/cursor_agent.rs` — implement Cursor ACP protocol
- `crates/roko-agent/src/codex_agent.rs` — implement Codex app-server JSON-RPC
- `crates/roko-agent/src/pool.rs` — add warm pool (pre-spawn/promote/evict)
- `crates/roko-agent/src/multi_pool.rs` — add fallback model retry, per-plan killing
- `crates/roko-agent/src/claude_cli_agent.rs` — add `--max-budget` per-turn dollar cap
- `crates/roko-compose/src/system_prompt_builder.rs` — add skill injection, iteration feedback compression
- `crates/roko-compose/src/context_provider.rs` — add disk-based context pack caching
- `crates/roko-cli/src/orchestrate.rs` — wire per-role tool permissions, streaming events, sccache env

### Acceptance criteria
- [ ] Cursor ACP backend works (persistent JSON-RPC over stdio)
- [ ] Codex app-server backend works
- [ ] `from_model(slug)` routes to correct backend automatically
- [ ] Warm pool pre-spawns and promotes agents (saves 5-15s)
- [ ] `--max-budget` caps per-turn spend ($0.40-$3.00 per role)
- [ ] Per-role tool permission matrix enforced
- [ ] Skills loaded from `.claude/skills/` and injected into prompts
- [ ] Iteration feedback compressed to 3-bullet directives on retry
- [ ] Context pack caching (SHA256-keyed, disk-persisted)
- [ ] Real-time `AgentEvent` streaming during execution

---

## Cross-cutting references

These documents provide broader context that applies to ALL passes:

**Architecture docs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/00-overview.md` (567 LOC) — refactor goals
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/04-framework.md` (1,269 LOC) — framework design
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/07-orchestration.md` (957 LOC) — orchestration design
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md` (944 LOC) — cognitive science foundations
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/13-unified-theory.md` (744 LOC) — unifying theory

**Mistakes to avoid**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MISTAKES-LEARNED.md` — 30+ catalogued mistakes from roko development

**Master checklist**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` (1,253 items)

**Component spec index**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/INDEX.md` (646 LOC) — maps every component to its spec

---

## Pass 7: Code Intelligence MCP Server + Live Index

### Scope
This is the **single most impactful infrastructure gap**. Mori agents can search code by symbol, find references, trace call graphs, and get workspace maps. Roko agents are blind — they can only grep. Build `roko-mcp` as a standalone MCP server and wire `roko-index` into an operational index.

### Reference files (MUST-READ)

**mori-mcp** (the reference MCP server):
- `/Users/will/dev/uniswap/bardo/crates/mori-mcp/src/` (3,331 LOC, 7 files) — complete MCP server with 12 tools:
  - `search_code` — symbol index search by name/kind/visibility
  - `get_symbol_context` — full symbol details (signature, docs, source, related)
  - `get_file_ast` — indexed symbols from a file
  - `find_similar_patterns` — HDC cosine similarity search
  - `find_references` — callers, importers, type users, implementors
  - `find_implementations` — impl blocks for a trait
  - `get_callers` — transitive call/dependency graph (BFS, N hops)
  - `workspace_map` — crate graph, inter-crate deps, symbol counts
  - `remember_context` / `recall_context` — namespace-scoped memory
  - `get_mcp_savings` — tool call count and token savings estimate
  - Remote tools: `queue_state`, `steering_command`

**mori-index** (the reference code index):
- `/Users/will/dev/uniswap/bardo/crates/mori-index/src/` (5,332 LOC, 15 files):
  - `db.rs` (1,112) — SQLite-backed schema: files, symbols, refs tables, WAL mode, HDC blob storage
  - `parser.rs` (590) — tree-sitter Rust parser: functions, structs, enums, traits, impls, type aliases, consts, modules, macros, use statements, refs
  - `search.rs` (330) — keyword + HDC + embedding hybrid search fused via Reciprocal Rank Fusion (k=60)
  - `graph.rs` (309) — directed dependency graph with PageRank, BFS with depth control
  - `update.rs` — content-hash based incremental re-index
  - `snapshot.rs` — rkyv zero-copy serialization for fast cold starts
  - `context_overlay.rs` (335) — namespace-scoped transient mutations (worktree/agent/shared)
  - `privacy.rs` (283) — `RedactionPolicy`, `PrivacyRequest`, HDC locality keys
  - `embedding.rs` — ONNX semantic embedding (feature-gated)
  - `merkle.rs` (311) — file hash tree for change detection
  - `fingerprint.rs` — 10,240-bit HDC vectors

**mori-context** (context assembly from index results):
- `/Users/will/dev/uniswap/bardo/crates/mori-context/src/` (702 LOC, 6 files):
  - `assemble.rs` — takes search results → extracts source snippets → finds related symbols → builds `ContextBlock`
  - `compress.rs` — token-budget-aware compression: score-based truncation, dedup, budget enforcement
  - `snippet.rs` — source window extraction around symbol locations
  - `query.rs` — `ContextQuery` with SearchStrategy (Keyword/Similar/Hybrid)

**Component spec**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/orchestrator/roko-index.md` (381 LOC)

**Mori-agents doc**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/18-code-intelligence-and-gateway.md` (1,066 LOC)

### Roko files to modify
- `crates/roko-index/src/` — upgrade from skeleton to full operational index:
  - Add SQLite database layer (`db.rs`)
  - Add hybrid search module (`search.rs`) with RRF fusion
  - Add incremental update module (`update.rs`)
  - Add context overlay module (`context_overlay.rs`)
  - Wire parser/graph/HDC/search into top-level `Index` struct
- `crates/roko-mcp-github/` or new `crates/roko-mcp/` — create standalone MCP server binary with 10+ tools
- `crates/roko-cli/src/orchestrate.rs` — auto-start roko-mcp as MCP server for agents

### Gap list
1. **No SQLite database** — roko-index has no persistence. Everything is in-memory only.
2. **No search module** — no keyword, structural, or hybrid search
3. **No incremental updates** — no content-hash change detection, full re-index every time
4. **No context overlays** — no per-worktree/per-agent scoped mutations
5. **No context compression** — no token-budget-aware truncation with score-based prioritization
6. **No MCP server binary** — agents cannot access code intelligence via MCP tools
7. **No `Index` orchestrator struct** — building blocks exist but nothing ties them together
8. **No privacy/redaction** — no policies for masking sensitive code in tool results

### Acceptance criteria
- [ ] `roko-index` has SQLite-backed persistence with files/symbols/refs tables
- [ ] Hybrid search (keyword + HDC) with Reciprocal Rank Fusion
- [ ] Incremental re-index via content-hash change detection
- [ ] `roko-mcp` binary provides `search_code`, `find_references`, `get_callers`, `workspace_map` tools
- [ ] MCP server auto-started by orchestrator with `--mcp-config`
- [ ] Context assembly from index results with token-budget compression
- [ ] End-to-end test: index a crate → search for a function → get context → verify result

---

## Pass 8: Prompt Composition Depth

### Scope
Roko has 10 of mori's 24 role-specific prompt builders. This pass adds the 14 missing prompt functions, the learning pack injection pipeline, skill loading, iteration feedback compression, context caching, and all per-role behavioral stanzas.

### Reference files (MUST-READ)

**Mori prompts (the master reference — read ALL of this)**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompts.rs` (5,914 LOC) — every line matters:
  - Lines 46-137: `budget_for()` per-role token caps (9 sections)
  - Lines 140-150: `PromptSection` with priority/cache_layer
  - Lines 164-334: **In-memory prefix cache** + **disk context-pack cache** (SHA-256 keyed to `.mori/memory/context-packs/`)
  - Lines 464-500: `compress_prior_task_outputs()` — max 3 outputs, 8 lines each, error/warning lines only
  - Lines 547-668: `build_learning_context_pack()` — assembles: plan learning, research memos, playbook hints, dependency/fixture/integration manifests. Cached by 14-component key.
  - Lines 790-842: `cross_plan_diff_section()` — diffs between merged batch and current worktree
  - Lines 1007-1086: `SharedPlanContext` + `format_shared_prefix()` — byte-identical prefix for API cache hits. Comment: "This ordering is load-bearing for prompt caching—do not rearrange."
  - Lines 1091-1143: `completion_summary_section()` — gathers all prior plan summaries
  - Lines 1153-1188: `compress_feedback()` — parses structured TOML review blocks, filters to unresolved blocking issues, produces tight fix directives
  - Lines 1220-1692: Implementer prompts (base + with-brief + fix-iteration)
  - Lines 1821-2507: Reviewer prompts (strategist, architect, auditor, combined)
  - Lines 2772-3110: Scribe/doc-revision/critic prompts
  - Lines 3265-3670: Refactorer, pre-planner, quick reviewer/fixer
  - Lines 3672-4970: Task implementer, batch implementer, file context injection
  - Lines 4757-4836: `scaled_prompt_cap()` — 16 task-metadata multipliers
  - Lines 5394-5780: Express mode (static brief, express implementer, auto-fix)

**Mori learning/skills**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/skills.rs` (129 LOC) — loads from `.claude/skills/{name}/SKILL.md`, role defaults, auto-detection, budget-aware injection
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/memory.rs` (3,107 LOC) — `IterationMemory` JSON persistence per-plan, error history between retries
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pattern_learning.rs` (487 LOC) — episode clustering → playbook rule promotion (+0.05) / demotion (-0.10) → provider recommendation
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/context.rs` (813 LOC) — `generate_workspace_map()`, `generate_filtered_workspace_map()`, `generate_preflight_snapshot()`, `extract_prd2_context()`, iteration archival, completion summaries
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/inject.rs` (1,266 LOC) — `ContextInjector`: copies plan artifacts into worktree `context/in/`, generates role-specific execution packs, filters AGENTS.md by role markers
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/reflection.rs` (227 LOC) — LLM-powered gate failure analysis: spawns claude-haiku-4-5 to generate structured "what failed / why / what to try differently / files to focus on" reflections, stores in iteration memory for next retry

**Mori-agents docs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/04-context-engineering.md` (548 LOC) — context strategy (McpFirst/Hybrid/InlineHeavy), file selection, prefix alignment
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/05-prompt-engineering.md` (540 LOC) — prompt design methodology, role stanzas
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/24-prompt-budget-engineering.md` (352 LOC) — token allocation, cache economics
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/mori-context-optimization.md` (583 LOC) — optimization techniques

**Component specs**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/prompt-templates.md` (481 LOC) — 9 template specs
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/context-pack-cache.md` (265 LOC) — SHA-256 keyed disk cache
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/prompt-budget-per-role.md` (237 LOC)
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/enrichment-steps.md` (609 LOC)

### Roko files to modify
- `crates/roko-compose/src/templates/` — add 14 missing prompt builders:
  - `implementer_fix.rs` — lightweight fix-only prompt for iteration 2+
  - `combined_reviewer.rs` — combined architect+auditor mode
  - `pre_planner.rs` — pre-planning pass
  - `batch_refactorer.rs` — cross-plan refactoring
  - `task_batch.rs` — batch task implementer (multiple tasks, one agent)
  - `plan_reviewer.rs` — post-merge review
  - `doc_verifier.rs` — documentation verification
  - `merge_resolver.rs` — merge conflict resolution
  - `error_diagnoser.rs` — structured error diagnosis
  - `dependency_validator.rs` — dependency check
  - `pattern_extractor.rs` — pattern extraction from episodes
  - `express.rs` — express mode (static brief + single-pass implementer + auto-fix)
- `crates/roko-compose/src/cache.rs` (NEW) — disk + memory prompt cache (SHA-256 keyed)
- `crates/roko-compose/src/learning_pack.rs` (NEW) — `build_learning_context_pack()` with 14-component cache key
- `crates/roko-compose/src/skills.rs` (NEW) — load from `.claude/skills/`, role defaults, auto-detect
- `crates/roko-compose/src/reflection.rs` (NEW) — LLM-powered gate failure reflection
- `crates/roko-compose/src/workspace_map.rs` (NEW) — generate workspace/crate map for prompts
- `crates/roko-compose/src/feedback_compress.rs` (NEW) — `compress_feedback()` parsing structured review TOML
- `crates/roko-cli/src/orchestrate.rs` — wire learning pack, skills, reflection, iteration memory

### Gap list (36 items)
1. **14 missing prompt functions** (implementer_fix, combined_reviewer, pre_planner, batch_refactorer, task_batch, plan_reviewer, doc_verifier, merge_resolver, error_diagnoser, dependency_validator, pattern_extractor, express_implementer, auto_fix, static_brief)
2. **No SharedPlanContext** — byte-identical prefix not built for cache alignment
3. **No in-memory prefix cache** — IMPLEMENTER_PREFIX_CACHE
4. **No disk context-pack cache** — SHA-256 keyed to `.roko/memory/context-packs/`
5. **No learning context pack** — episodes+playbook+research+deps+fixtures assembled into prompt section
6. **No playbook rule promotion/demotion** — +0.05 on success, -0.10 on failure, prune below 0.2
7. **No episode clustering** for playbook rule extraction
8. **No skill loading** from `.claude/skills/`
9. **No skill auto-detection** (e.g., ratatui skill for terminal work)
10. **No iteration memory** — JSON files per-plan between retries
11. **No compress_prior_task_outputs()** — max 3 outputs, 8 lines each, error/warning only
12. **No compress_feedback()** — parse structured TOML review, filter to unresolved blocking
13. **No implementer_fix_prompt()** — lightweight retry-specific prompt
14. **No cross_plan_diff_section()** — diff between merged batch and worktree
15. **No completion_summary_section()** — gather all prior plan summaries
16. **No workspace map generation** — `generate_workspace_map()`, `generate_filtered_workspace_map()`
17. **No preflight snapshot** — git log, git status, cargo check status
18. **No PRD2 extraction** — `extract_prd2_context()`
19. **No iteration archival** — archive briefs/reviews/docs between iterations
20. **No scaled_prompt_cap()** — 16 task-metadata multipliers for dynamic section sizing
21. **No express mode** — single-pass, no reviews, auto-fix on gate failure
22. **No inline file context injection** — `build_file_context_section()` reading actual files into prompts
23. **No ContextStrategy modes** — McpFirst/Hybrid/InlineHeavy with per-strategy file counts
24. **No compact_mcp_first_prefix()** — stripped prefix for MCP-first mode
25. **No prompt logging** — `log_prompt()` persisting metadata per prompt
26. **No LLM reflection** — spawning haiku to analyze gate failures with structured output
27. **No reflection → iteration memory** — storing reflections for next retry injection
28. **No reflection → playbook refresh** — `maybe_refresh_playbook_from_history()`
29. **No per-task metadata multipliers** (context_weight, reasoning_level, speed_priority, quality_profile) scaling caps
30. **No verification isolation** — implementer can see test source code (enables reward hacking)
31. **No role-specific guidance stanzas** — `mori_role_guidance()`, `mori_role_artifact_hint()`, `mori_tool_usage_guidance()`
32. **No auto-respond to agent questions** — detect "Would you like..." patterns and auto-reply to maintain flow (3 attempts then stop)
33. **No time estimator** — EMA-corrected duration estimates across phases
34. **No per-plan/per-task cost accumulation** — runtime cost tracking per plan_id and task_id
35. **No combined reviewer mode** — single agent running both architect+auditor perspectives
36. **No worktree context injection** — `ContextInjector` copying artifacts into `context/in/`

### Acceptance criteria
- [ ] All 24 role-specific prompt functions have templates
- [ ] SharedPlanContext built once per plan, byte-identical prefix for cache alignment
- [ ] Disk context-pack cache at `.roko/memory/context-packs/` with SHA-256 keys
- [ ] Learning pack assembles episodes+playbook+research+deps+fixtures
- [ ] Skills loaded from `.claude/skills/`, injected with budget-aware `<skill>` tags
- [ ] `compress_feedback()` parses structured review TOML, extracts unresolved issues
- [ ] `compress_prior_task_outputs()` caps at 3 outputs × 8 lines, error/warning filter
- [ ] LLM reflection spawns on gate failure, stores structured diagnosis in iteration memory
- [ ] Express mode: static brief → implementer → auto-fix, no reviews
- [ ] Prompt logging persists metadata per prompt to `.roko/learn/prompt-log.jsonl`

---

## Pass 9: Live Health Monitoring

### Scope
Mori has a 12-pattern `MonitorPool` that provides **live self-correction during agent execution**. This is the single largest architectural gap in agent supervision. Roko only has post-hoc gate results — no live anomaly detection.

### Reference files (MUST-READ)

**Mori monitor** (the reference implementation):
- `/Users/will/dev/uniswap/bardo/apps/mori/src/monitor/` (~535 LOC) — `MonitorPool` with:
  - `PatternMatcher` trait: receives `MonitorEvent`, returns `Option<Intervention>`
  - Cooldown system per pattern (configurable, default 5 minutes)
  - Intervention history tracking
  - 12 specific patterns:
    1. `CompileFailRepeat` — same compile error 3+ times → nudge with error context
    2. `ContextWindowPressure` — token usage > 80% → nudge to wrap up; > 95% → restart
    3. `StuckPattern` — agent repeating same output chunks → restart
    4. `TokenBurnRate` — >2000 tokens/min sustained 3+ min → warn
    5. `PhaseTimeout` — phase running > 20 min → graduated (restart → restart → force-advance)
    6. `TerminalUnresponsive` — 3+ health check failures → restart
    7. `TerminalRenderRegression` — screen passed, now fails → alert
    8. `GolemLifecycleViolation` — 3+ lifecycle failures → abort
    9. `SpecDriftAccumulation` — drift > 25% → alert
    10. `CoverageDrop` — coverage dropped > 5% → alert
    11. `CrossCrateBreak` — cross-crate conflict → alert
    12. `SpecWeakeningDetector` — invariant assertions removed → block

**Roko conductor** (existing, to extend):
- `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/` — has 10 watchers but they read signals after-the-fact, not live during execution

### Roko files to modify
- `crates/roko-conductor/src/monitor.rs` (NEW) — `MonitorPool` with `PatternMatcher` trait
- `crates/roko-conductor/src/monitor/` (NEW directory) — one file per pattern
- `crates/roko-cli/src/orchestrate.rs` — wire monitor into agent execution loop, feed streaming events

### Gap list
1. **No `MonitorPool`** — no live pattern matching during agent execution
2. **No `PatternMatcher` trait** — no interface for pluggable anomaly detectors
3. **No `Intervention` type** — no structured response to anomalies (nudge/restart/abort)
4. **No cooldown system** — no dedup preventing same intervention from re-firing
5. **No steering injection** — anomaly detection cannot inject messages into running agents
6. **12 missing live patterns** — CompileFailRepeat, ContextWindowPressure, StuckPattern, TokenBurnRate, PhaseTimeout, TerminalUnresponsive, TerminalRenderRegression, GolemLifecycleViolation, SpecDriftAccumulation, CoverageDrop, CrossCrateBreak, SpecWeakeningDetector

### Acceptance criteria
- [ ] `MonitorPool` with `PatternMatcher` trait and cooldown dedup
- [ ] 12 pattern matchers with live streaming event input
- [ ] Interventions injected as steering messages into running agents
- [ ] Integration test: simulate stuck agent → verify intervention fires within cooldown window

---

## Pass 10: Configuration Depth + Per-Role Settings

### Scope
Roko's config has ~15 top-level fields. Mori's has ~80+. The critical missing knobs: per-role model selection, per-role effort, per-role budget caps, 3-tier task routing, multi-backend support, agent role toggles, express mode.

### Reference files (MUST-READ)

**Mori config**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/state/config.rs` (2,149 LOC) — complete config schema with all ~80 fields
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/roles.rs` (297 LOC) — 28 roles, per-role backend/model/budget

**Mori-agents doc**:
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/21-configuration-reference.md` (1,217 LOC) — complete config reference

### Roko files to modify
- `crates/roko-cli/src/config.rs` — extend config schema
- `crates/roko-core/src/agent.rs` — extend `AgentRole` enum

### Key additions needed
1. **`[roles.*]` config section** — per-role model, effort, context_limit, budget_usd, enabled toggle
2. **`[task_routing]` section** — fast_task_model, standard_task_model, complex_task_model
3. **Multi-backend** — `[providers.claude]`, `[providers.codex]`, `[providers.cursor]` with per-backend defaults
4. **Express mode** — `express_mode = true`, `auto_fix_model`, `max_auto_fix_attempts`
5. **Knowledge injection toggles** — `knowledge_file_intel`, `knowledge_warnings`, `knowledge_wave_context`, etc.
6. **Per-plan overrides** — `[plan_overrides."plan-name"]` with model/provider/strategy
7. **Nuclear shutdown** — `kill_all_roko_descendants()` in process registry

### Acceptance criteria
- [ ] `roko.toml` supports `[roles.implementer]` with model/effort/budget/context_limit
- [ ] `roko.toml` supports `[task_routing]` with 3-tier model selection
- [ ] `roko.toml` supports `[providers.*]` with multiple backend configs
- [ ] `roko.toml` supports `express_mode` and `auto_fix_model`
- [ ] Per-role budget caps enforced ($0.40-$3.00 range)
- [ ] `kill_all_roko_descendants()` implemented as nuclear shutdown fallback

---

## Updated summary

| Pass | Scope | New gaps found in pass 2 | Total estimated LOC |
|---|---|---|---|
| **1. TUI** | Port mori's terminal UI | (already comprehensive) | ~15K |
| **2. Gates** | Build infrastructure | (already comprehensive) | ~2K |
| **3. Orchestrator** | Task-level scheduling | (already comprehensive) | ~5K |
| **4. Cognitive** | Grimoire + Daimon + Dreams | (already comprehensive) | ~15K |
| **5. Conductor** | LLM conductor + graduated actions | (already comprehensive) | ~3K |
| **6. Agent** | Backends + warm pool + streaming | (already comprehensive) | ~5K |
| **7. Code Intelligence** | MCP server + live index | **NEW** — the single most impactful gap | ~8K |
| **8. Prompt Composition** | **NEW** — 14 missing prompts, learning pack, skills, caching, reflection | **36 new gap items** | ~6K |
| **9. Live Monitoring** | **NEW** — 12-pattern MonitorPool with live steering | **12 new gap items** | ~2K |
| **10. Configuration** | **NEW** — per-role settings, multi-backend, express mode | **13 new config fields** | ~1K |
| **TOTAL** | | | **~62K** |

---

## Pass 11: Runtime Loop — Async Event-Driven Architecture

### Scope
The single biggest architectural gap: roko's orchestration loop is synchronous (blocks per action). Mori's is fully async with concurrent agent/gate/timer arms. This pass restructures `orchestrate.rs` from a tick loop to a `tokio::select!` event-driven loop.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs` (17,999 LOC) — read the main event loop (line ~9609), agent event handling, gate result processing, timer arms, message throttling
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` (3,358 LOC) — persistent agent pool, turn_start/turn_interrupt, warm pool, instance IDs

### Roko files to modify
- `crates/roko-cli/src/orchestrate.rs` — restructure main loop from tick-based to event-driven
- `crates/roko-agent/src/` — add `AgentEvent` enum, persistent pool, streaming support

### Key behavioral changes needed
1. Replace synchronous `dispatch_action().await` with concurrent `tokio::select!` arms
2. Agent events (MessageDelta, TurnCompleted, TokenUsage, Error, Exited) stream in real-time
3. Gates run in `tokio::spawn` tasks with 20-min timeout (not inline blocking)
4. Persistent agent pool with `turn_start()` / `turn_interrupt()` for multi-turn sessions
5. Global agent budget enforcement across all plans
6. Message processing throttle (20 per tick)
7. Spawn generation tracking to prevent stale completions
8. Duplicate dispatch suppression
9. Dead-on-arrival detection (<15s exit with <80 chars output)
10. Agent question auto-respond ("Would you like..." patterns, 3 attempts)

### Acceptance criteria
- [ ] Main loop uses `tokio::select!` with concurrent agent/gate/timer arms
- [ ] Agents stream MessageDelta events in real-time
- [ ] Gates run concurrently (not inline blocking)
- [ ] Persistent agent pool reuses warm processes across turns
- [ ] Global agent budget enforced (max N concurrent across all plans)

---

## Pass 12: Git Operations — Branch Strategy + Safe Merge

### Scope
Port mori's 3-tier branch hierarchy, worktree health diagnosis, safe merge pipeline, and file overlay system.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` (3,225 LOC) — full worktree lifecycle: create (5 types), health diagnosis (5 states), recovery snapshots, file overlay, IDE config injection, disk management, safe merge, cherry-pick refresh
- `/Users/will/dev/uniswap/bardo/apps/mori/src/git/mod.rs` (1,272 LOC) — batch branch setup, merge-via-temp-worktree, auto-stash session, branch tree builder
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/20-mori-data-directory.md` (862 LOC) — full .mori/ layout reference

### Roko files to modify
- `crates/roko-orchestrator/src/worktree.rs` — upgrade from 1-type to 5-type worktree creation, add health diagnosis, recovery snapshots, file overlay
- `crates/roko-orchestrator/src/git.rs` (NEW) — batch branch management, safe merge pipeline, auto-stash
- `crates/roko-cli/src/orchestrate.rs` — wire batch branch, merge queue, worktree-safe merges

### Key additions
1. Batch branch: `roko/batch/YYYYMMDD` as integration target
2. Task branches: `roko/plan/{base}/{task_id}` for parallel task isolation
3. Health diagnosis: 5-state `RecoveryDecision` (Healthy/NeedsResync/NeedsRebase/Quarantine/ManualAttention)
4. Recovery snapshots: archive diffs, logs, status under `.roko/runs/recovery/`
5. File overlay: copy Cargo.toml, Cargo.lock, .cargo/ into worktrees
6. IDE config injection: write .cargo/config.toml (target redirect + sccache)
7. Safe merge: merge inside worktree + `git update-ref` (never checkout in main repo)
8. Merge feasibility: `git merge-tree --write-tree` dry run
9. Wire existing `MergeQueue` into the merge path
10. Auto-tagging: `plan/{plan_base}` tags on merge

### Acceptance criteria
- [ ] Batch branch created per execution session
- [ ] Health diagnosis returns 5-state `RecoveryDecision`
- [ ] Merge uses worktree-safe strategy (never runs in main repo)
- [ ] `MergeQueue` serializes concurrent merges with file-overlap detection
- [ ] File overlay copies workspace root files into worktrees
- [ ] `.cargo/config.toml` written into each worktree (shared target + sccache)

---

## Pass 13: Safety Hardening — Loop Guard + Audit Chain + Proptest

### Scope
Add the three most critical safety gaps: degenerate-loop detection, tamper-evident audit log, and property-based testing.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/crates/golem-safety/src/loop_guard.rs` (~230 LOC) — sliding window, identical repetition detection (block at 5), tool domination warning (>80%)
- `/Users/will/dev/uniswap/bardo/crates/golem-safety/src/audit.rs` (~400 LOC) — SHA-256 hash chain, 11 event types, verification, on-chain anchoring
- `/Users/will/dev/uniswap/bardo/crates/golem-safety/tests/proptests.rs` (518 LOC) — proptest suites for permit lifecycle, loop guard, taint, allowlist

### Roko files to modify
- `crates/roko-agent/src/safety/loop_guard.rs` (NEW) — `LoopGuard` with sliding window, identical call detection, domination warning
- `crates/roko-agent/src/safety/mod.rs` — wire loop guard into `SafetyLayer::check_pre_execution()`
- `crates/roko-agent/src/safety/audit.rs` (NEW) — `AuditChain` with SHA-256 hash chain, verification
- `crates/roko-agent/tests/proptests.rs` (NEW) — proptest suites for all 6 safety policy families
- `crates/roko-gate/tests/proptests.rs` (NEW) — proptest for gate pipeline invariants
- `crates/roko-orchestrator/tests/proptests.rs` (NEW) — proptest for executor state machine

### Acceptance criteria
- [ ] `LoopGuard` blocks at 5th identical (tool, args) call in sliding window
- [ ] `LoopGuard` warns when one tool exceeds 80% of window
- [ ] `AuditChain` produces tamper-evident hash chain with `verify()` function
- [ ] proptest suites exist for safety policies (bash, git, network, path, scrub, rate_limit, loop_guard)
- [ ] proptest suites exist for executor state machine (phase transitions, snapshot roundtrip)
- [ ] `cargo test --workspace` passes with all new proptests

---

## Pass 14: Task Metadata + Enrichment Routing

### Scope
Add mori's 20+ task routing metadata fields (category, reasoning_level, speed_priority, quality_profile, context_weight, complexity_band) and the `enrich routing` command that classifies tasks via LLM.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/tasks.rs` (2,086 LOC) — full task schema with 20+ routing fields, `TaskCategory` (8 variants), `TaskReasoningLevel`, `TaskSpeedPriority`, `TaskQualityProfile`, `TaskContextWeight`, `TaskComplexityBand`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/complexity.rs` (467 LOC) — 4-level plan classification, 6-phase heuristic routing band for individual tasks
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompts.rs` lines 4757-4836 — `scaled_prompt_cap()` with 16 task-metadata multipliers
- `/Users/will/dev/uniswap/bardo/apps/mori/src/cli_tools.rs` — `mori enrich routing` subcommand

### Roko files to modify
- `crates/roko-cli/src/task_parser.rs` — add 20+ routing fields to `TaskDef`
- `crates/roko-core/src/task.rs` — add `TaskCategory`, `TaskReasoningLevel`, etc. enums
- `crates/roko-compose/src/budget.rs` — wire `scaled_prompt_cap()` with task-metadata multipliers
- `crates/roko-cli/src/main.rs` — add `roko enrich routing` subcommand

### Acceptance criteria
- [ ] `TaskDef` has category, reasoning_level, speed_priority, quality_profile, context_weight, complexity_band fields
- [ ] `roko enrich routing` classifies tasks via LLM into routing metadata
- [ ] `scaled_prompt_cap()` adjusts per-section budgets based on task metadata (16 multipliers)
- [ ] Plan-level complexity classification (Trivial/Simple/Standard/Complex) affects review pipeline

---

## Updated complete summary

| Pass | Scope | Gaps | Est. LOC |
|---|---|---|---|
| 1. TUI | Port mori's terminal UI | 68 | ~15K |
| 2. Gates | Build infrastructure | 17 | ~2K |
| 3. Orchestrator | Task scheduling + merge queue | 30+ | ~5K |
| 4. Cognitive | Grimoire + Daimon + Dreams | 40+ | ~15K |
| 5. Conductor | LLM conductor + actions | 15 | ~3K |
| 6. Agent | Backends + pool + prompts | 15 | ~5K |
| 7. Code Intelligence | MCP server + live index | 8 | ~8K |
| 8. Prompt Composition | 14 prompts + caching + learning | 36 | ~6K |
| 9. Live Monitoring | 12-pattern MonitorPool | 12 | ~2K |
| 10. Configuration | Per-role settings + multi-backend | 13 | ~1K |
| **11. Runtime Loop** | **Async event-driven architecture** | **28** | **~5K** |
| **12. Git Operations** | **Branch strategy + safe merge** | **28** | **~4K** |
| **13. Safety Hardening** | **Loop guard + audit chain + proptest** | **8** | **~2K** |
| **14. Task Metadata** | **Routing fields + enrich command** | **6** | **~2K** |
| **TOTAL** | | **~300+** | **~75K** |

---

## Pass 15: Learning Feedback — Fix the Broken Read-Back Path

### Scope
The most operationally significant fix. Roko correctly records 11 learning updates per run, but the data never flows back into agent prompts because `MatchContext` is empty and there's no plan-scoped retrieval. This pass wires the read-back path.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/memory.rs` (3,107 LOC) — read ALL:
  - `render_plan_learning_md()` (lines 1490-1721) — 230-line plan-scoped learning markdown generator
  - `render_file_intel_md()` (lines 2693-2770) — per-file difficulty profiles
  - `render_plan_research_md()` — research context with verification posture
  - `build_reflection_playbook_rules()` — extract rules from failure clusters
  - `build_success_playbook_rules()` — extract rules from success clusters
  - `compute_file_profiles()` — pass rate, avg iterations, common errors per file
  - `find_error_patterns()` — error pattern clustering
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/reflection.rs` (227 LOC) — read ALL:
  - `spawn_reflection()` — Claude Haiku analyzes gate failures
  - Structured output: "what failed / why / what to try / files to focus on"
  - Deduplication: skips if same error pattern already reflected
  - Storage: iteration memory JSON per plan
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/inject.rs` (1,266 LOC) — read context injection functions:
  - `inject_implementer_context_in_worktree()` — writes learning.md, research.md, file-intel.md into worktree
  - `apply_playbook_hints_to_task()` — matched playbook rules override model/provider/strategy

### Roko files to modify
- `crates/roko-cli/src/orchestrate.rs` — fix `build_learned_context()`:
  - **Populate `MatchContext.files`** from task's `files` field (currently `Vec::new()`)
  - **Populate `MatchContext.tags`** from task tags (currently `Vec::new()`)
  - **Add plan-scoped episode retrieval** — filter episodes by `plan_id` before assembling learning context
- `crates/roko-learn/src/plan_learning.rs` (NEW) — port `render_plan_learning_md()`:
  - Same-plan success/failure episode aggregation
  - Common failure signature clustering
  - Matched playbook rules with confidence and match reasons
  - Learned execution hints (model, provider, strategy)
- `crates/roko-learn/src/file_profiles.rs` (NEW) — port `compute_file_profiles()` and `render_file_intel_md()`:
  - Per-file pass rate, avg iterations, common errors
  - Best model/provider per file
  - Error pattern clusters with fix patterns
- `crates/roko-learn/src/reflection.rs` (NEW) — port `spawn_reflection()`:
  - On gate failure, call Claude Haiku with gate output
  - Generate structured diagnosis (what/why/fix/files)
  - Store in iteration memory JSON per plan
  - Dedup by error pattern
- `crates/roko-learn/src/iteration_memory.rs` (NEW) — port `IterationMemory`:
  - JSON persistence per plan under `.roko/learn/iteration-memory/`
  - Read between retries to provide error history
  - Include latest reflection in next prompt

### Acceptance criteria
- [ ] `MatchContext.files` populated from task files (not empty)
- [ ] `MatchContext.tags` populated from task tags (not empty)
- [ ] Playbook rules with file triggers actually fire during task execution
- [ ] Plan-scoped episode history injected into prompts (only episodes from current plan)
- [ ] File difficulty profiles computed and available for prompt injection
- [ ] Gate failure reflection generates structured diagnosis via Haiku
- [ ] Iteration memory persists between retries and feeds into next prompt
- [ ] End-to-end test: run 3 tasks in same plan, verify 3rd task sees learning from 1st and 2nd

---

## Pass 16: Conductor Action Vocabulary + Graduated Watchers

### Scope
Expand `ConductorDecision` from 3 variants to 10+, add `SendMessage` capability (the most effective intervention), implement graduated escalation in existing watchers, add 4 missing watchers.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/watchers.rs` (967 LOC) — read ALL:
  - AgentSilence: 180s no output → nudge (doubled for Claude)
  - TaskStall: 300s no progress → nudge
  - PhaseTimeout: 30min → graduated (restart 1 → restart 2 → ForceAdvance), per-plan `phase_timeout_restarts` counter
  - TaskContinuation: finished tasks + more queued → `AssignAdditionalTasks` to warm agent
  - CompileFailRepeat: 3→nudge with error text, 5→restart, 7→ForceAdvance
  - ContextWindowPressure: 80%→nudge "wrap up and write to CONTEXT.md", 95%→hard restart
  - TestFailureBudget: ≥70% pass rate → ForceAdvance (accept partial success)

### Roko files to modify
- `crates/roko-conductor/src/conductor.rs` — expand `ConductorDecision` enum:
  ```rust
  enum ConductorDecision {
      Continue,
      Nudge { message: String },           // NEW: send guidance to running agent
      Restart,
      ForceAdvance,                         // NEW: skip past stuck phase
      SkipReviews,                          // NEW: bypass review loop
      AssignAdditionalTasks,                // NEW: feed more work to warm agent
      Fail,
  }
  ```
- `crates/roko-conductor/src/watchers/compile_fail_repeat.rs` — add graduated escalation (3/5/7)
- `crates/roko-conductor/src/watchers/context_window_pressure.rs` — add 95% hard-stop tier
- `crates/roko-conductor/src/watchers/review_loop.rs` — produce SkipReviews instead of Restart
- `crates/roko-conductor/src/watchers/test_failure_budget.rs` — change from regression detection to pass-rate acceptance (≥70% → ForceAdvance)
- `crates/roko-conductor/src/watchers/agent_silence.rs` (NEW) — 180s no output → nudge
- `crates/roko-conductor/src/watchers/task_stall.rs` (NEW) — 300s no progress → nudge
- `crates/roko-conductor/src/watchers/phase_timeout.rs` (NEW) — 30min graduated with per-plan restart counter
- `crates/roko-conductor/src/watchers/task_continuation.rs` (NEW) — warm agent task assignment
- `crates/roko-cli/src/orchestrate.rs` — handle new `ConductorDecision` variants (Nudge sends message to agent, ForceAdvance advances plan phase, SkipReviews bypasses review)

### Acceptance criteria
- [ ] `ConductorDecision` has Nudge, ForceAdvance, SkipReviews, AssignAdditionalTasks variants
- [ ] CompileFailRepeat: 3→nudge with error text, 5→restart, 7→ForceAdvance
- [ ] ContextWindowPressure: 80%→nudge, 95%→restart
- [ ] AgentSilence: 180s no output → nudge message
- [ ] PhaseTimeout: 30min → graduated (restart → restart → ForceAdvance) with per-plan counter
- [ ] TestFailureBudget: ≥70% pass rate → ForceAdvance
- [ ] Nudge messages actually delivered to running agents (requires agent streaming from Pass 11)

---

## Pass 17: Observability — File Tracing + Crash Reports + OS Metrics

### Scope
Three blockers for production use: logs disappear when TUI runs, crashes lose context, no system resource visibility.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/sys_metrics.rs` (369 LOC) — `SysCollector`: CPU/memory/disk/network, process attribution to plans, active `rustc` compilation detection via `/proc`-style process arg parsing
- `/Users/will/dev/uniswap/bardo/apps/mori/src/main.rs` lines 1205-1309 — crash reporting: panic hook, `CRASH_STATE` global, `CrashReport` struct, error signatures, environment capture
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/util.rs` lines 566-723 — atomic checkpoint writing via temp file + rename + `spawn_blocking`

### Roko files to modify
- `crates/roko-cli/src/main.rs` — add:
  - `tracing_appender` dependency for file-based logging (route to `.roko/runs/roko.log`)
  - Panic hook that captures backtrace + executor state + recent logs → writes `CrashReport` to `.roko/crashes/`
  - `CRASH_STATE` global updated every 2s from orchestration loop
  - SIGPIPE ignore
- `crates/roko-core/src/obs/sys_metrics.rs` (NEW) — port `SysCollector`:
  - CPU/memory/disk/network polling via `sysinfo` crate (already a workspace dep)
  - Process tree walking to attribute child processes to plans
  - Active `rustc` compilation detection per worktree
  - Rolling 60-sample history for sparklines
  - Background thread with channel delivery
- `crates/roko-cli/src/orchestrate.rs` — add `#[instrument]` to 15+ key functions (gate runs, agent dispatch, plan phase transitions, merge operations)

### Acceptance criteria
- [ ] Tracing routed to `.roko/runs/roko.log` when TUI is active
- [ ] Crash reports written to `.roko/crashes/` with backtrace, executor state, recent logs
- [ ] `SysCollector` provides CPU/memory/disk/network metrics on background thread
- [ ] Process tree attribution identifies which plan a `rustc` process belongs to
- [ ] 15+ functions annotated with `#[instrument]` for structured span context

---

## Final complete summary (all 17 passes)

| # | Pass | Gaps | Est. LOC | Priority |
|---|---|---|---|---|
| 1 | TUI | 68 | ~15K | P1 (visual) |
| 2 | Gates infrastructure | 17 | ~2K | P0 (performance) |
| 3 | Orchestrator (task scheduling) | 30+ | ~5K | P0 (correctness) |
| 4 | Cognitive (grimoire/daimon/dreams) | 40+ | ~15K | P2 (intelligence) |
| 5 | Conductor (LLM + actions) | 15 | ~3K | P1 (recovery) |
| 6 | Agent (backends + pool) | 15 | ~5K | P1 (model diversity) |
| 7 | Code Intelligence (MCP + index) | 8 | ~8K | P0 (agent quality) |
| 8 | Prompt Composition | 36 | ~6K | P0 (agent quality) |
| 9 | Live Monitoring | 12 | ~2K | P1 (recovery) |
| 10 | Configuration | 13 | ~1K | P1 (tuning) |
| 11 | Runtime Loop (async) | 28 | ~5K | P0 (architecture) |
| 12 | Git Operations | 28 | ~4K | P0 (safety) |
| 13 | Safety Hardening | 8 | ~2K | P1 (safety) |
| 14 | Task Metadata | 6 | ~2K | P1 (routing) |
| **15** | **Learning Feedback (broken read-back)** | **8** | **~3K** | **P0 (self-improvement)** |
| **16** | **Conductor Actions + Graduated Watchers** | **12** | **~2K** | **P0 (recovery)** |
| **17** | **Observability (tracing + crashes + metrics)** | **5** | **~2K** | **P0 (operations)** |
| | **TOTAL** | **~350+** | **~82K** | |

### Recommended execution order (by P0 impact):

1. **Pass 15** — Fix learning feedback read-back (highest ROI: data already recorded, just needs to flow back)
2. **Pass 12** — Git operations safety (merging in main repo is dangerous)
3. **Pass 11** — Async runtime loop (enables streaming, monitoring, concurrency)
4. **Pass 16** — Conductor action vocabulary (enables graduated recovery)
5. **Pass 7** — Code intelligence MCP server (biggest agent quality improvement)
6. **Pass 8** — Prompt composition depth (14 missing prompts + caching)
7. **Pass 2** — Gate infrastructure (affected-crate scoping, error digests)
8. **Pass 17** — Observability (file tracing, crash reports, OS metrics)
9. **Pass 3** — Orchestrator task scheduling + merge queue
10. **Pass 1** — TUI visual parity

---

## Pass 18: Prompt Text Depth — Port Mori's Operational Guidance

### Scope
Roko's templates have the right structure but only 20-30% of mori's battle-tested guidance text. This pass ports the specific behavioral instructions, anti-patterns, checklists, and accountability framing that prevent common agent failures.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompts.rs` (5,914 LOC) — read the TEXT content of every prompt function, not just the structure. Key sections:
  - Lines 1204-1216: `implementation_reconciliation_guidance()` — 8-item "Working Against A Live Repo" checklist
  - Lines 1246-1262: "What Reviewers Will Check" pre-emption block (architect + auditor criteria)
  - Lines 1296-1305: Self-validation sequence with retry limits
  - Lines 1325-1352: "Before You Finish" checklist + completion report format
  - Lines 1446-1476: Iteration/fix mode guidance ("STOP — COMPILATION FAILED", "DO NOT re-implement from scratch")
  - Lines 2180-2217: Reviewer "BE EXHAUSTIVE" + "DO NOT REVISE for" nit exclusion list
  - Lines 2223-2225: "APPROVE if compile and tests pass" default + fix_hint mandate
  - Lines 2429-2440: Auditor invariant verification procedure
  - Lines 2817-2999: Scribe 7-section structure + diagram requirements + pre-submission checklist
  - Lines 5749-5780: Auto-fix dependency-churn prevention rules
  - Lines 867-936: MCP tools stanza (specific tool signatures) + role pack guidance

### Roko files to modify
- `crates/roko-compose/src/role_prompts.rs` — expand every `*_ROLE_IDENTITY` constant from ~500 chars to ~3,000+ chars, porting mori's guidance text verbatim (adapted for roko's context)
- `crates/roko-compose/src/templates/implementer.rs` — add "Before You Finish" checklist, completion report format, iteration-specific guidance
- `crates/roko-compose/src/templates/reviewer.rs` — add "DO NOT REVISE for" list, "BE EXHAUSTIVE" instruction, fix_hint mandate, "APPROVE if compile and tests pass" default
- `crates/roko-compose/src/templates/quick.rs` — add auto-fix dependency-churn prevention rules
- `crates/roko-compose/src/templates/scribe.rs` — add 7-section structure instructions, diagram requirements, pre-submission 13-item checklist
- `crates/roko-compose/src/templates/common.rs` — expand MCP tools stanza with specific tool signatures, add role pack guidance

### Key text to port (20+ items)
1. Full 8-item "Working Against A Live Repo" reconciliation guidance
2. "What Reviewers Will Check" pre-emption block with accountability framing
3. 5-step self-validation sequence with "max 3 attempts, then document and move on"
4. "Before You Finish" 4-item checklist (grep .unwrap(), check exports, verify docs, re-read reviewer criteria)
5. Completion report format (types defined, deviations, test results, self-check TOML)
6. "STOP — COMPILATION FAILED" iteration escalation
7. "DO NOT re-implement from scratch" on fix iterations
8. "DO NOT REVISE for" nit exclusion list (8 specific exclusions)
9. "APPROVE if compile and tests pass" default reviewer posture
10. fix_hint mandate with example
11. "An incomplete review that forces a second cycle is a failure of your role"
12. Auto-fix dependency-churn prevention (items 4-7 from mori's auto_fix_prompt)
13. "Do NOT re-run cargo test — use gate results" for reviewers
14. MCP tool signatures (search_code, get_symbol_context, find_references, workspace_map)
15. "Start with context/in/{role}-pack.md when present"
16. "Check first: If tmp/agent-messages.md exists, read it"
17. Scribe 7-section detailed instructions (what goes in each section)
18. Scribe diagram requirements (types, placement, color coding, numbering, max nodes)
19. Scribe pre-submission 13-item checklist
20. Auditor 4-step invariant coverage check procedure
21. "If a dependency from a prior plan is missing, add a stub with todo!()"

### Acceptance criteria
- [ ] Every role identity constant is 2,000+ chars (up from ~500)
- [ ] Implementer has "Before You Finish" checklist and completion report format
- [ ] Reviewer has "DO NOT REVISE for" exclusion list and fix_hint mandate
- [ ] Auto-fixer has dependency-churn prevention rules
- [ ] Scribe has 7-section instructions, diagram requirements, and pre-submission checklist
- [ ] MCP stanza lists specific tool signatures, not generic "use MCP tools"
- [ ] Running `roko plan run` produces agents that follow the guidance (manual verification)

---

## Pass 19: Wire Dispatch Loop Gates + Express Mode Routing

### Scope
Two specific stubs that need to become real: (1) the no-op `run_template_gates()` in the dispatch path, (2) the express mode routing logic in the orchestrator.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/gates.rs` (1,758 LOC) — gate execution in mori's runtime
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompts.rs` lines 5394-5780 — `generate_static_brief()`, `express_implementer_prompt()`, `auto_fix_prompt()`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs` — search for "express" to trace the express mode flow

### Roko files to modify
- `crates/roko-serve/src/dispatch.rs` — replace `run_template_gates()` stub with real gate execution using `roko-gate` pipeline
- `crates/roko-cli/src/orchestrate.rs` — add express mode routing: check `config.express_mode`, skip enrichment/strategist phases, bypass reviews after gates pass, wire rebase recovery on auto-fix exhaustion
- `crates/roko-compose/src/templates/express.rs` (NEW) — `generate_static_brief()` + `express_implementer_prompt()`

### Acceptance criteria
- [ ] `run_template_gates()` runs real gates (compile, test, clippy at minimum) on webhook-dispatched agent output
- [ ] Express mode skips strategist/enrichment when `express_mode = true`
- [ ] Express mode bypasses reviews after gates pass
- [ ] Express mode auto-fix uses fast model (haiku) with minimal prompt
- [ ] `generate_static_brief()` programmatically builds context bundle without LLM

---

## FINAL complete summary (all 19 passes)

| # | Pass | Gaps | Est. LOC | Priority |
|---|---|---|---|---|
| 1 | TUI | 68 | ~15K | P1 |
| 2 | Gates infrastructure | 17 | ~2K | P0 |
| 3 | Orchestrator (task scheduling) | 30+ | ~5K | P0 |
| 4 | Cognitive (grimoire/daimon/dreams) | 40+ | ~15K | P2 |
| 5 | Conductor (LLM + actions) | 15 | ~3K | P1 |
| 6 | Agent (backends + pool) | 15 | ~5K | P1 |
| 7 | Code Intelligence (MCP + index) | 8 | ~8K | P0 |
| 8 | Prompt Composition (structure) | 36 | ~6K | P0 |
| 9 | Live Monitoring | 12 | ~2K | P1 |
| 10 | Configuration | 13 | ~1K | P1 |
| 11 | Runtime Loop (async) | 28 | ~5K | P0 |
| 12 | Git Operations | 28 | ~4K | P0 |
| 13 | Safety Hardening | 8 | ~2K | P1 |
| 14 | Task Metadata | 6 | ~2K | P1 |
| 15 | Learning Feedback (read-back) | 8 | ~3K | P0 |
| 16 | Conductor Actions + Watchers | 12 | ~2K | P0 |
| 17 | Observability | 5 | ~2K | P0 |
| **18** | **Prompt Text Depth** | **21** | **~2K** | **P0** |
| **19** | **Dispatch Gates + Express Mode** | **5** | **~2K** | **P1** |
| | **TOTAL** | **~370+** | **~86K** | |

### Updated execution order by impact:

**P0 — Blocks self-hosting quality:**
1. Pass 15 — Fix learning feedback read-back (highest ROI, smallest change)
2. Pass 18 — Port prompt operational guidance (biggest agent behavior improvement)
3. Pass 12 — Git merge safety (merging in main repo is dangerous)
4. Pass 11 — Async runtime loop (enables streaming, monitoring, concurrency)
5. Pass 16 — Conductor action vocabulary (enables graduated recovery)
6. Pass 7 — Code intelligence MCP server (biggest structural understanding boost)
7. Pass 8 — Prompt composition structure (14 missing prompt builders + caching)
8. Pass 2 — Gate infrastructure (affected-crate scoping, error digests)
9. Pass 17 — Observability (file tracing, crash reports, OS metrics)
10. Pass 3 — Orchestrator task scheduling + merge queue

**P1 — Important but not blocking:**
11. Pass 1 — TUI visual parity
12. Pass 19 — Dispatch gates + express mode
13. Pass 5 — Conductor LLM agent
14. Pass 6 — Agent backends + warm pool
15. Pass 9 — Live monitoring patterns
16. Pass 10 — Configuration depth
17. Pass 13 — Safety hardening (loop guard, audit chain, proptest)
18. Pass 14 — Task metadata + enrichment routing

**P2 — Aspirational:**
19. Pass 4 — Cognitive layer (grimoire/daimon/dreams full depth)

---

## Pass 20: Wire Dead Signal Features — Decay, Score, GC, Taint

### Scope
The Signal type has rich features that are architecturally complete but functionally dead at runtime. This pass wires the most impactful ones: scoring for retrieval ranking, decay for knowledge freshness, GC for log maintenance, and taint propagation for safety.

### Reference files (MUST-READ)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/score.rs` — 4-axis scoring system
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/decay.rs` — 4 decay variants
- `/Users/will/dev/nunchi/roko/roko/crates/roko-fs/src/gc.rs` — GcEngine with scan/collect/dry_run
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/safety/taint_propagation.rs` — TaintTracker (15 tests)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/scorer.rs` — CompositionScorer
- `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/scorer.rs` — SumScorer, MulScorer

### Roko files to modify
- `crates/roko-cli/src/orchestrate.rs` — replace `NoOpScorer` with `CompositionScorer`; set meaningful Decay on signals; instantiate `TaintTracker`; schedule periodic `GcEngine::collect()`
- `crates/roko-cli/src/run.rs` — same: wire real scorer and decay
- `crates/roko-compose/src/prompt.rs` — use `weight_at()` (score × decay) for section priority ranking

### Acceptance criteria
- [ ] Gate verdict signals have Score with confidence from pass rate
- [ ] Episode signals have Decay::HalfLife (not just WISDOM)
- [ ] Agent output signals from external provenance are marked `tainted: true`
- [ ] TaintTracker instantiated and propagation checked before tool dispatch
- [ ] GcEngine runs on daemon shutdown or periodic schedule
- [ ] `CompositionScorer` instantiated and used for prompt section ranking
- [ ] `FileSubstrate::query()` returns meaningfully different ordering based on score+decay

---

## Pass 21: Wire Dead Budget + Cost Infrastructure

### Scope
`BudgetConfig`, `TurnBudget`, `CostsDb`, `CostsLog` all exist but are decorative. This pass wires them into the runtime so costs are tracked, budgets enforced, and the data flows back into routing decisions.

### Reference files (MUST-READ)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` lines 704-736 — `BudgetConfig`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs` lines 189-219 — `TurnBudget` per role
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_db.rs` — `CostsDb`, `CostRecord`, `CostSummary`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_log.rs` — `CostsLog` (append-only JSONL)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/events.rs` lines 314-358 — per-agent/plan/task cost tracking
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/provider_routing.rs` (298 LOC) — cost-weighted provider recommendation

### Roko files to modify
- `crates/roko-cli/src/orchestrate.rs` — instantiate CostsDb + CostsLog; after each agent turn, populate CostRecord and append; maintain running plan_cost total; check max_plan_usd after each turn; check TurnBudget.base_usd before dispatch; pass `--max-budget` to ClaudeCliAgent
- `crates/roko-agent/src/claude_cli_agent.rs` — add `--max-budget` CLI flag support
- `crates/roko-conductor/src/watchers/cost_overrun.rs` — wire to use live plan cost total instead of depending on signals

### Acceptance criteria
- [ ] `CostsDb` and `CostsLog` instantiated in PlanRunner
- [ ] `CostRecord` written after every agent turn
- [ ] Running `plan_cost` total maintained per plan
- [ ] Execution paused when `plan_cost > max_plan_usd`
- [ ] `--max-budget` passed to Claude CLI per TurnBudget.base_usd for the dispatched role
- [ ] Dynamic context window from provider (not hardcoded per model name)
- [ ] Context prune limit scales with actual model context window

---

## Pass 22: Wire AGENTS.md + Dead Task Routing Enums

### Scope
Connect two built-but-unused modules: the AGENTS.md parser and the task routing enums.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/inject.rs` lines 176-209 — role-filtered AGENTS.md injection with `<!-- role: ... -->` markers
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompts.rs` — how `agents_md` content is used as priority-5 cache-layer-1 section
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/agents_md.rs` — existing parser (never called)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/task.rs` — TaskReasoningLevel, TaskSpeedPriority, TaskQualityProfile, TaskContextWeight (defined, never consumed)

### Roko files to modify
- Create `/Users/will/dev/nunchi/roko/roko/AGENTS.md` — project-level AGENTS.md with `<!-- role: ... -->` markers
- `crates/roko-compose/src/agents_md.rs` — add role-filtering via `<!-- role: ... -->` marker parsing
- `crates/roko-cli/src/orchestrate.rs` — load AGENTS.md, filter by role, inject into SystemPromptBuilder conventions layer for ALL roles (not just reviewer/scribe)
- `crates/roko-cli/src/task_parser.rs` — parse TaskReasoningLevel etc. from tasks.toml
- `crates/roko-compose/src/budget.rs` — wire task routing enums into `scaled_prompt_cap()` multipliers
- `crates/roko-learn/src/cascade_router.rs` — incorporate reasoning_level and speed_priority into context vector

### Acceptance criteria
- [ ] AGENTS.md exists in repo root with `<!-- role: ... -->` markers
- [ ] Each agent receives only its role-relevant AGENTS.md sections
- [ ] AGENTS.md content injected into SystemPromptBuilder conventions layer
- [ ] TaskReasoningLevel, TaskSpeedPriority, TaskQualityProfile, TaskContextWeight parsed from tasks.toml
- [ ] `scaled_prompt_cap()` uses task metadata to adjust section budgets (16 multipliers)

---

## Pass 23: Edge Case Hardening — Concurrent Runs, Disk, OOM

### Scope
Critical operational gaps: no concurrent run protection, no disk space awareness, no OOM distinction.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` lines 1492-1574 — disk space monitoring and reclamation
- `/Users/will/dev/uniswap/bardo/apps/mori/src/main.rs` lines 1205-1309 — crash reporting + global CRASH_STATE

### Roko files to modify
- `crates/roko-cli/src/orchestrate.rs` — add advisory file lock on `.roko/state/executor.lock`; reject empty task array; add pre-flight disk space check; distinguish OOM/SIGKILL from timeout in failure classification; warn on stale snapshot resume
- `crates/roko-fs/src/file_substrate.rs` — add `sync_all()` after `put()` writes (currently only `flush()`)
- `crates/roko-fs/src/layout.rs` — add `executor_lock()` path helper

### Acceptance criteria
- [ ] Advisory lock prevents two `roko plan run` from running simultaneously
- [ ] Empty task array rejected with clear error
- [ ] Pre-flight check warns when disk < 5 GiB
- [ ] OOM-killed agents classified as `FailureKind::OutOfMemory` (distinct from timeout)
- [ ] Stale snapshot produces warning log on resume
- [ ] `FileSubstrate::put()` calls `sync_all()` for durability

---

## FINAL complete summary (all 23 passes)

| # | Pass | Gaps | Est. LOC | Priority |
|---|---|---|---|---|
| 1 | TUI | 68 | ~15K | P1 |
| 2 | Gates infrastructure | 17 | ~2K | P0 |
| 3 | Orchestrator (task scheduling) | 30+ | ~5K | P0 |
| 4 | Cognitive (grimoire/daimon/dreams) | 40+ | ~15K | P2 |
| 5 | Conductor (LLM + actions) | 15 | ~3K | P1 |
| 6 | Agent (backends + pool) | 15 | ~5K | P1 |
| 7 | Code Intelligence (MCP + index) | 8 | ~8K | P0 |
| 8 | Prompt Composition (structure) | 36 | ~6K | P0 |
| 9 | Live Monitoring | 12 | ~2K | P1 |
| 10 | Configuration | 13 | ~1K | P1 |
| 11 | Runtime Loop (async) | 28 | ~5K | P0 |
| 12 | Git Operations | 28 | ~4K | P0 |
| 13 | Safety Hardening | 8 | ~2K | P1 |
| 14 | Task Metadata | 6 | ~2K | P1 |
| 15 | Learning Feedback (read-back) | 8 | ~3K | P0 |
| 16 | Conductor Actions + Watchers | 12 | ~2K | P0 |
| 17 | Observability | 5 | ~2K | P0 |
| 18 | Prompt Text Depth | 21 | ~2K | P0 |
| 19 | Dispatch Gates + Express Mode | 5 | ~2K | P1 |
| **20** | **Wire Dead Signal Features** | **7** | **~1K** | **P1** |
| **21** | **Wire Dead Budget + Cost** | **7** | **~2K** | **P0** |
| **22** | **Wire AGENTS.md + Task Enums** | **6** | **~2K** | **P1** |
| **23** | **Edge Case Hardening** | **6** | **~1K** | **P0** |
| | **TOTAL** | **~400+** | **~92K** | |

---

## Pass 24: Fix Test Compilation (URGENT — Blocks Verification)

### Scope
8 crates cannot compile for tests, blocking 1,169 test annotations. This is the single easiest high-impact fix.

### Root causes and fixes
1. **Feature gate cascade** — `roko-learn/Cargo.toml` imports `roko-golem` without `features = ["scaffold"]`. Fix: add the feature flag, or make the `AffectEngine` import conditional with `#[cfg(feature = "golem")]`.
2. **API rename** — `roko-agent/tests/safety_integration.rs` calls `with_safety_policy()` (renamed to `with_safety()`). Fix: update the test call.
3. **notify crate API** — `roko-plugin` uses removed `CreateKind`/`ModifyKind`/`RemoveKind`. Fix: update to current notify API.
4. **Lifetime error** — `roko-mcp-slack` test line 878. Fix: bind temporary to a variable.
5. **Stale assertions** — 16 tests with wrong expected values after refactors. Fix: update expected strings.

### Acceptance criteria
- [ ] `cargo test --workspace` compiles all 27 crates
- [ ] All previously-passing tests still pass
- [ ] Total test count ≥ 1,665 (current passing) + recovered from 8 broken crates

---

## Pass 25: Enrichment Extraction Quality

### Scope
Upgrade the 7 non-LLM enrichment extraction functions from keyword greps to structured parsers matching mori's quality.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/support_enrich/mod.rs` lines 907-1556 — read every extraction function: `extract_prd_refs()`, `extract_brief()`, `extract_tasks()`, `generate_research()`, `generate_dependency_manifest()`, `generate_fixture_manifest()`, `generate_integration_md()`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/memory.rs` lines 1490-1770 — `render_plan_learning_md()`, `render_plan_research_md()` (called by generate_research)

### Roko files to modify
- `crates/roko-compose/src/enrichment/prompts.rs` — rewrite all 7 extraction functions
- `crates/roko-compose/src/enrichment/inputs.rs` — extend `StepInputs` to carry `PathBuf` for repo_root and plan_dir (or pass pre-computed structured data from the pipeline)
- `crates/roko-compose/src/enrichment/pipeline.rs` — populate richer inputs from filesystem before calling extraction

### Key upgrades per step
1. **PRD**: Parse backtick paths, normalize `prd2/` → `prd/`, deduplicate
2. **Brief**: Target Prerequisites/Imports/Exports sections specifically, add verification checklist
3. **Tasks**: Filter 7 non-implementation heading patterns, extract file paths from backtick segments, compute estimated_seconds
4. **Research**: Load episode history and playbook from filesystem, render plan-scoped learning markdown
5. **Dependencies**: Parse tasks.toml into typed struct, scan sibling plans for cross-plan overlap, detect infrastructure keywords, emit typed DependencySpec
6. **Fixtures**: Parse upstream dependency manifest, detect infrastructure services, emit specs with commands and healthchecks
7. **Integration**: Synthesize dependency + fixture manifests, detect surfaces from task metadata, emit suggested test commands

### Acceptance criteria
- [ ] `extract_prd` produces structured reference list with normalized paths
- [ ] `extract_brief` targets specific sections (Prerequisites/Imports/Exports), not all headings
- [ ] `extract_tasks` filters non-implementation headings and extracts file paths from backticks
- [ ] `generate_research` includes plan-scoped episode history and playbook rule matches
- [ ] `generate_dependency_manifest` emits typed DependencySpec with cross-plan scanning
- [ ] `generate_fixture_manifest` emits actionable specs with build commands and healthchecks
- [ ] `generate_integration` synthesizes manifests into surfaces and suggested test commands

---

## Pass 26: Wire Format Bandit End-to-End

### Scope
The format bandit has 10 format families, 3 bandit implementations, per-model profiles, and Galileo scoring — all thoroughly tested. But the selected format never reaches the agent and feedback is never recorded. Wire the three disconnection points.

### Roko files to modify
- `crates/roko-agent/src/claude_cli_agent.rs` — add `with_format(ToolFormat)` builder method; if format != AnthropicBlocks, use the appropriate translator
- `crates/roko-agent/src/exec.rs` — add `with_format(ToolFormat)` to ExecAgent
- `crates/roko-agent/src/translate/capability.rs` — make `translator_for()` bandit-aware (accept optional override format)
- `crates/roko-cli/src/orchestrate.rs` — pass `selected_format` into agent dispatch; after agent completion, call `format_bandit.feedback()` with `ToolOutcome` constructed from `AgentResult`
- `crates/roko-cli/src/config.rs` — add config option to select bandit implementation (profile/epsilon-greedy/track-and-stop)

### Acceptance criteria
- [ ] `selected_format` from bandit is passed to agent construction
- [ ] Agent uses the selected translator based on format (not hardcoded per backend)
- [ ] `format_bandit.feedback()` called after every agent run with success/failure/latency/cost
- [ ] After 50+ runs, bandit arm statistics show non-uniform selection (learning is happening)
- [ ] Config allows switching between ProfileBandit, EpsilonGreedy, and TrackAndStop

---

## Pass 27: Wire DAG Task-Level Execution

### Scope
The `UnifiedTaskDag` computes correct task-level waves but the executor doesn't consume them. Wire the DAG into the execution loop so task parallelism and wave-based grouping actually work.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/unified_dag.rs` (1,216 LOC) — `next_runnable()` with file-conflict runtime check, `__whole__` nodes, skipped propagation, `independent_groups()`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/dag.rs` (780 LOC) — plan-level wave computation with `compute_waves()`, `critical_path()`, `file_overlap_analysis()`

### Roko files to modify
- `crates/roko-orchestrator/src/executor/mod.rs` — add task-level dispatch: instead of `SpawnAgent { task: "next" }`, use `UnifiedTaskDag::next_runnable()` to find ready tasks; enforce `max_concurrent_tasks`
- `crates/roko-orchestrator/src/dag.rs` — add `next_runnable()` with runtime file-conflict checking; add `__whole__` synthetic nodes for plans without tasks; add skipped task propagation
- `crates/roko-cli/src/orchestrate.rs` — handle task-specific dispatch (translate `GlobalTaskId` to actual task context)

### Acceptance criteria
- [ ] Executor dispatches individual tasks (not just "next" per plan)
- [ ] `max_concurrent_tasks` enforced across all plans
- [ ] File-overlap conflicts checked dynamically at dispatch time
- [ ] Tasks in the same wave run in parallel (up to max_concurrent_tasks)
- [ ] Skipped tasks propagate resolution to dependents

---

## FINAL complete summary (all 27 passes)

| # | Pass | Gaps | Est. LOC | Priority |
|---|---|---|---|---|
| 1 | TUI | 68 | ~15K | P1 |
| 2 | Gates infrastructure | 17 | ~2K | P0 |
| 3 | Orchestrator (task scheduling) | 30+ | ~5K | P0 |
| 4 | Cognitive (grimoire/daimon/dreams) | 40+ | ~15K | P2 |
| 5 | Conductor (LLM + actions) | 15 | ~3K | P1 |
| 6 | Agent (backends + pool) | 15 | ~5K | P1 |
| 7 | Code Intelligence (MCP + index) | 8 | ~8K | P0 |
| 8 | Prompt Composition (structure) | 36 | ~6K | P0 |
| 9 | Live Monitoring | 12 | ~2K | P1 |
| 10 | Configuration | 13 | ~1K | P1 |
| 11 | Runtime Loop (async) | 28 | ~5K | P0 |
| 12 | Git Operations | 28 | ~4K | P0 |
| 13 | Safety Hardening | 8 | ~2K | P1 |
| 14 | Task Metadata | 6 | ~2K | P1 |
| 15 | Learning Feedback (read-back) | 8 | ~3K | P0 |
| 16 | Conductor Actions + Watchers | 12 | ~2K | P0 |
| 17 | Observability | 5 | ~2K | P0 |
| 18 | Prompt Text Depth | 21 | ~2K | P0 |
| 19 | Dispatch Gates + Express Mode | 5 | ~2K | P1 |
| 20 | Wire Dead Signal Features | 7 | ~1K | P1 |
| 21 | Wire Dead Budget + Cost | 7 | ~2K | P0 |
| 22 | Wire AGENTS.md + Task Enums | 6 | ~2K | P1 |
| 23 | Edge Case Hardening | 6 | ~1K | P0 |
| **24** | **Fix Test Compilation** | **5** | **~0.5K** | **P0 URGENT** |
| **25** | **Enrichment Extraction Quality** | **7** | **~3K** | **P0** |
| **26** | **Wire Format Bandit** | **5** | **~1K** | **P1** |
| **27** | **Wire DAG Task-Level Execution** | **7** | **~2K** | **P0** |
| | **TOTAL** | **~430+** | **~98K** | |

---

## Pass 28: Human-in-the-Loop Controls (TUI Actions)

### Scope
Mori's TUI provides keyboard-driven orchestrator control (2,483 LOC). Roko's dashboard is read-only. This pass adds interactive controls that let operators manage long-running plan executions without killing the process.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/tui_actions.rs` (2,483 LOC) — read ALL. Key action handlers:
  - `TogglePause` — pauses/resumes the execution loop
  - `RestartPhase` — kills active agent, re-dispatches with iteration-aware prompts
  - `ForceAdvance` — force-commits current state, advances to next plan
  - `ResetPlanState` — deletes git tag, removes events/context, resets to Pending
  - `StartInject` / `SubmitInject` — message injection into running agents
  - `ApproveCommand` / `RejectCommand` — tool-call approval gates
  - `GitReconcile` — commit/merge/prune from keyboard
  - `MergeBatchToMain` / `MergeSelectedPlan` — merge controls
  - `OpenTaskPicker` / `TaskPickerConfirm` — task re-ingestion
  - Config cycling — model/effort/context changes with agent respawn

### Roko files to modify
- `crates/roko-cli/src/tui/app.rs` — add `TuiAction` enum with execution-affecting variants
- `crates/roko-cli/src/tui/input.rs` (NEW) — key dispatch to TuiAction
- `crates/roko-cli/src/orchestrate.rs` — add `TuiActionReceiver` channel; handle pause/restart/force-advance/inject in the main loop; add `PipelineRunState::Paused` support

### Acceptance criteria
- [ ] `p` key pauses/resumes plan execution
- [ ] `Ctrl-R` restarts current phase (kills agent, re-dispatches)
- [ ] `Ctrl-X` force-advances stuck plan
- [ ] `Ctrl-I` opens inject modal for steering messages
- [ ] `y`/`n` approve/reject agent tool calls (when approval mode enabled)
- [ ] Config changes from TUI take effect without restart

---

## Pass 29: Structured Auto-Fix + Fixture Lifecycle

### Scope
Two mori subsystems with no roko equivalent: structured error classification for auto-fix routing, and fixture lifecycle management for test dependencies.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/autofix.rs` (915 LOC) — read ALL:
  - `CompileErrorClass` (5 variants), `parse_cargo_json_errors()`, `collect_rustc_suggestions()`, `apply_rustc_fixes()`, `generate_compile_fix_plan()`
  - `InvariantFailureClass` (3 variants), `classify_invariant_failures()`, `write_spec_issue()`, `generate_issue_plan()`
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/fixture_lifecycle.rs` (345 LOC) — read ALL:
  - `FixtureSpec`, `FixtureManager` (start/healthcheck/stop), `generate_mock_config()`, `load_fixture_manifest()`

### Roko files to modify
- `crates/roko-gate/src/autofix.rs` (NEW) — port `CompileErrorClass`, `parse_cargo_json_errors()`, `apply_rustc_fixes()`. Route simple errors to `cargo fix`, complex to agent.
- `crates/roko-gate/src/invariant.rs` (NEW) — port `InvariantFailureClass`, spec-aware test routing
- `crates/roko-orchestrator/src/fixture.rs` (NEW) — port `FixtureManager` with start/healthcheck-poll/stop lifecycle
- `crates/roko-cli/src/orchestrate.rs` — wire auto-fix classification into `handle_autofix()` (simple → `cargo fix` first, complex → agent); wire fixture lifecycle into task dispatch

### Acceptance criteria
- [ ] Cargo JSON diagnostics parsed into typed `CompileErrorClass`
- [ ] `ImportNotFound` and `MissingField` errors auto-fixed by `cargo fix` WITHOUT agent
- [ ] Complex errors generate structured fix-plan TOML for agent
- [ ] Invariant failures classified as CodeBug/SpecIssue/MissingTest
- [ ] SpecIssue routed to human review queue, not agent fix loop
- [ ] `FixtureManager` starts/stops fixtures from `fixture-manifest.toml`
- [ ] Healthcheck polling with configurable timeout
- [ ] Reusable fixtures shared across tasks in same plan

---

## Inter-Pass Dependency Graph

```
Phase 0 (do first, no deps):
  Pass 24 (Fix Tests)

Phase 1 (foundational, no deps):
  Pass 11 (Runtime Loop) ← most things depend on this
  Pass 15 (Learning Feedback)
  Pass 17 (Observability)
  Pass 10 (Configuration)
  Pass 14 (Task Metadata)
  Pass 12 (Git Operations)
  Pass 2  (Gates Infrastructure)
  Pass 7  (Code Intelligence)
  Pass 3  (Orchestrator)

Phase 2 (depends on Phase 1):
  Pass 16 (Conductor Actions) ← needs 11
  Pass 6  (Agent) ← needs 10, 11
  Pass 8  (Prompt Composition) ← needs 7, 14, 15
  Pass 19 (Dispatch + Express) ← needs 2, 10
  Pass 25 (Enrichment Quality) ← needs 15
  Pass 27 (DAG Task Execution) ← needs 3
  Pass 29 (Auto-Fix + Fixtures)

Phase 3 (depends on Phase 2):
  Pass 1  (TUI) ← needs 11, 17
  Pass 5  (Conductor LLM) ← needs 11, 16
  Pass 9  (Live Monitoring) ← needs 11, 16
  Pass 18 (Prompt Text Depth) ← needs 8
  Pass 21 (Budget + Cost) ← needs 6, 10
  Pass 22 (AGENTS.md + Enums) ← needs 8, 14
  Pass 26 (Format Bandit) ← needs 6, 11
  Pass 28 (Human Controls) ← needs 1, 11

Phase 4 (standalone, any time):
  Pass 4  (Cognitive) — P2
  Pass 13 (Safety)
  Pass 20 (Signal Features)
  Pass 23 (Edge Cases)
```

Critical path: `11 → 16 → 5` (3 hops) or `11 → 6 → 21` (3 hops)

---

## FINAL summary (all 29 passes)

| Metric | Count |
|---|---|
| **Total gap items** | **450+** |
| **Depth passes** | **29** |
| **Estimated LOC to full parity** | **~102K** |
| **P0 passes** | 15 |
| **P1 passes** | 12 |
| **P2 passes** | 2 |
| **Execution phases** | 5 (0-4) |
| **Critical path depth** | 3 hops |
| **Un-audited mori files remaining** | ~45 of 158 (mostly platform/, server/, state/ internals) |

---

## Pass 30: Structured Review Parsing + Review Pipeline

### Scope
The implement→review→fix convergence loop cannot work without structured review parsing. Currently the agent's review output is opaque text — the orchestrator can't tell "approved" from "needs revision" or route to quick-fix vs full re-implementation.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/review.rs` (502 LOC) — read ALL:
  - `StructuredReview`, `ReviewVerdict` (code/docs/overall × Approve/Revise/Skip), `ReviewIssue` (7 categories, 3 severities)
  - `parse_structured_review()` — 3-strategy parse (direct JSON, fenced JSON, fenced TOML)
  - `is_quick_fixable()` / `all_issues_quick_fixable()` — routing logic
  - `REVIEW_JSON_SCHEMA` — for `--json-schema` structured output
  - `REVIEW_TOML_TEMPLATE` — format instructions for prompts
- `/Users/will/dev/uniswap/bardo/apps/mori/src/state/mod.rs` — review pipeline HashMaps: `plan_pending_reviews`, `plan_review_stage` (5-stage enum), `plan_doc_revisions`, `plan_code_revisions`

### Roko files to modify
- `crates/roko-compose/src/review.rs` (NEW) — port `StructuredReview`, `ReviewVerdict`, `ReviewIssue`, `parse_structured_review()`, `is_quick_fixable()`
- `crates/roko-compose/src/templates/reviewer.rs` — inject `REVIEW_JSON_SCHEMA` as `--json-schema` parameter
- `crates/roko-orchestrator/src/executor/plan_state.rs` — add review pipeline tracking fields
- `crates/roko-cli/src/orchestrate.rs` — parse review output, route to quick-fix/re-implementation/doc-revision based on verdict

### Acceptance criteria
- [ ] `parse_structured_review()` parses JSON and TOML review output
- [ ] Reviewers emit structured JSON via `--json-schema`
- [ ] Quick-fixable issues (Compilation/Docs/Style) route to quick-fix path
- [ ] Non-quick-fixable issues (MissingImpl/SpecDeviation) route to full re-implementation
- [ ] code=Approve + docs=Revise routes to DocRevision only (no code changes)
- [ ] Review cap (3 iterations) triggers force-advance

---

## Pass 31: Iteration Memory + Prompt Logging

### Scope
Two missing persistence layers that directly improve agent quality on retries and debugging.

### Reference files (MUST-READ)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/iteration_memory.rs` (276 LOC) — `IterationMemory`, `IterationEntry` (gate_results + diagnosis + files_changed), `format_reflections_md()` with smart compression (last 3 full, older → 180 chars), `has_error_pattern()` dedup
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompt_log.rs` (221 LOC) — `PromptLogEntry`, `LogSection`, `count_tokens()` (cl100k), `split_sections()`, `log_prompt()` fire-and-forget

### Roko files to modify
- `crates/roko-learn/src/iteration_memory.rs` (NEW) — port `IterationMemory` with JSON persistence at `.roko/learn/iteration-memory/{plan_id}.json`
- `crates/roko-learn/src/prompt_log.rs` (NEW) — port `PromptLogEntry` with per-invocation JSON at `.roko/learn/prompt-logs/`
- `crates/roko-cli/src/orchestrate.rs` — record iteration memory after each gate failure; inject `format_reflections_md()` into retry prompt; call `log_prompt()` after every agent dispatch

### Acceptance criteria
- [ ] Gate failure records `IterationEntry` with gate results + reflection diagnosis
- [ ] Retry prompt includes last 3 reflections (full) + older compressed to 180 chars
- [ ] `has_error_pattern()` dedup prevents duplicate reflections for same error
- [ ] Prompt log captures full text + per-section token breakdown at `.roko/learn/prompt-logs/`
- [ ] Prompt log includes context-packing metadata (cache_hit, playbook_hits)

---

## 5 Quick Wins (Zero Dependencies, Under 50 Lines Each)

These can be done IMMEDIATELY, before any depth pass, with no architectural changes:

### QW-1: Enable roko-golem scaffold feature (1 line)
**File**: `crates/roko-learn/Cargo.toml`
**Change**: `roko-golem = { path = "../roko-golem" }` → `roko-golem = { path = "../roko-golem", features = ["scaffold"] }`
**Impact**: Unblocks test compilation for 5 crates (roko-learn, roko-cli, roko-conductor, roko-serve, roko-neuro) = 1,169 test annotations

### QW-2: Populate MatchContext files/tags (~8 lines)
**File**: `crates/roko-cli/src/orchestrate.rs` line 4117
**Change**: See Pass 15 exact code change (replace `Vec::new()` with `task_def.files` and role+tier tags)
**Impact**: Playbook rules with file/tag triggers start firing. The learning→prompt feedback loop starts working.

### QW-3: Add sync_all() to FileSubstrate::put() (1 line)
**File**: `crates/roko-fs/src/file_substrate.rs` line 208
**Change**: Add `file.sync_all().await?;` after `file.flush().await?;`
**Impact**: Signals are durable on power loss (currently only flushed to OS page cache)

### QW-4: Fix roko-plugin notify imports (~7 lines)
**File**: `crates/roko-plugin/src/lib.rs`
**Change**: Update `notify::event::{CreateKind, ModifyKind, RemoveKind}` imports to current notify API
**Impact**: Unblocks roko-plugin test compilation (12 tests)

### QW-5: Fix safety_integration API rename (~35 lines)
**File**: `crates/roko-agent/tests/safety_integration.rs`
**Change**: Replace `with_safety_policy(SafetyPolicy{...})` with `with_safety(SafetyLayer::new(...))` across 7 construction sites
**Impact**: Unblocks roko-agent test compilation (638 tests)

---

## Codex Batch 4 Audit Results (2026-04-10)

> 83 commits on `roko-batch4-cognitive` (4C.03–5F.23). Branch: `roko-batch4-cognitive`.
> Workspace compiles clean (only doc warnings). Zero `TODO`/`todo!()`/`unimplemented!()`.
> Full audit details with line numbers and code references: `tmp/MORI-PARITY-GAP-ANALYSIS.md` §47.

### Commit series covered

| Series | Commits | Area | Depth pass overlap |
|---|---|---|---|
| 4C.03–4C.14 | 12 | Deploy/Cloud (railway, fly, docker, cloud exec, webhooks) | None (new area) |
| 5B.01–5B.09 | 9 | Context Assembly (5-stage pipeline: gather→rank→compress→inject→validate) | **Pass 4** §Grimoire retrieval, **Pass 8** §Prompt composition |
| 5C.02–5C.14 | 13 | Daimon/AffectEngine (PAD model, appraisal, modulation, persistence) | **Pass 4** §Daimon |
| 5D.02–5D.17 | 16 | Dreams/DreamCycle (offline learning, G2-G8, regression detection) | **Pass 4** §Dreams |
| 5E.01–5E.10 | 10 | Operating Frequencies (3-speed cognition: Gamma/Theta/Delta) | No existing pass |
| 5F.01–5F.23 | 23 | C-Factor (11 sub-metrics, cascade router, dashboard, neuro/daimon/dreams APIs, anti-knowledge) | **Pass 15** §Learning feedback |

### Per-pass impact analysis

#### Pass 4 (Cognitive Layer) — PARTIALLY COVERED, DEPTH STILL NEEDED

**What batch4 built:**
- `KnowledgeStore`: JSONL-backed store with `query()` (keyword + recency decay + confirmation boost scoring), `ingest()`, `decay()`, `gc()`. Wired for reads in orchestrate.rs:4269-4290. See gap analysis §47.4.
- `DaimonState`: PAD model with 8 event handlers (GateResult, TaskOutcome, Blocked, TimePressure, QueueWait, DreamFailure), `modulate()` for dispatch params (5 strategies: Escalating/Exploratory/Conservative/Proactive/Balanced), persistence to `.roko/daimon/affect.json`. See §47.5.
- `DreamRunner`: consolidation cycle via `DreamCycle::run()`, pattern mining via NeuroTierProgression, `schedule()` for auto-dream timing. See §47.6.
- `AntiKnowledge`: confidence reduction (×0.5) on ingestion. See §47.7.

**What batch4 did NOT build (Pass 4 depth work remains):**

| Missing Algorithm | PRD Reference | Mori Reference | Notes |
|---|---|---|---|
| **Ebbinghaus decay curves** | prd/04-memory/01-grimoire.md | golem-grimoire/src/decay.rs (931 LOC) | Batch4 has `recency_factor = 0.5^(age/half_life)` but NOT the full 4-class decay (Volatile=24h, Standard=30d, Resilient=365d, Permanent=∞) with `R = e^(-t/S)` and floor 0.05 |
| **A-MAC admission gate** | prd/04-memory/01-grimoire.md §Admission | golem-grimoire/src/admission.rs (540 LOC) | No admission scoring. `ingest()` accepts all entries. Missing: `score = 0.30×accuracy + 0.20×memorability + 0.25×actionability + 0.15×consistency + 0.10×non_redundancy`, threshold 0.45 |
| **Causal graph** | prd/04-memory/01-grimoire.md §Causal | golem-grimoire/src/causal.rs (1,229 LOC) | No causal graph with typed directed edges. Missing: `CausalLink`, `CausalGraph`, edge types (causes, prevents, suggests, contradicts) |
| **Curator cycle** | prd/04-memory/01-grimoire.md §Curator | golem-grimoire/src/curator.rs (907 LOC) | `decay()` and `gc()` exist but are NEVER called. No 50-tick maintenance cycle (validate/prune/compress/cross-reference) |
| **Memetic fitness** | prd/04-memory/01b-grimoire-memetic.md | golem-grimoire/src/memetic.rs (550 LOC) | No replicator dynamics (`dx_i/dt = x_i × (W_i - W_bar)`), no Price equation, no epistemic parasite detection |
| **Hierarchical RAPTOR retrieval** | prd/04-memory/01-grimoire.md §Retrieval | golem-grimoire/src/hierarchical.rs (1,702 LOC) | No two-level retrieval. Query uses flat keyword scoring only |
| **OCC/Scherer 8-step appraisal** | prd/03-daimon/ §Appraisal | golem-daimon/src/appraisal.rs (1,691 LOC) | Batch4 has simple event → delta table. Missing: novelty check, intrinsic pleasantness, goal significance, coping potential, norm compatibility, self-ideal compatibility, power/control, adjustment |
| **Somatic landscape (k-d tree)** | prd/03-daimon/ §Somatic | golem-daimon/src/somatic.rs (612 LOC) | No k-d tree over strategy space. Missing: behavioral bias lookup from somatic markers (Damasio 1994) |
| **ALMA three-layer EMA** | Gebhard 2005 | golem-daimon/src/alma.rs (377 LOC) | No emotion→mood→personality layering. Batch4 has single-layer PAD with uniform decay |
| **Learned helplessness** | Seligman 1972 | golem-daimon/src/memory.rs:769 | No detection of D < -0.3 for 200+ ticks |
| **Mood-congruent retrieval** | Bower 1981 | golem-daimon/src/memory.rs:769 | No 5-30% retrieval boost based on mood-content congruence |
| **Cross-agent affect contagion** | prd/03-daimon/ §Contagion | golem-daimon/src/contagion.rs (302 LOC) | No cross-agent PAD transfer |
| **Mattar-Daw replay** | Mattar & Daw 2018 | golem-dreams/src/evolution/ | No utility-weighted replay (`gain × need × (1 - 0.5×spacing_penalty)`). Batch4 uses flat clustering |
| **Pearl SCM counterfactuals** | Pearl 2000 | N/A | 5D.13 has "HDC vector permutation" but NOT structural causal models |
| **Memory triage** | Stickgold & Walker 2013 | N/A | No preserve/abstract/forget classification. No staging buffer |
| **Dream budget allocation** | prd/05-dreams/ §Budget | N/A | No phase-dependent budgets (thriving: 34% NREM, 30% REM; terminal: 16% NREM, 57% consolidation) |
| **Threat simulation** | Revonsuo 2000 (G5) | N/A | G5 was SKIPPED entirely in batch4 |

#### Pass 8 (Prompt Composition) — PARTIALLY COVERED

Batch4's `ContextAssembler` (5B.01–5B.09) implements a 5-stage pipeline but needs verification against:
- Active inference scoring formula from 12a §E2
- Liu et al. U-shape attention curve (is reordering actually implemented or just stubbed?)
- Token estimation accuracy (`content.len() / 4` is rough)

#### Pass 15 (Learning Feedback) — PARTIALLY COVERED

C-Factor computation is solid but has 17 specific gaps documented in §47.10. The most impactful:
- `detect_cfactor_regression()` defined but never called (5F.08 claim is false — regression alerting is NOT wired)
- `knowledge_integration_rate` and `convergence_velocity` always return 0.0 (data producers missing)
- `social_sensitivity` returns 0.0 unless tasks have `depends_on` dependencies
- Anti-knowledge ID format mismatch means confidence halving never actually fires
- Weight sum bug: maximum `overall` is 0.96, not 1.0

### Batch4 gaps requiring depth passes — Complete List

| # | Gap | Pass | Location | What's needed |
|---|---|---|---|---|
| 47a | J3 social sensitivity always 0.0 | 15 | `runtime_feedback.rs:1068` reads `context-attribution.jsonl` | Wire attribution records for ALL upstream context references in orchestrate.rs, not just `depends_on` items |
| 47b | J4/J6 knowledge integration + convergence always 0.0 | 4 | `runtime_feedback.rs:1131,1195` reads `KnowledgeConfirmationRecord` | Wire `KnowledgeStore::ingest()` in orchestrate.rs after task completion, or auto-trigger dream |
| 47d | J10 metrics endpoint missing | 6 | `roko-serve/src/routes/learning.rs` | Add `/api/metrics/c_factor` route reading `c-factor.jsonl` |
| 47e | `detect_cfactor_regression()` never called | 15 | `cfactor.rs:455` — no callers | Call from `refresh_cfactor_snapshot()`, emit signal on regression |
| 47f | C-Factor stale-by-one-run | 15 | `orchestrate.rs:6691` reads stale snapshot | Refresh periodically during run, not just at completion |
| 47g | FleetCFactor disconnected from episode CFactor | 15 | `efficiency.rs:561` vs `cfactor.rs:222` | Merge or document; FleetCFactor not persisted or used by router |
| 47h | c-factor.jsonl grows O(N) per run | 15 | `runtime_feedback.rs:637` appends per episode | Append only at run boundaries or compact periodically |
| 47i | Docker deploy missing push | N/A | `main.rs:2991-3004` | Add `docker push` (~3 lines) |
| 47j | AntiKnowledge ID format mismatch | 4 | `cycle.rs:~1463` creates `"insight:{plan}:{type}:{model}"` but distiller uses content-hash IDs | Standardize ID format or add secondary index |
| 47k | AntiKnowledge auto-generation not wired | 4 | No caller in orchestrate.rs | Add post-gate-failure → `ingest(AntiKnowledge)` or auto-dream |
| 47l | `decay()`/`gc()` never scheduled | 4 | `knowledge_store.rs:284,304` | Schedule in PlanRunner::finish() or background task |
| 47m | Daimon only appraises GateResult | 4 | `orchestrate.rs:3110` — sole call site | Add `TaskOutcome`, `Blocked` event calls |
| 47n | DreamRunner not auto-triggered | 4 | `main.rs:2868` — manual only | Wire `schedule()` check in PlanRunner |
| 47o | roko-golem scaffolds dead code | N/A | 6 engines in `roko-golem/src/` | Delete or redirect |
| 47p | Weight sum bug (max overall = 0.96) | 15 | `cfactor.rs:372-383` | Adjust weights to sum to 1.0 |
| 47q | Fly deploy fully hardcoded | N/A | `main.rs:2982-2989` | Read from `config.deploy.fly.*` |

### What this means for depth pass prioritization

**Pass 4 (Cognitive Layer) remains the #1 priority depth pass.** Batch4 built the scaffolding (store, engine, runner) but NOT the academic algorithms. The PRD references (McClelland 1995 CLS, Ebbinghaus forgetting, A-MAC admission, OCC/Scherer appraisal, Mattar-Daw replay, Price equation replicator dynamics) are entirely absent. Pass 4 is ~50% done by volume but ~15% done by depth.

**Pass 15 (Learning Feedback) needs the data pipeline fixes.** The C-Factor computation itself is solid (1,356 lines, 10+ tests) but 3 of 11 sub-metrics are dead (always return 0.0) because nobody writes the data they read. The regression alerting function exists but is never called. These are wiring gaps, not algorithm gaps.

**Passes 8, 6 have minor overlap.** Context assembly (Pass 8) needs depth verification. Agent infrastructure (Pass 6) needs the C-Factor HTTP endpoint.

---

## FINAL FINAL summary (all 31 passes + 5 quick wins + batch4 audit)

| Metric | Count |
|---|---|
| **Total gap items** | **470+** (17 new from batch4 audit) |
| **Depth passes** | **31** |
| **Quick wins** | **5** (52 total lines) |
| **Estimated LOC to full parity** | **~105K** (batch4 contributed ~15K but at surface depth) |
| **Batch4 items marked done** | **~69** across 5B-5F and 4C |
| **Batch4 items with gaps** | **17** (see §47.10 in gap analysis) |
| **P0 passes** | 16 |
| **P1 passes** | 13 |
| **P2 passes** | 2 |
| **Execution phases** | 5 (0-4) |
| **Un-audited mori files remaining** | ~45 of 158 (platform/, state internals, sequential mode — low priority) |
| **Total document lines** | ~3,500 |
| **Critical depth-pass items still needed** | Pass 4 (Grimoire/Daimon/Dreams algorithms), Pass 15 (C-Factor data pipeline fixes) |
