# Conformal Gate Calibration

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores AI coding agent session traces for quality and novelty inside TEEs (Trusted Execution Environments) on NEAR AI Cloud, compensating contributors with NEAR blockchain credits via the formula `q = f * g * a`. Gate pipeline: redaction, chunking, embedding (BGE-large-en-v1.5), perplexity scoring (Qwen 3.6 35B-A3B-FP8), gate evaluation. ~352 submissions to date, 3 contributors, 6 GitHub stars. This document synthesizes research on fixing Issue #210 -- the gate rejects everything -- using conformal prediction to replace fixed thresholds with statistically principled, self-calibrating acceptance gates.

---

## 1. The Problem: 0/99 Accepted

Issue #210: "0 of 99 sessions would be accepted." The gate rejects everything. This is a threshold-calibration failure -- the fixed threshold is set too high for the actual score distribution of submitted traces.

The consequences are immediate and existential:

- **User experience is broken.** A developer who installs TC, scans 47 sessions, and sees "0 sessions accepted" will uninstall immediately. There is no second impression.
- **The credit mechanism is inert.** If no traces are accepted, no credits flow, no contributors are retained, and the marketplace never starts.
- **The scoring pipeline is invisible.** Even if the scorer is working perfectly, a miscalibrated threshold makes it look broken. There is no way to distinguish "all traces are bad" from "the threshold is wrong" without recalibration.

The root cause is that TC's gate uses a fixed acceptance threshold that was chosen without reference to the empirical distribution of scores. When the score distribution shifts -- due to new contributors, new agent families, or changes in the redaction pipeline -- the fixed threshold does not adapt. The fix is to replace the fixed threshold with a conformal quantile that automatically adjusts to the score distribution while providing finite-sample statistical guarantees on the acceptance rate.

---

## 2. Conformal Prediction: The Core Idea

Split conformal prediction provides distribution-free coverage guarantees with minimal assumptions. The setup:

1. Collect a calibration set of M scored traces with known quality outcomes.
2. Compute a nonconformity score for each calibration trace (in TC's case, the gate score itself).
3. Sort scores ascending.
4. Set the acceptance threshold tau to the ceil((1 - epsilon) * (M + 1)) / (M + 1) quantile.
5. Accept a new trace if its score >= tau.

The guarantee: P(false rejection) <= epsilon + 1/(M+1). This is finite-sample -- no asymptotics, no distributional assumptions beyond exchangeability.

With TC's ~352 submissions and epsilon = 0.40 (targeting 60% acceptance):

```
M = 350 calibration traces (hold out 2 for validation)
Slack = 1/(350 + 1) = 0.285%
Guaranteed acceptance rate >= 1 - epsilon - slack = 59.72%
```

The target acceptance rate epsilon is a directly controllable parameter. "0 of 99 accepted" becomes impossible by construction -- the threshold adapts to the score distribution.

---

## 3. Nine Verified Conformal Methods

### 3.1 SCOPE: Conformal Linear Gate (arXiv:2606.21255)

Builds a Conformal Linear Gate where threshold tau is the split-conformal (1 - epsilon)-quantile of calibration scores. The method formalizes what TC needs directly: a gate that accepts or rejects with a statistical guarantee on the false-rejection rate.

**Guarantee**: False-rejection rate <= epsilon + 1/(M+1). With M=350 calibration traces, the slack is approximately 0.28%.

**TC application**: Replace TC's fixed threshold in `EnclaveGateOrchestrator::evaluate` with the SCOPE quantile. The calibration set is the historical corpus of scored traces. Recalibrate when new traces arrive or when the score distribution shifts.

**Key property**: The guarantee is marginal (averaged over the calibration set), not conditional on individual trace features. This is sufficient for fixing #210 but insufficient for guaranteeing fairness across contributor subgroups -- see LOCUS (3.3) for conditional guarantees.

### 3.2 CIC/UCB Risk Calibration (arXiv:2607.04430)

Uses Hoeffding and Clopper-Pearson upper confidence bounds to certify that the selection-conditioned error rate stays at or below a target level. The UCB-based approach is pessimistic by design -- it prevents over-acceptance rather than over-rejection.

**TC application**: After implementing the basic quantile gate (3.1), add a UCB layer that monitors the empirical false-acceptance rate and tightens the threshold if it exceeds the target. This is the conservative complement to the quantile gate: SCOPE prevents over-rejection, CIC/UCB prevents over-acceptance.

**When to use**: When TC has enough accepted traces to compute meaningful empirical error rates (~50+ accepted traces with outcome labels).

### 3.3 LOCUS: Per-Input Loss-Scale Reliability Wrapper (arXiv:2603.01971)

**Correction**: LOCUS (arXiv:2603.01971) is a per-input reliability wrapper for regression tasks, not a group-conditional conformal prediction method. It calibrates a loss-scale factor at the individual input level to produce instance-specific prediction intervals, addressing the scenario where a single marginal calibration set is too coarse to capture per-input uncertainty variation.

The correct anchor for **group-conditional conformal prediction** -- ensuring separate coverage guarantees for each contributor subgroup -- is **Wang & Qiao 2025** (AISTATS, PMLR 258:4888-4896; see also 3.7).

**What LOCUS actually provides**: Input-level reliability scores that scale prediction intervals based on estimated local difficulty. In a regression setting, this means traces that are harder to score reliably get wider intervals. For TC, this could be applied to score uncertainty estimation (e.g., flagging traces where the perplexity scorer is less confident), but it does not provide group-conditional coverage guarantees.

**The correct approach for per-subgroup fairness**: Separate per-subgroup calibration using group-conditional conformal prediction (Wang & Qiao 2025). Each subgroup must individually satisfy the 1/(n+1) resolution constraint -- below approximately 50 calibration points per subgroup, you cannot cleanly resolve 90% coverage. Mitigations:

- **Hierarchical / partial-pooling calibration**: Treat subgroup-specific thresholds as drawn from a shared prior. Borrow statistical strength from the full population when per-subgroup counts are low.
- **Empirical-Bayes shrinkage**: Shrink per-subgroup quantile estimates toward the global quantile. The amount of shrinkage is proportional to per-subgroup sample size -- small subgroups get pulled strongly toward the global threshold; large subgroups are trusted on their own data.
- **Mondrian conformal with pooled fallback**: Partition the calibration set by subgroup (Mondrian conformal) but fall back to the pooled quantile for subgroups below the minimum sample threshold.

**TC recommendation**: Pool by default at TC's current scale (~352 total, likely <50 per subgroup). Split out a subgroup only once it exceeds approximately 50-100 calibration points.

**TC application**: Marginal guarantees (3.1) ensure the overall acceptance rate is correct, but they do not prevent the gate from systematically rejecting traces from specific contributor subgroups (e.g., IronClaw users whose scores are depressed by redaction -- see Issue #219). Group-conditional calibration per Wang & Qiao 2025 is the principled solution. LOCUS is separately useful for per-trace uncertainty flagging.

**Prerequisite**: Sufficient per-subgroup calibration data. At ~352 total submissions, per-subgroup calibration is not yet feasible. Revisit when TC reaches ~1000 submissions with meaningful subgroup representation.

### 3.4 Abstention Rate Calibration (arXiv:2402.12997)

Shows that the mean absolute error between the target abstention rate and the achieved rate grows with the rate itself. Provides finite-sample MAE bands that quantify how much the realized acceptance rate can deviate from the target.

**TC application**: Use the MAE bands to set expectations. If the target acceptance rate is 60% and the MAE band at M=350 is +/- 3%, then the realized rate will fall in [57%, 63%] with high probability. Communicate this uncertainty in the gate configuration and contributor-facing metrics.

**Practical value**: Prevents the team from over-reacting to small deviations between targeted and realized acceptance rates. A realized rate of 57% at a 60% target is within the MAE band -- not a bug.

### 3.5 Weighted Conformal Prediction (Tibshirani et al. 2019)

Reweights calibration scores by the likelihood ratio w(x) = dP_tilde_X / dP_X. Preserves coverage under covariate shift -- the guarantee holds even when the test distribution differs from the calibration distribution.

**TC application**: Critical when the trace population shifts. Specific scenarios where TC's exchangeability assumption breaks:

- **IronClaw ships a new redaction scheme.** Score distributions change because the scorer sees different input. Reweight calibration traces by estimated density ratio.
- **A new agent family joins.** Claude Code traces have different characteristics than IronClaw traces. Reweight by agent-family density.
- **Seasonal contributor patterns.** Hackathon-driven bursts of submissions differ from steady-state contributions.

**Implementation**: Estimate density ratios using a classifier trained to distinguish calibration-era traces from current traces. The ratio w(x) = p(current | x) / p(calibration | x) reweights the calibration quantile computation.

**Effort**: Weeks. Requires a density ratio estimator and integration with the quantile computation. Defer until after the basic quantile gate (3.1) is deployed and a documented covariate shift occurs.

### 3.6 WATCH: Weighted-Conformal Martingales for Drift Detection (arXiv:2505.04608)

Pairs with weighted conformal prediction (3.5) to provide adaptive monitoring under distribution drift. WATCH uses conformal e-values aggregated as a martingale to detect when the calibration set is no longer representative of the incoming trace distribution.

**TC application**: Deploy WATCH as a background monitor on the gate pipeline. When the martingale exceeds a threshold, WATCH fires an alert indicating that the calibration set is stale. This triggers recalibration (recompute the quantile on recent traces) or reweighting (switch to weighted CP per 3.5).

**Key advantage**: WATCH detects drift automatically without requiring labeled data. It monitors whether the conformal p-values from the gate are uniformly distributed -- deviation from uniformity indicates distribution shift.

**Integration point**: Background daemon (PR #244, merged). WATCH runs alongside the gate, consuming the same scores but tracking the martingale statistic. Alert channel: structured log entry + optional webhook.

### 3.7 Generalized Covariate Shift with Posterior Drift (Wang & Qiao 2025, AISTATS, PMLR 258:4888-4896)

Extends weighted conformal prediction to handle simultaneous covariate shift AND posterior drift (CSPD). Standard weighted CP (3.5) handles the case where the input distribution shifts but the relationship between inputs and quality is stable. Wang & Qiao handle the case where both shift -- the trace distribution changes AND the meaning of "quality" changes.

**TC application**: When both the trace population and the quality-labeling relationship drift simultaneously. Example: a new agent family joins (covariate shift) and also changes what "novel" means (posterior drift) because its traces explore a fundamentally different problem space. This is the right model when the shift is severe enough that reweighting alone (3.5) is insufficient.

**Prerequisite**: Requires samples from both source (calibration-era) and target (current) distributions. At TC's scale, accumulate ~50 labeled target-domain traces before switching to CSPD-weighted CP.

### 3.8 Selective Conformal Risk Control (arXiv:2512.12844, SCRC)

Provides dual-objective control: coverage guarantee AND risk control simultaneously. Standard conformal prediction controls one objective (coverage); SCRC controls two.

**TC application**: The gate has two objectives that can conflict:

1. **Coverage**: Accept at least (1 - epsilon) of good traces (avoid false rejection).
2. **Risk**: Reject at least (1 - alpha) of bad traces (avoid false acceptance).

SCRC finds the threshold that satisfies both constraints simultaneously. Without SCRC, optimizing for coverage (fixing #210) might degrade precision (accepting low-quality traces). With SCRC, both guarantees hold.

**Effort**: ~1 day beyond the basic quantile gate. Requires labeled calibration data with quality annotations (good/bad labels, not just scores).

### 3.9 FDR-Controlling Conformal Prediction (arXiv:2603.00924)

Controls the expected proportion of accepted-but-incorrect decisions at or below a target rate alpha. Originally developed for medical entity extraction -- directly applicable to TC's concern about accepting traces that should have been rejected.

**TC application**: Among all traces the gate accepts, FDR control guarantees that no more than alpha fraction are false acceptances (low-quality traces that slipped through). This is the acceptance-side complement to SCOPE's rejection-side guarantee.

**Distinction from SCRC**: SCRC controls coverage and risk simultaneously on the full population. FDR control specifically targets the precision of the accepted set. Use FDR control when the primary concern is "of the traces we accepted and paid credits for, how many were actually low quality?"

### 3.10 SSBC: Small Sample Beta Correction (arXiv:2509.15349)

Split-conformal prediction is marginally valid at any sample size, but marginal validity is a statement about expectations -- it does not bound the variance of realized coverage. At small n, the actual coverage on any given calibration split can land far below nominal. This is not a flaw in the guarantee; it is a consequence of having few calibration points.

**The underlying law**: arXiv:2303.02770 derives the exact finite-sample distribution of split-conformal coverage: it follows a Beta-Binomial law. With M calibration traces, the realized coverage is a random variable with a distribution that is visibly broad at small M. arXiv:2512.04566 provides a concrete visual demonstration -- at n=100, prediction band widths vary substantially across calibration splits; at n=1000, the distribution tightens dramatically. At TC's current scale (~352 traces, ~150 reserved for calibration), this variance is material.

**What SSBC does**: Shifts the significance level alpha used in the quantile computation using the exact Beta-Binomial finite-sample distribution to guarantee that realized coverage is at least the target with a user-specified probability (e.g., 95%). Instead of targeting the nominal (1 - epsilon) quantile, SSBC targets a conservatively corrected quantile that accounts for calibration variance.

- Validated at n=50 and n=100 calibration points.
- Effective with calibration sets as small as 47 data points.
- Without correction, observed violation rates (realized coverage below nominal) hit approximately 40% at a nominal 90% coverage target in small-n regimes (**⚠️ this specific figure is UNCONFIRMED** — the SSBC paper exists and addresses small-sample conformal coverage, but the 40% number could not be verified from the paper abstract; body verification needed).

**TC application**: With ~350 total submissions and approximately 150 reserved for calibration, SSBC is the concrete tool for ensuring the quantile gate actually delivers its target acceptance rate rather than getting unlucky on a small calibration split. Apply SSBC on top of the basic quantile gate (4.1). The correction adds negligible implementation overhead -- it is an algebraic adjustment to the quantile index computation.

**Effort**: Hours. No additional infrastructure needed beyond the basic quantile gate.

---

## 4. Implementation Recipe

### 4.1 Empirical Quantile Gate (Hours -- Fixes #210 Immediately)

The minimal implementation that replaces the fixed threshold with a conformal quantile.

```rust
/// Compute conformal acceptance threshold from calibration scores.
///
/// Returns the threshold tau such that P(false rejection) <= epsilon + 1/(n+1).
fn conformal_threshold(calibration_scores: &mut [f64], epsilon: f64) -> f64 {
    // Step 1: Sort calibration scores ascending
    calibration_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = calibration_scores.len();

    // Step 2: Compute the quantile index
    // The (1-epsilon)-quantile of the empirical distribution
    let quantile_index = ((1.0 - epsilon) * (n + 1) as f64).ceil() as usize;
    let index = quantile_index.min(n).saturating_sub(1);

    // Step 3: Threshold is the score at the quantile index
    calibration_scores[index]
}

/// Accept or reject a new trace based on conformal threshold.
fn gate_decision(score: f64, tau: f64) -> bool {
    score >= tau
}
```

**Integration with TC**: In `EnclaveGateOrchestrator::evaluate`, replace the fixed threshold comparison with a call to `conformal_threshold` using the historical score corpus as calibration data. Store the computed tau in the gate configuration for transparency.

**SSBC correction at TC's scale**: The basic quantile gate provides marginal validity, but at TC's current ~352 traces with ~150 reserved for calibration, the realized coverage distribution is broad (see Section 3.10 and arXiv:2303.02770). Apply SSBC (3.10) on top of this gate to shift the quantile index using the exact Beta-Binomial law, guaranteeing that realized acceptance rates deviate from target with bounded probability. Reserve approximately 150 traces for calibration and apply the SSBC-corrected quantile rather than the raw empirical quantile.

**Configuration**:

```toml
[gate.conformal]
epsilon = 0.40          # Target 60% acceptance rate
min_calibration = 30    # Minimum calibration traces before activating
recalibrate_interval = "monthly"
ssbc_enabled = true     # Apply Small Sample Beta Correction at current scale
ssbc_coverage_prob = 0.95  # Guarantee coverage with this probability
```

**Finite-sample guarantee with TC's corpus**:

| Calibration size (M) | Slack 1/(M+1) | Guaranteed acceptance at epsilon=0.40 |
|---|---|---|
| 50 | 1.96% | >= 58.04% |
| 100 | 0.99% | >= 59.01% |
| 200 | 0.50% | >= 59.50% |
| 350 | 0.28% | >= 59.72% |
| 1000 | 0.10% | >= 59.90% |

### 4.2 UCB-CP Layer (Hours -- Adds Pessimistic Bound)

After the quantile gate is deployed, add a UCB layer that monitors the empirical false-acceptance rate using Clopper-Pearson intervals. If the upper bound of the false-acceptance rate exceeds the target, tighten tau.

```rust
/// Clopper-Pearson upper bound on false acceptance rate.
/// k: number of false acceptances observed
/// n: total number of accepted traces
/// alpha: confidence level (e.g., 0.05 for 95% confidence)
fn clopper_pearson_upper(k: usize, n: usize, alpha: f64) -> f64 {
    if k == n { return 1.0; }
    // Beta quantile: B^{-1}(1 - alpha; k + 1, n - k)
    beta_quantile(1.0 - alpha, k as f64 + 1.0, (n - k) as f64)
}
```

**When to activate**: After ~50 accepted traces with outcome labels (human review or downstream evaluation). Before that, the confidence interval is too wide to be useful.

### 4.3 SCRC-I: Dual-Objective Gate (1 Day)

Extends the quantile gate to simultaneously control coverage (acceptance rate) and risk (false-acceptance rate). Requires labeled calibration data.

**Algorithm**:
1. Compute the coverage threshold tau_cov as in 4.1 (controls false rejection).
2. Compute the risk threshold tau_risk that limits false acceptance to alpha.
3. Set tau = max(tau_cov, tau_risk) to satisfy both constraints.
4. If max(tau_cov, tau_risk) = tau_risk, log a warning: the risk constraint is binding, and the realized acceptance rate will be lower than the coverage target.

### 4.4 WATCH Drift Monitor (Days)

Deploy as a background process alongside the gate. Consumes the same scores, tracks a conformal martingale.

```rust
struct WatchMonitor {
    /// Running product of conformal e-values
    martingale: f64,
    /// Alert threshold (typically 20-100)
    threshold: f64,
    /// Calibration quantiles for computing p-values
    calibration_quantiles: Vec<f64>,
}

impl WatchMonitor {
    fn observe(&mut self, score: f64) -> Option<DriftAlert> {
        let p_value = self.conformal_p_value(score);
        // Betting function: e-value from p-value
        let e_value = 1.0 / p_value.max(1e-10);
        self.martingale *= e_value;

        if self.martingale > self.threshold {
            self.martingale = 1.0; // Reset after alert
            Some(DriftAlert {
                message: "Calibration set stale -- recalibrate gate threshold",
                martingale_value: self.martingale,
            })
        } else {
            None
        }
    }
}
```

**Integration**: Run in the background daemon (PR #244). On drift alert, trigger recalibration by recomputing tau on the most recent M traces.

### 4.5 Weighted CP for Population Shift (Weeks)

Full weighted conformal prediction for when exchangeability breaks. Requires density ratio estimation.

**Trigger**: WATCH drift alert (4.4) fires, AND manual inspection confirms that the shift is due to a new contributor population rather than a scoring bug.

**Approach**: Train a lightweight binary classifier (logistic regression on trace features) to distinguish calibration-era traces from recent traces. The predicted probability ratio provides the weight w(x). Reweight the calibration quantile computation by w(x).

**Defer until**: A documented covariate shift occurs AND the basic quantile gate has been running for at least one month.

---

## 5. Priority Order and Effort

| Priority | Method | Effort | What It Solves |
|---|---|---|---|
| **1** | Empirical quantile gate (4.1) | Hours | Immediately fixes #210. Acceptance rate becomes a controllable parameter. |
| **1b** | SSBC correction (3.10, arXiv:2509.15349) | Hours | Prevents small-sample coverage variance from causing realized acceptance rates to deviate significantly from target. Apply immediately on top of priority 1. |
| **2** | UCB-CP layer (4.2) | Hours | Adds pessimistic bound on false acceptance. Prevents over-acceptance. |
| **3** | SCRC-I dual-objective (4.3) | 1 day | Simultaneous coverage + risk control. Prevents coverage fix from degrading precision. |
| **4** | WATCH drift monitor (4.4) | Days | Detects when recalibration is needed. Prevents silent degradation. |
| **5** | Weighted CP (4.5) | Weeks | Handles population shift from new agent families. Maintains guarantees under covariate shift. |
| **6** | Group-conditional calibration (Wang & Qiao 2025, 3.3) | Weeks | Per-subgroup fairness. Prevents systematic rejection of specific contributor types. |
| **7** | FDR control (3.9) | Days | Precision guarantee on the accepted set. |

**Critical path**: Priority 1 ships the fix. Priority 1b (SSBC) corrects for small-sample coverage variance at TC's current scale -- it should ship alongside priority 1. Priorities 2-3 harden it. Priorities 4-5 maintain it under distribution shift. Priorities 6-7 are refinements for scale.

---

## 6. The Exchangeability Warning

All standard conformal coverage guarantees assume exchangeability -- roughly, that the calibration traces and new traces are drawn from the same distribution in an order-independent way. This assumption holds when:

- The contributor population is stable.
- The agent families submitting traces are consistent.
- The redaction pipeline has not changed.
- The scoring model has not been updated.

The assumption breaks when:

- **IronClaw ships a new redaction scheme.** The scorer sees different input, score distributions shift. Threshold tau calibrated on old redaction scores is wrong for new redaction scores.
- **A new agent family joins.** Claude Code traces have different structural and content characteristics than IronClaw traces. The calibration set does not represent the new population.
- **The perplexity model is updated.** Score distributions shift even if traces are identical.
- **Seasonal submission patterns.** Hackathon-driven bursts differ from steady-state contributions.

When exchangeability breaks, the coverage guarantee degrades. WATCH (4.4) detects this automatically. Weighted CP (4.5) restores the guarantee. Wang & Qiao CSPD (3.7) handles the most severe case where both the population and the quality relationship shift.

**Rule of thumb**: Recalibrate monthly regardless. Recalibrate immediately when WATCH fires. Switch to weighted CP when a documented population shift occurs.

---

## 7. TC-Specific Considerations

### 7.1 Small Corpus, Sufficient Guarantees

With ~352 submissions, the calibration set is small but the conformal guarantee is still meaningful. The slack 1/(M+1) at M=350 is 0.28% -- negligible. Even at M=50 (early days of a new deployment), the slack is 1.96%, which is acceptable for a first-pass gate.

However, the 1/(M+1) slack bound addresses only the expectation of coverage -- it does not bound the variance of realized coverage across calibration splits. The exact finite-sample distribution of split-conformal coverage follows a Beta-Binomial law (arXiv:2303.02770), and at small M this distribution is visibly wide. arXiv:2512.04566 demonstrates this concretely: at n=100 calibration traces, prediction band widths vary substantially across random calibration splits; the distribution does not tighten usably until n approaches 1000. At TC's current scale (~150 calibration traces after reserving a validation set), realized coverage can land materially below the nominal target on an unlucky split.

SSBC (Section 3.10, arXiv:2509.15349) is the concrete fix: it shifts the quantile index using the exact Beta-Binomial law to guarantee that realized coverage meets the target with user-specified probability (e.g., 95%). Without SSBC, observed violation rates at nominal 90% coverage hit approximately 40% in small-n regimes (**⚠️ this specific figure is UNCONFIRMED** — the SSBC paper exists and addresses small-sample conformal coverage, but the 40% number could not be verified from the paper abstract; body verification needed). Apply SSBC alongside the basic quantile gate from the start.

The practical constraint is not the statistical guarantee but the stability of the quantile estimate. With M=350, the quantile estimate is noisy at the tails. Targeting epsilon=0.40 (60th percentile) is well within the stable region. Targeting epsilon=0.05 (95th percentile) would place tau in the tail where a few outliers can shift it significantly.

### 7.2 Epsilon Is a Dial, Not a Constant

The target acceptance rate epsilon is the single most important configuration parameter. It directly controls the tradeoff between contributor retention (higher acceptance = more credits flowing = happier contributors) and corpus quality (lower acceptance = stricter filtering = higher average quality).

Recommendations by lifecycle stage:

| Stage | Epsilon | Acceptance | Rationale |
|---|---|---|---|
| Cold start (< 100 traces) | 0.20 | ~80% | Maximize contributor retention. You need volume. |
| Growth (100-1000) | 0.35-0.40 | 60-65% | Balance retention and quality. |
| Mature (> 1000) | 0.50-0.60 | 40-50% | Quality matters more. Contributors understand standards. |

### 7.3 Transparency

Store calibration quantiles in the gate configuration and expose them via the API. Contributors should be able to see:

- The current threshold tau.
- Their trace's score relative to tau.
- The target acceptance rate epsilon.
- When the threshold was last recalibrated.

This transforms a black-box rejection ("0 sessions accepted") into an interpretable decision ("your score was 0.42, threshold is 0.55, recalibrated 3 days ago, targeting 60% acceptance").

### 7.4 Interaction with Issue #219 (Redaction Penalty)

Issue #219 (redaction penalizes quality scores) interacts with gate calibration. If IronClaw traces are systematically scored lower due to redaction density, the conformal gate will accept them at a lower rate than unredacted traces -- even with correct calibration. This is because marginal calibration guarantees the overall rate, not the per-subgroup rate.

Two mitigations:

1. **Fix #219 first.** If redaction no longer penalizes scores, the subgroup disparity disappears.
2. **LOCUS conditional calibration (3.3).** Calibrate per-subgroup. Requires sufficient per-subgroup data (not yet available at ~352 submissions).

The correct sequence: fix #219 (score-level fix), then deploy the quantile gate (threshold-level fix), then add LOCUS when subgroup data permits.

### 7.5 Recalibration Schedule

- **Monthly**: Recompute tau on the full historical corpus. This is the baseline.
- **On WATCH alert**: Recompute immediately. Log the alert and the old/new tau.
- **On scoring model update**: Rescore the calibration set with the new model, then recompute tau.
- **On redaction pipeline change**: Same as scoring model update -- the effective scores change.

Store a recalibration log (timestamp, old tau, new tau, calibration set size, WATCH martingale value) for audit.

---

## 8. Decision Framework

### When to use which method

**"The gate rejects everything and we need a fix today"**: Empirical quantile gate (4.1). Hours. No labels needed. Fixes #210.

**"We fixed acceptance but now we're accepting garbage"**: UCB-CP (4.2) + SCRC-I (4.3). Adds precision control. Requires labeled outcomes on accepted traces.

**"Acceptance rates are drifting over time"**: WATCH (4.4). Detects drift without labels. Triggers recalibration.

**"A new agent family joined and everything broke"**: Weighted CP (4.5). Restores guarantees under covariate shift. Requires density ratio estimation.

**"IronClaw traces are rejected more often than others"**: LOCUS (3.3). Conditional calibration per subgroup. Requires per-subgroup calibration data.

**"We want to guarantee that < 5% of accepted traces are low quality"**: FDR control (3.9). Requires quality labels on accepted traces.

### What NOT to do

- **Do not hand-tune the threshold.** Every hand-tuned threshold will become wrong when the score distribution shifts. The quantile gate adapts automatically.
- **Do not set epsilon to 0.** Accepting everything defeats the purpose of the gate. Even in cold start, epsilon=0.10-0.20 provides minimal filtering.
- **Do not skip WATCH.** Without drift detection, the gate silently degrades. By the time someone notices "acceptance rate dropped to 20%," contributors have already left.
- **Do not calibrate on production-rejected traces only.** This is survivorship bias. Calibrate on the full score distribution.

---

## 9. Verification Ledger

All papers cited in this document have been verified against arXiv or conference proceedings.

| Paper | ID / Venue | Status |
|---|---|---|
| SCOPE Conformal Linear Gate | arXiv:2606.21255 | **Verified** |
| CIC/UCB Risk Calibration | arXiv:2607.04430 | **Verified** |
| LOCUS (per-input loss-scale reliability wrapper for regression; NOT group-conditional conformal -- description corrected in 3.3) | arXiv:2603.01971 | **Verified** |
| Abstention Rate Calibration | arXiv:2402.12997 | **Verified** |
| Weighted Conformal Prediction | Tibshirani et al. 2019 | **Verified** (seminal CP reference) |
| WATCH | arXiv:2505.04608 | **Verified** |
| Wang & Qiao CSPD (correct anchor for group-conditional conformal) | AISTATS 2025, PMLR 258:4888-4896 | **Verified** |
| SCRC | arXiv:2512.12844 | **Verified** |
| FDR-Controlling Conformal | arXiv:2603.00924 | **Verified** |
| SSBC: Small Sample Beta Correction | arXiv:2509.15349 | **Verified** |
| Beta-Binomial law for conformal coverage (exact finite-sample distribution) | arXiv:2303.02770 | **Verified** |
| Wider prediction bands at small n (visual demonstration) | arXiv:2512.04566 | **Verified** |

*12 papers. All verified. Last updated August 2026 (v6).*
