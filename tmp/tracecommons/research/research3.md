# TraceCommons Round-3 Deep Research Sweep: Net-New, Actionable Findings

## TL;DR
- **Both blocking issues are solvable with off-the-shelf, verified 2025–2026 methods.** Issue #210 (gate inversion) is fundamentally a *threshold-calibration* problem, not a scoring-logic problem: replace fixed thresholds with **quantile / conformal-risk-controlled gates** that pin acceptance rate by construction (the accept-side is well-precedented — e.g., SCOPE's Conformal Linear Gate, arXiv:2606.21255, and UCB-based risk calibration, arXiv:2607.04430). Issue #219 (redaction penalty) is solvable by **excluding placeholder token positions from the perplexity average and masking multi-subword placeholders as a unit** (Salazar 2020 PLL — arXiv:1910.14659, which "computed by masking tokens one by one"; plus Kauf & Ivanova 2023 PLL-word-l2r, arXiv:2305.10588), plus adding typed placeholders as single in-vocabulary tokens.
- **The single most defensible citation that TC's current design is perverse** is arXiv:2603.29497: random `[REDACTED]` masking *increased* a privacy-risk score by Δ=−0.31 because "uninformed redaction disrupts coherence while preserving identifying content" — i.e., a coherence-based scorer reacts to the *presence of mask noise*, exactly TC's failure mode.
- **A verification pass found real errors in TC's prior corpus:** SkillSieve's F1 is **0.800, not 0.920** (arXiv:2604.06550); the "93% performance at 6% cost" active-learning result is from arXiv:**2502.16892**, not the survey 2502.11767; and Open-SWE-Traces contains exactly **207,489** trajectories (arXiv:2606.16038). Several forward-dated IDs in the brief could not be independently resolved within budget and are flagged UNVERIFIED below.

---

## Executive Summary — Ranked Highest-Impact Net-New Findings

**#1 (fixes #219) — Placeholder-excluded pseudo-perplexity + whole-span masking.** The vanilla perplexity scorer penalizes redaction because (a) it averages surprisal over placeholder positions and (b) multi-subword placeholders like `<API_KEY>` fragment under BPE and look maximally surprising. Salazar et al. 2020 "Masked Language Model Scoring" (ACL 2020, pp. 2699–2712, arXiv:1910.14659) gives the pseudo-log-likelihood (PLL) baseline — scores "computed by masking tokens one by one." Kauf & Ivanova 2023 "A Better Way to Do Masked Language Model Scoring" (ACL 2023, arXiv:2305.10588) contributes **PLL-word-l2r**, which masks the target token and all within-word subtokens to its right — precisely the fix for multi-token placeholders. *TC path:* compute perplexity only over non-placeholder tokens while still conditioning on placeholders as context; treat each placeholder span as a single masked unit. Expected impact: removes the systematic penalty that disadvantages IronClaw's thorough redaction. Net-new vs. TC's corpus, which had only the raw perplexity metric.

**#2 (fixes #219) — Typed placeholders as single in-vocabulary tokens.** Lehman et al. "Do We Still Need Clinical Language Models?" (arXiv:2302.08091) add de-identification tags (e.g. `[NAME]`) to the tokenizer vocabulary so each tag is ONE in-vocab token; this reduced the MIMIC corpus from 2,400,714,781 to 2,335,573,220 tokens and eliminates subword-fragmentation surprise. *TC path:* extend the Qwen scorer's tokenizer (or a scoring-only wrapper vocabulary) with TC's fixed placeholder set (`<PERSON>`, `<API_KEY>`, `<SECRET>`, …) and initialize embeddings with vocab-mean / grounded init (arXiv:2604.02324, arXiv:2604.16656).

**#3 (fixes #219) — Prefer typed/realistic surrogates over bare `[REDACTED]`.** Two independent 2025–2026 results show blank placeholders destroy utility while typed/realistic surrogates keep text in-distribution: Wu et al. "Anonymization and Information Loss" (arXiv:2511.15364) uses readable numbered placeholders (PERSON_1, ORG_1) and still measures ~20–67% information loss depending on scheme (a head-to-head "horse race" collapsed a sentiment coefficient from 2.331 to 0.775, ≈67% loss); Vakili et al. 2024 (BMC Med Inform Decis Mak, DOI 10.1186/s12911-024-02546-8) show end-to-end pseudonymization with realistic same-type surrogates causes statistically minimal F1 harm across 300 models. *TC path:* if privacy policy permits, replace opaque markers with typed synthetic surrogates before scoring (score-time only; storage can remain redacted).

**#4 (fixes #219, alternative) — Infilling-likelihood coherence score.** A redaction-invariant quality signal that structurally cannot penalize the mask: score how well a fill-in-the-middle model predicts plausible content for the masked span given prefix+suffix. AST-FIM / Real-FIM-Eval (arXiv:2506.00204) compute perplexity only over the middle span; MARIA (arXiv:2502.06901) enables efficient masked-span infilling (MARIA-7B downstream perplexity 2.82 at low mask ratio vs DiffuLlama's 6.74–10.36). *TC path:* add an infilling-based coherence sub-score and blend with the perplexity factor `f`.

