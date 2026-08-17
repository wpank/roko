# TraceCommons Round-4 Deep Research Sweep: Net-New, High-Impact Findings

## TL;DR
- **Two decision-blockers are now resolvable with published evidence.** (A1) Intel TDX attestation binds to the boot-time measurement chain (MRTD + RTMR SHA384 extends), NOT to internal pipeline step-ordering — so reordering "score-then-redact" inside an already-attested enclave needs **no re-attestation**, provided both stages live inside the measured binary. This is a **go** for Issue #219. (A3) The redaction→perplexity penalty is real and measurable in peer-reviewed work, and random/uninformed redaction can perversely *raise* privacy scores — validating the problem but showing TC's internal "8–12 nats/placeholder" estimate is unsupported and should be replaced with TC's own measurements.
- **Three exponential-efficiency wins are credible and net-new:** VS-Graph hyperdimensional computing (arXiv:2512.03394, verified) delivers up to **450× training speedup** for graph classification at D=128 for trajectory-graph novelty scoring; cardinality-constrained BQP diversity retrieval (arXiv:2604.02554) gives **2.4–22.9× speedup over MMR** with sublinear-in-k scaling; and Qwen3-Coder's native FIM (verified) means TC's redaction-invariant infilling sub-score needs **no second model in-enclave**.
- **The biggest structural risk is TC's own citation base.** Of the flagged IDs: one is a *different paper* (2509.24291 → GIRCSE, not hard-negative mining), one is *withdrawn* (2504.17703), one has a *wrong description* (2603.01971 "LOCUS" is a per-input reliability wrapper, not group-conditional conformal), and TC's "xMemory" (2602.02007) is actually titled "Beyond RAG for Agent Memory." Full ledger below.

---

## RANKED EXECUTIVE SUMMARY — 14 highest-impact net-new findings

