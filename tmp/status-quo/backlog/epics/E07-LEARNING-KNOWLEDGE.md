# E07 — Learning & Knowledge Loops

> Executable backlog epic · derived from status-quo pack `40-LEARNING-TELEMETRY.md` + `39-NEURO-KNOWLEDGE.md`
> HEAD `5852c93c05` · schema: `crates/roko-cli/src/task_parser.rs::TaskDef` (`[meta]` + `[[task]]`)
> **Depends on E01** (foundational: build/rustc fix + canonical `.roko/` path helpers).

## Goal

Close the learning and knowledge feedback loops that are currently *write-only, inert, or
reset-on-restart*. roko records a rich stream of learning signals but several loops never feed
back into behaviour: the contextual bandit forgets its weights on every restart, the knowledge
economy only taxes and never credits, HDC is compiled out of every shipped binary, and one of
four knowledge writers bypasses the admission gate. This epic makes the loops *durable and
closed* so the router actually improves across restarts and knowledge balances become live.

## Findings (verified against HEAD `5852c93c05`)

| # | Defect | Where | Impact | Severity |
|---|---|---|---|---|
| a | **LinUCB A/b matrices never persisted.** `CascadeRouter::snapshot()` hardcodes `linucb_state: None` (`cascade_router.rs:1795`); `load_from` discards `linucb_state: _linucb_state` (`:1832`). Only `confidence_stats` + `total_observations` round-trip. | `roko-learn/src/cascade_router.rs`, `cascade/persistence.rs:14-24` (`LinUCBSnapshot` = dead schema), `model_router.rs:426` (`a_matrix`/`b_vector` live on `ArmState`) | Router with `>200` obs stays in **UCB stage** after restart but resets every arm to identity-A / zero-b → contextual bandit is untrained + exploration-dominated on the **main dispatch path**. Silent routing regression on every restart. | **P1 (fix first)** |
| b | **Demurrage taxes, income is dead.** `reinforce()`/`record_usage`/`batch_record_usage` have **zero external prod callers** (tests only). `RuntimeKnowledgeLifecycle` facade referenced by nothing outside the crate. `store.apply_demurrage()` is guarded by `balance > 0.0` which is never true → no-op. | `roko-neuro/src/knowledge_store.rs:1173-1546`, `lifecycle.rs:194-380` | Live entries sit at `balance: 0.0`; economy is decorative. Balance also never enters `score_entry_for_query` (`keyword·confidence·recency·emotional`). Doubly inert: never credited, decay short-circuits, not scored. | **P0 (most dead)** |
| c | **Learning wired to 3 different depths, no parity.** (1) Legacy `orchestrate.rs` = full `LearningRuntime` fan-out (~20 subsystems) + in-router knowledge selection. (2) Runner v2 = thinner `FeedbackFacade` sinks + a **manual** `dispatch_plan.model` nudge (`runner/event_loop.rs:4231-4340`). (3) roko-acp = **write-only**: records observations but never selects, hardcodes `DaimonPolicy::default()` (`bridge_events.rs:634`). | orchestrate.rs / runner/event_loop.rs / roko-acp/bridge_events.rs | Same episode gets different learning treatment per surface. Two divergent knowledge-routing mechanisms. ACP writes learning it never reads. | **P1** |
| d | **HDC compiled out of every binary.** `roko-neuro`'s `hdc` cargo feature enabled by **no** downstream binary (roko-cli/serve/dreams depend bare). Encoder, MemoryIndex, AntiKnowledge repulsion, ResonanceDetector all compiled out. | `roko-neuro/Cargo.toml:15-17`, roko-cli/serve/dreams Cargo.tomls | Knowledge entries have `hdc_vector: null`; HDC-based admission/query/resonance dead. | **P1** |
| e | **4 knowledge writers, one bypasses admission.** `record_lifecycle_knowledge` (INT-20) does a direct `knowledge_store.add` for `AgentLifecycleTransition` events (has a code TODO). | `roko-cli/src/knowledge_helpers.rs:199-270`, `orchestrate.rs:5641-5648` | Lifecycle events skip novelty/trust gating that the other writers face. | **P2** |
| f | **Adaptive gate thresholds persist only at graceful shutdown.** Sole writer = `PlanRunner::shutdown` (`orchestrate.rs:5947-5959`). The declared `gate_thresholds_every_n` cadence (`runtime_feedback.rs:234`) has zero consumers. `gate-thresholds.json` absent on disk. | orchestrate.rs / runtime_feedback.rs | Crash/kill loses all in-memory EMA updates; the loop is half-open. | **P0** |

