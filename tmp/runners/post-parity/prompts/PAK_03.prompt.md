# PAK_03: Convert audit-only prompts to concrete implementation tasks

## Task
Review 5 audit-only prompts (PB_01, PC_01, PD_01, PE_01, PY_01) and convert each into a concrete implementation task with specific code changes, or fold their findings into existing implementation prompts.

## Runner Context
Runner PAK (Testing Gaps), batch 3 of 3. Depends on PB_01, PC_01, PD_01, PE_01, PY_01 (must run audits first).

## Problem
5 prompts are pure audit/research tasks that produce markdown reports but no code changes:
- **PB_01**: Document current chat dispatch data flow
- **PC_01**: Document current streaming data flow
- **PD_01**: Audit slash command handlers
- **PE_01**: Audit `dangerously_skip_permissions` usage
- **PY_01**: Audit direct `agent_exec` call sites

These are useful as research but a Codex mini agent can't execute them mechanically — they require judgment about what to document and how. Their findings should flow into concrete fix prompts.

## Exact Changes

### Step 1: For each audit prompt, determine the concrete fix

Read the audit prompt and its dependent implementation prompts. Determine:
1. Does the audit produce findings that are already covered by a later batch?
2. Can the audit be converted to a specific code change?
3. Should the audit findings be merged into the implementation prompt's "Current Code" section?

### Step 2: Convert or merge each prompt

**PB_01** (chat dispatch audit) → Merge findings into PP_03 (chat loop deduplication). PP_03 already has the exact code locations. PB_01's data flow diagram should be added as a comment block in PP_03's write scope.

**PC_01** (streaming data flow) → Merge findings into PAD_01 (stream parser consolidation). PAD_01 already lists the 6+ parser locations. Convert PC_01 to "Add doc comment in canonical stream parser explaining the data flow" — a concrete 10-line code change.

**PD_01** (slash command audit) → Convert to: "Add `unreachable!()` panic guard to unhandled slash command branches." Concrete: find the slash command match block, add `_ => { tracing::warn!("unknown slash command: {cmd}"); }` as default arm.

**PE_01** (dangerously_skip_permissions audit) → Already covered by PE_04 (which enumerates all 5 `permissive()` call sites). Merge PE_01's audit findings into PE_04's "Current Code" section and mark PE_01 as superseded.

**PY_01** (agent_exec call site audit) → Already covered by PY_02, PY_03, PY_04 (which replace all call sites). Merge PY_01's findings into PY_02's "Current Code" section and mark PY_01 as superseded.

### Step 3: Rewrite each prompt file

For prompts being converted: rewrite with exact file:line, before/after code, and verify command.

For prompts being superseded: replace content with:
```markdown
# PX_YY: [Original title] — SUPERSEDED

This audit's findings have been incorporated into [prompt ID]. No independent action needed.
```

### Step 4: Update batches.toml

Remove superseded prompts from batches.toml or mark them `skip = true`.

## Write Scope
- `tmp/runners/post-parity/prompts/PB_01.prompt.md` (merge into PP_03 or convert)
- `tmp/runners/post-parity/prompts/PC_01.prompt.md` (convert to doc comment task)
- `tmp/runners/post-parity/prompts/PD_01.prompt.md` (convert to default arm guard)
- `tmp/runners/post-parity/prompts/PE_01.prompt.md` (superseded by PE_04)
- `tmp/runners/post-parity/prompts/PY_01.prompt.md` (superseded by PY_02/03/04)
- `tmp/runners/post-parity/batches.toml` (update entries)

## Read-Only Context
- All existing prompts referenced above (PP_03, PAD_01, PE_04, PY_02-04)
- The codebase files listed in each audit prompt

## Verify
```bash
# Verify no broken cross-references in batches.toml
grep -c 'prompt =' tmp/runners/post-parity/batches.toml
ls tmp/runners/post-parity/prompts/*.prompt.md | wc -l
# Counts should match
```

## Acceptance Criteria
- No pure-audit prompts remain — each is either converted to concrete code change or superseded
- Superseded prompts explicitly reference their replacement
- batches.toml updated to reflect changes
- All remaining prompts are mechanically executable by a Codex mini agent

## Do NOT
- Delete any prompt files (mark as superseded instead)
- Change the implementation prompts (PP_03, PAD_01, PE_04, PY_02-04)
- Create new runner categories for converted tasks
