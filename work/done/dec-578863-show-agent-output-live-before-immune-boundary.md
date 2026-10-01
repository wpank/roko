+++
id = "dec-578863"
kind = "decision"
title = "Show agent output live before the immune boundary screens it?"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-agent/safety", "roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03-backend-live-events#T07"
discovered_from = "plan:portal-programme/03-backend-live-events#T04"
anchors = ["crates/roko-agent/src/immune_boundary.rs", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch"]
links = { depends_on = [], blocks = ["gap-be0ac2", "bug-28f2b9"], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
by = "Will (recorded by team-lead); implementation confirmed by plan:portal-programme/03c-backend-local-access#T10"
run_id = "graph-03c-backend-local-access-ecb447db-1cfe-4ad3-9f93-2bc34ba59f34"
evidence = "Decision recorded: tool steps live + opt-in full text. LIVE-OUTPUT-CHECK: PASS (15 checks) confirmed the full implementation: tool steps arrive mid-task in default mode (live:true, target only, no input), unscreened text and tool results arrive in trusted mode (live:true, screened:false), the startup line announces the mode on stdout, and the screened transcript still arrives after agent_completed."
+++

The immune boundary (`crates/roko-agent/src/immune_boundary.rs`) releases only the screened final body after a turn; its `run_streaming` implementation explicitly states "Provider events are not replayed." Streaming raw provider output before the boundary screens it is a safety and trust question, not a wiring question.

Three options:

1. **Keep per-turn output** (current behaviour): `agent_output` arrives once per turn, carrying the screened final result. `agent_spawned` and `agent_heartbeat` (every 5 s) show liveness while the agent works. No unscreened content is ever exposed.
2. **Stream to the local operator under an explicit trust setting**: when the server is loopback-only and a configured trusted-operator flag is set, emit tool-call events and partial output before the immune boundary processes them. Requires explicit opt-in and a documented trust boundary.
3. **Stream tool-call names only**: emit a minimal `agent_tool_called` event for each tool invocation — no arguments, no results — giving visible progress without exposing content. Immune-boundary-compatible; lower information value.

Until this decision is recorded and closed, gap-be0ac2 (agent output forwarded only after finish) and bug-28f2b9 (output replayed in burst) are blocked: heartbeats now show liveness, but live streaming requires resolving the trust question first.

**Decided 2026-09-28 (Will): "tool steps live + opt-in full text"**, a hybrid of options 2 and 3.

- **Always:** each tool call streams as it happens, with its name and target only (for example `Write apps/x.ts`, `Read Cargo.toml`, or ``Bash `cargo test -p roko-serve` `` truncated). No arguments beyond the target, and no results.
- **Opt-in:** full live text, reasoning and tool results, before screening. Two conditions must both hold:
  - the server is loopback-only;
  - `[serve] live_agent_output = "trusted"` is set. The default is `"tool_steps"`.
  A non-loopback bind ignores the setting and logs a warning at startup.
- **Unchanged:** the screened transcript still arrives at the end of the turn and replaces any live text. The TUI Output pane shows the same live tool steps.

Implementation is planned as `plans/portal-programme/03c-backend-local-access` T12-T20; the portal side is plan 08c.
