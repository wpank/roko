# 03-Composition Parity Analysis

Gap analysis of `docs/03-composition/` (14 documents) vs the actual Roko composition stack. Covers what is production-real, what is over-documented dead code, what is better handled in later learning/eval batches, and what should be executable by an unattended agent without prior context.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **prompt-budget activation + composition runtime hardening**, not as a license to implement every mechanism-design or evaluation idea in docs `07-12`.

- Prefer wiring already-shipped composition code over inventing new composition subsystems.
- Prefer batches that close one dead-code seam at a time.
- If a task starts depending on learning-policy, evaluation-policy, or distributed-systems design, record the seam and defer.
- For overnight runs, each batch should be able to stop with a clear pass/fail/block result and leave behind concrete evidence: files changed, commands run, unresolved blockers, and explicit deferrals.

Recommended single-agent serial order inside batch `03`:

`P1 -> P4 -> P7 -> P2 -> P3 -> P6 -> P5 -> P8`

Reasoning:

- `P1` and `P4` are narrow composition-owned hardening tasks with low runtime risk.
- `P7` closes a smaller but concrete system-prompt parity gap before budget activation widens the surface.
- `P2` and `P3` turn the documented budget architecture into real runtime behavior.
- `P6` hardens the live context path without forcing immediate orchestration changes.
- `P5` is the broadest runtime activation task and should happen after the surrounding composition surfaces are clearer.
- `P8` is a truth-in-advertising cleanup that should land after the scorer/runtime picture is settled.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-composer-core.md](A-composer-core.md) | 00, 01, 06 (Composer trait, PromptComposer, U-shape) | A.01-A.10 | 10/10 DONE |
| [B-system-prompt-builder.md](B-system-prompt-builder.md) | 02 (7/9-layer SystemPromptBuilder) | B.01-B.11 | 8 DONE / 2 PARTIAL / 1 NOT DONE |
| [C-role-templates.md](C-role-templates.md) | 03 (Role templates + budgets) | C.01-C.08 | 3 DONE / 5 PARTIAL |
| [D-enrichment-context.md](D-enrichment-context.md) | 04, 08 (13-step enrichment + 5-stage context) | D.01-D.12 | 7 DONE / 3 SCAFFOLD / 2 NOT DONE |
| [E-budget-management.md](E-budget-management.md) | 05 (3-tier budget architecture) | E.01-E.06 | 3 DONE / 3 PARTIAL |
| [F-advanced-allocation.md](F-advanced-allocation.md) | 07, 09, 10, 11, 12 (scoring, affect, VCG, MVT, distributed) | F.01-F.12 | 3 DONE / 6 PARTIAL / 3 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |

Doc 13 (`13-current-status-and-gaps.md`) and doc `INDEX.md` are absorbed into this file.

---

## Overall Parity: 34/59 items DONE (58%)

The composition layer is in a different state from the earlier batches:

- the **core prompt assembly path is real and already used in production**,
- but several surrounding systems are **documented as live while having zero production callers**,
- and the advanced scoring / auction / eval material is **partly implemented, partly misleadingly named, and partly future design**.

### Tier 1 — Should exist now (self-hosting relevant)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| C.04 / C.05 | `PromptBudget` / `budget_for()` exist but templates hardcode caps | PARTIAL | HIGH |
| C.08 / E.02 | complexity-adaptive budgets have zero runtime effect | PARTIAL | HIGH |
| D.01 / D.03 | `EnrichmentPipeline` and `LlmClient` are library-only | SCAFFOLD | HIGH |
| D.08 | doc 08's canonical pipeline is not the shipped context path | NOT DONE | HIGH |
| E.06.1 | no `min_tokens` / min-useful-context floor | PARTIAL | HIGH |

### Tier 2 — Should exist soon (operational quality)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| C.06 | role-template coverage incomplete (Researcher, Conductor, Refactorer) | PARTIAL | MEDIUM |
| B.09 | only 2 of 4 cache markers emitted | PARTIAL | MEDIUM |
| C.08.6 | `MCP_TOOLS_STANZA` bypasses the main role-prompt path | PARTIAL | MEDIUM |
| D.12 | HDC dedup exists as primitives but not in live context pruning | NOT DONE | MEDIUM |
| F.02 | `ActiveInferenceScorer` is neither real EFE nor runtime-wired | PARTIAL | MEDIUM |