## Reconciliation with existing plans

| Plan | Scope | Overlap with E07 | Resolution |
|---|---|---|---|
| **P19-cascade-router-acp** | Wires cascade **selection** (not just observation) into the ACP dispatch path (`bridge_events.rs`), loads real `DaimonState`, normalizes slug/arm keys, records the decision in episode metadata. | Directly addresses finding **(c) surface #3 (ACP)**. | **Adopt P19 as-is** for the ACP arm of finding (c). E07-T09 (unify) *depends on P19 landing* and covers only the remaining parity work (unify the two knowledge-routing mechanisms in legacy vs Runner-v2). Do **not** duplicate P19's ACP-selection tasks here. |
| **P26-hdc-similarity-lookup** | Adds `EpisodeLogger::query_similar_episodes` over **episode** `hdc_fingerprint`s (already populated in both write paths) and injects similar-episode context pre-dispatch. | Touches "HDC" but the **episode** fingerprint path — orthogonal to finding **(d)**, which is the **roko-neuro knowledge-store** `hdc` cargo feature being compiled out. | **Keep P26 separate.** E07-T07/T08 enable the *neuro knowledge* HDC feature + backfill knowledge-entry vectors — a different subsystem. Note the naming collision so they are not confused. Episode HDC is *on*; neuro-knowledge HDC is *off*. |

## Task graph

```
E01 (foundational)
 └─> E07-T01 export/import LinUCB ─> E07-T02 wire snapshot/load ─> E07-T03 cross-restart test
 └─> E07-T04 wire knowledge reinforce ─> E07-T05 balance in query score
 └─> E07-T06 route 4th writer through admission
 └─> E07-T07 enable neuro HDC feature ─> E07-T08 backfill knowledge vectors
 └─> E07-T09 unify learning parity (depends: P19 landed)
 └─> E07-T10 incremental gate-threshold flush
```

## Tasks

