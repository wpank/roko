# ACP: Spec Version Bump (v0.12.2 -> v0.13.6)

> **Source**: `crates/roko-acp/src/types.rs`, ACP changelog
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`
> **Created**: 2026-08-15

## Current State

- Codebase declares: `ACP_SPEC_VERSION = "0.12.2"` (line 9 of `crates/roko-acp/src/types.rs`)
- Protocol version: `ACP_PROTOCOL_VERSION = 1` (unchanged across all 0.12.x / 0.13.x releases)
- Latest stable crate: `agent-client-protocol-schema v1.6.0` (2026-07-21)
- Latest stable spec release: `schema-v1.20.0` (2026-07-21)
- Latest changelog entry relevant to v1 stabilization: v0.13.6 (2026-06-05)

The 0.12.2 -> 0.13.6 gap spans **7 releases** over ~6 weeks (2026-04-23 to 2026-06-05).
Wire protocol version stays at `1`; all changes are additive schema-level stabilizations.

## Existing Implementation vs Required

Our `types.rs` already has partial coverage of some 0.13.x features (session/close is
handled, session/resume is handled via load_session path). The gaps are mostly about
missing capability advertisement, missing method handlers, and missing type definitions.

---

## Changes Required

### v0.12.2 (2026-04-23) -- Current Baseline

**Stabilized**:
- `session/close` (#1062) -- **Already implemented** in handler.rs
- `session/resume` (#1051) -- **Already implemented** via `session/load` fallback in handler.rs

No code changes needed. This is our current declared version.

---

### v0.13.0 (2026-05-12)

**Added**:
- Experimental MCP-over-ACP message types (#1185, #1173)
  - New method types: `mcp/connect`, `mcp/message`, `mcp/disconnect`
  - Behind `unstable_mcp_over_acp` feature flag in the official crate

**Other**:
- v2 schema scaffolding started (#1099) -- not relevant to v1
- Module reorganized to `v1` module (#1094) -- internal to upstream crate only

| What needs updating | File | Change | Effort |
|---|---|---|---|
| Bump `ACP_SPEC_VERSION` to `"0.13.0"` (interim) | `types.rs:9` | Constant update | Trivial |
| Add MCP-over-ACP types (optional, unstable) | `types.rs` | New structs: `McpConnectParams`, `McpMessageParams`, `McpDisconnectParams` | Medium |
| Add MCP-over-ACP handlers (optional, unstable) | `handler.rs` | New match arms for `mcp/connect`, `mcp/message`, `mcp/disconnect` | Medium |

**Verdict**: The MCP-over-ACP types are unstable and optional. Skip for now unless we want
to act as an MCP gateway. The version bump alone is safe.

---

### v0.13.1 (2026-05-16)

**Added**:
- Unstable `session/delete` support (#1216)
  - New method: `session/delete` (removes a session from `session/list` results)
  - Behind unstable feature flag at this version

| What needs updating | File | Change | Effort |
|---|---|---|---|
| Add `SessionDeleteParams` type | `types.rs` | New struct with `session_id: String` | Trivial |
| Add `session/delete` handler (unstable at this version) | `handler.rs` | New match arm, calls `sessions.delete_session()` | Small |
| Add `SessionManager::delete_session()` | `session.rs` | Remove session from active map + persisted storage | Small |

**Verdict**: Preparation for stabilization in v0.13.6. Worth adding now.

---

### v0.13.2 (2026-05-17)

**Fixed**:
- Updated `additionalDirectories` guidance (#1227)
  - Clarified that `additionalDirectories` must contain absolute paths
  - Clarified that omission on load/resume does not restore previous roots
  - No new types; documentation/behavior clarification only

| What needs updating | File | Change | Effort |
|---|---|---|---|
| No structural changes required | -- | Behavioral guidance only | None |

**Verdict**: No code changes. Informational only.

---

### v0.13.3 (2026-05-22)

**Added**:
- **Stabilize `logout` method** (#1273)
  - New method: `logout` (ends authenticated state)
  - New capability: `auth.logout` in `AgentCapabilities`
  - New types: `LogoutRequest` / `LogoutResponse`, `AuthCapabilities`, `LogoutCapabilities`

**Fixed**:
- Renamed provider method types to singular (#1272) -- unstable, not relevant

**Other**:
- `additionalDirectories` RFD moved to Preview (#1276)
- Set minimum supported Rust version (MSRV) (#1232)
- Documented ACP versioning semantics (#1229)

| What needs updating | File | Change | Effort |
|---|---|---|---|
| Add `AuthCapabilities` struct | `types.rs` | New struct: `{ logout: Option<LogoutCapabilities> }` | Trivial |
| Add `LogoutCapabilities` struct | `types.rs` | Empty marker struct `{}` | Trivial |
| Add `auth` field to `AgentCapabilities` | `types.rs` | New field: `auth: AgentAuthCapabilities` | Trivial |
| Add `LogoutParams` type | `types.rs` | Empty struct or no params | Trivial |
| Add `logout` handler | `handler.rs` | New match arm returning `{}` (roko has no auth state to clear) | Trivial |
| Advertise `auth.logout` capability in `initialize` | `handler.rs:296` | Set capability in `InitializeResult` | Trivial |

**Verdict**: Small additive change. Implement for spec compliance even though roko
currently has no authentication flow.

---

### v0.13.4 (2026-05-27)

**Added**:
- Unstable plan operations (#1299)
  - New unstable types for plan management (not yet stabilized)
  - Relates to the `PlanCapabilities` capability key

| What needs updating | File | Change | Effort |
|---|---|---|---|
| No stable changes required | -- | Unstable only | None |

**Verdict**: Skip -- unstable feature. Our existing `PlanEntry` / `PlanStatus` types cover
the stable plan update notification shape.

---

### v0.13.5 (2026-06-01)

**Added**:
- **Stabilize `additionalDirectories` for sessions** (#1327)
  - New capability: `sessionCapabilities.additionalDirectories`
  - New field on `session/new`, `session/load`, `session/resume` params: `additionalDirectories: string[]`
  - Sessions report `additionalDirectories` in `session/list` results
  - New type: `SessionAdditionalDirectoriesCapabilities`

- Annotate lenient deserialize opportunities (#1328) -- schema annotation, no code change

- Remove unstable session model API (#1325) -- cleanup of unstable types, no v1 impact
- Remove dedicated session modes and models APIs from v2 (#1324) -- v2 only
- Add v2 enum extension RFD and fallbacks (#1304) -- v2 only

**Other**:
- Moved existing protocol docs to v1 (#1326) -- upstream docs only

| What needs updating | File | Change | Effort |
|---|---|---|---|
| Add `SessionCapabilities` struct | `types.rs` | New struct with `close`, `resume`, `delete`, `list`, `additional_directories` fields (all `Option<T>` marker structs) | Small |
| Add `session_capabilities` field to `AgentCapabilities` | `types.rs` | New field: `session_capabilities: SessionCapabilities` | Trivial |
| Add `additional_directories` field to `SessionNewParams` | `types.rs` | New optional field: `additional_directories: Option<Vec<String>>` | Trivial |
| Add `additional_directories` field to `SessionLoadParams` | `types.rs` | New optional field: `additional_directories: Option<Vec<String>>` | Trivial |
| Create `SessionResumeParams` type (distinct from load) | `types.rs` | New struct with `session_id`, `cwd`, `mcp_servers`, `additional_directories` | Small |
| Add `additional_directories` to `SessionInfo` | `types.rs` | New optional field in session list results | Trivial |
| Advertise `sessionCapabilities` in `initialize` | `handler.rs:296` | Populate `session_capabilities` with `close`, `resume`, `list`, `additional_directories` | Small |
| Pass `additional_directories` through session creation | `session.rs` | Store and expose the field | Small |

**Verdict**: This is the largest structural change. The `AgentCapabilities` struct needs
a new `session_capabilities` field that replaces the flat `load_session` boolean with a
structured `SessionCapabilities` object. This is a **breaking change** to our serialized
`InitializeResult` shape.

---

### v0.13.6 (2026-06-05)

**Added (Stable)**:
- **Stabilize optional message IDs** (#1372)
  - New optional field on streaming chunks: `messageId: string`
  - Agent-generated, opaque, unique per session
  - Allows clients to group chunks into logical messages

- **Stabilize session usage updates** (#1371)
  - New notification: `usage_update` session update type
  - Reports current context-window utilization (`used`, `size`, `cost`)
  - Formally stabilizes what we already have as `UsageUpdate` in `SessionUpdate`

- **Stabilize `session/delete`** (#1370)
  - Promotes `session/delete` from unstable to stable
  - New capability: `sessionCapabilities.delete`

**Added (Unstable v2 only -- skip)**:
- Remove MCP SSE transport, make stdio opt-in (#1368) -- v2 only
- Clean up capability objects (#1367) -- v2 only
- Require message IDs in v2 chunks (#1352) -- v2 only
- Adopt `plan_update` as v2 plan shape (#1347) -- v2 only
- Remove v2 client filesystem and terminal surface (#1346) -- v2 only

**Fixed**:
- Fix plan capability key (#1369) -- unstable plan feature

| What needs updating | File | Change | Effort |
|---|---|---|---|
| Add `message_id` field to `AgentMessageChunk` | `types.rs` | New optional field: `message_id: Option<String>` | Trivial |
| Add `message_id` field to `AgentThoughtChunk` | `types.rs` | New optional field: `message_id: Option<String>` | Trivial |
| Add `message_id` field to `ToolCall` update | `types.rs` | New optional field: `message_id: Option<String>` | Trivial |
| Generate and track message IDs per session | `session.rs` or `bridge_events.rs` | Counter or UUID per logical message, passed through streaming | Small |
| Add `sessionCapabilities.delete` to capability | `types.rs` / `handler.rs` | Set `delete: Some(SessionDeleteCapabilities {})` | Trivial |
| Add `session/delete` handler (if not done in v0.13.1) | `handler.rs` | Match arm for `session/delete` | Small |
| Verify `UsageUpdate` matches stabilized shape | `types.rs` | Compare our `UsageUpdate { used, size, cost }` vs official `UsageUpdate { used, size, cost }` | Trivial |
| Bump `ACP_SPEC_VERSION` to `"0.13.6"` | `types.rs:9` | Final constant update | Trivial |

**Verdict**: Message IDs are the most impactful addition -- they require threading a
`message_id` value through the streaming pipeline in `bridge_events.rs`. The session/delete
stabilization is straightforward if already added in the v0.13.1 step.

---

## Summary: All Code Changes

### `crates/roko-acp/src/types.rs`

| Change | Lines affected | Effort |
|---|---|---|
| Bump `ACP_SPEC_VERSION` to `"0.13.6"` | Line 9 | Trivial |
| Add `SessionCapabilities` struct (with `close`, `resume`, `delete`, `list`, `additional_directories`, `meta` fields) | New ~30 lines | Small |
| Add marker structs: `SessionCloseCapabilities`, `SessionResumeCapabilities`, `SessionDeleteCapabilities`, `SessionListCapabilities`, `SessionAdditionalDirectoriesCapabilities` | New ~25 lines | Trivial |
| Add `session_capabilities` field to `AgentCapabilities` | 1 line | Trivial |
| Add `AuthCapabilities` / `AgentAuthCapabilities` / `LogoutCapabilities` structs | New ~15 lines | Trivial |
| Add `auth` field to `AgentCapabilities` | 1 line | Trivial |
| Add `SessionDeleteParams` struct | New ~5 lines | Trivial |
| Add `additional_directories` field to `SessionNewParams`, `SessionLoadParams` | 2 lines each | Trivial |
| Create `SessionResumeParams` as distinct type from `SessionLoadParams` | New ~10 lines | Trivial |
| Add `message_id` field to `AgentMessageChunk`, `AgentThoughtChunk`, `ToolCall` variants in `SessionUpdate` | 3 lines | Trivial |
| Add `additional_directories` to `SessionInfo` | 1 line | Trivial |

**Estimated total: ~100 new/changed lines in types.rs**

### `crates/roko-acp/src/handler.rs`

| Change | Lines affected | Effort |
|---|---|---|
| Import new types | ~5 lines | Trivial |
| Populate `session_capabilities` in `InitializeResult` | ~15 lines | Small |
| Add `auth` capability in `InitializeResult` | ~5 lines | Trivial |
| Add `session/delete` match arm | ~15 lines | Small |
| Add `logout` match arm | ~10 lines | Trivial |

**Estimated total: ~50 new/changed lines in handler.rs**

### `crates/roko-acp/src/session.rs`

| Change | Lines affected | Effort |
|---|---|---|
| Add `delete_session()` method to `SessionManager` | ~15 lines | Small |
| Store/expose `additional_directories` per session | ~10 lines | Small |
| Track message ID counter per session | ~10 lines | Small |

**Estimated total: ~35 new/changed lines in session.rs**

### `crates/roko-acp/src/bridge_events.rs`

| Change | Lines affected | Effort |
|---|---|---|
| Thread `message_id` through streaming chunk emissions | ~20 lines | Small |
| Increment message ID on new logical messages | ~5 lines | Trivial |

**Estimated total: ~25 new/changed lines in bridge_events.rs**

### `crates/roko-acp/tests/protocol_conformance.rs`

| Change | Lines affected | Effort |
|---|---|---|
| Update `initialize` response assertions for new capabilities shape | ~15 lines | Small |
| Add `session/delete` conformance test | ~25 lines | Small |
| Add `logout` conformance test | ~15 lines | Small |
| Add `messageId` presence test | ~20 lines | Small |

**Estimated total: ~75 new/changed lines in tests**

---

## Overall Effort Estimate

| Category | Effort |
|---|---|
| Type definitions (types.rs) | ~100 lines, 1-2 hours |
| Handler updates (handler.rs) | ~50 lines, 30 min |
| Session logic (session.rs) | ~35 lines, 30 min |
| Streaming (bridge_events.rs) | ~25 lines, 30 min |
| Tests (protocol_conformance.rs) | ~75 lines, 1 hour |
| **Total** | **~285 lines, ~4 hours** |

The largest risk is the `AgentCapabilities` restructuring: our flat `load_session: bool`
becomes a structured `session_capabilities: SessionCapabilities` object. Any editor
client parsing our `InitializeResult` will see the new shape. Zed and Cursor both handle
this gracefully (they use the official schema types), but custom integrations may break.

---

## Alternative: Use Official Crate

- **Crate**: [`agent-client-protocol-schema`](https://crates.io/crates/agent-client-protocol-schema)
- **Current version**: v1.6.0 (2026-07-21)
- **License**: Apache-2.0

### Pros

1. **Always up to date** -- new stabilizations arrive via `cargo update`
2. **Schema-validated** -- types are generated from the canonical JSON Schema
3. **Feature flags** -- unstable features gated behind `unstable_*` Cargo features
4. **Routing enums** -- `AgentRequest`, `ClientRequest` etc. provide exhaustive dispatch
5. **Builder patterns** -- ergonomic construction of complex types
6. **JSON Schema generation** -- `schemars` derives included for docs/validation
7. **No maintenance burden** -- eliminates the 1,300-line hand-maintained `types.rs`

### Cons

1. **Dependency weight** -- adds `schemars`, `diffy` transitively; increases compile time
2. **Type naming mismatch** -- official names like `NewSessionRequest` differ from our
   `SessionNewParams`; requires a migration pass across handler.rs, session.rs,
   bridge_events.rs, transport.rs, and tests (~40 type references)
3. **Serialization differences** -- official crate may serialize `Option<T>` or enum
   discriminants differently; needs end-to-end testing with Zed and Cursor
4. **Loss of roko extensions** -- our `SessionBudgetStatus`, `ConfigSources`,
   `ConfigWarnings`, `BudgetStatusUpdate`, `McpStatusUpdate`, `SessionInfoUpdate` are
   roko-specific extensions not in the official crate; would need wrapper types or `_meta`
5. **Version coupling** -- bumping the crate version could introduce unintended schema
   changes if not pinned carefully
6. **Feature flag churn** -- unstable features can be removed or restructured between
   minor versions without SemVer guarantees

### Recommendation

**Keep hand-maintained types.rs for now, but consider switching after v2 stabilizes.**

The official crate is well-maintained and authoritative, but switching mid-stream would
require ~200 lines of adapter code for our roko-specific extensions and risk subtle
serialization regressions in Zed/Cursor. The 285-line manual bump is safer and faster
than a full crate migration (~4 hours vs ~8-12 hours including testing).

When ACP v2 lands (expected late 2026), the type inventory will change significantly.
That would be the natural migration point: adopt the official crate for v2 types and
deprecate our v1 hand-maintained types simultaneously.

---

## References

- [ACP GitHub repository](https://github.com/agentclientprotocol/agent-client-protocol)
- [ACP CHANGELOG.md](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/CHANGELOG.md)
- [ACP Updates page](https://agentclientprotocol.com/updates)
- [ACP v1 Schema docs](https://agentclientprotocol.com/protocol/v1/schema)
- [ACP Session Setup spec](https://agentclientprotocol.com/protocol/v1/session-setup)
- [agent-client-protocol-schema on crates.io](https://crates.io/crates/agent-client-protocol-schema)
- [agent-client-protocol-schema on docs.rs](https://docs.rs/agent-client-protocol-schema)
- [Additional Workspace Roots RFD](https://agentclientprotocol.com/rfds/additional-directories)
- [DeepWiki overview](https://deepwiki.com/agentclientprotocol/agent-client-protocol)
