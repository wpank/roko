//! Loop state machine and the six false-demotion guards (S03 §4.6; backlog 5114).
//!
//! [`Auditor::evaluate`] takes one loop's [`LoopStatus`] and the evidence
//! gathered since ([`LoopEvidence`]: the structural pre-checks, ε, ι and the
//! benefit estimates with their confidence sequences at the loop's α/K) and
//! applies S03 §4.6's rules cheapest first: the structural pre-checks, ε
//! below ε_min with the decomposed reason, `inert`, `live`, `harm`, `null`,
//! and re-probation after T_re, a version shift or a recorded repair. The
//! guards:
//!
//! 1. the confidence sequences are at α/K over the enforced loops
//!    ([`AuditParams::loop_alpha`]);
//! 2. β is judged only after N_β opportunities and with ε̂ ≥ ε_min, so a
//!    dormant loop is never `null` or `harm`;
//! 3. at most one transition per loop per dwell (24 h and 100 opportunities);
//! 4. `null` needs a narrow sequence after N_null opportunities;
//! 5. a placebo transition, an SRM alarm or an ordering violation on a clean
//!    loop sets `audit_broken`, which freezes every transition;
//! 6. `enforce = false` keeps the learned policy, and exempt loops are never
//!    demoted. `outcome_sensor_degraded` also freezes β-based transitions for
//!    loops whose outcome uses the false-green label.
//!
//! Each evaluation returns the state, reason, qualifiers and executed policy,
//! and the transition record with its rule and evidence when the state moved.
//! S03 §4.10's defaults are [`AuditParams::default`]; backlog 5122 maps
//! `[learning.audit]` onto them.

use serde::{Deserialize, Serialize};

use super::estimators::Estimate;
use super::exposure::{ExposureEstimate, InfluenceEstimate};
use super::spec::{AuditState, Qualifier, ReasonCode};

/// Seconds in an hour.
const HOUR: i64 = 3_600;

/// The auditor's parameters (S03 §4.10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuditParams {
    /// The family-wise α.
    pub alpha: f64,
    /// K, the enforced loops α is split over (at least 1).
    pub enforced_loops: usize,
    /// N_ε: learned-arm opportunities before ε is judged.
    pub n_eps: u64,
    /// ε_min.
    pub eps_min: f64,
    /// N_ι: opportunities before ι is judged.
    pub n_iota: u64,
    /// ι_min, net of the A/A floor.
    pub iota_min: f64,
    /// N_β: opportunities before β is judged.
    pub n_beta: u64,
    /// N_null: opportunities before a narrow null is declared.
    pub n_null: u64,
    /// δ_min: the half-width a null's sequence must fit inside.
    pub delta_min: f64,
    /// δ_ni: the non-inferiority margin on β_pass.
    pub delta_ni: f64,
    /// The dwell's time between transitions, in seconds.
    pub dwell_secs: i64,
    /// The dwell's opportunities between transitions.
    pub dwell_opportunities: u64,
    /// T_re: time in `flagged` or `demoted` before re-probation, in seconds.
    pub reprobation_secs: i64,
    /// The ι between state versions above which a loop re-enters probation.
    pub version_shift: f64,
    /// N∧: a stratum's opportunities before SPIBB lets it run learned.
    pub spibb_n: u64,
    /// The holdout rate h on probation.
    pub h_probation: f64,
    /// h once live.
    pub h_live: f64,
    /// h when flagged or demoted.
    pub h_suspect: f64,
}

impl Default for AuditParams {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            enforced_loops: 1,
            n_eps: 30,
            eps_min: 0.5,
            n_iota: 50,
            iota_min: 0.05,
            n_beta: 200,
            n_null: 800,
            delta_min: 0.03,
            delta_ni: 0.03,
            dwell_secs: 24 * HOUR,
            dwell_opportunities: 100,
            reprobation_secs: 14 * 24 * HOUR,
            version_shift: 0.2,
            spibb_n: 20,
            h_probation: 0.20,
            h_live: 0.05,
            h_suspect: 0.50,
        }
    }
}

impl AuditParams {
    /// α_ℓ = α/K, the level of each loop's confidence sequences.
    #[must_use]
    pub fn loop_alpha(&self) -> f64 {
        self.alpha / self.enforced_loops.max(1) as f64
    }

