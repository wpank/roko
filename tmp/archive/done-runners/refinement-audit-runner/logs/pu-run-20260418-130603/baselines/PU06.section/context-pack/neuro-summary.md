# Neuro Summary — Batch 06

Concise runtime picture for agents entering `06` without prior context.

## What Is Already Real

- `KnowledgeEntry`, `KnowledgeKind`, `KnowledgeTier`, decay, AntiKnowledge floor, and append-only neuro persistence are real.
- the core HDC stack in `roko-primitives` is solid and reused across the workspace.
- `ContextAssembler` is not a stub; it already contains budgeting, ranking, compression, and somatic/PAD bias logic.
- `Distiller`, `TierProgression`, Dreams-cycle analysis, and orchestrator-side tier feedback all exist.
- `SomaticLandscape` and strategy-space primitives are real.
- `roko-golem` is gone from the workspace.

## What Is Misleading Today

- the main orchestrator still bypasses `ContextAssembler`,
- the query contract is real but too implicit,
- doc `08` still reads like cross-domain transfer is built when its main types are absent,
- backup / restore / publish is still doc-only,
- some later status docs still understate Dreams, pheromone, and transfer-adjacent surfaces that already ship in narrower forms.

## What Batch 06 Should Usually Do

1. activate the best existing retrieval seam,
2. make the query and distillation contracts easier to execute and verify,
3. define bounded ownership for ingest sources and backup/restore,
4. keep exchange, chain, and frontier work explicitly deferred unless a batch clearly owns it.