### E07-T01 — Export/import LinUCB A/b matrices on `LinUCBRouter`
- **tier**: integrative
- **files**: `crates/roko-learn/src/model_router.rs`
- **depends_on**: E01
- **acceptance**: `LinUCBRouter` gains `export_linucb_snapshot() -> LinUCBSnapshot` (flattens each arm's `a_matrix`/`b_vector`, records `dim` + `observations`) and `import_linucb_snapshot(&mut self, snap)` (restores per-arm A/b by slug order, tolerant of arm-count/dim mismatch). Round-trips exactly for a trained router.
- **verify**: `grep -q 'fn export_linucb_snapshot\|fn import_linucb_snapshot' model_router.rs`; `cargo check -p roko-learn`.

### E07-T02 — Populate `CascadeSnapshot.linucb_state` in `snapshot()` and consume in `load_from`
- **tier**: focused
- **files**: `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/cascade/persistence.rs`
- **depends_on**: E07-T01
- **acceptance**: `snapshot()` sets `linucb_state: Some(self.linucb.export_linucb_snapshot())` (removes the hardcoded `None` at `:1795`); `load_from` calls `import_linucb_snapshot` on the destructured state instead of dropping `_linucb_state` (`:1832`). `LinUCBSnapshot` visibility promoted if needed. After a run, `.roko/learn/cascade-router.json` contains a non-null `linucb_state`.
- **verify**: `grep -q 'linucb_state: Some' cascade_router.rs`; `! grep -q 'linucb_state: None' cascade_router.rs`; `cargo check -p roko-learn`.

### E07-T03 — Cross-restart LinUCB persistence test + on-disk assertion
- **tier**: focused
- **files**: `crates/roko-learn/src/cascade_router.rs`
- **depends_on**: E07-T02
- **acceptance**: A `#[test]` trains a router past `CONFIDENCE_TO_UCB_THRESHOLD` (>200 obs), saves, reloads from disk, and asserts a representative arm's `a_matrix != identity` (weights survived) and routing parity across the restart. Documents the runtime check: after `plan run`, `jq '.linucb_state != null' .roko/learn/cascade-router.json` is `true`.
- **verify**: `cargo test -p roko-learn linucb_persist`; `grep -q 'a_matrix' cascade_router.rs`.

### E07-T04 — Wire knowledge reinforcement into the episode-completion path (close demurrage income)
- **tier**: integrative
- **files**: `crates/roko-cli/src/knowledge_helpers.rs`, `crates/roko-cli/src/orchestrate.rs`
- **depends_on**: E01
- **acceptance**: On episode completion, retrieved/gated/cited/quoted knowledge IDs (available at `orchestrate.rs:15575` context-pack IDs) drive `KnowledgeStore::reinforce`/`record_usage` (or `RuntimeKnowledgeLifecycle`) so `balance` is credited. After `roko run "..."`, at least one entry in `.roko/neuro/knowledge.jsonl` has `balance > 0.0`.
- **verify**: `grep -q 'reinforce\|record_usage\|RuntimeKnowledgeLifecycle' crates/roko-cli/src/orchestrate.rs`; runtime: `grep -o '"balance":[0-9.]*' .roko/neuro/knowledge.jsonl | sort -u` shows a value `> 0`; `cargo check -p roko-cli`.

### E07-T05 — Make `balance`/freshness a factor in `score_entry_for_query`
- **tier**: focused
- **files**: `crates/roko-neuro/src/knowledge_store.rs`
- **depends_on**: E07-T04
- **acceptance**: Default query scoring incorporates balance/freshness (either add a bounded balance term to the `keyword·confidence·recency·emotional` product, or switch the default path to `ContextAssemblyWeights::composite`). A unit test shows a reinforced (balance>0) entry outranks a balance-0 entry on equal keywords/confidence.
- **verify**: `cargo test -p roko-neuro score_entry_for_query`; `grep -q 'balance' crates/roko-neuro/src/knowledge_store.rs` inside the scoring fn.

### E07-T06 — Route `record_lifecycle_knowledge` (4th writer) through the admission gate
- **tier**: focused
- **files**: `crates/roko-cli/src/knowledge_helpers.rs`
- **depends_on**: E01
- **acceptance**: `record_lifecycle_knowledge` builds a `KnowledgeCandidateRecord` (with a lifecycle evidence chain + `SourceChannel`) and calls `KnowledgeAdmissionStore::submit_candidate` instead of the direct `knowledge_store.add`. The code TODO is removed.
- **verify**: `! grep -A30 'fn record_lifecycle_knowledge' knowledge_helpers.rs | grep -q 'knowledge_store.add'`; `grep -A30 'fn record_lifecycle_knowledge' knowledge_helpers.rs | grep -q 'submit_candidate'`; `cargo check -p roko-cli`.

### E07-T07 — Enable the `hdc` cargo feature in shipped binaries
- **tier**: focused
- **files**: `crates/roko-cli/Cargo.toml`, `crates/roko-serve/Cargo.toml`
- **depends_on**: E01
- **acceptance**: roko-cli and roko-serve depend on `roko-neuro` with `features = ["hdc"]` (or make it default in `roko-neuro/Cargo.toml`). New knowledge entries written after ingest carry a non-null `hdc_vector`.
- **verify**: `cargo tree -e features -p roko-cli | grep -q 'roko-neuro.*hdc'`; `cargo build -p roko-cli -p roko-serve`.

### E07-T08 — Backfill HDC vectors for existing knowledge entries
- **tier**: focused
- **files**: `crates/roko-cli/src/commands/knowledge.rs` (new `knowledge backfill-hdc` or fold into `gc`)
- **depends_on**: E07-T07
- **acceptance**: A one-shot pass loads `.roko/neuro/knowledge.jsonl`, calls `ensure_hdc_vector` on entries with `hdc_vector: null`, and rewrites. After running, no live entry has `hdc_vector: null`.
- **verify**: runtime: `! grep -q '"hdc_vector": null\|"hdc_vector":null' .roko/neuro/knowledge.jsonl`; `cargo check -p roko-cli`.

### E07-T09 — Unify legacy vs Runner-v2 knowledge routing (parity, post-P19)
- **tier**: integrative
- **files**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/orchestrate.rs`
- **depends_on**: E07-T02; **plan P19-cascade-router-acp** (ACP surface) landed
- **acceptance**: Runner v2's manual `dispatch_plan.model` nudge (`event_loop.rs:4231-4340`) is replaced by a call to the same `select_for_frequency_among_with_knowledge` entrypoint the legacy path uses, so both surfaces share reward/fidelity semantics. ACP parity is delivered by P19; this task only closes the legacy/Runner-v2 divergence.
- **verify**: `grep -q 'select_for_frequency_among_with_knowledge' crates/roko-cli/src/runner/event_loop.rs`; `cargo check -p roko-cli`.

### E07-T10 — Incremental adaptive gate-threshold flush
- **tier**: focused
- **files**: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-learn/src/runtime_feedback.rs`
- **depends_on**: E01
- **acceptance**: Wire the already-declared `gate_thresholds_every_n` cadence (`runtime_feedback.rs:234`) into `record_completed_run`, or add a per-task flush after `observe_pipeline`, so `AdaptiveThresholds` persist without a graceful shutdown. After `plan run` followed by `kill -9`, `.roko/learn/gate-thresholds.json` exists.
- **verify**: `grep -q 'gate_thresholds_every_n\|adaptive_thresholds.save' crates/roko-learn/src/runtime_feedback.rs`; runtime: `test -f .roko/learn/gate-thresholds.json` after an interrupted run; `cargo check -p roko-cli`.

## First 3 tasks (valid native `tasks.toml`)

```toml
[meta]
plan = "E07-learning-knowledge"
total = 3
done = 0
status = "ready"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────────────
# E07-T01: Export/import LinUCB A/b matrices on LinUCBRouter
#
# CascadeRouter::snapshot() hardcodes linucb_state:None (cascade_router.rs:1795)
# and load_from discards _linucb_state (:1832). The LinUCBSnapshot type already
# exists (cascade/persistence.rs:14-24) but is dead schema. Per-arm A/b live on
# ArmState (model_router.rs:426 a_matrix / b_vector). Add export/import so the
# stage-3 contextual bandit survives a restart instead of resetting to identity.
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E07-T01"
title = "Add export_linucb_snapshot/import_linucb_snapshot to LinUCBRouter"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-20250514"
max_loc = 70
files = ["crates/roko-learn/src/model_router.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-learn/src/model_router.rs", lines = "420-465", why = "ArmState with pub a_matrix / b_vector — the parameters to serialize" },
    { path = "crates/roko-learn/src/model_router.rs", lines = "681-720", why = "LinUCBRouter struct + arms field + constructor — where to add methods" },
    { path = "crates/roko-learn/src/cascade/persistence.rs", lines = "9-24", why = "LinUCBSnapshot {a_matrices, b_vectors, dim, observations} — the target type (currently pub(crate), dead)" },
    { path = "crates/roko-learn/src/model_router.rs", lines = "1060-1086", why = "update_features_internal — confirms A += x·xᵀ, b += reward·x shape (dim x dim)" },
]
symbols = [
    "struct LinUCBRouter { arms: Vec<ArmState>, .. } — at model_router.rs:681",
    "struct ArmState { a_matrix: Vec<Vec<f64>>, b_vector: Vec<f64>, slug: String, .. }",
    "LinUCBSnapshot { a_matrices: Vec<Vec<f64>>, b_vectors: Vec<Vec<f64>>, dim: usize, observations: usize } — cascade/persistence.rs:15",
    "CONTEXT_DIM = 18 (model_router.rs:61) — the A/b dimensionality",
]
anti_patterns = [
    "Do NOT change the ArmState field types — read a_matrix/b_vector as-is.",
    "Do NOT panic on arm-count or dim mismatch in import — skip/pad defensively so an old snapshot with a different model set loads cleanly.",
    "Do NOT flatten inconsistently — a_matrices[i] is arm i's row-major dim×dim matrix; keep the same order snapshot() will persist by slug.",
]

# Add two methods on impl LinUCBRouter:
#   pub fn export_linucb_snapshot(&self) -> LinUCBSnapshot
#     - a_matrices: each arm's a_matrix flattened row-major into a Vec<f64>
#     - b_vectors: each arm's b_vector cloned
#     - dim: CONTEXT_DIM, observations: total across arms
#   pub fn import_linucb_snapshot(&mut self, snap: &LinUCBSnapshot)
#     - for each arm by index, if snap has matching entry + dim, un-flatten into
#       a_matrix and copy b_vector; otherwise leave the arm at its constructed default.
# Promote LinUCBSnapshot to pub(crate) reachable from model_router (it already is).

[[task.verify]]
phase = "structural"
command = "grep -q 'fn export_linucb_snapshot' crates/roko-learn/src/model_router.rs && grep -q 'fn import_linucb_snapshot' crates/roko-learn/src/model_router.rs"
fail_msg = "LinUCBRouter must expose export_linucb_snapshot and import_linucb_snapshot"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-learn 2>&1"
fail_msg = "roko-learn must compile after adding LinUCB export/import"

# ─────────────────────────────────────────────────────────────────────────────
# E07-T02: Populate CascadeSnapshot.linucb_state and consume it on load
#
# With export/import available on LinUCBRouter, stop hardcoding None in
# snapshot() (cascade_router.rs:1795) and stop discarding _linucb_state in
# load_from (:1832). This makes .roko/learn/cascade-router.json carry the
# bandit weights so the UCB stage resumes trained after a restart.
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E07-T02"
title = "Persist and restore linucb_state in CascadeRouter snapshot/load_from"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 30
files = [
    "crates/roko-learn/src/cascade_router.rs",
    "crates/roko-learn/src/cascade/persistence.rs",
]
role = "implementer"
depends_on = ["E07-T01"]

[task.context]
read_files = [
    { path = "crates/roko-learn/src/cascade_router.rs", lines = "1757-1810", why = "snapshot()/snapshot_json — linucb_state:None hardcode at :1795 to replace" },
    { path = "crates/roko-learn/src/cascade_router.rs", lines = "1820-1885", why = "load_from — destructures linucb_state:_linucb_state (:1832), discarded today" },
    { path = "crates/roko-learn/src/cascade/persistence.rs", lines = "26-46", why = "CascadeSnapshot.linucb_state: Option<LinUCBSnapshot> — the field to populate" },
]
symbols = [
    "CascadeRouter::snapshot(&self) -> CascadeSnapshot — sets linucb_state",
    "CascadeRouter::load_from(snapshot) — must call import_linucb_snapshot",
    "self.linucb — the LinUCBRouter field inside CascadeRouter (locate exact field name)",
    "LinUCBRouter::export_linucb_snapshot / import_linucb_snapshot — from E07-T01",
]
anti_patterns = [
    "Do NOT break backward compat: linucb_state is #[serde(default)] Option — a None/absent field must still load (fresh bandit).",
    "Do NOT remove total_observations restore — stage recovery still depends on it.",
    "Do NOT import a snapshot whose dim mismatches without going through import_linucb_snapshot's defensive path.",
]

# In snapshot(): replace `linucb_state: None` with
#   `linucb_state: Some(self.<linucb_field>.export_linucb_snapshot())`.
# In load_from(): rename `_linucb_state` to `linucb_state` and, when Some,
#   call `router.<linucb_field>.import_linucb_snapshot(&state)` after arms are built.

[[task.verify]]
phase = "structural"
command = "grep -q 'linucb_state: Some' crates/roko-learn/src/cascade_router.rs"
fail_msg = "snapshot() must populate linucb_state with Some(...)"

[[task.verify]]
phase = "structural"
command = "! grep -q 'linucb_state: None' crates/roko-learn/src/cascade_router.rs"
fail_msg = "the hardcoded linucb_state: None must be removed"

[[task.verify]]
phase = "structural"
command = "grep -q 'import_linucb_snapshot' crates/roko-learn/src/cascade_router.rs"
fail_msg = "load_from must consume the snapshot via import_linucb_snapshot"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-learn 2>&1"
fail_msg = "roko-learn must compile after wiring linucb_state persistence"

# ─────────────────────────────────────────────────────────────────────────────
# E07-T03: Cross-restart LinUCB persistence test
#
# Prove the fix: train a router past the UCB threshold (>200 obs), save to disk,
# reload, and assert the A matrix is no longer identity (weights survived) and
# that routing is stable across the reload. Guards against silent regression.
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E07-T03"
title = "Add cross-restart LinUCB persistence test to cascade_router"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 60
files = ["crates/roko-learn/src/cascade_router.rs"]
role = "implementer"
depends_on = ["E07-T02"]

[task.context]
read_files = [
    { path = "crates/roko-learn/src/cascade_router.rs", lines = "1854-1885", why = "load_or_new / load_from — the disk round-trip entry points under test" },
    { path = "crates/roko-learn/src/cascade/types.rs", lines = "245-250", why = "COLD_START_THRESHOLD=50, CONFIDENCE_TO_UCB_THRESHOLD=200 — obs count needed to reach UCB stage" },
    { path = "crates/roko-learn/src/model_router.rs", lines = "1916-1965", why = "linucb_selects_best_arm_after_training test — pattern for feeding observations" },
    { path = "crates/roko-learn/src/model_router.rs", lines = "2205-2225", why = "identity-A assertion pattern to invert (post-train A must differ from identity)" },
]
symbols = [
    "CascadeRouter::load_or_new(path: &Path, model_slugs: Vec<String>) -> Self",
    "CascadeRouter::save / snapshot — persistence entry point",
    "ArmState.a_matrix — assert != identity after training",
    "CONFIDENCE_TO_UCB_THRESHOLD = 200",
]
anti_patterns = [
    "Do NOT write to the real .roko/ — use tempfile::tempdir for the router path.",
    "Do NOT assert exact float equality on A — compare against identity with an epsilon tolerance.",
    "Do NOT depend on network/LLM — feed synthetic observations directly via the observe/update API.",
]

# Add #[test] linucb_persists_across_restart:
# 1. tempdir; build router with a few model slugs.
# 2. Feed >200 observations (varied context + reward) so it enters UCB stage.
# 3. save() to the tempdir path.
# 4. load_or_new() from the same path (simulated restart).
# 5. Assert reloaded router is in UCB stage AND a representative arm's a_matrix[0][0]
#    differs from the identity value 1.0 by > epsilon (weights survived).
# 6. Assert routing the same context yields the same primary slug pre/post reload.

[[task.verify]]
phase = "structural"
command = "grep -q 'fn linucb_persists_across_restart\\|linucb_persist' crates/roko-learn/src/cascade_router.rs"
fail_msg = "must add a cross-restart LinUCB persistence test"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-learn linucb_persist 2>&1"
fail_msg = "cross-restart LinUCB persistence test must pass"
```

## Runtime verification (epic-level, after tasks land)

- `jq '.linucb_state != null' .roko/learn/cascade-router.json` → `true` after a `plan run` (finding a).
- `grep -o '"balance":[0-9.]*' .roko/neuro/knowledge.jsonl | sort -u` shows values `> 0` (finding b).
- `cargo tree -e features -p roko-cli | grep 'roko-neuro.*hdc'` non-empty; no `"hdc_vector": null` in fresh entries (finding d).
- `test -f .roko/learn/gate-thresholds.json` after an interrupted (`kill -9`) run (finding f).
