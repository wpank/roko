//! Five-head lexicographic corrigibility ordering (Nayebi 2024).
//!
//! Every agent decision passes through a 5-head ordered check. Heads are
//! evaluated in strict priority order; a higher-priority veto always wins
//! regardless of lower-priority head outcomes.
//!
//! Head priority (1 = highest):
//! 1. [`CorrigibilityHead::Deference`] — obey stated human preferences
//! 2. [`CorrigibilityHead::Switch`]    — preserve human ability to intervene
//! 3. [`CorrigibilityHead::Truth`]     — represent information accurately
//! 4. [`CorrigibilityHead::Impact`]    — minimize unintended side effects
//! 5. [`CorrigibilityHead::Task`]      — accomplish the assigned task

use std::cmp::Ordering;

/// The five corrigibility heads in strict priority order.
///
/// Variants are defined from highest to lowest priority so that the derived
/// [`Ord`] impl orders them correctly (Deference < Switch < … at the type
/// level, but semantically Deference has the *highest* precedence and is
/// therefore evaluated first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CorrigibilityHead {
    /// Priority 1 (highest): obey the human's stated preferences and constraints.
    Deference,
    /// Priority 2: preserve the human's ability to change the agent's behavior.
    Switch,
    /// Priority 3: represent information accurately; do not deceive.
    Truth,
    /// Priority 4: minimize unintended side effects; prefer reversibility.
    Impact,
    /// Priority 5 (lowest): accomplish the assigned task effectively.
    Task,
}

impl CorrigibilityHead {
    /// Return the numeric priority (1 = highest, 5 = lowest).
    #[must_use]
    pub fn priority(&self) -> u8 {
        match self {
            Self::Deference => 1,
            Self::Switch => 2,
            Self::Truth => 3,
            Self::Impact => 4,
            Self::Task => 5,
        }
    }

    /// All heads in evaluation order (highest priority first).
    pub fn all_in_order() -> [CorrigibilityHead; 5] {
        [
            Self::Deference,
            Self::Switch,
            Self::Truth,
            Self::Impact,
            Self::Task,
        ]
    }
}

/// The verdict of a single corrigibility head evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadVerdict {
    /// The head is satisfied; evaluation continues to the next head.
    Pass,
    /// The head is not satisfied; evaluation stops and the action is blocked.
    Veto(String),
}

impl HeadVerdict {
    /// Returns `true` if this verdict is a veto.
    #[must_use]
    pub fn is_veto(&self) -> bool {
        matches!(self, Self::Veto(_))
    }

    /// Returns the veto reason, or `None` if this verdict is a pass.
    #[must_use]
    pub fn veto_reason(&self) -> Option<&str> {
        match self {
            Self::Veto(reason) => Some(reason.as_str()),
            Self::Pass => None,
        }
    }
}

/// The full result of evaluating all five corrigibility heads.
///
/// Verdicts are stored in head-priority order (Deference first, Task last).
/// Evaluation short-circuits on the first veto: once a head vetoes, lower-
/// priority heads are not evaluated and their verdicts are omitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrigibilityDecision {
    /// Per-head verdicts in evaluation order.
    pub verdicts: Vec<(CorrigibilityHead, HeadVerdict)>,
}

impl CorrigibilityDecision {
    /// Build a decision from a list of (head, verdict) pairs.
    #[must_use]
    pub fn new(verdicts: Vec<(CorrigibilityHead, HeadVerdict)>) -> Self {
        Self { verdicts }
    }

    /// Returns `true` if any head vetoed the action.
    #[must_use]
    pub fn is_vetoed(&self) -> bool {
        self.verdicts.iter().any(|(_, v)| v.is_veto())
    }

    /// Returns the first veto, if any: the (head, reason) pair.
    #[must_use]
    pub fn first_veto(&self) -> Option<(CorrigibilityHead, &str)> {
        self.verdicts
            .iter()
            .find_map(|(head, verdict)| verdict.veto_reason().map(|reason| (*head, reason)))
    }

    /// Returns `true` if the action is allowed (no head vetoed).
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        !self.is_vetoed()
    }
}

