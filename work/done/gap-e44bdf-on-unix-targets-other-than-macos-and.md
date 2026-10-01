+++
id = "gap-e44bdf"
kind = "gap"
title = "On Unix targets other than macOS and Linux, orphan cleanup never kills registered agents"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-agent/process"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-agent/src/process/identity.rs::process_identity", "crates/roko-agent/src/process/registry.rs::cleanup_orphaned_agents"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -qE "target_os = \"(freebsd|openbsd|netbsd)\"" crates/roko-agent/src/process/identity.rs'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:52Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:35Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`process_identity` returns `None` on every target except macOS and Linux (identity.rs:114-117). Orphan cleanup skips any registered PID whose identity it cannot read (registry.rs:327). That is the safe choice, but it means orphaned agents are never cleaned up on FreeBSD and other Unix targets.

Fix: add an identity probe for those targets, or document the limitation.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- FreeBSD, OpenBSD, NetBSD and DragonFly now read identities from `ps -o lstart= -o ppid= -o comm=` in the C locale (`ps_identity`): the start fingerprint is the printed start time as `YYYYMMDDhhmmss`, `started_at_ms` its local-time epoch. Native `sysctl` probes per BSD could not be compiled or tested here, while `ps` behaves the same on macOS and Linux, so `ps_identity_agrees_with_the_native_probe` checks it against the native probes there; `ps_identity_parses_bsd_ps_output` pins the parser. Other targets still report no identity.
