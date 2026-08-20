# Mori-Diffs Audit Gap Extraction

**Generated:** 2026-08-19
**Sources scanned:**
- `tmp/mori-diffs/00-OVERVIEW.md`
- `tmp/mori-diffs/21-FEATURE-PARITY-MATRIX.md`
- `tmp/mori-diffs/23-HANDOFF-OPEN-ITEMS.md`
- `tmp/mori-diffs/24-DEFINITIVE-GAP-LIST.md`
- `tmp/mori-diffs/28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md`
- `tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md`

**Method:** Extracted every concrete unchecked task from the five audit docs,
then cross-referenced against the existing backlog items 01-110 to identify
coverage. Items that are already substantially covered are noted as such.
Items that are not meaningfully covered get a suggested number (starting at 111)
and priority/size estimates.

---

## Legend

- **Existing coverage:** maps to one or more backlog items that cover the gap
- **New:** not covered by any existing backlog item; assigned a suggested number
- **Partial overlap:** a related backlog item exists but does not fully address this gap

---

## Group A: Runtime Convergence / orchestrate.rs Retirement

### A-1: PRD and cloud-worker execution still use legacy PlanRunner
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §2 "Phase C", §7; 24-DEFINITIVE-GAP-LIST.md §8
- **Description:** `prd.rs::run_generated_plans()` and `worker/cloud.rs` both call
  `PlanRunner::from_plans_dir`, which is the old runtime. They are not on the runner-v2
  path. This means PRD auto-plan execution and cloud worker execution have memory leak
  risk (`PlanRunner.efficiency_events: Vec<...>`) and bypass runner-v2 safety and
  learning wiring. Three concrete sub-tasks: port PRD auto-plan to runner-v2, port
  cloud worker to runner-v2, add a CI grep guard blocking new `PlanRunner::from_plans_dir`
  call sites.
- **Existing coverage:** item 103 (Plan Execution Resilience) touches some resilience
  aspects but does not address the PRD/cloud migration. Partial overlap with item 20
  (Event Loop Decomposition) and item 61 (Dispatch Consolidation).
- **Status:** **New** — the specific migration of PRD auto-plan and cloud worker to
  runner-v2 is not explicitly covered.
- **Suggested number:** 111
- **Priority:** P1 — the `PlanRunner` code path has an acknowledged memory leak and
  bypasses runner-v2 safety/learning wiring; the PRD auto-plan trigger is a frequently
  used flow
- **Size:** S (1-2d) — mostly mechanical porting; runner-v2 API already exists

---

### A-2: orchestrate.rs freeze / retirement
- **Source:** 24-DEFINITIVE-GAP-LIST.md §8; 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §2 "Phase D";
  23-HANDOFF-OPEN-ITEMS.md §08; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-01
- **Description:** `orchestrate.rs` is still exported from `lib.rs`, still contains
  unique behavior (replan ledger, knowledge helpers, bandit format), and still used as a
  reference for some helper functions. The plan: (1) confirm no active call sites for
  unique behavior, (2) add a "frozen — do not add code here" banner, (3) progressively
  move reusable helpers into runner-v2 or shared crates, (4) rename/quarantine once empty.
- **Existing coverage:** item 20 (Event Loop Decomposition) is about restructuring
  event_loop.rs, not orchestrate.rs. Item 61 (Dispatch Consolidation) mentions the
  legacy path but focuses on the dispatch layer. Neither covers orchestrate.rs retirement
  as a distinct task.
- **Status:** **New**
- **Suggested number:** 112
- **Priority:** P1 — two runtimes in parallel is the root cause of most architectural
  gaps; a freeze banner and call-site census is a prerequisite for safe retirement
- **Size:** M (2-3d) — requires careful audit of call sites and behavior transfer

---

### A-3: Unified canonical activity/event schema across runner, server, and TUI
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §2 "Phase E"; 29-CURRENT-RUNTIME-GAP-LEDGER.md P2-01
- **Description:** The codebase mixes at least five distinct event/activity terms:
  Signal/Engram, DashboardEvent, RunnerEvent, Episode, and ProjectionEvent. There is no
  single canonical schema that maps between them with documented ownership. Files
  `.roko/signals.jsonl`, `.roko/engrams.jsonl`, and `.roko/events.jsonl` have
  undocumented relationship and migration rules. This causes observer drift: the TUI,
  HTTP serve, and proof scripts can disagree on runtime state because they read from
  different sources.
- **Existing coverage:** item 110 (Deprecate JSONL / StateHub as single source of truth)
  addresses the storage layer. This gap is about the type-level schema and conversion
  contracts, not just the storage backend — they are complementary, not redundant.
- **Status:** **Partial overlap** with item 110, but the schema/terminology problem is
  distinct from the storage problem. Noting here so it can be bundled or tracked separately.
