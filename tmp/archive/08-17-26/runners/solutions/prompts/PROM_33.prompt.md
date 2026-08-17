# PROM_33: Role Identity from TOML Config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-33`](../ISSUE-TRACKER.md#prom-33)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.33
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Load role identity text from `.roko/roles/<role>.toml` files at
startup, falling back to compiled-in defaults.

## Exact Changes

1. Add `pub fn load_role_identity(role: &str, roko_dir: &Path) -> String`:
   - Try to read `.roko/roles/<role>.toml`
   - Parse `[role].identity` field
   - Fall back to `role_identity_for()` static strings
2. Add `[role.tier_adjustments]` support:
   - Surgical: use `tier_adjustments.surgical` text (terse)
   - Focused: use `tier_adjustments.focused` text (moderate)
   - Full: use base `identity` text (comprehensive)
3. Cache loaded roles for the duration of a plan run (use a HashMap)

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Without `.roko/roles/` directory, existing static role identities are used (no regression)
- [ ] With `.roko/roles/implementer.toml`, the custom identity is used for Implementer role
- [ ] Tier adjustments work: Surgical gets terse identity, Full gets comprehensive
- [ ] `is_local_model("ollama/llama3.2")` returns true
- [ ] Dispatching to Ollama produces system prompt <= 4K tokens
- [ ] Dispatching to Sonnet produces system prompt <= 12K tokens
- [ ] Dispatching to Opus produces system prompt <= 24K tokens
- [ ] `BudgetPredictor.predict()` is called before assembly in `dispatch_agent_with()`
- [ ] `BudgetPredictor.record()` is called after gate results are known
- [ ] After 10+ tasks, predicted budgets are used (blended with static defaults)
- [ ] `SectionInfluence.weights()` is applied as a multiplier during composition
- [ ] Sections with negative lift are visibly demoted in budget after 20+ tasks
- [ ] `roko chat` produces system prompt with role identity and conventions
- [ ] ACP `run_multi_role_review()` uses `ReviewerTemplate` (zero inline format strings)
- [ ] Long chat sessions compact after 30+ turns
- [ ] VCG allocation activates after 5+ warm observations per bidder
- [ ] MultiPatchForager is used for context retrieval with early stopping
- [ ] Per-model attention curves are populated for 5+ model families
- [ ] Prompt A/B testing is functional via ExperimentStore
- [ ] All prompts have version tags for learning attribution
- [ ] Knowledge confidence thresholds adapt to tier
- [ ] Episode history count adapts to tier
- [ ] Content-type-aware token estimation is the default counter
- [ ] Gate pass rate by tier: Surgical >= 70%, Focused >= 80%, Full >= 85%
- [ ] Token efficiency: system prompt size / model context window <= 15% (Focused/Full), <= 30% (Surgical)
- [ ] Learning convergence: after 50 tasks, BudgetPredictor estimates within 30% of actual for 80%+ of types
- [ ] 100% of dispatches go through PromptAssemblyService or SystemPromptBuilder
- [ ] 0 dispatches with empty system prompts
- [ ] 0 inline `format!()` role descriptions
- [ ] Assembly latency: < 50ms for Surgical, < 200ms for Full (excluding knowledge store queries)
- [ ] Memory: per-dispatch allocation < 1MB
- [ ] `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer builder, build methods
- [ ] `crates/roko-compose/src/prompt_assembly_service.rs` -- PromptAssemblyService, assembly pipeline
- [ ] `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model, budgets
- [ ] `crates/roko-compose/src/budget.rs` -- adjusted_budget_for, Complexity
- [ ] `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence
- [ ] `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, ModelAttentionCurves
- [ ] `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, should_stop_searching
- [ ] `crates/roko-compose/src/compaction.rs` -- compact_history, CompactionPolicy
- [ ] `crates/roko-compose/src/prompt.rs` -- PromptComposer, section scoring
- [ ] `crates/roko-compose/src/auction.rs` -- vcg_allocate, LearningBidder
- [ ] `crates/roko-compose/src/strategy.rs` -- CompositionStrategy, VCG warmup
- [ ] `crates/roko-compose/src/cognitive_workspace.rs` -- CognitiveWorkspace audit trail
- [ ] `crates/roko-compose/src/token_counter.rs` -- TokenCounter heuristic
- [ ] `crates/roko-compose/src/role_prompts.rs` -- RoleSystemPromptSpec, role_identity_for
- [ ] `crates/roko-compose/src/templates/common.rs` -- PromptBudget, budget_for
- [ ] `crates/roko-compose/src/templates/reviewer.rs` -- ReviewerTemplate
- [ ] `crates/roko-compose/src/context_mesh.rs` -- SharedContextEntry, ContextMesh
- [ ] `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, model selection, inline prompts
- [ ] `crates/roko-cli/src/run.rs` -- roko run path
- [ ] `crates/roko-cli/src/chat_session.rs` -- chat REPL, build_chat_system_prompt
- [ ] `crates/roko-acp/src/runner.rs` -- run_multi_role_review, inline prompts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Without `.roko/roles/` directory, existing static role identities are used (no regression)
- With `.roko/roles/implementer.toml`, the custom identity is used for Implementer role
- Tier adjustments work: Surgical gets terse identity, Full gets comprehensive
- `is_local_model("ollama/llama3.2")` returns true
- Dispatching to Ollama produces system prompt <= 4K tokens
- Dispatching to Sonnet produces system prompt <= 12K tokens
- Dispatching to Opus produces system prompt <= 24K tokens
- `BudgetPredictor.predict()` is called before assembly in `dispatch_agent_with()`
- `BudgetPredictor.record()` is called after gate results are known
- After 10+ tasks, predicted budgets are used (blended with static defaults)
- `SectionInfluence.weights()` is applied as a multiplier during composition
- Sections with negative lift are visibly demoted in budget after 20+ tasks
- `roko chat` produces system prompt with role identity and conventions
- ACP `run_multi_role_review()` uses `ReviewerTemplate` (zero inline format strings)
- Long chat sessions compact after 30+ turns
- VCG allocation activates after 5+ warm observations per bidder
- MultiPatchForager is used for context retrieval with early stopping
- Per-model attention curves are populated for 5+ model families
- Prompt A/B testing is functional via ExperimentStore
- All prompts have version tags for learning attribution
- Knowledge confidence thresholds adapt to tier
- Episode history count adapts to tier
- Content-type-aware token estimation is the default counter
- Gate pass rate by tier: Surgical >= 70%, Focused >= 80%, Full >= 85%
- Token efficiency: system prompt size / model context window <= 15% (Focused/Full), <= 30% (Surgical)
- Learning convergence: after 50 tasks, BudgetPredictor estimates within 30% of actual for 80%+ of types
- 100% of dispatches go through PromptAssemblyService or SystemPromptBuilder
- 0 dispatches with empty system prompts
- 0 inline `format!()` role descriptions
- Assembly latency: < 50ms for Surgical, < 200ms for Full (excluding knowledge store queries)
- Memory: per-dispatch allocation < 1MB
- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer builder, build methods
- `crates/roko-compose/src/prompt_assembly_service.rs` -- PromptAssemblyService, assembly pipeline
- `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model, budgets
- `crates/roko-compose/src/budget.rs` -- adjusted_budget_for, Complexity
- `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence
- `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, ModelAttentionCurves
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, should_stop_searching
- `crates/roko-compose/src/compaction.rs` -- compact_history, CompactionPolicy
- `crates/roko-compose/src/prompt.rs` -- PromptComposer, section scoring
- `crates/roko-compose/src/auction.rs` -- vcg_allocate, LearningBidder
- `crates/roko-compose/src/strategy.rs` -- CompositionStrategy, VCG warmup
- `crates/roko-compose/src/cognitive_workspace.rs` -- CognitiveWorkspace audit trail
- `crates/roko-compose/src/token_counter.rs` -- TokenCounter heuristic
- `crates/roko-compose/src/role_prompts.rs` -- RoleSystemPromptSpec, role_identity_for
- `crates/roko-compose/src/templates/common.rs` -- PromptBudget, budget_for
- `crates/roko-compose/src/templates/reviewer.rs` -- ReviewerTemplate
- `crates/roko-compose/src/context_mesh.rs` -- SharedContextEntry, ContextMesh
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, model selection, inline prompts
- `crates/roko-cli/src/run.rs` -- roko run path
- `crates/roko-cli/src/chat_session.rs` -- chat REPL, build_chat_system_prompt
- `crates/roko-acp/src/runner.rs` -- run_multi_role_review, inline prompts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
