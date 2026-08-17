# SAFE_15: Unified Security Audit Trail

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-15`](../ISSUE-TRACKER.md#safe-15)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.15
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Create a single append-only JSONL audit log for all security-relevant
events. This is the foundation for compliance and for Tasks 17.1-17.14 to
report their enforcement actions.

## Exact Changes

1. Define `SecurityAuditEvent`:
   ```rust
   pub struct SecurityAuditEvent {
       pub timestamp: DateTime<Utc>,
       pub event_type: AuditEventType,
       pub agent_id: String,
       pub task_id: Option<String>,
       pub detail: serde_json::Value,
       pub severity: AuditSeverity,
   }

   pub enum AuditEventType {
       ContractLoaded, ContractViolation,
       PermissionGranted, PermissionDenied,
       ToolCallBlocked, ToolOutputSanitized,
       RateLimitHit, NetworkBlocked,
       GateConfigVerified, GateConfigTampered,
       SecretRedacted, PathViolation,
   }

   pub enum AuditSeverity { Info, Warning, Violation, Critical }
   ```
2. Create `SecurityAuditLogger` that writes to `.roko/audit/security.jsonl`
   (append-only, never truncated)
3. Add file rotation when log exceeds 100MB (keep 10 rotated files)
4. Expose as `Arc<SecurityAuditLogger>` for use by all safety checks
5. Add `roko audit show` CLI command that displays recent security events
   with severity filtering

## Write Scope

- `crates/roko-runtime/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Every contract enforcement from Task 17.2 produces a `SecurityAuditEvent`
- [ ] Every permission decision (grant/deny) is logged
- [ ] Log files are append-only (no overwrite, no truncation)
- [ ] `roko audit show` displays recent events with `--severity` filter

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every contract enforcement from Task 17.2 produces a `SecurityAuditEvent`
- Every permission decision (grant/deny) is logged
- Log files are append-only (no overwrite, no truncation)
- `roko audit show` displays recent events with `--severity` filter
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
