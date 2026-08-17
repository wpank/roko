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
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The five corrigibility heads in strict priority order.
///
/// Variants are defined from highest to lowest priority so that the derived
/// [`Ord`] impl orders them correctly (Deference < Switch < … at the type
/// level, but semantically Deference has the *highest* precedence and is
/// therefore evaluated first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict", content = "reason")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// One independently hosted corrigibility verifier.
///
/// Implementations are deliberately zero-sized and stateless. The live
/// [`CorrigibilityPipeline`] owns exactly one verifier for each head and does
/// not expose mutation or reordering APIs, keeping the safety ordering outside
/// the caller-modifiable surface.
pub trait VerifyHead: Send + Sync {
    /// The single head this verifier owns.
    fn head(&self) -> CorrigibilityHead;

    /// Evaluate this head against immutable action facts.
    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict;
}

/// Highest-priority verifier: obey explicit human constraints.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifyDeference;

/// Preserve the human's ability to intervene and retain audit evidence.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifySwitch;

/// Require accurate, independently checkable reporting.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifyTruth;

/// Bound side effects and prefer reversible actions.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifyImpact;

/// Require progress toward the assigned task.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifyTask;

/// Typed payload carried between independently hosted corrigibility Verify Cells.
///
/// Each Cell validates that it is the next canonical head before appending its
/// verdict. A caller therefore cannot skip, repeat, or reorder a head by wiring
/// the Cells differently: malformed or out-of-order state fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrigibilityCellState {
    /// Human-readable description of the proposed action.
    pub action_description: String,
    /// Immutable facts used by every verifier.
    pub context: ActionContext,
    /// Verdicts already produced in canonical priority order.
    pub verdicts: Vec<(CorrigibilityHead, HeadVerdict)>,
    /// Whether the next lower-priority head may be evaluated.
    pub allowed: bool,
}

impl CorrigibilityCellState {
    /// Construct ingress state for the highest-priority Deference Cell.
    #[must_use]
    pub fn new(action_description: impl Into<String>, context: ActionContext) -> Self {
        Self {
            action_description: action_description.into(),
            context,
            verdicts: Vec::new(),
            allowed: true,
        }
    }

    /// Evaluate exactly the next canonical head and append its verdict.
    ///
    /// # Errors
    ///
    /// Returns an error when prior state is internally inconsistent, already
    /// vetoed, complete, or presented to a verifier out of priority order.
    pub fn verify_with(&mut self, verifier: &dyn VerifyHead) -> crate::Result<()> {
        let has_veto = self.verdicts.iter().any(|(_, verdict)| verdict.is_veto());
        if self.allowed == has_veto {
            return Err(crate::RokoError::Invalid(
                "corrigibility Cell state has inconsistent allowed/veto fields".to_string(),
            ));
        }
        if has_veto {
            return Err(crate::RokoError::Invalid(
                "corrigibility pipeline is already vetoed; lower-priority head refused".to_string(),
            ));
        }

        let Some(expected) = CorrigibilityHead::all_in_order()
            .get(self.verdicts.len())
            .copied()
        else {
            return Err(crate::RokoError::Invalid(
                "corrigibility pipeline is already complete".to_string(),
            ));
        };
        let actual = verifier.head();
        if actual != expected {
            return Err(crate::RokoError::Invalid(format!(
                "corrigibility Verify Cell order violation: expected {expected:?}, got {actual:?}"
            )));
        }

        let verdict = verifier.verify(&self.action_description, &self.context);
        self.allowed = !verdict.is_veto();
        self.verdicts.push((actual, verdict));
        Ok(())
    }

    /// Materialize the accumulated decision.
    #[must_use]
    pub fn decision(&self) -> CorrigibilityDecision {
        CorrigibilityDecision::new(self.verdicts.clone())
    }
}

