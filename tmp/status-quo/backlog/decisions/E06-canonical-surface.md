# ADR: Canonical Prompt-Assembly Surface for Roko

**Status:** Accepted
**Date:** 2026-08-03
**Epic:** E06-COMPOSE-UNIFY
**Author:** E06-T01 (architectural decision task)

## Context

Roko has six parallel prompt-assembly surfaces that evolved during development:

| Surface | Location | Delegates to canonical builder? | Live? |
|---------|----------|--------------------------------|-------|
| A | `SystemPromptBuilder` (12-slot builder) | Is the canonical builder | Yes (non-default paths) |
| B | `PromptComposer` (budget knapsack + auction) | Consumes builder sections | Yes (non-default paths) |
| C | `RoleSystemPromptSpec::build*` (role->builder+composer wrapper) | Yes (A+B) | Yes |
| D | `PromptAssembler` (CLI runner-v2, `dispatch/prompt_builder.rs`) | No -- self-contained shortcut | Yes (DEFAULT `plan run`) |
| E | `PromptAssembler` (compose `templates/assembly.rs`) | No (own knapsack) | No (zero callers) |
| F | `PromptAssemblyService` (core foundation trait impl) | Delegates internally | Partial |

The default `roko plan run` path uses surface D exclusively. This means the 12-slot builder,
U-shape placement, VCG/greedy auction, affect modulation, pheromones, section-effectiveness
priority bumps, and cache markers never run on the default path.

## Decision

### Canonical surface: C via `build_role_system_prompt`

Surface C (`RoleSystemPromptSpec` exposed through `prompting.rs::build_role_system_prompt`)
is the canonical prompt-assembly entrypoint. It composes surfaces A (builder) and B (composer)
and already has live callers in `prompting.rs`, `run.rs`, `dispatch_helpers.rs`,
`prompt_helpers.rs`, and `orchestrate.rs`.

### Surface retirement plan

1. **Surface E (templates/assembly.rs): DELETE immediately.**
   Zero callers. The `PromptAssembler` struct, its `assemble_from` convenience methods,
   and the `pub mod assembly` declaration in `templates/mod.rs` are removed. The
   `cache_stability.rs` integration test is updated to use `RolePromptTemplate::sections`
   directly (it already has the template + input; it just needs a different assembly call).

2. **Surface D (CLI runner-v2 PromptAssembler): ROUTE through C, then retire authorship.**
   The `PromptAssembler::assemble()` method in `dispatch/prompt_builder.rs` is refactored
   to delegate system-prompt construction to `build_role_system_prompt` (surface C).
   The adapter types `AssembledPrompt` and `PromptContext` remain as the interface between
   the runner event loop and the dispatcher. Local markdown section authorship
   (`# Role`, `# Task`, `# Files in scope`, `# Acceptance criteria`, `# Verify`) is
   retired from the system-prompt path once the canonical builder handles those layers.
   The user-prompt path (task context, acceptance, verify rendering for the user message)
   is preserved.

3. **Surface F (PromptAssemblyService): NO CHANGE.**
   Already delegates internally. Remains as the service-layer adapter for
   `roko-orchestrator` and `roko-runtime` workflows.

### VCG verdict: Downgrade Auto to diagnostics-only (density-greedy)

VCG is downgraded from an auto-selectable strategy to a diagnostics-only mode:

- `CompositionStrategy::Auto` always resolves to `DensityGreedy`, regardless of
  bidder observation counts. The warmup threshold check is removed from the auto path.
- VCG payment summaries continue to be computed and emitted in `CompositionManifest`
  for diagnostic/monitoring purposes when the strategy is explicitly set to `Vcg`.
- `LearningBidder` persistence (E06-T06) proceeds independently for section-value
  learning, but bidder observations do not trigger an automatic flip to VCG allocation.
- Users who want VCG allocation set `composition_strategy = "vcg"` explicitly in config.

**Rationale:** The VCG warmup path has never been exercised at runtime. The observation
counters have zero non-test callers. Rather than shipping an untested auto-flip to a
mechanism-design allocator, we ship the learning infrastructure (bidder persistence,
section-effectiveness tracking) and let operators opt in to VCG explicitly. This avoids
the risk of a cold-start VCG allocator producing worse prompts than the battle-tested
density-greedy path.

## Consequences

- After E06 completes, `roko plan run` (the default path) exercises the full 12-slot
  builder with U-shape placement, affect modulation, pheromones, and cache markers.
- Section-effectiveness de-duplicates: only the compose-owned `effective_priority`
  implementation remains; the CLI-side `apply_section_effectiveness` copy is removed.
- VCG remains available as an explicit opt-in strategy, preserving the auction
  infrastructure for future experimentation.
- The `cache_stability.rs` test must be updated when surface E is deleted.
