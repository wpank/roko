# SAFE_21: MCP Tool Call Sanitization

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-21`](../ISSUE-TRACKER.md#safe-21)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.21
- Priority: **P2**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Sanitize inputs to and outputs from MCP tool calls. Block SSRF
patterns, path traversal, and data exfiltration.

## Exact Changes

1. Input sanitization:
   - Block SSRF patterns in URL arguments (private IPs, cloud metadata)
   - Block path traversal in file arguments
   - Validate argument types against MCP tool schemas
2. Output sanitization:
   - Apply `ResultFilter` pipeline (reuse from Task 17.7)
   - Strip prompt injection payloads
   - Size-limit responses (reject > 1MB)
3. Rate-limit MCP tool calls per server (default: 60/minute)
4. Log all MCP calls to `.roko/audit/mcp.jsonl`

## Write Scope

- `crates/roko-agent/src/safety/mod.rs`

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

- [ ] MCP call with `url: "http://169.254.169.254/metadata"` is blocked
- [ ] MCP response containing `<system>` injection markers is sanitized
- [ ] MCP calls exceeding rate limits return a clear error

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- MCP call with `url: "http://169.254.169.254/metadata"` is blocked
- MCP response containing `<system>` injection markers is sanitized
- MCP calls exceeding rate limits return a clear error
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
