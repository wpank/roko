# ACPM_32: Implement A2A Protocol Types

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-32`](../ISSUE-TRACKER.md#acpm-32)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.32
- Priority: **P2**
- Effort: 5 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Google's A2A protocol defines Agent Cards, Tasks, Messages, and Artifacts. It is HTTP/JSON-RPC based, which aligns with `roko-serve`. The minimal surface area needed: Agent Card (discovery), Task send/status (collaboration), Message/Part types (content).

## Exact Changes

1. Create `crates/roko-a2a/` with `Cargo.toml` (deps: `serde`, `serde_json`, `chrono`, `url`, `uuid`).
2. Define types in `types.rs`:
   - `AgentCard { name, url, description, version, capabilities, skills, default_input_modes, default_output_modes, authentication }`
   - `AgentSkill { id, name, description, input_modes, output_modes }`
   - `A2ATask { id, session_id, status, messages, artifacts, metadata }`
   - `TaskStatus`: `Submitted`, `Working`, `InputRequired`, `Completed`, `Failed`, `Canceled`
   - `A2AMessage { role, parts }` and `A2APart` (`TextPart`, `FilePart`, `DataPart`)
   - `A2AArtifact { name, description, parts, index }`
   - `AuthenticationInfo { schemes: Vec<AuthScheme> }`
3. All types derive `Serialize`, `Deserialize`, `Debug`, `Clone`.
4. Add the crate to workspace `Cargo.toml` members.

## Write Scope

- `Cargo.toml`

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

- [ ] Serde round-trip tests for all types pass
- [ ] Agent Card JSON matches A2A spec schema structure

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Serde round-trip tests for all types pass
- Agent Card JSON matches A2A spec schema structure
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