- **Existing coverage verdict:** Partial — item 110 covers storage migration; schema
  normalization is a distinct deliverable. Could be a sub-task of 110 or its own item.
- **Suggested number:** 113 (if tracked separately)
- **Priority:** P2 — blocks observability proof but not execution
- **Size:** S (1d) — define the schema and conversion adapters; does not require moving data

---

## Group B: Provider Dispatch and Model Call Path

### B-1: CascadeRouter feature-vector integration (real routing features, not deterministic default)
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §01; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-02
- **Description:** `ModelRouter` exists but the `CascadeRouter` receives the same
  deterministic default features even when a cascade router is configured. The routing
  context needs to include real signals: role, task category, complexity estimate, affect
  state, and provider health. Without real features, the CascadeRouter cannot improve
  its selection beyond round-robin.
- **Existing coverage:** item 84 (Cascade Router Task Category Awareness) covers the
  task-category dimension. Item 90 (UX34 Override Learning Isolation) covers the
  force_backend bypass. Together they address parts of this gap.
- **Status:** **Partial overlap** — items 84 and 90 together cover most of this, but
  the "real routing features into CascadeRouter context" framing (role, complexity,
  affect, provider health) is broader than either.
- **Existing coverage verdict:** Mostly covered by items 84 + 90. No new item needed;
  track as sub-tasks of those.

---

### B-2: Full provider proof matrix (live credentials through one dispatch path)
- **Source:** 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-03; 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §6;
  21-FEATURE-PARITY-MATRIX.md §PM-04
- **Description:** Provider support for Anthropic, OpenAI, Moonshot, Z.AI, Perplexity,
  Claude CLI, and Codex CLI is claimed but not end-to-end proven through the active runner.
  A proof script is needed that: accepts env vars for each provider, runs a minimal plan
  task, asserts non-empty streamed output, asserts events/episodes/efficiency data contain
  resolved provider/model labels, marks unsupported providers explicitly rather than silent
  fallback.
- **Existing coverage:** item 12 (E2E Test Harness) covers generic e2e testing. Item 61
  (Dispatch Consolidation) covers dispatch unification as a prerequisite.
- **Status:** **New** — a provider-specific proof matrix script/test is not present in
  any existing backlog item.
- **Suggested number:** 114
- **Priority:** P2 — required before claiming multi-provider support; unblocked once
  dispatch is consolidated (item 61)
- **Size:** M (2-3d) — mostly script work once dispatch is unified; requires live keys
  to run fully

---

### B-3: `dangerously_skip_permissions` must be opt-in, not runner default
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §3 "Safety is not universal";
  29-CURRENT-RUNTIME-GAP-LEDGER.md P1-05
- **Description:** `commands/plan.rs` sets `dangerously_skip_permissions: true` in
  `RunConfig`. This bypasses CLI-level sandbox for every plan run by default. The fix:
  make it opt-in via a `--dangerously-skip-permissions` flag or a config key, and emit
  a safety audit record when it is set. This is distinct from backlog item 60, which
  addresses tool allowlists and role contracts.
- **Existing coverage:** item 60 (Safety Dispatch Hardening) addresses the role-tools
  allowlist escalation bug and the optional SafetyLayer. It does not address the
  `dangerously_skip_permissions` default specifically.
- **Status:** **New** — the specific issue of `dangerously_skip_permissions` defaulting
  to `true` in the runner config is not tracked.
- **Suggested number:** 115
- **Priority:** P1 — this is a security posture issue: every plan run bypasses
  CLI sandbox protections by default, even when the user has not consented
- **Size:** XS (1-2h) — change the default, add a flag, emit an audit record

---

## Group C: Prompt Assembly and Context

### C-1: Query playbooks and neuro knowledge during prompt assembly (two-run proof)
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §02; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-04;
  21-FEATURE-PARITY-MATRIX.md `PC-02`
- **Description:** `PromptAssembler` exists and can load knowledge, but the active runner
  path needs a two-run proof: run 1 creates or reinforces knowledge/playbook entries, run 2
  injects them with documented knowledge ids in prompt diagnostics. Without this proof,
  the prompt assembly "wiring" is not confirmed to change behavior.
- **Existing coverage:** item 67 (HDC Prompt Assembly Wiring) is directly related.
- **Status:** **Existing coverage** — item 67 covers this. No new item needed.

---

### C-2: Role-specific tool allowlists enforced in prompt assembly and dispatch
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §02; 21-FEATURE-PARITY-MATRIX.md table row "Tool allowlists by role"
- **Description:** `AssembledPrompt` carries a `tool_allowlist` field and `AgentContract`
  enforces tools via `permits_tool()`. The gap is that `dispatch/prompt_builder.rs`'s
  role-specific allowlist is not verified to reach every provider path (Claude CLI, Codex
  CLI, API tool loops, ExecAgent). A role matrix proof is needed.
