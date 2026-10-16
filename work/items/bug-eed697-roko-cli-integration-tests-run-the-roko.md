+++
id = "bug-eed697"
kind = "bug"
title = "roko-cli integration tests run the roko binary with the developer's HOME, so it loads ~/.roko/.env and the default ladder routes fixture tasks to real paid models"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-delivery.md (roko-delivery, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: testing bug-74164c"
anchors = ["crates/roko-cli/tests/common/mod.rs::Isolated", "crates/roko-cli/tests/spawn_isolation.rs", "crates/roko-cli/tests/worktree_task_diff.rs::run_plan", "crates/roko-cli/tests/plan_branch_integration.rs", "crates/roko-cli/src/dispatch/model_routing.rs"]
links = { depends_on = [], blocks = [], related = ["bug-c63cca", "bug-4e336b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_test_that_spawns_roko_isolates_it' crates/roko-cli/ && cargo test -p roko-cli --test spawn_isolation"
+++

## Problem

roko-cli integration tests that spawn the `roko` binary (for example `worktree_task_diff`, `plan_branch_integration`,
`attempt_diff_canary`, `operator_checkout_clean`) do not isolate `HOME`. On a developer machine whose `~/.roko/.env`
holds provider keys, the spawned binary loads them, and the default routing ladder (cheap `gpt-oss-120b`, mid
`glm-4.7`, strong `gpt-5.4-mini`) routes the fixture's task to a real model instead of the fixture's fake
`claude_cli` provider. It also makes a real rung-probe call first.

Seen 2026-10-15 (roko-delivery, main a0c7d0982): with `HOME=/Users/will`, the fixture of
`worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing` logged
`creating agent via provider adapter … slug=gpt-5.4-mini provider=openai_compat base_url=https://api.openai.com/v1`;
the real model wrote `feature.txt` so that `grep -q missing feature.txt` passed, and the test failed. The same run
with `HOME=<scratch>` routes to the fake and fails the plan, as the test expects. `plan_branch_integration`'s c3 and
c4 also failed with the real `HOME` and c3 passes with a scratch one.

## Why it matters

- Every `cargo test -p roko-cli` on such a machine spends real money from the developer's keys (small per call, but
  unbounded across agents and reruns) and sends fixture prompts to a provider.
- The tests pass or fail at random depending on the machine, so a real regression can hide behind them, and agents
  waste time on failures that are not theirs.

## Where

- The integration tests' `Command::new(cargo_bin("roko"))` helpers in `crates/roko-cli/tests/*.rs`: none sets
  `HOME`, `env_clear` or the env roko reads keys from.
- `crates/roko-cli/src/dispatch/model_routing.rs`: the routing ladder's default rungs, which win over a workspace whose
  only configured model is a fake.

## Plan

1. Give the integration tests one helper that runs `roko` with a scratch `HOME` (and `USER`, `PATH`), no provider
   key variables, and use it everywhere a test spawns the binary.
2. Decide whether a workspace whose `[agent] default_model` and only `[models.*]` entry is a fake should be routed
   to the built-in ladder's API models at all (an explicit model should win).

## Done when

`spawn_isolation` passes (the `[[verify]]` command): every test file that spawns `roko` isolates each spawn,
through `common::Isolated` or by setting `HOME` itself, and an isolated `roko` reads its global config from the
scratch home. The integration tests, run with a `HOME` whose `.roko/.env` holds a (dummy) OpenAI key, open no
connection to a provider. `worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing` still fails,
for another reason (bug-ee2752).

## Notes

- 2026-10-16 (roko-init): plan step 2 is done by reg-e2038b (commit 34baabd5b, branch `work/reg-e2038b`). A ladder
  rung now runs a built-in model only when the ladder is the workspace's own or the workspace configures no model
  at all, so a fixture with a fake `claude_cli` model and `default_model = "fake"` keeps every task on it whatever
  keys `~/.roko/.env` holds. Step 1, one helper that isolates `HOME` for every test that spawns `roko`, remains:
  16 of the 34 files that spawn it set no `HOME`. gap-1a30ca, filed for step 1 the same night, is closed as a
  duplicate of this item.
