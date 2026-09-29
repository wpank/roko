+++
id = "gap-ac78fb"
kind = "gap"
title = "ACP Elicitation, Notices, and Interactive UX Primitives"
status = "open"
triage = "verified"
severity = "p2"
size = "L"
goal = "hermes"
subsystem = ["roko-acp"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/433-acp-elicitation-notices-interactive-ux.md#433 — ACP Elicitation, Notices, and Interactive UX Primitives"
discovered_from = "audit:tmp/backlog/archive/433-acp-elicitation-notices-interactive-ux.md#433 — ACP Elicitation, Notices, and Interactive UX Primitives"
anchors = ["crates/roko-acp/src/transport.rs::StdioTransport::send_request", "crates/roko-acp/src/bridge_events/permissions.rs::request_permission", "crates/roko-acp/src/types.rs::ClientCapabilities", "crates/roko-acp/src/types.rs::SessionUpdate", "crates/roko-acp/src/runner.rs::emit_plan_update", "crates/roko-acp/src/handler.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq '\"elicitation/create\"' crates/roko-acp/src && grep -rqw 'fn elicitation_create_round_trip' crates/roko-acp/ && cargo test -p roko-acp elicitation_create_round_trip"
+++

## Problem

When `roko acp` runs inside an editor (Zed, JetBrains, Cursor), roko has no way to:

- ask a structured question mid-turn (a form with options) and wait for the answer. The only interactive request
  it sends is `session/request_permission` (allow/reject/always-allow);
- ask the user to approve a generated plan before it runs;
- push operational notices (MCP server dropped, provider rate limit, budget at 80%, config reloaded) into the
  editor's notification area rather than the transcript.

Expected, per the source spec (backlog #433): an `elicitation/create` request when the client advertises
elicitation support (falling back to `session/request_permission`), plan approval through elicitation, and
`notice` session updates when the client advertises notice support.

## Why it matters

Goal `hermes`: the roko side of the Nous/Hermes demo runs through ACP in an editor. Structured questions, plan
approval and visible notices are what make that demo interactive rather than a text stream. The risk of leaving
it is UX polish, not correctness: p2.
Related: `spec-704c28` (the umbrella ACP spec; its Parts 3 and 4 are this item, so do them here),
`bug-426c9d` (spec clients silently drop non-spec session updates, so notices must be spec-shaped), `gap-83d081`
(spec version bump), `bug-f0f108` (bridge crash under load).

## Where

- `crates/roko-acp/src/transport.rs`: `StdioTransport::send_request(method, params)` sends an outbound JSON-RPC
  request and awaits the matching response through `pending_requests` (id -> `oneshot::Sender`).
  `handle_incoming_response` routes replies, and `PendingRequestGuard` cleans up on cancel.
- `crates/roko-acp/src/bridge_events/permissions.rs`: `request_permission` (line 18) is the working pattern to copy.
  It clones the transport, calls `send_request("session/request_permission", …)`, `select!`s against a 30 s sleep,
  and fails closed to `PermissionDecision::Reject` on error, timeout or an unparsable reply.
  `request_permission_for_event` is called from `bridge_events/mod.rs:191`.
- `crates/roko-acp/src/types.rs`: `ClientCapabilities` (line 134) parses only `fs`, `terminal` and `mcp_servers`,
  with no elicitation or notice capability. `SessionUpdate` (around lines 640-732) has 11 variants and no `Notice`.
  `AgentCapabilities` has no elicitation field.
- `crates/roko-acp/src/runner.rs`: the ACP workflow runner (`/develop`-style pipelines). `emit_plan_update` (line
  ~1173) sends `CognitiveEvent::PlanUpdate`. This is where a plan-approval elicitation would go, before execution.
- `crates/roko-acp/src/handler.rs`: the `initialize` handler (line ~279) builds `AgentCapabilities`.
- Notice sources to wire: `crates/roko-acp/src/config_watch.rs` (config reload), the MCP status path that already
  emits `McpStatusUpdate`, the budget path that emits `BudgetStatusUpdate` (`bridge_events/cost.rs`), provider
  rate-limit and health handling (`crates/roko-agent`).
- Entry point: `roko acp` (stdio), driven by an editor or by the test client in
  `crates/roko-acp/tests/protocol_conformance.rs`.

## Current state

- Done: the outbound request transport with response correlation (source criterion A1),
  `StdioTransport::send_request`, added in `99511019a` and used for `session/request_permission`.
- Partly done: file edits already produce `ContentBlock::Diff` tool-call content (`runner.rs:1085`, source criterion
  D2). They carry only a unified `diff` string, with `old_text`/`new_text` set to `None`. ACP's diff content
  expects `newText`, so check whether spec clients render it (see `bug-426c9d`).
- Missing: any `elicitation` text in `crates/roko-acp/src`, client/agent elicitation capabilities, plan approval in
  ACP, the `Notice` session update, a `send_notice` helper, and notice sources.
- The affect section of the source (C1: read `affect_enabled` instead of a hard-coded `false` in
  `bridge_events.rs`) is stale. That code no longer exists. ACP now reads `DaimonState` read-only for routing
  (`bridge_events/cost.rs:472-500`) and has an `/affect` slash command (`bridge_events/slash_commands.rs:258`).
- Unknown: the exact current ACP schema for elicitation and notices. The sources say ACP stabilized
  `elicitation/create` in July 2026 and the `notice` update on 2026-09-18, with a client capability
  `session.notices`. Nobody has checked this against agentclientprotocol.com. roko hand-rolls its ACP types and does
  not use the `agent-client-protocol` crate.

## Plan

1. Read the current ACP schema (agentclientprotocol.com, or the published JSON schema). Record the exact method
   names, param/result shapes and capability flags for elicitation and notices in the PR description. If either is
   not in the stable spec, send it only when the client advertises it, and put roko-only fields under `_meta`
   (the `bug-426c9d` rule).
2. Types (`types.rs`): add the client capability fields (for example `elicitation` and `session.notices`, as the
   schema names them) to `ClientCapabilities`, `ElicitationCreateParams`/`ElicitationCreateResult` (accept, decline
   and cancel), the agent-side capability, and a `SessionUpdate::Notice { severity, title, description }` variant
   with a `NoticeSeverity` enum. Store the parsed client capabilities on the session at `initialize`/`session/new`.
3. Elicitation helper: add `request_elicitation` next to `request_permission`, with the same pattern (cloned
   transport, `send_request("elicitation/create", …)`, `select!` with a timeout, fail closed). Use a 120 s
   default, configurable. Decline, cancel, timeout and error all mean "no".
4. Tool permission: when the client supports elicitation, ask "Allow X?" plus "Always allow?" through elicitation
   and map the answer onto `PermissionDecision`, including `grant_always_allow`. Otherwise keep
   `session/request_permission` exactly as it is.
5. Plan approval: in the ACP workflow runner, after a plan is generated and before execution, send an elicitation
   with the options Execute / Edit first / Confirm each step / Cancel. Without elicitation support, keep the
   current behaviour (no prompt).
6. Notices: add `send_notice` (a no-op unless the client advertised notice support), then wire at least these
   sources: MCP server disconnect, provider 429/rate limit, budget at 80 % and exceeded, config reload, provider
   unhealthy/recovered. Keep notices out of session replay and history persistence.
7. Tests in `crates/roko-acp/tests/protocol_conformance.rs`, using its `TestClient`:
   - `elicitation_create_round_trip`: the client advertises elicitation, the server sends `elicitation/create`,
     the client answers, and the decision is applied;
   - `elicitation_falls_back_to_request_permission`: the client does not advertise it;
   - `notice_not_sent_without_capability`.
8. Out of scope; file separate items if still wanted: affect notices on PAD change, model auto-escalation on
   frustration, the dream journal at session start (source section C), and multi-block progressive-disclosure tool
   cards (section D1).

## Done when

- With a client that advertises elicitation, tool permission and plan approval arrive as `elicitation/create`
  forms. With a client that does not, `session/request_permission` behaves as before.
- Notices appear only for clients that advertise support, and never in replayed history.
- The three tests above pass, along with `cargo clippy -p roko-acp --no-deps -- -D warnings`.
- Verify: `grep -rq '"elicitation/create"' crates/roko-acp/src && grep -rqw 'fn elicitation_create_round_trip' crates/roko-acp/ && cargo test -p roko-acp elicitation_create_round_trip`

## Notes

- Safety: permission handling must stay fail-closed. Any error, timeout, decline or unparsable reply means
  reject. Do not auto-approve plans when elicitation is unavailable.
- Do not change the `session/request_permission` wire format. Existing editors depend on it.
- `bug-f0f108` (sustained-load residue) rewrites the wait loop in `bridge_events/permissions.rs::request_permission_for_event`,
  the file this item extends for elicitation. Do the two one after the other, not in parallel.
- `spec-704c28` is an umbrella over the same files (`types.rs`, `handler.rs`, `bridge_events/`). Do not run both at
  once.

## Original notes

these are the UX primitives that make roko expressive and aesthetic in editors. The best AI agents in 2026 are distinguished not by what they can do, but by how expressively they communicate what they're doing. Research across Zed, Cursor, JetBrains, VS Code, and Windsurf shows five UX patterns…

Imported without verification from:
- `tmp/backlog/archive/433-acp-elicitation-notices-interactive-ux.md#433 — ACP Elicitation, Notices, and Interactive UX Primitives`

Some cited files are gone: `elicitation/create`, `session/request_permission`.

How to verify: Check: Outbound JSON-RPC request transport implemented with response correlation; `elicitation/create` sends structured forms to the editor; Plan approval uses elicitation when client supports it [evidence: 00-INDEX (2026-09-21) listed active: ACP v2, Editor UX, and MCP Modernization (#18, #39]

Verified 2026-09-28: still true. This is editor UX polish, so p1 -> p2. Nothing in crates/roko-acp/src mentions `elicitation/create`. Outbound request/response correlation does exist (the transport.rs pending-request registry, :32/:69/:229, used for `session/request_permission`), so the transport prerequisite is partly in place. No elicitation forms, notices or elicitation-based plan approval exist.