- **Existing coverage:** item 60 (Safety Dispatch Hardening) covers the contract
  enforcement gaps. Item 45 (ACP Tool Permission Gate) covers ACP specifically.
- **Status:** **Existing coverage** — items 60 + 45 cover this.

---

### C-3: Code index context injected as structured section, not raw concatenation
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §02
- **Description:** When the code intelligence index is included in prompts, it is
  concatenated as raw text rather than appearing as a typed section with priority, token
  budget accounting, and effectiveness tracking. The fix: introduce a `CodeIndexSection`
  type in `PromptAssembler` that participates in section dropping when the token budget
  is exceeded.
- **Existing coverage:** item 67 (HDC Prompt Assembly Wiring) addresses the HDC
  fingerprint in prompt assembly but not the code-index section type. Item 03 (Context
  Injection Scoping) is about context scoping broadly.
- **Status:** **New** — structured code-index section type is not in any existing item.
- **Suggested number:** 116
- **Priority:** P2 — improves prompt quality and enables token-budget enforcement for
  code context; not blocking
- **Size:** S (1d)

---

### C-4: Snapshot tests for implementer, reviewer, and retry prompt shapes
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §02; 21-FEATURE-PARITY-MATRIX.md §PM-04
- **Description:** No snapshot tests verify that the prompt for the implementer role
  differs from the reviewer role, or that a retry prompt includes structured gate
  feedback. Without these, prompt regression is invisible.
- **Existing coverage:** item 12 (E2E Test Harness) covers integration tests broadly.
  No item specifically targets prompt snapshot tests.
- **Status:** **New**
- **Suggested number:** 117
- **Priority:** P2 — prevents silent prompt regression as prompt assembly evolves
- **Size:** XS (2-4h) — instatest snapshots once PromptAssembler is stable

---

## Group D: Plan Execution, DAG, and Merge

### D-1: Multi-task concurrent execution requires per-plan agent-handle map
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §03; 29-CURRENT-RUNTIME-GAP-LEDGER.md P1-01
- **Description:** `max_concurrent_tasks` cannot be raised above 1 safely until there
  is a per-plan/per-task agent-handle map that prevents double-dispatch and enables
  targeted cancellation. `TaskDag::mark_running` prevents duplicate dispatch at the DAG
  level, but the runner event loop still uses a single `agent_handle` rather than a map.
- **Existing coverage:** item 20 (Event Loop Decomposition) mentions the single handle
  problem. Item 103 (Plan Execution Resilience) does not address concurrent handle
  management.
- **Status:** **New** — the specific handle-map gap for concurrent task execution is
  not tracked as its own item.
- **Suggested number:** 118
- **Priority:** P2 — required for true multi-task parallelism; current single-task
  throughput is functional but limits plan speed
- **Size:** M (2-3d)

---

### D-2: Replan-on-gate-failure not wired in runner-v2
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §3 "Replan-on-gate-failure";
  23-HANDOFF-OPEN-ITEMS.md §08; 29-CURRENT-RUNTIME-GAP-LEDGER.md P1-02
- **Description:** Legacy `orchestrate.rs` has a replan ledger, strategy selection, and
  plan mutation code. Runner-v2 only retries with backoff or marks a fatal terminal state.
  The `NeedsReplan` failure kind exists but produces no replan record and no DAG mutation.
  Required: a `RetryAction::Replan` path that generates a revised task from gate failure
  context, persists a replan ledger, mutates the DAG, and resumes. Needs deduplication
  and max-replans-per-plan caps.
- **Existing coverage:** item 14 (Plan Mutation Protocol) covers the typed `PlanMutation`
  enum for expressing plan changes. Item 04 (Compile Auto-Fix Path) covers the compile
  auto-fix sub-case.
- **Status:** **Partial overlap** — item 14 provides the mutation mechanism; the actual
  replan-on-gate-failure wiring (ledger, DAG mutation, resume, deduplication) is not
  explicitly covered.
- **Suggested number:** 119
- **Priority:** P1 — without replan, gate failures can only be retried N times then
  blocked; Mori-level self-correction requires structural replan capability
- **Size:** M (2-3d) — builds on item 14's mutation protocol

---

### D-3: Merge success/conflict proof via active runner (not auto-success stub)
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §03; 24-DEFINITIVE-GAP-LIST.md GAP-05;
  29-CURRENT-RUNTIME-GAP-LEDGER.md P1-01
- **Description:** `PlanMerger` and `GitMergeBackend` exist. The gap is a reproducible
  proof: (1) non-conflicting merge completes and appears in events/projection, (2)
  conflicting merge produces conflict evidence in events/HTTP state, (3) post-merge
  regression gate failure does not become `MergeSucceeded`. The merge auto-success stub
  path in the original runner must be confirmed retired.
