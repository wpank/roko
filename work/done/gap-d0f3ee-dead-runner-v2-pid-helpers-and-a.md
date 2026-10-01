+++
id = "gap-d0f3ee"
kind = "gap"
title = "Dead Runner-v2 PID helpers and a duplicate process start-time probe remain outside the PID registry"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/runner", "roko-cli/agent_serve"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-cli/src/runner/persist.rs::save_agent_pids", "crates/roko-cli/src/runner/persist.rs::cleanup_orphaned_agents", "crates/roko-cli/src/agent_serve.rs::get_process_start_time", "crates/roko-agent/src/process/identity.rs::process_identity"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -qE "pub fn (save_agent_pids|cleanup_orphaned_agents)" crates/roko-cli/src/runner/persist.rs && ! grep -q "fn get_process_start_time" crates/roko-cli/src/agent_serve.rs'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:50Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:49Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`runner/persist.rs::save_agent_pids` and `cleanup_orphaned_agents` have no production caller (only a round-trip test). The latter reads the legacy `agent_pids.json` and calls `register_spawned_pid` for each entry. `register_spawned_pid` records the identity of whatever process holds that PID at that moment, so if anything re-wired this helper, stale Runner-v2 PIDs could be registered against unrelated processes and later killed by cleanup. Separately, `agent_serve.rs` keeps its own per-OS `get_process_start_time` (three cfg variants) instead of using `roko_agent::process::identity::process_identity`.

Fix: delete the dead helpers and reuse the registry's identity probe.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- Deleted `runner/persist.rs`'s `save_agent_pids` and `cleanup_orphaned_agents` (no caller outside a round-trip test, also deleted); roko-agent's registry already reads the legacy `agent_pids.json` itself (`legacy_orphans`). `agent_serve.rs`'s three per-OS `get_process_start_time` probes became `process_start_fingerprint`, over `roko_agent::process::identity::process_identity`. Linux values are unchanged (both read `/proc` start ticks); on macOS an `agents.json` entry recorded by an older roko held a hash of `ps -o lstart=` and no longer matches, so `roko agent stop` declines to signal it, the safe side, until the agent is restarted. Touches `runner/persist.rs` (also anchored by wk-honestbench's items).
