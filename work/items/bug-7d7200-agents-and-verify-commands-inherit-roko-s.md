+++
id = "bug-7d7200"
kind = "bug"
title = "Agents and verify commands inherit roko's whole environment, including provider API keys"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["roko-gate/shell"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "tmp/cybernetic-harness/assessment-2026-09-28/s10-s11-demo-deploy.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s10-s11-demo-deploy.md"
anchors = ["crates/roko-gate/src/gate_env.rs:39", "crates/roko-core/src/child_env.rs", "crates/roko-agent/src/process/env.rs::apply_credential_scrub", "crates/roko-agent/src/claude_cli_agent.rs:440"]
links = { depends_on = [], blocks = [], related = ["bug-7eef96"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'env_clear()' crates/roko-gate/src/gate_env.rs && grep -q 'fn gate_env_keeps_allowlist_and_drops_everything_else' crates/roko-core/src/child_env.rs && grep -q 'apply_credential_scrub' crates/roko-agent/src/claude_cli_agent.rs && cargo test -p roko-core --lib child_env"

[closed]
at = 2026-09-29
commit = "dc99a9e81"
by = "reconcile of the portal session's merges 2026-09-29 (static check)"
evidence = "dc99a9e81 (merged in 1d923e377): gate commands start from env_clear() plus an allowlist (crates/roko-gate/src/gate_env.rs:39; rules in crates/roko-core/src/child_env.rs, tests gate_env_keeps_allowlist_and_drops_everything_else and gate_env_drops_secret_looking_names_even_under_allowed_prefixes). Provider CLIs get a credential scrub (process/env.rs apply_credential_scrub; called from claude_cli_agent.rs:440, cursor_cli_agent.rs:271 and exec.rs:512)."
+++
roko loads `~/.roko/.env` and `<workdir>/.roko/.env` into its own process at startup (`main.rs:4502-4517`); those files hold provider API keys. Child processes then inherit everything:
- `ShellGate` builds verify commands with `Command::new` and adds only `CARGO_TARGET_DIR` and `extra_env` (`roko-gate/src/shell.rs:92`). There is no `env_clear` anywhere in roko-gate.
- The Claude CLI agent removes only `CLAUDECODE` (`claude_cli_agent.rs:420`).

So an agent-written test, or the agent itself, can read every provider key (and a serve admin key when one is set) and spend on providers the run never chose. The repo is public, so this is a security issue; for benchmarks it also breaks cost ledgers and hidden-test secrecy.

Fix: build child environments from an allowlist (PATH, HOME, toolchain variables, and only the keys a provider explicitly needs), for gates and agents alike. The verify command is a proxy; replace it with a test once the allowlist exists.

Checked 2026-09-29 at f99e45dba: unchanged. Seen again in 63bdfa9a3: a roko-cli unit test (hint_for_unconfigured_provider_runs_same_slug_then_default_model) failed during a verify step because OPENAI_API_KEY inherited from ~/.roko/.env made gpt-4o dispatch to the live OpenAI API; the test was patched to shadow the providers, the leak itself remains.

Checked 2026-09-29: Gates use env_clear plus an allowlist. Provider CLIs use a credential scrub, a denylist of recognised credential names, not env_clear, so a key under an unrecognised variable name would still reach an agent. The previous verify expected env_clear in claude_cli_agent.rs and a test that was never written.