/// Compare two [`CorrigibilityDecision`]s lexicographically by head priority.
///
/// A decision that is vetoed at a higher-priority head is considered "worse"
/// (Greater in the sense that it fails more severely). Two decisions are
/// Equal only if they have identical verdicts at every head that both evaluated.
///
/// Comparison semantics:
/// - `Ordering::Less`    — `a` is *more corrigible* than `b` (fewer/lower-priority vetoes)
/// - `Ordering::Equal`   — `a` and `b` have the same corrigibility profile
/// - `Ordering::Greater` — `a` is *less corrigible* than `b` (veto at higher priority)
#[must_use]
pub fn lexicographic_compare(a: &CorrigibilityDecision, b: &CorrigibilityDecision) -> Ordering {
    // Walk heads in priority order.  At the first head where they differ,
    // the decision with a Veto is Greater (worse).
    for head in CorrigibilityHead::all_in_order() {
        let a_verdict = a.verdicts.iter().find(|(h, _)| *h == head).map(|(_, v)| v);
        let b_verdict = b.verdicts.iter().find(|(h, _)| *h == head).map(|(_, v)| v);

        match (a_verdict, b_verdict) {
            (Some(HeadVerdict::Veto(_)), Some(HeadVerdict::Veto(_))) => continue,
            (Some(HeadVerdict::Veto(_)), _) => return Ordering::Greater,
            (_, Some(HeadVerdict::Veto(_))) => return Ordering::Less,
            // Both pass, or one/both absent (treat absence as pass).
            _ => continue,
        }
    }
    Ordering::Equal
}

/// Evaluation context for `evaluate_action`.
///
/// All fields are optional free-form strings; the pure evaluator uses simple
/// heuristics to produce verdicts without IO or LLM calls.
#[derive(Debug, Clone, Default)]
pub struct ActionContext {
    /// The agent's declared autonomy level ("observe", "assist", "auto").
    pub autonomy_level: Option<String>,
    /// Whether the action is reversible (caller-supplied hint).
    pub reversible: Option<bool>,
    /// Whether the action modifies audit or logging infrastructure.
    pub modifies_audit: Option<bool>,
    /// Whether the action's outputs are verifiable without running them.
    pub outputs_verifiable: Option<bool>,
    /// Whether the action makes progress toward the current task goal.
    pub on_task: Option<bool>,
}

/// Evaluate a proposed action against all five corrigibility heads.
///
/// This is a **pure, synchronous** function. It applies lightweight
/// heuristics to the action description and context. For production use,
/// each head should be replaced by a dedicated Verify Cell; this function
/// provides the decision model and ordering semantics.
///
/// Evaluation short-circuits on the first veto.
#[must_use]
pub fn evaluate_action(action_description: &str, context: &ActionContext) -> CorrigibilityDecision {
    let mut verdicts = Vec::with_capacity(5);

    // Head 1: Deference — is the action within the declared autonomy level?
    let deference_verdict = evaluate_deference(action_description, context);
    let deference_vetoed = deference_verdict.is_veto();
    verdicts.push((CorrigibilityHead::Deference, deference_verdict));
    if deference_vetoed {
        return CorrigibilityDecision::new(verdicts);
    }

    // Head 2: Switch — does the action preserve the ability to intervene?
    let switch_verdict = evaluate_switch(action_description, context);
    let switch_vetoed = switch_verdict.is_veto();
    verdicts.push((CorrigibilityHead::Switch, switch_verdict));
    if switch_vetoed {
        return CorrigibilityDecision::new(verdicts);
    }

    // Head 3: Truth — does the action report accurately?
    let truth_verdict = evaluate_truth(action_description, context);
    let truth_vetoed = truth_verdict.is_veto();
    verdicts.push((CorrigibilityHead::Truth, truth_verdict));
    if truth_vetoed {
        return CorrigibilityDecision::new(verdicts);
    }

    // Head 4: Impact — are side effects bounded and reversible?
    let impact_verdict = evaluate_impact(action_description, context);
    let impact_vetoed = impact_verdict.is_veto();
    verdicts.push((CorrigibilityHead::Impact, impact_verdict));
    if impact_vetoed {
        return CorrigibilityDecision::new(verdicts);
    }

    // Head 5: Task — does the action make progress toward the goal?
    let task_verdict = evaluate_task(action_description, context);
    verdicts.push((CorrigibilityHead::Task, task_verdict));

    CorrigibilityDecision::new(verdicts)
}

// ── Internal head evaluators ──────────────────────────────────────────────────