- **Existing coverage:** no existing backlog item specifically covers merge proof.
  Item 103 (Plan Execution Resilience) covers general execution failures.
- **Status:** **New**
- **Suggested number:** 120
- **Priority:** P2 — without this proof, multi-agent plan execution on the same repo
  cannot be trusted; required before enabling concurrent tasks
- **Size:** S (1d) — the machinery exists; this is proof scripting and fixing any
  gaps found during the proof run

---

## Group E: Persistence and Resume

### E-1: Router state and gate thresholds not persisted across crashes
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §04; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-06
- **Description:** `RunStateSnapshot` exists and `run-state.json` is written. However,
  the cascade router state and gate threshold EMA are not included in the crash-safe
  snapshot. A crash between tasks loses router observations and threshold updates that
  occurred during the current run. Fix: include `CascadeRouter` serialized state and
  `AdaptiveThresholds` in `RunStateSnapshot`, restore them on resume.
- **Existing coverage:** item 80 (Learning Subsystem Data Quality) covers data quality
  issues in the learning files but not the crash-persistence gap.
- **Status:** **New**
- **Suggested number:** 121
- **Priority:** P1 — without this, every crash resets learning state for the current
  run; particularly harmful for long multi-task plans
- **Size:** XS (2-4h) — add two fields to RunStateSnapshot and restore them in resume.rs

---

### E-2: `run_id` missing from executor snapshot data
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §04; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-06
- **Description:** `run_id` is present in runtime events but not in the executor
  snapshot. This makes it impossible to correlate a snapshot with the events that
  produced it, or to query events by run id from the snapshot alone. Fix: add `run_id`
  to `RunStateSnapshot` and `ExecutorSnapshot`.
- **Existing coverage:** not covered by any backlog item.
- **Status:** **New**
- **Suggested number:** 122
- **Priority:** P2 — enables run-scoped HTTP queries and correlates snapshots with
  event logs; not blocking execution
- **Size:** XS (1h)

---

### E-3: Crash/resume proof matrix (no duplicate completion, JSONL recovery)
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §04; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-06;
  21-FEATURE-PARITY-MATRIX.md `PARITY-RESUME`
- **Description:** Resume code exists in `runner/resume.rs`. A reproducible crash/resume
  proof is needed covering: (1) crash during agent output (pre-gate), (2) crash post-agent
  pre-gate, (3) crash in-gate, (4) crash post-gate pre-snapshot, (5) stale pid files,
  (6) stale plan ids, (7) JSONL tail corruption. Each scenario must prove no duplicate
  task completion.
- **Existing coverage:** item 97 (Snapshot Backup & Staging Cleanup) covers snapshot
  integrity. Item 103 (Plan Execution Resilience) covers some failure modes.
- **Status:** **Partial overlap** — none of the existing items specifically cover the
  crash/resume proof matrix as a test scenario set.
- **Suggested number:** 123
- **Priority:** P1 — required for reliability claims; without proof, resume is
  untested under real crash conditions
- **Size:** S (1-2d) — proof scripting plus fixing any bugs found

---

## Group F: Learning, Knowledge, and Dreams

### F-1: Per-turn efficiency events (not only per-task summaries)
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §05; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-05
- **Description:** Runner currently emits one efficiency event per task completion.
  Mori emitted per-turn events with per-turn token/cost deltas. Per-turn efficiency
  events enable finer-grained prompt section effectiveness tracking and more accurate
  cost attribution. Fix: emit `AgentEfficiencyEvent` on every `TurnCompleted` event
  from the agent stream.
- **Existing coverage:** item 80 (Learning Subsystem Data Quality) covers data quality
  issues but not per-turn emission frequency. Item 44 (Calibration Feedback Loop)
  covers calibration broadly.
- **Status:** **New**
- **Suggested number:** 124
- **Priority:** P2 — improves learning data quality; not blocking
- **Size:** XS (1-2h)

---

### F-2: Hardcoded backend/role values in runner episode logging
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §05; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-05
- **Description:** Runner episode logging still contains hardcoded `backend = "claude"`
  and similar synthetic values in some paths when the actual dispatch data is not
  available at logging time. Fix: populate provider/model/role from the actual dispatch
  context, failing with a warning rather than substituting a hardcoded string.
- **Existing coverage:** item 80 (Learning Subsystem Data Quality) covers data quality.
- **Status:** **Partial overlap** — item 80 covers data quality but is focused on
  different specific issues (stale .tmp files, empty knowledge receipts, etc.). The
  hardcoded backend string is a distinct specific bug.
- **Suggested number:** 125
- **Priority:** P2 — learning data with wrong provider labels produces misleading
  routing decisions
- **Size:** XS (1h)

---