### Tier 3 — Future / theoretical (Phase 2+)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| B.10 | learned layer reordering / `LayerOrderPolicy` | PARTIAL | LOW |
| B.11 | compression controller / per-layer compression methods | NOT DONE | LOW |
| F.06 | truthful VCG / fairness-floor attention mechanism | PARTIAL | LOW |
| F.07 | calibrated MVT foraging curves | PARTIAL | LOW |
| F.09 | Level-3 distributed context engineering | PARTIAL | LOW |
| F.10-F.12 | RAGAS / CLEAR / CIV / Meta-Harness eval stack | NOT DONE | LOW |

### Already shipped

| ID | Title | Status |
|----|-------|--------|
| A.01 | `Composer` trait | DONE |
| A.07 | `PromptComposer` core assembly | DONE |
| A.09 | U-shape placement ordering | DONE |
| B.01-B.08 | `SystemPromptBuilder` core layers | DONE / exceeds docs |
| C.01 | `RoleSystemPromptSpec` production path | DONE |
| D.09-D.11 | `ContextProvider` + tiered context types | DONE |
| E.03-E.05 | context-tier budgets + tokenizer stack | DONE |
| F.01 | `SectionScorer` | DONE |
| F.04 | affect persistence | DONE |
| F.05 | neuro knowledge injection | DONE |

---

## Execution Boundaries

Items that are real gaps but should usually be handled in later batches rather than forced into `03`:

| Item | Better Home | Why |
|------|-------------|-----|
| real EFE / active-inference learning policy | `tmp/docs-parity/05` | learning owns the actual active-inference model |
| Thompson-sampled layer ordering | `tmp/docs-parity/05` | this is learning-policy work, not core prompt activation |
| compression-controller / LLMLingua-style strategies | later composition hardening pass | budget wiring should land before semantic compression systems |
| truthful VCG, fairness floors, mechanism-design work | `05` or post-parity research pass | not required to make composition runtime-real |
| MVT patch modeling / social foraging | `05` | learning/economics ownership |
| RAGAS / CLEAR / CIV / Meta-Harness | post-parity eval pass | evaluation harness work depends on stable runtime behavior |
| distributed context engineering / agent mesh | post-parity roadmap | not a self-hosting need |

Batch `03` should generally produce:

- one real budget source of truth,
- one real complexity-aware prompt path,
- one decision about whether dormant composition subsystems are activated or explicitly deferred,
- and explicit handoffs for learning/eval-heavy work.

---

## Critical Composition Issues

1. **Budget APIs exist but are not authoritative**. `PromptBudget`, `budget_for()`, and `adjusted_budget_for()` are documented as runtime policy, while the live templates still hardcode numbers by hand.
2. **The documented enrichment pipeline is not the runtime enrichment pipeline**. The CLI uses strategist-agent enrichment in `orchestrate.rs`, while `EnrichmentPipeline` remains library code with test-only clients.
3. **The “canonical” context-assembly doc is no longer canonical**. Production uses `ContextProvider` plus `roko-neuro::ContextAssembler`, not the doc's five-stage path as written.
4. **Several role and prompt details are close but incomplete**. Missing role templates, partial cache markers, and main-path omission of `MCP_TOOLS_STANZA` make the prompt layer less uniform than the docs imply.
5. **Some advanced composition names are stronger than the implementation**. `ActiveInferenceScorer` is the clearest case: the name implies EFE-backed active inference, but the implementation is a goal-directed heuristic and is not live in the orchestrator scorer chain.

---

## Key Insight

Unlike batch `00`, composition is not missing its primary abstraction.

Unlike batch `02`, the biggest issue is not type ownership.

The core batch-`03` problem is:

**the composition core ships, but too much of the surrounding policy surface is still “designed, exported, tested, and unused.”**

That means the highest-value work here is usually:

1. make documented budget logic actually drive runtime prompts,
2. close the obvious role/prompt parity gaps,
3. decide whether dormant enrichment/context helpers are going live or staying deferred,
4. keep learning/eval-heavy work out of this batch unless it is a narrow truth-in-advertising cleanup.

---

## Batch 03 Success Definition

Batch `03` is successful when:

- prompt budgets have one real runtime source of truth,
- complexity changes the prompt path in at least one production flow,
- role-template coverage is good enough that key roles no longer fall back to generic strings,
- the live context/enrichment path is clearer and less misleading,
- and the advanced learning/eval concepts are cleanly deferred instead of being half-built.
