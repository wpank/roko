# Research Synthesis 2: Strategic and Technical Intelligence for Roko

Synthesized from 13 research documents (research10-15, ressearch2/research1-8).
Date: 2026-04-29.

---

## 1. ARCHITECTURE AND CONTROL PLANE POSITIONING

### 1.1 The "Agent Coordination Plane" category is open and named

The strongest finding across all research: a16z partner Malika Aubakirova published the
exact category definition Roko should claim. Her Big Ideas 2026 essay (Dec 9, 2025) states:

> "The bottleneck becomes coordination: routing, locking, state management, and policy
> enforcement across massive parallel execution. The winning platforms will be the only
> ones capable of surviving the deluge of tool execution that follows."

This maps to a four-element problem -- routing + locking + state + policy -- that no
incumbent owns end-to-end. It is broader than workflow orchestration (Temporal/Camunda),
narrower than distributed systems coordination (etcd/Consul), adjacent to but distinct
from service mesh (Istio). The category sits at the "Market Guide" stage in Gartner's
taxonomy, meaning there is a 12-18 month window to claim the name before analyst
codification locks in a competitor's framing.

**Roko implementation relevance:**
- `crates/roko-orchestrator/` already implements the DAG executor and parallel execution
- `crates/roko-agent/src/dispatcher/mod.rs` handles routing decisions
- `crates/roko-gate/` implements policy enforcement (11 gates, 7-rung pipeline)
- `crates/roko-runtime/src/pipeline_state.rs` manages state
- The coordination primitives exist but are framed as "orchestration" not "coordination
  plane" -- the architectural narrative should be updated to use Aubakirova's vocabulary

### 1.2 Control plane / data plane separation as the core architectural frame

Martin Casado's entire career is built on separating control planes from data planes (SDN,
Nicira, VMware). His 2007 Stanford dissertation introduced vocabulary that maps directly
onto Roko's architecture:

| Casado concept | Roko analog | Crate |
|---|---|---|
| Logically centralized control | Orchestrator + PlanRunner | `roko-orchestrator`, `roko-cli/src/orchestrate.rs` |
| Control plane / data plane separation | Gate pipeline (control) vs agent execution (data) | `roko-gate` / `roko-agent` |
| Default-off | Safety contracts, pre/post checks | `roko-agent/src/safety/` |
| Flow-level granularity | Per-task gating, per-turn efficiency events | `roko-gate`, `roko-learn` |
| Trusted computing base minimization | AgentContract limiting tool access | `roko-agent/src/safety/` |
| Strong binding between flow and origin | HDC fingerprinting per episode | `roko-primitives` |
| Replication for fault-tolerance | Session persistence + resume | `.roko/state/executor.json` |
| Incremental deployability | Adapter-trait architecture, TOML config | `roko-core/src/config/` |

**Implementation action:** The pitch line for Roko should be: "Ethane reduced enterprise
networks to dumb forwarding elements governed by a logically centralized policy. Roko
reduces LLM agents to dumb invocation elements governed by a logically centralized
routing-and-gating policy." Every architectural doc and public description should adopt
the control-plane/data-plane vocabulary.

### 1.3 Differentiation from Keycard and Temporal

**Keycard** (a16z portfolio, $38M, led by Aubakirova/Lackey) occupies identity and
authorization -- "is this agent allowed to call this tool right now." It issues
identity-bound, task-scoped JWTs via Security Token Service (RFC 8693). Its pricing is
per-transaction ($500/month for 100K transactions). Keycard explicitly does NOT do:
agent-to-agent coordination, cost-aware model routing, shared knowledge/memory, agent
reputation, behavioral verification, multi-agent orchestration, or economic primitives.

**Temporal** ($5B Series D, Feb 2026, Sarah Wang) occupies single-tenant durable execution.
Their April 2026 blog states the boundary plainly: "The agent framework handles the AI.
Temporal handles the infrastructure." Temporal explicitly does not provide: portable
cross-org agent identity, shared persistent memory across agents, cost-aware multi-LLM
routing, agent reputation, or behavioral verification.

**Roko's positioning:** The coordination, prediction, and shared-knowledge layer that runs
ABOVE Keycard's identity primitive and ABOVE Temporal's durable execution. The framing:
"Keycard answers 'is this agent allowed.' Temporal answers 'did this code run.' Roko
answers 'which agent, with which memory, at what price, with a receipt the counterparty
can verify.'"

**Relevant crate surfaces for this differentiation:**
- Cost-aware routing: `crates/roko-agent/src/model_call_service.rs`, CascadeRouter in
  `crates/roko-learn/`
- Shared knowledge: `crates/roko-neuro/src/knowledge_store.rs`
- Behavioral verification: `crates/roko-gate/src/gate_service.rs`
- Agent reputation (HDC fingerprint): `crates/roko-primitives/`

---

## 2. MODEL ROUTING AND COST OPTIMIZATION

### 2.1 Bandit-theoretic routing has rigorous convergence guarantees

The research provides the mathematical backing for CascadeRouter's approach:

- **UCB1** (Auer, Cesa-Bianchi, Fischer 2002): achieves O(sqrt(KT log T)) regret. For K=5
  models and T=1000 queries, expected regret is ~525 normalized units, dropping below 5% of
  optimal within 200-500 rounds.
- **Thompson Sampling** matches asymptotically (Russo & Van Roy 2016).
- **Hedge algorithm** gives O(sqrt(T ln K)) regret for predict-publish-correct loops framed
  as expert aggregation (Cesa-Bianchi & Lugosi 2006).
- **Borkar & Meyn (2000)** "The O.D.E. Method for Convergence of Stochastic Approximation
  and Reinforcement Learning" -- EMA threshold adaptation is literally Robbins-Monro
  stochastic approximation with constant step size.

