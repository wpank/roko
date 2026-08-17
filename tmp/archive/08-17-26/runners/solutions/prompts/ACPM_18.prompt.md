# ACPM_18: Build MCP Server Registry

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-18`](../ISSUE-TRACKER.md#acpm-18)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.18
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Each MCP server (`roko-mcp-code`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`) runs as an isolated subprocess with no awareness of peers. `roko-mcp-stdio` (`crates/roko-mcp-stdio/src/lib.rs`, ~252 LOC) provides the shared transport; it is the natural home for federation infrastructure.

## Exact Changes

1. Add deps to `roko-mcp-stdio/Cargo.toml`: `serde`, `serde_json`, `chrono`, `fs2` (file locking).
2. Define `McpServerEntry` in `registry.rs`:
   ```rust
   pub struct McpServerEntry {
       pub name: String,
       pub pid: u32,
       pub tools: Vec<String>,
       pub socket_path: PathBuf,
       pub registered_at: DateTime<Utc>,
   }
   ```
3. Define `McpRegistry` backed by a file at a configurable path (default `.roko/mcp-registry.json`):
   ```rust
   pub struct McpRegistry {
       path: PathBuf,
   }
   ```
4. Implement:
   - `register(entry: McpServerEntry) -> Result<()>` with `fs2::FileExt` file-locking for concurrent writes
   - `unregister(name: &str) -> Result<()>`
   - `discover(tool_name: &str) -> Option<McpServerEntry>` -- finds the server exposing a given tool
   - `list_all() -> Vec<McpServerEntry>` -- returns all registered servers
   - `health_check() -> usize` -- removes entries whose pid is not running (platform-specific check), returns count removed
5. Add `pub mod registry;` to `lib.rs`.

## Design Guidance

Use file-based registry with advisory locking (`fs2`). The registry is append-heavy and read-heavy, with very few entries (typically 2-5 servers). JSON file is fine for this scale. Health check uses `kill(pid, 0)` on Unix to test process existence.

## Write Scope

- `crates/roko-mcp-stdio/src/lib.rs`
- `crates/roko-mcp-stdio/Cargo.toml`

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

- [ ] Unit test: register 3 servers, discover by tool name
- [ ] Unit test: health check removes entry for non-existent pid
- [ ] Unit test: file locking prevents corruption under concurrent writes (spawn 2 threads writing simultaneously)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: register 3 servers, discover by tool name
- Unit test: health check removes entry for non-existent pid
- Unit test: file locking prevents corruption under concurrent writes (spawn 2 threads writing simultaneously)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
