# Finding: Trait Sufficiency

> Are six Synapse traits sufficient? Analysis of boundary operations, a 7th-trait candidate,
> and merge candidates — all of which the architecture handles correctly.

**Status**: Analysis
**Crate**: `roko-core`
**Depends on**: [Six Synapse Traits](../../reference/05-operators/README.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

Six traits are sufficient. The three boundary operations (signal transformation, telemetry
emission, batch verification) fit as degenerate Composer and Policy. A Transform trait was
evaluated and rejected. All four merge candidates were rejected because traits differ in at
least two of: sync/async, stateful/stateless, input cardinality, output type, and layer
assignment.

**Verdict: Keep six.**

---

## Methodology

Analyzed all 131 trait implementations found in the codebase:

| Trait | Implementation count | Concrete types |
|---|---|---|
| Substrate | 4 | MemorySubstrate, FileSubstrate, HdcSubstrate, ChainSubstrate |
| Scorer | 7 | SumScorer, MulScorer, ConstScorer, RelevanceScorer, ReputationScorer, ToolRelevanceScorer, NoOpScorer |
| Gate | 33 | ShellGate, PropertyTestGate, GeneratedTestGate, IntegrationGate, WalletGate, TxSimGate, VerifyChainGate, LlmJudgeGate, FactCheckGate, SymbolGate, GatePipeline, OrderedGate, MockGate, NoOpGate, and 19 more |
| Router | 7 | FirstRouter, HighestScoreRouter, RoundRobinRouter, CascadeRouter, LinUCBRouter, WeightedRouter, NoOpRouter |
| Composer | 5 | PromptComposer, ContextPackComposer, PlanComposer, SystemPromptBuilder, NoOpComposer |
| Policy | 4 | EpisodePolicy, ConductorPolicy, PheromonPolicy, NoOpPolicy |

Also searched for TODO/HACK/FIXME/workaround markers near trait usage. Found 8 TODOs, all in
UI/API boundary code (`roko-cli/src/tui/`, `roko-serve/src/routes/`), none in core trait usage.

---

## Boundary Operations

Three operations sit at the boundary of the trait model:

| Operation | Current Implementation | Fit Quality |
|---|---|---|
| **Signal transformation** (e.g., summarize, translate) | `Composer::compose(&[single], &Budget::UNLIMITED, ...)` | Adequate. Budget parameter is unused but harmless. |
| **Telemetry emission** (metrics, traces) | `Policy::decide(&[], ctx)` returning metric Engrams | Adequate. Empty stream input is awkward but functional. |
| **Batch verification** (verify N signals at once) | Loop calling `Gate::verify` N times | Adequate. External loop is standard; batch Gate would be premature optimization. |

---

## The 7th Trait Candidate: Transform

**Candidate**: `fn transform(signal: &Signal, ctx: &Context) -> Signal`

A dedicated Transform trait would capture 1:1 Signal→Signal mappings without budget or stream
semantics. Examples: summarize text, translate language, extract structured data.

**Arguments for:**
- Cleaner API for 1:1 transformations (no meaningless budget parameter)
- Semantic clarity: "transform" is a distinct cognitive operation from "compose"

**Arguments against:**
- Parsimony: 6 traits → 720 orderings; 7 traits → 5,040 orderings. Combinatorial explosion.
- Degenerate Composer handles it: `Composer::compose(&[x], &Budget::UNLIMITED, ...)` works.
- Cognitive load: "one noun, six verbs" is a powerful mnemonic.

**Verdict: Keep six.** The benefit of parsimony outweighs the API awkwardness. The mnemonic
is valuable for onboarding and architectural reasoning. If a future domain produces dozens of
budget-free, stream-free transforms, reconsider.

---

## Merge Candidates

| Candidate Merge | Argument For | Argument Against | Verdict |
|---|---|---|---|
| Scorer + Router | Both evaluate signals | Router has `feedback()` (stateful); Scorer is stateless and pure | **No merge** |
| Gate + Scorer | Both assess quality | Gate is async (external I/O); Scorer is sync (pure computation). Gate returns Verdict with rich evidence; Scorer returns Score. | **No merge** |
| Policy + Gate | Both examine outputs | Policy is reactive (many→many, no verdict); Gate is verificatory (one→Verdict). Fundamentally different cardinalities. | **No merge** |
| Scorer + Gate | Could merge into "Assessor" | Different output types, different execution models, different layer assignments | **No merge** |

All merge candidates fail because the traits differ in at least two of: sync/async,
stateful/stateless, input cardinality, output type, and layer assignment.

---

## Comparison to Other Trait-Based Agent Systems

| System | Number of Core Abstractions | Roko Equivalent |
|---|---|---|
| **CoALA** (Sumers et al. 2023) | 5 memories + 3 action types | Roko's 6 traits subsume CoALA's decomposition |
| **LIDA** (Franklin et al. 2016) | Codelets (perception, attention, action, learning) | Each codelet type maps to a trait implementation |
| **Google multi-agent patterns** (2025) | 3 execution primitives (sequential, loop, parallel) | Orchestrator composes trait calls in these patterns |
| **Agent Design Pattern Catalogue** (arXiv:2405.10467) | 18 patterns | Patterns compose from trait implementations; not a competing decomposition |

Roko's trait decomposition is **coarser than codelet architectures** (LIDA may have hundreds
of codelet types) but **finer than framework abstractions** (LangChain's "chain" is coarser
than any single Synapse trait). The six-trait level appears to be the right granularity for
a Rust trait system: fine enough for meaningful composition, coarse enough for human reasoning.

---

## Related Findings

- [F3 — Layer Taxonomy](03-finding-layer-taxonomy.md): Trait assignments are part of layer classification.
- [F10 — Cross-Cut Isolation](06-finding-crosscut-isolation.md): Cross-cuts interact with trait injection patterns.
- [07 — Category Theory](07-finding-category-theory.md): Formal analysis of traits as morphisms.

## References

- Koopmans et al. (2024). "Agent Design Pattern Catalogue." arXiv:2405.10467
- Google Cloud (2025). "Choose a Design Pattern for Agentic AI Systems."
- Franklin, S. et al. (2016). "LIDA: A Systems-level Architecture." IEEE Trans. AMD 6(1)
- Sumers, T. et al. (2023). "Cognitive Architectures for Language Agents (CoALA)." arXiv:2309.02427

## Open Questions

- If the Transform use-case grows (e.g., a major Neuro feature needs many 1:1 transforms),
  is the 5,040-ordering combinatorial cost acceptable?
