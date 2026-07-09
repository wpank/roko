# Prompt Assembly: Issues

Cataloged issues, root causes, and severity. Each issue references the exact
code location and explains why the current behavior is problematic.

---

## Critical Issues

### ISS-01: Task TOMLs Overload Small Models with Context

**Severity:** Critical
**Symptom:** When a task is dispatched to a small/local model (Haiku, Ollama,
Gemma, DeepSeek, Phi), the system prompt can exceed the model's context window.
The model either truncates silently (losing task instructions at the end) or
errors out entirely.

**Root cause:** The per-role budgets in `templates/common.rs:budget_for()` are
designed for Opus-class models. An Implementer's total budget is ~109K characters
(~27K tokens). Even a "trivial" complexity-adjusted budget (halved workspace_map
and brief, zero prd2/context/skills) still allows ~62K characters (~15K tokens).
A Haiku model has 200K context but degrades past 32K. An Ollama/Gemma model
may have only 4K-8K effective context.

**The fix exists but is not wired:** `ContextTier` in `context_provider.rs`
defines exactly the right budgets (4K/12K/24K tokens) and `is_local_model()`
correctly identifies small models. But `dispatch_agent_with()` in `orchestrate.rs`
never calls `ContextTier::from_task_and_model()`. The tier system and the main
builder path are completely disconnected.

**Where:**
- `crates/roko-compose/src/templates/common.rs` lines 43-123 (per-role budgets)
- `crates/roko-compose/src/context_provider.rs` lines 38-75 (ContextTier, unused)
- `crates/roko-cli/src/orchestrate.rs` (dispatch_agent_with, no tier check)

**Impact:** Every task dispatched to a small model gets a prompt designed for a
200K-context model. This is the user's core pain point.

---

### ISS-02: BudgetPredictor Built But Never Called

**Severity:** Critical
**Symptom:** Token budgets are static constants. A task that historically uses
10K tokens gets the same 109K character budget as a task that uses 100K tokens.
Over-budgeting wastes context window on low-value sections. Under-budgeting
(rare with the current huge defaults) causes important context to be dropped.

**Root cause:** `BudgetPredictor` in `budget_predictor.rs` is fully implemented
with EMA-based prediction, failure inflation, partial-match fallback, and
persistence. But no caller invokes `predictor.predict()` before assembly.

**Where:**
- `crates/roko-compose/src/budget_predictor.rs` (entire file, 679 LOC, tested)
- `crates/roko-compose/src/prompt_assembly_service.rs` line 165 (`with_token_budget`)
  accepts a static value, never from predictor

**Impact:** The learning loop that would make budgets converge on actual usage
over time is completely inert.

---

### ISS-03: roko chat and dispatch_direct Bypass Builder Entirely

**Severity:** Critical
**Symptom:** `roko chat` and `roko "prompt"` (dispatch_direct) send bare prompts
to the agent with zero system prompt. No role identity, no conventions, no
knowledge injection, no anti-patterns, no playbooks. The agent operates with
no context about the project, the codebase, or prior work.

**Root cause:** `dispatch_direct.rs` spawns a Claude CLI subprocess directly
without going through `PromptAssemblyService` or `SystemPromptBuilder`. This
was the original implementation before the builder existed and was never updated.

**Where:**
- `crates/roko-cli/src/dispatch_direct.rs` (entire file)
- `crates/roko-cli/src/chat_session.rs` or equivalent chat entry point

**Impact:** The two most common interactive entry points provide the worst
prompt quality. Users get noticeably worse results from `roko chat` than from
`roko plan run` because the agent lacks all context.

---

## High Severity Issues

### ISS-04: SectionInfluence Data Not Fed Back Into Allocation

**Severity:** High
**Symptom:** Section influence is tracked (lift per section per role) but the
weights are not used to adjust budget allocation. The system collects data
about what helps and what hurts, then ignores it.

**Root cause:** `SectionInfluence` has `.weights()` and `.lift_for()` methods
that return per-section multipliers. `PromptComposer` has the machinery to
apply per-section weights. But nobody connects the two.

**Where:**
- `crates/roko-compose/src/budget_predictor.rs` lines 276-375 (SectionInfluence)
- `crates/roko-compose/src/prompt.rs` (PromptComposer, accepts section priorities)
- Gap: no caller maps influence weights -> section priorities

**Impact:** The system cannot learn that (for example) "prd2 sections hurt
Implementer success rate" and stop including them.

---

### ISS-05: ACP Runner Inline Prompts Duplicate Templates

**Severity:** High
**Symptom:** `run_multi_role_review()` in `roko-acp/runner.rs` hardcodes full
role descriptions for "Architect Reviewer" and "Security & Correctness Auditor"
in `format!()` strings. These descriptions partially duplicate and partially
conflict with `ReviewerTemplate` in `templates/reviewer.rs`.

**Root cause:** The ACP runner was written before the template system was
wired. It was never updated to use templates.

**Where:**
- `crates/roko-acp/src/runner.rs` lines ~515-593 (run_multi_role_review)
- `crates/roko-compose/src/templates/reviewer.rs` (ReviewerTemplate)