**#5 (fixes #210) — Conformal / quantile acceptance-rate gates.** TC's "0 of 99 accepted" is a threshold-calibration failure. The accept-side is well-precedented: SCOPE (arXiv:2606.21255) builds a **Conformal Linear Gate** where the threshold τ is the split-conformal (1−ε)-quantile of calibration scores, guaranteeing false-rejection ≤ ε + 1/(M+1); UCB-based risk calibration (arXiv:2607.04430) uses Hoeffding / Clopper-Pearson upper bounds to certify the selection-conditioned error at a target rate. *TC path:* set the gate threshold to the empirical quantile of calibration-trace scores matching a target acceptance rate (e.g. accept the top 60%), rather than a hand-set absolute cutoff. This makes acceptance rate a directly controllable dial and mathematically prevents "0 accepted."

**#6 (fixes #210) — Target-acceptance quantile tuning with a validation set.** LOCUS (arXiv:2603.01971) formalizes tuning the acceptance threshold λ on a labeled validation set to hit a target conditional exceedance level η, noting that "marginal calibration does not guarantee that the induced accepted set satisfies a desired conditional exceedance rate." Abstention-rate calibration (arXiv:2402.12997) shows MAE between target and achieved abstention rate grows with the rate. *TC path:* with ~350 calibration traces, tune to a target acceptance and report the finite-sample MAE band.

**#7 — Weighted / covariate-shift conformal prediction for shifting contributor cohorts.** Net-new beyond TC's ToolChain-CRC / Role-Stratified-CRC: Tibshirani et al. 2019 weighted conformal prediction reweights calibration scores by the train/test likelihood ratio w(x)=dP̃_X/dP_X to preserve coverage under covariate shift; WATCH (arXiv:2505.04608) uses weighted-conformal martingales for adaptive monitoring; Wang & Qiao 2025 (AISTATS, PMLR 258:4888–4896) handle *generalized covariate shift with posterior drift*. *TC path:* when a new agent family (e.g. a new IronClaw release) shifts the trace distribution, reweight calibration scores rather than recollecting labels.

**#8 — Judge-aware, ground-truth-free quality estimation (fixes PR #216 confound).** TC's bake-off used implicitly biased labels. Judge-Aware Ranking (arXiv:2601.21817) extends Bradley-Terry-Luce with judge-specific discrimination parameters, jointly estimating latent trace quality and judge reliability *without reference labels*; "Bias and Uncertainty in LLM-as-a-Judge" (arXiv:2605.06939) applies the Rogan-Gladen estimator to correct for judge sensitivity/specificity; "Reliability without Validity" (arXiv:2606.19544) shows raw exact-match overstates judge reliability and recommends reporting chance-corrected agreement. *TC path:* replace single-scorer labels with a judge-aware latent-quality model; this is the statistically correct fix for the bake-off confound.

**#9 — Behavioral / runtime skill verification is the validated missing layer (TC's I3 thesis confirmed).** Post-ClawHavoc, three independent 2026 benchmarks show static analysis alone is insufficient and execution traces are required: SkillVetBench (arXiv:2606.00925) reports semantic/signature-only baselines miss up to **89%** of malicious skills and adds sandbox execution-trace verification; MalSkillBench (arXiv:2606.07131) captures syscall traces (`strace -f`, `inotifywait`) over the agent process tree; ClawGuard (arXiv:2604.11790) is a runtime framework. *TC path:* TC's execution-provenance corpus is exactly the behavioral layer these benchmarks say is missing — position a "verified skills, backed by execution provenance" tier.

**#10 — Trace provenance / anti-Sybil is now a concrete design menu.** Beyond VET (arXiv:2512.15892, which finds Web Proofs practical at <3× overhead and a TEE Proxy sufficient for public APIs), Proof-of-Execution (arXiv:2607.05397) issues Execution Attestation Certificates with ~2.7 ms overhead on a minimal flow, 4.4% on batch workloads, and ~1.1 KB per 8-event trace. *TC path:* since TC already runs Intel TDX, attest the *capture* step (TEE-signed logs at ingestion) as the cheapest credible anti-fabrication guarantee; reserve Web Proofs for high-value submissions.

**#11 — Corpus seeding is immediately available.** Open-SWE-Traces (arXiv:2606.16038, verified) = **207,489** agent trajectories across 9 languages incl. Rust, permissively licensed (MIT/Apache/BSD), already PII-filtered; NVIDIA hosts it on Hugging Face. Nebius SWE-rebench-openhands-trajectories adds 67,074 more. *TC path:* seed the HNSW novelty index and calibration set immediately — directly attacks the "3 contributors / 352 submissions" cold-start.