### F-3: Knowledge write-back from successful runner completions (query proof)
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §05; 29-CURRENT-RUNTIME-GAP-LEDGER.md P1-03;
  21-FEATURE-PARITY-MATRIX.md `KM-02`
- **Description:** `RuntimeKnowledgeLifecycle::ingest_episode` is called from
  `runner/event_loop.rs`. The gap is a proof that `.roko/neuro/knowledge.jsonl` actually
  receives non-empty entries after a successful runner task, and that the HTTP endpoint
  `/api/neuro/query` and CLI `roko knowledge query` return those entries. Also needed:
  prove that a later prompt includes the retrieved entry (with knowledge id in diagnostics).
- **Existing coverage:** no backlog item specifically covers this knowledge write-back
  proof.
- **Status:** **New**
- **Suggested number:** 126
- **Priority:** P2 — required for Mori-level self-improvement; currently the write
  path exists but is unproven end-to-end
- **Size:** S (1d) — proof scripting plus any wiring fixes

---

### F-4: Dream consolidation trigger after plan completion
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §05; 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §4 "Dreams";
  29-CURRENT-RUNTIME-GAP-LEDGER.md P1-03; 21-FEATURE-PARITY-MATRIX.md `DR-01`
- **Description:** `DreamRunner`, `DreamTriggerSink`, and `PlanCompletionTriggerPolicy`
  exist. The active runner has some direct dream code. The gap: (1) define a trigger
  policy (after plan complete, after N episodes, idle timer, or explicit CLI), (2) run
  dream consolidation non-blocking after a successful plan when policy allows, (3) emit
  dream lifecycle events into `.roko/events.jsonl`, (4) prove episodes become
  knowledge/playbook/routing recommendations after a dream run.
- **Existing coverage:** item 83 (Dream Consolidation Deadlock) covers the concurrency
  bug. No item covers the trigger policy and end-to-end dream-to-knowledge proof.
- **Status:** **New**
- **Suggested number:** 127
- **Priority:** P2 — required for Mori-level consolidation; currently dreams run
  only manually
- **Size:** S (1d)

---

### F-5: Daimon affect deltas not persisted; affect not in prompt/episode metadata
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §4 "Daimon/Affect";
  29-CURRENT-RUNTIME-GAP-LEDGER.md P1-03
- **Description:** Runner loads affect state and passes it into CascadeRouter, but:
  (1) affect deltas after task/gate outcomes are not persisted back to `.roko/daimon/affect.json`,
  (2) affect state is not included in prompt assembly (no affect-based section priority),
  (3) affect state is not in episode metadata (cannot correlate learning outcomes with
  affect state at the time).
- **Existing coverage:** item 10 (Daimon TUI View) is about visualization. No item
  covers affect persistence and metadata propagation.
- **Status:** **New**
- **Suggested number:** 128
- **Priority:** P2 — closes the affect feedback loop; currently affect is loaded but
  changes have no effect on future affect state
- **Size:** S (1d)

---

### F-6: Prompt section effectiveness loop not proven in runner-v2
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §4 "Section effectiveness and prompt learning"
- **Description:** Legacy `PlanRunner` builds prompts with `section_effectiveness_snapshot()`
  and records prompt sections into efficiency events. `LearningRuntime::append_efficiency_event`
  updates a `SectionEffectivenessRegistry`. Runner-v2 needs a specific proof that:
  (1) efficiency events include non-empty `prompt_sections`, (2) the registry is read
  before composing the next prompt, (3) a learned section changes priority/tokens in
  a measurable before/after comparison.
- **Existing coverage:** item 44 (Calibration Feedback Loop) covers calibration broadly.
  Item 80 (Learning Subsystem Data Quality) covers data quality.
- **Status:** **Partial overlap** — neither item specifically addresses the prompt
  section effectiveness registry loop in runner-v2.
- **Suggested number:** 129
- **Priority:** P2 — enables prompt quality to improve over time; currently the
  registry may not be fed or consulted correctly by runner-v2
- **Size:** S (1d)

---

## Group G: Observability and Projection

### G-1: `run_id` missing from persisted runtime event payloads
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §06; 29-CURRENT-RUNTIME-GAP-LEDGER.md P0-06
- **Description:** `run_id` is available at runner startup but is not stamped into
  every event written to `.roko/events.jsonl`. Without it, events from multiple runs in
  the same workspace are indistinguishable. Fix: stamp every `RunnerEvent` with `run_id`
  before persistence.
- **Existing coverage:** see E-2 above (run_id in snapshot). These are related but
  distinct: E-2 is about the snapshot, this is about event payloads.
- **Status:** **New** — can be bundled with E-2 (item 122) or tracked separately.
- **Suggested number:** 130
- **Priority:** P2 — blocks run-scoped event queries
- **Size:** XS (1h)

---

