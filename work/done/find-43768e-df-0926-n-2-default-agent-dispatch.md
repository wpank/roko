+++
id = "find-43768e"
kind = "finding"
title = "Default agent_dispatch_secs=600 too low for build-heavy tasks"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-core/config"]
created = 2026-09-26
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds"
anchors = ["crates/roko-core/src/config/timeouts.rs::default_agent_dispatch_secs", "roko.toml:393", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::base_attempt_timeout_ms", "crates/roko-cli/src/graph_task_dispatch/streaming.rs::dispatch_streaming", "crates/roko-graph/src/engine.rs::max_retries"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -A1 'const fn default_agent_dispatch_secs' crates/roko-core/src/config/timeouts.rs | grep -qE '^\\s*600\\s*$' && ! grep -qE '^agent_dispatch_secs = 600' roko.toml"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:31Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:15Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
Tasks that build and verify exceed 600s, fail with error_class=Timeout and burn the full retry budget (3 x 600s) learning nothing.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds`

How to verify: Review default and timeout-retry policy.

Verified 2026-09-28 (static check against 3d0ee4d02): The default is still 600s (roko-core/src/config/timeouts.rs:139-141; test :346) and the workspace roko.toml:393 sets agent_dispatch_secs = 600. Graph dispatch uses the task's timeout_secs when set, else agent_dispatch_secs (graph_task_dispatch.rs:3395-3399). A per-task override exists (task_parser.rs:170), but nothing treats a Timeout failure differently: the engine retries by FailureStrategy max_retries alone (roko-graph/src/engine.rs:2761), and no timeout-specific retry or escalation handling was found on the Graph path.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. The default and roko.toml are still 600 s, and the Graph path still has no timeout-specific retry or escalation (provider failover reacts to refusals and usage exhaustion only). The timeout selection is now at graph_task_dispatch.rs:3629 (dispatch) and :4202 (dispatch_streaming). Related: bug-562c22 (a timed-out Claude CLI attempt records zero cost).

Checked 2026-09-29: Partly fixed in e0673e3e0: a retry after a timeout gets 1.5x the previous timeout, up to 4x the base. Still open: the 600 s default is unchanged and there are no per-tier dispatch timeouts; the verify command checks the default.

## Notes

- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  `timeouts.agent_dispatch_secs` now defaults to 1800 s (it was 600), and this repo's `roko.toml` sets 1800. With
  e0673e3e0's 1.5x escalation, a timed-out task's attempts get 1800, 2700 and 4050 s, capped at 4x. Silent attempts
  are still ended sooner by the stall watchdog (`[conductor] task_stall_secs`, default 300 s), so the longer bound
  only lets active work finish. `plan_total_secs` (3600) still exceeds it. The `[[verify]]` greps pass statically.
  Revert if 1800 is the wrong product default; any value other than 600 satisfies the verify.
