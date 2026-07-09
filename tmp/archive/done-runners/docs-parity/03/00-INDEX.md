# 03-Composition Parity Refresh

Audit-aligned refresh of `docs/03-composition/` against the current codebase.

Generated: 2026-04-18

---

## Batch Posture

This area was not missing its core abstractions. It was over-described.

The live composition path already exists:

- `RoleSystemPromptSpec` in `crates/roko-compose/src/role_prompts.rs:154`
- `SystemPromptBuilder` in `crates/roko-compose/src/system_prompt_builder.rs:53`
- validated CLI entrypoints in `crates/roko-cli/src/prompting.rs:50-72`
- orchestration wiring in `crates/roko-cli/src/orchestrate.rs:11709-11727` and `:14263-14295`
- budget-aware prompt composition through `PromptComposer`

The parity job for batch `03` is therefore:

1. keep the live path truthful,
2. narrow claims about adjacent helper systems,
3. defer theory-heavy material that does not ship today.

Do not treat this batch as permission to build VCG auctions, MVT routing, distributed context engineering, or a full eval harness.

---

## Section Map

| File | Current State | Audit Verdict | Notes |
|------|---------------|---------------|-------|
| [A-composer-core.md](A-composer-core.md) | Wired | `keep` | `Composer`, `PromptComposer`, and U-shape placement are real. |
| [B-system-prompt-builder.md](B-system-prompt-builder.md) | Wired | `rewrite` | The builder is already in the runtime path via `RoleSystemPromptSpec`; keep the entrypoint description smaller and more honest. |
| [C-role-templates.md](C-role-templates.md) | Mostly wired | `rewrite` | Template system is real; separate template-backed roles from inline fallbacks. |
| [D-enrichment-context.md](D-enrichment-context.md) | Mixed | `rewrite` | `EnrichmentPipeline` exists and even has a CLI runtime client, but it is not the same thing as the default prompt-time enrichment path. |
| [E-budget-management.md](E-budget-management.md) | Partial | `narrow` | Budget APIs exist; runtime still relies on hardcoded caps in templates and context-window validation. |
| [F-advanced-allocation.md](F-advanced-allocation.md) | Mostly aspirational | `defer` | Keep `SectionScorer`; treat `ActiveInferenceScorer` as heuristic; defer VCG/MVT/distributed/eval stack. |
| [BATCHES.md](BATCHES.md) | Rewritten | `rewrite` | Smaller execution batches, realistic for one agent in 90 minutes. |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | Rechecked | `keep` | Anchors updated to current files/lines. |

---

## What Exists Now

- `Composer` and `PromptComposer` are live prompt assembly primitives.
- `SystemPromptBuilder` is not a design sketch. It is the production system-prompt builder.
- `RoleSystemPromptSpec` is the typed entrypoint that feeds the builder from CLI call sites.
- Prompt-time learning hooks are real:
  - section-effectiveness snapshot from `roko-learn` is wired into orchestrate,
  - prompt experiments persist through `ExperimentStore`.
- `EnrichmentPipeline` is real code with a real CLI runtime client, but it powers the strategist enrichment phase, not the default system-prompt path.
- `ContextProvider` plus `roko-neuro::ContextAssembler` are the important live context surfaces.

---

## What Must Be Narrowed

- Describe the runtime builder path as existing now, not as a proposed seven- or nine-layer design.
- Describe templates as a real subsystem with some inline fallbacks, not as a missing prompt architecture.
- Describe budget helpers as partial plumbing, not as the active single source of truth.
- Describe `ActiveInferenceScorer` as a goal-directed heuristic. Do not imply formal expected-free-energy machinery.

---

## What Is Deferred

These remain out of scope for composition parity execution:

- VCG attention auction
- predictive foraging / MVT as a first-class runtime subsystem
- distributed context engineering
- RAGAS / CLEAR / CIV / Meta-Harness evaluation stack
- learned layer ordering and compression-controller research work

If a task in this batch needs any of the above to succeed, the task is mis-scoped.

---

## Recommended Batch Order

Use the narrowed execution contract in [BATCHES.md](BATCHES.md):

`P1 -> P4 -> P7 -> P2 -> P3 -> P6 -> P5 -> P8`

That order keeps the critical path narrow:

1. audit the static budget source-of-truth,
2. close the small role-identity seams,
3. verify cache-marker and MCP-stanza behavior on the real builder path,
4. check one wired complexity-budget seam,
5. tighten prompt-build observability,
6. confirm the shipped `ContextProvider` to `ContextAssembler` path,
7. audit the existing enrichment runtime seam without promoting it to the default path,
8. finish with scorer naming honesty and explicit deferrals.

---

## Success Definition

Batch `03` is successful when:

- the parity docs describe the shipped prompt path in present tense,
- library-only or side-path composition helpers are clearly labeled,
- template and budget claims match the code,
- advanced theory stays in deferred sections instead of live-status tables.
