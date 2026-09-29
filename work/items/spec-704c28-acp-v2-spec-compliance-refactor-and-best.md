+++
id = "spec-704c28"
kind = "spec"
title = "ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "hermes"
subsystem = ["roko-acp"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/18-acp-spec-upgrade-and-refactor.md#18 — ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX"
discovered_from = "audit:tmp/backlog/archive/18-acp-spec-upgrade-and-refactor.md#18 — ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX"
anchors = ["crates/roko-acp/src/types.rs:10", "crates/roko-acp/src/types.rs::AgentCapabilities", "crates/roko-acp/src/handler.rs::run_acp_server", "crates/roko-acp/src/session.rs::SessionManager", "crates/roko-acp/src/acp_adapter.rs::AcpAdapter", "crates/roko-acp/tests/protocol_conformance.rs"]
links = { depends_on = [], blocks = [], related = ["gap-83d081", "find-b2ae71"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '\"session/delete\"' crates/roko-acp/src/handler.rs && grep -q '\"logout\"' crates/roko-acp/src/handler.rs && grep -q '\"authenticate\"' crates/roko-acp/src/handler.rs && ! grep -q 'ACP_SPEC_VERSION: &str = \"0.12' crates/roko-acp/src/types.rs && grep -qw 'fn session_delete_removes_active_and_persisted' crates/roko-acp/tests/protocol_conformance.rs && cargo test -p roko-acp --test protocol_conformance session_delete_removes_active_and_persisted"
+++

## Problem

`roko acp` (the Agent Client Protocol server that editors such as Zed, JetBrains and Cursor spawn over stdio)
implements an old slice of the protocol:

- `crates/roko-acp/src/handler.rs` answers `initialize`, `session/new`, `session/list`, `session/load`,
  `session/prompt`, `session/config/update` / `session/set_config_option`, `session/close`, `session/resume`,
  `session/set_mode` and `session/cancel`. It has no `authenticate`, `logout`, `session/delete`, `session/fork`
  or `auth/status`. Editors that call them get `METHOD_NOT_FOUND`, and editors that key UI off capabilities
  (for example a "Delete session" button) disable those features.
- `ACP_SPEC_VERSION` is still `"0.12.2"` (`crates/roko-acp/src/types.rs:10`).
- Missing protocol features, per the source spec: `message_id` on message chunks, a stable `name` on tool
  calls, `additionalDirectories` on `session/new`, structured session/auth capabilities, elicitation, notices,
  `state_update` (running/idle/requires_action), per-cell plan updates for Graph runs, and streamed tool output
  chunks.

This item is an umbrella: backlog #18, sized XXL (4-6 weeks, 10 parts). Some parts are done (see Current state).

## Why it matters

Goal `hermes`: the Nous/Hermes demo drives roko from an editor over ACP. An agent that looks like a legacy ACP
agent loses editor features, and spec-validating clients drop messages they cannot parse (`bug-426c9d`).
Related: `gap-83d081` (narrow tracker for the spec-version bump), `gap-ac78fb` (elicitation and notices, Parts
3-4 here), `bug-426c9d` (non-spec session updates are dropped), `bug-f0f108` (sustained-load residue in the same
bridge), `find-b2ae71` (superseded duplicate about 0.12 vs 0.13).

## Where

- `crates/roko-acp/src/types.rs`: hand-rolled protocol types (roko does not use the `agent-client-protocol`
  crate). `ACP_PROTOCOL_VERSION = 1` (line 7) is what `initialize` negotiates. `ACP_SPEC_VERSION` (line 10) is used
  only in a log line (`handler.rs:646`). Also here: `ClientCapabilities` (line 134), `AgentCapabilities`
  (`load_session`, `prompt_capabilities`, `mcp_capabilities` only), `SessionUpdate` (11 variants, around lines
  640-732) and `ToolCall` (has `locations`, no `name`).
- `crates/roko-acp/src/handler.rs`: `run_acp_server` loop (line ~141) and the method `match` (lines ~279-480).
  `initialize` advertises `auth_methods: Vec::new()`.
- `crates/roko-acp/src/session.rs`: `SessionManager`. Sessions persist to `.roko/sessions/<session_id>.json`
  (`persist_session`, line 1374). There are `close_session` (line 1453) and `gc_old_sessions` (line 1462), but no
  `delete_session` or `fork_session`.
- `crates/roko-acp/src/bridge_events/`: prompt dispatch, streaming, permissions, cost, slash commands.
- `crates/roko-acp/src/acp_adapter.rs`: `AcpAdapter`, an `EventConsumer` that maps `RuntimeEvent`s to ACP events.
- `crates/roko-acp/src/runner.rs`: the ACP workflow runner (`run_with_workflow_engine`, `emit_plan_update`).
- Tests: `crates/roko-acp/tests/protocol_conformance.rs` spawns the server with a `TestClient`.

## Current state

Checked at HEAD, by part of the source spec:

- Part 1, spec bump and new methods: not done. See Problem.
- Part 2, split `bridge_events.rs`: mostly done in `244f564e1`. It is now a 12-file module. `mod.rs` is still
  1,085 lines (target under 1,000), and 8 `#[allow(clippy::too_many_arguments)]` remain (`dispatch.rs` 5,
  `runner.rs` 2, `cost.rs` 1).
- Parts 3-4, elicitation and notices: not done. They are tracked in `gap-ac78fb`. The outbound request transport
  they need exists (`transport.rs::send_request`).
- Part 5, `state_update`: not done.
- Part 6, plan updates: partial. The ACP workflow runner emits `SessionUpdate::Plan` for pipeline phases
  (`runner.rs::emit_plan_update`). Nothing emits per-cell plan updates for Graph plan runs.
- Part 7, tool cards: `locations` done (`types.rs:667-669`). No `name` field and no streamed content chunks.
  `acp_adapter.rs` maps 53 `RuntimeEvent` variants, but `AcpAdapter::new` is never called outside that file's unit
  tests, so none of this mapping is live.
- Part 8, usage: done. `SessionUpdate::UsageUpdate { used, size, cost }` is sent after provider responses
  (`bridge_events/mod.rs:862`), with the model's context window as `size`. `BudgetStatusUpdate` is kept as a roko
  extension. `bug-426c9d` says spec clients drop it.
- Part 9, session titles: 9.1 done (a title from the first prompt, `bridge_events/mod.rs:1054-1060`); 9.2 (update
  the title on context change) not done.
- Part 10, integration gaps: no ACP lifecycle `RuntimeEvent` variants, no `crates/roko-serve/src/routes/acp.rs`,
  and no ACP sessions in `roko dashboard`. `force_backend` still exists in `roko-agent`/`roko-cli` but not in
  roko-acp; whether Gap 1 still applies is unknown.
- Unknown: the current ACP version. The sources disagree ("0.13" in `gap-83d081`; "v1 stable, schema 1.7.0+, Aug
  2026" and "v2 draft, 2026-07-20" in backlog #18), and none of them was checked against agentclientprotocol.com.

## Plan

1. Read the current ACP schema and pick the target version. Record the method names and capability shapes in the
   PR. Where this spec and the schema disagree, follow the schema.
2. Deliver Part 1 as this item's core:
   - `authenticate`: roko advertises no `authMethods`, so return success.
   - `logout`: a no-op success unless roko gains stored credentials.
   - `session/delete`: add `SessionManager::delete_session(id)`, which cancels the session, removes it from the
     active map and deletes `.roko/sessions/<id>.json`. Reject ids that are not a plain session-id token before
     building the path.
   - `session/fork`: optional; copy the history into a new session.
   - Capabilities: advertise the new session/auth capabilities in the `initialize` result as the schema shapes them.
   - `message_id` on `AgentMessageChunk`/`AgentThoughtChunk`, `name` on `ToolCall`/`ToolCallUpdate`.
   - `additionalDirectories` on `session/new`, threaded into the session's allowed roots.
   - Bump `ACP_SPEC_VERSION`. Keep `protocolVersion` negotiation backward compatible with clients that send 1.
3. Add conformance tests to `tests/protocol_conformance.rs`, including
   `session_delete_removes_active_and_persisted`, plus `authenticate`, `logout` and an unknown-session delete.
4. Leave Parts 3-4 to `gap-ac78fb`. File new items (`python3 tools/work.py new`) for: Part 5 (`state_update`);
   Part 6 (per-cell plan updates for Graph runs, which means wiring `AcpAdapter`, so do `bug-f0f108` first);
   Part 7 (tool `name`, streamed tool output); Part 9.2; Part 10 (ACP lifecycle events, serve route, dashboard).
   Link them here under `links.related`.
5. Finish Part 2 while you are in `bridge_events/mod.rs`: get it under 1,000 lines and remove the
   `too_many_arguments` allows with parameter structs. Optional.
6. Close this umbrella when Part 1 has landed and the follow-ups are filed.

## Done when

- `authenticate`, `logout` and `session/delete` answer per the schema. Delete removes both the in-memory session
  and its file, and an invalid id is rejected without touching the filesystem.
- `ACP_SPEC_VERSION` names the schema version actually targeted, and `initialize` advertises matching
  capabilities.
- The new conformance tests pass, along with `cargo clippy -p roko-acp --no-deps -- -D warnings`.
- Follow-up items exist for the parts left open.
- Verify: `grep -q '"session/delete"' crates/roko-acp/src/handler.rs && grep -q '"logout"' crates/roko-acp/src/handler.rs && grep -q '"authenticate"' crates/roko-acp/src/handler.rs && ! grep -q 'ACP_SPEC_VERSION: &str = "0.12' crates/roko-acp/src/types.rs && grep -qw 'fn session_delete_removes_active_and_persisted' crates/roko-acp/tests/protocol_conformance.rs && cargo test -p roko-acp --test protocol_conformance session_delete_removes_active_and_persisted`

## Notes

- Security: `session/delete` turns a client-supplied id into a file deletion under `.roko/sessions/`. Validate
  the id, and never follow `..` or path separators.
- Keep roko-only fields under `_meta`, and keep spec message shapes exact (`bug-426c9d`). Do not add more non-spec
  `SessionUpdate` variants.
- Overlaps: `gap-83d081` has its own verify, which needs `ACP_SPEC_VERSION` of 0.13 or later and a
  `spec_schema_conformance` test. Do the version bump once, and close or re-scope `gap-83d081` in the same change.
- Parallel safety: this item, `gap-ac78fb`, `gap-83d081`, `bug-426c9d` and `bug-f0f108` all edit
  `crates/roko-acp/src/{types.rs,handler.rs,bridge_events/}`. Run only one of them at a time.
- The source's "roko acp connects with Zed, Cursor and JetBrains" needs a manual editor check. Record which editor
  and version were tried.

## Original notes

spec compliance, editor compatibility, UX expressiveness. Roko includes `roko-acp`, a 19,915-line crate implementing the Agent Client Protocol (ACP) — the JSON-RPC protocol used by Zed, JetBrains IDEs, Cursor, Neovim, and Devin Desktop to interact with AI agents. When an editor wants roko to…

Imported without verification from:
- `tmp/backlog/archive/18-acp-spec-upgrade-and-refactor.md#18 — ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-ACP-1 (Subsystem: ACP)`

Some cited files are gone: `session/list`, `session/new`.

How to verify: Check: `ACP_SPEC_VERSION` is `"2.0.0-draft"` in `types.rs`; `session/delete` handler wired; removes from active map and disk; `logout` handler wired [evidence: CONSOLIDATED P1-ACP-1: open; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): XL | 6 |]

Verified 2026-09-28: still open. Changed p0 -> p1 to match its CONSOLIDATED P1-ACP-1 source; a spec upgrade is not a broken core loop. `ACP_SPEC_VERSION` is still 0.12.2 (crates/roko-acp/src/types.rs:10). The handler serves initialize and session/new, load, list, resume, close, prompt, cancel, set_mode, set_config_option and config/update, but not `session/delete`, `logout` or `authenticate`. The narrower tracker is gap-83d081.
