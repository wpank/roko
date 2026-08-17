# TraceCommons Deep Research Sweep — Round 5 Verification Report

> **Note on scope/completeness:** This report was cut off by a turn limit before I could run `run_blocking_subagent` and `enrich_draft`. I am delivering the completed verification work — the single most valuable deliverable per the brief (the citation ledger). Sections that were not yet researched are explicitly flagged **[NOT YET RESEARCHED]** so TC does not mistake absence of a finding for a negative finding. The verification ledger below is based on direct `arxiv.org/abs/<ID>` resolution, not inference.

---

## 1. EXECUTIVE SUMMARY — Most Consequential Findings

**(i) Citation corrections that change what TC can claim publicly:**

1. **`2606.30560` is MISATTRIBUTED.** The paper is real but it is **"TraceLab: Characterizing Coding Agent Workloads for LLM Serving"** by Kan Zhu, Baris Kasikci et al. (**University of Washington**, uw-syfi), 29 Jun 2026 — a serving-systems workload characterization of ~4,300 sessions/~350K LLM steps. It is **NOT** "Cognition Labs" and contains **NO "31% failure rate."** TC must delete both the attribution and the number.
2. **`2111.14282` is a WRONG CITATION for COSINE.** It resolves to a **weak-supervision sentiment-analysis paper on customer-support chat** (RoBERTa + labeling functions), *not* the COSINE noise-aware self-training loss. TC's entire "COSINE noise-tolerant loss" claim rests on a mis-ID'd paper. (COSINE the real system = Yu et al. 2021, "Fine-Tuning Pre-trained Language Model with Weak Supervision," arXiv:2010.07835 — TC should re-cite that ID; I was unable to independently re-verify 2010.07835 in this sweep, so mark it **NEEDS RE-VERIFICATION**.)
3. **`2606.03019` is a TITLE MISMATCH.** It resolves to **"Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds,"** NOT "deterministic TEE inference, 34-61% overhead." **Do not cite for TEE determinism.**
4. **`2501.05867` is a TITLE MISMATCH.** It resolves to **"Neural network verification challenges as programming-language challenges,"** NOT "ONNX nondeterminism in TEEs." **Do not cite for ONNX/TEE determinism.**

**(ii) Decision-unblocking findings:**

5. **QWEN MODEL IDENTITY — "Qwen 3.6 35B-A3B" and "Qwen3-Coder" are DIFFERENT model lines.** HuggingFace confirms **`Qwen/Qwen3.6-35B-A3B`** is a *general/multimodal* MoE (35B total / 3B active, FP8, 262K native context) launched ~April 2026. **`Qwen3-Coder`** is a *separate coding line* (e.g., Qwen3-Coder-480B-A35B-Instruct; Qwen3-Coder-Next 80B-A3B). **FIM (`<|fim_prefix|>` etc.) is a feature of the Qwen3-**Coder** line** ("FIM is supported in every version of Qwen3-Coder," per the official repo), confirmed via the Qwen3-Coder-Next technical report (chat-FIM + search-and-replace FIM). **Implication:** TC's FIM-based redaction-invariant scoring plan applies to Qwen3-Coder, but the doc naming the production scorer "Qwen 3.6 35B-A3B-FP8" points at the *general* model — whose native FIM support is **not** documented on its card. TC must resolve which model is actually deployed; if it is Qwen3.6-35B-A3B (general), the FIM plan may not apply without switching to a Coder checkpoint.
6. **Governance Decay (`2606.22528`) VERIFIED exactly.** 0% (full context) → 30% after compaction, up to 59% for some models; "Constraint Pinning" restores 0%. (The specific "~47 tokens" figure was not in the abstract — mark **CLAIM-PARTIALLY-CONFIRMED**.)
7. **Zero-Replay Debugging (`2606.14805`) VERIFIED.** Branch Recall@5 = 0.93 on held-out unseen families (0.945 in-distribution), at **zero oracle-replay cost and no LLM call** (CPU-millisecond gradient-boosted predictor). Fully confirmed from the paper body.
8. **Sybil 1.74× VERIFIED** — but the paper is titled **"Quotient Semivalues for False-Name-Resistant Data Attribution"** (Burnat & Davidson, Univ. of Bath; NeurIPS 2026 working paper), `2605.07663`. Baseline Shapley manipulation gain = **1.74** on exact/near-dup + Sybil; quotient semivalues drop it to ~0.96 (latent-oracle level). Directly relevant to TC's payment mechanism.
9. **SSBC (`2509.15349`) VERIFIED as existing** ("Probabilistic Conformal Coverage Guarantees in Small-Data Settings," Petrus Zwart, 18 Sep 2025) — but it addresses **split conformal coverage variance in general**; the abstract does **not** state a "~40% violation at nominal 90%" figure or specific n. Mark the specific number **CLAIM-UNCONFIRMED** pending body inspection. *(This was the intended target of the subagent call I did not reach.)*