1. **[DECISION-UNBLOCKING — A1] TDX attestation scope permits pipeline reordering without re-attestation.** RTMRs measure the boot chain (firmware, kernel, initrd, config) via `RTMR[i] = SHA384(RTMR[i] || value)`; they do NOT record intra-application control-flow order. If both "score" and "redact" live in the same measured enclave image, reordering them does not change the quote. **Go.** Caveat: the reordered path must be inside the measured TCB and determinism must hold (see #9).

2. **[DECISION-UNBLOCKING — A2] Qwen3-Coder has native FIM; no second model needed.** Qwen3-Coder ships `<|fim_prefix|>/<|fim_suffix|>/<|fim_middle|>` tokens trained for infilling; Qwen3-Coder-Next (arXiv:2603.00729, verified) adds chat-FIM and search-and-replace FIM. TC can compute an infilling-coherence sub-score from the *same* enclave model, avoiding the VRAM doubling that MARIA (2502.06901) or AST-FIM (2506.00204) would require.

3. **[DECISION-UNBLOCKING — A3] Redaction→perplexity penalty is real and measurable; replace the internal estimate.** Peer-reviewed anchor: arXiv:2309.08628 ("Recovering from Privacy-Preserving Masking with LLMs," Vats et al., ICASSP 2024, pp. 10771–10775) shows allowList masking pushing perplexity from oracle 37.3 → 120.1, while milder entityTagger masking barely moves it (→41.7) — the penalty depends heavily on masking scheme and volume. arXiv:2603.29497 (verified, Δ=−0.31) shows *random* redaction *increases* privacy scores because uninformed masking disrupts coherence. TC's "15–30% shift / 8–12 nats per placeholder" is unsupported; run TC's own within-enclave A/B.

4. **[EXPONENTIAL — B3] VS-Graph HDC: up to 450× faster graph classification, verified.** arXiv:2512.03394 (Poursiami, Snyder, Cong, Potok, Parsa; ORNL/GMU; 3 Dec 2025): VS-Graph "matches or exceeds the performance of the GNN baselines on several datasets while accelerating the training by a factor of up to 450x," and outperforms the prior HDC baseline by 4–5% on MUTAG/DD, robust at D=128. For trajectory-graph novelty/dedup this is a genuine exponential compute win vs GNN scoring and is CPU/edge-friendly (fits the single-VM constraint). HyperGraphX (144.5× transductive) and an FPGA HDC paper (2512.08089) corroborate the paradigm.

5. **[EXPONENTIAL — B10] Cardinality-constrained BQP diversity retrieval beats MMR 2.4–22.9×.** arXiv:2604.02554 shows MMR has NO approximation guarantee (non-monotone submodular) and runs O(knd); their BQP method scales sublinearly in k and is 2.4–22.9× faster at the practically-relevant θ≥0.5. DPP is *slower* than MMR and its log-det term is unbounded/hard to tune. Net-new vs TC's untested MMR λ=0.7 prescription.

6. **[DECISION-UNBLOCKING — A8] Judge-aware BTL identifiability confirmed — but TC's correlated-judges problem is a genuine violation.** arXiv:2601.21817 (Xu/Tan/Wu/Zhou, ICML 2026, verified) proves identifiability "up to natural normalizations" plus consistency/asymptotic normality; the follow-on 2605.05073 requires connected per-judge comparison graphs. BUT TC's PerplexityScorer and TokenRarityScorer derive from the SAME forward pass — violating conditional independence in Dawid-Skene/latent-class aggregation, which makes CIs too narrow and biases consensus (BT-σ, Qian et al., via 2605.09702). Fix: collapse the two into one "forward-pass judge" now; model the junction-tree covariance (Snorkel-style) as the endgame.

7. **[MODULARITY/INTEROP — C1] The interop substrate has consolidated: OTel GenAI + OpenInference + W3C PROV, governed by the Linux Foundation AAIF.** gen_ai.* moved to a dedicated semantic-conventions-genai repo (v1.42.0, 12 Jun 2026) but remains "Development" (not Stable). W3C PROV-DM is the provenance backbone (survey 2606.04990). To be a hub not a silo, TC should ingest OTel + OpenInference and export W3C PROV-mapped lineage.

8. **[DECISION-UNBLOCKING — A10] OTEL_SEMCONV_STABILITY_OPT_IN support is uneven — pin versions.** Datadog native support landed in OTel v1.37; OpenAI Python instrumentation is most mature; Anthropic/Cohere/Bedrock via community libs. The `gen_ai_latest_experimental` flag is honored inconsistently ("not every framework honors the variable identically" — John Hodge, Jul 2026). TC's dual-emission strategy is correct but must be validated per-SDK against a real exported span.

9. **[DECISION-UNBLOCKING — A9] Deterministic in-enclave inference is achievable but costly; ONNX Runtime in TEE is documented.** A Python-in-SGX MLaaS system treating models-as-data exists (MDPI 2624-800X/6/1/23); batch-invariant kernels give bit-identical outputs at ~34–61% throughput cost (Thinking Machines/SGLang, via 2606.03019). ONNX Runtime does NOT expose deterministic primitive selection (2501.05867). For attestation to be meaningful, scoring must be deterministic; seed HNSW RNG and pin thread count.

10. **[EXPONENTIAL/MECHANISM — B4] Shapley-based multi-contributor apportionment is mature; use Owen/Banzhaf sampling to kill the 2^N cost.** For orchestrator+sub-agent (A2A) credit, Shapley is the principled split; Owen Sampling (arXiv:2508.21261) and Data-Banzhaf accelerate estimation. But contribution metrics are volatile round-to-round (arXiv:2405.08044) — a real risk at N=3.

11. **[MECHANISM — B5] At N=3, staking alone is insufficient; false-name-resistant semivalues exist but don't solve genuine collusion.** arXiv:2605.07663 ("Quotient Semivalues for False-Name-Resistant Data Attribution," verified) targets Sybil/false-name manipulation; arXiv:2506.12619 (verified) shows standard semivalues are "gameable." Honest recommendation: no payment mechanism is robustly collusion-proof when all three participants can collude — quality gates + provenance attestation must carry the load until N grows.

12. **[UX — C4/C5] LLM pre-labeling induces measurable anchoring bias; sequence-annotation tooling is thin.** TC should withhold LLM suggestions on a calibration subset to measure the reliability penalty. Real-time partial-trace quality feedback during a session is offered by no incumbent — a UX wedge.

13. **[MODULARITY — C8] The Snorkel label model is ~200–500 LOC reimplementable in Rust.** Core algorithm: build the LF matrix Λ, compute the inverse generalized covariance of the LF-dependency junction tree, matrix-complete to recover P(LF|Y), emit probabilistic labels for a noise-aware loss (Ratner et al. 2018; confirmed 2111.14282, 2511.13891). No fundamental Python dependency — unblocks TC's Rust-native goal.

14. **[REGULATORY — D1/D2] Both regulatory anchors confirmed with primary-source-grade citations.** (D1) The GPAI Training Data Summary Template was published 24 Jul 2025 under Art. 53(1)(d), Regulation (EU) 2024/1689. (D2) The Digital Omnibus is now law: **Regulation (EU) 2026/1744**, published in the OJ 24 Jul 2026, in force 27 Jul 2026, deferring Annex III standalone high-risk obligations to **2 December 2027** and Annex I embedded to 2 August 2028.

---

## PART A — DECISION-BLOCKING QUESTIONS

### A1. TEE attestation scope for pipeline reordering — **GO**
Intel TDX records state into MRTD (build-time) and four RTMRs (runtime boot-chain: kernel, initrd, cmdline, ACPI, config) via `RTMR[i] = SHA384(RTMR[i] || value)` (Linux kernel TDX docs; Intel TDX Module Base Spec 2025; enclaive.cloud). Attestation (TDREPORT → Quote via the SGX Quoting Enclave) binds to *those measurements plus the enclave image*, not to the internal ordering of function calls within the application binary. **Implication:** reordering "score raw text → then redact" inside the same measured enclave binary does not change the quote and requires no re-attestation. If you split the stages across processes, each new binary/config extends an RTMR and DOES change the measurement. Expected impact: unblocks Issue #219 — score perplexity on raw text pre-redaction, emit only the scalar + redacted text. Caveat: raw text now transits the enclave pre-redaction, so your threat model must trust the TDX boundary for that window (it already does for scoring).

### A2. Qwen FIM capability — **native, no second model**
Qwen3-Coder was trained for FIM with sentinel tokens (Qwen3-Coder GitHub; DeepWiki FIM benchmarks over HumanEval-Infilling/CrossCodeEval). Qwen3-Coder-Next (arXiv:2603.00729, verified) adds chat-FIM and search-and-replace FIM. Base Qwen3 (non-coder) FIM support is ambiguous (QwenLM/Qwen3 Discussion #1277 unresolved) — confirm the exact production checkpoint. Net-new: TC believed it might need MARIA (2502.06901, verified as "MARIA: Masked and Autoregressive Infilling Architecture"; note "7B" is not stated in the abstract) or AST-FIM (2506.00204, verified as "Structure-Aware Fill-in-the-Middle Pretraining for Code," Gong et al., 30 May 2025). If the production scorer is Qwen3-Coder-family, the smallest FIM-capable co-resident model is *itself* — zero extra VRAM. If it is base Qwen3, the cheapest add is a small dedicated FIM head; HLP (arXiv:2410.03103) shows FIM planning gains at zero inference overhead.

### A3. Quantifying the redaction penalty — **replace internal estimate with measurements**
Empirical anchors:
- **arXiv:2309.08628** ("Recovering from Privacy-Preserving Masking with LLMs," Vats et al., ICASSP 2024, pp. 10771–10775): allowList masking pushes PPL from oracle 37.3 → baseline 120.1; entityTagger masking is far milder (→41.7). The penalty depends heavily on masking scheme and how much is masked. *Use this peer-reviewed citation as the primary anchor.*
- **arXiv:2603.29497** (verified, Δ=−0.31): random 30% `[REDACTED]` *raises* the privacy score because uninformed redaction disrupts coherence while preserving identifying content — a privacy-risk metric, not perplexity, exactly as TC flagged.
- **arXiv:2408.08930** (DePrompt) formalizes readability loss via PPL before/after desensitization.
- A widely-cited secondary (Firstsource/ClearView blog) reports masking raising PPL 1.16 → 2.83 and DP-SGD (ε=1) → 4.87 on OpenLlama-3B; **treat as unconfirmed** — prefer the ICASSP paper.

On subword fragmentation: the mechanism (a placeholder fragmenting into many subword tokens raises local surprisal) is consistent with the entityTagger-vs-allowList gap, but no paper directly regresses penalty on subword count — **an open empirical question TC is well-positioned to answer.** Recommendation: run TC's own within-enclave A/B (raw vs redacted PPL on the 352 traces) and publish it; "8–12 nats/placeholder" is currently unsupported.

### A4. Typed placeholder vocabulary extension — **mean-init is the floor, not the ceiling**
Never leave new special-token embeddings randomly initialized (catastrophic-forgetting risk at embedding + LM-head). Standard: initialize as the mean of constituent/semantically-similar token embeddings (confirmed across 2508.15807, 2509.26124, 2503.19693, RedWhale 2408.11294). Advanced: FVT/Fast Vocabulary Transfer (2402.01035), FOCUS, OFA, HyperOFA, TokAlign. Use PEFT/LoRA with `modules_to_save` for embed+head. **Gap TC correctly identified:** clinical de-id tags are semantically meaningful; TC's placeholders mark *absence*. Verification: 2604.02324 = "Grounded Token Initialization (GTI)" (matches TC's "grounded vocab init" belief); 2604.16656 = "Defragmenting Language Models" (vocab-expansion/interpretability — related but NOT the same "grounded init" method). Lehman et al. 2302.08091 remains valid prior. Implementation: add ~10–15 tokens, mean-init from bracket/tag tokens, LoRA-tune with consistent placeholder usage, evaluate on MMLU/code for regression.

### A5. Surrogate generation for CODE entities — **transfer is unproven; build it**
Clinical/text surrogate generators are mature and open-source: Azure Health Data Services de-id (TAG/REDACT/SURROGATE, 27 entities incl. 18 HIPAA); JULIELab ClinicalSurrogateGeneration; Eder et al. SurrogateGeneration (German email); nedap/deidentify (Stubbs 2015 method). SurrogateShield (94.85% clinical BERTScore) and Wu et al. (2511.15364, verified — anonymization info-loss, earnings-call domain; the specific "20–67%" figure is not confirmable from the abstract) are TC's known anchors. **Critical gap:** none target code entities (function/variable names, API endpoints, file paths). Code surrogate substitution must preserve *referential consistency* (same variable → same surrogate across the trace) and *syntactic validity* — materially harder than name-swapping. Recommendation: build a code-aware surrogate generator using AST-anchored consistent renaming (cAST/AST-FIM machinery, 2506.15655/2506.00204, both verified); treat as net-new IP, not a port.

### A6. Conformal prediction at n=100–500, heterogeneous — **use SSBC; expect wide coverage bands**
At small n, split-conformal coverage is *marginally* valid but has a broad distribution — realized coverage can land far below nominal (arXiv:2303.02770 gives the exact Beta-Binomial law; 2512.04566 shows visibly wider bands at n=100 vs n=1000). **Small Sample Beta Correction (SSBC)** (arXiv:2509.15349) is the recommended plug-in: it shifts the significance level using the exact finite-sample distribution to guarantee ≥target coverage with user-defined probability; validated at n=50 and n=100, effective with calibration sets as small as 47 pixels. Without correction, observed violation rates hit ~40% at nominal 90%. When exchangeability breaks (different agent families, redaction levels), use nonexchangeable conformal (Barber et al., *Annals of Statistics* 2023) to quantify and reweight. SSBC is the concrete tool for TC's ~352-trace scale.

### A7. Group-conditional conformal with tiny subgroups — **borrow strength; don't calibrate separately below ~50**
TC's LOCUS belief is a **description mismatch**: arXiv:2603.01971 "Locus" is a per-input loss-scale reliability wrapper for regression, NOT group-conditional conformal. Correct anchor: Wang & Qiao 2025 (AISTATS PMLR 258:4888–4896). Principle: separate per-subgroup calibration needs each subgroup to individually satisfy the 1/(n+1) resolution — below ~50 you cannot resolve 90% coverage cleanly. Use hierarchical/partial-pooling or empirical-Bayes shrinkage to borrow strength, or Mondrian conformal with a pooled fallback. Recommendation: pool by default at TC's scale; split out a subgroup only once it exceeds ~50–100 calibration points.

### A8. Judge-aware BTL with 4–5 judges + correlated scorers — **identifiable, but fix the shared forward pass**
Identifiability: 2601.21817 (verified) establishes it up to normalization with consistency/asymptotic normality; connected comparison graphs suffice (2605.05073). 4–5 judges is enough *if* comparisons connect the items. **The real problem TC flagged is correct and serious:** PerplexityScorer + TokenRarityScorer from one forward pass are conditionally dependent. In Dawid-Skene/latent-class models, unmodeled correlation causes overconfidence (too-narrow CIs) and biased consensus (BT-σ, Qian et al. 2026, via 2605.09702; classic Hui-Walter conditional-dependence literature). Fixes: (1) model the dependency via the LF junction-tree covariance (Snorkel approach); (2) treat the shared-forward-pass signals as ONE judge; (3) hierarchical judge-aware BTL. Recommendation: ship option (2) now; option (1) is the principled endgame.

### A9. ONNX/deterministic ML in TDX — **feasible, ~34–61% throughput cost**
ONNX Runtime in enclaves is documented (models-as-data reduces TCB, avoids pickle RCE; MDPI 2624-800X/6/1/23, SGX-focused). Determinism obstacles: FP non-associativity + dynamic-batch reduction order (Thinking Machines Sept-2025 result; batch-invariant kernels give bit-identical Qwen3-8B across 1000 runs at ~61.5% cost, reduced to ~34.35% with CUDA graphs by SGLang — via 2606.03019). ONNX specifically does not expose deterministic primitive selection or reduction order (2501.05867). For HNSW: seed the RNG and pin thread count (randomized layer assignment + parallel insert order are the nondeterminism sources). Net-new: TC's attestation is only meaningful if scoring is deterministic; budget the throughput hit or use a verify-rollback scheme (LLM-42, 2601.17768) to pay the cost only where needed.

### A10. OTel dual-emission breadth — **pin per-SDK; validate with real spans**
As of 2026: gen_ai.* is still "Development" across the registry; moved to the semantic-conventions-genai repo at v1.42.0 (12 Jun 2026) with no versioned release/schema-URL yet. Datadog native support from OTel v1.37; OpenAI Python instrumentation most mature; Anthropic/Bedrock/Cohere via community libs. `OTEL_SEMCONV_STABILITY_OPT_IN=gen_ai_latest_experimental` is honored inconsistently across frameworks. Coding agents VS Code Copilot, OpenAI Codex, and Claude Code (beta) already emit OTel GenAI traces. Recommendation: TC's version-pinning is correct; add a per-SDK conformance test that inspects an actually-exported span rather than trusting the flag.

---

## PART B — NOVEL MECHANISMS, SCALING, EFFICIENCY

### B1. Breaking the circular bootstrap
The safest link to approximate first is the *scorer*, not the labels: cold-start with a single cheap deterministic scorer (perplexity), treat its output as weak/noisy supervision (Snuba/Snorkel-style, VLDB p223), and only wire the full ensemble once you have ~100 traces to estimate LF accuracies. Self-training with a noise-aware loss (COSINE, 2111.14282) breaks the "need labels → need disagreement → need scorers" cycle without ground truth. Minimum viable bootstrap: 1 deterministic scorer → weak labels → 1 discriminative model → active-learning queries → add scorers. Do NOT start from conformal (needs the most data).

### B2. Minimum corpus size / power
The inconsistent minimums in TC's docs reflect different resolution floors: conformal needs n≥~200–300 for tight bands (or SSBC below that); Hui-Walter latent-class needs enough for the 2^(judges) contingency cells; BTL needs a connected comparison graph, not a fixed n. At ~352 traces you CAN run all simultaneously but with wide uncertainty; the binding constraint is per-cell/per-subgroup counts and calibration-set size, not total n. Recommendation: reserve ~150 for conformal calibration with SSBC, use the rest for weak-supervision + BTL.

### B3. Sublinear novelty/dedup
- **VS-Graph (2512.03394, verified): up to 450× training speedup**, competitive with GNNs, robust at D=128 — the headline exponential win for trajectory-graph novelty. HDC vectors are cheap to bind/bundle and CPU-friendly (fits the single VM).
- HyperGraphX (144.5× transductive), CiliaGraph (2405.19033), FPGA HDC (2512.08089) corroborate.
- For near-dup at corpus scale, MinHash/HNSW remain fine at 352 traces; HDC's advantage is classification + similarity in one cheap representation. **What beats HNSW at small-mid corpora:** at TC's scale, brute-force exact cosine over HDC hypervectors is simpler and deterministic (also helps A9/B9).

### B4. Multi-agent credit apportionment
Shapley is the principled split for orchestrator+sub-agent composite goods; use Owen Sampling (2508.21261) or Data-Banzhaf to avoid 2^N. Caveats: Shapley contribution metrics are volatile round-to-round (2405.08044) and semivalues are gameable (2506.12619). Recommendation: Shapley for *attribution reporting*, but cap payment variance with smoothing at N=3.

### B5. Collusion resistance at single-digit N
Honest finding: **no payment mechanism is robustly collusion-proof when N=3 and all three can collude.** False-name-resistant semivalues (2605.07663, verified) address Sybil (one actor, many identities) but not genuine multi-party collusion. Staking shifts the Sybil breakeven but not the collusion breakeven. Recommendation: lean on quality gates + provenance attestation (IronClaw-signed sessions) to make low-quality collusion unprofitable; defer clever payment mechanisms until N is larger.

### B6. Usage-linked credit / downstream attribution
TC's Vana VRC-14 and Ocean liquidity knowledge is current. Downstream value attribution has long measurement lag and needs cross-system instrumentation (which trace fed which RAG query/skill/training run). W3C PROV `wasDerivedFrom`/`wasGeneratedBy` edges are the right substrate. Recommendation: instrument retrieval→outcome links now (even if you can't value them yet); LRAT (2604.04949, verified, "Learning to Retrieve from Agent Trajectories," Zhou, 30 Mar 2026) is relevant prior art.