The predict-correct loop tracks the ODE c_dot = c* - c, which is globally exponentially
stable. This provides rigorous backing for the adaptive gate thresholds in
`crates/roko-learn/`.

**Cautionary reference:** "When Routing Collapses" (arXiv:2602.03478) shows routers can
converge to degenerate single-model policies. This should inform CascadeRouter's
exploration strategy.

**Implementation paths in Roko:**
- CascadeRouter persistence: `.roko/learn/cascade-router.json`
- Adaptive gate thresholds (EMA): `.roko/learn/gate-thresholds.json`
- Prompt experiments (A/B): `.roko/learn/experiments.json`
- The existing bandit implementation should be validated against the UCB1 regret bound
  empirically -- log cumulative regret per 100 rounds and verify the sqrt(T) envelope

### 2.2 The "Bitter Economics" thesis validates cost-aware routing structurally

Casado's published thesis (Feb 2026 Latent Space episode): frontier labs are "gross margin
positive on the last training run but gross margin negative on the next, borrowing against
the future." The implication: model providers cannot be trusted to optimize cost on the
customer's behalf because they are subsidizing themselves. A coordination layer that
arbitrages across providers is structurally sound, not a thin wrapper.

His exact quote: "Everybody has to be on the token path and everybody has to ask... how do I
extract margin on the tokens that are going through?" -- this is exactly what CascadeRouter
does.

**Market validation data:**
- FrugalGPT demonstrated 98% cost reduction with routing
- RouteLLM showed 48% cheaper at 95% GPT-4 quality
- Cursor forum has dozens of posts about "how do I keep agent costs under $X"
- Replit publicly disclosed gross margins swung from 36% to negative 14% from agent
  inference costs

**Implementation in Roko:**
- `crates/roko-agent/src/model_call_service.rs` -- model selection
- `crates/roko-cli/src/model_selection.rs` -- CLI model routing
- `crates/roko-learn/src/` -- learning from routing decisions
- Gap: the CascadeRouter does not yet learn from manual `force_backend` overrides
  (UX34 in CLAUDE.md)

### 2.3 OpenTelemetry gen_ai.* semantic conventions as the telemetry standard

The gen_ai.* semconv is experimental but vendor-supported by six observability platforms
simultaneously (Datadog, Honeycomb, Langfuse, Phoenix, Langtrace, Grafana). The Rust
SDK ships the GEN_AI_* constants from recent semconv builds. No Rust equivalent of
Python's opentelemetry-instrumentation-openai auto-instrumentation exists yet.

**Key conventions (v1.37+):**
- `gen_ai.input.messages`, `gen_ai.output.messages` (structured message form)
- `gen_ai.provider.name` replaces deprecated `gen_ai.system`
- `gen_ai.operation.name`: chat, text_completion, embeddings, execute_tool, retrieval,
  create_agent, invoke_agent
- `gen_ai.conversation.id` -- map to Slack thread_ts for trace correlation
- `gen_ai.usage.cache_read.input_tokens`, `gen_ai.usage.cache_creation.input_tokens`

**Implementation action for Roko:**
- `crates/roko-runtime/src/jsonl_logger.rs` already logs events; adding OTel span emission
  alongside JSONL would give six vendor integrations for ~200 LOC
- Pin emit shape to v1.37+ structured-message form
- Contain attribute mapping to a single module for future-proof refactoring
- The `crates/roko-agent/src/usage.rs` usage tracking should feed gen_ai.usage.* attributes

---

## 3. ECOSYSTEM AND PLUGIN ARCHITECTURE

### 3.1 The Bevy Plugin trait as the adapter model

Bevy's Plugin trait is the closest structural analog to Roko's adapter needs:

```rust
pub trait Plugin: Downcast + Any + Send + Sync {
    fn build(&self, app: &mut App);          // required
    fn ready(&self, _app: &App) -> bool { true } // async-setup gate
    fn finish(&self, _app: &mut App) {}      // post-ready
    fn cleanup(&self, _app: &mut App) {}     // pre-run
    fn name(&self) -> &str { /* type_name */ }
    fn is_unique(&self) -> bool { true }
}
```

The masterstroke is the blanket impl that makes any `fn(&mut App)` automatically a Plugin.
This duality means the simplest plugin is a five-line function.

