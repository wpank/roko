# ACPM_22: Add Federation Config to roko.toml

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-22`](../ISSUE-TRACKER.md#acpm-22)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.22
- Priority: **P2**
- Effort: 2 hours
- Depends on: `ACPM_18` (source 9.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Federation behavior should be configurable via `roko.toml`. Currently `roko-core/src/config/schema.rs` defines the `RokoConfig` structure.

## Exact Changes

1. Add `McpFederationConfig` to config schema:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct McpFederationConfig {
       pub enabled: bool,                      // default: true
       pub registry_path: String,              // default: ".roko/mcp-registry.json"
       pub timeout_ms: u64,                    // default: 30000
       pub circuit_breaker_threshold: u32,     // default: 3
   }
   ```
2. Add `pub federation: Option<McpFederationConfig>` to the MCP config section.
3. Parse from TOML:
   ```toml
   [mcp.federation]
   enabled = true
   registry_path = ".roko/mcp-registry.json"
   timeout_ms = 30000
   circuit_breaker_threshold = 3
   ```
4. When `enabled = false` or section absent, federation client is not constructed.
5. Pass config to MCP servers via environment variables (`ROKO_MCP_FEDERATION_REGISTRY`, `ROKO_MCP_FEDERATION_TIMEOUT_MS`).

## Write Scope

- `crates/roko-core/src/config/schema.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Config section parses correctly
- [ ] `enabled = false` disables all federation features
- [ ] Default values work when section is omitted

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Config section parses correctly
- `enabled = false` disables all federation features
- Default values work when section is omitted
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
