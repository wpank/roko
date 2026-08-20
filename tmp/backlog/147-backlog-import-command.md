# 147 — Backlog Import Command (`roko backlog import`)

**Priority**: P2 — The PRD pipeline (idea → draft → plan) exists end-to-end, but there is no way to batch-import the markdown backlog files into it; this breaks the self-hosting loop where Claude should be able to convert a backlog item directly into a running implementation plan.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/commands/`, `crates/roko-cli/src/prd.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §4.2

---

## Background

The roko self-hosting workflow is: backlog item → PRD idea → PRD draft → implementation plan → execution. Steps 2-5 are automated: `roko prd idea "<text>"`, `roko prd draft new <slug>`, `roko prd plan <slug>`, `roko plan run`. But step 1 (converting a structured markdown backlog file into a PRD idea) is manual — someone has to read the backlog file and write a `roko prd idea` command by hand.

The `tmp/backlog/*.md` files follow a consistent schema: title, priority, size, background, current state, implementation plan, acceptance criteria. This structure maps cleanly to a PRD idea: the title becomes the idea title, the background becomes the description, the implementation plan becomes the initial approach, and the acceptance criteria become the "success looks like" section.

`roko backlog import` reads one or more backlog files, parses the known schema, creates PRD ideas, and optionally triggers draft generation and plan generation.

## Current State

- `crates/roko-cli/src/prd.rs` — `cmd_prd_idea(text: &str)` creates a PRD idea from text.
- `tmp/backlog/*.md` — 150 backlog items following the schema from this session.
- No `roko backlog` subcommand exists.
- No markdown parser is applied to backlog files.

## Implementation Plan

1. **Create `crates/roko-cli/src/commands/backlog.rs`**: Implement the `backlog` subcommand group:
   - `roko backlog import <path>` — import a single backlog file.
   - `roko backlog import <dir>` — import all `*.md` files in a directory.
   - `roko backlog list` — list all backlog items that have been imported as PRD ideas.

2. **Backlog file parser**: Implement `BacklogItem::from_markdown(content: &str) -> Result<Self>`:
   - Extract title: the `# NNN — Title` first heading.
   - Extract priority and size from the `**Priority**` and `**Size**` bold lines.
   - Extract background from the `## Background` section.
   - Extract acceptance criteria from the `## Acceptance Criteria` numbered list.
   - Extract implementation plan from the `## Implementation Plan` section.

3. **Create PRD idea**: Call `cmd_prd_idea(format!("{}: {}", item.number, item.title))` and include the background and acceptance criteria in the idea text.

4. **Optional flags**:
   - `--draft` — after creating the idea, immediately run `roko prd draft new <slug>` to generate a draft (uses an LLM call).
   - `--plan` — after drafting, immediately generate an implementation plan (`roko prd plan <slug>`).
   - `--execute` — after plan generation, immediately start `roko plan run` (use with caution).

5. **Idempotency**: Check if a PRD idea with the same backlog number already exists (by checking `.roko/prd/` for an idea with `source_backlog: NNN`). If it exists, skip with a message. Do not create duplicates.

6. **Progress output**: Print one line per imported backlog item: `[IMPORT] 100-cli-error-message-quality → idea: prd-000042`.

7. **Batch import**: When importing a directory, process files in numeric order (100, 101, ...) and report a summary at the end: `Imported 40 backlog items (38 new, 2 already existed)`.

## Acceptance Criteria

1. `roko backlog import tmp/backlog/111-screenshot-command-completion.md` creates a PRD idea.
2. The PRD idea title matches the backlog item title.
3. The PRD idea description includes the backlog item's background text.
4. Running import twice on the same file does not create a duplicate idea.
5. `roko backlog import tmp/backlog/` imports all `*.md` files (skipping index files starting with `_` or `00`).
6. `--draft` flag triggers draft generation after idea creation.
7. `roko backlog list` shows which backlog items have been imported.

## Verification Checklist

- [ ] `roko backlog import tmp/backlog/111-screenshot-command-completion.md` exits 0.
- [ ] `roko prd list` shows a new idea with "screenshot command" in the title.
- [ ] Re-run import on the same file; verify "already exists" message and no duplicate.
- [ ] `roko backlog import tmp/backlog/` imports all numbered `.md` files in the directory.
- [ ] `roko backlog import --draft tmp/backlog/111-screenshot-command-completion.md` creates a draft PRD.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/backlog.rs` | New file: `BacklogItem` parser, import command |
| `crates/roko-cli/src/commands/mod.rs` | Register `backlog` subcommand |
| `crates/roko-cli/src/main.rs` | Wire `roko backlog` top-level command |
| `crates/roko-cli/src/prd.rs` | Expose `cmd_prd_idea` as a library function for programmatic use |