**Roko adaptation:**
- Ship a `RokoPlugin` trait with the `fn(&mut RokoBuilder)` blanket impl
- Ship a `#[derive(RokoAdapter)]` macro for boilerplate methods
- Per-adapter capability scopes declared in TOML (following Tauri's model)
- Conformance test crate (`roko-conformance`) modeled on Airbyte CAT and Terraform
  plugin-testing

**Relevant existing surfaces:**
- `crates/roko-core/src/foundation.rs` -- the trait system (Signal + 6 verbs)
- `crates/roko-std/src/tool/` -- 19 builtin tools already follow a trait pattern
- The adapter trait should compose with the existing Substrate/Scorer/Gate/Router/Composer/
  Policy verb traits

### 3.2 Terraform's 95% discovery finding inverts the long-tail assumption

Terraform's February 2018 RedMonk audit: 376 modules total, 42 HashiCorp-verified -- and
those 42 verified modules accounted for over 95% of all downloads. This inverts the usual
ecosystem advice. The lesson: a verification badge on a small curated set does nearly all
the discovery work.

**Roko application:**
- Plan v1 around 10-20 "Roko Verified" reference adapters
- A binary `verified: true` field is sufficient below ~1,000 adapters
- First-party-vs-community ratio collapses fast: n8n has 400 official vs 5,834 community
  nodes (1:14 ratio). Once Roko passes ~50 functional adapters, first-party share drops
  below 20%.
- The verification, scaffolding, and bounty machinery must be in place before adapter #25

### 3.3 Recipe composition as the retention flywheel

Terraform modules data: provider count ~3,500, module count exceeds 14,000. The top
modules internally call 30+ resources from multiple providers. The compression ratio (1
import = 5+ adapters wired correctly) is the mechanism.

n8n's top usage templates are dominated by three-system-minimum workflows. Zapier's
multi-step Zaps were the largest single product upgrade in Zapier history. The dominant
template shape is trigger -> enrich -> write+notify.

**Implementation in Roko:**
- The `recipe.toml` concept should use Cargo-style `[adapters]` table with SemVer
  constraints
- Backstage-style `parameters` (JSON Schema) for auto-generated UI forms
- Verb-first naming convention: "Sync new GitHub issues to Linear with Semgrep risk
  tagging" (proven across Zapier and n8n for SEO)
- `crates/roko-cli/src/chain_registry.rs` could serve as the registry foundation

---

## 4. GATE PIPELINE AND VERIFICATION

### 4.1 Continuous compliance attestation as a streaming product

The novel pattern: every agent action is gated, signed, and emitted as a real-time
compliance event. This transforms the gate pipeline from QA infrastructure into a
revenue-generating product.

Precedent economics:
- Vanta: $220M ARR at $4.15B valuation (July 2025), built entirely on SOC 2 enforcement
  timing
- OneTrust: $5.3B last private round (2021), built on GDPR enforcement timing (2018)
- The repeating pattern: regulatory deadline + 18-24 month build window = $3-6B outcome

EU AI Act Article 50 enforcement arrives August 2, 2026 -- 14 weeks from report date.
Three-quarters of EU enterprises (per Deloitte Q1 2026, n=3,235) have no AI provenance
infrastructure today.

**Roko implementation surfaces:**
- `crates/roko-gate/src/gate_service.rs` -- the 11 gates, 7-rung pipeline
- `.roko/episodes.jsonl` -- episode logging with HDC fingerprints
- `crates/roko-runtime/src/jsonl_logger.rs` -- event logging
- Gap: gate results are logged but not emitted as structured compliance events suitable
  for enterprise SIEM/GRC integration. Adding an OTel span per gate result closes this.

### 4.2 Sigstore/SLSA as the near-term audit trail anchor

Sigstore has dramatically more empirical support than ERC-8004 for the audit-trail
narrative today:
- 101M+ Rekor transparency log entries
- 33,000+ unique OSS projects signing
- 21M+ Fulcio short-lived certificates
- 16,000+ npm packages with provenance

The September-November 2025 npm "Shai-Hulud" supply-chain worm and GitHub's resulting
2FA mandate for local publishing turned willingness-to-pay for verification from
theoretical to post-incident. The in-toto attestation format and SLSA framework are
the de-facto winners.

**Roko application:** Frame the audit trail as "applying Sigstore/in-toto primitives at
the agent boundary instead of the build boundary." Position ERC-8004 compliance as
future-proofing detail rather than lead narrative. The investor framing: "33,000 OSS
projects already pay for this primitive at build time; nobody yet ships it at
agent-action time."

### 4.3 Bandit-driven prompt experiment promotion

The novel pattern: production routing IS the experiment, because the bandit converges to
the winner online. No separate eval framework, no offline scoring. This requires the
learning loop to work -- a fresh-start agent cannot do this.

This differs from Braintrust, PromptLayer, and LangSmith which all separate eval from
production. Roko's structure is unified.

**Already implemented in Roko:**
- `crates/roko-learn/src/` -- experiment store, bandit routing
- `.roko/learn/experiments.json` -- experiment persistence
- `crates/roko-cli/src/orchestrate.rs` -- ExperimentStore integration

**Gap:** The experiment results do not currently feed back into the CascadeRouter's model
selection. The learning loop from experiment outcome -> routing weight update should be
made explicit.

---

## 5. INTEGRATION PRIORITIES AND COMPETITIVE INTELLIGENCE

### 5.1 Linear AgentSession as the gateway integration

Cursor's Linear integration is publicly broken (forum threads /134796, /135033, /158505
document duplicate session creation). Sweep AI never shipped Linear webhook support.
No Rust-native runtime has shipped a compliant Linear adapter.

Linear's AgentSession protocol imposes two simultaneous deadlines:
- HTTP-200 acknowledgment within 5 seconds of webhook delivery
- First agentActivityCreate mutation within 10 seconds of a `created` event

The moat is emit-then-async orchestration: the webhook handler must spawn a tokio::task
that emits a `thought` activity within 10s, then drive the LLM round-trip. This is
precisely what Roko's async runtime handles well.

**Implementation cost:** 10-14 working days for v1 minimal adapter using `graphql_client`
0.14 with custom-scalars module. There is no usable Rust Linear SDK on crates.io (only
`linear_sdk` v0.0.1 from 2022, abandoned).

**Relevant Roko surfaces:**
- `crates/roko-agent/src/` -- agent dispatch would handle the async orchestration
- `crates/roko-serve/src/routes/` -- webhook receiver patterns already exist
- The adapter should use `hmac` + `sha2` for HMAC-SHA256 webhook verification (~120 LOC)

### 5.2 OpenAI Codex CLI as existential repositioning trigger

Codex CLI is Apache-2.0, ~95% Rust, 72K+ GitHub stars, with native MCP client+server
support. This collapses Roko's generic "Apache-2.0 Rust agent runtime" positioning.

The differentiation that survives has four pillars:
1. **Adapter-trait architecture** -- Roko's 18-crate Bevy-style design lets users swap
   inference, queue, observability, and integration layers; Codex is coupled to OpenAI