### B7. Tiered multiplier calibration
No literature pins TC's 1.0/1.25/1.5× tiers; the mechanism-design principle is that adoption-incentive multipliers should DECAY as adoption rises (early-contributor premium). Recommendation: make multipliers a declining function of cumulative provenance-tier supply, not fixed constants.

### B8. Delayed-activation / multi-session malicious detection
TC's cross-session corpus is genuinely differentiated: existing benchmarks test single-session. Verified anchors: MalSkillBench (2606.07131, runtime-verified malicious skills, Guo, Jun 2026), the open-agentic-skill security benchmark (2606.00925), SkillAudit (2606.14239, ground-truth-free skill-evolution auditing), Trace2Skill (2603.25158). For behavioral drift across versions/invocations: use distributional two-sample tests (MMD/KS) on tool-call patterns, resource access, and output distributions; CALIBURN (2605.24696, verified) applies regime-dependent conformal risk control to streaming detection — directly applicable to "badge expiry on skill update." Net-new: cross-session drift detection is a defensible product wedge.

### B9. Privacy-preserving retrieval / PIR in TEEs
HNSW's random memory-access patterns leak via side-channels inside enclaves; oblivious RAM/index alternatives exist but are expensive. VET (2512.15892, verified, "VET Your Agent," Grigor, 17 Dec 2025) and Proof-of-Execution (2607.05397, verified, Rhodes, Apr 2026) are relevant to attested retrieval. Recommendation: at 352 traces, deterministic brute-force scan over HDC vectors sidesteps HNSW side-channels AND provides determinism for attestation — a rare two-birds win. Revisit ORAM only at large corpus scale.