### G-2: HTTP endpoints for querying events and gates by run id
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §06; 29-CURRENT-RUNTIME-GAP-LEDGER.md P1-04;
  21-FEATURE-PARITY-MATRIX.md `OB-01`
- **Description:** Events are written to `.roko/events.jsonl` but there is no HTTP
  endpoint to query them by run id, task id, event category, or gate rung. The proof
  standard requires that a proof script can start `roko serve`, run a tiny plan, and
  query events/gates/knowledge through HTTP rather than reading files directly.
- **Existing coverage:** item 105 (HTTP API Design Consistency) covers API design
  broadly. No item specifically covers run-scoped event query endpoints.
- **Status:** **New**
- **Suggested number:** 131
- **Priority:** P2 — required for the "TUI/API/CLI one truth" parity row; proof
  cannot be trusted until HTTP can query the same data the TUI shows
- **Size:** M (2-3d) — requires event indexing by run_id plus REST endpoint(s)

---

### G-3: Tool calls, token/cost updates, gate output, retry decisions not published to projection
- **Source:** 23-HANDOFF-OPEN-ITEMS.md §06; 21-FEATURE-PARITY-MATRIX.md table rows
  "Tool activity visible", "Token/cost live updates", "Gate output visible while running"
- **Description:** The projection layer publishes a subset of runtime events. Missing:
  tool call start/end events, per-turn token/cost delta events, gate output streaming
  (while gate is running), retry decision events (which retry attempt, which backoff).
  These gaps mean the TUI and HTTP cannot show live cost or gate progress.
- **Existing coverage:** item 108 (TUI Live Feedback Gaps) covers TUI feedback gaps.
  Item 109 (TUI Realtime Streaming Parity) covers streaming.
- **Status:** **Existing coverage** — items 108 + 109 together cover the TUI side;
  the projection layer publishing side is not separately tracked. Recommend these
  mori-diffs findings be used as input to items 108/109 scope.

---

### G-4: `signals.jsonl` dead path — decide fate and migrate or remove
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §1 S4; 29-CURRENT-RUNTIME-GAP-LEDGER.md P2-01
- **Description:** `RokoLayout` still exposes `.roko/signals.jsonl`. Canonical signal
  storage is `.roko/engrams.jsonl`. Some serve/status/TUI paths still reference or
  fall back to `signals.jsonl`. Decision needed: deprecated alias (add migration shim)
  or remove. Update all consumers to use canonical path.
- **Existing coverage:** item 110 (Deprecate JSONL / StateHub) partially covers this
  in that signals.jsonl is one of the files to eliminate. But item 110 is about moving
  to StateHub, not about the signals.jsonl vs engrams.jsonl alias confusion.
- **Status:** **Partial overlap** with item 110. Could be a sub-task or predecessor.
- **Suggested number:** 132 (if tracked as its own item before 110 lands)
- **Priority:** P3 — not blocking; the confusion is operational noise
- **Size:** XS (1-2h)

---

## Group H: Adaptive Gate Thresholds

### H-1: Adaptive gate thresholds not loaded/updated/saved in runner-v2
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §3 "Adaptive gate thresholds";
  29-CURRENT-RUNTIME-GAP-LEDGER.md P1-02; 23-HANDOFF-OPEN-ITEMS.md §S7
- **Description:** `AdaptiveThresholds` exists in `roko-gate`. `LearningPaths` reserves
  `.roko/learn/gate-thresholds.json`. Runner-v2 `gate_dispatch.rs` does not load,
  update, or save `AdaptiveThresholds`. The fix: (1) load `gate-thresholds.json` at
  runner startup, (2) call threshold update method on every `GateCompletion`, (3) save
  after update and on shutdown, (4) include threshold value and policy decision in
  `RunnerEvent::GateCompleted`, (5) prove repeated pass/fail outcomes alter the threshold.
- **Existing coverage:** item 40 (Gate Rung Input Completion) is about richer gate
  rung inputs for rungs 3-6. It does not cover adaptive threshold wiring.
- **Status:** **New**
- **Suggested number:** 133
- **Priority:** P1 — without this, gate threshold learning is not wired despite the
  infrastructure existing; self-calibration of the gate pipeline does not happen
- **Size:** S (1d)

---

## Group I: Migration and Parity Proof Harness

### I-1: Feature parity proof harness (prove-feature-parity.sh + generated report)
- **Source:** 21-FEATURE-PARITY-MATRIX.md §PM-02, §PM-03, §PM-04, §PM-05, §PM-06;
  29-CURRENT-RUNTIME-GAP-LEDGER.md §Minimum Proof Standard