2. **Model-agnostic from day one** -- Roko speaks OpenAI-compat, ollama-rs, Anthropic,
   Vertex independently
3. **EU sovereignty and self-hostability** -- no US-cloud control plane required
4. **Integration depth** -- Linear AgentSession, Slack-thread-to-trace, Sentry adapter

**Relevant Roko surfaces:**
- `crates/roko-agent/src/openai_compat_backend.rs` -- OpenAI-compatible backend
- `crates/roko-agent/src/provider/` -- multi-provider support
- `crates/roko-agent/src/claude_cli_agent.rs` -- Claude CLI backend
- The model-agnostic story is already built; the messaging needs to lead with it

### 5.3 Competitive landscape: the crowded agent infrastructure space

**Well-funded competitors in adjacent lanes (as of April 2026):**

| Player | Raised | Lane | Threat to Roko |
|---|---|---|---|
| Keycard | $38M | Agent identity/auth | Complementary (layer below) |
| Capsule Security | $7M | Runtime behavioral monitoring | Adjacent (during-action) |
| Nava Labs | $8.3M | On-chain DeFi verification | Adjacent (different substrate) |
| Sycamore | $65M | "Agent Operating System" | Direct (similar framing) |
| /dev/agents | $56M | Agent OS, ex-Google/Meta | Direct (similar framing) |
| t54 Labs | $5M | x402-secure agent payments | Adjacent (fintech-specific) |

**Agent coding tools competitive data:**

| Company | ARR (est.) | Valuation | Key fact |
|---|---|---|---|
| Cursor | $1B+ (mid-2025) | $9.9B+ | Fastest to $100M ARR ever |
| Cognition/Devin | ~$73M (Mar 2025) | $10.2B | 140x ARR multiple; Answer.AI showed 15% task completion |
| Augment | $20-40M | Series B ($227M raised) | Enterprise-only, strongest of second tier |
| Factory.ai | <$10M | Series A (~$19M) | Positioning shifted 3x in 18 months |
| Cosine | <$5M | ~$25M raised | SWE-bench results contested |
| Poolside | Pre-revenue | $3B ($500M+ raised) | Pivoted from foundation model |
| Magic.dev | Not GA | $1.58B ($465M raised) | Product not GA; credibility issues |

**Aggregate bear case:** The Factory/Cosine/Augment/Poolside/Magic group has raised
~$1.5B+ collectively but has likely <$100M aggregate ARR. The "funded ahead of revenue"
pattern is the key objection Roko must pre-empt.

### 5.4 The five strongest integration chains

Ranked by compounding effect:

1. **Linear webhook -> plan -> GitHub PR -> CI -> Linear status** (Chain B) -- Roko's lead
   demo. Devin ships it closed-source; Cursor ships it broken; Sweep never shipped it.
   Cognition publicly reports 659 Devin PRs merged in their best week.

2. **Sentry error -> plan -> GitHub PR -> OTel trace -> Linear closed** (Chain A) --
   Closest analog is Sentry Seer Autofix. Seer does not close Linear and does not emit
   gen_ai.* spans. Roko's delta: the agent's plan/tool-use becomes a span attached to the
   originating trace ID.

3. **GitHub label -> plan -> PR -> Slack approval -> merge** (Chain C) -- Sweep's label
   trigger (7.4k stars) plus Cursor's Slack approval gate, combined into one recipe.

4. **Slack thread -> agent -> tool use -> Slack reply with traces** (Chain D) -- The killer
   demo. No closed-source competitor ships inline observability trace URLs back into Slack
   threads. "Cursor and Devin show you what happened. Roko shows you why, with receipts."

5. **Recipe-as-template composition** (Chain E) -- Validated by terraform-aws-modules/eks
   (139.9M total downloads). One canonical recipe with sub-recipes for variants.

---

## 6. FIRST-DOLLAR MARKETS BEYOND DEVOPS

### 6.1 Bioinformatics pipelines -- strongest non-obvious market

Market size: $19.6B in 2024, projected $60.4B by 2034 (16.4% CAGR). Nextflow has 130k+
users; Seqera Labs raised $30M Series B (2024). nf-core has 100+ curated pipelines.

**Why it maps to Roko:** A Nextflow pipeline IS a plan. Each `process` block IS a task.
Each output validation IS a gate rung. The shape is a literal one-to-one mapping to
Roko's plan -> execute -> gate -> learn loop. The 7-rung gate pipeline maps to:
schema-validate -> smoke-run -> check expected output formats -> biological-sanity-check
-> cost-budget-check.

Named buyers: Broad Institute, Sanger Institute, EMBL-EBI, Genomics England, Recursion,
Genesis Therapeutics, insitro, 23andMe.

No agent-native player exists. Empty quadrant.

### 6.2 Smart-contract auditors -- highest margin niche

Market: $2.5B in 2024. Top firms: Trail of Bits ($60M+ ARR), OpenZeppelin ($30M+),
CertiK, Halborn. Per-audit prices: $50K-$1M+.

**Why it maps to Roko:** The gate pipeline IS the audit pipeline. The 7-rung structure
with strictly proper scoring and adversarial test injection is a literal audit playbook.
The bandit-routing layer can route between Slither (fast, noisy), Mythril (slow, deeper),
and Halmos (slowest, most rigorous) based on contract complexity -- exactly what
CascadeRouter is designed for.

Revenue potential: 5% capture per audit = $15K. 200 audits/year x $15K = $3M ARR per
top-tier firm.

### 6.3 Healthcare interoperability (HL7 FHIR) -- phase 2

Market: $3.4B in FHIR-related healthcare IT spend (2024). 21st Century Cures Act and
TEFCA mandate FHIR APIs for all US-certified EHRs. Named buyers: Epic, Oracle Health,
Mayo Clinic, Cleveland Clinic.

