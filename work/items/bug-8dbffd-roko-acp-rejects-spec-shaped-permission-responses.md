+++
id = "bug-8dbffd"
kind = "bug"
title = "roko acp rejects spec-shaped permission responses"
status = "open"
triage = "verified"
severity = "p1"
size = "S"
goal = "hermes"
subsystem = ["roko-acp/permissions"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-10-01
last_verified_rev = "a17d9d766"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/types.rs::PermissionOutcome", "crates/roko-acp/src/types.rs::PermissionResponse", "crates/roko-acp/src/bridge_events/permissions.rs::request_permission"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn permission_response_accepts_spec_outcome_shape' crates/roko-acp/ && cargo test -p roko-acp permission_response_accepts_spec_outcome_shape"
+++

## Problem

When `roko acp` needs approval for a mutating tool call (`write_file`, `edit_file`, `bash`), it sends a
server-to-client `session/request_permission` request and parses the client's reply as `PermissionResponse`.
`PermissionOutcome` only accepts a roko-specific shape, with the discriminator in a `type` field:

```json
{"outcome": {"type": "selected", "optionId": "allow_once"}}
```

The ACP spec (`RequestPermissionResponse` / `RequestPermissionOutcome`) puts the discriminator in a field
named `outcome`:

```json
{"outcome": {"outcome": "selected", "optionId": "allow_once"}}
{"outcome": {"outcome": "cancelled"}}
```

A client built on the spec SDK sends the second shape. `serde_json::from_value::<PermissionResponse>` fails
(missing field `type`), the handler logs "permission response could not be parsed; defaulting to Reject", and
the tool call is denied. Actual: every approval from a spec client is rejected. Expected: `selected` with a
known option id maps to its decision, and `cancelled` maps to Reject.

## Why it matters

- Goal `hermes`. Hermes and other editor clients speak the spec. Rejecting every approval means an ACP session
  cannot write files or run commands with a spec client, which breaks the demo's main path.
- It fails closed (deny), so this is a functional break, not a safety hole.
- Related: `bug-426c9d` (roko acp emits session updates that spec ACP clients drop), which is the same class of
  spec-conformance defect in `crates/roko-acp/src/types.rs`.

## Where

Entry point: an ACP session prompt whose model calls a mutating builtin tool ->
`AcpBuiltinToolHandler::execute` sends `CognitiveEvent::PermissionRequest` -> `request_permission_for_event`
-> `request_permission`.

- `crates/roko-acp/src/types.rs::PermissionOutcome` (`:1172-1184`): `#[serde(rename_all = "camelCase",
  tag = "type")]` enum with `Cancelled` and `Selected { #[serde(alias = "optionId")] option_id }`. Note that
  `rename_all` on an enum renames variants only, so it serializes the field as `option_id`, not `optionId`.
- `crates/roko-acp/src/types.rs::PermissionResponse` (`:1164-1170`): `{ outcome: PermissionOutcome }`.
- `crates/roko-acp/src/types.rs::PermissionOptionKind::decision_from_option_id` (`:1136`): maps
  `allow_once` -> Allow, `allow_always` -> AlwaysAllow, `reject_once`/`reject_always` -> Reject, anything
  else -> `None` (the caller treats that as Reject).
- `crates/roko-acp/src/bridge_events/permissions.rs::request_permission` (`:18`; parse at `:89-106`): sends the
  request with a 30 s timeout and maps the parsed outcome to a `PermissionDecision`. On `AlwaysAllow` it
  persists the grant with `AcpSession::save_workspace_trust`.
- Tests: `crates/roko-acp/src/types.rs` unit test around `:1315-1345` (round trip plus a "what Zed actually
  sends" case using the `type` shape); `crates/roko-acp/src/bridge_events/tests.rs` uses the helper
  `reply_to_permission_request(client, json)` with an in-memory `duplex` transport (`:1343-1372`, `:1431`,
  `:2198`), all with the `type` shape.
- Docs: `docs/v2/ACP-INTEGRATION-GUIDE.md` "Mutation tool permission flow" (`~:800-814`) lists the option ids
  but not the response shape.

## Current state

- At HEAD only the `type`-tagged shape parses. The unit-test comment claims Zed sends `type`; the spec schema
  and the spec SDK use `outcome`. Nothing in the repo sends or parses the spec shape.
- The `type`-tagged shape dates from `fb27bdea3` (2026-05-05) and has not changed since; later commits on these
  files (`310860465`, `244f564e1`) did not touch it.
- The request side (`RequestPermissionParams`, `PermissionOption { optionId, name, kind }`) already uses spec
  field names; this item is only about the response.

## Plan

1. Make `PermissionOutcome` accept both shapes. Options:
   - (a) Recommended: keep the public enum, derive `Serialize` with the spec shape
     (`#[serde(tag = "outcome", rename_all = "snake_case")]`, field `#[serde(rename = "optionId")]`), and
     write a manual `impl<'de> Deserialize<'de> for PermissionOutcome` that reads a private helper struct
     `RawPermissionOutcome { #[serde(alias = "type")] outcome: String, #[serde(rename = "optionId",
     alias = "option_id")] option_id: Option<String> }`. Map `"selected"` (requires `option_id`) and
     `"cancelled"`; any other value is an error. Clear errors, and one place to add future variants.
   - (b) `#[serde(untagged)]` over two tagged enums (spec and legacy). Less code, but serde's untagged errors
     are opaque ("data did not match any variant").
2. Keep the fail-closed behaviour in `request_permission`: an unparseable reply, an unknown option id, or
   `cancelled` still returns `PermissionDecision::Reject`.
3. Tests:
   - unit test `permission_response_accepts_spec_outcome_shape` (in `types.rs` tests or
     `bridge_events/tests.rs`): the spec `selected` shape parses to `Selected { option_id: "allow_once" }`;
     `{"outcome":{"outcome":"cancelled"}}` parses to `Cancelled`; the legacy `type` shape still parses;
     serializing `Selected` produces `{"outcome":"selected","optionId":...}`.
   - end-to-end: extend the `reply_to_permission_request` tests with a spec-shaped `allow_once` reply ->
     `PermissionDecision::Allow`, and a spec `cancelled` reply -> `Reject`.
   - golden files if you want them (the original note asked for them): put spec JSON samples under
     `crates/roko-acp/tests/fixtures/` and load them with `include_str!`.
4. Fix the misleading "what Zed actually sends" comment, and add the response shape to the permission section of
   `docs/v2/ACP-INTEGRATION-GUIDE.md`.

## Done when

- A spec-shaped `selected` reply with `optionId: "allow_once"` produces `PermissionDecision::Allow`, and
  `allow_always` produces `AlwaysAllow` and persists the trust grant.
- A spec-shaped `cancelled` reply produces `Reject`.
- The legacy `type`-tagged shape still parses (existing tests pass unchanged).
- Verify:
  `grep -rqw 'fn permission_response_accepts_spec_outcome_shape' crates/roko-acp/ && cargo test -p roko-acp permission_response_accepts_spec_outcome_shape`

## Notes

- Do not loosen the fail-closed default: parse failures and unknown option ids must still deny.
- `allow_always` writes `.roko/trust/permissions.json` through `AcpSession::save_workspace_trust`; the tests
  should use a tempdir workdir, as the existing always-allow test does.
- Small, self-contained change in `types.rs` plus tests. Safe to run in parallel with most work. Expect a small
  merge overlap with `bug-426c9d` if both edit `crates/roko-acp/src/types.rs` at the same time.
- The verify command is sound; no change needed.
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  `PermissionOutcome` now serializes in the spec shape and has a manual `Deserialize` that reads the spec `outcome`
  or the legacy `type` discriminator (`optionId` or `option_id`); unknown outcomes and a selection without an option
  id are errors, so the handler still rejects. Tests: `types.rs::permission_response_accepts_spec_outcome_shape` and
  `bridge_events/tests.rs::request_permission_accepts_spec_shaped_responses` (allow_once, allow_always with the trust
  file, cancelled); the guide documents the response shape.

## Original notes

`PermissionOutcome` deserializes with `#[serde(rename_all = "camelCase", tag = "type")]` (`roko-acp/src/types.rs:1174`), i.e. `{"type":"selected","optionId":..}`.
Clients built on the ACP spec SDK answer `session/request_permission` with `{"outcome":{"outcome":"selected","optionId":"allow_once"}}`, which fails to parse, so every shell/write/edit approval from a spec client is rejected.
Fix: accept the spec shape (keep the legacy shape as an alias) and add golden-file conformance tests for permission responses.