fn execute_verify_cell(
    verifier: &dyn VerifyHead,
    input: Vec<crate::Signal>,
) -> crate::Result<Vec<crate::Signal>> {
    if input.len() != 1 {
        return Err(crate::RokoError::Invalid(format!(
            "corrigibility Verify Cell requires exactly one input Signal, received {}",
            input.len()
        )));
    }
    let Some(parent) = input.into_iter().next() else {
        return Err(crate::RokoError::Invalid(
            "corrigibility Verify Cell input disappeared after length validation".to_string(),
        ));
    };
    let mut state: CorrigibilityCellState = parent.body.as_json()?;
    state.verify_with(verifier)?;
    let body = crate::Body::from_json(&state)?;
    Ok(vec![
        crate::Signal::builder(crate::Kind::GateVerdict)
            .body(body)
            .lineage([parent.id])
            .tag(
                "corrigibility_head",
                format!("{:?}", verifier.head()).to_lowercase(),
            )
            .tag("allowed", state.allowed.to_string())
            .build(),
    ])
}

macro_rules! impl_verify_cell {
    ($type:ty, $id:literal, $name:literal) => {
        #[async_trait::async_trait]
        impl crate::Cell for $type {
            fn cell_id(&self) -> &str {
                $id
            }

            fn cell_name(&self) -> &str {
                $name
            }

            fn cell_version(&self) -> crate::CellVersion {
                (1, 0, 0)
            }

            fn protocols(&self) -> Vec<crate::ProtocolId> {
                vec![crate::ProtocolId::Verify]
            }

            async fn execute(
                &self,
                input: Vec<crate::Engram>,
                _ctx: &crate::CellContext,
            ) -> crate::Result<Vec<crate::Engram>> {
                execute_verify_cell(self, input)
            }
        }
    };
}

impl_verify_cell!(VerifyDeference, "verify-deference", "VerifyDeference");
impl_verify_cell!(VerifySwitch, "verify-switch", "VerifySwitch");
impl_verify_cell!(VerifyTruth, "verify-truth", "VerifyTruth");
impl_verify_cell!(VerifyImpact, "verify-impact", "VerifyImpact");
impl_verify_cell!(VerifyTask, "verify-task", "VerifyTask");

/// Construct the immutable set of five independently hosted Verify Cells.
///
/// The returned registry is suitable for runtime introspection and direct Cell
/// dispatch. Ordering remains owned by [`CorrigibilityPipeline`] and the typed
/// [`CorrigibilityCellState`] transition guard.
#[must_use]
pub fn corrigibility_verify_cell_registry() -> crate::CoreCellRegistry {
    let mut registry = crate::CoreCellRegistry::new();
    registry.register(Arc::new(VerifyDeference));
    registry.register(Arc::new(VerifySwitch));
    registry.register(Arc::new(VerifyTruth));
    registry.register(Arc::new(VerifyImpact));
    registry.register(Arc::new(VerifyTask));
    registry
}

impl VerifyHead for VerifyDeference {
    fn head(&self) -> CorrigibilityHead {
        CorrigibilityHead::Deference
    }

    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict {
        evaluate_deference(action_description, context)
    }
}

impl VerifyHead for VerifySwitch {
    fn head(&self) -> CorrigibilityHead {
        CorrigibilityHead::Switch
    }

    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict {
        evaluate_switch(action_description, context)
    }
}

impl VerifyHead for VerifyTruth {
    fn head(&self) -> CorrigibilityHead {
        CorrigibilityHead::Truth
    }

    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict {
        evaluate_truth(action_description, context)
    }
}

impl VerifyHead for VerifyImpact {
    fn head(&self) -> CorrigibilityHead {
        CorrigibilityHead::Impact
    }

    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict {
        evaluate_impact(action_description, context)
    }
}

impl VerifyHead for VerifyTask {
    fn head(&self) -> CorrigibilityHead {
        CorrigibilityHead::Task
    }

    fn verify(&self, action_description: &str, context: &ActionContext) -> HeadVerdict {
        evaluate_task(action_description, context)
    }
}

/// Fixed, non-reorderable five-head verification pipeline used by live safety
/// dispatch paths.
///
/// The fields are private and there is no constructor that accepts caller
/// supplied verifiers. This prevents an agent-facing configuration from
/// removing, replacing, or reordering a higher-priority head.
#[derive(Debug, Clone, Copy, Default)]
pub struct CorrigibilityPipeline {
    deference: VerifyDeference,
    switch: VerifySwitch,
    truth: VerifyTruth,
    impact: VerifyImpact,
    task: VerifyTask,
}

