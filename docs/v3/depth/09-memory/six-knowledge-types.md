# Six Knowledge Types

> **v3 depth -- 09-memory** | Source: v1/06-neuro/01

Every knowledge entry in Neuro is classified into one of six semantic
categories, each with a distinct half-life, retrieval behavior, and role in
the agent's cognitive lifecycle.

The type system is critical because different kinds of knowledge behave
differently. An Insight ("Rust's borrow checker errors often mean you need
Arc here") needs regular revalidation -- it has a 30-day base half-life.
A Warning ("Never use `unwrap()` in production paths") must be aggressively
current -- it has a 1-hour base half-life and degrades quickly if not
reconfirmed. AntiKnowledge ("Moving to async doesn't always improve
throughput") never fully decays -- it has a confidence floor of 0.3 because
knowing what is false is permanently valuable.

The six types emerged from two design lineages: the original Grimoire (now
Neuro) design specified five canonical types (Insight, Heuristic, Warning,
CausalLink, StrategyFragment), and the memetic evolution extension added
AntiKnowledge as a sixth type to represent knowledge about what is wrong.

---

## 1. Insight

**Definition.** A validated observation -- a compact causal or correlational
statement distilled from one or more episodes, treated as true until
contradicted by evidence.

**Base half-life:** 30 days. Observations need regular revalidation because
the world changes. An insight about API behavior may become stale when the
API is updated. An insight about code patterns may become irrelevant after a
refactor.

**Coding domain examples:**
- "Rust's borrow checker errors often mean you need `Arc` here"
- "The `roko-gate` crate's test suite is sensitive to timing; use
  `tokio::time::pause()` to avoid flaky tests"
- "When `cargo clippy` warns about needless `clone()`, the fix is usually to
  take a reference instead"

**Chain domain examples:**
- "ETH gas spikes correlate with NFT mints"
- "Uniswap V3 concentrated liquidity positions need rebalancing when price
  moves >5% from the center tick"
- "MEV bots front-run large DEX trades with a median latency of 12ms on
  Ethereum mainnet"

**Research domain examples:**
- "GPT-4-level models consistently underperform on multi-step arithmetic
  despite strong language understanding"
- "Academic papers published on arXiv before peer review have a higher
  retraction rate for biomedical topics than for CS topics"

**Retrieval behavior:** Standard confidence-weighted retrieval. Insights are
the most common knowledge type -- they form the bulk of a mature agent's
NeuroStore.

**Promotion path:** An Insight that proves useful across 5+ episodes and
achieves >= 0.7 confidence can be promoted to a Heuristic by the D2 stage of
the distillation pipeline (see `4-tier-distillation-pipeline.md`).

---

## 2. Heuristic

**Definition.** A reusable rule of thumb -- a compact, actionable pattern
that the agent can apply directly without further reasoning. Heuristics are
more durable than insights because they encode generalized patterns rather
than specific observations.

**Base half-life:** 90 days. Rules of thumb are more durable than
observations because they represent patterns validated across multiple
contexts. A heuristic that works for 90 days is likely capturing a genuine
regularity.

**Coding domain examples:**
- "Always run clippy before committing"
- "When a function exceeds 50 lines, split it into smaller functions"
- "Use `#[must_use]` on functions that return `Result` or `Option`"
- "For async Rust code, prefer `tokio::spawn` over `std::thread::spawn`"

**Chain domain examples:**
- "Set slippage >1% during high volatility"
- "Never swap >5% of pool depth in a single transaction"
- "Rebalance yield farming positions on >5% drift from target allocation"

**Research domain examples:**
- "Cross-reference at least 3 independent sources before accepting a claim"
- "Prefer primary sources (original papers) over secondary sources (blog
  posts, summaries)"

**Retrieval behavior:** Heuristics are retrieved with higher priority than
Insights when the agent is in a high-confidence, execution-focused behavioral
state (Daimon state: Focused or Coasting). They serve as fast System 1
shortcuts that skip detailed reasoning.

**Origin:** Heuristics are produced by the D2 stage of the distillation
pipeline -- they emerge from clusters of 5+ related Insights that share a
common pattern with >= 0.7 confidence.

---

## 3. Warning

**Definition.** A known pitfall -- a specific danger signal that the agent
should watch for and avoid. Warnings are the most aggressive knowledge type:
they have short half-lives because danger signals must be current.

**Base half-life:** 1 hour (v3 ships with the aggressive 1-hour constant;
v1 specified 7 days). Danger signals must be fresh. A security vulnerability
warning from two months ago may have been patched. A gas price warning from
last week may no longer reflect current network conditions. If a warning is
still relevant, it will be reconfirmed by ongoing experience and its
confidence will stay high.

**Coding domain examples:**
- "Never use `unwrap()` in production paths"
- "The `chrono` crate has known issues with time zone handling on Windows; use
  `time` instead"
- "Do not call `std::process::exit()` in library code -- it prevents cleanup"
- "The `reqwest` default timeout is 30 seconds, which is too long for most
  API calls"

**Chain domain examples:**
- "Never swap >5% of pool depth"
- "Avoid interacting with contracts deployed for less than 24 hours"
- "The Curve stETH/ETH pool can depeg during high withdrawal demand"
- "Flash loan attacks frequently target oracle contracts with single-source
  price feeds"

**Operations domain examples:**
- "The staging environment's database has a 100-connection limit; exceeding it
  silently drops queries"
- "Never run migrations on the production database during peak hours"

**Retrieval behavior:** Warnings receive a retrieval boost when the agent's
Daimon state shows high arousal (urgency) or low dominance (uncertainty). The
somatic landscape (see `somatic-integration.md`) gives warnings negative
valence markers, causing them to surface during pre-action safety checks.

**Interaction with AntiKnowledge:** Warnings describe what to avoid ("never
do X"). AntiKnowledge describes what is false ("X seems true but isn't"). The
distinction is between prescriptive (Warning) and descriptive (AntiKnowledge)
negative knowledge.

---

## 4. CausalLink

**Definition.** A cause-effect relationship -- a structured observation that
one phenomenon reliably leads to another. CausalLinks encode directional
relationships using HDC permutation to distinguish cause from effect.

**Base half-life:** 60 days. Causal relationships need periodic confirmation
because underlying mechanisms can change.

**Coding domain examples:**
- "Increasing thread pool size -> reduced I/O latency (up to CPU core count)"
- "Adding `#[inline]` to hot-path functions -> 5-15% throughput improvement
  in tight loops"
- "Removing `Box<dyn Error>` in favor of concrete error types -> reduced
  allocation overhead"
- "Enabling LTO -> 10-30% binary size reduction but 2-5x longer compile time"

**Chain domain examples:**
- "Large buy order -> price impact -> arbitrage opportunity"
- "ETH staking queue length increase -> delayed validator activation ->
  staking APY increase"
- "High gas base fee -> user migration to L2 -> L2 TVL growth"

**HDC encoding:** CausalLinks use the permute operation to encode
directionality:
```
causal_vector = PERM(cause_vector, 1) XOR PERM(effect_vector, 2)
```
This ensures that `CAUSE -> EFFECT` is distinguishable from `EFFECT -> CAUSE`
in the HDC space. The permutation shifts encode the role (cause at position 1,
effect at position 2).

**Retrieval behavior:** CausalLinks are retrieved when the agent encounters a
situation matching either the cause or the effect. If the agent detects a
"large buy order" (matching the cause), the CausalLink surfaces the predicted
effect. If the agent observes "arbitrage activity" (matching the effect), the
CausalLink surfaces possible causes.

**Research basis:** Pearl's Structural Causal Models (Pearl 2000, 2009)
provide the theoretical foundation. The HDC permutation encoding is an
efficient computational approximation of a causal graph edge.

---

## 5. StrategyFragment

**Definition.** A partial strategy for a problem class -- a multi-step action
pattern or recipe that the agent can apply or adapt to similar situations.
StrategyFragments are more complex than Heuristics (which are single rules)
but less complete than Playbooks (which are compiled from multiple heuristics
and strategies).

**Base half-life:** 14 days. Strategies are context-dependent -- they depend
on current tool versions, API behaviors, market conditions, and other
environmental factors that change frequently.

**Coding domain examples:**
- "Rate-limited APIs: exponential backoff + jitter + circuit breaker"
- "Rust async debugging: 1) Check for `Send` bound violations 2) Look for
  held-across-await locks 3) Check for recursive async calls 4) Use
  `tokio::runtime::Builder::enable_all()` in tests"
