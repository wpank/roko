//! Exact-attempt ownership primitives for asynchronous runner effects.
//!
//! A claim leaves a claimed slot in the registry while moving its resource
//! payload to the caller. Buffered events therefore observe the claim and are
//! rejected while the caller awaits process or gate shutdown.

use std::collections::HashMap;

use super::deadlines::{MonotonicTime, OwnershipTiming, monotonic_now};
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
    pub timing: OwnershipTiming,
}

impl AttemptOwner {
    pub fn new(phase: AttemptPhase, effect: EffectRef) -> Self {
        Self::new_at(phase, effect, monotonic_now())
    }

    pub fn new_at(phase: AttemptPhase, effect: EffectRef, now: MonotonicTime) -> Self {
        Self {
            phase,
            effect,
            cancellation: CancellationState::None,
            agent: None,
            timing: OwnershipTiming::new(now),
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>, pid: Option<u32>) -> Self {
        self.agent = Some(AgentOwnership {
            agent_id: agent_id.into(),
            pid,
        });
        self
    }

    fn transition_to(&mut self, phase: AttemptPhase, effect: EffectRef, now: MonotonicTime) {
        if self.phase != phase || self.effect != effect {
            self.timing.transition(now);
            if phase == AttemptPhase::Agent {
                self.timing.record_agent_activity(now);
            }
        }
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

    pub fn clear_agent(&mut self) {
        self.owner.agent = None;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineCandidate {
    pub attempt: TaskAttemptRef,
    pub phase: AttemptPhase,
    pub effect: EffectRef,
    pub cancellation: CancellationState,
    pub claimed: bool,
    pub eligible: bool,
    pub timing: OwnershipTiming,
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

    pub(crate) fn resource(&self, attempt: &TaskAttemptRef) -> Option<&R> {
        self.owners.get(attempt).and_then(|slot| {
            if slot.claimed {
                None
            } else {
                slot.resource.as_ref()
            }
        })
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

    pub fn deadline_candidates(&self) -> Vec<DeadlineCandidate> {
        let mut candidates = self
            .owners
            .iter()
            .map(|(attempt, slot)| DeadlineCandidate {
                attempt: attempt.clone(),
                phase: slot.owner.phase,
                effect: slot.owner.effect,
                cancellation: slot.owner.cancellation,
                claimed: slot.claimed,
                eligible: !slot.claimed && slot.owner.cancellation == CancellationState::None,
                timing: slot.owner.timing,
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| candidate.attempt.key());
        candidates
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

    pub fn record_agent_activity(
        &mut self,
        attempt: &TaskAttemptRef,
        effect: EffectRef,
        now: MonotonicTime,
    ) -> bool {
        let Some(slot) = self.owners.get_mut(attempt) else {
            return false;
        };
        if slot.claimed
            || slot.owner.phase != AttemptPhase::Agent
            || slot.owner.effect != effect
            || slot.owner.cancellation != CancellationState::None
        {
            return false;
        }
        slot.owner.timing.record_agent_activity(now);
        true
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
        self.claim_cancellation_exact(attempt, None)
    }

    pub fn claim_cancellation_exact(
        &mut self,
        attempt: &TaskAttemptRef,
        expected: Option<(AttemptPhase, EffectRef)>,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        let Some(slot) = self.owners.get_mut(attempt) else {
            return Err(OwnershipError::Missing);
        };
        if slot.claimed
            || slot.resource.is_none()
            || slot.owner.cancellation == CancellationState::Cancelling
            || expected.is_some_and(|(phase, effect)| {
                slot.owner.phase != phase || slot.owner.effect != effect
            })
        {
            return Err(OwnershipError::Ineligible);
        }
        let nonce = Self::advance_claim_nonce(&mut self.next_nonce, slot);
        slot.owner.cancellation = CancellationState::Cancelling;
        let owner = slot.owner.clone();
        // Safety: resource.is_none() was checked above before any mutation.
        let resource = slot.resource.take().unwrap();
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
        self.complete_claim_recoverable(claim)
            .map_err(|failure| failure.error)
    }

    pub fn complete_claim_recoverable(
        &mut self,
        claim: AttemptClaim<R>,
    ) -> Result<(), ClaimFailure<R>> {
        let Some(slot) = self.owners.get(&claim.attempt) else {
            return Err(ClaimFailure {
                error: OwnershipError::Missing,
                claim,
            });
        };
        if !slot.claimed || slot.resource.is_some() || slot.claim_nonce != Some(claim.nonce) {
            return Err(ClaimFailure {
                error: OwnershipError::Ineligible,
                claim,
            });
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
        claim.owner.transition_to(phase, effect, monotonic_now());
        self.restore_claim(claim)
    }

    pub fn transition_claim_at(
        &mut self,
        mut claim: AttemptClaim<R>,
        phase: AttemptPhase,
        effect: EffectRef,
        now: MonotonicTime,
    ) -> Result<(), ClaimFailure<R>> {
        claim.owner.transition_to(phase, effect, now);
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

    pub(crate) fn unrecovered_attempts(&self) -> Vec<TaskAttemptRef> {
        self.unrecovered_claims
            .iter()
            .map(|claim| claim.attempt.clone())
            .collect()
    }

    pub(crate) fn retry_unrecovered_claims(&mut self) {
        let claims = std::mem::take(&mut self.unrecovered_claims);
        for claim in claims {
            if let Err(claim) = self.force_restore_cancellation_failure(claim) {
                self.unrecovered_claims.push(claim);
            }
        }
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
        agents.extend(self.unrecovered_claims.iter().filter_map(|claim| {
            claim
                .owner
                .agent
                .as_ref()
                .map(|agent| (claim.attempt.key(), agent.agent_id.clone(), agent.pid))
        }));
        agents.sort_by(|left, right| left.0.cmp(&right.0));
        SurvivingAgentMetadata {
            active: !agents.is_empty(),
            pids: agents.iter().filter_map(|(_, _, pid)| *pid).collect(),
            agent_ids: agents.into_iter().map(|(_, id, _)| id).collect(),
        }
    }

    /// Mark a slot as claimed and return a fresh nonce.
    ///
    /// Callers must verify `slot.resource.is_some()` **before** calling this
    /// function; it unconditionally sets `claimed = true`.
    ///
    /// Accepts `next_nonce` by mutable reference to avoid double-borrowing
    /// `self` when the slot is already borrowed from `self.owners`.
    fn advance_claim_nonce(next_nonce: &mut u64, slot: &mut OwnershipSlot<R>) -> u64 {
        slot.claimed = true;
        let nonce = *next_nonce;
        *next_nonce = next_nonce.wrapping_add(1).max(1);
        slot.claim_nonce = Some(nonce);
        nonce
    }

    fn take_resource(
        &mut self,
        attempt: &TaskAttemptRef,
    ) -> Result<AttemptClaim<R>, OwnershipError> {
        let slot = self
            .owners
            .get_mut(attempt)
            .ok_or(OwnershipError::Missing)?;
        if slot.resource.is_none() {
            return Err(OwnershipError::Ineligible);
        }
        let nonce = Self::advance_claim_nonce(&mut self.next_nonce, slot);
        // Safety: resource.is_none() was checked above before any mutation.
        let resource = slot.resource.take().unwrap();
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
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("returned", Some(41)),
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
    fn mixed_cancellation_preserves_failed_owner_and_sibling_metadata() {
        let failed = TaskAttemptRef::new("plan", "a-failed", 1);
        let sibling = TaskAttemptRef::new("plan", "z-sibling", 1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                sibling.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("sibling-agent", Some(52)),
                "sibling-handle",
            )
            .unwrap();
        ownership
            .insert(
                failed.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("failed-agent", Some(51)),
                "failed-handle",
            )
            .unwrap();

        let failed_claim = ownership.claim_cancellation(&failed).unwrap();
        ownership
            .restore_cancellation_failure(failed_claim)
            .unwrap();

        assert_eq!(
            ownership.surviving_agent_metadata(),
            SurvivingAgentMetadata {
                active: true,
                pids: vec![51, 52],
                agent_ids: vec!["failed-agent".to_string(), "sibling-agent".to_string()],
            }
        );
        assert_eq!(
            ownership.cancellation_state(&failed),
            Some(CancellationState::CancellationFailed)
        );
        assert_eq!(
            ownership.cancellation_state(&sibling),
            Some(CancellationState::None)
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
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("returned", Some(41)),
                "returned-handle",
            )
            .unwrap();
        let claim = ownership.claim_cancellation(&key).unwrap();
        ownership.owners.insert(
            key.clone(),
            OwnershipSlot {
                owner: AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("live", Some(42)),
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
        ownership.retain_unrecovered_claim(refused);
        let survivors = ownership.surviving_agent_metadata();
        assert!(survivors.pids.contains(&41));
        assert!(survivors.pids.contains(&42));
        assert_eq!(ownership.unrecovered_attempts(), vec![key]);
    }

    #[test]
    fn claims_and_transitions_preserve_monotonic_attempt_clocks() {
        let key = attempt(1);
        let started = MonotonicTime::from_millis(10);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(AttemptPhase::Agent, AGENT, started),
                "handle",
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        ownership
            .transition_claim_at(
                claim,
                AttemptPhase::Agent,
                AGENT,
                MonotonicTime::from_millis(20),
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        assert_eq!(claim.owner().timing.phase_started_at, started);
        ownership
            .transition_claim_at(
                claim,
                AttemptPhase::Gate,
                GATE,
                MonotonicTime::from_millis(30),
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Gate, GATE)
            .unwrap();
        assert_eq!(claim.owner().timing.attempt_started_at, started);
        assert_eq!(claim.owner().timing.last_agent_activity_at, started);
        assert_eq!(
            claim.owner().timing.phase_started_at,
            MonotonicTime::from_millis(30)
        );
    }

    #[test]
    fn agent_activity_requires_exact_eligible_agent_owner() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(AttemptPhase::Agent, AGENT, MonotonicTime::from_millis(1)),
                (),
            )
            .unwrap();
        assert!(!ownership.record_agent_activity(
            &key,
            EffectRef(999),
            MonotonicTime::from_millis(5)
        ));
        assert!(ownership.record_agent_activity(&key, AGENT, MonotonicTime::from_millis(6)));
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        assert_eq!(
            claim.owner().timing.last_agent_activity_at,
            MonotonicTime::from_millis(6)
        );
    }

    #[test]
    fn stale_agent_activity_cannot_move_owner_clock_backwards() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(AttemptPhase::Agent, AGENT, MonotonicTime::from_millis(1)),
                (),
            )
            .unwrap();

        assert!(ownership.record_agent_activity(&key, AGENT, MonotonicTime::from_millis(10)));
        assert!(ownership.record_agent_activity(&key, AGENT, MonotonicTime::from_millis(5)));
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        assert_eq!(
            claim.owner().timing.last_agent_activity_at,
            MonotonicTime::from_millis(10)
        );
    }

    #[test]
    fn agent_chatter_advances_only_silence_clock() {
        let key = attempt(1);
        let started = MonotonicTime::from_millis(10);
        let activity = MonotonicTime::from_millis(20);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(AttemptPhase::Agent, AGENT, started),
                (),
            )
            .unwrap();
        let before = ownership.deadline_candidates().pop().unwrap().timing;

        assert!(ownership.record_agent_activity(&key, AGENT, activity));
        let after = ownership.deadline_candidates().pop().unwrap().timing;

        assert_eq!(after.attempt_started_at, before.attempt_started_at);
        assert_eq!(after.phase_started_at, before.phase_started_at);
        assert_eq!(after.last_agent_activity_at, activity);
    }

    #[test]
    fn cancellation_failure_recovery_preserves_all_owner_clocks() {
        let key = attempt(1);
        let started = MonotonicTime::from_millis(10);
        let activity = MonotonicTime::from_millis(20);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(AttemptPhase::Agent, AGENT, started),
                "handle",
            )
            .unwrap();
        assert!(ownership.record_agent_activity(&key, AGENT, activity));

        let claim = ownership.claim_cancellation(&key).unwrap();
        ownership.restore_cancellation_failure(claim).unwrap();
        let retry = ownership.claim_cancellation(&key).unwrap();

        assert_eq!(retry.owner().timing.attempt_started_at, started);
        assert_eq!(retry.owner().timing.phase_started_at, started);
        assert_eq!(retry.owner().timing.last_agent_activity_at, activity);
    }

    #[test]
    fn entering_agent_resets_phase_and_silence_baselines_only() {
        let key = attempt(1);
        let started = MonotonicTime::from_millis(10);
        let entered = MonotonicTime::from_millis(30);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(AttemptPhase::Dispatching, EffectRef(0), started),
                (),
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Dispatching, EffectRef(0))
            .unwrap();
        ownership
            .transition_claim_at(claim, AttemptPhase::Agent, AGENT, entered)
            .unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, AGENT)
            .unwrap();
        assert_eq!(claim.owner().timing.attempt_started_at, started);
        assert_eq!(claim.owner().timing.phase_started_at, entered);
        assert_eq!(claim.owner().timing.last_agent_activity_at, entered);
    }

    #[test]
    fn deadline_candidates_expose_identity_and_ineligibility_without_payload() {
        let active = attempt(1);
        let cancelling = TaskAttemptRef::new("plan", "cancel", 1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                active.clone(),
                AttemptOwner::new_at(AttemptPhase::Agent, AGENT, MonotonicTime::from_millis(4)),
                "active-handle",
            )
            .unwrap();
        ownership
            .insert(
                cancelling.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "cancel-handle",
            )
            .unwrap();
        let _claim = ownership.claim_cancellation(&cancelling).unwrap();

        let candidates = ownership.deadline_candidates();
        let active_candidate = candidates
            .iter()
            .find(|candidate| candidate.attempt == active)
            .unwrap();
        assert!(active_candidate.eligible);
        assert_eq!(active_candidate.effect, AGENT);
        assert_eq!(
            active_candidate.timing.attempt_started_at,
            MonotonicTime::from_millis(4)
        );
        let cancelling_candidate = candidates
            .iter()
            .find(|candidate| candidate.attempt == cancelling)
            .unwrap();
        assert!(cancelling_candidate.claimed);
        assert!(!cancelling_candidate.eligible);
        assert_eq!(
            cancelling_candidate.cancellation,
            CancellationState::Cancelling
        );
    }

    #[test]
    fn stale_deadline_candidate_cannot_claim_replacement_effect() {
        let key = attempt(1);
        let old_effect = EffectRef(40);
        let replacement_effect = EffectRef(41);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new_at(
                    AttemptPhase::Agent,
                    old_effect,
                    MonotonicTime::from_millis(1),
                ),
                "replacement-handle",
            )
            .unwrap();
        let stale = ownership.deadline_candidates().pop().unwrap();
        let claim = ownership
            .claim_phase(&key, AttemptPhase::Agent, old_effect)
            .unwrap();
        ownership
            .transition_claim_at(
                claim,
                AttemptPhase::Agent,
                replacement_effect,
                MonotonicTime::from_millis(2),
            )
            .unwrap();

        assert!(matches!(
            ownership.claim_cancellation_exact(&key, Some((stale.phase, stale.effect)),),
            Err(OwnershipError::Ineligible)
        ));
        assert!(ownership.event_is_eligible(&key, AttemptPhase::Agent, replacement_effect));
        let replacement = ownership
            .claim_phase(&key, AttemptPhase::Agent, replacement_effect)
            .unwrap();
        assert_eq!(replacement.resource(), &"replacement-handle");
    }

    #[test]
    fn duplicate_deadline_claim_has_one_linear_winner() {
        let key = attempt(1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                key.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT),
                "handle",
            )
            .unwrap();

        let winner = ownership
            .claim_cancellation_exact(&key, Some((AttemptPhase::Agent, AGENT)))
            .unwrap();
        assert!(matches!(
            ownership.claim_cancellation_exact(&key, Some((AttemptPhase::Agent, AGENT))),
            Err(OwnershipError::Ineligible)
        ));
        assert_eq!(winner.resource(), &"handle");
    }

    #[test]
    fn missing_resource_claims_leave_slot_and_nonce_unchanged() {
        let key = attempt(1);
        let owner =
            AttemptOwner::new_at(AttemptPhase::Agent, AGENT, MonotonicTime::from_millis(17));
        let mut ownership = AttemptOwnership::<&'static str>::default();
        ownership.next_nonce = 41;
        ownership.owners.insert(
            key.clone(),
            OwnershipSlot {
                owner: owner.clone(),
                resource: None,
                claimed: false,
                claim_nonce: None,
            },
        );

        assert!(matches!(
            ownership.claim_cancellation_exact(&key, Some((AttemptPhase::Agent, AGENT)),),
            Err(OwnershipError::Ineligible)
        ));
        assert_eq!(ownership.next_nonce, 41);
        let slot = &ownership.owners[&key];
        assert_eq!(slot.owner, owner);
        assert_eq!(slot.resource, None);
        assert!(!slot.claimed);
        assert_eq!(slot.claim_nonce, None);
        assert_eq!(slot.owner.cancellation, CancellationState::None);

        assert!(matches!(
            ownership.take_resource(&key),
            Err(OwnershipError::Ineligible)
        ));
        assert_eq!(ownership.next_nonce, 41);
        let slot = &ownership.owners[&key];
        assert_eq!(slot.owner, owner);
        assert_eq!(slot.resource, None);
        assert!(!slot.claimed);
        assert_eq!(slot.claim_nonce, None);
        assert_eq!(slot.owner.cancellation, CancellationState::None);
    }

    #[test]
    fn quarantined_agent_metadata_has_stable_attempt_order() {
        let quarantined = TaskAttemptRef::new("plan", "a-quarantined", 1);
        let live = TaskAttemptRef::new("plan", "z-live", 1);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                quarantined.clone(),
                AttemptOwner::new(AttemptPhase::Agent, AGENT)
                    .with_agent("quarantined-agent", Some(61)),
                "quarantined-handle",
            )
            .unwrap();
        let claim = ownership.claim_cancellation(&quarantined).unwrap();
        ownership.owners.insert(
            quarantined.clone(),
            OwnershipSlot {
                owner: AttemptOwner::new(AttemptPhase::Gate, GATE),
                resource: Some("replacement"),
                claimed: false,
                claim_nonce: None,
            },
        );
        let claim = ownership
            .force_restore_cancellation_failure(claim)
            .unwrap_err();
        ownership.retain_unrecovered_claim(claim);
        ownership
            .insert(
                live,
                AttemptOwner::new(AttemptPhase::Agent, AGENT).with_agent("live-agent", Some(62)),
                "live-handle",
            )
            .unwrap();

        assert_eq!(
            ownership.surviving_agent_metadata(),
            SurvivingAgentMetadata {
                active: true,
                pids: vec![61, 62],
                agent_ids: vec!["quarantined-agent".to_string(), "live-agent".to_string()],
            }
        );
        assert_eq!(ownership.unrecovered_attempts(), vec![quarantined]);
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
