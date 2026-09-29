+++
id = "bug-7d7200"
kind = "bug"
title = "Agents and verify commands inherit roko's whole environment, including provider API keys"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["roko-gate/shell"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/s10-s11-demo-deploy.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s10-s11-demo-deploy.md"
anchors = ["crates/roko-gate/src/shell.rs:92", "crates/roko-agent/src/claude_cli_agent.rs:420", "crates/roko-cli/src/main.rs:4502", "crates/roko-cli/src/graph_task_dispatch.rs:1778"]
links = { depends_on = [], blocks = [], related = ["bug-7eef96"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test \"$(grep -rho env_clear crates/roko-gate/src | wc -l)\" -gt 0"
+++
roko loads `~/.roko/.env` and `<workdir>/.roko/.env` into its own process at startup (`main.rs:4502-4517`); those files hold provider API keys. Child processes then inherit everything:
- `ShellGate` builds verify commands with `Command::new` and adds only `CARGO_TARGET_DIR` and `extra_env` (`roko-gate/src/shell.rs:92`). There is no `env_clear` anywhere in roko-gate.
- The Claude CLI agent removes only `CLAUDECODE` (`claude_cli_agent.rs:420`).

So an agent-written test, or the agent itself, can read every provider key (and a serve admin key when one is set) and spend on providers the run never chose. The repo is public, so this is a security issue; for benchmarks it also breaks cost ledgers and hidden-test secrecy.

Fix: build child environments from an allowlist (PATH, HOME, toolchain variables, and only the keys a provider explicitly needs), for gates and agents alike. The verify command is a proxy; replace it with a test once the allowlist exists.