- "Large refactor procedure: 1) Create comprehensive test coverage
  2) Extract interfaces 3) Implement new code behind feature flag 4) Migrate
  callers 5) Remove old code"

**Chain domain examples:**
- "Yield farming: compound daily, harvest weekly, rebalance on >5% drift"
- "Token launch analysis: 1) Check contract source verification 2) Analyze
  holder distribution 3) Check liquidity lock duration 4) Monitor initial
  trading volume 5) Wait 48h before entering position"

**Retrieval behavior:** StrategyFragments are retrieved when the agent's
current task matches the problem class. They serve as starting templates that
the agent can adapt rather than reasoning from scratch.

**Relationship to Playbooks:** When the distillation pipeline detects
clusters of related StrategyFragments and Heuristics that consistently
succeed together, it compiles them into a PLAYBOOK.md file (see
`4-tier-distillation-pipeline.md`).

---

## 6. AntiKnowledge

**Definition.** Things that seem true but are not -- validated negative
knowledge about common misconceptions, failed approaches, and debunked
beliefs. AntiKnowledge is the epistemic immune system of the agent.

**Base half-life:** 30 days (with confidence floor of 0.3). Known unknowns
are always valuable. An agent that forgets what it has learned is wrong will
re-discover and re-try failed approaches, wasting resources. AntiKnowledge
persists indefinitely through its confidence floor -- it never reaches zero.

**Coding domain examples:**
- "Moving to async doesn't always improve throughput" (false: async helps
  I/O-bound workloads but hurts CPU-bound ones)