- **Description:** A proof harness script at `tests/proof/mori-diffs/prove-feature-parity.sh`
  is needed that generates `tmp/mori-diffs/generated/feature-parity-report.json`. The
  harness should exercise the defined parity scenarios: PARITY-SIMPLE-TASK,
  PARITY-MULTI-TASK-DAG, PARITY-FAILED-PREREQ, PARITY-GATE-RETRY, PARITY-RESUME,
  PARITY-ROUTING-SECOND-RUN, PARITY-KNOWLEDGE-SECOND-RUN, PARITY-PROJECTION-HTTP-TUI,
  PARITY-MERGE-CONFLICT, PARITY-DREAM-TRIGGER, PARITY-PROVIDER-MATRIX. Exit non-zero
  if any P0 scenario fails.
- **Existing coverage:** item 12 (E2E Test Harness) is the closest match.
- **Status:** **Partial overlap** — item 12 is generic E2E; the mori-diffs proof harness
  is specifically the parity acceptance suite with defined scenario names and a generated
  JSON report. Could be a deliverable within item 12 or its own item.
- **Suggested number:** 134
- **Priority:** P2 — the parity claim requires reproducible proof; this is the
  machinery to generate and maintain it
- **Size:** M (2-3d)

---

### I-2: Episode log deduplication — root vs learn episode canonical path decision
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §4 "Episode logging"
- **Description:** Runner-v2 appends to `.roko/episodes.jsonl`. `LearningRuntime` also
  appends to `.roko/learn/episodes.jsonl`. These are two separate episode logs with
  potentially divergent schemas and consumers. A decision is needed: either make one
  canonical (and document the other as derived), or document both with their distinct
  consumers and schemas. Currently, a consumer reading the wrong path gets incomplete data.
- **Existing coverage:** item 80 (Learning Subsystem Data Quality) covers learning data
  quality.
- **Status:** **Partial overlap** — item 80 does not specifically address the episode
  log duplication. This is a concrete schema decision, not a quality issue.
- **Suggested number:** 135
- **Priority:** P3 — operational confusion; not blocking
- **Size:** XS (1-2h) — decision + documentation + migration if collapsing to one log

---

## Group J: Enrichment Artifacts

### J-1: Enrichment artifacts schema and runner-v2 receipt
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §1 "#15 Enrichment artifacts empty"
- **Description:** When enrichment is enabled (not skipped), there is no defined artifact
  schema or canonical path for enrichment output. Runner-v2 does not write a receipt for
  skipped, successful, or failed enrichment. Prompt assembly does not confirm it consumed
  or skipped an enrichment artifact. Fix: define a `EnrichmentReceipt` type, write it
  to `.roko/state/enrichment-<run_id>.json`, and make prompt assembly check for it.
- **Existing coverage:** no existing backlog item covers enrichment artifact schema.
- **Status:** **New**
- **Suggested number:** 136
- **Priority:** P3 — enrichment is currently enabled via `skip_enrichment = false`,
  but the artifact contract does not exist; this is correctness debt
- **Size:** S (1d)

---

## Group K: Legacy Timeout Consolidation

### K-1: Replace scattered 120s provider timeout defaults with a named policy object
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §1 "#9 Enrichment timeout hardcode"
- **Description:** Multiple provider adapters contain `unwrap_or(120_000)` or equivalent
  hardcoded 120-second timeouts: `roko-agent/src/provider/mod.rs`,
  `anthropic_api/tool_loop.rs`, `openai_compat.rs`, `claude_agent.rs`, `codex_agent.rs`,
  `roko-gate/src/integration_gate.rs`. The fix: create a named `TimeoutPolicy` struct
  owned by `RunConfig`/`RuntimeContext`, derive all timeout values from it, and make the
  proof verify a configured timeout value is used end-to-end.
- **Existing coverage:** item 101 (Async Runtime Anti-Patterns) covers broad async
  issues including some timeout patterns.
- **Status:** **Partial overlap** — item 101 covers the anti-pattern class but may not
  specifically target the 120s timeout hardcoding across all providers.
- **Suggested number:** 137 (if not already in item 101 scope)
- **Priority:** P2 — hardcoded timeouts cause silent failures on slow networks and
  make timeout config ineffective
- **Size:** S (1d)

---

## Group L: Safety Audit Events

### L-1: Safety denials must emit durable audit events queryable via HTTP/TUI
- **Source:** 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md §3 "Safety is not universal";
  29-CURRENT-RUNTIME-GAP-LEDGER.md P1-05
- **Description:** When a safety check denies a tool call, path access, or network
  call, no durable audit record is emitted. Without these records, it is impossible to
  verify that safety is functioning or to audit what was denied in a past run. Fix: emit
  a `RunnerEvent::SafetyDenial` (with role, reason, redacted evidence) on every denial,
  include it in `.roko/events.jsonl`, and add an HTTP query endpoint for denials by run id.
- **Existing coverage:** item 60 (Safety Dispatch Hardening) covers tool allowlist
  enforcement. It does not cover the audit event emission or HTTP queryability.