Higher friction (HIPAA, BAAs, multi-quarter sales) but 10-20x larger deal sizes.
Roko's open-source, on-premise deployment is a feature for hospital VPCs where PHI
never leaves the firewall.

### 6.4 Emerging buyer roles

- **Platform Engineer**: #2 fastest-growing engineering role per LinkedIn. $165K+ median
  TC. Owns developer experience; natural Roko buyer.
- **AI Engineer**: #1 fastest-growing role. $180-220K TC. 350K+ openings worldwide Q1 2026.
- **Forward-Deployed Engineer (FDE)**: Anthropic listed 40+ FDE openings Q1 2026; OpenAI
  30+. $300-450K TC. $50-200K tooling discretion. FDEs at frontier labs WILL pull Roko
  into customer engagements if the integration story is good.

---

## 7. OSS ECOSYSTEM GROWTH MECHANICS

### 7.1 Contributor retention is driven by response time, not labels

Time-to-first-response is the single most-cited retention lever, dwarfing label hygiene,
mentor programs, and contribution guides (Calefato/Gerosa/Iaffaldano/Lanubile/Steinmacher
2022). All core developers eventually take breaks; ~45% completely disengage for 1+ year;
return probability drops from 35-55% to 21-26% past one year.

Counterintuitive "good first issue" finding: developers whose initial contribution is on a
Good First Bug are LESS likely to become long-term contributors (MSR 2015, Mozilla data).
Expert involvement negatively correlates with retention because experts often just complete
the work for the newcomer.

**Roko action:** Commit in CONTRIBUTING.md to <72 hour first response on PRs. Pair every
GFI with explicit mentor assignment where the mentor coaches, not finishes.

### 7.2 Distribution mechanisms that compound

**Supabase growth curve (best-documented OSS-first playbook):**
- Founded 2020, 100K GitHub stars by 2026
- Growing from 1M to 4.5M developers in <1 year
- $5B Series E (Oct 2025)
- Mechanism: "Launch Week" every 3-4 months -- ship a new feature every day for a week

**Content hierarchy that converts:** technical deep-dives > X-vs-Y comparisons (most SEO
leverage) > migration guides > benchmarks > "Modern $field stack" posts. The single
highest-SEO-ROI artifact for Roko would be a "Roko vs LangChain" comparison page.

**Education-as-distribution:** Rustlings (94 exercises, ~12 hours) is the canonical
template. A `rokolings` equivalent with 10 high-quality exercises (build a tool adapter,
swap an LLM, add memory, add a custom trait impl) would meaningfully accelerate community
formation.

### 7.3 Days 2-7 retention playbook

Supabase's activation keystone is "create a database," not signup. The translation for
Roko: the activation event is "first successful agent invocation that hit a real model API
and returned a trace" -- not `cargo install`, not signup, not first adapter scaffolded.

**Event-driven, not calendar-driven retention:**
- Day 1: First adapter scaffolded, agent runs once, produces a trace URL
- Day 2: Real repo run produces a shareable trace URL. Pre-filled tweet template
- Day 3: Event-triggered email on >1 agent run. Personal founder email if inactive (2-3x
  retention lift per OpenView 2022 benchmarks)
- Day 4-5: Second adapter wired = activation
- Day 6-7: Roko Week ticket with user's name and agent count

k-factor target: 0.2 in Year 1 (SaaS typical; k > 1.0 incredibly rare and almost never
sustained per Saxifrage benchmarks).

---

## 8. BUSINESS MODEL AND MONETIZATION

### 8.1 Temporal pattern: monetize at 18 months, not 4 years

Temporal (the closest comparable) raised $18.75M Series A in October 2020 with zero
commercial product. Revenue began as enterprise support contracts on the open-source
server, sold to early production users (Snap, Box, Coinbase, Checkr). Cloud didn't GA
until October 2022. First paying customers at 18-24 months, not HashiCorp's 4-year wait.

Supabase had paying Pro tiers by August 2020 (~7 months from founding). dbt Cloud
launched September 2018 (~2 years from dbt-core start). Stars don't gate monetization;
production usage does.

### 8.2 Three-tier commercial offering

**Tier 1 -- Production Support -- $24,000/year ($2,000/month):**
Private Slack channel, 24-hour SLA on critical bugs, two architecture review calls per
quarter, priority on adapter authoring requests. Engineering effort: zero. Could be sold
today.

**Tier 2 -- Custom Adapter Authoring -- $10,000-$25,000 fixed-fee:**
Will commissions a specific adapter. IP returns to OSS. 4-8 week delivery. The
Temporal/Supabase pattern of design partners paying you to build core OSS.

**Tier 3 -- Roko Cloud Early Access -- $499-$1,499/month flat:**
Hand-deployed managed Roko, modeled on Temporal's 2021 "Cells" -- explicitly framed as
"manual ops, white-glove," not self-serve.

**90-day target:** 2 signed Tier 1 contracts ($48K ARR run-rate) + 1 Tier 2 adapter
contract ($15K bookings) = $63K bookings, $48K+ ARR run-rate, zero CAC.

**Contract template:** Common Paper Design Partner Agreement v1.3 (CC BY 4.0, used by
Temporal/Snyk). Key clause: Provider-owns-Feedback IP (Section 1.3 + 6) is the
load-bearing protection -- do not concede. No exclusivity. 6-month term. The fee should
NOT be zero -- a free pilot is psychologically a beta; a $12K contract is a commitment.

### 8.3 Open-source licensing strategy

Apache 2.0 core, cloud-hosted commercial. The line: gate operational convenience (hosted,
support, SLA), never gate functionality.

