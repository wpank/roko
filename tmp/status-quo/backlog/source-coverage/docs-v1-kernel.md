# docs/v1 Kernel Source Coverage

Purpose: ledger for the DOC-v1-kernel executable plan. The plan covers the
kernel slice of `docs/v1`: root markdown files plus `00-architecture`,
`01-orchestration`, `02-agents`, `03-composition`, and `04-verification`.

Initial status for every row is `pending-reconciliation`. When the matching
DOC-v1 task runs, replace that with one of:

- `mapped`: existing E01-E18 task refs cover the source requirement.
- `doc-follow-up`: a DOC-v1 follow-up task was added or refined in
  `tmp/status-quo/backlog/plans/DOC-v1-kernel/tasks.toml`.
- `deferred`: target-state/spec-debt material is intentionally out of the
  status-quo backlog, with a reason.
- `no-op`: source is index/navigation/context only, with a reason.

## Summary

| Area | Source docs | Local reconciliation task |
|---|---:|---|
| Root overview and positioning | 7 | `DOC-V1-ROOT-OVERVIEW` |
| Root references | 4 | `DOC-V1-ROOT-REFERENCE` |
| `00-architecture` | 39 | `DOC-V1-ARCHITECTURE` |
| `01-orchestration` | 15 | `DOC-V1-ORCHESTRATION` |
| `02-agents` | 18 | `DOC-V1-AGENTS` |
| `03-composition` | 15 | `DOC-V1-COMPOSITION` |
| `04-verification` | 15 | `DOC-V1-VERIFICATION` |
| **Total** | **113** | |

## Coverage Rows

| Source | Local task | Status | Backlog mapping / follow-up / reason |
|---|---|---|---|
| `docs/v1/COMPARISON.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/EXECUTIVE-SUMMARY.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/INDEX.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/QUICKSTART.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/STATUS.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/USE-CASES.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/VISION-RUN-ANYWHERE.md` | `DOC-V1-ROOT-OVERVIEW` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/API-REFERENCE.md` | `DOC-V1-ROOT-REFERENCE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/BENCHMARKS.md` | `DOC-V1-ROOT-REFERENCE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/CLI-REFERENCE.md` | `DOC-V1-ROOT-REFERENCE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/INTEGRATION-GUIDE.md` | `DOC-V1-ROOT-REFERENCE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/00-vision-and-thesis.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/01-naming-and-glossary.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/02-engram-data-type.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/02b-pulse-ephemeral-event.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/03-score-7-axis-appraisal.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/04-decay-variants.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/05-provenance-and-attestation.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/06-synapse-traits.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/07-substrate-trait.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/07b-bus-transport-fabric.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/08-scorer-gate-router-composer-policy.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/09-universal-cognitive-loop.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/10-three-cognitive-speeds.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/11-dual-process-and-active-inference.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/12-five-layer-taxonomy.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/13-cognitive-cross-cuts.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/14-c-factor-collective-intelligence.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/15-crate-map.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/16-autocatalytic-and-cybernetics.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/17-design-principles-and-frontier-summary.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/18-decay-tier-matrix.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/19-compositional-kinds.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/20-configuration-schema.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/21-performance-numerical-stability.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/22-error-handling-recovery.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/23-architectural-analysis-improvements.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/24-cross-section-integration-map.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/25-attention-as-currency.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/26-cognitive-immune-system.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/27-temporal-knowledge-topology.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/28-emergent-goal-structures.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/29-cognitive-energy-model.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/30-cross-pollination-innovations.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/31-implementation-readiness-audit.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/32-comprehensive-test-strategy.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/33-refactor-plan-phases.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/34-synergy-integration-map.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/35-consolidated-roadmap.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/00-architecture/INDEX.md` | `DOC-V1-ARCHITECTURE` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/00-layer-overview.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/01-plan-discovery.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/02-unified-task-dag.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/03-parallel-executor.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/04-plan-phases.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/05-executor-actions.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/06-runtime-harness.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/07-worktree-isolation.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/08-merge-queue.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/09-snapshot-recovery.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/10-event-log.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/11-conductor-integration.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/12-stigmergy-niche.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/13-cross-domain-orchestration.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/01-orchestration/INDEX.md` | `DOC-V1-ORCHESTRATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/00-agent-trait.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/01-provider-registry.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/02-provider-adapters.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/03-chat-types.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/04-agent-roles.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/05-agent-pools.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/06-mcp-integration.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/07-tool-loop.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/08-harness-engineering.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/09-format-translation.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/10-temperament-profiling.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/11-dual-process-routing.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/12-extensibility.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/13-creation-sites.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/14-provider-integrations.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/15-status-gaps.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/16-domain-profiles.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/02-agents/INDEX.md` | `DOC-V1-AGENTS` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/00-composer-trait.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/01-prompt-composer.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/02-system-prompt-builder-7-layer.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/03-role-templates.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/04-enrichment-pipeline-13-step.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/05-token-budget-management.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/06-lost-in-the-middle-u-shape.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/07-active-inference-context-selection.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/08-5-stage-assembly-pipeline.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/09-predictive-foraging-mvt.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/10-vcg-attention-auction.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/11-distributed-context-engineering.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/12-affect-modulated-retrieval.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/13-current-status-and-gaps.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/03-composition/INDEX.md` | `DOC-V1-COMPOSITION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/00-gate-trait.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/01-gate-implementations.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/02-6-rung-selector.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/03-gate-pipeline.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/04-artifact-store.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/05-ratcheting.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/06-adaptive-thresholds.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/07-process-reward-models.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/08-agent-feedback-from-gates.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/09-evaluation-lifecycle.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/10-autonomous-eval-generation.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/11-evoskills.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/12-forensic-ai-causal-replay.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/15-verdicts-as-signals.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |
| `docs/v1/04-verification/INDEX.md` | `DOC-V1-VERIFICATION` | pending-reconciliation | Map to E01-E18, add DOC follow-up, or record no-op/deferred reason. |

## Validation Commands

```sh
cargo run -q -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/DOC-v1-kernel

for p in $(find docs/v1 -maxdepth 1 -type f -name '*.md' | sort; find docs/v1/00-architecture docs/v1/01-orchestration docs/v1/02-agents docs/v1/03-composition docs/v1/04-verification -type f -name '*.md' | sort); do
  grep -Fq "$p" tmp/status-quo/backlog/source-coverage/docs-v1-kernel.md || exit 1
done
```
