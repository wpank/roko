+++
id = "bug-ec244b"
kind = "bug"
title = "Eight more done items fail their static verify: three name tests that no longer exist, the rest check moved code, a dropped figure or a wrong path"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "62eee2176"
source = "gap-6ee3c5 worker, 2026-10-16"
discovered_from = "gap-6ee3c5: the static regression sweep verify --closed-since 2026-09-26 --static-only"
anchors = ["work/done/bug-1f098f-whitepaper-pdf-drops-the-space-before-the.md", "work/done/dec-648cce-should-roko-deploy-railway-forward-roko-serve.md", "work/done/dec-af63cc-should-t0-reflexes-serve-tasks-that-something.md", "work/done/gap-198c9c-pk04-operator-control-and-guards-roko-s.md", "work/done/gap-7ec3ef-pk49-m3-self-model-compute-the-m3.md", "work/done/gap-cced54-two-m3-self-model-assumptions-cheap-arm.md", "work/done/q-61b3f8-should-plan-set-parallelism-stay-opt-in.md", "work/done/q-bc6cc5-confirm-licence-coverage-for-another-contributor-s.md"]
links = { depends_on = [], blocks = [], related = ["gap-6ee3c5", "bug-f5f4d2", "bug-2f0b39", "bug-ac2a01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/work.py verify --static-only bug-1f098f dec-648cce dec-af63cc gap-198c9c gap-7ec3ef gap-cced54 q-61b3f8 q-bc6cc5"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T19:50:51Z"
commit = "62eee2176"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-16T19:19:36Z"
forced = true
evidence = "62eee2176: each of the eight failing verifies is retired with retire-verify, and six items get a replacement [[verify]] that passes. bug-1f098f: retired by 7898493f2 (figure gone; b91be1473 deleted status_matrix.py); verify 2 checks the figures README's librsvg rule and that no figure starts a <tspan> with a space. dec-648cce: its grep targeted deployment.rs, which never held the Railway env code, and decision (a) is unimplemented; filed gap-ea9ed8 (verify fails now) and retired the check by it. dec-af63cc: option (b)'s test was never written; retired by b883db4e6; verify 2 runs bug-b4c565's narrow-reflex test. gap-198c9c: 6dd936e4b pushed inherit_gate_env_from past verify 5's 25-line window; verify 11 reads the whole function (fails before 256c9ab86). gap-7ec3ef: verify 4 retired by 5350b2c14 (gap-9aca5d made the default shadow). gap-cced54: retired by b883db4e6 (code confirmed as is); verify 2 checks arm_key and cheap_arms. q-61b3f8: verify 2 checks default_max_parallel_plans() returns 1. q-bc6cc5: added the ## Outcome section from the recorded evidence, retired by 15243f788, verify 2 finds it. FORCED because the item's verify, run in this worktree, fails only on gap-7ec3ef verify 1, which needs the untracked benchmarks/viabilitybench/.venv. Run read-only in the main checkout, that command passes (5 passed), and every other entry passes or is RETIRED. dec-648cce is listed as having every verify retired: gap-ea9ed8 carries its check. check: 0 problems."
+++

## Problem

`python3 tools/work.py verify --static-only bug-1f098f dec-648cce dec-af63cc gap-198c9c gap-7ec3ef gap-cced54 q-61b3f8
q-bc6cc5` prints a FAIL for each of these done items. None of them is among the 60 that the 2026-10-15 assessment
classified (gap-6ee3c5 retired the superseded ones; bug-f5f4d2 has the moved ones). Found on 2026-10-16 in the static
regression sweep (`verify --closed-since 2026-09-26 --static-only`) run for gap-6ee3c5:

| Item | Why its verify fails now (first look, 2026-10-16) | Likely action |
|---|---|---|
| bug-1f098f | `figures/fig2-golden-path.svg` and the status-matrix check: the whitepaper rewrite (7898493f2) dropped both | retire (`--by 7898493f2`; bug-ac2a01 owns status_matrix.py) |
| dec-648cce | `ROKO__SERVE__AUTH__API_KEY` is no longer in `crates/roko-cli/src/deployment.rs`; it is in `config_cmd.rs` and `serve_client.rs` | repoint |
| dec-af63cc | no test `a_reflex_on_a_checked_task_runs_verification_on_its_cached_output` anywhere under `crates/` | find where it went (`git log -S`), else a regression |
| gap-198c9c | verify 5's `grep -A25 'async fn regression_output'` window is too short: `inherit_gate_env_from` is at `graph_execution/delivery.rs:427`, 29 lines down, so the fix holds | repoint |
| gap-7ec3ef | verify 4: no test `self_model_config_defaults_off` under `crates/roko-core/src/config/` | find where it went, else a regression |
| gap-cced54 | no test `matrix_replay_uses_the_registry_arm_key_not_the_replay_fallback` under `crates/roko-learn/` | find where it went, else a regression |
| q-61b3f8 | `grep -n 'max_parallel_plans' crates/roko-core/src/config/` greps a directory without `-r`, so it always fails | repoint (`grep -rn`) |
| q-bc6cc5 | it greps `work/items/q-bc6cc5-….md`, but closing moved the file to `work/done/` | repoint |

## Why it matters

Goal `tooling`: the regression sweep is only useful while a FAIL means a regression. Three of these (dec-af63cc,
gap-7ec3ef, gap-cced54) name tests that no longer exist, which is either a rename or lost coverage.

## Where

The eight item files (the anchors), and the code each verify names (table above).

## Current state

All eight are done. Nothing else tracks these failures.

## Plan

For each item, find what changed (`git log -S'<symbol>'`, `git log --diff-filter=D -- <path>`). If a later change made
the check obsolete, retire it with `python3 tools/work.py retire-verify <id> --index N --reason "…" --by <commit>`. If
the code only moved, retire the stale entry and add a `[[verify]]` that checks the new place. If the fix itself is gone,
file a regression item that links the old one.

## Done when

The `[[verify]]` command passes, with no FAIL for these items.

## Notes

gap-7ec3ef's first verify runs the viabilitybench venv's pytest, which only the main checkout has: judge that one there.
In a worktree the same sweep also fails about 80 other done items whose verifies read untracked paths (`tmp/`,
`.roko/`, the viabilitybench `.venv`, `target/`, `.claude/skills/`); only the main checkout can judge those. Until
bug-2f0b39 is fixed, leave dec-01be49, gap-fcb44c and gap-31c0e8 out of a `--static-only` sweep: their static parts run
a cargo test, a Docker build and a real-binary pytest.
