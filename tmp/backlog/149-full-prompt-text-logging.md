# 149 — Full Prompt Text Logging (Configurable, Bounded Retention)

**Priority**: P2 — Understanding why an agent made a specific decision requires knowing exactly what it saw; without full prompt logs, reproducing and debugging agent behaviour is guesswork.
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-core/src/config/`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §3.3, `tmp/backlog/_mori-old-gaps.md` MO-40

---

## Background

`AgentEfficiencyEvent` tracks section metadata: which prompt sections were included, how many tokens each took, whether sections were truncated or dropped. This metadata is useful but insufficient for debugging. To understand why an agent wrote a specific implementation or made a specific error, the reviewer needs the verbatim prompt text.

Mori stored full prompts in `.mori/memory/prompt-logs/<episode-id>.json` with the complete assembled text, per-section breakdown, token counts (via tiktoken), and context strategy. Roko stores section metadata but not the full text.

This is opt-in (off by default) because disk cost is significant: 20 concurrent agents × 50 tasks × average 10KB prompt = 10 MB per run. With retention of the last 100 prompts, disk is bounded. The feature is most useful during debugging and dogfood runs, not production operation.

## Current State

- `crates/roko-compose/src/` — `PromptAssembler` or `SystemPromptBuilder` produces the final prompt text.
- `crates/roko-cli/src/runner/event_loop.rs` — dispatches agents with assembled prompts; does not write the text to disk.
- No prompt log directory exists.
- `AgentEfficiencyEvent.prompt_sections` — carries section metadata but not full text.

## Implementation Plan

1. **Add config option**:
   ```toml
   [runner]
   log_prompts = false          # default off
   prompt_log_retention = 100   # keep last N prompt logs
   ```

2. **Create log writer**: In `event_loop.rs`, when `log_prompts = true` and immediately before dispatching an agent:
   ```rust
   let log_entry = PromptLogEntry {
       episode_id: episode_id.clone(),
       task_id: task.id.clone(),
       plan_id: plan.id.clone(),
       role: role.to_string(),
       assembled_at: Utc::now(),
       total_tokens: prompt_text.len() / 4,  // rough estimate
       sections: prompt_sections.clone(),     // from PromptAssembler
       full_text: prompt_text.clone(),
   };
   let path = format!(".roko/prompt-logs/{}-{}.json", episode_id, attempt);
   tokio::fs::write(&path, serde_json::to_string(&log_entry)?).await?;
   ```

3. **Bounded retention via GC**: After writing, count files in `.roko/prompt-logs/` and delete oldest files (by mtime) when the count exceeds `prompt_log_retention`.

4. **Directory creation**: Create `.roko/prompt-logs/` on first write if it does not exist.

5. **Wire into `roko diagnose`**: When `roko diagnose <plan-id>` is run (from #114), include a `prompt_log_path: Option<String>` field in the report pointing to the most recent prompt log for the failed task.

6. **Gitignore**: Add `.roko/prompt-logs/` to `.gitignore` to prevent prompt text from being committed.

## Acceptance Criteria

1. With `log_prompts = false` (default), no prompt log files are written.
2. With `log_prompts = true`, a log file is written for each agent dispatch.
3. Log files contain valid JSON with `full_text`, `sections`, `episode_id`, and `role` fields.
4. When the retention limit is reached, oldest files are deleted automatically.
5. `roko diagnose <plan-id>` includes `prompt_log_path` when a log exists.
6. `.roko/prompt-logs/` is gitignored.

## Verification Checklist

- [ ] Set `log_prompts = true`; run a plan; verify `.roko/prompt-logs/` has `.json` files.
- [ ] Inspect a log file; verify `full_text` contains the complete assembled prompt.
- [ ] Write more than `prompt_log_retention` prompts; verify old files are removed.
- [ ] Set `log_prompts = false`; run a plan; verify no log files are written.
- [ ] `git status` shows `.roko/prompt-logs/` as ignored.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Write prompt log file when `log_prompts = true` |
| `crates/roko-core/src/config/` | Add `log_prompts` and `prompt_log_retention` to config schema |
| `crates/roko-cli/src/commands/diagnose.rs` | Add `prompt_log_path` field to `DiagnoseReport` |
| `.gitignore` | Add `.roko/prompt-logs/` |
