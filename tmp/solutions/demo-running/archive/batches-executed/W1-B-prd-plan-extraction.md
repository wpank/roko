# W1-B: Fix `prd plan` Silent Extraction Failure

**Priority**: P0 — blocks demo pipeline
**Effort**: 30-60 minutes
**Files to modify**: 1-2 files
**Dependencies**: None

## Problem

`roko prd plan <slug>` dispatches an agent successfully, but no tasks.toml is created. The command exits 0 silently. The LLM agent uses tool calls (write_file) instead of outputting TOML to stdout. The extraction code only searches stdout for fenced ```toml blocks.

## Root Cause

The plan generation happens in `crates/roko-cli/src/prd.rs`, function `generate_plan_from_prd_with_outcome()` (line 926). This dispatches an agent with tools enabled. The agent's prompt says "output the plan content" but the LLM may decide to use `write_file` instead. When it does, stdout contains only tool-use metadata (~28 bytes), and the TOML extraction finds nothing.

## Exact Code to Change

### File: `crates/roko-cli/src/prd.rs`

### Understanding the flow

The function at line 926 builds a prompt with `task_prompt` (line 998-1005 area) that says:
```
Do NOT create files directly. Instead, output the plan content as follows:
```

But the agent still has tool capabilities. The fix is two-pronged:

### Fix 1: Strip write tools from plan generation dispatch

Find where `AgentExecOpts` is constructed for the plan generation call (inside `generate_plan_from_prd_with_outcome`). It calls either `run_agent_capture_logged` or `run_agent_logged` from `crate::agent_exec`.

Look for the `AgentExecOpts` struct being built. Add a field or parameter that disables file-writing tools. The key is to NOT pass tool configs that include `write_file`, `create_file`, etc.

Search for how tools are configured in `AgentExecOpts`:
```bash
grep -n 'AgentExecOpts' crates/roko-cli/src/prd.rs
grep -n 'tools\|tool_config\|write_file' crates/roko-cli/src/prd.rs
```

If `AgentExecOpts` has a `tools` or `tool_allowlist` field, set it to exclude write tools:
```rust
let opts = AgentExecOpts {
    // ... existing fields
    tool_allowlist: Some(vec!["read_file".to_string(), "search".to_string()]),
    // OR
    disable_write_tools: true,
    // ... whatever the struct supports
};
```

If there's no such field, the simplest approach is to add one. Check `crates/roko-cli/src/agent_exec.rs` for the struct definition.

### Fix 2: Post-dispatch validation (safety net)

After the agent returns, check if a tasks.toml was actually produced. If not, print an actionable error instead of silently exiting 0.

In `generate_plan_from_prd_with_outcome`, after the agent dispatch returns:

```rust
// After agent completes, check if any tasks files were produced
let tasks_after = dry_run_fs::snapshot_tasks_files(&plans_root);
let new_tasks: Vec<_> = tasks_after
    .iter()
    .filter(|p| !tasks_before.contains(p))
    .collect();

if new_tasks.is_empty() {
    // Check if agent output contains TOML that wasn't written
    // This is the extraction fallback
    anyhow::bail!(
        "Plan generation failed: no tasks.toml was produced.\n\
         The planning agent may have used tool calls instead of text output.\n\
         hint: Try again, or create .roko/plans/tasks.toml manually."
    );
}
```

Find where `tasks_before` is captured (line ~958: `let tasks_before = dry_run_fs::snapshot_tasks_files(&plans_root);`) — there should be a corresponding post-check. If there isn't one, add it.

### Fix 3: Also check for TOML in agent output as fallback

If the agent DID output TOML to stdout but it wasn't extracted and written, extract it:

```rust
// If no files were written, try extracting from agent output
if new_tasks.is_empty() {
    if let Some(toml_content) = extract_toml_block(&agent_output) {
        let tasks_path = plans_root.join("tasks.toml");
        std::fs::create_dir_all(&plans_root)?;
        std::fs::write(&tasks_path, &toml_content)?;
        println!("  Extracted tasks.toml from agent output ({} bytes)", toml_content.len());
    } else {
        anyhow::bail!(
            "Plan generation failed: no tasks.toml produced and no TOML found in output.\n\
             hint: Try again, or create .roko/plans/tasks.toml manually."
        );
    }
}
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W1-B-prd-plan-extraction.md and implement all changes described in it. The batch requires investigating AgentExecOpts in crates/roko-cli/src/agent_exec.rs to determine how to restrict tools. Read that file first. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Just make the code changes and mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 1 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation, test with: `cargo run -p roko-cli -- prd plan <slug>` — should produce tasks.toml or print a clear error.

## Checklist

- [x] Find where `AgentExecOpts` is built in `generate_plan_from_prd_with_outcome` (line ~926 in prd.rs)
- [x] Strip or restrict write tools from plan generation dispatch
- [x] Add post-dispatch check: if no tasks.toml was produced, try extracting from output
- [x] If extraction fails too, return an actionable error (not silent exit 0)
- [ ] Verify: `roko prd plan <slug>` produces tasks.toml OR prints clear error
- [ ] Pre-commit checks pass
