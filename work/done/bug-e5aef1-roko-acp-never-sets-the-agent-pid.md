+++
id = "bug-e5aef1"
kind = "bug"
title = "`roko acp` never sets the agent PID registry root, so its agents register under the current directory"
status = "done"
triage = "verified"
severity = "p3"
goal = "hermes"
subsystem = ["roko-cli/main", "roko-acp"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-cli/src/main.rs::main", "crates/roko-agent/src/process/registry.rs::set_registry_root", "crates/roko-acp/src/handler.rs::run_acp_server"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "set_registry_root" crates/roko-acp/src || sed -n "/ACP early exit/,/run_acp_server/p" crates/roko-cli/src/main.rs | grep -q set_registry_root'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:26Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T17:37:08Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`main` calls `roko_agent::process::set_registry_root(&workdir)` after the logging setup (main.rs:3397). The ACP early exit (main.rs:3303-3340) runs before that point and roko-acp does not set the root either, so agents spawned by `roko acp --workdir X` are registered under the process's current directory, which the registry uses until a root is set. Registry-based cleanup for workspace X cannot find them.

Fix: set the registry root from the ACP `--workdir` in the early-exit block, or in roko-acp's server start.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- `run_acp_server_inner` (crates/roko-acp/src/handler.rs) keys the PID registry to the canonical `--workdir` right after creating `.roko/`, before any agent spawns. It sits in the stdio entry point rather than `run_acp_server_with_transport`, so the tests that drive the server through an injected transport do not move the process-wide registry root. There is no unit test: the root is process-global and has no reader, and this entry point reads the real stdin. The verify command greps for the call.