Cautionary tales:
- HashiCorp's BUSL relicense (Aug 2023) provoked the OpenTofu fork within weeks
- PlanetScale killed free tier with tone-deaf messaging; community revolted
- Railway killed free tier with trial credits and $5/month entry -- community responded
  warmly

The rule: never relicense the OSS core. Declare the structure upfront.

Ferrous Systems/Ferrocene provides the Rust-native commercial precedent: Euro 25/seat/month
for binary distributions + LTS + qualified compiler. OSS remains Apache-2.0 + MIT. They
sell certainty, not features.

### 8.4 Platform multiples require three signals

Equal Ventures' BVP Cloud Index analysis: platforms trade at 8.2x EV/revenue vs 3.9x for
traditional SaaS (2.1x premium). Median market cap $26.4B vs $4.1B (6.4x size premium).

The drivers: NRR >= 130%, gross margin > 75%, growth > 30%, and Rule of X score > 60.
Without those, you are a tool no matter what you call yourself.

Reference points:
- Temporal $5B on ~$80-120M ARR = 40-60x multiple
- Cursor $29.3B on ~$1B ARR = 29x
- LangChain $1.25B on ~$12-16M ARR = 80-100x (pure narrative)
- GitLab 2.4x despite $1B ARR and 25% growth (failed to escape "single sign-on" framing)

---

## 9. OBSERVABILITY AND VISUALIZATION

### 9.1 Execution trace UI patterns

The strongest converging signal: Chrome DevTools and Datadog independently land on the same
pattern -- multi-track, time-aligned, canvas-rendered, hover-synced.

Key elements to adopt:
- Multiple synchronized tracks sharing one x-axis (agent execution flame, $/sec ribbon,
  token throughput, latency p99)
- Per-agent categorical color palette (hash-to-HSL, from Jaeger UI)
- Mini-map context strip at top (Jaeger pattern)
- Shade-by-cost ramp: hue=agent, lightness=cost; expensive spans literally glow (Datadog
  Trace View 2023 redesign)
- Animated dashed "marching ants" for in-flight events (Temporal Workflow Timeline)
- Inline cost labels on every bar in monospace right-edge (LangSmith minimum viable copy)

**Roko application:** The `roko dashboard` TUI (F1-F7 tabs) could adopt the multi-track
synchronized timeline for the execution view. The SSE/WebSocket routes in `roko-serve`
could stream trace data in a format consumable by a web-based trace viewer.

### 9.2 Knowledge graph terrain visualization

Recommended: d3-contour terrain map where compounding maps to elevation (peaks rise as
confidence accumulates) and demurrage maps to erosion (peaks shrink, valleys widen).
~200 LOC delta from current d3-force code. 60fps on M1.

This directly visualizes the knowledge store in `crates/roko-neuro/src/knowledge_store.rs`
-- facts with higher confidence appear as higher terrain; stale facts erode.

### 9.3 Cost comparison visualization

For the 30x cost reduction proof:
- "Crushed Bar" pattern: two horizontal bars, baseline at 100% width (red) vs Roko at
  3.3% width (green), with dotted vertical at 3.3% labeled "30x less"
- Pure HTML/CSS, two divs, no library. On-viewport-enter animation over 800ms
- Tufte-correct: proportional ink, no axis break, no log trickery

---

## 10. EU/BERLIN STRATEGIC ADVANTAGES

### 10.1 Non-dilutive capital: Euro 80-150K available

**NLnet NGI Zero Commons Fund:** Euro 5,000-50,000 grants. Next deadline June 1, 2026.
Rust projects routinely funded. No incorporation required. Apply under "Roko: open-source
Rust agent runtime for digital sovereignty."

**Sovereign Tech Fund (now Agency):** Euro 23M+ across 60 OSS projects. Past Rust grants
include uutils coreutils (Euro 99,060). Requires cost of work above Euro 50,000 and open
digital base technologies.

**Sovereign Tech Fellowship:** Euro 64K-82K/year employed or freelance hourly. 2026 cycle
closed April 6; next window TBD (likely Q1 2027).

**Rust Foundation Community Grants:** $100K allocation restarted in 2026. Modest stipends
($1,500/month + $4,000 travel/equipment historically).

### 10.2 Berlin Rust community map

**Key contacts:**
- Florian Gilcher (skade): Ferrous Systems Managing Director, Rust Foundation Project
  Director. Anchors at Wallstr. 59, 10179 Berlin Mitte (same office as Slint and KDAB).
  Monthly Rust Berlin On Location meetup there (~25 capacity).
- Jan-Erik Rediger (badboy/jer): Mozilla Glean lead, Berlin-confirmed, long-term meetup
  organizer
- Olivier Goffart and Simon Hausmann: Slint co-founders, Berlin

Coordination channels: rust-berlin.zulipchat.com (Zulip), Matrix
!xycQxSjSAvEezkyztA:chat.berline.rs, X @RustBerlin.

**Conferences ranked for Roko:**
1. EuroRust (Oct 14-17, Barcelona) -- highest priority, infra/servers/WASM/CLI scope
2. RustWeek (May 19-20, Utrecht) -- highest networking density (900+ attendees)
3. RustLab (Nov 1-3, Bologna) -- medium
4. Skip Oxidize (Berlin, but embedded/industrial -- wrong audience)

### 10.3 Berlin VC landscape (corrected)

- La Famiglia merged into General Catalyst (2024) -- Jeannette zu Furstenberg now MD at GC
- Cavalry Ventures rebranded to NAP (Feb 2025)
- **468 Capital** (Berlin + SF + Madrid, $1.3B+ raised) -- only Berlin-anchored fund with
  explicit "AI & Automation, Infrastructure & Enterprise Software" thesis
