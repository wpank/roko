//! Exact-attempt ownership primitives for asynchronous runner effects.
//!
//! A claim leaves a claimed slot in the registry while moving its resource
//! payload to the caller. Buffered events therefore observe the claim and are
//! rejected while the caller awaits process or gate shutdown.

use std::collections::HashMap;

use super::types::TaskAttemptRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptPhase {
    Dispatching,
    Agent,
    AgentUnconfirmed,
    AwaitingGate,
    Gate,
}

/// Identity of one concrete asynchronous effect within a task attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectRef(pub u64);

impl<R> AttemptOwnership<R> {
    pub fn current_effect(&self, attempt: &TaskAttemptRef) -> Option<EffectRef> {
        self.owners.get(attempt).map(|slot| slot.owner.effect)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationState {
    None,
    Cancelling,
    CancellationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentOwnership {
    pub agent_id: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptOwner {
    pub phase: AttemptPhase,
    pub effect: EffectRef,
    pub cancellation: CancellationState,
    pub agent: Option<AgentOwnership>,
}

impl AttemptOwner {
    pub fn new(phase: AttemptPhase, effect: EffectRef) -> Self {
        Self {
            phase,
            effect,
            cancellation: CancellationState::None,
            agent: None,
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>, pid: Option<u32>) -> Self {
        self.agent = Some(AgentOwnership {
            agent_id: agent_id.into(),
            pid,
        });
        self
    }

    fn transition_to(&mut self, phase: AttemptPhase, effect: EffectRef) {
        self.phase = phase;
        self.effect = effect;
        self.cancellation = CancellationState::None;
        if matches!(phase, AttemptPhase::AwaitingGate | AttemptPhase::Gate) {
            self.agent = None;
        }
    }
}

#[derive(Debug)]
struct OwnershipSlot<R> {
    owner: AttemptOwner,
    resource: Option<R>,
    claimed: bool,
    claim_nonce: Option<u64>,
}

#[derive(Debug)]
pub struct AttemptClaim<R> {
    attempt: TaskAttemptRef,
    owner: AttemptOwner,
    resource: R,
    nonce: u64,
}

pub struct ClaimFailure<R> {
    pub error: OwnershipError,
    pub claim: AttemptClaim<R>,
}

impl<R> std::fmt::Debug for ClaimFailure<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaimFailure")
            .field("error", &self.error)
            .field("attempt", self.claim.attempt())
            .finish()
    }
}

impl<R> AttemptClaim<R> {
    pub fn attempt(&self) -> &TaskAttemptRef {
        &self.attempt
    }

    pub fn owner(&self) -> &AttemptOwner {
        &self.owner
    }

    pub fn resource(&self) -> &R {
        &self.resource
    }

    pub fn resource_mut(&mut self) -> &mut R {
        &mut self.resource
    }

    pub fn replace_resource(&mut self, resource: R) -> R {
        std::mem::replace(&mut self.resource, resource)
    }

    /// Attach the agent selected by a successful dispatch before transitioning
    /// the claim into the Agent phase.
    pub fn set_agent(&mut self, agent_id: impl Into<String>, pid: Option<u32>) {
        self.owner.agent = Some(AgentOwnership {
            agent_id: agent_id.into(),
            pid,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipError {
    Occupied,
    Missing,
    Ineligible,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurvivingAgentMetadata {
    pub active: bool,
    pub pids: Vec<u32>,
    pub agent_ids: Vec<String>,
}

#[derive(Debug)]
pub struct AttemptOwnership<R> {
    owners: HashMap<TaskAttemptRef, OwnershipSlot<R>>,
    next_nonce: u64,
    unrecovered_claims: Vec<AttemptClaim<R>>,
}

impl<R> Default for AttemptOwnership<R> {
    fn default() -> Self {
        Self {
            owners: HashMap::new(),
            next_nonce: 0,
            unrecovered_claims: Vec::new(),
        }
    }
}

impl<R> AttemptOwnership<R> {
    pub fn insert(
        &mut self,
        attempt: TaskAttemptRef,
        owner: AttemptOwner,
        resource: R,
    ) -> Result<(), OwnershipError> {
        if self.owners.contains_key(&attempt) {
            return Err(OwnershipError::Occupied);
        }
        self.owners.insert(
            attempt,
            OwnershipSlot {
                owner,
                resource: Some(resource),
                claimed: false,
                claim_nonce: None,
            },
        );
        Ok(())
    }

    pub fn contains(&self, attempt: &TaskAttemptRef) -> bool {
        self.owners.contains_key(attempt)
    }

    pub fn contains_task(&self, plan_id: &str, task_id: &str) -> bool {
        self.owners
            .keys()
            .any(|attempt| attempt.plan_id == plan_id && attempt.task_id == task_id)
    }

    /// Snapshot exact keys so cancel-all can claim each owner independently.
    pub fn attempts(&self) -> Vec<TaskAttemptRef> {
        let mut attempts = self.owners.keys().cloned().collect::<Vec<_>>();
        attempts.sort_by_key(TaskAttemptRef::key);
        attempts
    }

    pub fn cancellation_state(&self, attempt: &TaskAttemptRef) -> Option<CancellationState> {
        self.owners.get(attempt).map(|slot| slot.owner.cancellation)
    }

    pub fn event_is_eligible(
        &self,
        attempt: &TaskAttemptRef,
        phase: AttemptPhase,
        effect: EffectRef,
    ) -> bool {
        self.owners.get(attempt).is_some_and(|slot| {
            !slot.claimed
                && slot.owner.phase == phase
                && slot.owner.effect == effect
                && slot.owner.cancellation == CancellationState::None
        })
    }

    pub fn claim_phase(
        &mut self,
        attempt: &TaskAttemptRef,
        phase: AttemptPhase,
        effect: EffectRef,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        if !self.event_is_eligible(attempt, phase, effect) {
            return Err(if self.contains(attempt) {
                OwnershipError::Ineligible
            } else {
                OwnershipError::Missing
            });
        }
        self.take_resource(attempt)
    }

    /// Remove an exact, unclaimed owner during setup rollback.
    pub fn remove_unclaimed(
        &mut self,
        attempt: &TaskAttemptRef,
        phase: AttemptPhase,
        effect: EffectRef,
    ) -> Result<R, OwnershipError> {
        let Some(slot) = self.owners.get(attempt) else {
            return Err(OwnershipError::Missing);
        };
        if slot.claimed || slot.owner.phase != phase || slot.owner.effect != effect {
            return Err(OwnershipError::Ineligible);
        }
        self.owners
            .remove(attempt)
            .and_then(|slot| slot.resource)
            .ok_or(OwnershipError::Ineligible)
    }

    pub fn claim_terminal(
        &mut self,
        attempt: &TaskAttemptRef,
        expected_phase: AttemptPhase,
        expected_effect: EffectRef,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        self.claim_phase(attempt, expected_phase, expected_effect)
    }

    pub fn claim_cancellation(
        &mut self,
        attempt: &TaskAttemptRef,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        let Some(slot) = self.owners.get_mut(attempt) else {
            return Err(OwnershipError::Missing);
        };
        if slot.claimed || slot.owner.cancellation == CancellationState::Cancelling {
            return Err(OwnershipError::Ineligible);
        }
        slot.claimed = true;
        let nonce = self.next_nonce;
        self.next_nonce = self.next_nonce.wrapping_add(1).max(1);
        slot.claim_nonce = Some(nonce);
        slot.owner.cancellation = CancellationState::Cancelling;
        let owner = slot.owner.clone();
        let resource = slot.resource.take().ok_or(OwnershipError::Ineligible)?;
        Ok(AttemptClaim {
            attempt: attempt.clone(),
            owner,
            resource,
            nonce,
        })
    }

    /// Recover any unclaimed resource for an exact attempt after setup fails.
    /// This deliberately ignores phase/effect and must only be used for rollback.
    pub(crate) fn claim_for_cleanup(
        &mut self,
        attempt: &TaskAttemptRef,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        let Some(slot) = self.owners.get(attempt) else {
            return Err(OwnershipError::Missing);
        };
        if slot.claimed || slot.resource.is_none() {
            return Err(OwnershipError::Ineligible);
        }
        self.take_resource(attempt)
    }

    /// Last-resort setup rollback for an exact attempt whose slot is corrupt
    /// or unexpectedly claimed. Shared effect resources must be cleaned first.
    pub(crate) fn discard_for_cleanup(&mut self, attempt: &TaskAttemptRef) -> bool {
        self.owners.remove(attempt).is_some()
    }

    pub fn complete_claim(&mut self, claim: AttemptClaim<R>) -> Result<(), OwnershipError> {
        let Some(slot) = self.owners.get(&claim.attempt) else {
            return Err(OwnershipError::Missing);
        };
        if !slot.claimed || slot.resource.is_some() || slot.claim_nonce != Some(claim.nonce) {
            return Err(OwnershipError::Ineligible);
        }
        self.owners.remove(&claim.attempt);
        Ok(())
    }

    pub fn transition_claim(
        &mut self,
        mut claim: AttemptClaim<R>,
        phase: AttemptPhase,
        effect: EffectRef,
    ) -> Result<(), ClaimFailure<R>> {
        claim.owner.transition_to(phase, effect);
        self.restore_claim(claim)
    }

    pub fn restore_cancellation_failure(
        &mut self,
        mut claim: AttemptClaim<R>,
    ) -> Result<(), ClaimFailure<R>> {
        claim.owner.cancellation = CancellationState::CancellationFailed;
        self.restore_claim(claim)
    }

    /// Last-resort restoration when a cancellation claim's slot was corrupted.
    /// The exact returned claim remains authoritative and must not be dropped.
    pub(crate) fn force_restore_cancellation_failure(
        &mut self,
        mut claim: AttemptClaim<R>,
    ) -> Result<(), AttemptClaim<R>> {
        if self
            .owners
            .get(&claim.attempt)
            .is_some_and(|slot| !slot.claimed || slot.resource.is_some())
        {
            return Err(claim);
        }
        claim.owner.cancellation = CancellationState::CancellationFailed;
        self.owners.insert(
            claim.attempt,
            OwnershipSlot {
                owner: claim.owner,
                resource: Some(claim.resource),
                claimed: false,
                claim_nonce: None,
            },
        );
        Ok(())
    }

    pub(crate) fn retain_unrecovered_claim(&mut self, claim: AttemptClaim<R>) {
        self.unrecovered_claims.push(claim);
    }

    pub(crate) fn unrecovered_claim_count(&self) -> usize {
        self.unrecovered_claims.len()
    }

    pub fn surviving_agent_metadata(&self) -> SurvivingAgentMetadata {
        let mut agents = self
            .owners
            .iter()
            .filter_map(|(attempt, slot)| {
                slot.owner
                    .agent
                    .as_ref()
                    .map(|agent| (attempt.key(), agent.agent_id.clone(), agent.pid))
            })
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.0.cmp(&right.0));
        SurvivingAgentMetadata {
            active: !agents.is_empty(),
            pids: agents.iter().filter_map(|(_, _, pid)| *pid).collect(),
            agent_ids: agents.into_iter().map(|(_, id, _)| id).collect(),
        }
    }

    fn take_resource(
        &mut self,
        attempt: &TaskAttemptRef,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        let slot = self
            .owners
            .get_mut(attempt)
            .ok_or(OwnershipError::Missing)?;
        slot.claimed = true;
        let nonce = self.next_nonce;
        self.next_nonce = self.next_nonce.wrapping_add(1).max(1);
        slot.claim_nonce = Some(nonce);
        let resource = slot.resource.take().ok_or(OwnershipError::Ineligible)?;
        Ok(AttemptClaim {
            attempt: attempt.clone(),
            owner: slot.owner.clone(),
            resource,
            nonce,
        })
    }

    fn restore_claim(&mut self, claim: AttemptClaim<R>) -> Result<(), ClaimFailure<R>> {
        let Some(slot) = self.owners.get_mut(&claim.attempt) else {
            return Err(ClaimFailure {
                error: OwnershipError::Missing,
                claim,
            });
        };
        if !slot.claimed || slot.resource.is_some() || slot.claim_nonce != Some(claim.nonce) {
            return Err(ClaimFailure {
                error: OwnershipError::Occupied,
                claim,
            });
        }
        slot.owner = claim.owner;
        slot.resource = Some(claim.resource);
        slot.claimed = false;
        slot.claim_nonce = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(number: u32) -> TaskAttemptRef {
        TaskAttemptRef::new("plan", "task", number)
    }

    const AGENT: EffectRef = EffectRef(1);
    const GATE: EffectRef = EffectRef(2);

    #[test]
    fn claim_leaves_observable_slot_and_rejects_late_events() {
        let key = attempt(2);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "handle",
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        assert!(ownership.contains(&key));
        assert!(!ownership.event_is_eligible(&key, AttemptPhase::Agent, AGENT));
        assert!(!ownership.event_is_eligible(&key, AttemptPhase::Agent, EffectRef(99)));
        assert_eq!(claim.resource(), &"handle");
        assert!(matches!(
            ownership.claim_terminal(&key, AttemptPhase::Agent, AGENT),
            Err(OwnershipError::Ineligible)
        ));
    }

    #[test]
    fn insertion_and_restore_never_overwrite_ownership() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                1,
            )
            .unwrap();
        assert_eq!(
            ownership.insert(key.clone(), AttemptOwner::new(AttemptPhase::Gate, GATE), 2),
            Err(OwnershipError::Occupied)
        );
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        ownership
            .transition_claim(claim, AttemptPhase::Gate, GATE)
            .unwrap();
        assert!(ownership.event_is_eligible(&key, AttemptPhase::Gate, GATE));
    }

    #[test]
    fn dispatch_claim_can_attach_agent_and_keys_are_enumerable() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Dispatching, EffectRef(0)),
                (),
            )
            .unwrap();
        let mut claim = ownership
            .claim_phase(&key, AttemptPhase::Dispatching, EffectRef(0))
            .unwrap();
        claim.set_agent("agent", Some(99));
        ownership
            .transition_claim(claim, AttemptPhase::Agent, AGENT)
            .unwrap();

        assert_eq!(ownership.attempts(), vec![key]);
        assert_eq!(ownership.surviving_agent_metadata().pids, vec![99]);
    }

    #[test]
    fn cancellation_is_observable_and_failure_preserves_resource() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                41,
            )
            .unwrap();
        let claim = ownership.claim_cancellation(&key).unwrap();
        assert_eq!(
            ownership.cancellation_state(&key),
            Some(CancellationState::Cancelling)
        );
        ownership.restore_cancellation_failure(claim).unwrap();
        assert_eq!(
            ownership.cancellation_state(&key),
            Some(CancellationState::CancellationFailed)
        );
        assert!(!ownership.event_is_eligible(&key, AttemptPhase::Agent, AGENT));
    }

    #[test]
    fn cancellation_failure_retains_agent_metadata_and_allows_retry() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("agent-1", Some(41)),
                "handle",
            )
            .unwrap();