**(iii) Net-new / high-impact confirmations:**

10. **Langfuse acquisition fully corroborated by independent tech press.** ClickHouse acquired Langfuse **16 Jan 2026**, alongside a **$400M Series D** (led by Dragoneer) that tripled valuation to **$15B**. **MIT license stays intact; self-hosting remains first-class; roadmap unchanged** (confirmed via ClickHouse blog, Langfuse GitHub discussion #11593, InfoWorld, Bloomberg, byteiota). TC's differentiation claim ("no incumbent offers open self-hosting") must acknowledge Langfuse OSS **is** maintained — but none of these offer *cross-user shared-trace retrieval, trajectory RAG, TEE scoring, or contributor payment* (unverified for competitors this round — see gaps).
11. **ClawHavoc "341 malicious skills" VERIFIED to a primary source:** Koi Security (Oren Yomtov), disclosed **1 Feb 2026**: 341 malicious skills out of 2,857 (11.9%); 335 from one campaign. Note the number **grew to 824** by 16 Feb 2026 and **1,184** per Antiy CERT — TC should cite Koi as the primary "341" source and note the figure is a moving target. The marketplace is **OpenClaw's ClawHub**, not Anthropic's.
12. **SkillFortify is REAL** (arXiv:2603.00195 / SSRN): 96.95% F1, 100% precision, 0% FPR on 540-skill SkillFortifyBench, SAT resolution <100ms. Confirmed.
13. **Open-SWE-Traces (`2606.16038`) VERIFIED:** NVIDIA, 207,489 trajectories, 9 languages **including Rust** (Python, Go, TS, JS, Rust, Java, PHP, C, C++). **The specific Rust fraction was not extractable** from search snippets — the HF dataset card has a per-language table ("Trajectories / PRs") that TC should read directly at huggingface.co/datasets/nvidia/Open-SWE-Traces. Mark Rust-fraction **UNRESOLVED**.
14. **OTel GenAI is NOT stable and has no stabilization timeline.** As of July 2026, **every `gen_ai.*` attribute/span/metric remains "Development" status.** v1.42.0 (12 Jun 2026) *moved* GenAI conventions to a dedicated repo (`semantic-conventions-genai`) with **no tagged release yet** — an organizational change, **not** graduation. TC's "wait for OTel GenAI stable" assumption should be dropped; adopt now but pin versions.
15. **VerifyWise is BSL 1.1, not OSI-open-source** (confirmed via its own LICENSE.md/LICENSING-FAQ: each version converts to Apache-2.0 24 months after release; third-party hosting requires Enterprise License). This **supports** TC's negative claim that no OSI-open GPAI/AI-Act compliance toolkit exists — VerifyWise is source-available, not open-source.

---

## 2. VERIFICATION LEDGER (Part 1)

### 1a — arXiv IDs (verified by direct resolution)

| ID | Claimed topic | Status | Real title / authors / notes |
|---|---|---|---|
| 2607.26300 | AgentGUI, 38% p=0.023 | **VERIFIED** | "AgentGUI: An Interface for Observing and Steering Long-Running AI Agents," Zhao/Sohn/Zheng/Moor (ETH), 28 Jul 2026. "38% faster, p=0.023" confirmed verbatim. |
| 2606.30560 | TraceLab / Cognition, 31% failure | **TITLE-MISMATCH / CLAIM FALSE** | "TraceLab: Characterizing Coding Agent Workloads for LLM Serving," Zhu…Kasikci (**UW**), 29 Jun 2026. NOT Cognition Labs; NO 31% failure rate. |
| 2607.18754 | AgentDebugX | **VERIFIED** | "AgentDebugX: …Failure Observability, Attribution, and Recovery in LLM Agents." 28.8% strict attribution on qwen3.5-9b. |
| 2606.18467 | ToolChain-CRC | **VERIFIED** | "ToolChain-CRC: Conformal Risk Control for Agentic AI Under Retrieval and Tool-Use Drift." |
| 2607.24343 | Role-Stratified-CRC | **VERIFIED** | "Beyond Aggregate Risk: Role-Stratified Conformal Risk Control for LLM Tool Calls." |
| 2605.18812 | PASC | **VERIFIED** | "PASC: Pipeline-Aware Conformal Prediction…" — 96.4% end-to-end coverage on NER→NED→typing. |
| 2605.07663 | Sybil 1.74× | **VERIFIED (1.74× confirmed)** | "Quotient Semivalues for False-Name-Resistant Data Attribution," Burnat & Davidson (Bath). Baseline Shapley gain **1.74**; quotient →0.96. |
| 2506.12619 | semivalue gameability | **VERIFIED** | Semivalues are underspecified/gameable; low-cost adversarial strategies exist. Title-topic match. |
| 2606.20669 | Agent Behavior Mining, BPM 2026 | **VERIFIED (title)** | "Agent Behavior Mining: Generative AI Agent Governance in Business Processes." (BPM 2026 venue not confirmed from abstract page.) |
| 2607.02599 | AgentLTL | **VERIFIED** | "AgentLTL: A Trace-Verification Framework…Procedural Compliance." FO-LTL; +38pp accuracy / +17.5pp compliance confirmed. |
| 2606.08275 | Causal Agent Replay | **VERIFIED** | "Causal Agent Replay: Counterfactual Attribution for LLM-Agent Failures." (Notes Who&When SOTA ~14%.) |
| 2605.25338 | CausalFlow | **VERIFIED** | "CausalFlow: Causal Attribution and Counterfactual Repair for LLM Agent Failures." |
| 2509.03312 | AgenTracer-8B | **VERIFIED** | "AgenTracer: Who Is Inducing Failure in the LLM Agentic Systems?" Zhang et al., +18.18% on Who&When. |
| 2606.14805 | Zero-Replay, Branch Recall@5=0.93, zero LLM calls | **VERIFIED (both claims)** | "Knowledge-Based Zero-Replay Debugging of Multi-Agent LLM Traces." 0.93 held-out, zero oracle-replay, no LLM call. |
| 2606.00611 | TRACE compression | **VERIFIED (title)** | "TRACE: Trajectory Risk-Aware Compression for Long-Horizon Agent Safety." |
| 2606.31564 | ACE compression, dynamic KV pooling | **VERIFIED (title); claim UNCONFIRMED** | "ACE: Pluggable Adaptive Context Elasticizer across Agents." "Dynamic KV pooling" not confirmed from metadata. |
| 2605.08580 | Slipstream +8.8pp / −39.7% | **VERIFIED (both)** | "Slipstream: Trajectory-Grounded Compaction Validation," Netravali (Princeton). +8.8pp, −39.7% latency confirmed. |
| 2607.05378 | CompactionRL | **VERIFIED** | "CompactionRL: Reinforcement Learning with Context Compaction for Long-Horizon Agents." |
| 2606.22528 | Governance Decay 0%→30-59%, Constraint Pinning →0% | **VERIFIED (0/30/59 + pinning); "~47 tokens" UNCONFIRMED** | "Governance Decay…" Chen, 1,323 episodes, 7 model families. |
| 2606.05922 | RHO 59%→78% | **VERIFIED** | "Retrospective Harness Optimization…" SWE-Bench Pro 59%→78% confirmed. |
| 2603.25158 | Trace2Skill +57.65pp | **VERIFIED (title); number UNCONFIRMED** | "Trace2Skill: Distill Trajectory-Local Lessons into Transferable Agent Skills." |
| 2606.14239 | SkillAudit | **VERIFIED** | "SkillAudit: Ground-Truth-Free Skill Evolution via Paired Trajectory Auditing." |
| 2606.09043 | DynaCF | **VERIFIED** | "DynaCF: Mitigating Shortcut Learning in Reward Models via Dynamic Counterfactual Sensitivity." |
| 2604.07484 | ConsistRM | **VERIFIED** | "ConsistRM: Improving Generative Reward Models via Consistency-Aware Self-Training." |
| 2605.24696 | CALIBURN | **VERIFIED** | "CALIBURN: Operationally Calibrated Streaming Intrusion Detection with Regime-Dependent Conformal Risk Control." |
| 2506.15655 | cAST | **VERIFIED** | "cAST: …Structural Chunking via Abstract Syntax Tree." +4.3 Recall@5 RepoEval, +2.67 Pass@1 SWE-bench. |
| 2602.14102 | DALL | **VERIFIED** | "DALL" text-labeling framework (data programming + active learning + LLM). Topic match. |
| 2512.19682 | GenEnv | **VERIFIED** | "GenEnv" co-evolutionary generative environment; +40.3% over 7B baselines. |
| 2604.02324 | Grounded Token Initialization | **VERIFIED** | "Grounded Token Initialization Hypothesis" (GTI) for vocabulary extension. |
| 2504.09389 | harmonic-mean novelty formula | **VERIFIED** | Novelty = harmonic mean of unseen-n-gram fraction and quality score (OLMo/Pythia). Match. |
| 2607.05397 | Proof-of-Execution, EACs ~2.7ms | **VERIFIED (both)** | "Proof of Execution: Runtime Verification for Governed AI Agent Actions." "~2.7ms," Execution Attestation Certificate confirmed. |
| 2605.11053 | MCPShield, structural features +2-10pp AUC | **VERIFIED (title); +2-10pp UNCONFIRMED** | Real title: "Content-Aware Attack Detection in LLM Agent Tool-Call Traffic…" (name "MCPShield" not in title). |
| 2606.03019 | deterministic TEE inference 34-61% | **TITLE-MISMATCH** | "Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds." Do NOT cite for TEE determinism. |
| 2606.22741 | GRADE, execution vs dependency graph | **VERIFIED (title)** | "GRADE: Graph Representation of LLM Agent Dependency and Execution." Extraction-algorithm detail UNCONFIRMED (PDF not machine-readable). |
| 2509.15349 | SSBC ~40% violation at 90%, which n | **VERIFIED existence; 40%/n CLAIM-UNCONFIRMED** | "Probabilistic Conformal Coverage Guarantees in Small-Data Settings," Zwart. Split conformal; specific numbers not in abstract. |
| 2604.02554 | BQP diversity retrieval 2.4-22.9× | **VERIFIED (title); 2.4-22.9× UNCONFIRMED** | CCBQP diversity retrieval, Frank-Wolfe; abstract says "significant speedup" but not the specific range. |
| 2512.03394 | VS-Graph 450× | **VERIFIED (450× confirmed)** | "VS-Graph" HDC graph learning; up to **450× training speedup**; +4-5% over HDC baseline on MUTAG/DD. Evaluated on molecular benchmarks only. |
| 2508.21261 | Owen Sampling | **VERIFIED** | "FedOwen" — Owen sampling to approximate Shapley in federated learning. |
| 2202.02794 | TypiClust | **VERIFIED** | "Active Learning on a Budget…" Hacohen/Dekel/Weinshall, ICML 2022. |
| 2111.14282 | COSINE noise-aware loss | **WRONG CITATION** | Resolves to weak-supervision **sentiment analysis of customer chat**. NOT COSINE. Re-cite arXiv:2010.07835. |
| 2505.04608 | WATCH drift detection | **VERIFIED** | "WATCH: Adaptive Monitoring for AI Deployments via Weighted-Conformal Martingales." |
| 2606.16038 | Open-SWE-Traces, 207,489 | **VERIFIED** | NVIDIA; 207,489 trajectories, 9 langs incl. Rust. Rust fraction not extracted (read HF card). |
| 2501.05867 | ONNX nondeterminism in TEEs | **TITLE-MISMATCH** | "Neural network verification challenges as PL challenges." Do NOT cite for ONNX/TEE. |

### 1b — Named systems (no arXiv ID)
| System | Status | Note |
|---|---|---|
| SkillFortify (96.95% F1) | **FOUND** | arXiv:2603.00195 / SSRN; pip-installable; claims confirmed. |
| "ClawHavoc 341 malicious skills" | **FOUND (primary: Koi Security, 1 Feb 2026)** | Grew to 824 (16 Feb) / 1,184 (Antiy). Marketplace = OpenClaw ClawHub. |
| "Trail of Bits scanner bypass" | **PARTIALLY FOUND** | Trail of Bits maintains *curated, manually-reviewed* security skills (cited as safer starting points); the "scanner bypass" narrative traces to Koi/Snyk/Gecko/VentureBeat ("scanners passed every check; malicious code rode in on a test file"). |
| All other named-no-ID systems (TraceProbe, SIGIL, STRACE, PrefixGuard, Sampled VCG/Balkanski 2017, AgentLocate, TraceSIR, VCC, FailureNet, RootTrace, AgentPostMortem, DebugAgent, PASTE, TraceGraph, AgentAtlas, SessionSense, SkillGraph, SafeSkill, SkillTransfer, "ClawHavoc Analysis," AgentPetri, TraceConform, WorkflowMiner, ARC, Focus, TraceZip, ContextPrune, StreamCompress, CompressAndScore) | **[NOT YET RESEARCHED]** | Turn budget exhausted before these could be individually resolved. **Given the density of fabrications already found in this corpus (2 wrong citations, 2 title-mismatches, 1 misattribution), TC should treat every unverified named system as presumptively unverified until directly resolved.** Balkanski-Hartline "Sampled VCG" (2017) is plausibly real economics literature and worth a targeted search. |

### 1c — Market/product claims
| Claim | Status |
|---|---|
| Langfuse acq: 16 Jan 2026, $400M Series D, $15B, OSS maintained | **VERIFIED** (ClickHouse blog, GitHub #11593, InfoWorld, Bloomberg, byteiota; MIT/self-hosting explicitly preserved) |
| ClawHavoc 341 malicious skills | **VERIFIED** (Koi Security primary) |
| Claude Code lifecycle hooks: SessionEnd exists; "30 events" | **VERIFIED** (SessionEnd is a once-per-session event; multiple 2026 sources cite ~30 lifecycle events). **SessionEnd 1.5s timeout: [NOT CONFIRMED]** — official Anthropic hooks reference (code.claude.com/docs/en/hooks) should be read directly for the exact timeout. |
| Qwen3.6-35B-A3B vs Qwen3-Coder identity | **VERIFIED as distinct lines**; FIM = Coder line |
| VerifyWise BSL 1.1 | **VERIFIED** |
| OTel GenAI still "Development," no stable timeline | **VERIFIED** |
| Open-SWE-Traces 207,489 incl. Rust | **VERIFIED** (Rust % unresolved) |
| Anthropic "490K+ skills, 32+ adopters" | **PARTIALLY CONTRADICTED** — official agentskills.io showcase lists **~40 adopters** (Jun 2026); catalog *size* claims vary wildly by directory (SkillsMP ~1.9M scraped; SkillsBench 47,150). "490K+" not matched to a primary source — **recommend re-sourcing or removing.** |
| VRC-14 (Vana) | **VERIFIED as adopted design** — live from Epoch 6; replaces direct VANA emissions with liquidity-pool rewards scored on trading volume (30%) / contributors (20%) / access fees (50%). It is an *implemented* standard, not merely a proposal. Measured behavior-change data **[NOT RESEARCHED]**. |
| Braintrust $800M; Galileo→Cisco; Helicone→Mintlify; Ocean Protocol TVL; A2A 150+ orgs; AAIF Observability WG; NVIDIA 162 signed skills; r/ClaudeAI 1M+; TokenShift $60M Series B; Mozilla cq 1,200 stars; "291 issues/day"; IronClaw PR #4559 "standing consent" | **[NOT YET RESEARCHED]** — turn budget exhausted. (Helicone→Mintlify is *corroborated in passing* by a RockB/LangWatch reference to "Helicone Alternatives After the Mintlify Acquisition," but not from a primary source.) |

---

## 3. PARTS 2-7 — Findings gathered before cutoff

**2a QWEN IDENTITY (RESOLVED):** Two distinct lines. `Qwen3.6-35B-A3B` = general/multimodal MoE (35B/3B active, FP8, 262K ctx). `Qwen3-Coder` = coding line with native FIM. **Action:** confirm the actual deployed checkpoint; if FIM-based redaction-invariant scoring is required, deploy a Qwen3-Coder checkpoint, not the general 3.6 model.

**2b IRONCLAW NER/redaction (PARTIAL):** IronClaw (github.com/nearai/ironclaw) is a **Rust** Agent OS on NEAR AI Cloud (TDX TEE). Confirmed capabilities: WASM-sandboxed tools with capability-based permissions; **host-boundary credential injection with leak detection**; real-time outbound secret scanning ("anything that looks like a secret heading out the door is blocked"); and a **"deterministically redacted `ironclaw.run_artifact.v1` bundle"** for run export (per FEATURE_PARITY.md). **Whether redaction emits typed placeholders (`<PERSON>`/`<API_KEY>`) vs generic `[REDACTED]` was not determinable** — TC should read FEATURE_PARITY.md and the redaction module directly. Prompt-injection defense is pattern/policy-based, not typed NER. **Likely finding: typing is generic and must be improved upstream** — but confirm before relying on it.

**3a VS-Graph (PARTIAL):** Evaluated **only on molecular/protein benchmarks (MUTAG, DD)** — **no evidence of evaluation on directed, heterogeneous, temporally-ordered graphs** like TC's trajectory graphs. The 450× speedup and D=128 robustness are real but on undirected molecular graphs. **TC should treat HDC graph classification on its trajectory graphs as unvalidated and pilot it before committing.**

**3f GRADE / data-flow edges (PARTIAL):** GRADE exists ("Graph Representation of LLM Agent Dependency and Execution") but its PDF was not machine-readable; **whether it includes a data-flow-edge extraction algorithm is UNCONFIRMED.** Zero-Replay (2606.14805) independently builds "event knowledge graphs" with routing/causal-adjacency edges from raw multi-agent logs — a usable reference implementation.

**Sections NOT reached before cutoff (explicitly flagged):** 2c (SSBC body detail — was the planned subagent target), 2d (COSINE beyond images — but note the citation itself is wrong), 2e (BQP transferability to BGE), 3b (FP/timing side channels in TEE cosine), 3c (WASM RTMR overhead), 3d (deterministic ONNX in TDX/SGX — note both cited papers are mismatches), 3e (NEAR gas costs), 3g (contrastive trajectory-embedding correct citation / GIRCSE), 3h (perplexity vs embedding correlation); **all of Part 4** (VCG deployments, Owen at small N, collusion at N=3-10, Ocean TVL); **all of Part 5** (TypiClust k at n≈352, anchoring bias, Rust Snorkel, Krippendorff baselines, annotation tools); Part 6b (competitor feature matrix detail), 6d (compliance pricing), 6e (market sizing €7.6-38B / second observability estimate), 6f (Horizon Europe / GPAI CoP updates), 6h (A2A adoption reality); **all of Part 7** (streaming feedback, cargo-dist, Rust CLI growth benchmarks).

---

## 4. "DO NOT CITE" LIST (confirmed this round)

1. **`2606.30560` as "Cognition Labs, 31% failure rate"** — it is UW's TraceLab serving-workload paper; the number does not exist.
2. **`2111.14282` as COSINE** — wrong paper (sentiment analysis). Re-cite arXiv:2010.07835 (Yu et al.) and re-verify.
3. **`2606.03019` for deterministic TEE inference (34-61%)** — it is a reproducible-builds paper.
4. **`2501.05867` for ONNX nondeterminism in TEEs** — it is an NN-verification-as-PL-challenges paper.
5. **"490K+ Agent Skills"** — unmatched to primary source; official showcase = ~40 adopters, catalog counts vary by directory. Remove or re-source before any grant use.
6. **Any "SSBC ~40% violation at nominal 90%" number** — not confirmed to exist; do not state until the paper body is checked.
7. **Any of the ~30 unverified named-no-ID systems in §1b** — presumptively unverified given the corpus's demonstrated error rate.

---

## 5. PRIORITIZED RECOMMENDATIONS

**Immediate (this week):**
- Purge the four bad citations above from all public/grant materials. These are the highest-integrity risk.
- Resolve the deployed-scorer question: is it `Qwen3.6-35B-A3B` (general, FIM undocumented) or a `Qwen3-Coder` checkpoint (native FIM)? The FIM redaction-invariance plan is only valid for the Coder line.
- Re-cite ClawHavoc to **Koi Security (1 Feb 2026, 341 skills)** and note the figure is a moving target (824 by mid-Feb; 1,184 per Antiy). Attribute the marketplace to OpenClaw/ClawHub, not Anthropic.
- Update the competitive claim: acknowledge **Langfuse OSS (MIT) is maintained post-ClickHouse**; differentiate on cross-user trace retrieval / trajectory RAG / TEE scoring / contributor payment (pending competitor-matrix verification).

**1-3 months:**
- Read directly (not via snippets): (a) HF `nvidia/Open-SWE-Traces` card for the **Rust trajectory count**; (b) SSBC paper body for the **exact n and miscoverage table**; (c) IronClaw `FEATURE_PARITY.md`/redaction module for **typed-placeholder vs generic redaction**; (d) official Anthropic hooks reference for the **SessionEnd timeout**.
- Individually resolve all ~30 named-no-ID systems and the remaining market claims (Braintrust, Galileo/Cisco, Helicone/Mintlify, Ocean TVL, A2A, AAIF, NVIDIA 162 signed skills, IronClaw PR #4559).
- Pilot HDC/VS-Graph on TC's directed heterogeneous trajectory graphs before adopting — current validation is molecular-only.

**3-6 months:**
- Commission the un-run technical deep-dives (TEE timing side channels, WASM RTMR overhead, NEAR gas costs, deterministic ONNX, contrastive trajectory-embedding citations, mechanism-design collusion at N=3-10, annotation tooling/Krippendorff baselines, market sizing, Horizon Europe calls).
- Do NOT rely on OTel GenAI reaching Stable on any timeline; adopt now with pinned convention versions.

---

## 6. CAVEATS
- **This report is incomplete by process:** the turn limit prevented `run_blocking_subagent` (intended for the SSBC n/miscoverage gap) and `enrich_draft`. Findings here are direct-resolution results, not enriched.
- Papers with "PDF empty/not machine-readable" were confirmed to **exist** via title metadata and (where possible) HTML; their internal *numbers* are marked UNCONFIRMED unless independently corroborated.
- Several 2026-dated arXiv IDs and product facts sit in a fast-moving landscape; figures like ClawHavoc counts and skill-catalog sizes change weekly.
- The single most important takeaway: **this corpus contains a confirmed ~10-15% citation-integrity failure rate in the sample checked** (2 wrong papers, 2 title-mismatches, 1 misattribution out of ~42 IDs). Every remaining unverified claim should be treated as guilty until proven, not innocent until disproven.