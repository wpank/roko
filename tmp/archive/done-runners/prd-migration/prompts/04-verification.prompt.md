# Prompt: 04-verification

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/04-verification/`. This is the Harness layer (L3): Gate trait, gate implementations, 6-rung selector, gate pipeline, ratcheting, adaptive thresholds, process reward models, agent feedback, evaluation lifecycle, autonomous eval generation, EvoSkills, Forensic AI causal replay.

## Step 1 — Read context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order (00 through 06). These define naming, reframe rules, writing rules, source locations, output structure.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` §Layer 3 Harness, §Process Reward Models, §Conductor as Meta-Cognition
2. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §Gate trait (returns Verdict directly), §Cybernetic loops (Outcome → Scaffold / Routing / Knowledge / Affect / Prediction / Collective)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §IX Forensic AI Causal Replay (regulatory compliance table), §X EvoSkills adversarial verification
4. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`
5. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 2J (prediction tracking + calibration)

## Step 3 — SOURCE-INDEX entry `## 04-verification.md`

Read every file. Key legacy sources:
- `bardo-backup/prd/16-testing/00-thesis-validation.md`, `01-gauntlet.md`, `05-evaluation-lifecycle.md`, `07-fast-feedback-loops.md`, `08-slow-feedback-loops.md`, `09-evaluation-map.md`
- `bardo-backup/prd/25-mori/mori-quality-gates.md`
- `bardo-backup/tmp/mori-refactor/06-harness.md` — full harness layer spec
- `bardo-backup/tmp/mori-refactor/11-safety-observability-learning.md`
- `bardo-backup/tmp/mori-agents/06-eval-and-scoring.md`, `11-benchmarks-and-evals.md`, `20-verification-first-architecture.md`
- `bardo-backup/tmp/mori-refactor-plan/18-testing-and-ci.md`, `28-integration-testing-and-ci.md`
- `bardo-backup/tmp/death/16-autonomous-verification.md` (extract mechanism, drop mortality framing)
- `bardo-backup/tmp/agent-chain/17-autonomous-eval-generation.md`

## Step 4 — implementation-plans

- `modelrouting/12-advanced-patterns.md` — gate-to-scaffold feedback, section effectiveness tracking (lift > 0.05), process rewards
- `modelrouting/13-architectural-gaps.md` §H Generated Test Gates (GVU verification)
- `11-sections/phase-7-8.md` — PRD-driven workflow gate verification

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/**/*.rs`
- Read: `lib.rs`, `pipeline.rs`, `selector.rs`, `gates/` (compile.rs, test.rs, clippy.rs, diff.rs, symbol.rs, llm_judge.rs, etc.), `ratchet.rs`, `artifact_store.rs`, `adaptive_thresholds.rs`

## Step 6 — Output directory and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/04-verification
```

