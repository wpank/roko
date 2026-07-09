# E — Budget Management

Coverage for `docs/03-composition/05-token-budget-management.md`.

---

## Verdict

`narrow`

Budget machinery exists, but the docs overstate how unified it is in runtime use.

---

## Current State

### Real budget surfaces

- `PromptBudget` in `crates/roko-compose/src/templates/common.rs:17`
- `budget_for()` in `crates/roko-compose/src/templates/common.rs:44`
- `Complexity` in `crates/roko-compose/src/budget.rs:23`
- `adjusted_budget_for()` in `crates/roko-compose/src/budget.rs:66`
- `build_with_context_window_and_section_effectiveness(...)` in `crates/roko-compose/src/role_prompts.rs:391-467`
- `TokenCounter` support through `SystemPromptBuilder::build_with_counter(...)` in `system_prompt_builder.rs:265-327`

### Partial reality

- per-template section caps are still hardcoded in the template modules,
- the validated runtime path leans on context-window thresholds and composer budget enforcement,
- `adjusted_budget_for()` does not currently appear on the main orchestrate prompt path.

That means the right description is “budget APIs exist, but unified budget policy is partial.”

---

## What To Keep

- role-level budget tables are real,
- context-window validation is real,
- token-count-aware builder logic is real.

---

## What To Narrow

- stop describing `budget_for()` as the single runtime source of truth,
- stop implying complexity-adaptive budgets are broadly active,
- keep the distinction between static template caps and runtime validation.

---

## What To Defer

Do not pull these into batch `03`:

- predictive budget models,
- learned budget controllers,
- per-layer compression systems,
- sophisticated allocation/eval loops.

---

## Follow-On Batch Shape

A reasonable follow-on code batch from here is small:

1. wire one live path to a shared budget helper,
2. or explicitly codify the remaining split in tests/comments,
3. stop there.

Trying to solve every budget seam in one pass is exactly the overscope the audit rejected.
