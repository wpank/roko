+++
id = "spec-9a3131"
kind = "spec"
title = "Epic: hygiene and cheap wins"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "L"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/main", "roko-serve/routes", "roko-core/config", "docs/v3", ".github/workflows"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #6-7, §3, §5); workstreams/assessment/W5-contention-parallelism.md (rec 2-3); W6-build-verify-throughput.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/main.rs", "crates/roko-serve/src/routes/plans.rs", "crates/roko-core/src/config/learning.rs::LearningConfig", "docs/v3/", ".github/workflows/ci.yml"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-cold"
links = { depends_on = ["find-8cc7ac", "bug-470de8", "dec-e70592", "gap-c8e1f1", "gap-0d0e81", "gap-a6de8d", "gap-cdf3fc", "bug-b16d55", "bug-b17805", "bug-91af0e", "bug-31bca6", "bug-9434c4", "bug-919fe8", "bug-c1950e", "bug-7df50d", "dec-01be49", "gap-4b3bd5", "bug-779ae7", "bug-ccfa0d", "gap-d0f52f", "bug-a70def", "bug-f3969d"], blocks = [], related = ["spec-ae5f94", "spec-b7303f", "gap-b23ebd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-cli/src/graph_task_dispatch/verification.rs && test -f crates/roko-serve/src/routes/plans/run_control.rs && ! grep -qE 'enum (LearnCmd|PlanCmd) [{]' crates/roko-cli/src/main.rs && grep -q '^dream_on_completion = false' roko.toml && ! grep -q 'This is law' docs/v3/30-CONDUCTOR.md"

[[verify]]
command = "sha=\"$(gh api repos/{owner}/{repo}/commits/main --jq .sha)\" && test \"$(gh run list --commit \"$sha\" --json conclusion --jq length)\" -gt 0 && test \"$(gh run list --commit \"$sha\" --json conclusion --jq '[.[]|select(.conclusion==\"failure\" or .conclusion==\"startup_failure\" or .conclusion==\"timed_out\")]|length')\" = 0 && test \"$(gh api repos/{owner}/{repo}/branches/main/protection/required_status_checks --jq '.contexts|length')\" -gt 0"
+++

## Problem

Small fixes that cut cost, contention and confusion:

- CI has never passed (find-8cc7ac).
- Every plan run pays for a dream consolidation nobody reads (bug-470de8).
- 18 open items belong to subsystems the plan parks, and `next` keeps offering them (dec-e70592).
- Three large files force one writer at a time:
  - `graph_task_dispatch.rs`, 7,899 lines;
  - `main.rs`, 8,191 lines;
  - serve's `routes/plans.rs`, 4,391 lines.
- The docs claim things the code does not do (gap-cdf3fc).

## Why it matters

- **Parallel agents:** W5 counts about 70 open pieces of work queued on `graph_task_dispatch.rs`, 23 on `main.rs` and
  11 on `routes/plans.rs`. The split of `graph_task_dispatch.rs` gates E4.2 and the E2 dispatch fixes.
- **Cost:** dreams are a paid call per plan whose output was denied 56 times out of 56 (research note B6).
- **Trust:** the whitepaper and every agent session read the docs and `CLAUDE.md`.
- **Merges:** CI is the only check that catches breakage that appears only after a merge (W6).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs` and `graph_task_dispatch/`.
- `crates/roko-cli/src/main.rs` and `commands/`.
- `crates/roko-serve/src/routes/plans.rs`.
- `crates/roko-core/src/config/learning.rs`, `presets.rs` and `roko.toml`.
- `docs/v3/`, `CLAUDE.md` and `.github/workflows/`.

## Current state

Checked at `41c7ffbd6`:
- **find-8cc7ac:** `ci.yml` has run 117 times: 92 failed, 25 were cancelled and none passed (W6). The last run, on
  2026-09-21, failed on a missing member manifest. The working branch triggers no CI.
- **Dreams:** `dream_on_completion` is true by default (`learning.rs:214`) and in `roko.toml:373`.
- **`graph_task_dispatch.rs`:** four submodules already exist in `graph_task_dispatch/`. The learn-a worktree adds a
  fifth, `prompt_experiment.rs`, still uncommitted.
- **`main.rs`:** 41 clap enums at lines 110–3238, and tests at 4539–8191. The env worktree (bug-7d7200) has uncommitted
  edits to it.
- **`routes/plans.rs`:** no writer today; the last change was `188c43c8d`.
- **Docs:** all 16 rows of tldr/05 §5 are still present.

## Plan

This is the implementation plan.

1. **Now, in parallel:**
   - bug-470de8: the dream default;
   - gap-a6de8d: split `routes/plans.rs`;
   - gap-cdf3fc: the docs, in the docs lane;
   - dec-e70592: the parking decision, which needs the author;
   - find-8cc7ac: CI, whenever convenient (E16 tracks it too).
2. **After the portal branches merge (env and learn-a):**
   - gap-c8e1f1: split `graph_task_dispatch.rs`, in a freeze window of about a day with one writer;
   - then gap-0d0e81: split `main.rs`, about half a day.

   Each split remaps its anchors in the same commit.
3. **Exit check:** the static checks in the first `[[verify]]` pass, and CI on `main` is green with required checks
   set.

## Done when

- [ ] find-8cc7ac: Some GitHub workflows fail on main and required checks are undefined (existing item)
- [x] bug-470de8: Every plan run pays for a dream consolidation nobody reads: dream_on_completion defaults to true
- [x] dec-e70592: Decide whether to park the 18 items the TL;DR says to drop
- [x] gap-c8e1f1: Split graph_task_dispatch.rs into modules without changing behaviour
- [ ] gap-0d0e81: Split main.rs: move the clap command enums into their command modules
- [ ] gap-a6de8d: Split roko-serve routes/plans.rs into run-control, authoring, merge and read modules
- [x] gap-cdf3fc: Correct the 13 docs claims that the code or the literature contradicts
- [x] bug-b16d55: ACP starts a paid dream consolidation every 10 episodes, and no config flag turns it off
- [x] bug-b17805: The docs/v3 [learning] config table gives wrong defaults for eight fields
- [ ] bug-91af0e: The graph_execution module doc still says delivery is backed by MergeQueue and GitHubWorkflow
- [ ] bug-31bca6: Nothing reads learning.dreams.max_concurrent, so the ACP trigger starts another dream on every turn while one runs
- [ ] bug-9434c4: roko config set rejects learning.t0_reflexes and every learning.dreams key
- [ ] bug-919fe8: roko redirects any workdir under a .roko directory to the outer project, including per-task worktrees in .roko/worktrees
- [x] bug-c1950e: roko config validate warns that agent.default_model references a missing model when the model is a builtin
- [ ] bug-7df50d: Every cargo update flips tempfile's getrandom dependency between 0.4.3 and 0.3.4 in Cargo.lock
- [ ] dec-01be49: Decide how the doctor tests stop depending on the machine's claude and API keys: injectable probes or relaxed assertions
- [ ] gap-4b3bd5: commands/plan.rs walks plan directories itself instead of reusing plan_validate's collect_tasks_files
- [ ] bug-779ae7: Three lib tests fail only under heavy load: a roko-gate tautology-filter test and two dispatcher timing tests
- [ ] bug-ccfa0d: config validate flags [profiles.<name>] keys that DomainProfile collects into extra, and tools.profiles has no schema template
- [ ] gap-d0f52f: LearningRuntime::discover_cross_episode_patterns has no caller, so EpisodeView::succeeded has no production reader
- [ ] bug-a70def: The Claude MCP isolation tests assume the host has no managed-mcp.json
- [ ] bug-f3969d: InFlightTasks keeps every ended attempt for the life of the process
- [ ] Both of the epic's `[[verify]]` commands pass.

## Notes

- **find-8cc7ac is shared with E16** and keeps its `release` goal. Branch protection (required checks) is a GitHub
  setting that only the author can change.
- **gap-c8e1f1 comes before gap-96f7ed (E4.2) and the E2 dispatch fixes.** Otherwise they land in the monolith and
  conflict again (W5, sequencing constraints).
- **Not filed:** W5 rec 2c, which moves the signal, interrupt and TUI code out of `plan_runner.rs` (lines 30–520,
  2–3 hours). Add it if the E7 scheduler work finds `plan_runner.rs` still contended.
- **Related:** gap-b23ebd (about 585 citation errors across `docs/v3`).
- **Decided 2026-09-29 (Will):** the 18 set-aside items are held rather than parked (dec-e70592).
- **Still open (not accepted on 2026-09-29):** branch protection on `main`, which this epic's exit check assumes.