**#12 — Distribution: cargo-dist + axoupdater gives TC its <90s install and zero-friction update.** cargo-dist's `install-updater = true` ships a standalone `axoupdater` alongside the binary so, per the axodotdev/cargo-dist CHANGELOG, "users who install your software via the shell or PowerShell installers will receive a standalone updater program alongside your program itself"; installers cover shell/PowerShell/npm/Homebrew/msi with embedded checksum validation and GitHub Artifact Attestations (verifiable via `gh attestation verify <file-path> --repo axodotdev/cargo-dist`). *TC path:* adopt directly; this is the exact mechanism TC's G1 targets.

---

## Category 1: Scoring & Quality

### Q-S1 Conformal prediction beyond TC's corpus
Net-new families TC did not list:
- **Weighted conformal prediction (covariate shift):** reweight calibration nonconformity scores by the likelihood ratio w(x)=dP̃_X/dP_X (Tibshirani et al. 2019, canonical; tutorial at stat.berkeley.edu). Preserves coverage when contributor populations shift.
- **Generalized covariate shift with posterior drift:** Wang & Qiao 2025, AISTATS (PMLR 258:4888–4896) — a weighted conformal classifier leveraging both source and target samples with target-domain coverage guarantee. This is the right model when *both* the trace distribution and the quality-labeling relationship drift.
- **Adaptive / online conformal under drift:** WATCH (arXiv:2505.04608) — weighted-conformal martingales for continuous deployment monitoring; doubly-robust calibration under covariate shift (PMC11398884).
- **Conformal risk control for accept/reject:** Selective Conformal Risk Control (arXiv:2512.12844); "Aligning Model Properties via Conformal Risk Control" (Overman et al.); FDR-controlling conformal calibration for accept/flag decisions (arXiv:2603.00924, controlling expected proportion of accepted-but-incorrect ≤ α).
- **Coverage with ~350 calibration traces:** the finite-sample slack is 1/(M+1) ≈ 0.28% at M=350, consistent with TC's known "overshoot ≤0.33% at n=300." TC's "92/100, 95% coverage" target is achievable, but note the exchangeability caveat below.
- **⚠️ Exchangeability caveat:** all standard coverage guarantees assume exchangeability; when IronClaw ships a new redaction scheme, exchangeability breaks and TC must switch to weighted CP. TC's prior IDs (ToolChain-CRC 2606.18467, Role-Stratified-CRC 2607.24343, PASC 2605.18812) could not be independently resolved within budget — see ledger.