    /// The holdout rate a loop runs at in `state`.
    #[must_use]
    pub const fn holdout(&self, state: AuditState) -> f64 {
        match state {
            AuditState::Probation => self.h_probation,
            AuditState::Live => self.h_live,
            AuditState::Flagged | AuditState::Demoted => self.h_suspect,
        }
    }
}

/// The policy a loop's decisions execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutedPolicy {
    /// The learned policy (with its holdout).
    Learned,
    /// The default policy π⁰.
    Default,
}

/// Who moved a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    /// The auditor's rules.
    Auditor,
    /// A recorded human override.
    Human,
}

/// One loop's standing between evaluations.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopStatus {
    /// Its audit state.
    pub state: AuditState,
    /// The state's reason, if any.
    pub reason: Option<ReasonCode>,
    /// When it last moved (unix seconds); `None` until its first transition.
    pub last_transition_at: Option<i64>,
    /// Opportunities seen when it last moved.
    pub opportunities_at_transition: u64,
    /// Whether demotions change the executed policy.
    pub enforce: bool,
    /// Audited but never demoted.
    pub exempt: bool,
    /// The placebo loop: any transition of it breaks the auditor.
    pub placebo: bool,
}

impl LoopStatus {
    /// A newly registered loop, on probation.
    #[must_use]
    pub const fn registered(enforce: bool, exempt: bool, placebo: bool) -> Self {
        Self {
            state: AuditState::Probation,
            reason: None,
            last_transition_at: None,
            opportunities_at_transition: 0,
            enforce,
            exempt,
            placebo,
        }
    }

    /// Take `evaluation`'s state, and its transition's time and count.
    pub fn apply(&mut self, evaluation: &Evaluation, opportunities: u64) {
        self.state = evaluation.state;
        self.reason = evaluation.reason;
        if let Some(transition) = &evaluation.transition {
            self.last_transition_at = Some(transition.at);
            self.opportunities_at_transition = opportunities;
        }
    }
}

/// The structural pre-checks of a probation loop (S03 §4.6, first row).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Structural {
    /// Some decision was assigned at or after it was decided.
    pub ordering_violated: bool,
    /// The state's decision-relevant variance is 0.
    pub degenerate: bool,
    /// No opportunity arises on the executed path.
    pub no_opportunity: bool,
    /// The state's count or digest did not move over N_ε settled
    /// opportunities.
    pub no_learning: bool,
}

/// What the auditor knows of one loop at an evaluation. The sequences in it
/// are at the loop's α/K.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoopEvidence {
    /// The evaluation's time (unix seconds).
    pub now: i64,
    /// Opportunities so far, on both arms.
    pub opportunities: u64,
    /// The structural pre-checks.
    pub structural: Structural,
    /// ε and its decomposition.
    pub exposure: Option<ExposureEstimate>,
    /// ι_net.
    pub influence: Option<InfluenceEstimate>,
    /// β.
    pub beta: Option<Estimate>,
    /// β_pass.
    pub beta_pass: Option<Estimate>,
    /// β_cost.
    pub beta_cost: Option<Estimate>,
    /// The loop's outcome uses the false-green label.
    pub uses_false_green: bool,
    /// A repair was recorded since the last transition.
    pub repaired: bool,
    /// ι between the current and the previous state version, if measured.
    pub version_influence: Option<f64>,
    /// Qualifiers the evidence carries (`pre_instrumentation`, …).
    pub qualifiers: Vec<Qualifier>,
}

/// Auditor-level signals, outside any one loop (S03 §4.6, last row).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuditorSignals {
    /// A layer's SRM e-value crossed 1/α_srm.
    pub srm_alarm: bool,
    /// An ordering violation on a clean loop.
    pub ordering_on_clean_loop: bool,
    /// The M4 sensor's catch rate fell below its floor.
    pub outcome_sensor_degraded: bool,
}

/// A loop's move, with its rule and evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// The state it left.
    pub from: AuditState,
    /// The state it entered.
    pub to: AuditState,
    /// The new state's reason, if any.
    pub reason: Option<ReasonCode>,
    /// The rule that fired.
    pub rule: String,
    /// The evidence snapshot.
    pub evidence: String,
    /// Who moved it.
    pub actor: Actor,
    /// When (unix seconds).
    pub at: i64,
}