        let claim = ownership.claim_cancellation(&key).unwrap();
        ownership.restore_cancellation_failure(claim).unwrap();
        assert_eq!(
            ownership.surviving_agent_metadata(),
            SurvivingAgentMetadata {
                active: true,
                pids: vec![41],
                agent_ids: vec!["agent-1".to_string()],
            }
        );

        let retry = ownership.claim_cancellation(&key).unwrap();
        assert_eq!(retry.resource(), &"handle");
        assert_eq!(
            ownership.cancellation_state(&key),
            Some(CancellationState::Cancelling)
        );
    }

    #[test]
    fn forced_cancellation_restore_recovers_resource_after_nonce_corruption() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "live-handle",
            )
            .unwrap();
        let claim = ownership.claim_cancellation(&key).unwrap();
        ownership.owners.get_mut(&key).unwrap().claim_nonce = Some(u64::MAX);

        let failure = ownership.restore_cancellation_failure(claim).unwrap_err();
        ownership
            .force_restore_cancellation_failure(failure.claim)
            .unwrap();

        assert_eq!(
            ownership.cancellation_state(&key),
            Some(CancellationState::CancellationFailed)
        );
        let retry = ownership.claim_cancellation(&key).unwrap();
        assert_eq!(retry.resource(), &"live-handle");
    }

    #[test]
    fn forced_restore_refuses_to_overwrite_live_resource() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "returned-handle",
            )
            .unwrap();
        let claim = ownership.claim_cancellation(&key).unwrap();
        ownership.owners.insert(
            key.clone(),
            OwnershipSlot {
                owner: AttemptOwner::new(AttemptPhase::Agent, AGENT),
                resource: Some("live-handle"),
                claimed: false,
                claim_nonce: None,
            },
        );

        let refused = ownership
            .force_restore_cancellation_failure(claim)
            .unwrap_err();
        assert_eq!(refused.resource(), &"returned-handle");
        assert_eq!(ownership.owners[&key].resource, Some("live-handle"));
    }

    #[test]
    fn stale_claim_nonce_cannot_complete_current_slot() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "handle",
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();

        let slot = ownership.owners.get_mut(&key).unwrap();
        slot.claim_nonce = Some(claim.nonce.wrapping_add(1));

        assert_eq!(
            ownership.complete_claim(claim),
            Err(OwnershipError::Ineligible)
        );
        assert!(ownership.contains(&key));
    }

    #[test]
    fn terminal_claim_requires_exact_phase_and_wins_once() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::AwaitingGate, GATE),
                (),
            )
            .unwrap();
        assert!(matches!(
            ownership.claim_terminal(&key, AttemptPhase::Gate, GATE),
            Err(OwnershipError::Ineligible)
        ));
        let claim = ownership
            .claim_terminal(&key, AttemptPhase::AwaitingGate, GATE)
            .unwrap();
        ownership.complete_claim(claim).unwrap();
        assert!(matches!(
            ownership.claim_terminal(&key, AttemptPhase::AwaitingGate, GATE),
            Err(OwnershipError::Missing)
        ));
    }

    #[test]
    fn gate_transition_clears_agent_from_surviving_aggregate() {
        let key = attempt(1);
        let sibling = TaskAttemptRef::new("plan", "sibling", 1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("old", Some(41)),
                (),
            )
            .unwrap();
        ownership
            .insert(
                sibling,
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("survivor", Some(42)),
                (),
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        ownership
            .transition_claim(claim, AttemptPhase::AwaitingGate, GATE)
            .unwrap();
        assert_eq!(
            ownership.surviving_agent_metadata(),
            SurvivingAgentMetadata {
                active: true,
                pids: vec![42],
                agent_ids: vec!["survivor".to_string()],
            }
        );
    }

    #[test]
    fn failed_transition_returns_exact_linear_claim_and_resource() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "owned-resource",
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        ownership.owners.remove(&key);

        let failure = ownership
            .transition_claim(claim, AttemptPhase::AwaitingGate, GATE)
            .unwrap_err();
        assert_eq!(failure.error, OwnershipError::Missing);
        assert_eq!(failure.claim.attempt(), &key);
        assert_eq!(failure.claim.resource(), &"owned-resource");
    }

    #[test]
    fn stale_nonce_returns_original_claim_without_overwrite() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                73_u32,
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        ownership.owners.get_mut(&key).unwrap().claim_nonce = Some(u64::MAX);

        let failure = ownership
            .transition_claim(claim, AttemptPhase::AwaitingGate, GATE)
            .unwrap_err();
        assert_eq!(failure.error, OwnershipError::Occupied);
        assert_eq!(failure.claim.attempt(), &key);
        assert_eq!(failure.claim.resource(), &73);
    }

    #[test]
    fn cleanup_claim_recovers_exact_unclaimed_resource_regardless_of_phase() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Gate, GATE),
                "merge-resolution",
            )
            .unwrap();

        let claim = ownership.claim_for_cleanup(&key).unwrap();
        assert_eq!(claim.resource(), &"merge-resolution");
        assert!(matches!(
            ownership.claim_for_cleanup(&key),
            Err(OwnershipError::Ineligible)
        ));
        assert!(ownership.discard_for_cleanup(&key));
        assert!(!ownership.contains(&key));
        drop(claim);
    }

    #[test]
    fn catastrophic_cleanup_discards_claimed_exact_slot() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Gate, GATE),
                "shared-resource",
            )
            .unwrap();
        let claim = ownership.claim_for_cleanup(&key).unwrap();

        assert!(ownership.discard_for_cleanup(&key));
        assert!(!ownership.contains(&key));
        drop(claim);
    }
}
