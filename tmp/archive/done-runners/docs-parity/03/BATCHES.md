# Batch Execution Contract

Narrowed execution plan for composition parity follow-on work.

Generated: 2026-04-18

---

## Batch Posture

- Start from the shipped prompt path: `RoleSystemPromptSpec -> SystemPromptBuilder -> PromptComposer`.
- Prefer small truth-in-advertising fixes, file moves, tests, and single-path wiring.
- Do not invent new mechanism-design, eval, or distributed-context subsystems in this batch.
- If a task depends on VCG, MVT, RAGAS, CLEAR, CIV, Meta-Harness, or new learned controllers, defer it.

Required reads for any follow-on implementation run:

- [00-INDEX.md](00-INDEX.md)
- the owning section file
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

`P1 -> P4 -> P7 -> P2 -> P3 -> P6 -> P5 -> P8`

This order keeps the work bounded:

1. start with the static budget table the docs already overstate,
2. close the small live role-template gaps,
3. verify builder-path glue such as cache markers and MCP stanza behavior,
4. then check one wired complexity-budget seam,
5. improve prompt-build observability only after the budget story is clearer,
6. verify the shipped context path before touching enrichment theory,
7. keep plan-enrichment work scoped to its real runtime seam,
8. finish with scorer naming honesty and hard deferrals.

---

## Batch Overview

| Batch | Purpose | Primary Write Scope | Verify |
|------|---------|---------------------|--------|
| `P1` | Audit the static role-budget source of truth on the live prompt path | `roko-compose` budget tables, templates, tests | `cargo test -p roko-compose` |
| `P2` | Check one wired complexity-budget seam without redesigning policy | `roko-compose`, `roko-cli` prompt helpers | `cargo test -p roko-compose -p roko-cli` |
| `P3` | Tighten prompt-build observability on the live composer path | `roko-compose/src/prompt.rs`, consumers/tests | `cargo test -p roko-compose` |
| `P4` | Close the remaining live role-identity and prompt-glue gaps | `roko-compose/src/templates/`, `role_prompts.rs` | `cargo test -p roko-compose` |
| `P5` | Audit the existing enrichment runtime seam and document its real scope | `roko-cli/src/orchestrate.rs`, `roko-compose/src/enrichment/` | `cargo test -p roko-compose -p roko-cli` |
| `P6` | Verify the shipped `ContextProvider` to `ContextAssembler` path and defer larger theory | `roko-cli`, `roko-compose`, `roko-neuro` | `cargo test -p roko-neuro -p roko-compose -p roko-cli` |
| `P7` | Verify cache-marker and MCP-stanza behavior on the real builder path | `roko-compose` builder/templates/tests | `cargo test -p roko-compose` |
| `P8` | Make advanced-scorer naming honest without expanding into active-inference theory | `roko-compose/src/scorer.rs`, callsites/comments/tests | `cargo test -p roko-compose -p roko-cli` |

---

## Batch Details

### P1 — Static Role-Budget Truth

**Owns**: static template-budget truth

**Problem**: `PromptBudget` and `budget_for()` exist, but template caps and docs drift make it hard to tell what is authoritative.

**Scope**:

1. Audit where the live prompt path uses the base budget table today.
2. Reduce duplicated or unexplained cap drift where practical.
3. Prefer tests and small cleanups over large refactors.

**Out of scope**:

- complexity redesign,
- predictive allocation,
- new prompt architecture.

---

### P2 — One Wired Complexity-Budget Path

**Owns**: complexity-budget activation

**Problem**: `adjusted_budget_for()` exists, but the main runtime path does not clearly prove where complexity changes the prompt budget story.

**Scope**:

1. Check one live path only.
2. Either wire the complexity helper there or leave a precise truth-in-advertising note in code/tests.
3. Preserve existing behavior outside that path.

**Out of scope**:

- broad policy redesign,
- new complexity models,
- learned budget routing.

---

### P3 — Prompt-Build Observability

**Owns**: prompt-build metadata

**Problem**: once the budget story is narrowed, the next gap is observability: what got dropped, what got kept, and how the live prompt build explains itself.

**Scope**:

1. Keep the work on the existing composer/build metadata path.
2. Improve inspection of dropped or truncated sections if the runtime already exposes the seam.
3. Avoid widening into new budget-policy work.

**Out of scope**:

- new controller frameworks,
- new evaluation systems,
- unrelated prompt refactors.

---

### P4 — Role Identity And Prompt-Glue Cleanup

**Owns**: small role-template seams

**Problem**: most role identities are template-backed, but `Researcher`, `Conductor`, and the `Refactorer` mapping still need small honesty or ownership cleanup.

**Scope**:

1. Close one or two obvious fallback seams.
2. Keep changes local to templates and `role_prompts.rs`.
3. Preserve current role behavior.

**Out of scope**:

- new role systems,
- orchestration redesign,
- broader template-engine work.

---

### P5 — Enrichment Runtime-Scope Audit

**Owns**: strategist enrichment truthfulness

**Problem**: `EnrichmentPipeline` is real runtime code, but parity docs should not treat it as the default per-dispatch context path.

**Scope**:

1. Trace the existing runtime seam in `orchestrate.rs`.
2. Keep its documented scope honest.
3. Only wire missing glue if the seam is already intended to ship.

**Out of scope**:

- broad enrichment redesign,
- new enrichment steps,
- turning plan enrichment into a default prompt-path dependency.

---

### P6 — Context Path Proof

**Owns**: runtime context truthfulness

**Problem**: older docs still blur `ContextProvider`, the `roko-neuro` assembler, and broader context-engineering theory into one story.

**Scope**:

1. Prove the shipped `ContextProvider -> ContextAssembler` path.
2. Keep HDC/dedup claims tied to the code that actually runs.
3. Defer larger context-theory work.

**Out of scope**:

- distributed context engineering,
- agent-mesh work,
- retrieval-policy redesign beyond the shipped path.

---

### P7 — Builder-Path Cache And MCP Audit

**Owns**: builder-path glue honesty

**Problem**: cache markers and `MCP_TOOLS_STANZA` exist, but the docs over-imply uniform coverage across all prompt paths.

**Scope**:

1. Verify the real builder path and template path separately.
2. Keep any fix to one path at a time.
3. Prefer tests and comments over structural churn.

**Out of scope**:

- new caching architecture,
- new prompt layers,
- global prompt cleanup unrelated to the cache/MCP seam.

---

### P8 — Scorer Naming Honesty + Deferred Theory

**Owns**: scorer truthfulness

**Problem**: `ActiveInferenceScorer` over-claims what the code does. It is a goal-directed heuristic with hashed text embeddings and belief weights, not a formal EFE implementation.

**Scope**:

1. Rename it or tighten its contract/docs/tests.
2. Keep `SectionScorer` as the stable baseline.
3. Record VCG, MVT, distributed context engineering, and eval stack as deferred.

**Out of scope**:

- implementing EFE,
- building auctions,
- building distributed prompt coordination,
- building RAGAS/CLEAR/CIV/Meta-Harness.
