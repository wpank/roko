+++
id = "find-3174bd"
kind = "finding"
title = "Security-Off-By-Default"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve"]
created = 2026-04-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S5. Security-Off-By-Default"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S5. Security-Off-By-Default"
anchors = ["crates/roko-serve/src/terminal.rs::CreateSessionRequest", "crates/roko-serve/src/terminal.rs:962", "crates/roko-serve/src/routes/mod.rs:305", "crates/roko-cli/src/agent_exec.rs:151", "crates/roko-serve/src/dispatch.rs::build_agent", "crates/roko-agent/src/claude_cli_agent.rs:126"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Systemic audit finding (2026-04-28) with 10 open checklist fixes: S5.1 Enable auth by default (or auto-enable on 0; S5.2 Move terminal routes inside auth middleware; S5.3 Warn/block when PORT is set without auth en; S5.4 Restrict default CORS to localhost; S5.5 Default to private gists, run…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S5. Security-Off-By-Default`

How to verify: Check each open sub-item (S5.1, S5.2, S5.3, S5.4, S5.5, S5.6, S5.7, S5.8, S5.9, S5.10). S5.1/S5.4/S5.5 likely closed by CONSOLIDATED P1-SRV-1 (auth default), P1-SRV-2 (CORS), P1-SEC-2 (gist scrubbing); verify S5.2-5.3 and S5.6-5.10 individually.

Verified 2026-09-28: 7 of the 10 checklist items are addressed and 3 remain. Addressed: S5.1 auth is on by default (crates/roko-core/src/config/serve.rs:197, enabled: true). S5.2 terminal routes require auth and scope even on loopback (crates/roko-serve/src/routes/mod.rs:422). S5.3 PORT keeps a loopback bind unless public bind is opted into, and serve warns on a public bind without auth (crates/roko-serve/src/lib.rs:284, :860). S5.4 CORS has a localhost-origin predicate (routes/middleware.rs:1473); which policy is chosen by default was not fully traced. S5.5 is obsolete because no gist or --share code remains. S5.9 post-dispatch violations use ViolationSeverity::Block (crates/roko-agent/src/safety/mod.rs:1020-1090). S5.10 AgentContract::permissive is #[cfg(test)] (safety/contract.rs:185). Still open: S5.6, the request's `command` goes straight to the PTY with no allowlist (terminal.rs:131, :962); this is mitigated because the terminal is off by default (config/serve.rs:92) and sits behind auth. S5.7, there is no cap on concurrent PTY sessions and no TTL, only a creation rate limiter (routes/mod.rs:304). S5.8, dangerously_skip_permissions: true is still hardcoded in agent_exec.rs:151 (run_agent_capture_impl), roko-serve dispatch.rs:2032 (build_agent) and the ClaudeCliAgent constructor (claude_cli_agent.rs:124). Severity lowered p1 to p2: the network-facing defaults (auth, CORS, bind) are fixed, and the terminal gaps apply only when an operator turns the terminal on.

Re-checked 2026-09-29: S5.1-S5.5, S5.9 and S5.10 remain addressed; S5.6, S5.7 and S5.8 remain open. S5.6: the request's command still goes straight to the PTY (crates/roko-serve/src/terminal.rs:962 -> create_session_inner :727-742), and the caller-supplied workdir is not validated either. S5.7: there is still no concurrent-session cap or TTL, only the creation rate limiter (crates/roko-serve/src/routes/mod.rs:305). S5.8: dangerously_skip_permissions: true is still hardcoded at crates/roko-cli/src/agent_exec.rs:151, crates/roko-serve/src/dispatch.rs:2036 (build_agent) and crates/roko-agent/src/claude_cli_agent.rs:126 (ClaudeCliAgent::new).
