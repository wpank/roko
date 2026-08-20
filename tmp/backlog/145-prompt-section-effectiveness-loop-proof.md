# 145 — Prompt Section Effectiveness Loop Proof in Runner-v2

**Priority**: P2 — The `SectionEffectivenessRegistry` is populated from efficiency events and should influence prompt assembly priority, but the runner-v2 path has not been proven to read from the registry before composing each prompt, so the learning signal may be discarded.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-compose/src/`, `crates/roko-learn/src/`
**Depends on**: #141 (per-turn efficiency events improve the granularity of section data)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §F-6 (suggested 129)

---

## Background

`SectionEffectivenessRegistry` tracks which prompt sections (e.g., `system_context`, `task_description`, `knowledge_injection`, `gate_feedback`) correlate with gate passes vs gate fails. Sections that appear before gate passes are weighted more highly; sections that appear in passes only when the agent retried are weighted lower.

Legacy `PlanRunner` called `section_effectiveness_snapshot()` at prompt assembly time and `LearningRuntime::append_efficiency_event()` recorded which sections were included. Together they formed a feedback loop: sections that correlate with pass outcomes get more token budget in future prompts.

Runner-v2 also calls `append_efficiency_event()` but the audit found no evidence that runner-v2 reads `section_effectiveness_snapshot()` before composing each prompt. If the read is missing, the registry is only written, never consulted — the learning signal is discarded.

This item is a proof exercise: run two plans and demonstrate that a section's priority changes between them based on effectiveness data from the first run.

## Current State

- `crates/roko-learn/src/` — `SectionEffectivenessRegistry` with `snapshot()` and `update()` methods.
- `crates/roko-compose/src/` — `PromptAssembler` or `SystemPromptBuilder` that assembles sections.
- `crates/roko-cli/src/runner/event_loop.rs` — dispatches agents with assembled prompts; unclear whether section effectiveness is consulted.
- `.roko/learn/section-effectiveness.json` — may or may not be written (path may not be reserved).

## Implementation Plan

1. **Audit the read path**: Read `event_loop.rs` and `dispatch_agent_with()` to find where `PromptAssembler` is invoked. Determine whether `SectionEffectivenessRegistry::snapshot()` is called and its output is passed to the assembler.

2. **Wire the read if missing**: If the registry is not consulted, add:
   ```rust
   let section_effectiveness = learning_runtime.section_effectiveness_snapshot();
   let prompt = prompt_assembler.assemble_with_effectiveness(task, role, section_effectiveness);
   ```

3. **Verify the write path**: After task completion, confirm `learning_runtime.update_section_effectiveness(episode, gate_passed)` is called. If not, add it in the gate completion handler.

4. **Reserve the file path**: Add `.roko/learn/section-effectiveness.json` to `LearningPaths` and write the registry there on each update.

5. **Two-run proof script** at `tests/section_effectiveness_proof.sh`:
   - **Run 1**: Run a task with multiple prompt sections. Record which sections were in the passing prompt.
   - Between runs: inspect `.roko/learn/section-effectiveness.json`; note section weights.
   - **Run 2**: Run a similar task. Verify that sections with higher effectiveness weights from run 1 appear with more token budget in run 2's prompt (verifiable via efficiency event `prompt_sections` field).

6. **Before/after comparison**: The proof needs a measurable signal. Add a `section_token_budget_override: HashMap<String, u32>` to prompt assembly that records which sections got budget from effectiveness data vs defaults. Include this in the efficiency event.

## Acceptance Criteria

1. `SectionEffectivenessRegistry::snapshot()` is called before each prompt assembly in runner-v2.
2. `.roko/learn/section-effectiveness.json` is written after each task completion.
3. Two-run proof: after run 1, at least one section weight changes in the registry.
4. In run 2, the changed section appears with a different token budget than in run 1.
5. Efficiency events in run 2 include `section_token_budget_override` showing the learned allocation.

## Verification Checklist

- [ ] Audit: confirm `section_effectiveness_snapshot()` is called in `event_loop.rs` before dispatch.
- [ ] After run 1, verify `.roko/learn/section-effectiveness.json` has non-default weights.
- [ ] Compare efficiency event `prompt_sections` between run 1 and run 2 for the same role; verify at least one section's token allocation changed.
- [ ] Unit test: `SectionEffectivenessRegistry` with a gate-pass episode increases the section weight.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Read `section_effectiveness_snapshot()` before dispatch; write on completion |
| `crates/roko-compose/src/` | Accept effectiveness snapshot in prompt assembly |
| `crates/roko-learn/src/` | Reserve section-effectiveness.json path; add `section_token_budget_override` to efficiency event |
| `tests/section_effectiveness_proof.sh` | New two-run proof script |