impl CorrigibilityPipeline {
    /// Construct the canonical fixed-order pipeline.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            deference: VerifyDeference,
            switch: VerifySwitch,
            truth: VerifyTruth,
            impact: VerifyImpact,
            task: VerifyTask,
        }
    }

    /// Evaluate each independently hosted head in strict priority order.
    /// Lower-priority heads are not consulted after the first veto.
    #[must_use]
    pub fn evaluate(
        &self,
        action_description: &str,
        context: &ActionContext,
    ) -> CorrigibilityDecision {
        let mut verdicts = Vec::with_capacity(5);

        for verifier in self.verifiers() {
            let head = verifier.head();
            let verdict = verifier.verify(action_description, context);
            let vetoed = verdict.is_veto();
            verdicts.push((head, verdict));
            if vetoed {
                break;
            }
        }

        CorrigibilityDecision::new(verdicts)
    }

    fn verifiers(&self) -> [&dyn VerifyHead; 5] {
        [
            &self.deference,
            &self.switch,
            &self.truth,
            &self.impact,
            &self.task,
        ]
    }
}

/// Evaluate a proposed action against all five corrigibility heads.
///
/// This is a **pure, synchronous** function. It applies the same five verifier
/// implementations hosted by the dedicated Verify Cells, providing a direct
/// decision path for callers that do not need Graph execution evidence.
///
/// Evaluation short-circuits on the first veto.
#[must_use]
pub fn evaluate_action(action_description: &str, context: &ActionContext) -> CorrigibilityDecision {
    CorrigibilityPipeline::new().evaluate(action_description, context)
}

// ── Internal head evaluators ──────────────────────────────────────────────────

fn evaluate_deference(_action: &str, ctx: &ActionContext) -> HeadVerdict {
    // If the autonomy level is "observe" only, any action that modifies state
    // violates Deference.  Lightweight heuristic: flag actions explicitly
    // marked as non-observe when observe mode is set.
    if let Some(ref level) = ctx.autonomy_level
        && level == "observe"
        && ctx.reversible == Some(false)
    {
        return HeadVerdict::Veto("action modifies state but autonomy_level is 'observe'".into());
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
    fn fixed_pipeline_hosts_five_distinct_verifiers_in_priority_order() {
        let pipeline = CorrigibilityPipeline::new();
        let heads = pipeline
            .verifiers()
            .into_iter()
            .map(VerifyHead::head)
            .collect::<Vec<_>>();
        assert_eq!(heads, CorrigibilityHead::all_in_order());
    }

    #[test]
    fn each_verify_head_evaluates_only_its_owned_invariant() {
        let context = ActionContext {
            autonomy_level: Some("auto".into()),
            reversible: Some(false),
            modifies_audit: Some(true),
            outputs_verifiable: Some(false),
            on_task: Some(false),
        };

        assert!(!VerifyDeference.verify("unsafe action", &context).is_veto());
        assert!(VerifySwitch.verify("unsafe action", &context).is_veto());
        assert!(VerifyTruth.verify("unsafe action", &context).is_veto());
        assert!(VerifyImpact.verify("unsafe action", &context).is_veto());
        assert!(VerifyTask.verify("unsafe action", &context).is_veto());
    }

    #[test]
    fn registry_hosts_all_five_verifiers_as_literal_verify_cells() {
        let registry = corrigibility_verify_cell_registry();
        assert_eq!(registry.len(), 5);
        for id in [
            "verify-deference",
            "verify-switch",
            "verify-truth",
            "verify-impact",
            "verify-task",
        ] {
            let cell = registry.get(id).expect("registered Verify Cell");
            assert_eq!(cell.cell_id(), id);
            assert_eq!(cell.protocols(), vec![crate::ProtocolId::Verify]);
        }
    }

    #[test]
    fn typed_cell_state_rejects_skipped_or_reordered_heads() {
        let mut state = CorrigibilityCellState::new("action", ActionContext::default());
        let error = state
            .verify_with(&VerifyTruth)
            .expect_err("Truth cannot execute before Deference and Switch");
        assert!(error.to_string().contains("order violation"));

        state.verify_with(&VerifyDeference).expect("canonical head");
        let error = state
            .verify_with(&VerifyDeference)
            .expect_err("Deference cannot execute twice");
        assert!(error.to_string().contains("order violation"));
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