- "Using `Rc` instead of `Arc` is always faster" (false: on single-threaded
  code yes, but introducing `Rc` in code that may later become multi-threaded
  creates tech debt)
- "More tests always means better quality" (false: poorly written tests can
  give false confidence)

**Chain domain examples:**
- "Higher APY doesn't mean higher risk-adjusted returns" (false: high APY
  often correlates with higher impermanent loss or smart contract risk)
- "DEX arbitrage is always profitable" (false: gas costs, MEV competition,
  and slippage can make arbitrage opportunities negative-EV)
- "Stablecoin pools are risk-free" (false: depeg events, smart contract risk,
  and regulatory risk exist)

**The Challenge Mechanism:** AntiKnowledge entries carry two special fields
not present on other types:
- `refuted_insight_id`: The ID of the Insight or Heuristic that this
  AntiKnowledge entry refutes
- `refutation_evidence`: Evidence explaining why the refuted entry was wrong

When an AntiKnowledge entry is created, it generates a **refutation warning**
that is attached to the original entry. The `refutation_warning()` method
produces strings like: "Previous insight ke_original was wrong because
benchmark showed 15% throughput regression."

**Retrieval behavior:** AntiKnowledge entries are retrieved in two contexts:
1. **Proactive**: When the agent retrieves knowledge for a task, AntiKnowledge
   entries matching the query are included alongside positive entries.
2. **Reactive**: When a new candidate Insight enters the knowledge base, it is
   checked against existing AntiKnowledge entries. If the candidate matches
   (high HDC similarity to a refuted claim), it is flagged for review.

**Memetic evolution context:** In the Dawkinsian replicator model used for
knowledge base health diagnostics (`W(E) = f * r * L` -- fidelity x fecundity
x longevity), AntiKnowledge entries serve as the **immune system** that
prevents epistemic parasites from proliferating.

---

## Type-to-Code Mapping

```rust
// From crates/roko-neuro/src/lib.rs
pub enum KnowledgeKind {
    Insight,           // compact causal observation
    Heuristic,         // reusable rule of thumb
    Warning,           // known pitfall or danger signal
    CausalLink,        // cause-effect with directional HDC encoding
    StrategyFragment,  // multi-step action pattern
    AntiKnowledge,     // validated negative knowledge
}
```

Legacy variants (`Fact`, `Procedure`, `Playbook`, `Constraint`) survive as
serde aliases so old persisted entries can still be read.

---

## Half-Life Summary

| Type | Base Half-Life | Rationale | Confidence Floor |
|------|---------------|-----------|-----------------|
| **Insight** | 30 days | Observations need regular revalidation | None |
| **Heuristic** | 90 days | Rules of thumb are more durable | None |
| **Warning** | 1 hour | Danger signals must be aggressively current | None |
| **CausalLink** | 60 days | Causal relationships need confirmation | None |
| **StrategyFragment** | 14 days | Strategies are context-dependent | None |
| **AntiKnowledge** | 30 days (with floor) | Known unknowns are permanently valuable | 0.3 |

These base half-lives are multiplied by the tier multiplier to produce the
effective half-life. See `type-half-lives.md` for detailed rationale and
`ebbinghaus-decay-with-tier.md` for the full decay formula.

---

## Domain-Agnostic Design

The six types are domain-agnostic -- they apply equally to any problem
domain. The types describe the *structure* of knowledge, not the *content*.
Domain-specific behavior comes from:

1. **Content**: Free-text `content` field.
2. **Tags**: Domain-specific tags (`["rust", "async"]`, `["defi", "uniswap"]`)
   enable domain-filtered retrieval.
3. **HDC encoding**: Role vectors are domain-configurable. Code symbols use
   `SymbolKind::Function`, `SymbolKind::Struct`, etc.
4. **Somatic landscape axes**: The 8-dimensional strategy space (see
   `somatic-integration.md`) is configured per domain. Coding agents use
   `[complexity, risk, novelty, confidence, time_pressure, scope,
   reversibility, dependency_depth]`.

---

## Academic Foundations

- Pearl, J. (2000). *Causality: Models, Reasoning, and Inference*. Cambridge.
- Dawkins, R. (1976). *The Selfish Gene*. Oxford. (Memetic replicator model)
- Damasio, A. R. (1994). *Descartes' Error*. Putnam. (Somatic markers for
  Warnings)
- Kahneman, D. (2011). *Thinking, Fast and Slow*. FSG. (System 1/System 2 --
  Heuristics as System 1 shortcuts)
- Kleyko, D. et al. (2022). "A Survey on Hyperdimensional Computing." *ACM
  Computing Surveys*, 54(6). (HDC encoding for CausalLinks)

---

## Cross-References

- `four-validation-tiers.md` -- how tiers multiply base half-lives
- `type-half-lives.md` -- detailed half-life rationale
- `hdc-knowledge-encoding.md` -- how entries are encoded as HDC vectors
- `antiknowledge-challenge.md` -- full AntiKnowledge challenge mechanism
- `4-tier-distillation-pipeline.md` -- how types interact with distillation