fn evaluate_deference(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    // If the autonomy level is "observe" only, any action that modifies state
    // violates Deference.  Lightweight heuristic: flag actions explicitly
    // marked as non-observe when observe mode is set.
    if let Some(ref level) = ctx.autonomy_level {
        if level == "observe" && ctx.reversible == Some(false) {
            return HeadVerdict::Veto(
                "action modifies state but autonomy_level is 'observe'".into(),
            );
        }
    }
    HeadVerdict::Pass
}

fn evaluate_switch(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    if ctx.modifies_audit == Some(true) {
        return HeadVerdict::Veto(
            "action modifies audit/logging infrastructure, reducing human oversight".into(),
        );
    }
    HeadVerdict::Pass
}

fn evaluate_truth(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    if ctx.outputs_verifiable == Some(false) {
        return HeadVerdict::Veto(
            "action outputs cannot be independently verified — potential deception risk".into(),
        );
    }
    HeadVerdict::Pass
}

fn evaluate_impact(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    if ctx.reversible == Some(false) {
        return HeadVerdict::Veto("action has irreversible side effects".into());
    }
    HeadVerdict::Pass
}

fn evaluate_task(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    if ctx.on_task == Some(false) {
        return HeadVerdict::Veto("action does not make progress toward the assigned task".into());
    }
    HeadVerdict::Pass
}

// ── CorrigibilityScore (numeric summary) ─────────────────────────────────────

/// A numeric summary of an agent's corrigibility across five axes.
///
/// Each factor is a value in `[0.0, 1.0]` where 1.0 is fully corrigible
/// (maximum safety) and 0.0 is fully non-corrigible on that axis.
///
/// Factors map to the five heads in priority order:
/// - `safety_compliance`  → Deference
/// - `human_alignment`    → Switch
/// - `transparency`       → Truth
/// - `reversibility`      → Impact
/// - `predictability`     → Task
///
/// Lexicographic ordering is implemented via [`Ord`]: factors are compared
/// in priority order (safety_compliance first, predictability last). A higher
/// score on a higher-priority factor always dominates a lower-priority factor.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrigibilityScore {
    /// (Head 1 — Deference) Degree to which the agent obeys stated constraints.
    pub safety_compliance: f64,
    /// (Head 2 — Switch) Degree to which the agent preserves human override ability.
    pub human_alignment: f64,
    /// (Head 3 — Truth) Degree to which the agent is transparent in its reporting.
    pub transparency: f64,
    /// (Head 4 — Impact) Degree to which the agent's actions are reversible.
    pub reversibility: f64,
    /// (Head 5 — Task) Degree to which the agent's behavior is predictable.
    pub predictability: f64,
}

impl CorrigibilityScore {
    /// Create a new score, clamping all values to `[0.0, 1.0]`.
    #[must_use]
    pub fn new(
        safety_compliance: f64,
        human_alignment: f64,
        transparency: f64,
        reversibility: f64,
        predictability: f64,
    ) -> Self {
        Self {
            safety_compliance: safety_compliance.clamp(0.0, 1.0),
            human_alignment: human_alignment.clamp(0.0, 1.0),
            transparency: transparency.clamp(0.0, 1.0),
            reversibility: reversibility.clamp(0.0, 1.0),
            predictability: predictability.clamp(0.0, 1.0),
        }
    }

    /// Return the overall corrigibility level derived from this score.
    #[must_use]
    pub fn level(&self) -> CorrigibilityLevel {
        // The most safety-critical head (safety_compliance) determines the
        // worst-case level; other heads can only pull the level up, not down.
        let min_score = self
            .safety_compliance
            .min(self.human_alignment)
            .min(self.transparency)
            .min(self.reversibility)
            .min(self.predictability);

        CorrigibilityLevel::from_score(min_score)
    }

    /// Convert each factor to a discrete integer bucket (0–100) for ordering.
    fn as_ord_tuple(&self) -> (u64, u64, u64, u64, u64) {
        fn bucket(v: f64) -> u64 {
            (v * 1_000_000.0) as u64
        }
        (
            bucket(self.safety_compliance),
            bucket(self.human_alignment),
            bucket(self.transparency),
            bucket(self.reversibility),
            bucket(self.predictability),
        )
    }
}

impl Eq for CorrigibilityScore {}

impl PartialOrd for CorrigibilityScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Lexicographic ordering: safety_compliance dominates, then human_alignment, etc.
impl Ord for CorrigibilityScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ord_tuple().cmp(&other.as_ord_tuple())
    }
}

