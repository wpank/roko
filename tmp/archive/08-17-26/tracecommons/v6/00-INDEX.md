# TraceCommons v6 -- Strategic Documents

**Date**: August 2026
**Synthesized from**: v5 (6 docs) + 27-agent research sweep (repo activity, market intel, 100+ new papers, UX patterns, grant program details) + 12-agent deep research sweep (research3.md findings) + research4.md verification sweep (18 claims verified, 6 citation corrections) + 4-agent gap extraction across all 22 docs (~200+ open items → doc 23) + v5 research index integration (26 papers verified, 7 docs updated, 3 title corrections, 1 misclassification flagged) + research6.md full integration (4 bad citations purged, 1 claim marked UNCONFIRMED, 1 figure marked UNSOURCED, 1 model identity caveat added)

---

## What Is TraceCommons?

TraceCommons (TC) is an open-source, Rust-based, privacy-preserving registry of AI coding agent session traces. Developers using AI coding assistants -- Claude Code, Codex, Cursor, and others -- generate session traces: records of what the agent did, which tools it called, what worked, and what failed. TC collects these traces (with privacy scrubbing), scores them for quality and novelty inside Trusted Execution Environments, and compensates contributors with credits on the NEAR blockchain.

The goal is a shared, contributor-owned corpus of agent behavior data that no single vendor controls. Built by Zaki Manian (co-creator of Cosmos SDK/IBC). ~235K LOC Rust, 6 crates, MIT/Apache-2.0 dual-licensed. Pilot deployed on GCP.

**Current traction**: ~352 submissions, ~13/week, 3 contributors, 6 GitHub stars.

---

## Glossary

| Term | Definition |
|---|---|
| **TC** | TraceCommons. This project. |
| **TEE** | Trusted Execution Environment. Hardware-isolated compute enclaves (Intel TDX, NVIDIA GPU TEE) where code runs in encrypted memory. TC scores traces inside TEEs so the scoring model never sees unencrypted data outside the enclave. This is the core privacy guarantee. |
| **NEAR** | A layer-1 blockchain. TC uses NEAR for three things: (1) credit settlement -- contributors earn NEAR-denominated credits for accepted traces, (2) TEE-hosted scoring -- NEAR AI Cloud provides Intel TDX + NVIDIA GPU TEE infrastructure for privacy-preserving model inference, (3) identity -- contributor NEAR accounts for payout. |
| **IronClaw** | NEAR AI's open-source agent runtime (12.6K GitHub stars). TC's primary integration partner -- 3 PRs merged, 20K+ lines of integration code. Supports 26+ LLM providers, runs across CLI, Telegram, Slack, Discord, Signal. |
| **Gate pipeline** | TC's multi-stage quality scoring pipeline. A submitted trace goes through: redaction (PII/secret scrubbing) -> chunking -> embedding (BGE-large-en-v1.5) -> cosine similarity against HNSW index -> perplexity scoring (via Qwen 3.6 35B on NEAR AI Cloud TEE) -> gate evaluation (accept/reject). Credit formula: `q = f * g * a` where f=quality factor, g=novelty factor, a=anomaly penalty. |
| **OTel** | OpenTelemetry. Vendor-neutral observability standard for traces, metrics, and logs. TC plans OTel-native ingest, but the GenAI semantic conventions (`gen_ai.*`) are still "Development" status -- not stable. |
| **MCP** | Model Context Protocol. Anthropic's standard for connecting LLMs to external tools and data sources. Relevant to TC's integration surface. |
| **A2A** | Agent-to-Agent protocol. Google's open protocol for inter-agent communication (v1.0.0, 150+ orgs). Relevant to TC's multi-agent trace stitching. |
| **GPAI** | General-Purpose AI. EU AI Act category covering foundation model providers. GPAI transparency obligations are live as of Aug 2, 2026 -- compliance logging is a TC market opportunity. |
| **SKILL.md** | A proposed convention (within the Agent Skills ecosystem, 490K+ skills (**⚠️ UNSOURCED** — cannot be traced to a primary source)) for declaring agent capabilities. TC's scoring and provenance tracking could serve as a quality layer for skill registries. |
| **cargo-dist** | Rust ecosystem standard for CLI binary distribution. Used by ripgrep, bat, delta, etc. TC's recommended distribution mechanism (doc 01). |