### Q-S2 Incentive-compatible data pricing (non-Shapley)
- TC's known result confirmed: "Do Data Valuations Make Good Data Prices?" (arXiv:2504.05563, verified) proves Leave-One-Out and Data Shapley fail to incentivize truthful reporting; **Myerson payment is buyer-optimal while incentive-compatible and individually rational**, but with both-sided private info the price of anarchy is unbounded.
- Net-new: "Designing DSIC Mechanisms for Data Sharing" (arXiv:2506.05379, verified — this is TC's "Q-MIA") introduces the **Marginal Utility Token (MUT)**: each agent's share ∝ product of verifiable quality qᵢ and marginal utility, provably making withholding/misreporting strictly worse; payments stay within budget via VCG-type budget-feasibility. **This directly validates TC's q = f·g·a formula direction** (quality × marginal-novelty structure).
- Net-new: "Learn then Decide: A Learning Approach for Designing Data Marketplaces" (arXiv:2503.10773) — a hybrid auction-then-posted-price mechanism satisfying IR and IC for impatient buyers; relevant to TC's streaming intake.
- Practical scaling: VCG's harm-computation is the bottleneck; greedy value-density winner determination approximates it (x402-RAM, ResearchGate 396792172). Collusion / Sybil: the "Credibility Trilemma" (arXiv:2605.26604) proves ghost-bid deviations are profitable and undetectable under sealed-bid VCG *and* Myerson, closed only by broadcast commitment — relevant to TC's Sybil concern.
- *TC path:* adopt the MUT structure explicitly; pair with staking / broadcast-commitment for Sybil resistance (see Q-T3).

### Q-S3 Process mining for agent traces
- Net-new representational upgrade: **GRADE** (arXiv:2606.22741) — Graph Representation of LLM Agent Dependency and Execution, distinguishing execution-layer projections (ReAct chains, tool-call trees) from dependency/PROV layers; a better substrate than flat conformance checking for high-variability traces.
- For flexible processes, declarative (DECLARE-style) mining is more appropriate than fitness thresholding; TC's known finding ("fitness thresholding alone too coarse") is corroborated. TC's cited BPM'26 / AgentLTL IDs unverified within budget.

### Q-S4 Causal attribution without re-execution
- Net-new: **GraphTracer** (arXiv:2510.10581, verified but **v2 was withdrawn 2025-12-22** — treat as preprint) constructs Information Dependency Graphs and localizes root causes via information flow rather than temporal order; GraphTracer-8B reports up to **18.18%** higher attribution accuracy on Who&When and 4.8–14.2% downstream gains. Critically, it reports strong "trajectory-only (w/o ground truth)" numbers — i.e., **observational** attribution, exactly TC's offline setting.
- **AgentGraph** (AAAI 2026, ojs.aaai.org/…/42393) — trace-to-graph platform doing causal attribution + perturbation robustness testing on logged traces.
- Content-aware tool-call attack detection via GNN on logged traffic (arXiv:2605.11053) shows execution-graph structure carries discriminative signal without re-execution.

### Q-S5 Joint compression + quality scoring
- TC's known systems (TRACE, ACE, Slipstream) unverified within budget. Cross-domain net-new for information-preserving compression with downstream-fidelity guarantees is covered under **Q-X3 (video summarization)** — keyframe / temporal-coverage methods provide diversity-aware selection with coverage guarantees that transfer to trace compression.

### Q-S6 Quality prediction without ground truth (fixes PR #216)
Strongest net-new cluster (see Exec #8):
- **Judge-Aware Ranking Framework** (arXiv:2601.21817) — judge-aware BTL, jointly estimating latent quality + judge reliability, no reference labels. Key warning: "more data can make evaluation more confidently wrong under misspecified aggregation" — directly relevant to TC pooling multiple scorers.
- **Rogan-Gladen correction** for judge sensitivity/specificity (arXiv:2605.06939) — remains valid under test-set distribution shift when the judge depends only on latent correctness.
- **Reliability without Validity** (arXiv:2606.19544) — report Cohen's / chance-corrected agreement, not raw exact-match, as the headline reliability number.
- Item Response Theory for judge reliability (arXiv:2602.00521, cited in 2606.19544) — a latent-trait alternative to Hui-Walter.
- *TC path:* these are the correct tools to rebuild the bake-off on a ground-truth-free footing.

### Q-S7 Contrastive learning for trace embeddings
- Net-new: **AST-FIM structure-aware pretraining** (arXiv:2506.00204) gives execution/structure-aware code representations; content-aware tool-call features (arXiv:2605.11053) demonstrate semantic embedding of tool-call graphs. For hard-negative recipes (superficially similar, functionally different), pair AST-level positives with execution-outcome-flipped negatives. TC's cited hard-negative ID 2509.24291 unverified within budget.

### Q-S8 Concept drift detection for trace populations
- **WATCH** (arXiv:2505.04608) — weighted-conformal martingales as an automatic drift/recalibration trigger in embedding space. Pairs naturally with weighted CP (Q-S1) so that a detected drift *both* raises an alert and reweights calibration. Classical PSI / KS / Wasserstein remain valid on embedding summaries.

---

## Category 2: Integrations & Ecosystem

### Q-I1 OTel GenAI convention stability (verified Aug 2026 state)
- As of July 17, 2026, **no GenAI-specific span, event, metric, or attribute in the dedicated repository is marked Stable — the GenAI conventions remain "Development"** (John Hodge, July 2026 analysis). On June 12, 2026 (semantic-conventions v1.42.0) all GenAI conventions were deprecated in the main repo and moved to the dedicated `open-telemetry/semantic-conventions-genai` repo, which **has no tagged release and no finalized schema URL**; v1.43.0 (July 3) shipped none.
- The dangerous rename is confirmed: `gen_ai.system` → `gen_ai.provider.name` (opentelemetry.io registry).
- Migration / pinning guidance: use the `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` env var for dual-emission during transition; pin to main-repo **v1.42.0** (the last versioned cut) since the new repo has no release to pin against. Datadog shipped native support in OTel SDK/Collector **v1.37 on December 1, 2026**, auto-mapping `gen_ai.request.model`, `gen_ai.usage.input_tokens`, `gen_ai.provider.name`, and `gen_ai.operation.name`. OpenInference (Arize/Phoenix) remains a parallel convention set.
- *TC path:* ingest against v1.42.0 attribute names, enable the opt-in dual-emission env var, and build a `system`↔`provider.name` alias shim now.

### Q-I2 Claude Code hook integration
- Confirmed constraint: SessionEnd hooks have a short default timeout, so blocking export is unsafe. *TC path:* the hook should only enqueue to a local background daemon (fire-and-forget) that batches and exports asynchronously — consistent with OTel's guidance that "OpenTelemetry is designed with asynchronous batch processing, so the impact on main application performance is less than 1%." (Third-party opentelemetry-hooks / claude_telemetry patterns corroborate.)

### Q-I3 Agent Skills security & trust registries (TC's thesis validated — Exec #9)
- ClawHavoc scale (note conflicting counts across sources): Koi Security flagged **341 malicious of 2,857** skills (335 from one operation) in Feb 2026 (per arXiv:2604.06550); Scandar reports **1,184 skills / ~300K users**; clawbot.blog says 1,200; Snyk's audit of 3,984 skills flagged 1,467 (36.82%) for at least one issue. Report the range, not a single number.
- **Correction to TC's corpus:** SkillSieve (arXiv:2604.06550) achieves **F1 = 0.800** (precision 0.752, recall 0.854) on a 400-skill benchmark drawn from 49,592 ClawHub skills at **$0.006/skill** — *not* the F1=0.920 TC recorded. SkillFortify (formal verification) reports ~96.95% F1 with zero false positives but only on executable code, not SKILL.md prose.
- Runtime layer is the demonstrated gap: SkillVetBench (arXiv:2606.00925) — static-only baselines miss up to 89%; MalSkillBench (arXiv:2606.07131) and ClawGuard (arXiv:2604.11790) require execution traces.
- *TC path:* offer a "verified skills backed by execution provenance" tier — this is TC's clearest differentiated wedge.

### Q-I4 Cross-agent session formats
- Confirmed within budget: VS Code Copilot emits OTel GenAI traces/metrics/events per agent interaction (opentelemetry.io blog, May 2026). Full per-tool matrix (local files vs API for Claude Code, Codex, Cursor, Copilot) was not fully resolved within budget — flagged as a remaining gap; recommend a short targeted follow-up.

### Q-I5 A2A observability & multi-agent tracing
- Provenance substrate: "From Agent Traces to Trust" (arXiv:2606.04990) maps evidence/execution units to typed provenance relations and grounds them in W3C PROV-DM + OpenTelemetry; GRADE (arXiv:2606.22741) provides the dependency-graph correlation beyond W3C traceparent. *TC path:* ingest cross-agent delegation edges as PROV relations keyed to trace IDs.

---

## Category 3: Growth & Distribution (weighted heavily)

### Q-G1 CLI install / auto-update
- **cargo-dist + axoupdater** (github.com/axodotdev/cargo-dist): `install-updater = true` ships a standalone updater ("users who install your software via the shell or PowerShell installers will receive a standalone updater program alongside your program itself"); installers = shell/PowerShell/npm/Homebrew/msi; embedded checksum validation; GitHub Artifact Attestations verifiable via `gh attestation verify`; cross-compile via cargo-zigbuild / cargo-xwin. This is a drop-in for TC's <90s-install / zero-friction-update goal.
- Homebrew tap auto-update via a small `update-homebrew-formula.sh` committed to the tap repo (Ivan Carvalho, ivaniscoding.github.io).

### Q-G2 / Q-G3 / Q-G7 Failure-data communities & viral artifacts
- Search budget was exhausted before I could fully resolve the community-channel and viral-artifact threads; the arXiv / tooling evidence above is solid but the growth-channel specifics (r/ClaudeAI dynamics, Error-Hub virality, opt-in rates) are the **thinnest-sourced area** and should be the next research round's focus. Working hypothesis from adjacent evidence: TC's viral artifact equivalent is a **signed "failure bundle" / skill-provenance link** (analogous to Sentry error pages / PostHog replay links) whose credibility comes from TEE attestation — but this is a hypothesis, not a sourced finding.

### Q-G4 Background daemon opt-in
- Corroborated principle (not newly sourced this round): async batch export keeps overhead <1% (OTel). TC's known Go-telemetry 10–20% opt-in benchmark stands; specific consent-UX data not re-verified this round.

### Q-G6 Synthetic trajectory seeding (verified)
- **Open-SWE-Traces** (arXiv:2606.16038) — **207,489** trajectories, 9 languages (Python, Go, TS, JS, Rust, Java, PHP, C, C++), sourced from 20,000 real PRs via OpenHands/SWE-agent, synthesized with Minimax-M2.5 (thinking) + Qwen3.5-122B (non-thinking), permissively licensed, PII-filtered; hosted at huggingface.co/datasets/nvidia/Open-SWE-Traces. Best fine-tuned model: 61.7% SWE-bench Verified. **VERIFIED and directly usable for corpus seeding.**
- Additional: Nebius SWE-rebench-openhands-trajectories = 67,074 open trajectories (3× more successful attempts than alternatives).
- TC's "GenEnv 2512.19682" could not be resolved within budget — flagged UNVERIFIED.

---

## Category 4: Strategy & Market

### Q-M1 GPAI obligations live Aug 2, 2026 (verified)
- GPAI obligations have applied since Aug 2, 2025; **the Commission's enforcement powers switch on Aug 2, 2026** (requests for information, model evaluations, compliance / recall measures, fines). Per the European Commission's official "The enforcement framework of the AI Act": "Other breaches, including of the obligations for GPAI models, may result in fines of up to **€15 million or 3% of total worldwide annual turnover**, whichever is higher… Infringements involving prohibited AI practices are subject to the highest penalties, of up to **€35 million or 7%**."
- The final GPAI Code of Practice (published July 10, 2025, per Latham & Watkins) has three chapters: **(1) Transparency, (2) Copyright, and (3) Safety and Security**. The Safety & Security chapter applies only to systemic-risk models above the **10^25 FLOP threshold — currently a small group of ~5–15 companies worldwide** (artificialintelligenceact.eu). Providers must also furnish deployers an information package (capabilities/limitations, safe-deployment instructions, known biases, use restrictions).
- Providers of models on the market before Aug 2, 2025 have until **Aug 2, 2027** to comply.
- *TC path:* TC's TEE-based redaction + provenance + quality scoring maps to the transparency-template and training-data-summary obligations; position as GPAI-compliance infrastructure. (TC's "Digital Omnibus defers Article 12 to Dec 2, 2027" claim was not independently confirmed this round — verify before citing.)

### Q-M5 Privacy-preserving data-sharing market (verified)
- **Vana**: Data Liquidity Pools run in TEEs (Satya Network / TEE Pool); the VRC-14 upgrade **replaced direct token emissions with liquidity / market-driven rewards** (DLPRewards); reward weights are Token Trading Volume 30% / Unique Contributors 20% / Data Access Fees 50%. Lesson: emissions-only incentives were insufficient; Vana pivoted to usage-linked rewards and Data Validator Staking (uptime/security/liquidity).
- **Ocean Protocol**: AMM / datatoken model; 2026 guides note liquidity is "still building" and regulatory clarity is lacking — a cautionary GTM lesson that pure-marketplace liquidity is hard to bootstrap.
- *TC path:* mirror Vana's shift — tie NEAR credits to *downstream usage / access fees* (RAG queries, verified-skill consumption), not just submission volume, to avoid the emissions trap.

---

## Category 5: New Technical Frontiers

- **Q-T3 Provenance / anti-Sybil (verified, Exec #10):** VET (arXiv:2512.15892) — Web Proofs <3× overhead, TEE Proxy for public APIs; Proof-of-Execution (arXiv:2607.05397) — EACs at ~2.7 ms / 4.4% overhead, ~1.1 KB per 8-event trace, composable with zkVMs; SVIP (inference-level provenance, cited in arXiv:2604.23280); "From Logic Monopoly to Social Contract" (arXiv:2603.25100) surveys TEE hardware-root-of-trust for agent economies. *TC path:* TEE-attest the capture step at ingestion (cheapest credible anti-fabrication), statistical fabrication detection as a second layer.
- **Q-T5 Skill extraction:** search budget exhausted before resolving TC's RHO / Trace2Skill / SkillAudit IDs independently; SkillVetBench / MalSkillBench (verified) demonstrate trace→behavior extraction pipelines that double as skill-behavior extractors. Remaining verification is a next-round item.
- **Q-T1 (streaming) / Q-T2 (multi-modal) / Q-T4 (idle-time pre-compute):** not independently sourced this round (budget exhausted); flagged for next round.

---

## Category 6: Under-Explored

- **Q-U2 GNNs for tool-call sequences (verified):** GraphTracer / GRADE / AgentGraph (above) + content-aware tool-call GNN (arXiv:2605.11053) capture branching/loops invisible to sequential HDC fingerprints. *TC path:* add a GNN structural embedding alongside HDC.
- **Q-U3 Reward modeling from pairwise preferences:** the judge-aware BTL work (arXiv:2601.21817) is the strongest sourced entry — pairwise "which trace is more useful?" jointly with judge reliability. TC's DynaCF / ConsistRM IDs unverified within budget.
- **Q-U5 RAG from trace corpora (TC's potential killer feature — assessment):** Strongly feasible and well-precedented. "Learning to Retrieve from Agent Trajectories" (arXiv:2604.04949), AgentIR (reasoning-aware retrieval), "Retrieval-Augmented LLM Agents: Learning to Learn from Experience" (arXiv:2603.18272, retrieval of *experience* for policy generalization), and "Towards Retrieving Interaction Spaces for Agentic Search" (arXiv:2606.06880) all validate retrieving prior trajectories so an agent can query "show me traces that solved similar problems." Caveat (arXiv:2602.02007): naive top-k over trajectory memory collapses into redundant dense regions — TC needs diversity-aware retrieval (decoupling + aggregation), not vanilla cosine top-k. **Verdict: this is a credible killer feature; build it on diversity-aware retrieval, not plain HNSW top-k.**
- **Q-U6 Active learning (verified + correction):** the "93% performance at ~6% annotation cost" number is from **arXiv:2502.16892** ("Applying LLMs to Active Learning," Zhang et al., IJIS 2025), **not** the survey arXiv:2502.11767. Corroborating: ActPRM (arXiv:2504.10559) reaches SOTA 75.0% ProcessBench at 6% annotation cost. *TC path:* uncertainty-sampled active labeling can hit near-full quality at ~6% of TC's annotation budget — critical given only 3 contributors.
- **Q-U1 (federated scorer updates) / Q-U4 (online anomaly detection):** not independently sourced this round; the federated-learning survey 2504.17703 and CALIBURN 2605.24696 remain UNVERIFIED.

---

## Category 7: Cross-Domain Inspiration
- **Q-X4 Secure aggregation / MPC:** "Agent-OSI" (arXiv:2602.13795) layer L5 standardizes a provenance interface admitting TEE attestations OR ZK proofs OR signed logs — a lighter-weight menu than full-TEE processing for multi-contributor aggregate statistics.
- **Q-X1 / X2 / X3 / X5 / X6** were not independently sourced this round (budget exhausted); flagged for next round. Video-summarization coverage-guarantee methods (X3) remain the most promising unexplored transfer for information-preserving trace compression (Q-S5).

---

## VERIFICATION LEDGER

| arXiv ID (as briefed) | Claimed topic | Status |
|---|---|---|
| 1910.14659 | Masked LM Scoring (PLL) | **Verified** |
| 2305.10588 | Kauf & Ivanova, better MLM scoring | **Verified** (net-new) |
| 2302.08091 | Clinical LMs / de-id tokens | **Verified** (net-new) |
| 2506.00204 | AST-FIM infilling | **Verified** (net-new) |
| 2502.06901 | MARIA masked infilling | **Verified** (net-new) |
| 2511.15364 | Anonymization and Information Loss | **Verified** |
| 2603.29497 | (real title) Distilling Human-Aligned Privacy Sensitivity | **Verified — title differs from brief; it's a privacy classifier, Δ=−0.31 result confirmed** |
| 2603.20208 | RedacBench | **Verified** |
| 2407.11770 | RUPTA utility-preserving anonymization | **Verified** |
| 2606.21255 | SCOPE conformal OOD gate | **Verified** (net-new) |
| 2607.04430 | UCB-based abstention risk calibration | **Verified** (net-new) |
| 2603.01971 | LOCUS loss-quantile score | **Verified** (net-new) |
| 2402.12997 | Abstention rate calibration | **Verified** (net-new) |
| 2505.04608 | WATCH weighted-conformal martingales | **Verified** (net-new) |
| 2512.12844 | Selective Conformal Risk Control | **Verified** (net-new) |
| 2603.00924 | FDR-controlling conformal med-entity | **Verified** (net-new) |
| 2504.05563 | Do Data Valuations Make Good Prices | **Verified** |
| 2506.05379 | DSIC / Marginal Utility Token (Q-MIA) | **Verified** |
| 2503.10773 | Learn then Decide data marketplaces | **Verified** (net-new) |
| 2605.26604 | Credibility Trilemma | **Verified** (net-new) |
| 2601.21817 | Judge-Aware Ranking (no ground truth) | **Verified** (net-new) |
| 2605.06939 | Bias/Uncertainty in LLM-as-Judge (Rogan-Gladen) | **Verified** (net-new) |
| 2606.19544 | Reliability without Validity | **Verified** (net-new) |
| 2510.10581 | GraphTracer | **Verified — but v2 withdrawn 2025-12-22; treat as preprint** |
| 2606.22741 | GRADE agent dependency/execution graph | **Verified** (net-new) |
| 2605.11053 | Content-aware tool-call GNN | **Verified** (net-new) |
| 2606.04990 | Evidence tracing / execution provenance | **Verified** |
| 2604.06550 | SkillSieve | **Verified — F1=0.800 (NOT 0.920 as TC recorded)** |
| 2606.00925 | SkillVetBench | **Verified** (net-new) |
| 2606.07131 | MalSkillBench | **Verified** (net-new) |
| 2604.11790 | ClawGuard | **Verified** (net-new) |
| 2512.15892 | VET verifiable execution traces | **Verified** |
| 2607.05397 | Proof of Execution | **Verified** (net-new) |
| 2604.23280 | AI Identity standards / SVIP | **Verified** (net-new) |
| 2603.25100 | Autonomous agent economies / TEE | **Verified** (net-new) |
| 2602.13795 | Agent-OSI layered stack | **Verified** (net-new) |
| 2606.16038 | Open-SWE-Traces | **Verified — 207,489 trajectories, 9 languages** |
| 2604.04949 | Learning to Retrieve from Agent Trajectories | **Verified** (net-new) |
| 2603.18272 | Retrieval-Augmented LLM Agents | **Verified** (net-new) |
| 2606.06880 | Retrieving Interaction Spaces | **Verified** (net-new) |
| 2602.02007 | Beyond RAG for Agent Memory | **Verified** (net-new) |
| 2502.11767 | LLM-based Active Learning **survey** | **Verified — but the 93%/6% figure is NOT from here** |
| 2502.16892 | Applying LLMs to Active Learning | **Verified — THIS is the source of 93%/6%** |
| 2504.10559 | ActPRM active PRM training | **Verified** (net-new) |
| 2508.05545 | PRvL PII redaction eval | **Verified** (net-new, via subagent) |
| DOI 10.1186/s12911-024-02546-8 | Vakili pseudonymization | **Verified** (net-new) |
| 2606.18467 / 2607.24343 / 2605.18812 | ToolChain-CRC / Role-Strat-CRC / PASC | **UNVERIFIED — not resolved within budget** |
| 2605.07663 / 2506.12619 | Sybil unfair payoff / semivalue gameable | **UNVERIFIED** |
| 2606.20669 / 2607.02599 | Agent Behavior Mining / AgentLTL | **UNVERIFIED** |
| 2606.08275 / 2605.25338 / 2509.03312 / 2606.14805 | CAR / CausalFlow / AgenTracer / Zero-Replay | **UNVERIFIED** |
| 2606.00611 / 2606.31564 / 2605.08580 / 2607.05378 / 2606.22528 | TRACE / ACE / Slipstream / CompactionRL / Governance Decay | **UNVERIFIED** |
| 2606.05922 / 2603.25158 / 2606.14239 | RHO / Trace2Skill / SkillAudit | **UNVERIFIED** |
| 2504.17703 / 2606.09043 / 2604.07484 | Fed-learning survey / DynaCF / ConsistRM | **UNVERIFIED** |
| 2605.24696 / 2506.15655 / 2602.14102 | CALIBURN / cAST / DALL | **UNVERIFIED** |
| 2512.19682 | GenEnv | **UNVERIFIED** |
| 2509.24291 | hard-negative mining contrastive | **UNVERIFIED** |

*Note on numbering: the brief's "current date" is Aug 12, 2026, and IDs prefixed 2602–2607 correspond to Feb–Jul 2026. Verified entries were confirmed by fetching arXiv/HF/ACL pages; UNVERIFIED entries simply were not reachable within the search budget and should not be treated as fabricated — but must be independently confirmed before citation.*

---

## Prioritized Implementation Roadmap

**Immediate (days — unblocks #210 and #219):**
1. **Fix #219 scoring:** (a) exclude placeholder positions from the perplexity average; (b) mask multi-subword placeholders as a single unit (Kauf & Ivanova PLL-word-l2r); (c) add TC's placeholder set as single in-vocab tokens (Lehman et al.). Re-run the "0/99" corpus and measure the IronClaw-vs-others score gap before/after.
2. **Fix #210 gate:** replace absolute thresholds with a **quantile gate** — accept scores above the empirical quantile matching a target acceptance rate on the calibration set (SCOPE Conformal Linear Gate). This mathematically cannot yield "0 accepted."
3. **Seed the corpus** from Open-SWE-Traces (207,489 traj., incl. Rust) to populate the HNSW index and calibration set.
4. **Adopt cargo-dist + axoupdater** for <90s install and self-update.

**1–3 months:**
5. Rebuild the bake-off (PR #216 fix) on the **judge-aware BTL + Rogan-Gladen** ground-truth-free footing; report chance-corrected agreement.
6. Add an **infilling-coherence sub-score** (AST-FIM / MARIA) as a redaction-invariant complement to perplexity.
7. Switch to **weighted conformal prediction** so new IronClaw / model releases trigger reweighting, not recollection; wire **WATCH** as the drift trigger.
8. Formalize credits as a **Marginal Utility Token** (arXiv:2506.05379) tied to *downstream usage* (Vana lesson), with staking / broadcast-commitment for Sybil resistance.
9. TEE-attest the **capture step** at ingestion (Proof-of-Execution style, ~ms overhead) as anti-fabrication.

**3–6 months:**
10. Ship the **"verified skills backed by execution provenance"** tier — TC's differentiated wedge validated by SkillVetBench / MalSkillBench.
11. Build the **RAG-over-traces** killer feature on diversity-aware retrieval (not plain HNSW top-k).
12. Add a **GNN structural embedding** for branching / looping traces alongside HDC.
13. Position TC explicitly as **GPAI-compliance infrastructure** (transparency template + training-data summary) ahead of Aug 2, 2026 enforcement.
14. Introduce **active learning** (uncertainty sampling) to reach ~93% quality at ~6% annotation cost given only 3 contributors.

**Benchmarks that would change these recommendations:** if post-fix acceptance rate cannot be stabilized within ±5% of target on held-out weeks → exchangeability is broken, escalate to weighted / online CP. If the IronClaw score gap persists after placeholder-exclusion → the penalty is semantic (information loss), not tokenization, and TC must move to surrogate substitution. If judge-aware BTL inter-scorer agreement (chance-corrected) stays < 0.4 → scorers are measuring different constructs and the ensemble should be split, not pooled.

---

## Caveats on Maturity, Replication & Model Access
- **Preprint / withdrawal risk:** GraphTracer's v2 was withdrawn; many net-new results (SkillVetBench, MalSkillBench, Proof-of-Execution, Judge-Aware Ranking) are 2026 preprints without peer review — replicate before shipping.
- **Corpus errors corrected:** SkillSieve F1 is 0.800 not 0.920; the 93%/6% active-learning figure is 2502.16892 not 2502.11767; these were factual errors in TC's prior corpus.
- **Unverified IDs:** a substantial block of IDs TC "believes exist" (conformal CRC variants, compression systems, skill-extraction systems, federated-learning survey) could not be reached within the search budget. They are flagged UNVERIFIED, not fabricated — confirm each before citation.
- **Model-access constraints:** placeholder-vocabulary extension requires either fine-tuning the Qwen scorer's tokenizer or a scoring-only wrapper; infilling-coherence scoring requires a FIM-capable model in the TEE. Both add inference cost to TC's single-VM pilot.
- **The "no dedicated paper" gaps:** per-document baseline subtraction and z-scoring perplexity against a redaction-matched reference have no primary source — TC would be building novel (publishable) methodology, which is also an NLnet-grant angle.
- **Thinnest-sourced area:** Category 3 growth channels (G2/G3/G5/G7) and Q-I4 session-format matrix were truncated by budget exhaustion and should headline the next research round.