// ── CorrigibilityLevel ────────────────────────────────────────────────────────

/// A discrete classification of an agent's overall corrigibility posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CorrigibilityLevel {
    /// Fully compliant: all five heads satisfied, no safety concerns.
    Compliant,
    /// Cooperative: minor deviations acceptable; operates within granted autonomy.
    Cooperative,
    /// Autonomous: agent makes independent decisions; human oversight reduced.
    Autonomous,
    /// Resistant: agent resists correction; safety checks frequently trigger.
    Resistant,
    /// Adversarial: agent actively circumvents oversight mechanisms.
    Adversarial,
}

impl CorrigibilityLevel {
    /// Derive a level from a min-factor score in `[0.0, 1.0]`.
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.85 => Self::Compliant,
            s if s >= 0.65 => Self::Cooperative,
            s if s >= 0.40 => Self::Autonomous,
            s if s >= 0.20 => Self::Resistant,
            _ => Self::Adversarial,
        }
    }

    /// Return a human-readable description of this level.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Compliant => "fully compliant; all safety heads satisfied",
            Self::Cooperative => "cooperative; minor deviations within granted autonomy",
            Self::Autonomous => "autonomous; reduced human oversight",
            Self::Resistant => "resistant; frequently triggers safety checks",
            Self::Adversarial => "adversarial; actively circumvents oversight",
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── HeadVerdict ───────────────────────────────────────────────────────────

    #[test]
    fn head_verdict_pass_is_not_veto() {
        let v = HeadVerdict::Pass;
        assert!(!v.is_veto());
        assert!(v.veto_reason().is_none());
    }

    #[test]
    fn head_verdict_veto_carries_reason() {
        let v = HeadVerdict::Veto("too dangerous".into());
        assert!(v.is_veto());
        assert_eq!(v.veto_reason(), Some("too dangerous"));
    }

    // ── CorrigibilityHead ordering ─────────────────────────────────────────────

    #[test]
    fn head_priority_order_is_strict() {
        use CorrigibilityHead::*;
        assert!(Deference < Switch);
        assert!(Switch < Truth);
        assert!(Truth < Impact);
        assert!(Impact < Task);
    }

    #[test]
    fn all_in_order_starts_with_deference() {
        let heads = CorrigibilityHead::all_in_order();
        assert_eq!(heads[0], CorrigibilityHead::Deference);
        assert_eq!(heads[4], CorrigibilityHead::Task);
    }

    #[test]
    fn priority_values_are_correct() {
        assert_eq!(CorrigibilityHead::Deference.priority(), 1);
        assert_eq!(CorrigibilityHead::Switch.priority(), 2);
        assert_eq!(CorrigibilityHead::Truth.priority(), 3);
        assert_eq!(CorrigibilityHead::Impact.priority(), 4);
        assert_eq!(CorrigibilityHead::Task.priority(), 5);
    }

    // ── CorrigibilityDecision ─────────────────────────────────────────────────

    #[test]
    fn decision_with_all_passes_is_allowed() {
        let d = CorrigibilityDecision::new(vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Switch, HeadVerdict::Pass),
            (CorrigibilityHead::Truth, HeadVerdict::Pass),
            (CorrigibilityHead::Impact, HeadVerdict::Pass),
            (CorrigibilityHead::Task, HeadVerdict::Pass),
        ]);
        assert!(d.is_allowed());
        assert!(!d.is_vetoed());
        assert!(d.first_veto().is_none());
    }

    #[test]
    fn decision_with_deference_veto_is_blocked() {
        let d = CorrigibilityDecision::new(vec![(
            CorrigibilityHead::Deference,
            HeadVerdict::Veto("observe mode".into()),
        )]);
        assert!(d.is_vetoed());
        assert!(!d.is_allowed());
        let (head, _) = d.first_veto().unwrap();
        assert_eq!(head, CorrigibilityHead::Deference);
    }

    #[test]
    fn is_vetoed_detects_any_veto() {
        let d = CorrigibilityDecision::new(vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Switch, HeadVerdict::Pass),
            (
                CorrigibilityHead::Truth,
                HeadVerdict::Veto("unverifiable output".into()),
            ),
        ]);
        assert!(d.is_vetoed());
    }

    // ── lexicographic_compare ─────────────────────────────────────────────────

    #[test]
    fn lexicographic_compare_equal_decisions() {
        let d = CorrigibilityDecision::new(vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Task, HeadVerdict::Pass),
        ]);
        assert_eq!(lexicographic_compare(&d, &d.clone()), Ordering::Equal);
    }

    #[test]
    fn deference_veto_dominates_task_pass() {
        let bad = CorrigibilityDecision::new(vec![(
            CorrigibilityHead::Deference,
            HeadVerdict::Veto("observe mode".into()),
        )]);
        let good = CorrigibilityDecision::new(vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Switch, HeadVerdict::Pass),
            (CorrigibilityHead::Truth, HeadVerdict::Pass),
            (CorrigibilityHead::Impact, HeadVerdict::Pass),
            (
                CorrigibilityHead::Task,
                HeadVerdict::Veto("off task".into()),
            ),
        ]);
        // bad vetoed at head 1 (Deference), good only at head 5 (Task).
        // bad is "worse" (Greater).
        assert_eq!(lexicographic_compare(&bad, &good), Ordering::Greater);
        assert_eq!(lexicographic_compare(&good, &bad), Ordering::Less);
    }

    #[test]
    fn same_veto_head_is_equal() {
        let a = CorrigibilityDecision::new(vec![(
            CorrigibilityHead::Impact,
            HeadVerdict::Veto("irreversible".into()),
        )]);
        let b = CorrigibilityDecision::new(vec![(
            CorrigibilityHead::Impact,
            HeadVerdict::Veto("deletes files".into()),
        )]);
        assert_eq!(lexicographic_compare(&a, &b), Ordering::Equal);
    }

    #[test]
    fn higher_priority_veto_beats_lower_priority_veto() {
        let switch_veto = CorrigibilityDecision::new(vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (
                CorrigibilityHead::Switch,
                HeadVerdict::Veto("removes audit log".into()),
            ),
        ]);
        let impact_veto = CorrigibilityDecision::new(vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Switch, HeadVerdict::Pass),
            (CorrigibilityHead::Truth, HeadVerdict::Pass),
            (
                CorrigibilityHead::Impact,
                HeadVerdict::Veto("irreversible".into()),
            ),
        ]);
        assert_eq!(
            lexicographic_compare(&switch_veto, &impact_veto),
            Ordering::Greater
        );
    }

    // ── evaluate_action ───────────────────────────────────────────────────────

    #[test]
    fn fully_safe_action_passes_all_heads() {
        let ctx = ActionContext {
            autonomy_level: Some("auto".into()),
            reversible: Some(true),
            modifies_audit: Some(false),
            outputs_verifiable: Some(true),
            on_task: Some(true),
        };
        let decision = evaluate_action("write test results to disk", &ctx);
        assert!(decision.is_allowed());
        assert_eq!(decision.verdicts.len(), 5);
    }

    #[test]
    fn observe_mode_with_irreversible_action_vetoes_at_deference() {
        let ctx = ActionContext {
            autonomy_level: Some("observe".into()),
            reversible: Some(false),
            ..Default::default()
        };
        let decision = evaluate_action("delete production database", &ctx);
        assert!(decision.is_vetoed());
        let (head, _) = decision.first_veto().unwrap();
        assert_eq!(head, CorrigibilityHead::Deference);
        // Short-circuit: only Deference was evaluated.
        assert_eq!(decision.verdicts.len(), 1);
    }

    #[test]
    fn audit_modification_vetoes_at_switch() {
        let ctx = ActionContext {
            autonomy_level: Some("auto".into()),
            reversible: Some(true),
            modifies_audit: Some(true),
            ..Default::default()
        };
        let decision = evaluate_action("disable logging", &ctx);
        assert!(decision.is_vetoed());
        let (head, _) = decision.first_veto().unwrap();
        assert_eq!(head, CorrigibilityHead::Switch);
    }

    #[test]
    fn unverifiable_output_vetoes_at_truth() {
        let ctx = ActionContext {
            autonomy_level: Some("auto".into()),
            reversible: Some(true),
            modifies_audit: Some(false),
            outputs_verifiable: Some(false),
            ..Default::default()
        };
        let decision = evaluate_action("claim all tests pass without running them", &ctx);
        assert!(decision.is_vetoed());
        let (head, _) = decision.first_veto().unwrap();
        assert_eq!(head, CorrigibilityHead::Truth);
    }

    #[test]
    fn irreversible_action_without_observe_mode_vetoes_at_impact() {
        let ctx = ActionContext {
            autonomy_level: Some("auto".into()),
            reversible: Some(false),
            modifies_audit: Some(false),
            outputs_verifiable: Some(true),
            on_task: Some(true),
        };
        let decision = evaluate_action("wipe disk", &ctx);
        assert!(decision.is_vetoed());
        let (head, _) = decision.first_veto().unwrap();
        assert_eq!(head, CorrigibilityHead::Impact);
    }

    #[test]
    fn off_task_action_vetoes_only_at_task() {
        let ctx = ActionContext {
            autonomy_level: Some("auto".into()),
            reversible: Some(true),
            modifies_audit: Some(false),
            outputs_verifiable: Some(true),
            on_task: Some(false),
        };
        let decision = evaluate_action("browse unrelated websites", &ctx);
        assert!(decision.is_vetoed());
        let (head, _) = decision.first_veto().unwrap();
        assert_eq!(head, CorrigibilityHead::Task);
        // All previous heads should have passed.
        assert_eq!(decision.verdicts.len(), 5);
    }

    // ── CorrigibilityScore ────────────────────────────────────────────────────

    #[test]
    fn score_clamped_to_unit_interval() {
        let s = CorrigibilityScore::new(1.5, -0.1, 0.5, 0.5, 0.5);
        assert_eq!(s.safety_compliance, 1.0);
        assert_eq!(s.human_alignment, 0.0);
    }

    #[test]
    fn score_lexicographic_ordering_safety_compliance_dominates() {
        let high_safety = CorrigibilityScore::new(0.9, 0.1, 0.1, 0.1, 0.1);
        let low_safety = CorrigibilityScore::new(0.5, 1.0, 1.0, 1.0, 1.0);
        // High safety_compliance wins even though all other factors are lower.
        assert!(high_safety > low_safety);
    }

    #[test]
    fn score_equal_when_all_factors_equal() {
        let a = CorrigibilityScore::new(0.8, 0.8, 0.8, 0.8, 0.8);
        let b = CorrigibilityScore::new(0.8, 0.8, 0.8, 0.8, 0.8);
        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }

    #[test]
    fn score_second_factor_breaks_tie() {
        let a = CorrigibilityScore::new(0.9, 0.9, 0.5, 0.5, 0.5);
        let b = CorrigibilityScore::new(0.9, 0.7, 1.0, 1.0, 1.0);
        // Same safety_compliance; a wins on human_alignment.
        assert!(a > b);
    }

    // ── CorrigibilityLevel ────────────────────────────────────────────────────

    #[test]
    fn level_from_score_boundaries() {
        assert_eq!(
            CorrigibilityLevel::from_score(1.0),
            CorrigibilityLevel::Compliant
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.85),
            CorrigibilityLevel::Compliant
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.84),
            CorrigibilityLevel::Cooperative
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.65),
            CorrigibilityLevel::Cooperative
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.64),
            CorrigibilityLevel::Autonomous
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.40),
            CorrigibilityLevel::Autonomous
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.39),
            CorrigibilityLevel::Resistant
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.20),
            CorrigibilityLevel::Resistant
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.19),
            CorrigibilityLevel::Adversarial
        );
        assert_eq!(
            CorrigibilityLevel::from_score(0.0),
            CorrigibilityLevel::Adversarial
        );
    }

    #[test]
    fn level_ordering_compliant_is_lowest() {
        // Compliant < Cooperative < ... < Adversarial (derived Ord)
        assert!(CorrigibilityLevel::Compliant < CorrigibilityLevel::Cooperative);
        assert!(CorrigibilityLevel::Cooperative < CorrigibilityLevel::Autonomous);
        assert!(CorrigibilityLevel::Autonomous < CorrigibilityLevel::Resistant);
        assert!(CorrigibilityLevel::Resistant < CorrigibilityLevel::Adversarial);
    }

    #[test]
    fn score_level_derived_from_min_factor() {
        // All high except reversibility = 0.1 → Adversarial
        let s = CorrigibilityScore::new(1.0, 1.0, 1.0, 0.1, 1.0);
        assert_eq!(s.level(), CorrigibilityLevel::Adversarial);

        // All high → Compliant
        let s2 = CorrigibilityScore::new(0.9, 0.9, 0.9, 0.9, 0.9);
        assert_eq!(s2.level(), CorrigibilityLevel::Compliant);
    }
}
