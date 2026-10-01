+++
id = "gap-4ec59f"
kind = "gap"
title = "Worktree Isolation: Flip Default and Add Startup Repair"
status = "open"
triage = "verified"
severity = "p0"
size = "L"
goal = "core"
subsystem = ["roko-cli/orchestrator"]
created = 2026-09-21
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "tmp/backlog/archive/400-worktree-isolation-defaults.md#400 — Worktree Isolation: Flip Default and Add Startup Repair"
discovered_from = "audit:tmp/backlog/archive/400-worktree-isolation-defaults.md#400 — Worktree Isolation: Flip Default and Add Startup Repair"
anchors = ["crates/roko-cli/src/orchestrator/executor/mod.rs::ExecutorConfig::default_use_worktrees", "crates/roko-cli/src/graph_execution/plan_runner.rs::GraphPlanRunParams", "crates/roko-cli/src/graph_execution/plan_runner.rs:1155", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::with_workspace_provider", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/commands/plan.rs::cmd_resume", "crates/roko-cli/src/main.rs:2138", "crates/roko-cli/src/serve_runtime.rs:888", "crates/roko-cli/src/serve_client.rs:560", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::clear_stale_locks", "crates/roko-cli/src/orchestrator/worktree/cleanup.rs::prune"]
links = { depends_on = [], blocks = [], related = ["bug-109b5a", "bug-53475e", "gap-d58ae8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -A1 'const fn default_use_worktrees' crates/roko-cli/src/orchestrator/executor/mod.rs | grep -q true && grep -rqn 'use_worktrees' crates/roko-cli/src/graph_execution crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/serve_runtime.rs && grep -rn 'clear_stale_locks()\\|clear_stuck_mutation_lock()' crates/roko-cli/src --include='*.rs' | grep -v 'orchestrator/worktree/' | grep -q ."
+++

## Problem

`roko plan run <dir>` runs every task of a plan in the user's own working tree unless the operator passes
`--worktree-per-task`. Nothing turns isolation on by default, and nothing repairs worktree state left by a
crashed run:

- `ExecutorConfig::default_use_worktrees()` returns `false`, and the Graph path never reads
  `[executor] use_worktrees` at all. Setting `use_worktrees = true` in `roko.toml` has no effect.
- The only switch is the CLI flag `--worktree-per-task`. `roko serve` hard-codes it off for server-started
  runs, and `roko plan run` refuses the flag when it delegates to a running server.
- The startup repair helpers (`WorktreeManager::prune`, `clear_stale_locks`, `clear_stuck_mutation_lock`) have
  no production caller, so a stale `.git/index.lock` or a stuck repository mutation lock from a killed run
  blocks the next run until someone removes it by hand.

Expected: task isolation on by default with an explicit opt-out (`--no-worktree-per-task` or
`[executor] use_worktrees = false`), the config value honoured on every entry point (CLI, resume, serve), and
stale git/worktree state repaired before the first task is dispatched.

## Why it matters

Goal `core` (plan runs work reliably). Without isolation, tasks that run at the same time in one plan share a
working directory. Two tasks that edit the same file overwrite each other with no error, no gate failure and
no audit trail. The self-hosting workflow (`roko plan run plans/`) never passes the flag, so roko develops
itself in the riskiest configuration.

Related: `bug-109b5a` (no pre-spawn stale `.git/index.lock` cleanup, including `.git` indirection in linked
worktrees), `bug-53475e` (failed worktree cleanup keeps the kernel mutation lock, so an operator has to step
in), `gap-d58ae8` (Mori-shaped workflow unproven: distinct worktrees per attempt plus serialized merge),
`bug-a3760a` (`GitDeliveryBackend::git_merge` checks out branches in the user's tree).

## Where

- `crates/roko-cli/src/orchestrator/executor/mod.rs`: `ExecutorConfig.use_worktrees` (:177-179), the default
  (`default_use_worktrees`, :225-227, returns `false`), and the test `executor_config_disables_worktrees_by_default`
  (:744-745), which asserts `false`.
- `crates/roko-cli/src/config.rs`: the partial `[executor]` layer parses `use_worktrees: Option<bool>` (:1981)
  and merges it (:2229). `roko config set executor.use_worktrees` is accepted (:1559).
- `crates/roko-cli/src/config_cmd.rs::run_init_wizard` (:132) already writes `executor.use_worktrees = true`
  into new `roko.toml` files. Today that value is ignored.
- `crates/roko-cli/src/main.rs:2130-2138`: the `--worktree-per-task` clap flag (opt-in, Graph engine only).
- `crates/roko-cli/src/commands/plan.rs`: passes the flag through (:484, :586, :631, :2532, :2580).
  `cmd_resume` (:1728) builds its run params with `worktree_per_task: false` (:1812).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `GraphPlanRunParams.worktree_per_task` (:659). A
  guard (:840-846) refuses the flag with `max_parallel_plans > 1` because "per-task worktrees are never merged
  back". The opt-in block (:1154-1171) builds a `WorktreeManager` (repo root = workdir, `base_branch = "HEAD"`,
  root `.roko/worktrees`, 1 h idle TTL) and a `WorktreeExecutionWorkspaceProvider`, then calls
  `dispatcher_builder.with_workspace_provider(..)`.
- `crates/roko-cli/src/graph_task_dispatch.rs`: `GraphTaskDispatcher::with_workspace_provider` (:1262). A lease
  is acquired per attempt (:3374). A failed attempt releases with `RetainForFailure` (:3762, :3820, :3912). A
  successful one releases with `Delete` (:3924-3948). The comment there says the changes "can be merged
  separately via the delivery pipeline. For now the worktree is cleaned up."
- `crates/roko-cli/src/graph_execution/workspaces.rs`: `WorktreeExecutionWorkspaceProvider` (acquire →
  `WorktreeManager::create_for_attempt`; release(Delete) → `WorktreeManager::remove`).
- `crates/roko-cli/src/orchestrator/worktree/mod.rs::WorktreeManager::remove` (:1044) refuses a dirty checkout
  (`WorktreeError::DirtyWorktree`).
- `crates/roko-cli/src/orchestrator/worktree/cleanup.rs`: `clear_stuck_mutation_lock` (:181, with a non-Unix
  stub at :257), `clear_stale_locks` (:269: removes `.git/index.lock` and `.git/worktrees/*/index.lock` older
  than 60 s, holding the repository mutation lock), and `prune` (:311: `git worktree prune`, which also clears
  stale locks first).
- `crates/roko-cli/src/serve_runtime.rs:888`: serve-started Graph runs hard-code `worktree_per_task: false`.
- `crates/roko-cli/src/serve_client.rs:531,560`: when a server owns the workspace, `roko plan run` refuses
  `--worktree-per-task` (added in `08a1fd272`).
- `crates/roko-agent/src/claude_cli_agent.rs::build_settings_json` (:39): the `PreToolUse` git hook blocks for
  the Claude CLI provider. These already exist.

Entry points: `roko plan run <dir>` → `commands/plan.rs` → `graph_execution::plan_runner` (the Graph run);
`roko resume` / `--resume-plan` → `cmd_resume`; `roko serve` → `serve_runtime.rs`.

## Current state

Checked at HEAD `a17d9d766`:

- The default is still `false`, and the test still asserts `false`.
- No code under `graph_execution/`, `commands/plan.rs` or `serve_runtime.rs` reads `executor.use_worktrees`.
  The field is parsed and never used.
- `prune`, `clear_stale_locks` and `clear_stuck_mutation_lock` have no caller outside `orchestrator/worktree/`.
- **Nothing merges back.** A per-task worktree is created from `HEAD` (committed state only). Nothing commits
  the agent's edits, and nothing merges the attempt branch into the run's target.
  `WorktreeManager::accept_attempt` / `accepted_for_plan` have no callers outside the worktree module, and
  `GitDeliveryBackend` is never constructed (see `bug-a3760a`). On success, `release(Delete)` calls `remove`,
  which refuses the dirty worktree. The failure is logged as "worktree release failed (best-effort)" and the
  task's changes stay in `.roko/worktrees/…`. So with `--worktree-per-task` today:
  - a successful task's edits never reach the user's working tree;
  - a dependent task starts from `HEAD` and does not see its predecessor's edits;
  - uncommitted edits in the user's tree are not visible to tasks.
- The item's original premise ("none of this matters because the default is false") is therefore incomplete.
  Flipping the default alone would make every multi-task plan lose its work. The flip needs merge-back first.

## Plan

Land it in this order. Steps 1-3 are safe now and keep the default `false`. Step 5 is the flip.

1. **Startup repair.** In `plan_runner.rs`, before the first dispatch, build the `WorktreeManager` once (move
   it out of the `if worktree_per_task` block) and call `clear_stuck_mutation_lock()`, `clear_stale_locks()`
   and `prune().await`. Log what was cleared; never fail the run on a repair error. Recommendation: always run
   `clear_stale_locks` (it only touches locks older than 60 s, and it helps `bug-109b5a`), and run `prune` only
   when worktrees are enabled.
2. **Config plumbing.** Resolve one effective value: `--worktree-per-task` → true, `--no-worktree-per-task`
   (new, `conflicts_with` the other flag in `main.rs`) → false, otherwise `roko_config.executor.use_worktrees`.
   Pass it into `GraphPlanRunParams.worktree_per_task`. Do the same in `cmd_resume` (replace the hard-coded
   `false` at `commands/plan.rs:1812`) and in `serve_runtime.rs:888`, which reads the server's config. In
   `serve_client.rs`, keep refusing both explicit flags when delegating (the server's config decides). The
   refusal message should say so.
3. **Parallel-plan guard.** `plan_runner.rs:840-846` turns into a hard error for any config with
   `max_parallel_plans > 1` once isolation is the default. Until merge-back exists, keep the guard but name the
   opt-out (`--no-worktree-per-task` or `--max-parallel-plans 1`) in the message.
4. **Merge-back (prerequisite for the flip).** On task success, before `release(Delete)`:
   - commit the attempt worktree (`git add -A`, then `git commit` on the attempt branch from
     `format_attempt_branch_name`);
   - integrate that commit into the run's target so later tasks and the user see it.

   Options:
   - (a) A plan integration branch in its own worktree. Tasks branch from it, and successful attempts merge
     into it serially under the repository mutation lock. This is Mori's model and fits `gap-d58ae8`.
   - (b) Apply the attempt diff (`git diff --binary`) to the main tree with `git apply --3way` under the
     mutation lock. Simpler, but it touches the user's tree and has to deal with their uncommitted edits.

   Recommend (a). Never `git checkout` in the user's tree (`bug-a3760a`). New task worktrees must branch from
   the integration branch, not `HEAD`. After merge-back works, relax the guard in step 3.
5. **Flip the default.** `default_use_worktrees()` → `true`. Rename the test to
   `executor_config_enables_worktrees_by_default` and assert `true`. Update the flag help in `main.rs` and the
   CLI docs.
6. Add tests:
   - a Graph run with config `use_worktrees = true` gets a workspace provider;
   - `--no-worktree-per-task` overrides the config;
   - startup repair removes a stale `index.lock` (the `is_stale_lock` age helper exists in `cleanup.rs`);
   - a two-task dependent plan in worktree mode: task 2 sees task 1's file, and the file ends up in the
     target.

## Done when

- `roko plan run <dir>` with no isolation flag and no `use_worktrees` key runs each task in `.roko/worktrees/…`.
  A dependent task sees its predecessor's changes, and the completed plan's changes are in the target
  branch/workdir.
- `--no-worktree-per-task` or `[executor] use_worktrees = false` runs in the shared tree, as today.
- `roko resume` and serve-started runs use the same resolved setting. No `worktree_per_task: false` hard-code
  is left in `serve_runtime.rs` or `cmd_resume`.
- A run after a simulated crash (stale `.git/index.lock` older than 60 s, stale `git worktree` metadata)
  repairs both before the first dispatch and logs it.
- `cargo test -p roko-cli` passes, including the renamed default test and the new tests above. Clippy is clean.
- The `[[verify]]` command passes: the default returns `true`, `use_worktrees` is read under
  `graph_execution/`, `commands/plan.rs` or `serve_runtime.rs`, and `clear_stale_locks()` or
  `clear_stuck_mutation_lock()` is called outside `orchestrator/worktree/`.

## Notes

- Do not flip the default (step 5) before merge-back (step 4) works. Without it, every plan run by default
  loses its output. If you only have time for steps 1-3, land them, leave this item open and record in a dated
  note what is left.
- Users who ran `roko setup` already have `executor.use_worktrees = true` in `roko.toml`. Step 2 turns
  isolation on for them immediately. That is one more reason to have step 4 in place first, or to ship steps 2
  and 4 together.
- Checkpoint fingerprints include execution config (`graph_checkpoint.rs` test
  `fingerprint_changes_with_execution_config`). Unknown whether `executor.use_worktrees` is part of the hashed
  config. If it is, flipping the default invalidates in-flight checkpoints (resume would need
  `--force-resume`). Check this before step 5.
- `clear_stuck_mutation_lock` deletes a lock file. Keep its liveness check (it leaves a lock owned by a live
  process alone). `roko serve` and a CLI run can share a workspace, so repair must never clear a lock a live
  process holds.
- `clear_stale_locks` assumes `.git` is a directory. In a linked worktree `.git` is a file (`bug-109b5a` covers
  this).
- Out of scope: git hook coverage for Codex CLI, Cursor ACP and Cursor CLI; disk reclaim and scoped
  `target/` pruning; API-only providers (prompt-only enforcement is acceptable, document it).
- Parallel safety: this touches hot files (`plan_runner.rs`, `commands/plan.rs`, `main.rs`,
  `serve_runtime.rs`, `serve_client.rs`, `graph_task_dispatch.rs`). Do not run it alongside other items that
  edit those files (for example `gap-0001a1`, parallel plan queue). This is git-safety code: test on a scratch
  repo, never on the user's checkout.
- 2026-10-01 (wk-tiers): Step 1 implemented on `work/gap-4ec59f` at `8e23f0a79`; cargo verification deferred to the
  batch check. Steps 2-6 are not done, and the item stays open.
  - Step 1 (repair before the first dispatch):
    - With `--worktree-per-task`, `plan_runner.rs::repair_worktree_state` runs before the first dispatch. It
      calls `clear_stuck_mutation_lock()`, `clear_stale_locks()` (logging each lock it removes) and `prune()`.
      A failure is logged, and the run goes on.
    - Shared mode skips this repair. There, the per-dispatch `clear_stale_index_lock` (bug-109b5a) covers the
      checkout with a 10-min threshold. The startup repair would remove the user's lock at 60 s and create the
      mutation lock file in their `.git`.
  - Step 5: the default is not flipped and the config is not honoured. Re-checking the premise at `32938ad4b`
    found blockers that the plan does not cover:
    - The bench's Roko arm (`benchmarks/viabilitybench/driver/run_roko.py`) runs `roko plan run` and scores the
      tree in the task workdir. With isolation, results land on the run's batch branch `roko/run/<id>`
      (spec-f830c4) unless `--promote <branch>` is given, so the arm would score an untouched tree.
    - e2e/CI tests and scratch runs use non-git temp dirs. Batch integration and worktrees need a git repo with a
      HEAD commit, so those runs would fail at startup.
    - `roko run`, `roko do`, PRD, cloud-worker, serve and resume runs all hard-code `worktree_per_task: false`.
      Resolving the default for them (step 2) would send a one-shot run's result to a batch branch as well.
    - `[executor] use_worktrees` is never loaded on the Graph path. The core loader drops tables it doesn't know,
      and `Config::from_roko_config` sets `ExecutorConfig::default()`. Honouring the key needs the field in
      `RokoConfig` first. Users who ran `roko setup` have `use_worktrees = true` and would switch to isolation
      at once.
  - Next:
    1. Decide where the results of a default isolated run go (promote into the user's branch and tree by
       default?).
    2. Fall back to shared mode in non-git workdirs.
    3. Then do steps 2 and 5 together.
- 2026-10-01 (wk-tiers): step 2 (config plumbing) on work/gap-4ec59f; cargo verification deferred to the batch check.
  - The setting lives in the core schema as `[runner] worktree_per_task` (default `false`, from
    `CoreRunnerConfig::default_worktree_per_task`). It is not under `[executor]`: wk-cfg removes `ExecutorConfig`
    and puts `[executor]` in `REMOVED_CONFIG_KEYS` (gap-666ab3).
  - `roko plan run` takes the new `--no-worktree-per-task` (it conflicts with `--worktree-per-task`). With neither
    flag, `commands/plan.rs::resolve_worktree_per_task` reads the config.
  - `--promote` no longer needs the flag: config can turn worktrees on. It conflicts with the opt-out, and is refused
    at run time when the resolved mode is shared.
  - `cmd_resume` passes neither flag, so it follows the config. Serve-started runs use the server's
    `roko_config.runner.worktree_per_task`.
  - `serve_client` still refuses both flags when it delegates, and says the server's config decides.
  - `roko config set runner.worktree_per_task` is accepted.
  - Tests: `cli_parses_the_worktree_per_task_opt_out` and `worktree_per_task_follows_the_flag_then_the_runner_config`.
  - Checkpoint fingerprint: worktree mode is not part of it. Only the exclusive paths change, and
    `roko-graph/src/fingerprint.rs::NodeFingerprint` leaves them out. So flipping the default does not invalidate
    in-flight checkpoints.
  - `roko run`, `roko do`, PRD runs and the cloud worker still run shared on purpose: they are one-shot runs into the
    user's tree.
- 2026-10-01 (wk-tiers): step 3 (parallel-plan guard) on work/gap-4ec59f; cargo verification deferred to the batch check.
  - The guard stays. Its message no longer claims that worktrees are never merged back, which has been false since
    batch integration (spec-f830c4, `8268c7498`). It now names both ways out: `--max-parallel-plans 1`, or
    `--no-worktree-per-task` / `[runner] worktree_per_task = false`.
  - Relaxing it looks possible. Deliveries into the batch take turns, and a dependent plan starts only after its
    prerequisite is delivered. That needs a live check of parallel plans in worktrees first, so it is left.
- 2026-10-01 (wk-tiers): groundwork for step 5 on work/gap-4ec59f; cargo verification deferred to the batch check.
  - Isolation that comes from config (and so from the default, once it flips) falls back to the shared working tree,
    with a warning, when the workdir is not a git checkout with a commit (`batch::has_head_commit`). This is the
    e2e/CI/scratch case. An explicit `--worktree-per-task` there still fails, as before.
  - Serve applies the same check to its config value.
  - Tests: `has_head_commit_needs_a_checkout_with_a_commit` and the extended
    `worktree_per_task_follows_the_flag_then_the_runner_config`.
  - Step 4 (results reaching the user's checkout) is waiting on a decision: the C3/C4 and proof-case-2 canaries
    assert that the operator's checkout never changes.
- 2026-10-01 (wk-tiers): step 6, part 1: `worktree_startup_repair_clears_a_stale_index_lock` (plan_runner tests) runs
  `repair_worktree_state` on a repository with a 2-minute-old `.git/index.lock` and checks the lock is cleared. This
  covers step 1's repair. Cargo verification deferred to the batch check.
- 2026-10-01 (wk-tiers): step 4 (merge-back) on work/gap-4ec59f, option (a) as the coordinator decided; cargo
  verification deferred to the batch check.
  - The golden path's invariant stays: roko never changes the operator's checkout (C3/C4, proof case 2). Merge-back
    already exists. Each passed attempt is accepted onto `roko/plan/<plan>` (gap-3b5361), and later attempts start
    from it. Each passed plan is delivered into `roko/batch/<run>` after a regression check (spec-f830c4). What was
    missing was telling the operator where the work is.
  - The end of a worktree run now says: "The work is on branch roko/batch/<id>; your checkout was not changed. To
    take it:", followed by the command from `batch::merge_command`:
    - `git merge --ff-only roko/batch/<id>` while the checkout is still behind the batch;
    - `git merge roko/batch/<id>` once the checkout has moved;
    - nothing once the checkout has the batch's work.
  - A `--promote` run prints the promotion's summary instead. The JSON summary's `batch` object gains
    `merge_command`.
  - `roko plan status <dir>` shows `delivered: <branch> at <commit>` and `take it with: <command>`, read from the
    checkpoint's `roko.batch@1` record (`graph_checkpoint::recorded_batch_delivery`). `--json` adds a `delivery`
    object.
  - Tests: `merge_command_fast_forwards_until_the_checkout_moves` (batch.rs) and
    `recorded_batch_delivery_reads_only_a_delivered_plan` (graph_checkpoint.rs).
  - Bench: the Roko arm's emitted roko.toml sets `[runner] worktree_per_task = false` (planemit-3), so its results
    keep landing in the task workdir the driver reads, and `planemit._check` refuses a config without it. The
    ViabilityBench suite passes: 373 passed, 5 skipped (3 of the skips because this worktree has no roko binary).
  - A possible follow-up for Will is option (b): an opt-in fast-forward of the operator's branch at the end of the
    run, when `HEAD` is still the batch base and the checkout is clean.

## Original notes

Roko's `WorktreeManager` and `WorktreeExecutionWorkspaceProvider` are production-quality infrastructure. The `PreToolUse` git hook blocks in `build_settings_json()` are already implemented and wired for ClaudeCliAgent. None of this matters because `default_use_worktrees()` returns `false`.

Imported without verification from:
- `tmp/backlog/archive/400-worktree-isolation-defaults.md#400 — Worktree Isolation: Flip Default and Add Startup Repair`

How to verify: Check: `default_use_worktrees()` returns `true`.; The test `executor_config_disables_worktrees_by_default` is renamed and; `config.executor.use_worktrees` is read at the graph-engine startup path and [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: still true - orchestrator/executor/mod.rs:225-227 default_use_worktrees() returns false and no Graph-path code reads executor.use_worktrees; Graph per-task worktrees are opt-in only (graph_execution/plan_runner.rs:1059-1073, graph_task_dispatch.rs:926; serve_runtime.rs:577 hard-codes worktree_per_task: false); startup repair helpers clear_stale_locks / clear_stuck_mutation_lock (orchestrator/worktree/cleanup.rs:181/269) have no production caller. Duplicates folded in: gap-1673bb, gap-c8d637, gap-764230.

Rechecked 2026-09-29 at d9e79e9d8. Still open. Anchors moved: the Graph worktree opt-in block is now plan_runner.rs:1154-1171, the serve hard-code is serve_runtime.rs:888, and the dispatcher's workspace provider is GraphTaskDispatcher::with_workspace_provider (graph_task_dispatch.rs:1259). Since 08a1fd272, when roko plan run delegates to a running serve, serve_client.rs refuses --worktree-per-task, so on that path worktree isolation cannot be turned on even per run.