**Impact:** Role behavior diverges between the ACP path and the orchestrator
path. Bug fixes to reviewer behavior must be applied in two places.

---

### ISS-06: Static Per-Role Budgets Use Character Counts, Not Tokens

**Severity:** High
**Symptom:** `PromptBudget` fields are in characters. The system uses a flat
4:1 ratio (`estimate_tokens = len / 4`) to convert. This ratio is inaccurate
for code-heavy content (closer to 3:1 for Rust code with long identifiers)
and for content with many special characters or whitespace.

**Root cause:** Real tokenization requires calling the provider's tokenizer,
which is unavailable offline. The heuristic was chosen for speed. But the
error compounds: a 50K character plan budget might be 12.5K tokens for prose
but 16.7K tokens for dense Rust code.

**Where:**
- `crates/roko-compose/src/prompt.rs` line 24 (`estimate_tokens = len / 4`)
- `crates/roko-compose/src/token_counter.rs` (TokenCounter::Heuristic with
  configurable chars_per_token, defaults to 4.0)

**Impact:** Budget enforcement is imprecise. Content-type-aware ratios
(3.0 for code, 4.0 for prose, 5.0 for markdown with lots of whitespace)
would improve accuracy without requiring real tokenization.

---

### ISS-07: Per-Model Attention Curves Are Empty

**Severity:** High
**Symptom:** `ModelAttentionCurves` supports per-model U-curve parameters but
only the default curve is populated. Claude Opus, Sonnet, and Haiku have
different attention profiles (Haiku is more sensitive to middle-section
degradation than Opus), but the same curve is used for all.

**Root cause:** Fitting per-model curves requires running placement experiments
(put critical information at different positions, measure task success rate).
This data collection has not been done.

**Where:**
- `crates/roko-compose/src/attention.rs` lines 58-82 (ModelAttentionCurves)

**Impact:** Section placement optimization is the same for all models, even
though models differ significantly in their sensitivity to position.

---

### ISS-08: MultiPatchForager Not Instantiated in Dispatch

**Severity:** High
**Symptom:** Context retrieval in `dispatch_agent_with()` uses direct queries
to the knowledge store, code index, and playbook store. The foraging optimizer
that would determine optimal visitation order and stopping criteria is not used.

**Root cause:** The forager was built as a standalone utility. Wiring it into
the dispatch path requires constructing `SourceForagingProfile` entries for
each context source, which requires calibration data (g_max, lambda, travel_cost)
that has not been measured.

**Where:**
- `crates/roko-compose/src/foraging.rs` (MultiPatchForager, built and tested)
- `crates/roko-cli/src/orchestrate.rs` (dispatch_agent_with, direct queries)

**Impact:** Context retrieval is not optimized for time/relevance tradeoff.
Every source is queried unconditionally, even when marginal value is low.

---

## Medium Severity Issues

### ISS-09: Conversation Compaction Not Wired into roko chat

**Severity:** Medium
**Symptom:** Long `roko chat` sessions eventually hit context window limits.
The conversation history grows without bound. Users must manually manage
context by starting new sessions.

**Root cause:** `compact_history()` in `compaction.rs` is fully implemented
with anchor preservation, gate result carry-forward, and iterative
summarization. But the chat REPL loop does not call it.

**Where:**
- `crates/roko-compose/src/compaction.rs` (compact_history, ready)
- `crates/roko-cli/src/chat_session.rs` (no compaction call)

**Impact:** Long sessions degrade. The solution is built and waiting.

---

### ISS-10: VCG Payments Are Diagnostic-Only But Not Marked As Such

**Severity:** Medium
**Symptom:** The VCG auction computes per-section payments (externality-based),
but payments do not affect allocation. The greedy sort already determines which
sections are included. Payments are metadata in the `CompositionManifest`.

**Root cause:** VCG was designed as a mechanism for welfare-maximizing
allocation, but the current implementation uses greedy allocation and then
computes payments post-hoc. This is conceptually confused -- VCG payments
are meaningful only when the allocation is actually VCG-optimal.

**Where:**
- `crates/roko-compose/src/auction.rs` lines 1-688 (vcg_allocate)

**Impact:** Code complexity without functional benefit. The auction module
is 688 LOC of sophisticated mechanism design that is purely decorative in
the current runtime. Consider either:
1. Actually using VCG allocation (replace greedy with combinatorial optimization)
2. Removing the payment computation and keeping only the bidder framework

---

### ISS-11: Section Effectiveness Threshold Is Binary

**Severity:** Medium
**Symptom:** Sections with effectiveness score < 0.1 are excluded entirely.
Sections with score >= 0.1 are included at full budget. There is no gradual
degradation -- a section at 0.09 is dropped, a section at 0.11 gets full
allocation.

**Root cause:** The `should_include()` check in `prompt_assembly_service.rs`
is a hard threshold. The `effective_budget_ratio()` method does compute a
weighted sum, but this only affects the overall budget scaling, not per-section
caps.