/// One evaluation's outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// The loop's state afterwards.
    pub state: AuditState,
    /// Its reason.
    pub reason: Option<ReasonCode>,
    /// Its qualifiers.
    pub qualifiers: Vec<Qualifier>,
    /// The policy its decisions execute afterwards.
    pub executed_policy: ExecutedPolicy,
    /// The holdout rate it runs at afterwards.
    pub holdout: f64,
    /// The move, if it moved.
    pub transition: Option<Transition>,
    /// The auditor is broken, so no loop moves.
    pub audit_broken: bool,
}

/// The loop auditor's rules and its broken flag.
#[derive(Debug, Clone)]
pub struct Auditor {
    params: AuditParams,
    broken: Option<String>,
}

impl Auditor {
    /// An auditor with `params`.
    #[must_use]
    pub const fn new(params: AuditParams) -> Self {
        Self {
            params,
            broken: None,
        }
    }

    /// Its parameters.
    #[must_use]
    pub const fn params(&self) -> &AuditParams {
        &self.params
    }

    /// Why the auditor is broken, if it is: every transition is frozen.
    #[must_use]
    pub fn broken(&self) -> Option<&str> {
        self.broken.as_deref()
    }

    /// Evaluate one loop. A tripwire in `signals`, or a placebo loop that
    /// would move, breaks the auditor; from then on no loop moves.
    pub fn evaluate(
        &mut self,
        status: &LoopStatus,
        evidence: &LoopEvidence,
        signals: &AuditorSignals,
    ) -> Evaluation {
        if self.broken.is_none() {
            if signals.srm_alarm {
                self.broken = Some("SRM e-value above 1/α_srm".to_string());
            } else if signals.ordering_on_clean_loop {
                self.broken = Some("ordering violation on a clean loop".to_string());
            }
        }
        let proposed = if self.broken.is_some() || self.dwelling(status, evidence) {
            None
        } else {
            self.rule(status, evidence, signals)
        };
        if let Some(proposal) = &proposed
            && status.placebo
        {
            self.broken = Some(format!("the placebo loop would move: {}", proposal.rule));
        }
        let transition = proposed.filter(|_| self.broken.is_none());
        let (state, reason) = transition
            .as_ref()
            .map_or((status.state, status.reason), |t| (t.to, t.reason));
        Evaluation {
            state,
            reason,
            qualifiers: evidence.qualifiers.clone(),
            executed_policy: executed_policy(status, state, reason),
            holdout: self.params.holdout(state),
            transition,
            audit_broken: self.broken.is_some(),
        }
    }

    /// Guard 3: the loop moved less than a dwell ago, in time or
    /// opportunities.
    fn dwelling(&self, status: &LoopStatus, evidence: &LoopEvidence) -> bool {
        status.last_transition_at.is_some_and(|at| {
            let since = evidence
                .opportunities
                .saturating_sub(status.opportunities_at_transition);
            evidence.now - at < self.params.dwell_secs || since < self.params.dwell_opportunities
        })
    }

