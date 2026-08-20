# 136 — Safety Denial Audit Events (Durable, Queryable via HTTP/TUI)

**Priority**: P1 — When safety checks deny tool calls, path access, or network calls, no durable record is written; without audit events, safety cannot be verified or debugged in production, and claims of E34 compliance are not provable from runtime artifacts.
**Size**: S (1 day)
**Crates**: `crates/roko-agent/src/safety/`, `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-serve/src/routes/`
**Depends on**: None
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §L-1 (suggested 138), `tmp/backlog/_mori-old-gaps.md` MO-35

---

## Background

The E34 safety layer (8/8 accepted) includes trust-origin IFC, five-layer immune Graph, corrigibility, and sandbox policies. When the safety layer denies a tool call (e.g., an agent tries to call a tool outside its role allowlist), the denial is enforced but not recorded anywhere accessible.

Without durable audit records, there is no way to:
- Prove that safety checks ran during a plan execution.
- Debug why an agent could not complete a task (was it a safety denial?).
- Monitor for patterns of repeated denials that might indicate an agent trying to bypass restrictions.
- Trust that safety is working during unattended runs.

The fix: emit a `RunnerEvent::SafetyDenial` on every denial, include it in `.roko/events.jsonl`, and add an HTTP query endpoint for denials by run ID. The TUI F7:inspect panel can show a denial count (see #127).

## Current State

- `crates/roko-agent/src/safety/` — safety checks are performed and denials are enforced, but no event is emitted.
- `.roko/events.jsonl` — runner events are written here; a `SafetyDenial` variant is not present.
- `crates/roko-serve/src/routes/` — HTTP routes exist; no route for safety denial queries.
- Backlog #60 (Safety Dispatch Hardening) — covers tool allowlist enforcement; does not cover audit event emission or HTTP queryability.

## Implementation Plan

1. **Add `RunnerEvent::SafetyDenial` variant**:
   ```rust
   SafetyDenial {
       run_id: String,
       task_id: Option<String>,
       agent_role: String,
       denial_kind: DenialKind,   // ToolDenied, PathDenied, NetworkDenied, CapabilityDenied
       tool_name: Option<String>,   // for ToolDenied
       redacted_evidence: String,   // one-line reason, no sensitive data
       timestamp: DateTime<Utc>,
   }
   ```

2. **Emit from safety layer**: In each safety check path in `crates/roko-agent/src/safety/`:
   - Tool allowlist denial: emit `SafetyDenial { denial_kind: ToolDenied, tool_name: ... }`.
   - Path access denial: emit `SafetyDenial { denial_kind: PathDenied }`.
   - Network call denial: emit `SafetyDenial { denial_kind: NetworkDenied }`.
   - The safety layer needs a channel to the runner's event bus. Pass it in at agent creation time.

3. **Write to `.roko/events.jsonl`**: The runner's event bus already writes events to this file. `SafetyDenial` events should appear alongside task events and gate events.

4. **HTTP query endpoint**: Add to `crates/roko-serve/src/routes/`:
   - `GET /api/safety/denials?run_id=<id>` — returns all denials for a given run ID.
   - Response: `{"denials": [{"timestamp": "...", "role": "...", "kind": "...", "tool": "..."}]}`

5. **TUI denial count**: In `TuiModel`, add `safety_denials_count: u64` incremented on each `SafetyDenial` event. Display in F7:inspect's system health panel.

6. **Privacy**: The `redacted_evidence` field must not contain agent output, user data, or sensitive file contents. It should only describe the denial reason (e.g., "tool 'write_file' not in allowlist for role 'reviewer'").

## Acceptance Criteria

1. A tool-use denial by the safety layer writes a `SafetyDenial` entry to `.roko/events.jsonl`.
2. `GET /api/safety/denials?run_id=<id>` returns the denials for that run.
3. The TUI F7 panel shows a non-zero denial count when denials occurred.
4. Denial events contain `role`, `denial_kind`, and `tool_name` fields.
5. No sensitive agent output appears in the `redacted_evidence` field.
6. Existing safety enforcement behaviour is unchanged (denials are still enforced).

## Verification Checklist

- [ ] Configure a role with a restricted tool allowlist; run an agent that attempts to call a denied tool; verify `SafetyDenial` entry in `.roko/events.jsonl`.
- [ ] Query `GET /api/safety/denials?run_id=<id>` and verify the denial appears.
- [ ] Verify `redacted_evidence` contains only the denial reason, not agent output.
- [ ] Run a plan with no safety denials; verify no `SafetyDenial` entries are written.
- [ ] `cargo test -p roko-agent` passes with the new event emission.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-agent/src/safety/` | Emit `SafetyDenial` event on each denial; accept event channel |
| `crates/roko-cli/src/runner/types.rs` | Add `RunnerEvent::SafetyDenial` variant |
| `crates/roko-cli/src/runner/event_loop.rs` | Route `SafetyDenial` to `.roko/events.jsonl` |
| `crates/roko-serve/src/routes/` | Add `GET /api/safety/denials` HTTP route |
| `crates/roko-cli/src/tui/app.rs` | Add `safety_denials_count` to `TuiModel` |
