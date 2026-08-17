# TraceCommons Research & Implementation Documents

> **Date**: 2026-08-10
> **Scope**: Self-contained documents for implementing features in TraceCommons from scratch, competitive analysis, grant proposals, and novel research directions.
> **No external dependencies** — all ideas are designed to be built directly into TraceCommons.
> **Repo**: [TraceCommons/trace-commons-server](https://github.com/TraceCommons/trace-commons-server)

---

## Documents

| # | Document | Lines | Focus |
|---|----------|-------|-------|
| 1 | [Implementation Roadmap](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a) | 2,865 | 7 features to build with full Rust code + PostgreSQL migrations |
| 2 | [Grant Proposals](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8) | 1,524 | 3 near-submission-ready proposals totaling $468K |
| 3 | [Privacy & Security](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5) | 2,224 | 10 privacy/security mechanisms with Rust implementations |
| 4 | [Novel Research Ideas](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d) | 2,573 | 8 cross-domain ideas, each a mini-paper proposal |
| 5 | [Competitive Landscape](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07) | 1,034 | 16 competitors, moat analysis, market sizing |
| 6 | [UX & Dashboard Design](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b) | 1,481 | TUI, web dashboard, onboarding, API docs with ASCII mockups |
| 7 | [IronClaw Integration](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a) | 1,605 | Trace capture, TEE attestation bridge, WASM types, NEAR integration |

**Total**: ~13,300 lines across 7 documents.

---

## 1. Implementation Roadmap

[Full document](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a)

Seven features to build in TraceCommons from scratch, each with Rust code, PostgreSQL migrations, and integration points into the existing codebase.

| # | Feature | Priority | Section |
|---|---------|----------|---------|
| 1 | [Adaptive Scoring](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#1-adaptive-scoring) | P0 | EMA thresholds, CUSUM change detection, BOCD probabilistic shift detection. Extends `EnclaveGateOrchestratorConfig`. |
| 2 | [Multi-Stage Gate Pipeline](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#2-multi-stage-gate-pipeline) | P0 | 7-rung pipeline with `GateRung` trait and short-circuit logic. Bloom filter, dedup, perplexity, novelty, cluster, TEE, consensus. |
| 3 | [HDC Fingerprinting](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#3-hdc-fingerprinting) | P1 | MAP-B binary vectors at 10,240 bits. Role-filler binding, `BundleAccumulator` for O(1) novelty queries. |
| 4 | [Dream Consolidation](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#4-dream-consolidation--offline-learning) | P1 | HDBSCAN clustering, pattern extraction, novelty recalibration. Background worker integration. |
| 5 | [Self-Learning Mechanisms](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#5-self-learning-mechanisms) | P1 | LinUCB bandit for scorer selection, A/B testing framework, efficiency tracking. |
| 6 | [Stigmergic Coordination](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#6-stigmergic-coordination) | P2 | Digital pheromone trails, capability maps, anti-evaporation for safety-critical patterns. |
| 7 | [Biological / Affective Mechanisms](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#7-biological--affective-mechanisms) | P2 | Affect-modulated scoring, somatic markers, circadian scheduling, immune system anomaly detection. |

---

## 2. Grant Proposals

[Full document](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8)

Three near-submission-ready proposals for working on TraceCommons.

| Program | Amount | Deadline | Section |
|---------|--------|----------|---------|
| [NLnet NGI Zero Restack](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-1-nlnet-foundation----ngi-zero-restack) | EUR 48,000 | Nov 3, 2026 | Privacy-preserving collective AI trace scoring. 4 milestones: adaptive scoring, multi-stage gates, privacy (DP + ZK), consolidation engine. |
| [NEAR Foundation DevHub](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-2-near-foundation-developer-hub-grants) | $120,000 | Rolling | Decentralized AI training data marketplace. 3 phases: NEAR integration, federated sharing, developer SDK + marketplace. |
| [NSF PESOSE Track 1](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-3-nsf-pesose-track-1) | $300,000 | ~Sep 1, 2026 | Sustainable open-source ecosystem. 2 years: core infra + community (Y1), ecosystem growth + sustainability (Y2). |

Each includes: abstract, problem statement, technical approach, timeline, budget justification, team template, broader impacts, sustainability plan, and references.

---

## 3. Privacy & Security

[Full document](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5)

Ten privacy and security mechanisms, each with Rust code, PostgreSQL migrations, and integration into TC's existing auth/encryption stack (Ed25519, AES-GCM, RLS, TEE).

| # | Mechanism | Priority | Section |
|---|-----------|----------|---------|
| 1 | [Differential Privacy](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#1-differential-privacy-p0) | P0 | OpenDP, Laplace/Gaussian noise, RDP accountant, per-contributor privacy budget. |
| 2 | [Zero-Knowledge Proofs](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#2-zero-knowledge-proofs-p0) | P0 | Bulletproofs range proofs, arkworks R1CS, RISC Zero for general ZK. |
| 3 | [C2PA v2.3 Integration](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#3-c2pa-v23-integration-p0) | P0 | Content provenance manifests for trace bundles via c2pa-rs. |
| 4 | [EU AI Act Compliance](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#4-eu-ai-act-compliance-p0) | P0 | Article 12 mandatory logging (effective Aug 2, 2026), Article 50, NIST AI RMF mapping. |
| 5 | [Homomorphic Encryption](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#5-homomorphic-encryption-considerations-p1) | P1 | CKKS for encrypted embeddings, TEE-vs-HE tradeoff analysis. |
| 6 | [SCITT (RFC 9943)](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#6-scitt-rfc-9943-p1) | P1 | Append-only transparency log with Merkle proofs. |
| 7 | [W3C DIDs + VCs](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#7-w3c-dids--verifiable-credentials-p1) | P1 | Decentralized identity, BBS+ signatures for selective disclosure. |
| 8 | [Private Similarity Search](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#8-private-similarity-search-p1) | P1 | LSH on encrypted vectors for cross-org trace matching. |
| 9 | [CaMeL Capabilities](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#9-camel-capabilities-model-p2) | P2 | Capability tokens with attenuation, replacing bearer auth. |
| 10 | [Secure MPC](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#10-secure-multi-party-computation-p2) | P2 | Shamir secret sharing for cross-instance aggregate analytics. |

---

## 4. Novel Research Ideas

[Full document](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d)

Eight cross-domain ideas from non-obvious fields applied to AI trace management. Each includes academic citations, Rust implementation (~100+ lines), and a paper proposal with venue recommendation.

| # | Idea | Field | Section |
|---|------|-------|---------|
| 1 | [VCG Auctions for Trace Valuation](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#1-vcg-auctions-for-trace-valuation) | Mechanism Design | Truthful pricing via Vickrey-Clarke-Groves. Dominant-strategy incentive compatibility for trace contributions. |
| 2 | [NCD Novelty Scoring](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#2-normalized-compression-distance-ncd-for-novelty-scoring) | Information Theory | Compression-based similarity (zstd). Model-free, language-agnostic novelty pre-filter. |
| 3 | [Replicator Dynamics](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#3-replicator-dynamics-for-trace-lineage) | Evolutionary Game Theory | Track which agent strategies are growing/declining via fitness-proportional selection. |
| 4 | [SIR Pattern Spread](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#4-sir-epidemiological-model-for-pattern-spread) | Epidemiology | Susceptible-Infected-Recovered for modeling coding pattern diffusion. R_0 per pattern. |
| 5 | [Prospect-Theory Credits](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#5-prospect-theory-credits) | Behavioral Economics | Kahneman-Tversky value function for credit framing. Loss aversion for contributor motivation. |
| 6 | [TDA Trace Clustering](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#6-topological-data-analysis-tda-for-trace-clustering) | Algebraic Topology | Persistent homology for coverage gap detection. Barcodes for corpus topology visualization. |
| 7 | [Avrami Crystallization](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#7-avrami-crystallization-detection) | Materials Science | Phase transition model for corpus maturity. Detect when quality standards converge. |
| 8 | [Predictive Coding Saliency](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#8-predictive-coding--free-energy-for-trace-saliency) | Neuroscience | Friston's free energy principle for prediction-error-based novelty scoring. |

---

## 5. Competitive Landscape

[Full document](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07)

16 competitors across 8 categories, with TC's unique four-pillar moat analysis.

| Category | Competitors | Section |
|----------|-------------|---------|
| [Observability Platforms](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#observability-platforms) | [Langfuse](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#11-langfuse-acquired-by-clickhouse-january-2026), [Braintrust](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#12-braintrust-80m-series-b-february-2026), [Galileo](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#13-galileo-acquired-by-cisco-april-2026), [Helicone](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#14-helicone-acquired-by-mintlify-march-2026) |
| [Data Marketplaces](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#data-marketplaces) | [Vana](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#21-vana-user-owned-data-tokens), [Ocean Protocol](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#22-ocean-protocol-decentralized-data-marketplace) |
| [Provenance & Traceability](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#provenance--traceability) | [OriginTrail](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#31-origintrail-decentralized-knowledge-graph--dkg), [C2PA](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#32-c2pa--content-authenticity-initiative-v23-february-2026) |
| [AI Agent Frameworks](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#ai-agent-frameworks) | [IronClaw](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#41-ironclaw-near-ai), [Agent Trace Spec](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#42-agent-trace-spec-cursor--cognition-january-2026), [Letta](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#43-letta-trajectory-open-source-trace-normalization) |
| [AI Safety](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#ai-safety--alignment) | [METR](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#51-metr-model-evaluation--threat-research), [UK/US AISI](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#52-uk-aisi--us-aisi-government-ai-safety-institutes) |
| [Enterprise Logging](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#enterprise-logging) | [Datadog](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#61-datadog), [Splunk](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#62-splunk-cisco), [New Relic](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#63-new-relic) |
| [Research Infra](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#research-infrastructure) | [W&B](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#71-weights--biases-acquired-by-coreweave-may-2025), [Hugging Face](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#72-hugging-face) |
| [Blockchain AI](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#blockchain-based-ai) | [Bittensor](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#81-bittensor), [SingularityNET](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#82-singularitynet-asi-alliance) |

**Strategic sections**: [TC's Four-Pillar Moat](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#tcs-unique-moat-the-four-pillar-position) | [Positioning Matrix](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#market-positioning-matrix) | [Standards Alignment](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#standards-alignment) | [Market Size](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#market-size-context) | [Recommendations](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07#strategic-recommendations)

---

## 6. UX & Dashboard Design

[Full document](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b)

Ten design areas with ASCII mockups, technology recommendations, and implementation sketches.

| # | Area | Priority | Section |
|---|------|----------|---------|
| 1 | [Terminal UI (TUI)](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#1-terminal-ui-tui-for-contributors) | P0 | ratatui dashboard with 5 tabs (Overview, Traces, Quality, Credits, Settings). |
| 2 | [Web Dashboard](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#2-web-dashboard) | P0 | Next.js + React SPA with wireframes for dashboard, trace detail, admin. |
| 3 | [Progressive Disclosure](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#3-progressive-disclosure) | P0 | Drill-down chains: trace list → detail → tool calls → raw data. |
| 4 | [Provenance Cards](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#4-provenance-cards) | P1 | Embeddable SVG badges with quality tier (Diamond/Gold/Silver/Bronze). |
| 5 | [SSE Real-Time Events](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#5-sse-real-time-dashboard) | P0 | 7 event types, axum SSE handler, client-side integration. |
| 6 | [PWA / Mobile](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#6-pwa--mobile) | P2 | Push notifications, offline queueing, service worker sync. |
| 7 | [Onboarding Flow](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#7-onboarding-flow) | P0 | 5-step CLI wizard (`tc-contributor init`) + web onboarding cards. |
| 8 | [API Documentation](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#8-api-documentation) | P1 | OpenAPI 3.1 via utoipa, code examples in 4 languages. |
| 9 | [Quality Visualization](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#9-quality-visualization) | P1 | 5 chart types: histogram, scatter, time series, heatmap, system health. |
| 10 | [CLI UX Improvements](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#10-cli-ux-improvements) | P0 | indicatif progress bars, structured output, shell completions, `--watch` mode. |

---

## 7. IronClaw Integration

[Full document](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a)

How to integrate [IronClaw](https://github.com/nearai/ironclaw) (NEAR AI's agent framework) with TraceCommons for trace capture, scoring, and NEAR ecosystem synergy.

| # | Area | Section |
|---|------|---------|
| 0 | [System Overviews](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#0-system-overviews) | TC architecture + IronClaw architecture (10 crate families, WASM, 26 providers) |
| 1 | [Trace Capture](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#1-trace-capture-from-ironclaw-agents) | `IronClawTraceAdapter` implementing `TraceSource`, event type mapping |
| 2 | [Schema Extensions](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#2-schema-extensions-for-ironclaw-traces) | `IronClawTraceExtension` with WASM fuel, credentials, channel metadata |
| 3 | [TEE Attestation Bridge](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#3-tee-attestation-bridge) | End-to-end attestation chain: IronClaw TEE → TC TEE, chained hash verification |
| 4 | [WASM Trace Types](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#4-wasm-specific-trace-types) | Fuel metering as quality signal, `wasm_quality_factor()`, sandbox compliance scoring |
| 5 | [Multi-Channel Unification](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#5-multi-channel-trace-unification) | Cross-channel session correlation, channel-specific redaction (Telegram, Signal, Discord) |
| 6 | [NEAR Ecosystem](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#6-near-ecosystem-integration) | Shared identity via NEAR accounts, on-chain provenance, credit flow |
| 7 | [8 Improvements](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#7-specific-integration-improvements) | [Agent fingerprinting](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#71-agent-fingerprinting), [Tool analytics](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#72-tool-usage-analytics), [Provider comparison](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#73-provider-comparison), [Safety scoring](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#74-safety-scoring), [Credential hygiene](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#75-credential-hygiene-scoring), [Channel effectiveness](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#76-channel-effectiveness), [Cost optimization](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#77-cost-optimization), [Federated scoring](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#78-federated-scoring) |
| 8 | [Implementation Roadmap](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#8-implementation-roadmap) | 4 phases across 17+ weeks |
| 9 | [Configuration Reference](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#9-configuration-reference) | IronClaw TOML, TC server env vars, contributor JSON |
| 10 | [Open Questions](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#10-open-questions) | 5 unresolved design decisions |

---

## Reading Order

**For implementers**: Start with [Implementation Roadmap](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a), then [Privacy & Security](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5).

**For business/strategy**: Start with [Competitive Landscape](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07), then [Grant Proposals](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8).

**For researchers**: Start with [Novel Research Ideas](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d).

**For IronClaw/NEAR team**: Start with [IronClaw Integration](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a), then the [NEAR Foundation grant](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-2-near-foundation-developer-hub-grants).

**For contributors/users**: Start with [UX Design](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b).

---

## Priority Summary

### P0 (Build First)
- [Adaptive scoring with EMA thresholds](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#1-adaptive-scoring)
- [Multi-stage gate pipeline](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#2-multi-stage-gate-pipeline)
- [EU AI Act Article 12 compliance](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#4-eu-ai-act-compliance-p0) — effective Aug 2, 2026
- [Differential privacy for aggregates](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#1-differential-privacy-p0)
- [SSE real-time events](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#5-sse-real-time-dashboard)
- [Contributor onboarding flow](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#7-onboarding-flow)

### P1 (Build Next)
- [HDC fingerprinting](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#3-hdc-fingerprinting)
- [Dream consolidation / offline learning](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#4-dream-consolidation--offline-learning)
- [Self-learning mechanisms](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#5-self-learning-mechanisms)
- [ZK attestations](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#2-zero-knowledge-proofs-p0)
- [C2PA v2.3 integration](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#3-c2pa-v23-integration-p0)
- [IronClaw trace capture](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#1-trace-capture-from-ironclaw-agents)
- [NCD novelty pre-filter](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#2-normalized-compression-distance-ncd-for-novelty-scoring)

### P2 (Build Later)
- [Stigmergic coordination](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#6-stigmergic-coordination)
- [Biological/affective mechanisms](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#7-biological--affective-mechanisms)
- [Homomorphic encryption](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#5-homomorphic-encryption-considerations-p1)
- [Secure MPC](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#10-secure-multi-party-computation-p2)
- [VCG auctions](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#1-vcg-auctions-for-trace-valuation)
- [Replicator dynamics](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#3-replicator-dynamics-for-trace-lineage)

---

## Grant Deadlines

| Program | Amount | Deadline | Proposal |
|---------|--------|----------|----------|
| [NSF PESOSE Track 1](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-3-nsf-pesose-track-1) | $300,000 | ~Sep 1, 2026 | Draft ready |
| [NLnet Restack](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-1-nlnet-foundation----ngi-zero-restack) | EUR 48,000 | Nov 3, 2026 | Draft ready |
| [NEAR Foundation DevHub](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-2-near-foundation-developer-hub-grants) | $120,000 | Rolling | Draft ready |

---

## Cross-Reference: What Feeds What

| If you build... | It enables... |
|-----------------|---------------|
| [Adaptive Scoring](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#1-adaptive-scoring) | [Dream Consolidation](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#4-dream-consolidation--offline-learning) (needs baselines to recalibrate), [Self-Learning](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#5-self-learning-mechanisms) (needs score distributions) |
| [Multi-Stage Gates](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#2-multi-stage-gate-pipeline) | [NCD Pre-Filter](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d#2-normalized-compression-distance-ncd-for-novelty-scoring) (slots in as rung 1.5), [HDC Fingerprinting](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a#3-hdc-fingerprinting) (slots in as rung 2) |
| [Differential Privacy](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#1-differential-privacy-p0) | [Private Similarity Search](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#8-private-similarity-search-p1) (builds on DP guarantees), [Secure MPC](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#10-secure-multi-party-computation-p2) (extends cross-org privacy) |
| [SSE Events](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#5-sse-real-time-dashboard) | [TUI](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#1-terminal-ui-tui-for-contributors) (consumes events), [Web Dashboard](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#2-web-dashboard) (consumes events), [CLI --watch](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b#10-cli-ux-improvements) (consumes events) |
| [IronClaw Trace Capture](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#1-trace-capture-from-ironclaw-agents) | [WASM Trace Types](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#4-wasm-specific-trace-types) (needs IC traces), [Provider Comparison](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a#73-provider-comparison) (needs IC metadata) |
| [EU AI Act Compliance](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#4-eu-ai-act-compliance-p0) | [NLnet Grant](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-1-nlnet-foundation----ngi-zero-restack) (compliance is a key selling point), [C2PA](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5#3-c2pa-v23-integration-p0) (provenance for Article 50) |