**Where:**
- `crates/roko-compose/src/prompt_assembly_service.rs` lines 185-189 (should_include)
- `crates/roko-compose/src/prompt_assembly_service.rs` lines 193-217 (effective_budget_ratio)

**Fix:** Use effectiveness scores as per-section budget multipliers, not just
inclusion/exclusion gates. A section with score 0.3 should get 30% of its
normal budget, not 100%.

---

### ISS-12: Workspace Map Cap Is Fixed at 200 Lines

**Severity:** Medium
**Symptom:** Large codebases (like roko with 18 crates) hit the 200-line
workspace map cap and lose file listings for later crates. The cap is the
same regardless of model context window or task relevance.

**Root cause:** `WORKSPACE_MAP_LINE_LIMIT = 200` in `prompt_assembly_service.rs`
is a constant. No adaptation to available budget or task scope.

**Where:**
- `crates/roko-compose/src/prompt_assembly_service.rs` line 22

**Fix:** Make the cap proportional to the context tier budget. Surgical: 50 lines.
Focused: 150 lines. Full: 300 lines. Extended: 500 lines. Or better: filter the
workspace map to show only files relevant to the current task's crate/module.

---

### ISS-13: Knowledge Confidence Thresholds Are Hardcoded

**Severity:** Medium
**Symptom:** Domain knowledge requires confidence >= 0.5 for inclusion.
Techniques require >= 0.3. Anti-patterns require >= 0.2. These thresholds
are appropriate for Opus but may be too permissive for small models (including
low-confidence knowledge wastes precious context) or too restrictive for
exploratory tasks (where uncertain knowledge could be valuable).

**Where:**
- `crates/roko-compose/src/prompt_assembly_service.rs` lines 548, 222-224, 237-239

**Fix:** Make confidence thresholds model-tier-dependent:
- Surgical: 0.8 (only high-confidence, proven knowledge)
- Focused: 0.5 (current defaults)
- Full: 0.3 (include more speculative knowledge)

---

## Low Severity Issues

### ISS-14: Episode Context Always Shows Last 5

**Severity:** Low
**Symptom:** `format_episode_context()` always takes the last 5 episodes
regardless of model tier or relevance to the current task.

**Where:**
- `crates/roko-compose/src/prompt_assembly_service.rs` line 344

**Fix:** Surgical tier: 0-1 episodes. Focused: 3 episodes. Full: 5 episodes.
Also filter to episodes with the same role or domain as the current task.

---

### ISS-15: Convention Detection Reads Up to 12 Source Files

**Severity:** Low
**Symptom:** `SOURCE_SAMPLE_LIMIT = 12` means convention detection reads at
most 12 source files. For large codebases, this may not be representative.
For small models, reading 12 files is expensive I/O for content that may
be truncated or excluded anyway.

**Where:**
- `crates/roko-compose/src/prompt_assembly_service.rs` line 21

---

### ISS-16: Token Counter Has No Provider-Specific Modes

**Severity:** Low
**Symptom:** `TokenCounter::Heuristic { chars_per_token: 4.0 }` is the only
counter used. No cl100k_base or model-specific tokenizer.

**Where:**
- `crates/roko-compose/src/token_counter.rs`

**Fix:** Add optional tiktoken-rs integration for offline token counting
when high accuracy is needed (e.g., when the budget is tight on a small model).

---

### ISS-17: Orchestrate.rs Retry/Replan Prompts Are Inline format!()

**Severity:** Low
**Symptom:** Gate failure retry hints, model escalation prompts, and
verification-failed fix prompts are all inline `format!()` strings in
`orchestrate.rs`. They bypass the template system.

**Where:**
- `crates/roko-cli/src/orchestrate.rs` (multiple locations, see AUDIT.md section 10)

**Fix:** Create template variants for retry/escalation/replan scenarios.

---

## Issue Dependency Graph

```
ISS-01 (overloaded small models)
  |-- depends on --> ISS-02 (no budget prediction)
  |-- depends on --> ISS-06 (char vs token budgets)
  |-- requires --> ContextTier wiring into dispatch

ISS-03 (chat bypasses builder)
  |-- independent

ISS-04 (influence not fed back)
  |-- depends on --> ISS-02 (predictor not wired)

ISS-05 (ACP inline prompts)
  |-- independent

ISS-08 (forager not instantiated)
  |-- nice-to-have after ISS-01 is fixed
```

---

## Sources

- `crates/roko-compose/src/prompt_assembly_service.rs` -- thresholds, limits, should_include
- `crates/roko-compose/src/templates/common.rs` -- budget_for(), PromptBudget
- `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model
- `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence
- `crates/roko-compose/src/budget.rs` -- adjusted_budget_for(), Complexity
- `crates/roko-compose/src/attention.rs` -- ModelAttentionCurves
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager
- `crates/roko-compose/src/compaction.rs` -- compact_history
- `crates/roko-compose/src/auction.rs` -- vcg_allocate
- `crates/roko-compose/src/prompt.rs` -- estimate_tokens
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, inline prompts
- `crates/roko-cli/src/dispatch_direct.rs` -- no system prompt
- `crates/roko-acp/src/runner.rs` -- run_multi_role_review inline prompts