Write **13 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-gate-trait.md` | Full Gate trait signature (async, returns Verdict directly NOT Result<Verdict>). Why: gate failure isn't an error, it's a verdict. Errors represented as `Verdict::fail()` with error digest. name() method. |
| 01 | `01-gate-implementations.md` | All 11+ gates: CompileGate, TestGate, ClippyGate, DiffGate, SymbolGate, LlmJudgeGate, TxSimGate, WalletGate, VerifyChainGate, PropertyTestGate, GeneratedTestGate, ProcessRewardGate. Per-gate purpose, inputs, outputs, failure modes. |
| 02 | `02-6-rung-selector.md` | The 6-rung selector system. Fast rungs (compile, test) to slow rungs (LLM judge, full eval suite). Rung ordering and selection logic. |
| 03 | `03-gate-pipeline.md` | GatePipeline trait. VerifyChainGate (short-circuit chains). Fail-fast ordering. Verdict aggregation. Artifact propagation between gates. |
| 04 | `04-artifact-store.md` | Hash-addressed artifact storage. Gate outputs as artifacts. Deduplication. Persistence in `.roko/` directory. |
| 05 | `05-ratcheting.md` | Failure ratcheting: prevents regression. Gate stores best-ever test count. New results must match or exceed. Implementation details. Per-rung ratchet state. |
| 06 | `06-adaptive-thresholds.md` | EMA-based per-rung thresholds. Online adjustment. State persisted to `.roko/learn/gate-thresholds.json`. How thresholds interact with ratcheting. |
| 07 | `07-process-reward-models.md` | Inspired by OpenAI "Let's Verify Step by Step" (Lightman et al.) and AgentPRM. Score intermediate reasoning steps, not just outcomes. Promise + Progress scoring. Actions with low Promise trigger early intervention. Actions with negative Progress trigger re-planning. Generation-verification gap (Song et al., ICLR 2025). |
| 08 | `08-agent-feedback-from-gates.md` | How gate verdicts flow back to other traits: Scorer (source trust from outcome), Router (bandit arms from model performance), Composer (section weights from which context led to success — lift > 0.05), Daimon (PAD update from pass/fail), Neuro (tier promotion/demotion), self-model (capability boundaries). This is the cybernetic feedback loop — the system is self-regulating. |
| 09 | `09-evaluation-lifecycle.md` | Fast feedback (compile, test) → medium (integration, eval suites) → slow (regression detection, drift, multi-day eval). Per-rung latency budgets. Integration with the plan-execute-gate-persist loop. |
| 10 | `10-autonomous-eval-generation.md` | Autonomous eval generation: the system writes its own tests. EVMbench, DSPy Bayesian optimizers, Karpathy autoresearch loop. GVU verification from modelrouting/13-architectural-gaps.md §H. |
| 11 | `11-evoskills.md` | EvoSkills (April 2026): self-evolving skill libraries via adversarial surrogate verification. 5-round loop: generate bundles → Isolated Surrogate Verifier creates test assertions → skills that pass get promoted → failed skills mutated/retried → cross-pollination. Results: 32% → 75% (surpasses human-curated by round 3). Cross-model transfer: +35 to +44 pp across 6 models. A skill developed by a Claude agent works when executed by GPT. Implications for Korai knowledge economy — evolved skills as tradeable assets. |
| 12 | `12-forensic-ai-causal-replay.md` | The capability nobody else has: replay any agent action with full context. Which Engrams were in the Substrate. Which Scores computed. Which Router selection. Which Composer context. Which Gate verdict. Which Policy fired. All content-addressed. Replay is cryptographically verifiable via BLAKE3. Regulatory compliance mapping table: EU AI Act Art. 14 (human oversight) + FRIA (fundamental rights), SEC/CFTC (trading decision reconstruction), HIPAA (clinical audit trail), SOX (financial control documentation). Pre-certified agent templates (SEC-Compliant Trading Agent with MiFID II Policy, HIPAA-Compliant Clinical Agent with PHI-aware Gate, GDPR-Compliant Data Agent with purpose-limitation Policy). Enterprise value $100-500K/month per regulated enterprise. Certification moat. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per `context-pack/04-writing-rules.md`. ≥200 lines per sub-doc, ≥3500 total. Preserve citations: Meta-Harness Lee et al. 2026 (arXiv:2603.28052), Lightman et al. "Let's Verify Step by Step", AgentPRM, Song et al. ICLR 2025 (generation-verification gap), EvoSkills 2026, C2PA, EU AI Act Art. 14 and FRIA, SEC/CFTC, HIPAA, SOX, MiFID II. Minimum 15 citation-like patterns.

Cross-reference topics 00-architecture (Gate trait definition), 02-agents (outputs verified by gates), 05-learning (feedback from gates), 07-conductor (watches gate failures), 11-safety (overlap with verification).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- Gate trait returns `Verdict` directly, NOT `Result<Verdict>`. This is important. Errors are `Verdict::fail()` with error digest.
- Apply naming map: golem→agent; mori→Roko Orchestrator.
- No death framing.
- Use Write tool. Don't ask questions. Continue.
