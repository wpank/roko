+++
id = "bug-e5aef1"
kind = "bug"
title = "`roko acp` never sets the agent PID registry root, so its agents register under the current directory"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
subsystem = ["roko-cli/main", "roko-acp"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-cli/src/main.rs::main", "crates/roko-agent/src/process/registry.rs::set_registry_root", "crates/roko-acp/src/handler.rs::run_acp_server"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "set_registry_root" crates/roko-acp/src || sed -n "/ACP early exit/,/run_acp_server/p" crates/roko-cli/src/main.rs | grep -q set_registry_root'
+++

`main` calls `roko_agent::process::set_registry_root(&workdir)` after the logging setup (main.rs:3397). The ACP early exit (main.rs:3303-3340) runs before that point and roko-acp does not set the root either, so agents spawned by `roko acp --workdir X` are registered under the process's current directory, which the registry uses until a root is set. Registry-based cleanup for workspace X cannot find them.

Fix: set the registry root from the ACP `--workdir` in the early-exit block, or in roko-acp's server start.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- `run_acp_server_inner` (crates/roko-acp/src/handler.rs) keys the PID registry to the canonical `--workdir` right after creating `.roko/`, before any agent spawns. It sits in the stdio entry point rather than `run_acp_server_with_transport`, so the tests that drive the server through an injected transport do not move the process-wide registry root. There is no unit test: the root is process-global and has no reader, and this entry point reads the real stdin. The verify command greps for the call.