- **Status:** **New**
- **Suggested number:** 138
- **Priority:** P1 — without audit events, safety cannot be verified or debugged in
  production; required before trusting automated plan execution
- **Size:** S (1d)

---

## Summary Table

| # | Title | Source | Existing Coverage | Priority | Size |
|---|---|---|---|---|---|
| New 111 | PRD/cloud-worker migration to runner-v2 | 28, 24 | None | P1 | S |
| New 112 | orchestrate.rs freeze/retirement | 24, 28, 23, 29 | None | P1 | M |
| New 113 | Unified activity/event schema | 28, 29 | Partial (item 110) | P2 | S |
| Covered | CascadeRouter feature-vector integration | 23, 29 | Items 84+90 | — | — |
| New 114 | Full provider proof matrix script | 29, 28, 21 | None | P2 | M |
| New 115 | dangerously_skip_permissions opt-in | 28, 29 | None | P1 | XS |
| Covered | Playbook/knowledge query in prompts | 23, 29, 21 | Item 67 | — | — |
| Covered | Role-specific tool allowlists | 23, 21 | Items 60+45 | — | — |
| New 116 | Code index as structured prompt section | 23 | None | P2 | S |
| New 117 | Prompt snapshot tests (implementer/reviewer/retry) | 23, 21 | None | P2 | XS |
| New 118 | Per-plan agent-handle map for concurrency | 23, 29 | None | P2 | M |
| New 119 | Replan-on-gate-failure in runner-v2 | 28, 23, 29 | Partial (item 14) | P1 | M |
| New 120 | Merge success/conflict proof | 23, 24, 29 | None | P2 | S |
| New 121 | Router + threshold state in crash snapshot | 23, 29 | None | P1 | XS |
| New 122 | run_id in executor snapshot | 23, 29 | None | P2 | XS |
| New 123 | Crash/resume proof matrix | 23, 29, 21 | Partial (97, 103) | P1 | S |
| New 124 | Per-turn efficiency events | 23, 29 | None | P2 | XS |
| New 125 | Hardcoded backend/role in episode logging | 23, 29 | Partial (80) | P2 | XS |
| New 126 | Knowledge write-back proof | 23, 29, 21 | None | P2 | S |
| New 127 | Dream consolidation trigger + proof | 23, 28, 29, 21 | Partial (83) | P2 | S |
| New 128 | Daimon affect delta persistence + metadata | 28, 29 | None | P2 | S |
| New 129 | Prompt section effectiveness loop proof | 28 | Partial (44, 80) | P2 | S |
| New 130 | run_id in event payloads | 23, 29 | None | P2 | XS |
| New 131 | HTTP run-scoped event/gate query endpoints | 23, 29, 21 | None | P2 | M |
| Covered | Tool/cost/gate events to projection | 23, 21 | Items 108+109 | — | — |
| New 132 | signals.jsonl alias decision + migration | 28, 29 | Partial (110) | P3 | XS |
| New 133 | Adaptive gate thresholds in runner-v2 | 28, 29, 23 | None | P1 | S |
| New 134 | Feature parity proof harness | 21, 29 | Partial (12) | P2 | M |
| New 135 | Episode log deduplication decision | 28 | Partial (80) | P3 | XS |
| New 136 | Enrichment artifact schema + receipt | 28 | None | P3 | S |
| New 137 | Timeout policy object (replace 120s hardcodes) | 28 | Partial (101) | P2 | S |
| New 138 | Safety denial audit events + HTTP query | 28, 29 | Partial (60) | P1 | S |

---

## Notes on What Is Already Well-Covered

The following significant mori-diffs concerns are already well-covered by existing
backlog items and do not need new tracking:

- **Dispatch unification / 4 parallel dispatch implementations** → item 61 (Dispatch Consolidation)
- **event_loop.rs god-object decomposition** → item 20 (Event Loop Decomposition)
- **CascadeRouter task category awareness** → item 84
- **force_backend override learning isolation** → item 90
- **Warm agent pool / session reuse** → item 16
- **Playbook/knowledge injection in prompts** → item 67
- **Gate rung 3-6 real inputs** → item 40
- **Safety tool allowlist enforcement** → item 60
- **ACP tool permission gate** → item 45
- **Compile auto-fix path** → item 04
- **Post-gate reflection** → item 15
- **Multi-process locking** → item 37
- **Dream consolidation deadlock** → item 83
- **Learning subsystem data quality (7 issues)** → item 80
- **Plan mutation protocol (PlanMutation enum)** → item 14
- **Provider error UX** → item 38
- **Deprecate JSONL / StateHub as source of truth** → item 110
- **E2E test harness** → item 12
- **Graph engine / runner-v2 parity** → item 54
- **Agent pool runtime integration** → item 55