    /// The first §4.6 rule that fires, cheapest first.
    fn rule(
        &self,
        status: &LoopStatus,
        evidence: &LoopEvidence,
        signals: &AuditorSignals,
    ) -> Option<Transition> {
        let p = &self.params;
        let n = evidence.opportunities;
        let move_to = |to, reason, rule: &str, detail: String| {
            Some(Transition {
                from: status.state,
                to,
                reason,
                rule: rule.to_string(),
                evidence: detail,
                actor: Actor::Auditor,
                at: evidence.now,
            })
        };
        let in_probation = status.state == AuditState::Probation;
        let audited = matches!(status.state, AuditState::Probation | AuditState::Live);
        if in_probation && let Some(code) = structural_reason(evidence.structural) {
            return move_to(
                AuditState::Flagged,
                Some(code),
                "structural pre-check",
                code.as_str().to_string(),
            );
        }
        let exposure = evidence.exposure.as_ref();
        let eps = exposure.map_or(0.0, |e| e.epsilon);
        if audited
            && let Some(exposure) = exposure
            && exposure.opportunities >= p.n_eps
            && let Some(code) = exposure.dormant_reason(p.eps_min)
        {
            let ucb = exposure.interval.map_or(exposure.epsilon, |(_, high)| high);
            return move_to(
                AuditState::Flagged,
                Some(code),
                "n_L ≥ N_ε and UCB(ε) < ε_min",
                format!(
                    "ε̂ {:.3}, UCB {ucb:.3} over {} opportunities",
                    exposure.epsilon, exposure.opportunities
                ),
            );
        }
        if audited
            && let Some(influence) = &evidence.influence
            && n >= p.n_iota
            && influence
                .interval
                .is_some_and(|(_, high)| high < p.iota_min)
        {
            return move_to(
                AuditState::Flagged,
                Some(ReasonCode::Inert),
                "n ≥ N_ι and UCB(ι_net) < ι_min",
                format!("ι_net {:.3}", influence.iota_net),
            );
        }
        // Guard 2 (and the M4 sensor's freeze): β only after N_β, with
        // exposure, and with a trusted outcome sensor.
        let judge_beta = n >= p.n_beta
            && eps >= p.eps_min
            && !(signals.outcome_sensor_degraded && evidence.uses_false_green);
        let interval = |estimate: &Option<Estimate>| estimate.as_ref().and_then(|e| e.interval);
        let lower = |estimate: &Option<Estimate>| interval(estimate).map(|(low, _)| low);
        let upper = |estimate: &Option<Estimate>| interval(estimate).map(|(_, high)| high);
        if judge_beta && in_probation {
            let beneficial = lower(&evidence.beta).is_some_and(|low| low > 0.0);
            let cheaper = lower(&evidence.beta_pass).is_some_and(|low| low > -p.delta_ni)
                && lower(&evidence.beta_cost).is_some_and(|low| low > 0.0);
            if beneficial || cheaper {
                return move_to(
                    AuditState::Live,
                    None,
                    "n ≥ N_β, ε̂ ≥ ε_min and LCB(β) > 0, or non-inferior and cheaper",
                    format!("ε̂ {eps:.3} over {n} opportunities"),
                );
            }
        }
        if judge_beta
            && audited
            && !status.exempt
            && upper(&evidence.beta).is_some_and(|high| high < 0.0)
        {
            return move_to(
                AuditState::Demoted,
                Some(ReasonCode::Harm),
                "n ≥ N_β and UCB(β) < 0",
                format!("UCB(β) {:.3}", upper(&evidence.beta).unwrap_or_default()),
            );
        }
        if judge_beta
            && in_probation
            && n >= p.n_null
            && lower(&evidence.beta).is_some_and(|low| low > -p.delta_min)
            && upper(&evidence.beta).is_some_and(|high| high < p.delta_min)
        {
            return move_to(
                AuditState::Flagged,
                Some(ReasonCode::Null),
                "n ≥ N_null and CS(β) ⊂ (−δ_min, δ_min)",
                format!("β inside ±{}", p.delta_min),
            );
        }
        let suspect = matches!(status.state, AuditState::Flagged | AuditState::Demoted);
        let expired = status
            .last_transition_at
            .is_some_and(|at| evidence.now - at >= p.reprobation_secs);
        let shifted = evidence
            .version_influence
            .is_some_and(|shift| shift > p.version_shift);
        if suspect && (expired || shifted || evidence.repaired) {
            return move_to(
                AuditState::Probation,
                None,
                "T_re elapsed, a version shift, or a recorded repair",
                format!(
                    "SPIBB: learned only in strata with ≥ {} opportunities",
                    p.spibb_n
                ),
            );
        }
        None
    }
}

/// The cheapest structural reason, if a pre-check fails.
fn structural_reason(checks: Structural) -> Option<ReasonCode> {
    let failing = [
        (checks.ordering_violated, ReasonCode::LabelOnly),
        (checks.degenerate, ReasonCode::Degenerate),
        (checks.no_opportunity, ReasonCode::NoOpportunity),
        (checks.no_learning, ReasonCode::NoLearning),
    ];
    ReasonCode::cheapest(
        failing
            .iter()
            .filter(|(fails, _)| *fails)
            .map(|(_, code)| *code),
    )
}