- **Air Street Capital** ($232M Fund III, March 2026) -- largest solo-GP fund in Europe,
  Berlin-friendly through Black Forest Labs and other portfolio companies
- Cherry Ventures Fund V ($500M, Feb 2025) -- backed Dash0 (observability)

### 10.4 APAC regulatory tailwinds

- India DPDP Act 2023: Rupee 2,000 crore (~$240M) tendered for AI governance tools in 2025
- Singapore PDPA + Model AI Governance Framework 2.0: $80M+ in AI governance contracts
- South Korea AI Basic Act (effective Jan 2026): first formal AI Act outside EU
- Indonesia PDP Law: creating Southeast Asia's largest single-country buyer block

Combined addressable market: $500M-1B today, growing 40%+ YoY. Roko's open-source runtime
+ sovereign deployment is the exact architecture APAC governments want.

---

## 11. PROTOCOL AND STANDARDS POSITIONING

### 11.1 MCP governance as an under-represented seat

MCP was donated to Linux Foundation on December 9, 2025. AAIF surpassed CNCF in membership
in three months (~170+ orgs). The SEP process is public, free, no membership gate. There is
no European Platinum member presence visible in MCP governance -- this is an under-
represented seat for a Berlin-based maintainer to occupy through SEP authorship.

**Roko relevance:** The MCP config passthrough is already wired (`agent.mcp_config` in
roko.toml -> `--mcp-config`). Authoring a SEP on agent-coordination-related MCP extensions
would position Roko as the reference implementation.

### 11.2 A2A v1.0 went stable April 9, 2026

150+ adopting organizations including Microsoft Azure AI Foundry, AWS Bedrock AgentCore,
and Google. Signed Agent Cards are now a primitive every coordination pitch should assume.
AP2 (Agent Payments Protocol) is live with 60+ orgs.

### 11.3 Langfuse partnership (corrected: acquired by ClickHouse Jan 16, 2026)

Langfuse was acquired by ClickHouse, NOT raised a Series A from Lightspeed as previously
assumed. Licensing remained MIT, 50K-observations/month free tier intact. The partnership
process is informal: GitHub Discussion + PR to langfuse-docs + co-marketing via Marc
Klingen (Berlin-based, @marcklingen).

**Critical correction:** Arize Phoenix is Elastic License 2.0, NOT Apache-2.0. License
symmetry argument collapses. The defensible pattern for Roko is to use
`opentelemetry-otlp` directly with the endpoint as a config knob, so the same code
repoints at Langfuse, Phoenix, Honeycomb, Grafana, or Laminar by changing env vars.

---

## 12. DEMO AND PRESENTATION INTELLIGENCE

### 12.1 The "recognition event" demo pattern

The strongest single demo move: open with Aubakirova's Big Ideas 2026 quote on screen
while Roko fans out 5,000 sub-tasks live. This converts a pitch into a recognition event
-- she wrote those exact words five months before the meeting.

### 12.2 Demo architecture

Eliminate every dependency you don't control. A CLI binary running against a local
controller, with all LLM responses served from a pre-warmed cache for demo prompts. If the
cache hits, no network. If it misses, fall through to real APIs. Backup hierarchy:
1. Local laptop demo with own LTE hotspot
2. Pre-recorded Loom as QuickTime file (no internet needed)
3. Annotated screenshots in deck
4. Whiteboarding architecture by hand

Pre-script a worker-kill recovery moment. This pre-empts the most likely a16z question of
2026 and signals fluency in the firm's durable-execution thesis. Sarah Wang's framing:
"reliability is not an optimization. It is a gating factor."

### 12.3 The "Collison Installation" pattern

Patrick Collison's "hand me your laptop" framing turned customer demos into both
integration and roadmap research. For Roko: `cargo install roko && roko run agent.toml`.
Executable in 60 seconds, with Casado's laptop as the witness node. Infinitely more
credible than a polished UI walkthrough.

**Roko's existing CLI surfaces support this:** The `roko run "<prompt>"` command is the
single-prompt universal loop (compose -> agent -> gate -> persist). This is the
demo-ready command.

### 12.4 Three-number traction format

Casado explicitly disqualified standard OSS dashboard metrics in his "Investing in Orbit"
post: "GitHub stars vary widely by sector... Downloads are similarly meaningless due to
automated downloads." He names these as vanity metrics.

The recommended traction triad:
1. Design partners in production with named logos (3-5 actively deploying)
2. A usage-depth metric tied to the agent thesis (agent runs, tool calls orchestrated)
3. One velocity number with a sharp comparison anchor

"177K lines of Rust" should come OFF the traction slide entirely -- every VC source treats
lines-of-code as a Goodhart's Law metric. If technical depth must be conveyed, reframe as
capability: "Rust runtime delivers Xms p99 latency at Y concurrency."

---

## 13. IMPLEMENTATION PRIORITIES FOR ROKO CODEBASE

Based on the full research synthesis, the following implementation actions are ranked by
compounding leverage for the Roko codebase specifically:

### Priority 1: Native gen_ai.* OTel emission (HIGH -- ~200 LOC, 1 week)

Add OTel span emission from Roko's core agent dispatch path. This single integration
delights six observability vendors simultaneously. The module should live adjacent to
`crates/roko-runtime/src/jsonl_logger.rs` and emit alongside existing JSONL logging.

### Priority 2: Vendor-neutral observability config knob (HIGH -- ~80 LOC, 2 days)

In roko.toml or recipe.toml, add:
```toml
[observability]
provider = "langfuse"  # or "phoenix" | "honeycomb" | "grafana" | "otlp-generic"
endpoint = "..."
protocol = "http/protobuf"
```
Use `opentelemetry-otlp` directly rather than vendor-specific crates.

