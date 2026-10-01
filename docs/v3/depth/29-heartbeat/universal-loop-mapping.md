# Universal Loop Mapping: CoALA to Synapse

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/01-universal-loop-mapping.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS3.

---

## 1. Overview

Roko's older 9-step pipeline, a CoALA-inspired elaboration (OBSERVE, RETRIEVE, ANALYZE, GATE, SIMULATE, VALIDATE,
EXECUTE, VERIFY, REFLECT) is the legacy framing that guided Roko's initial cognitive
architecture. Roko's canonical universal loop is now the seven-step SENSE / ASSESS /
COMPOSE / ACT / VERIFY / PERSIST + BROADCAST / REACT loop.

The universal Synapse loop is not a different pipeline. It is the same runtime
expressed in terms of the composable Synapse traits (Substrate, Bus, Scorer, Gate,
Router, Composer, Policy) plus the cognitive cross-cuts. CoALA is the theoretical
frame; the Synapse loop is the implementation frame. Domain-specific heartbeat
variants (chain, coding, research) are parameterizations of the universal loop,
not separate architectures. `HeartbeatPolicy` emits the tick Pulses and domain
consumers subscribe, rather than importing their own schedulers.

---

## 2. Side-by-Side Mapping Table

| Historical framing | Canonical loop | Synapse trait(s) | Layer | Description |
|---|---|---|---|---|
| OBSERVE + RETRIEVE | **SENSE** | `Substrate.query()`, `Bus.subscribe()` | L0 | Read durable Signals, live Pulses, and external I/O |
| ANALYZE + GATE | **ASSESS** | `Scorer.score()`, `Router.select()` | L1/L2 | Score candidates, compute surprise/confidence, choose tier |
| _(implicit)_ | **COMPOSE** | `Composer.compose()` | L2 | Build the prompt or action bundle under budget |
| EXECUTE | **ACT** | `Agent.execute()` | L1 | Execute the selected action: LLM, tool, or chain call |
| SIMULATE + VALIDATE + VERIFY | **VERIFY** | `Gate.verify()`, domain gates | L3 | Pre-flight, safety, stream-gates, ground-truth verification |
| _(persistence folded into REFLECT)_ | **PERSIST** | `Substrate.put()` | L0 | Store durable Signals with lineage and provenance |
| _(historically under-described)_ | **BROADCAST** | `Bus.publish()` | L0 | Publish ephemeral Pulses for subscribers |
| REFLECT | **REACT** | `Policy.decide()` | L3-L4 | Observe, emit follow-on Pulses and Signals |

### 2.1 Key Differences

1. **COMPOSE is explicit.** CoALA assumes context is assembled somehow; Synapse
   makes it a first-class step with a dedicated trait (`Composer`). This reflects
   the thesis that context engineering is the primary determinant of agent
   performance (Meta-Harness, Lee et al. 2026, arXiv:2603.28052).

2. **PERSIST and BROADCAST are both explicit.** CoALA lumps persistence into
   REFLECT. Synapse separates durable storage from transport because content-
   addressed Signal storage and topic-addressed Pulse delivery are both
   foundational concerns. That separation keeps the Bus visible.

3. **REACT absorbs the old REFLECT tail.** CoALA's REFLECT covered both reactive
   adaptation and meta-cognitive assessment. In the canonical loop, the reactive
   part becomes REACT, while self-assessment is handled by injected cross-cuts
   rather than a ninth step.

4. **SIMULATE/VALIDATE are domain-specific extensions.** The universal loop does
   not add extra steps. They inject into VERIFY by domain-specific agent types.

---

## 3. Layer Traversal

A single tick of the cognitive loop crosses the full five-layer stack:

```
L0 Runtime     --> Substrate.query()      [SENSE: fetch from storage]
L0 Runtime     --> Bus.subscribe()        [SENSE: listen for live Pulses]
L2 Scaffold    --> Scorer.score()         [ASSESS: score relevance]
L1 Framework   --> Router.select()        [ASSESS: choose cognitive tier]
L2 Scaffold    --> Composer.compose()     [COMPOSE: build context window]
L1 Framework   --> Agent.execute()        [ACT: call LLM backend]
L3 Harness     --> Gate.verify()          [VERIFY: check against ground truth]
L0 Runtime     --> Substrate.put()        [PERSIST: store with lineage]
L0 Runtime     --> Bus.publish()          [BROADCAST: deliver Pulses]
L3-L4 Harness  --> Policy.decide()        [REACT: detect patterns, emit reactions]
Cross-cut      --> Daimon.assess()        [injected into ASSESS/ACT]
```

Every tick traverses L0 -> L2 -> L1 -> L2 -> L1 -> L3 -> L0 -> L0 -> L3-L4 with
cross-cut injections. Dependencies flow strictly downward. Cross-cutting concerns
(Neuro, Daimon, Dreams) are injected via trait objects, never via direct imports
of higher layers.

---

## 4. Domain Parameterization

The universal loop is one loop, parameterized by domain. Domain-specific behavior
comes from domain-specific trait implementations and configuration, not from
architectural modifications.

### 4.1 Coding Agent

Uses the universal loop as-is. No additional steps between ASSESS and ACT.

