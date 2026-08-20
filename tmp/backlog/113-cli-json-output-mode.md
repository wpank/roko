# 113 — CLI JSON Output Mode (`--json` on Core Commands)

**Priority**: P1 — Agents and scripts that drive roko programmatically need structured output; plain-text parsing from `roko status` or `roko plan list` is fragile and breaks on formatting changes.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/commands/`, `crates/roko-cli/src/main.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §0.4, `tmp/backlog/_mori-old-gaps.md` MO-04

---

## Background

The self-hosting loop requires Claude to query roko's state programmatically. Currently, `roko status`, `roko doctor`, `roko learn all`, `roko plan list`, `roko plan show <id>`, and `roko agent list` all produce human-formatted output. Claude must parse multi-column tables and prose descriptions, which is error-prone and breaks when column widths or phrasing changes.

Mori solved this with a `--json` flag on all diagnostic commands, producing stable documented schemas. Roko has partial JSON support on a few routes but no consistent pattern across the core CLI. The `roko doctor` help text mentions `--json` but the output format is not actually guaranteed.

The fix is mechanical: for each command, add a `--json` flag that serializes the same data the text view renders into a stable JSON object, then prints it and exits. The JSON schema should be documented in a comment next to each serialization so that future changes require intentional updates.

## Current State

- `roko doctor` — mentions `--json` in help text; actual structured output format is inconsistent.
- `roko status` — plain text only (signal counts, episode counts).
- `roko learn all` — plain text summary of learning state.
- `roko plan list` — table format only.
- `roko plan show <id>` — human-readable format only.
- `roko agent list` — table format only.
- Backlog items #77 (CLI UX Consistency) and #100 (CLI Error Message Quality) touch CLI output but neither specifies JSON mode specifically.

## Implementation Plan

1. **Shared `--json` flag pattern**: Add a `json: bool` field to each relevant CLI subcommand struct. When `true`, serialize the command's result struct to JSON via `serde_json::to_string_pretty` and print. Avoid printing any other text (no banner, no table headers) when `--json` is active.

2. **`roko status --json`**:
   - Output: `{"signal_count": u64, "episode_count": u64, "last_run_id": Option<String>, "last_run_at": Option<String>, "workspace": String}`
   - Source: same reads as the text status command.

3. **`roko doctor --json`**:
   - Output: `{"checks": [{"name": String, "status": "pass"|"warn"|"fail", "message": String}], "overall": "pass"|"warn"|"fail"}`
   - Already partially structured internally; wire the JSON path.

4. **`roko learn all --json`**:
   - Output: `{"cascade_router": {...}, "gate_thresholds": {...}, "experiment_count": u64, "episode_count": u64, "efficiency_event_count": u64}`
   - Source: read the same JSONL/JSON files the text output reads.

5. **`roko plan list --json`**:
   - Output: `{"plans": [{"id": String, "name": String, "status": "pending"|"running"|"done"|"failed", "task_count": u64, "completed_task_count": u64}]}`

6. **`roko plan show <id> --json`**:
   - Output: full plan struct including tasks, dependencies, gate results, run history.

7. **`roko agent list --json`**:
   - Output: `{"agents": [{"name": String, "domain": String, "status": "running"|"stopped", "pid": Option<u32>}]}`

8. **Error handling**: If `--json` is set and the command fails, output `{"error": "message"}` with exit code 1. Never output partial JSON.

9. **Machine-stability guarantee**: Add a comment block above each output struct with `// JSON schema: stable. Changes require backlog item.` to signal intent.

## Acceptance Criteria

1. `roko status --json` outputs valid JSON and exits 0.
2. `roko doctor --json` outputs a `checks` array with `name`/`status`/`message` per check.
3. `roko learn all --json` outputs structured learning state without requiring jq to parse.
4. `roko plan list --json` outputs a `plans` array with status per plan.
5. `roko plan show <id> --json` outputs the full plan including task list.
6. `roko agent list --json` outputs an `agents` array.
7. All six commands output `{"error": "..."}` on failure when `--json` is set.
8. None of the JSON paths print banners, ANSI colour codes, or table headers.

## Verification Checklist

- [ ] `roko status --json | jq .signal_count` returns an integer without error.
- [ ] `roko doctor --json | jq '.checks[] | select(.status=="fail")'` lists failing checks.
- [ ] `roko plan list --json | jq '.plans | length'` returns the plan count.
- [ ] Run each command with `--json` and pipe to `jq .` to verify valid JSON with no text preamble.
- [ ] Simulate a workspace error (delete `.roko/`) and verify `--json` commands emit `{"error": "..."}`.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/mod.rs` | Add JSON output helpers / shared pattern |
| `crates/roko-cli/src/commands/do_cmd.rs` | Add `--json` to `roko status`, `roko plan list`, `roko plan show` |
| `crates/roko-cli/src/commands/learn.rs` | Add `--json` to `roko learn all` |
| `crates/roko-cli/src/commands/plan.rs` | Add `--json` to `roko plan list` and `roko plan show` |
| `crates/roko-cli/src/main.rs` | Wire `--json` flag for doctor, agent list commands |
