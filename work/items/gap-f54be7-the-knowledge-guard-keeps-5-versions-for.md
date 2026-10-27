+++
id = "gap-f54be7"
kind = "gap"
title = "The knowledge guard keeps 5 versions for every proposer: since bug-f8655e each committed distillation is a version, so 5 of a serve's gated webhook dispatches push a plan run's knowledge version out of roko learn rollback's reach (Undertow keeps v13 to v17, its last 5 runs)"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/guarded-commit", "roko-neuro/knowledge"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 roko-learnguard (bug-f8655e, the lead's ask; Undertow .roko/learn/commits/knowledge copy of 2026-10-27 01:24)"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-learn/src/guarded_commit.rs::LKG_DEPTH", "crates/roko-neuro/src/episode_completion.rs::write_distilled", "crates/roko-neuro/src/knowledge_store/commit.rs::LaterBatches"]
links = { depends_on = [], blocks = [], related = ["bug-f8655e", "gap-1220c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_distillation_never_takes_a_run_version_out_of_reach' crates/roko-neuro/src && cargo test -p roko-neuro a_distillation_never_takes_a_run_version_out_of_reach"
+++

## Problem

Every proposer shares the knowledge guard's 5 kept versions. Since bug-f8655e, distillations are proposers too, so a
busy serve can push a plan run's version out of `roko learn rollback`'s reach.

- **The guard keeps `LKG_DEPTH` = 5 versions** (`crates/roko-learn/src/guarded_commit.rs`). `roko learn rollback
  knowledge --to N` needs version N to still be kept. Each committed or observed proposal writes a version; a
  rolled-back proposal writes none.
- **Plan runs:** one version per run. Undertow's guard (copy taken 2026-10-27 01:24) has 17 rows, one per plan run,
  and keeps v13 to v17, which are round v7's last 5 runs.
- **Distillations, since bug-f8655e:**
  - a distillation that writes at least one entry under `observe` or `enforce` proposes its own batch,
    `distill:<episode>`, and so becomes a version;
  - by default only an episode that passed a gate is distilled. Today that means `roko serve`'s webhook dispatches
    whose template runs gates;
  - a subscription by default has `concurrency_limit = 1`, `cooldown_secs = 0` and `debounce_ms = 0`
    (`crates/roko-core/src/config/subscriptions.rs`), so every matching webhook event or cron tick can distil one
    batch;
  - with `[learning] distill_ungated = true`, every successful capture episode (plan writes, research, direct agent
    calls) and every successful ACP turn proposes a version too.
- **So 5 committed distillations after a plan run** put that run's version beyond `roko learn rollback`. A serve
  with one gated subscription on an active repository gets there within one run.

## Why it matters

- Rollback is a person's undo for a bad run's knowledge (decision 8103's guard). Its reach should count runs, not
  every background proposal.
- Goal: learning.

## Where

- `crates/roko-learn/src/guarded_commit.rs::LKG_DEPTH`, `GuardedStore::commit` (it drops versions beyond the depth).
- `crates/roko-neuro/src/episode_completion.rs::write_distilled` (bug-f8655e's per-distillation proposal).
- `crates/roko-neuro/src/knowledge_store/commit.rs::LaterBatches`, which reads the kept versions after `to`.

## Plan

Pick one; a design call:
1. **Depth per proposer kind:** keep the last 5 run versions, plus the last few distillation versions, so that a
   distillation never drops a run's version.
2. **One rolling distillation version:** distillations between two runs extend a single version, not one each.
3. **Make rollback independent of the snapshots:** a committed batch records its entry ids in its row, or in a side
   ledger, and `LaterBatches` reads them from there instead of from the kept snapshots.

## Done when

- The `[[verify]]` passes: after a plan run's version and 5 committed distillations, `roko learn rollback knowledge
  --to <the run's parent>` still works.

## Notes

- Found while doing bug-f8655e, at the lead's ask: "count how many a busy serve could produce per run; if they
  could still push plan-run versions out of `roko learn rollback`'s reach, file a separate item … and don't build it
  here".
- Related: bug-f8655e, gap-1220c5.