### Priority 3: Gate results as structured compliance events (MEDIUM -- ~150 LOC, 3 days)

Each gate result in `crates/roko-gate/src/gate_service.rs` should emit an OTel span with
structured attributes suitable for SIEM/GRC integration. This transforms the gate pipeline
from internal QA to externally consumable compliance stream.

### Priority 4: CascadeRouter experiment feedback loop (MEDIUM -- ~200 LOC, 1 week)

Wire experiment outcomes from `.roko/learn/experiments.json` back into CascadeRouter
routing weight updates. The bandit should promote winning prompt/model combinations
automatically. Currently the experiment store and router operate independently.

### Priority 5: Force_backend override learning (LOW -- ~100 LOC, 2 days)

UX34 gap: the CascadeRouter does not learn from manual `force_backend` overrides. These
overrides represent explicit human routing preferences and should feed the learning loop.
Location: `crates/roko-learn/`.

### Priority 6: Knowledge store consultation for model selection (LOW -- ~150 LOC, 1 week)

Item 13 from CLAUDE.md: the neuro knowledge store is not yet consulted for model selection
in CascadeRouter. Historical knowledge about model performance on similar tasks should
inform routing decisions. Location: wire `crates/roko-neuro/src/knowledge_store.rs` into
`crates/roko-learn/`.

---

## 14. MARKET SIZING AND VALUATION BENCHMARKS

### 14.1 Agentic AI capital deployment (2026 YTD)

$2.66B across 44 rounds YTD 2026 vs $1.09B/71 rounds in same period 2025 -- a 143% dollar
increase in fewer, larger rounds. Capital is concentrating in plumbing (Nava, Capsule,
Keycard) and regulated verticals (Rilian/defense $17.5M, AppZen/finance $180M).
Pure-horizontal coordination plays are the structurally interesting unfilled seat.

### 14.2 Agent infrastructure valuation comps (April 2026)

| Company | Valuation | ARR (est) | Multiple | Category |
|---|---|---|---|---|
| Temporal | $5B | $80-120M | 40-60x | Durable execution |
| Cursor | $29.3B+ | $1B+ | ~29x | AI IDE |
| LangChain | $1.25B | $12-16M | 80-100x | Agent framework |
| Braintrust | $300M | <$10M | 30-50x | Agent eval |
| Vanta | $4.15B | $220M | ~18x | Compliance |
| CrowdStrike | -- | -- | 19.8x EV/Rev | Cybersecurity |

### 14.3 The Jevons paradox for agent infrastructure

GPT-4 -> 4o made inference 10x cheaper and usage went up 100x+. Cheaper, smarter models
mean more agents, more concurrent runs, more failure modes -- every one of which the
coordination plane mediates. The correct investor framing: "We want models to get better.
Every capability jump moves the bottleneck to orchestration, taste, governance -- the
layer we own."

---

## 15. KEY CORRECTIONS TO EXISTING ASSUMPTIONS

| Item | Previous assumption | Corrected fact |
|---|---|---|
| Cursor ARR | "$2B" | $1B+ by mid-2025 (Reuters); $2B not yet sourceable |
| Braintrust valuation | $800M | $300M Series B (TechCrunch May 2025) |
| Temporal Series D lead | Casado | Sarah Wang and Raghu Raghuram |
| Langfuse funding | Series A from Lightspeed | Acquired by ClickHouse (Jan 16, 2026) |
| Arize Phoenix license | Apache-2.0 | Elastic License 2.0 |
| Cluely scandal | Casado deal | Bryan Kim deal |
| "Stripe data-room mandatory" | a16z policy | Editorial speculation, not confirmed |
| Casado's SDN coining | Casado coined SDN | Kate Greene (MIT Tech Review, 2009) |
| Casado's thesis type | Control theory (Lyapunov) | Systems architecture; no convergence proofs |
| Devin task completion | High (implied by funding) | 15% (Answer.AI Jan 2025, unrefuted) |
| Linear market | Green field | 11+ shipped third-party agents already |
| Codex CLI language | Unspecified | Apache-2.0, ~95% Rust, 72K+ stars |
| HashiCorp acquisition | N/A | IBM for $6.4B cash (Feb 2025) |
| Lovable ARR | N/A | $100M+ within 6 months of launch |

---

## 16. SYNTHESIS: WHAT THIS MEANS FOR ROKO'S NEXT 90 DAYS

The research collapses to five bets:

**Bet 1: Claim the Agent Coordination Plane category.** The vocabulary is set (Aubakirova),
the category is named, the window is 12-18 months before Gartner codifies something else.
Update all Roko docs, CLI help text, and README to use "coordination plane" framing.

**Bet 2: Ship gen_ai.* OTel emission and vendor-neutral observability.** One integration,
six vendor partners. The implementation cost is ~200 LOC. This is the highest-leverage
integration per line of code in the entire research corpus.

**Bet 3: Wire the learning loop end-to-end.** The experiment store, CascadeRouter, gate
thresholds, and knowledge store are all built but operate independently. Connecting them
creates the "predict-publish-correct" closed loop that answers Casado's control-loop
objection with running code.

**Bet 4: Lead with compliance-as-streaming, not compliance-as-checkpoint.** The gate
pipeline is the product, not just QA. Emitting structured compliance events via OTel
transforms the existing gate infrastructure into a revenue-generating surface. Article 50
enforcement is 14 weeks away.

**Bet 5: Optimize for production-deployed paid design partners, not stars.** Common Paper
Design Partner Agreement v1.3. $24K/year. Two contracts in 90 days = $48K ARR run-rate.
This is the metric VCs evaluating Series A on AI infra in 2026 actually trust.