---

## Documents

| # | Document | Focus |
|---|---|---|
| [01](01-user-acquisition-and-growth.md) | **User Acquisition & Growth** | cargo-dist distribution, Claude Code hooks integration, Sentry wizard model (90-second time-to-insight), artifact-driven viral loops, Error Hub flywheel (AgentDebugX 55.8%→63.6% via failure repairs), cold-start playbook, AgentGUI verified (38% faster trace inspection, p=0.023), AgenTracer-8B, TRAIL failure taxonomy |
| [02](02-scoring-and-quality-pipeline.md) | **Scoring & Quality Pipeline** | **CRITICAL: Issue #210 (0/99 accepted) and #219 (redaction penalty)**, conformal prediction (ToolChain-CRC), data valuation (Shapley gameability proven -> VCG/Myerson), 100+ new papers integrated, causal failure attribution (CAR ~14% baseline), process mining (STRACE/PrefixGuard), trajectory compression (ACE/Slipstream) |
| [03](03-integrations-and-ecosystem.md) | **Integrations & Ecosystem** | **CORRECTED: OTel GenAI NOT stable** (all "Development" status), A2A v1.0.0 (150+ orgs), Agent Skills 490K+ (ClawHavoc incident, SkillSieve/SkillSpector), cross-agent cost tracking (TokenShift/Exceeds Ink), Claude Code 30 hooks, IronClaw status update, Error Hub expanded (AgentDebugX metrics, AgenTracer-8B TracerTraj corpus, TRAIL taxonomy), Externalization Review schema gap analysis, AgentSpec scaffold metadata |
| [04](04-production-hardening.md) | **Production Hardening** | Updated with shipped items (PR #239 cell suppression, #240 binary releases, #244 background daemon, #212 gate API extraction), remaining work, open PRs (#241 private insights, #248 Linux GTK, #249 Windows, #251 logit capture) |
| [05](05-strategy-and-grants.md) | **Strategy & Grants** | **CORRECTED: EU AI Act Article 12 deferred to Dec 2027** (Digital Omnibus); GPAI enforcement live Aug 2026; compliance market 17-38B EUR by 2030; NLnet opens Sep 3 (Provability Fabric precedent); Mozilla no active MTF call; NEAR community-DAO $45M+ |
| [06](06-research-paper-index.md) | **Research Paper Index v6.2** | ~150 verified papers across 12 categories. v6.2 updates: 3 title corrections (ReasoningBank, RHO, Dynamic Cheatsheet), 18 papers upgraded to v6-verified, 1 misclassification flagged (Inference-Time Steering = ROBOTICS paper, not LLM agent UX), AgenTracer-8B ICLR 2026 status unverified |
| [07](07-deep-research-queries.md) | **Deep Research Queries v2** | 25 next-round queries across 5 categories, generated from gaps identified by research sweep |

### Deep Research Documents (from research3.md sweep)

| # | Document | Focus |
|---|---|---|
| [08](08-redaction-invariant-scoring.md) | **Redaction-Invariant Scoring** | **Fixes Issue #219.** 5 approaches. **Updated: TDX attestation GO** (pipeline reordering is threat-model-neutral). Qwen3-Coder native FIM confirmed (zero extra VRAM). arXiv:2309.08628 anchors PPL penalty (3.2× for allowList masking). Recommended path: placeholder exclusion → TEE raw scoring → Qwen FIM. |
| [09](09-conformal-gate-calibration.md) | **Conformal Gate Calibration** | **Fixes Issue #210.** 9+ conformal methods. **Updated: LOCUS corrected** (per-input reliability wrapper, NOT group-conditional). **SSBC added** for small-sample correction (n≥47, arXiv:2509.15349). Wang & Qiao 2025 for group-conditional. |
| [10](10-ground-truth-free-quality.md) | **Ground-Truth-Free Quality** | **Fixes PR #216 confound.** Judge-Aware Ranking. **Updated: 4 independent judges** (not 5). PerplexityScorer + TokenRarityScorer collapsed to ForwardPassJudge (shared forward pass violates conditional independence). |
| [11](11-verified-skills-tier.md) | **Verified Skills Tier** | TC's differentiated wedge. SkillVetBench (89% static miss rate), MalSkillBench, ClawGuard, SkillSieve (F1 **corrected** to 0.800). Multi-session TEE-attested behavioral provenance. **New §6A: Quality Architecture** — SkillOS (trainable skill curator, TC gate pipeline as judge reward, arXiv:2605.06614), SkillRevise (TC as fixed verifier, SkillsBench 36.05%→61.63%, arXiv:2606.01139). |
| [12](12-trajectory-rag.md) | **Trajectory RAG** | TC's potential **killer feature**. BQP replaces MMR (2.4-22.9× faster). Brute-force HDC for side-channel-free retrieval. **New §7: Beyond Retrieval** — Dynamic Cheatsheet (TC IS the cross-agent cheatsheet, arXiv:2504.07952), ReasoningBank (distill from success+failure, arXiv:2509.25140), LEGOMem (typed decomposition before indexing, arXiv:2510.04851), Sleep-time Compute (~5× test-time reduction, arXiv:2504.13171), RHO (self-supervised harness optimization, 59%→78%, arXiv:2606.05922). |
| [13](13-trace-provenance-anti-sybil.md) | **Trace Provenance & Anti-Sybil** | 4-phase implementation. Deterministic TEE inference has throughput costs (**⚠️ previously cited 34-61% from arXiv:2606.03019 is a WRONG CITATION** — needs re-sourcing). Brute-force HDC scan (determinism + side-channel resistance). **New §2.7-2.10:** TEE Survey for Agentic AI (compound attestation + GPU-TEE open challenges, arXiv:2605.03213), GaaS Trust Factor → TC anomaly penalty (arXiv:2508.18765), Privilege Attenuation (monotonic permission restriction, arXiv:2602.11865), Cryptographic Verifiability linkability requirement (arXiv:2503.22573). 10 papers verified (up from 6). |
| [14](14-corpus-seeding.md) | **Corpus Seeding** | Open-SWE-Traces (207,489 trajectories, 9 languages, incl. Rust). HNSW seeding pipeline. Bias mitigation (temporal downweighting, per-workload namespaces). 3-phase transition: pure seed → hybrid → organic-majority. |
| [15](15-gpai-compliance.md) | **GPAI Compliance** | **Updated: Digital Omnibus is Regulation (EU) 2026/1744** (OJ 24 Jul 2026). Training data summary template 3-section structure confirmed. VerifyWise (BSL 1.1, not FOSS). ClickHouse/Langfuse acquisition. LLM observability market $1.97B→$9.26B. Pricing figures unverified. |
| [16](16-incentive-mechanism-design.md) | **Incentive Mechanism Design** | Shapley proven gameable (3 papers). **Updated: N=3 collusion impossible to resist** — quality gates carry the load. Owen Sampling for multi-agent credit (arXiv:2508.21261). New section on collusion at single-digit N. |
| [17](17-structural-embeddings.md) | **Structural Embeddings** | VS-Graph (450×, pure Rust). **Updated: accuracy caveat** (MUTAG/DD benchmarks ≠ TC trajectory graphs). **Brute-force HDC scan** recommended over HNSW (determinism + side-channel-free). ONNX non-deterministic in TEEs (source needed — previously cited arXiv:2501.05867 is a **WRONG CITATION**). |
| [18](18-otel-genai-technical-state.md) | **OTel GenAI Technical State** | Precise convention status: all Development, no Stable attributes. `gen_ai.system` → `gen_ai.provider.name` rename. Dedicated repo has no tagged release. Pinning strategy + alias shim design (~50 LOC). OpenInference parallel conventions. |
| [19](19-active-learning-bootstrap.md) | **Active Learning Bootstrap** | **Updated: anchoring bias warning expanded** (LLM pre-labels inflate agreement). Snorkel reimplementable in ~200-500 LOC Rust. Pairwise comparison raises α vs absolute scoring. Realistic α is 0.4-0.6. |
| [20](20-deep-research-queries-v4.md) | **Deep Research Queries v4** | 52 queries. **Updated: 5 of 10 quick-start queries now ANSWERED** (TDX GO, Qwen FIM, GPAI template, Digital Omnibus citation, partial answers for JAR/collusion/agreement). 6 citation corrections applied. |
| [21](21-bootstrap-sequencing.md) | **Bootstrap Sequencing** | **NEW.** Breaks the circular dependency (scorers → labels → disagreement → scorers). Start with single cheap scorer as weak supervision. 4-phase sequence: permissive threshold → weak labels → ensemble → conformal gate LAST. Anti-patterns. |
| [22](22-interop-and-agent-formats.md) | **Interop & Agent Formats** | Cross-agent session format matrix (Claude Code, Codex, Cursor, Copilot, Gemini, IronClaw). OTel + OpenInference + W3C PROV as interop stack. WASM-sandboxed scorer plugins. **Updated:** LatentMAS latent-space coverage gap (arXiv:2511.20639, ICML 2026 Spotlight), AgentSpec scaffold architecture (arXiv:2606.14674), Externalization four-category taxonomy (arXiv:2604.08224), TRAIL failure annotation standard (arXiv:2505.08638), ACP/ANP protocols added to four-protocol phased roadmap (MCP→ACP→A2A→ANP), Evidence Tracing six-dimensional taxonomy (arXiv:2606.04990). |
| [23](23-deep-research-queries-v5.md) | **Deep Research Queries v5** | **NEW.** 69 queries across 11 categories + citation verification batch (79 items). Synthesized from ~200+ gaps extracted across all 22 v6 docs. 5 internal data questions, 5 decision-blockers, market intelligence, regulatory/grants, mechanism design, TEE/determinism, scoring, annotation, agent ecosystem. Supersedes doc 20 for 5 fully answered queries; retains all others with updated context. |

---

## Critical Intelligence Updates (v5 -> v6)

1. **Issue #210: "0 of 99 sessions would be accepted"** -- Fundamental scoring logic inversion. The gate is rejecting everything. This is the single most urgent fix. All growth efforts are blocked until resolved.

2. **Issue #219: Redaction penalizes quality scores** -- Thorough redaction (which TC *requires*) is penalized by the perplexity scorer, which sees redaction markers as incoherent noise. Perverse incentive: less privacy = higher scores. IronClaw contributors are systematically disadvantaged because IronClaw's redaction is particularly thorough.

3. **OTel GenAI conventions are NOT stable** -- v5 stated "v1.42.0 is the de facto standard." All `gen_ai.*` conventions are still "Development" status. Conventions moved to a dedicated repo (June 2026) with unstable schema. TC must pin attribute versions and plan for breaking changes.

4. **EU AI Act Article 12 deadline deferred** -- v5 stated "Article 12 is law as of August 2, 2026." The Digital Omnibus Regulation (adopted July 2026) deferred standalone high-risk AI system deadlines to **December 2, 2027**. However, GPAI provider transparency obligations ARE live as of Aug 2, 2026. Grant applications must use precise language.

5. **Third contributor arrived** -- brapse PR #250, Aug 10, 2026. TC's first contribution outside core team. Unknown acquisition channel.

### New Research Intelligence

- **100+ new papers** not in v5's index, across 6 categories
- **Shapley fragility proven** -- data valuation via Shapley/semivalue is gameable; DSIC mechanisms (VCG/Myerson) required for incentive compatibility
- **Conformal prediction for quality** -- ToolChain-CRC, distribution-free coverage guarantees applicable to TC gate scores
- **Causal failure attribution** -- correlational methods achieve only ~14% accuracy; interventional methods (CAR) dramatically better
- **Agent Skills at scale** -- 490K+ skills (**⚠️ UNSOURCED** — cannot be traced to a primary source), ClawHavoc attack (341 malicious), security scanner bypass documented
- **Process mining applied to agent traces** -- Agent Behavior Mining, PrefixGuard, STRACE
- **Trajectory compression SOTA** -- ACE (dynamic KV pooling), Slipstream (spec-exec), CompactionRL, ARC

### Research4 Verification Updates

- **TDX attestation GO for pipeline reordering** -- RTMRs measure boot chain, not intra-application control flow. Score-then-redact within same enclave binary requires no re-attestation (doc 08)
- **Qwen3-Coder native FIM confirmed** -- `<|fim_prefix|>/<|fim_suffix|>/<|fim_middle|>` tokens verified. Zero extra VRAM for FIM infilling (doc 08)
- **SSBC for small-sample conformal** -- arXiv:2509.15349, validated at n=47-100, fixes ~40% coverage violation at nominal 90% (**⚠️ the 40% figure is UNCONFIRMED** — paper exists but the specific number could not be verified from the abstract; body verification needed) (doc 09)
- **BQP replaces MMR for diversity retrieval** -- arXiv:2604.02554, 2.4-22.9× faster with sublinear-in-k scaling (doc 12)
- **Brute-force HDC scan is the correct approach at current scale** -- determinism + side-channel resistance; HNSW deferred to 100K+ traces (docs 12, 13, 17)
- **N=3 collusion impossible to resist via payment mechanism** -- quality gates + provenance attestation carry anti-manipulation load (doc 16)
- **ClickHouse (not Databricks) acquired Langfuse** -- January 16, 2026, $400M Series D (docs 12, 15, 22)
- **Digital Omnibus is Regulation (EU) 2026/1744** -- OJ 24 Jul 2026, in force 27 Jul 2026. Parliament 423-57, Council 29 Jun 2026 (doc 15)
- **Snorkel reimplementable in ~200-500 LOC Rust** -- no fundamental Python dependency (doc 19)
- **6 citation corrections applied** -- 2509.24291 is GIRCSE (not hard-negative mining), 2504.17703 WITHDRAWN, 2604.16656 is "Defragmenting LMs" (not grounded init), 2602.02007 title mismatch, 2605.09702 attribution correction, VerifyWise is BSL 1.1 (not FOSS)
- **2 additional citation corrections (post-v6)** -- arXiv:2501.05867 ("Neural network verification challenges as programming-language challenges") is a WRONG CITATION for the ONNX non-determinism claim; arXiv:2606.03019 ("Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds") is a WRONG CITATION for the 34-61% batch-invariant kernel throughput cost claim. Both technical claims are likely correct but need re-sourcing. Marked as WRONG CITATION in docs 13 and 17.

### v5 Research Index Integration (v6.2)

- **26 papers verified** from v5 research index (arXiv + web confirmation)
- **7 documents updated**: 01, 03, 06, 11, 12, 13, 22
- **3 title corrections**: ReasoningBank ("Scaling Agent Self-Evolving with Reasoning Memory"), RHO ("Evolving Agents in the Dark: Retrospective Harness Optimization via Self-Preference"), Dynamic Cheatsheet (arXiv:2504.07952, ICLR 2026)
- **1 misclassification flagged**: Inference-Time Steering (arXiv:2411.16627) is a ROBOTICS paper (ICRA 2025), not LLM agent UX
- **1 claim verified**: AgentGUI 38% faster trace inspection (p=0.023), ETH Zurich confirmed
- **1 claim unverified**: AgenTracer-8B ICLR 2026 acceptance — cannot confirm via arXiv metadata or web search
- **Key new concepts integrated**: Dynamic Cheatsheet (TC IS the cross-agent cheatsheet), SkillOS/SkillRevise (TC gate pipeline as judge/verifier), LEGOMem (typed trace decomposition), Sleep-time Compute (pre-computation from traces), RHO (self-supervised harness optimization), LatentMAS (latent-space coverage gap), AgentSpec (scaffold architecture), Privilege Attenuation, GaaS Trust Factor, TRAIL failure taxonomy

### Recent Repo Activity (110+ PRs)

**Merged:** PR #244 (background daemon), #240 (binary releases on tag), #239 (cell suppression replacing DP), #212 (gate API extraction), #250 (third contributor brapse).

**Open:** PR #251 (logit capture design), #241 (private insights via NEAR AI enclave), #249 (Windows support), #248 (Linux GTK contributor shell).

---

## Priority Order (Across All Documents)

### Urgent (Days)

1. **Fix Issue #210: conformal gate calibration + SSBC** (02, 09) -- 0/99 accepted. Quantile gate + SSBC small-sample correction fixes this immediately. Nothing works until this is fixed.
2. **Fix Issue #219: TEE raw scoring (GO)** (02, 08) -- TDX attestation permits pipeline reordering. Phase 2-alt is now the recommended path (days, not weeks).

### Now (Weeks)

3. **Follow bootstrap sequence** (21) -- permissive threshold → weak labels → ensemble → conformal gate LAST. Breaks the circular dependency.
4. **Seed corpus from Open-SWE-Traces** (14) -- 207K trajectories, bootstraps HNSW index and calibration set
5. cargo-dist binary distribution (01, PR #240 partially addresses)
6. Claude Code `SessionEnd` hook integration (01, hours)
7. Wire TokenRarityScorer as ForwardPassJudge (02, 10, hours) -- collapse with PerplexityScorer into 1 judge for independence
8. MinHash dedup via Rensa (02, 1-2 days)
9. `tc scan` with immediate local insights (01, 1-2 weeks)
10. Prometheus metrics + tower-http TraceLayer (04, ~80 LOC + 1 line)
11. Graceful shutdown + /health/ready (04, ~150 LOC)
12. OTel alias shim for gen_ai.system → gen_ai.provider.name (18, 22, ~50 LOC)
13. Brute-force HDC scan (not HNSW) for structural fingerprints (17) -- deterministic + side-channel-free at current scale

### Next (1-3 Months)

14. **Active learning bootstrap** (19) -- 4-week protocol, ~35 person-hours, pairwise comparison for higher α
15. **Judge-aware quality estimation with 4 judges** (10) -- ForwardPassJudge + 3 independent judges, fixes PR #216 confound
16. OTel-native ingest + OpenInference (03, 18, 22, 2-4 weeks)
17. Fix bake-off corpus (02, ~1 week)
18. **Wire VCG into credit settlement** (16) -- `vcg_allocate` already built, replace gameable Shapley
19. IronClaw critical fixes -- 4 items (03)
20. NLnet application -- submit Nov 3, GPAI compliance angle (05, 15)
21. Snorkel weak supervision in Rust (~200-500 LOC) (19)
22. Error Hub MVP (03, 6-8 weeks)
23. First corpus analysis post (01, 1 week)
24. TEE-signed ingestion attestation (13, Phase 1)
25. NEAR DevHub application (05, rolling)
26. Qwen3-Coder native FIM infilling sub-score (08, zero extra VRAM)

### Then (3-6 Months)

27. **Verified skills tier** (11) -- TC's differentiated wedge, multi-session behavioral provenance
28. **Trajectory RAG MVP with BQP diversity** (12) -- BQP replaces MMR (2.4-22.9× faster)
29. Multi-layer novelty pipeline with conformal prediction + structural embeddings (02, 09, 17)
30. **VS-Graph HDC structural embedding** (17) -- pure Rust, accuracy validation needed on TC's trajectory graphs
31. **GPAI compliance positioning** (15) -- Digital Omnibus now law (Reg (EU) 2026/1744)
32. **Usage-linked credits** (16) -- avoid Vana emissions trap
33. WASM-sandboxed scorer plugins with per-plugin RTMR attestation (22)
34. Trajectory replay prototype (03, 8-10 weeks)
35. Mozilla Tech Fund application -- wait for active call (05)
36. Compound system auto-optimization (02)
37. Container image (04)

---

## Key Corrections From v5

| v5 Claim | v6 Correction | Source |
|---|---|---|
| "OTel GenAI v1.42.0 is the de facto standard" | Conventions are still "Development" status; moved to dedicated repo June 2026 | OTel GenAI SIG repo |
| "Article 12 is law as of August 2, 2026" | Standalone high-risk deferred to Dec 2, 2027 via Digital Omnibus; GPAI obligations live Aug 2026 | EU Digital Omnibus Regulation |
| Gate pipeline working normally | Issue #210: 0/99 sessions accepted; scoring logic inversion | TC GitHub Issues |
| Redaction is transparent to scoring | Issue #219: redaction penalizes quality scores | TC GitHub Issues |
| "~40 compatible products" for Agent Skills | 490K+ skills (**⚠️ UNSOURCED** — cannot be traced to a primary source), 32+ adopters, but ClawHavoc attack (341 malicious) and SkillSieve bypass documented | AgentSkills ecosystem research |
| SkillSieve F1 = 0.920 | F1 = **0.800** (precision 0.752, recall 0.854). 0.920 is the full three-layer pipeline, not the SSD-augmented Layer-2 ablation. | arXiv:2604.06550 |
| "93% performance at 6% cost" from arXiv:2502.11767 | Source is arXiv:**2502.16892** (Zhang & Takada). "93%" is relative performance retention (85.42% vs 94.63%), "6%" is computational cost vs GPT. | arXiv:2502.16892 |
| Datadog OTel GenAI support "December 1, 2026" | Actually **December 1, 2025** (past, not future). OTel SDK/Collector v1.37. | Datadog docs |
| GraphTracer 18.18% improvement | arXiv:2510.10581 **v2 withdrawn** December 2025 due to "fundamental error in methodology." All empirical claims invalidated. | arXiv withdrawal notice |
| Langfuse acquired by Databricks | **ClickHouse** acquired Langfuse, January 16, 2026, $400M Series D. ClickHouse valuation tripled to ~$15B. | ClickHouse/Langfuse announcement |
| LOCUS = group-conditional conformal | LOCUS (arXiv:2603.01971) is a **per-input loss-scale reliability wrapper for regression**, NOT group-conditional conformal. Correct anchor: Wang & Qiao 2025 (AISTATS PMLR 258:4888-4896). | arXiv:2603.01971 |
| arXiv:2509.24291 = hard-negative mining | Actually **GIRCSE** (generative contrastive sentence embeddings). Completely different paper. | arXiv:2509.24291 |
| arXiv:2504.17703 = federated learning survey | Paper **WITHDRAWN** due to disputed authorship. Do not cite. | arXiv:2504.17703 |
| arXiv:2604.16656 = "grounded vocab init" | Actually **"Defragmenting Language Models"** (vocab expansion/interpretability). Correct grounded init paper: arXiv:2604.02324 (GTI). | arXiv:2604.16656 |
| arXiv:2602.02007 = "xMemory" | Actual title: **"Beyond RAG for Agent Memory: Retrieval by Decoupling and Aggregation"** (Hu et al.). "xMemory" is an internal shorthand. | arXiv:2602.02007 |
| VerifyWise is FOSS | VerifyWise uses **BSL 1.1** (Business Source License) — source-available, NOT open source per OSI definition. | verifywise.ai |
| PerplexityScorer + TokenRarityScorer = 2 independent judges | They share the SAME forward pass through Qwen 3.6 35B. **ONE judge** (ForwardPassJudge) for all independence-sensitive analyses. | BT-σ / arXiv:2605.09702 |
| MMR is the recommended diversity retrieval | **BQP** (arXiv:2604.02554) is 2.4-22.9× faster than MMR at θ≥0.5 with sublinear-in-k scaling. MMR has no approximation guarantee. | arXiv:2604.02554 |
