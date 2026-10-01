+++
id = "gap-5e818f"
kind = "gap"
title = "Per-task waste (sync ExperimentStore RMW, double dream advice, unread generated-tests/)"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback", "crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::assign_retrieval_strategy_arm"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/spawn_blocking/{b=NR} /ExperimentStore::(settle_attempt|load_or_new|transaction)\\(/{ if (NR-b>6) bad=1 } END{exit bad}' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_task_dispatch/verification.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:39Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:18Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
ExperimentStore does a full synchronous JSON read-modify-write per task on the reactor, dream routing advice loads twice per task, and EvalGenerator writes generated-tests/ files nothing reads (they litter the tree).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste`

How to verify: Check git status for generated-tests/ files after a run.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed (committed at HEAD, commit not pinned): dream advice is loaded once per dispatch and shared with dream_routing_bias (graph_task_dispatch.rs:203-207, :2826). EvalGenerator is opt-in via gates.write_eval_artifacts and writes under .roko/generated-tests/ instead of the tree (:3065-3074), though when enabled nothing in plan run reads the files. Retrieval-arm assignment moved to spawn_blocking (:2779-2782, :2850). Remaining: ExperimentStore::settle_attempt is still called inline in the async dispatch path (:1676). ExperimentStore::load_or_new runs synchronously inside plain tokio::spawn tasks (:2523 in the spawn at :2473, :2649 in the spawn at :2630), not in spawn_blocking, so per-task synchronous JSON store I/O still runs on reactor threads.

Rechecked 2026-09-29 at d9e79e9d8: the dream-advice and EvalGenerator parts stay fixed (one load_dream_routing_advice call per dispatch at graph_task_dispatch.rs:3456; eval artifacts are opt-in and go to .roko/generated-tests/, read by the generated-test gate in runner/gate_dispatch.rs:1911-1919). Remaining: ExperimentStore::settle_attempt runs synchronously inside async fn emit_feedback (graph_task_dispatch.rs:1769 at HEAD), and ExperimentStore::load_or_new plus the retrieval-outcome write run synchronously inside async fn settle_task_verification (:2644 gate fail, :2770 gate pass), not under spawn_blocking. The 87 untracked generated-tests/*.rs files at the repo root date from 2026-09-26, before the fix, and can be deleted.

## Notes

- 2026-10-01 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check.
  At BASE the prompt-treatment settlement already ran in `spawn_blocking` (`prompt_experiment.rs:94`), and the
  retrieval-arm assignment too (`graph_task_dispatch.rs:1130`). The two locked read-modify-writes left inline in
  `settle_task_verification`, the gate-fail and gate-pass retrieval outcomes (`verification.rs`), now run in
  `spawn_blocking`. The verify command now also reads `verification.rs` and counts `ExperimentStore::transaction`;
  it fails at BASE.
- Left: `prompt_experiment::context` still reads the store (`load_strict`, no write) inline when an attempt is
  prepared (`graph_task_dispatch.rs:1068`, `streaming.rs:157`). The old untracked `generated-tests/*.rs` files sit in
  MAIN's tree; deleting them is the coordinator's call.
