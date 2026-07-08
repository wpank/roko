# Prompt: 20-technical-analysis

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/`. Covers the **generalized** Oracle trait (domain-agnostic predictive systems), chain oracles (price, volatility, MEV), coding oracles (build time, test failure, complexity drift — TA equivalents for code), research oracles, hyperdimensional TA, spectral liquidity manifolds, adaptive signal metabolism, causal microstructure discovery, predictive foraging, active inference state space, emergent multiscale intelligence. **Frame TA as universal oracle primitives with domain-specific instances, not as chain-only.**

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §4 Oracles & Predictive Systems (generalized trait, domain-specific oracles for Chain/Coding/Research, predictive foraging integration)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §VII Predictive Foraging + CalibrationTracker, §XIX.A Active Inference State Space (factorized POMDP)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §4 Active Inference Integration (EFE)

## Step 3 — SOURCE-INDEX entry `## 20-technical-analysis.md`

Key legacy:
- All of `bardo-backup/prd/23-ta/` (00-witness-as-TA, 01-hyperdimensional-TA, 02-spectral-liquidity-manifolds, 03-adaptive-signal-metabolism, 04-causal-microstructure-discovery, 05-predictive-geometry, 06-resonant-pattern-ecosystem, 07-defi-native-TA, 08-adversarial-signal-robustness, 09-somatic-TA, 10-emergent-multiscale-intelligence)
- `bardo-backup/tmp/agent-chain/10-predictive-foraging.md` — full predictive foraging spec
- `bardo-backup/tmp/agent-chain/14-academic-foundations.md` — 15 research traditions

## Step 4 — implementation-plans

- `modelrouting/12-advanced-patterns.md` — predictive foraging calibration, residual aggregation
- `12a-cognitive-layer.md` §J C-Factor metrics (information flow rate, turn-taking equality, knowledge integration, etc.)

## Step 5 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis
```

Write **14 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-ta-generalized.md` | TA is NOT chain-only. Universal Oracle primitives. Domain-specific instances. Why generalize: structural analogy across domains enables cross-domain insight. |
| 01 | `01-oracle-trait.md` | `Oracle` trait. Full Rust signature. `predict(query, ctx) -> Prediction`. `evaluate(prediction, outcome) -> PredictionAccuracy`. Prediction struct (value, confidence, horizon). |
| 02 | `02-chain-oracles.md` | Chain TA primitives: price prediction (moving averages, Bollinger bands, RSI), volatility estimation, gas price forecasting, liquidity depth analysis, MEV opportunity detection. |
| 03 | `03-coding-oracles.md` | **Coding equivalents of TA primitives.** Build time prediction (≈ price prediction). Test failure probability (≈ risk assessment). Complexity drift detection (≈ trend analysis). Dependency risk scoring (≈ portfolio risk). Performance regression forecasting (≈ volatility estimation). Why code is analogous to markets — both are structured, measurable, with feedback loops. |
| 04 | `04-research-oracles.md` | Research TA: source reliability estimation, information completeness assessment, contradiction detection across sources. Less mature than chain/coding oracles but the trait is the same. |
| 05 | `05-witness-as-ta-generalized.md` | Witness-as-technical-analyst concept generalized beyond chain. A witness is an observer that emits predictions + verifications. Any domain can have a witness. |
| 06 | `06-hyperdimensional-ta.md` | HDC for pattern matching across market data + code data + research data. Using 10,240-bit vectors to find structural similarity in time-series data. Cross-reference 06-neuro.md §08 (cross-domain HDC transfer). |
| 07 | `07-spectral-liquidity-manifolds.md` | From legacy 23-ta/02. Spectral decomposition of liquidity. Eigenmodes. Frequency-domain analysis. |
| 08 | `08-adaptive-signal-metabolism.md` | From 23-ta/03. How signals are "metabolized" — converted from raw data into decisions. Rate adaptation. Noise filtering. |
| 09 | `09-causal-microstructure-discovery.md` | From 23-ta/04. Pearl's causal models (cross-reference 10-dreams.md §03) applied to market microstructure. Discovering cause-effect relationships. Intervention analysis. |
| 10 | `10-predictive-geometry-and-resonant-patterns.md` | From 23-ta/05 and 06. Predictive geometry (geometric interpretation of price trajectories). Resonant pattern ecosystem. |
| 11 | `11-adversarial-signal-robustness.md` | From 23-ta/08. How the system handles adversarial manipulation of signals. Robustness guarantees. Cross-reference 11-safety.md. |
| 12 | `12-somatic-ta-and-emergent-multiscale.md` | From 23-ta/09-10. Somatic TA — intersection with Daimon (cross-reference 09-daimon.md). Emergent multiscale intelligence — cross-timescale pattern detection. |
| 13 | `13-predictive-foraging-and-active-inference.md` | Predictive foraging (from 09-innovations.md §VII): PredictionStore tracks predictions, residuals feed arithmetic corrector (~50ns), CalibrationTracker per (model, task_category). Active inference state space (from §XIX.A): factorized discrete POMDP (6 × 5 × 3 = 90 states tractable), A/B/C/D matrices from pymdp. Full EFE decomposition (pragmatic + epistemic - ambiguity). |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥3000 total. Citations: Kalman filter, Mallat wavelets, Damasio (for somatic TA), Friston Free Energy Principle, Pearl causality, MIDAS anomaly detection, Koudahl et al. 2024 active inference, Parr et al. 2024.

Cross-reference 00-architecture (Oracle as part of cognitive subsystems), 09-daimon (somatic TA intersection), 03-composition (active inference for context), 10-dreams (Pearl SCM for counterfactuals), 06-neuro (HDC patterns).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- **TA is generalized, not chain-only.** Lead with the universal Oracle trait, then domain-specific examples.
- Coding oracles are the counterpart to chain oracles — make this parallel explicit.
- Apply naming map: golem → agent.
- Use Write tool. Don't ask questions.