/// The policy a loop executes in `state` with `reason`: π⁰ for an enforced
/// demotion, `inert` or `null`; the learned policy otherwise, and always
/// with `enforce = false`. A dormant loop's decisions already equal π⁰'s.
fn executed_policy(
    status: &LoopStatus,
    state: AuditState,
    reason: Option<ReasonCode>,
) -> ExecutedPolicy {
    let demoted = state == AuditState::Demoted
        || matches!(reason, Some(ReasonCode::Inert | ReasonCode::Null));
    if status.enforce && !status.exempt && demoted {
        ExecutedPolicy::Default
    } else {
        ExecutedPolicy::Learned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A narrow estimate around `center` after `n` increments.
    fn estimate(center: f64, half_width: f64, n: u64) -> Estimate {
        Estimate {
            n,
            mean: center,
            variance: 0.0,
            interval: Some((center - half_width, center + half_width)),
        }
    }

    /// An exposure estimate with ε̂ `epsilon` over `n` learned-arm
    /// opportunities, its interval ±0.05.
    fn exposure(epsilon: f64, n: u64) -> ExposureEstimate {
        ExposureEstimate {
            opportunities: n,
            epsilon,
            read: 1.0,
            reach: 1.0,
            honest: 1.0,
            receipt: epsilon,
            interval: Some(((epsilon - 0.05).max(0.0), (epsilon + 0.05).min(1.0))),
            read_failures: (0, 0, 0),
        }
    }

    /// Evidence of a loop that reaches its decisions (ε̂ = 0.9) after `n`
    /// opportunities, with `beta` as β.
    fn exposed(n: u64, beta: Estimate) -> LoopEvidence {
        LoopEvidence {
            now: 0,
            opportunities: n,
            exposure: Some(exposure(0.9, n)),
            beta: Some(beta),
            ..LoopEvidence::default()
        }
    }

    /// Guard 2: a loop whose exposure is below ε_min never becomes `null`
    /// or `harm`, whatever its benefit sequence says, so its demotion
    /// cannot cost an outcome.
    #[test]
    fn dormant_loop_never_null_or_harm() {
        let mut auditor = Auditor::new(AuditParams::default());
        let status = LoopStatus::registered(true, false, false);
        for epsilon in [0.0, 0.2, 0.45] {
            for (n_learned, opportunities) in [(10, 900), (29, 1_000), (400, 2_000)] {
                for beta in [estimate(-0.4, 0.05, 2_000), estimate(0.0, 0.01, 2_000)] {
                    let evidence = LoopEvidence {
                        now: 0,
                        opportunities,
                        exposure: Some(exposure(epsilon, n_learned)),
                        beta: Some(beta),
                        ..LoopEvidence::default()
                    };
                    let signals = AuditorSignals::default();
                    let evaluation = auditor.evaluate(&status, &evidence, &signals);
                    assert_ne!(evaluation.state, AuditState::Demoted, "ε̂ {epsilon}");
                    assert!(
                        !matches!(evaluation.reason, Some(ReasonCode::Null | ReasonCode::Harm)),
                        "ε̂ {epsilon}: {evaluation:?}"
                    );
                }
            }
        }
        // With exposure, the same harmful sequence demotes it.
        let evaluation = auditor.evaluate(
            &status,
            &exposed(2_000, estimate(-0.4, 0.05, 2_000)),
            &AuditorSignals::default(),
        );
        assert_eq!(evaluation.state, AuditState::Demoted);
        assert_eq!(evaluation.reason, Some(ReasonCode::Harm));
        assert_eq!(evaluation.executed_policy, ExecutedPolicy::Default);
    }

    /// Guard 3: after a transition, no other one within 24 h or 100
    /// opportunities; once both have passed, the next rule fires.
    #[test]
    fn no_transition_within_dwell() {
        let mut auditor = Auditor::new(AuditParams::default());
        let mut status = LoopStatus::registered(true, false, false);
        let live = auditor.evaluate(
            &status,
            &exposed(300, estimate(0.2, 0.1, 300)),
            &AuditorSignals::default(),
        );
        assert_eq!(live.state, AuditState::Live, "{live:?}");
        status.apply(&live, 300);

        let harmful = |now: i64, opportunities: u64| LoopEvidence {
            now,
            ..exposed(opportunities, estimate(-0.3, 0.05, opportunities))
        };
        for (hours, extra) in [(1, 500), (30, 50)] {
            let evidence = harmful(hours * HOUR, 300 + extra);
            let held = auditor.evaluate(&status, &evidence, &AuditorSignals::default());
            assert!(held.transition.is_none(), "{hours} h, +{extra}: {held:?}");
            assert_eq!(held.state, AuditState::Live);
        }
        let evidence = harmful(25 * HOUR, 450);
        let moved = auditor.evaluate(&status, &evidence, &AuditorSignals::default());
        assert_eq!(moved.state, AuditState::Demoted, "{moved:?}");
    }

    /// Guard 5: a placebo loop that would move breaks the auditor instead,
    /// and then no loop moves, not even one whose evidence is conclusive.
    #[test]
    fn placebo_transition_freezes_auditor() {
        let mut auditor = Auditor::new(AuditParams::default());
        let placebo = LoopStatus::registered(false, false, true);
        let harmful = exposed(2_000, estimate(-0.3, 0.05, 2_000));
        let evaluation = auditor.evaluate(&placebo, &harmful, &AuditorSignals::default());
        assert!(evaluation.audit_broken);
        assert!(evaluation.transition.is_none());
        assert_eq!(evaluation.state, AuditState::Probation);
        assert!(auditor.broken().is_some_and(|why| why.contains("placebo")));

        let real = LoopStatus::registered(true, false, false);
        let frozen = auditor.evaluate(&real, &harmful, &AuditorSignals::default());
        assert!(
            frozen.audit_broken && frozen.transition.is_none(),
            "{frozen:?}"
        );

        let mut srm = Auditor::new(AuditParams::default());
        let signals = AuditorSignals {
            srm_alarm: true,
            ..AuditorSignals::default()
        };
        let alarmed = srm.evaluate(&real, &harmful, &signals);
        assert!(alarmed.audit_broken && alarmed.transition.is_none());
    }

    /// Guard 6: an exempt loop is audited but never demoted.
    #[test]
    fn exempt_loop_never_demoted() {
        let mut auditor = Auditor::new(AuditParams::default());
        let exempt = LoopStatus::registered(true, true, false);
        let evaluation = auditor.evaluate(
            &exempt,
            &exposed(2_000, estimate(-0.5, 0.05, 2_000)),
            &AuditorSignals::default(),
        );
        assert_ne!(evaluation.state, AuditState::Demoted, "{evaluation:?}");
        assert_eq!(evaluation.executed_policy, ExecutedPolicy::Learned);
    }

    /// Guard 6: with `enforce = false` a demotion is a flag only, and the
    /// loop keeps executing its learned policy.
    #[test]
    fn enforce_false_keeps_learned_policy() {
        let mut auditor = Auditor::new(AuditParams::default());
        let flags_only = LoopStatus::registered(false, false, false);
        let evaluation = auditor.evaluate(
            &flags_only,
            &exposed(2_000, estimate(-0.5, 0.05, 2_000)),
            &AuditorSignals::default(),
        );
        assert_eq!(evaluation.state, AuditState::Demoted);
        assert_eq!(evaluation.executed_policy, ExecutedPolicy::Learned);
        assert_eq!(evaluation.holdout, AuditParams::default().h_suspect);
    }

    /// The structural pre-checks fire cheapest first, ε below ε_min takes
    /// its decomposed reason, and a narrow null after N_null is `null`.
    #[test]
    fn rules_fire_cheapest_first() {
        let mut auditor = Auditor::new(AuditParams::default());
        let status = LoopStatus::registered(true, false, false);
        let structural = LoopEvidence {
            structural: Structural {
                degenerate: true,
                no_learning: true,
                ..Structural::default()
            },
            ..LoopEvidence::default()
        };
        let evaluation = auditor.evaluate(&status, &structural, &AuditorSignals::default());
        assert_eq!(evaluation.reason, Some(ReasonCode::Degenerate));

        let mut unlogged = exposure(0.0, 40);
        unlogged.receipt = 0.0;
        let evidence = LoopEvidence {
            opportunities: 40,
            exposure: Some(unlogged),
            ..LoopEvidence::default()
        };
        let evaluation = auditor.evaluate(&status, &evidence, &AuditorSignals::default());
        assert_eq!(
            evaluation.reason,
            Some(ReasonCode::Unlogged),
            "{evaluation:?}"
        );

        let null = auditor.evaluate(
            &status,
            &exposed(900, estimate(0.0, 0.01, 900)),
            &AuditorSignals::default(),
        );
        assert_eq!(null.reason, Some(ReasonCode::Null), "{null:?}");
        assert_eq!(null.executed_policy, ExecutedPolicy::Default);
    }
}