### B10. Diversity retrieval beyond MMR
Use cardinality-constrained BQP (2604.02554): 2.4–22.9× faster than MMR at θ≥0.5, sublinear in k, with a bounded interpretable trade-off (MMR has no approximation guarantee; DPP is slower and its log-det term is unbounded/hard to tune). TC's untested λ=0.7 MMR should be replaced. TC's "xMemory (2602.02007)" belief is a **title mismatch**: 2602.02007 is "Beyond RAG for Agent Memory: Retrieval by Decoupling and Aggregation" — relevant to retrieval but not the top-k-collapse citation TC thinks it is.

### B11. Retrieval eval without downstream observability
Use offline/counterfactual protocols: interleaving experiments, implicit-feedback proxies, off-policy estimation. LRAT (2604.04949, verified) learns retrieval from trajectories; ExpRAG (2603.18272, verified as "Retrieval-Augmented LLM Agents: Learning to Learn from Experience," Ferraz et al. — "ExpRAG" acronym not in title). Recommendation: log which retrieved trace was selected + a lightweight consumer-side signal (was it cited in the agent's plan?) as proxy reward.

### B12. Context budget management
Options for 10K+ token traces: (a) return consumer-specified budget, (b) summarize/compress (TRACE 2606.00611 verified; Slipstream 2605.08580 verified; CompactionRL 2607.05378 verified — but note Governance Decay 2606.22528 verified: compaction silently erases safety constraints), (c) return structured sub-traces. Recommendation: default to sub-trace return with consumer-specified budget; only compress with an explicit "safety-preserving" flag given the Governance Decay finding.

---

## PART C — MODULARITY, COMPOSABILITY, INTEROP, UX

### C1. Agent trace interchange standards (2026)
The substrate has consolidated: OTel GenAI conventions (Development status, own repo since v1.42.0) + OpenInference (richer LLM metadata, first-class RAG/span types — Arthur AI chose it) sit on the OTLP wire format; W3C PROV-DM/PROV-O is the provenance backbone (PROV-AGENT; survey 2606.04990). Governance: Linux Foundation **Agentic AI Foundation (AAIF)**, Dec 2025, hosting MCP + A2A; W3C AI Agent Protocol CG active (specs expected 2026–2027). To be a hub: ingest OTel+OpenInference, export PROV-mapped lineage, and support MCP/A2A identity for cross-agent credit.

### C2. Pluggable/modular scorer architectures
Pattern: WASM-sandboxed scoring plugins, each independently versioned and independently *attested* — each plugin extends an RTMR at load, so its measurement appears in the quote. Policy-as-code (OPA/Rego-style) for gates. This makes scorers hot-swappable while keeping attestation meaningful: the quote enumerates exactly which plugin versions ran. Net-new: attest-per-plugin is the composability unlock that also satisfies A1/A9.

### C3. Cross-agent session format matrix (2026)
| Agent | Where session data lives | On-disk format | Parseable w/o OTel? |
|---|---|---|---|
| Claude Code | Local files; emits OTel GenAI (beta) | JSON transcript / JSONL | Yes (local files) |
| OpenAI Codex | Local; emits OTel GenAI | JSON/JSONL | Yes |
| Cursor | Local app state + cloud | SQLite/JSON (app-specific) | Partially |
| GitHub Copilot | Mostly API/cloud; limited local | proprietary | Rarely (API) |
| Gemini CLI | Local | JSON/JSONL | Yes |
| IronClaw | NEAR AI runtime (integration partner) | TC-defined | Yes (native) |

*This matrix remains partially inferred — verify each on the exact 2026 client versions before load-bearing use.*

### C4. UX for trace observability & feedback
Best-in-class patterns from the observability market: span-tree trace views (invoke_agent → chat → execute_tool), node-by-node state diffs (LangSmith), issue→eval→PR closed loop (Latitude), replay (AgentOps). For contributor feedback TC should show: per-sub-score attribution ("your quality score dropped because infilling-coherence fell after redaction"), streaming partial-trace scoring, and progressive provenance disclosure. Net-new for TC: real-time partial-trace quality feedback during a session is not offered by incumbents.

### C5. Annotation tooling + anchoring bias
Sequence/trace annotation tooling is thinner than text-classification tooling. LLM pre-labeling induces measurable anchoring bias — humans over-accept LLM suggestions, inflating apparent agreement and depressing true reliability. Recommendation: for a calibration subset, withhold LLM suggestions and measure the agreement delta; this quantifies TC's reliability penalty. (Exact published magnitude is the thinnest-sourced item — see caveats.)

### C6. IAA baselines for novelty/similarity
"Is this trace novel?" is inherently low-agreement, like code-quality judgments. TC's thresholds (0.50/0.67/0.80 Krippendorff α) are uncited; realistic α for subjective novelty judgments is often 0.4–0.6. Recommendation: treat α≥0.67 as aspirational, not a gate; report observed α and use pairwise comparison (C7) to raise it.

### C7. Pairwise vs absolute
Pairwise comparison yields higher inter-annotator agreement than absolute scoring for subjective quality (well-established; k-DPP/BTL literature). Avoid O(n²) with adaptive/tournament designs (sorting-based active comparison). Recommendation: switch novelty/quality labeling to pairwise with adaptive sampling; feed into judge-aware BTL (A8).

### C8. Weak supervision in Rust
The Snorkel label model is reimplementable in ~200–500 LOC Rust. Full algorithm: (1) assemble LF matrix Λ (K items × m LFs, with abstain); (2) specify LF dependency graph; (3) compute inverse generalized covariance of the junction tree; (4) matrix-complete to recover conditional LF accuracies P(LF|Y); (5) emit probabilistic labels for a noise-aware loss (Ratner et al. 2018; confirmed 2111.14282, 2511.13891, 2604.08578). Conflict resolution and correlation modeling are the only non-trivial parts; for independent LFs it collapses to a simple generative model. Net-new: no fundamental Python dependency — unblocks TC's Rust-native goal.

---

## PART D — REGULATORY, MARKET, COMPETITIVE

### D1. GPAI Training Data Summary Template — **build export to this schema**
Published 24 Jul 2025 by the European Commission AI Office under Art. 53(1)(d) of Regulation (EU) 2024/1689 (WilmerHale: "On July 24, 2025, the European Commission's AI Office published its template for the public summary of training content for general-purpose AI (GPAI) models under Regulation (EU) 2024/1689"). Structure (Commission Explanatory Notice; corroborated Bird & Bird, Jones Day): **(1) General Information** — model/provider identity, date placed on market, knowledge-cutoff, overall data size + modalities (e.g., token count for text); **(2) List of Data Sources** — large publicly available datasets (named), licensed/third-party data (narrative), scraped content (most relevant domains), user data, synthetic data; narrative-style, aggregated, not work-by-work. The AI Office may verify compliance and issue corrective measures from 2 Aug 2026; pre-existing models have until 2 Aug 2027. TC's export function should target these fields.

### D2. Digital Omnibus citation — **confirmed and now final**
**Regulation (EU) 2026/1744**, the Digital Omnibus on AI, published in the Official Journal 24 Jul 2026, entered into force 27 Jul 2026 — six days before the AI Act's original 2 Aug 2026 high-risk deadline (Cloud Security Alliance; White & Case; K&L Gates; Gibson Dunn). Defers Annex III standalone high-risk obligations from 2 Aug 2026 to **2 December 2027**; Annex I embedded to 2 Aug 2028; adds nudifier/CSAM prohibitions (from 2 Dec 2026). Parliament endorsed 16 Jun 2026 by 423-57 with 174 abstentions; Council final approval 29 Jun 2026 (Secure Privacy; DLA Piper). Note: earlier TC-era reporting had this as merely "proposed" — it is now law.

### D3. Open-source GPAI/AI-Act compliance toolkit — **claim largely holds, with nuance**
No dominant, mature open-source GPAI-compliance toolkit surfaced. VerifyWise (verifywise.ai) is an emerging FOSS AI-governance project and should be checked as a partial counterexample before TC repeats the "none exists" claim absolutely. Recommendation: soften to "no mature, widely-adopted open-source GPAI Art. 53 compliance toolkit" and cite VerifyWise as the nearest.

### D4. Compliance platform pricing
Holistic AI, Credo AI, TrustArc, OneTrust pricing is largely gated/quote-based; TC's cited figures lack sources. Recommendation: mark all such figures "vendor quote, unverified" until confirmed from primary sources — do not use in grant applications without a citation.

### D5. Competitive verification (Aug 2026) — **correct TC's Langfuse belief**
**ClickHouse acquired Langfuse on 16 January 2026** (NOT Databricks), alongside a $400M Series D led by Dragoneer that tripled ClickHouse's valuation to ~$15B (ClickHouse blog, 16 Jan 2026: "ClickHouse has acquired Langfuse, the leading open-source platform for LLM observability, evaluations, and prompt management"; corroborated MLflow, Latitude, MarkTechPost). Resolve TC's internal ClickHouse-vs-Databricks conflict in favor of **ClickHouse**. Braintrust = eval-first, proprietary, large 2026 raise. LangSmith = deepest LangChain/LangGraph integration, proprietary. **None offer cross-user/shared trace retrieval, trajectory RAG marketplace, TEE-based scoring, or contributor compensation** — TC's differentiation on those four axes stands. Market context: The Business Research Company sizes LLM observability at $1.97B (2025) → $2.69B (2026) → $9.26B (2030) at a 36.2% forecast CAGR (via MarkTechPost, 9 Aug 2026 — treat as vendor market-research estimate).

### D6. Verified skill registry demand
TC's "490K+ skills / 32+ adopters" figures remain uncited and should be treated as unverified. Enterprise willingness-to-pay for *verified/certified* skills is plausible but unquantified in the public sources found. Recommendation: do not cite the 490K/32 figures without a primary source; frame demand as qualitative.

---

## VERIFICATION LEDGER

| arXiv ID | TC's claim | Verdict | Actual |
|---|---|---|---|
| 2606.18467 | ToolChain-CRC | ✅ VERIFIED | Conformal Risk Control for Agentic AI (title match) |
| 2607.24343 | Role-Stratified-CRC | ✅ VERIFIED | Role-Stratified CRC for LLM Tool Calls, Rahman, 27 Jul 2026 |
| 2605.18812 | PASC | ✅ VERIFIED | Pipeline-Aware Conformal Prediction, Kotte, May 2026 |
| 2605.07663 | Sybil unfair payoff 1.74× | ✅ VERIFIED (title) | "Quotient Semivalues for False-Name-Resistant Data Attribution"; 1.74× unconfirmed from title |
| 2506.12619 | semivalue gameability | ✅ VERIFIED | semivalues gameable, low-cost adversarial strategies |
| 2606.20669 | Agent Behavior Mining | ✅ VERIFIED | Agent Behavior Mining, business-process governance |
| 2607.02599 | AgentLTL | ✅ VERIFIED | AgentLTL trace-verification, Elkoussy, 1 Jul 2026 |
| 2606.08275 | Causal Agent Replay | ✅ VERIFIED | Causal Agent Replay, Shah, 6 Jun 2026 |
| 2605.25338 | CausalFlow | ✅ VERIFIED | CausalFlow, Bonagiri, 25 May 2026 |
| 2509.03312 | AgenTracer-8B | ✅ VERIFIED | AgenTracer, Zhang, 3 Sep 2025 |
| 2606.14805 | Zero-Replay Debugging | ✅ VERIFIED | Knowledge-Based Zero-Replay Debugging |
| 2606.00611 | TRACE compression | ✅ VERIFIED | Trajectory Risk-Aware Compression |
| 2606.31564 | ACE | ✅ VERIFIED | ACE: Pluggable Adaptive Context Elasticizer (the "31" is a sequence number, not a day) |
| 2605.08580 | Slipstream | ✅ VERIFIED | Slipstream compaction validation, Chen, 9 May 2026 |
| 2607.05378 | CompactionRL | ✅ VERIFIED | CompactionRL, Li, 6 Jul 2026 |
| 2606.22528 | Governance Decay | ✅ VERIFIED | Governance Decay via compaction, Chen, 21 Jun 2026 |
| 2606.05922 | RHO | ✅ VERIFIED | Retrospective Harness Optimization, Pan, 4 Jun 2026 |
| 2603.25158 | Trace2Skill | ✅ VERIFIED | Trace2Skill |
| 2606.14239 | SkillAudit | ✅ VERIFIED | SkillAudit, paired-trajectory auditing |
| 2504.17703 | federated learning survey | ⚠️ WITHDRAWN | Title matches (Rahman) but arXiv-admin withdrawn (disputed authorship) — **DO NOT CITE** |
| 2606.09043 | DynaCF | ✅ VERIFIED | DynaCF reward-model shortcut mitigation |
| 2604.07484 | ConsistRM | ✅ VERIFIED | ConsistRM consistency-aware self-training |
| 2605.24696 | CALIBURN | ✅ VERIFIED | CALIBURN, regime-dependent conformal risk control |
| 2506.15655 | cAST | ✅ VERIFIED | cAST AST structural chunking, Zhang, 18 Jun 2025 |
| 2602.14102 | DALL | ✅ VERIFIED | DALL text-labeling framework |
| 2512.19682 | GenEnv | ✅ VERIFIED | GenEnv co-evolutionary generative environment |
| 2509.24291 | hard-negative mining | ❌ DIFFERENT PAPER | Actually **GIRCSE** (generative contrastive sentence embeddings) |
| 2604.02324 | grounded vocab init | ✅ VERIFIED | Grounded Token Initialization (GTI) |
| 2604.16656 | grounded vocab init | ⚠️ RELATED, DIFFERENT | "Defragmenting Language Models" (vocab expansion, not "grounded init") |
| 2512.03394 | VS-Graph 450× | ✅ VERIFIED | VS-Graph, up to 450× speedup, D=128 robust; +4–5% over prior HDC on MUTAG/DD |
| 2602.02007 | xMemory (top-k collapse) | ⚠️ TITLE MISMATCH | "Beyond RAG for Agent Memory: Retrieval by Decoupling and Aggregation" |
| 2604.04949 | LRAT | ✅ VERIFIED | Learning to Retrieve from Agent Trajectories, Zhou, 30 Mar 2026 |
| 2603.18272 | ExpRAG | ✅ VERIFIED (topic) | "Retrieval-Augmented LLM Agents: Learning to Learn from Experience," Ferraz; "ExpRAG" not in title |
| 2605.03344 | T3 (+56.3%) | ✅ VERIFIED (topic) | "RAG over Thinking Traces Can Improve Reasoning"; +56.3% unconfirmed from title |
| 2605.11053 | MCPShield (2–10pp AUC) | ⚠️ LIKELY MISMATCH | Empirical tool-call-traffic detection study; "MCPShield" not in title |
| 2606.22741 | GRADE | ✅ VERIFIED | GRADE graph representation of agent dependency/execution |
| 2607.05397 | Proof-of-Execution | ✅ VERIFIED | Proof of Execution, Rhodes, Apr 2026 |
| 2602.13795 | Agent-OSI | ✅ VERIFIED | Agent-OSI six-layer stack |
| 2512.15892 | VET | ✅ VERIFIED | VET Your Agent, verifiable execution traces, Grigor, 17 Dec 2025 |
| 2601.21817 | Judge-Aware Ranking | ✅ VERIFIED | Judge-Aware Ranking, Xu et al., ICML 2026 |
| 2603.01971 | LOCUS (group-conditional conformal) | ⚠️ DESCRIPTION MISMATCH | "Locus" = per-input loss-scale reliability wrapper, NOT group-conditional conformal |
| 2502.06901 | MARIA-7B | ✅ VERIFIED (topic) | MARIA infilling architecture; "7B" not stated in abstract |
| 2506.00204 | AST-FIM | ✅ VERIFIED | Structure-Aware FIM Pretraining, Gong, 30 May 2025 |
| 2511.15364 | Wu surrogate 20–67% | ✅ VERIFIED (topic) | Anonymization info-loss; author + 20–67% figure unconfirmed |
| 2606.00925 | SkillVetBench | ✅ VERIFIED (topic) | Open-agentic-skill security benchmark; acronym likely internal |
| 2606.07131 | MalSkillBench | ✅ VERIFIED | MalSkillBench, Guo, Jun 2026 |
| 2603.00729 | (Qwen3-Coder-Next) | ✅ VERIFIED | Qwen3-Coder-Next Technical Report (FIM confirmed) |

**Non-arXiv items:** SurrogateShield (94.85% clinical BERTScore) — consistent with clinical surrogate literature but not independently verified this round. SIGIL on-chain skill registry, Trail of Bits agent-skill scanner bypass, Sampled VCG (Balkanski 2017), SkillFortify (~96.95% F1), SIGIL — **not verified this round; flag as unconfirmed.**

---

## PRIORITIZED ROADMAP

**Immediate (weeks):**
1. Ship the A1 fix: reorder score→redact inside the existing measured enclave (no re-attestation). Confirm both stages are in the measured TCB.
2. Turn on Qwen native FIM infilling-coherence sub-score (A2) — zero extra VRAM if the production scorer is Qwen3-Coder-family.
3. Run TC's own raw-vs-redacted perplexity A/B on the 352 traces (A3); replace the "8–12 nats" internal number with measured values and publish, citing arXiv:2309.08628 as the anchor.
4. Adopt SSBC for conformal calibration (A6); reserve ~150 traces for calibration.
5. Correct docs: Langfuse→**ClickHouse (16 Jan 2026)** (D5); Digital Omnibus = **Reg (EU) 2026/1744**, Annex III → 2 Dec 2027 (D2); fix the four citation errors (2509.24291, 2504.17703, 2603.01971, 2602.02007).

**1–3 months:**
6. Prototype VS-Graph HDC novelty scoring (B3) — target the 450× compute reduction; brute-force HDC scan also solves A9 determinism + B9 side-channels.
7. Replace MMR with cardinality-constrained BQP diversity retrieval (B10).
8. Collapse the two shared-forward-pass scorers into one judge OR model the dependency (A8).
9. Reimplement the Snorkel label model in Rust (~200–500 LOC, C8); bootstrap via the B1 minimum sequence.
10. Build the GPAI Training Data Summary export function (D1).

**3–6 months:**
11. Build a code-aware, referentially-consistent surrogate generator (A5) — net-new IP.
12. Ship WASM attest-per-plugin scorer architecture (C2) + PROV-mapped lineage export (C1) to become an interop hub.
13. Cross-session behavioral-drift detection for delayed-activation skills (B8) — the defensible product wedge; use CALIBURN-style regime-dependent conformal control for badge expiry.
14. Switch novelty/quality labeling to pairwise + judge-aware BTL (C7/A8); measure LLM pre-labeling anchoring bias (C5).

**What would change these recommendations:**
- If the production scorer is base Qwen3 (not Coder-family), A2's "zero extra VRAM" is wrong — you need a small FIM head.
- If determinism throughput cost (34–61%) is unacceptable on the single VM, defer meaningful attestation of scoring or move to verify-rollback (LLM-42, 2601.17768).
- If N grows past ~10 contributors, revisit collusion-resistant payment mechanisms (B5) — they become viable.
- If corpus grows past ~10⁴–10⁵ traces, revisit HNSW/ORAM (B9) over brute-force HDC scan.
- If OTel gen_ai.* reaches Stable, drop dual-emission complexity (A10).

---

## CAVEATS
- **Maturity:** VS-Graph (450×), BQP retrieval, and the judge-aware BTL papers are 2025–2026 preprints; replication in TC's exact setting is unproven. HDC accuracy on TC's trajectory graphs (vs MUTAG/DD molecular graphs) needs validation before committing.
- **Model access:** the FIM recommendation depends on the exact production checkpoint; confirm before committing.
- **Figures flagged as editorial/secondary:** the $2.69B market size (The Business Research Company via MarkTechPost), "up to 450×," "34–61% throughput cost," and all compliance-vendor pricing should be treated as directional, not audited. The 1.16→2.83 masking-perplexity figures (Firstsource/ClearView blog) are unconfirmed — prefer arXiv:2309.08628.
- **Citation hygiene:** several IDs are topic-correct but the specific acronym/figure is unconfirmable from metadata (T3 +56.3%, MCPShield 2–10pp, Sybil 1.74×, surrogate 20–67%); do not quote those exact numbers without fetching the papers. Do NOT cite 2504.17703 (withdrawn).
- **Thinnest-sourced items:** the cross-agent session format matrix (C3) and the LLM pre-labeling anchoring-bias magnitude (C5) should be verified on current client versions / with a targeted search before load-bearing use.