```
SENSE     -> FileSubstrate.query() + Bus.subscribe()
ASSESS    -> RecencyScorer + ReputationScorer + CascadeRouter
COMPOSE   -> PromptComposer [code-aware templates, U-shape placement]
ACT       -> Agent.execute() [call Claude/GPT, produce code changes]
VERIFY    -> CompileGate -> TestGate -> ClippyGate -> DiffGate
PERSIST   -> FileSubstrate.put() [store with lineage]
BROADCAST -> Bus.publish() [emit Pulses for subscribers]
REACT     -> EpisodePolicy + DaimonPolicy + PredictionPolicy
```

### 4.2 Chain Agent

Adds SIMULATE (mirage-rs pre-flight) and VALIDATE (position limits) inside VERIFY
rather than as standalone universal steps.

```
SENSE     -> FileSubstrate.query() + ChainSubstrate.query() + Bus.subscribe()
ASSESS    -> RecencyScorer + CatalystScorer + PredictiveScorer + Router.select()
COMPOSE   -> AttentionAuction + PromptComposer [VCG bidding for context budget]
ACT       -> Agent.execute() [submit transaction, invoke tools]
VERIFY    -> VerifyChainGate + TxSimGate + WalletGate
PERSIST   -> FileSubstrate.put() + ChainSubstrate.put()
BROADCAST -> Bus.publish() + ChainBus.publish()
REACT     -> EpisodePolicy + DaimonPolicy + PredictionPolicy + CFactorPolicy
```

### 4.3 Research Agent

Simplified loop. SIMULATE and VALIDATE are typically skipped.

```
SENSE     -> MemorySubstrate.query() + Bus.subscribe()
ASSESS    -> RecencyScorer + Router.select()
COMPOSE   -> PromptComposer [research-focused templates, large budget]
ACT       -> Agent.execute() [web search, synthesis, citation tracking]
VERIFY    -> LlmJudgeGate [subjective quality check]
PERSIST   -> MemorySubstrate.put() [store findings]
BROADCAST -> Bus.publish() [emit research Pulses]
REACT     -> EpisodePolicy + DaimonPolicy
```

---

## 5. The Translation from Legacy Architecture

```
Historical pipeline:              Canonical universal loop:
1. Observe
2. Retrieve                    ->  SENSE

3. Analyze
4. Gate                        ->  ASSESS

(implicit context assembly)    ->  COMPOSE

5. Simulate
6. Validate
7. Execute
8. Verify                      ->  ACT + VERIFY

9. Reflect                     ->  PERSIST + BROADCAST + REACT
```

The key change: the old heartbeat was a monolithic chain-specific pipeline. The
canonical universal loop is a composable, domain-agnostic architecture where each
step is a pluggable trait implementation. This enables:

- **Cross-domain agents**: A single agent can write Solidity contracts (coding
  domain), simulate deployment (chain domain), and research competing protocols
  (research domain) -- all using the same universal loop.
- **New domains without kernel changes**: Adding medical, legal, or scientific
  domains requires implementing domain-specific traits (Gates, Scorers, Probes)
  but no modifications to the universal loop.
- **Composable verification**: Gates from different domains can be chained:
  `CompileGate -> TestGate -> TxSimGate -> VerifyChainGate`.

---

## 6. Formal Relationship: CoALA as Theory, Synapse as Implementation

| Dimension | CoALA | Synapse Loop |
|---|---|---|
| **Level** | Theoretical taxonomy | Concrete trait-based implementation |
| **Composability** | Describes phases | Each phase is a pluggable trait implementation |
| **Domain** | Agnostic (by design) | Agnostic (by implementation) |
| **Memory** | "Internal actions: retrieval, reasoning, learning" | Explicit Substrate + Neuro + Scorer chain |
| **Verification** | "Grounding actions" | Explicit Gate pipeline with domain-specific injections |
| **Affect** | Not modeled | Daimon cross-cut injected into ASSESS and ACT |
| **Multi-scale** | Single cycle | Three concurrent scales (Gamma/Theta/Delta) |
| **Cost optimization** | Not modeled | T0/T1/T2 gating (~80% free ticks) |
| **Meta-cognition** | Part of "learning" | Cross-cut regulation injected into ASSESS, ACT, speed selection |

CoALA provides the intellectual justification. The Synapse loop provides the
engineering realization. The two are not in tension -- they are layers of the
same architecture.

---

## 7. References

- **Sumers, Yao, Narasimhan & Griffiths 2023** -- "Cognitive Architectures for
  Language Agents" (arXiv:2309.02427). The CoALA framework.
- **Lee et al. 2026** -- "Meta-Harness: Optimizing Agent Scaffolds"
  (arXiv:2603.28052). Evidence that scaffold optimization matters more than
  model selection.
- **Conant & Ashby 1970** -- "Every Good Regulator of a System Must Be a Model
  of That System" (International Journal of Systems Science 1(2)).
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience
  11(2)). Prediction error as the organizing signal.

---

## Cross-References

- `docs/v3/depth/29-heartbeat/coala-9-step-pipeline.md` -- CoALA theoretical foundation
- `docs/v3/depth/29-heartbeat/chain-heartbeat-variant.md` -- Chain domain extension
- `docs/v3/depth/29-heartbeat/attention-auction-and-gating.md` -- VCG COMPOSE mechanism
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
