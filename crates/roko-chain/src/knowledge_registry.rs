//! In-memory state machine for the on-chain knowledge-registry contract.
//!
//! The type is deliberately transport-independent: a chain adapter is
//! responsible for authorization and transaction submission, while this
//! module owns deterministic lifecycle transitions and effect/event data.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::phase2::u256;

/// Ninety days in seconds.
pub const STALE_AFTER_SECS: u64 = 90 * 24 * 60 * 60;

/// Lifecycle state for an on-chain knowledge entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryState {
    /// Published and available for validation.
    Active,
    /// A challenge is awaiting governance resolution.
    Challenged,
    /// Validated by at least one independent identity.
    Validated,
    /// Withdrawn after an upheld challenge.
    Retracted,
    /// Not refreshed within the registry staleness window.
    Stale,
}

/// A knowledge entry persisted by the registry contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRegistryEntry {
    /// Stable entry identifier.
    pub entry_id: [u8; 32],
    /// Passport that published the entry.
    pub publisher_id: u256,
    /// Hash of the full content.
    pub content_hash: [u8; 32],
    /// Optional HDC fingerprint. The full vector may remain private.
    pub hdc_fingerprint: Option<Vec<u64>>,
    /// Discovery tags.
    pub tags: Vec<String>,
    /// Current lifecycle state.
    pub state: EntryState,
    /// Number of distinct validators.
    pub validation_count: u32,
    /// Number of challenges opened over the entry's lifetime.
    pub challenge_count: u32,
    /// Unix time at publication.
    pub published_at: u64,
    /// Unix time of the last owner refresh.
    pub last_refreshed: u64,
}

impl KnowledgeRegistryEntry {
    /// Build a publication draft. [`KnowledgeRegistry::publish`] assigns the ID.
    #[must_use]
    pub fn draft(
        publisher_id: u256,
        content_hash: [u8; 32],
        hdc_fingerprint: Option<Vec<u64>>,
        tags: Vec<String>,
        published_at: u64,
    ) -> Self {
        Self {
            entry_id: [0; 32],
            publisher_id,
            content_hash,
            hdc_fingerprint,
            tags,
            state: EntryState::Active,
            validation_count: 0,
            challenge_count: 0,
            published_at,
            last_refreshed: published_at,
        }
    }
}

/// Governance mechanism configured for challenge resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionMode {
    /// N-of-M designated signers.
    #[default]
    Multisig,
    /// A designated domain arbitrator.
    Arbitrator,
    /// Reputation-weighted validator voting.
    ValidatorVote,
}

/// Registry configuration mirrored from the contract deployment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRegistryConfig {
    /// Challenge-resolution mechanism. Authorization is enforced by the chain adapter.
    pub resolution_mode: ResolutionMode,
}

/// A challenge against a published entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    /// Stable challenge identifier.
    pub challenge_id: [u8; 32],
    /// Entry under challenge.
    pub entry_id: [u8; 32],
    /// Challenging passport.
    pub challenger_id: u256,
    /// Hash of counter-evidence.
    pub evidence_hash: [u8; 32],
    /// Human-readable challenge reason.
    pub reason: String,
    /// Unix deadline for governance resolution.
    pub resolution_deadline: u64,
    /// Whether governance has resolved the challenge.
    pub resolved: bool,
    /// Whether the challenge was accepted.
    pub upheld: bool,
}

/// Reputation mutation emitted for a caller to apply to ReputationRegistry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReputationEffect {
    /// Passport receiving the effect.
    pub passport_id: u256,
    /// Reputation domain.
    pub domain: String,
    /// Signed score delta.
    pub delta: f64,
}

/// Durable event payload emitted by registry transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeRegistryEvent {
    /// A new entry was accepted.
    Published {
        /// Entry identifier.
        entry_id: [u8; 32],
        /// Publisher passport.
        publisher_id: u256,
    },
    /// A distinct validator validated an entry.
    Validated {
        /// Entry identifier.
        entry_id: [u8; 32],
        /// Validator passport.
        validator_id: u256,
    },
    /// An entry was challenged.
    Challenged {
        /// Entry identifier.
        entry_id: [u8; 32],
        /// Challenge identifier.
        challenge_id: [u8; 32],
        /// Challenger passport.
        challenger_id: u256,
    },
    /// Governance resolved a challenge.
    ChallengeResolved {
        /// Challenge identifier.
        challenge_id: [u8; 32],
        /// Resolution outcome.
        upheld: bool,
        /// Configured governance mechanism.
        mode: ResolutionMode,
    },
    /// An entry changed lifecycle state.
    StateChanged {
        /// Entry identifier.
        entry_id: [u8; 32],
        /// Previous state.
        from: EntryState,
        /// New state.
        to: EntryState,
    },
}

/// Knowledge-registry state-machine errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KnowledgeRegistryError {
    /// Entry identifier is already registered.
    #[error("knowledge entry already exists")]
    DuplicateEntry,
    /// Entry was not found.
    #[error("knowledge entry not found")]
    EntryNotFound,
    /// Challenge was not found.
    #[error("knowledge challenge not found")]
    ChallengeNotFound,
    /// Challenge identifier is already registered.
    #[error("knowledge challenge already exists")]
    DuplicateChallenge,
    /// The requested transition is not legal from the current state.
    #[error("invalid knowledge entry state: {0:?}")]
    InvalidState(EntryState),
    /// Publisher attempted to validate or challenge its own entry.
    #[error("publisher cannot attest to its own entry")]
    SelfAttestation,
    /// Validator already validated this entry.
    #[error("validator already validated this entry")]
    DuplicateValidation,
    /// Only the publishing passport may refresh the entry.
    #[error("caller is not the entry publisher")]
    NotPublisher,
    /// Challenge is already resolved.
    #[error("challenge is already resolved")]
    AlreadyResolved,
    /// Challenge deadline must be nonzero.
    #[error("challenge resolution deadline must be nonzero")]
    InvalidDeadline,
    /// A persisted snapshot violated a registry invariant.
    #[error("invalid knowledge registry snapshot")]
    InvalidSnapshot,
}

/// Versioned snapshot of the complete knowledge-registry lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRegistrySnapshot {
    /// Snapshot schema version.
    pub schema_version: u32,
    /// Registry configuration.
    pub config: KnowledgeRegistryConfig,
    /// Entries, sorted by ID.
    pub entries: Vec<KnowledgeRegistryEntry>,
    /// Challenges, sorted by ID.
    pub challenges: Vec<Challenge>,
    /// Validators already counted for each entry.
    pub validators: Vec<([u8; 32], Vec<u256>)>,
    /// Undrained lifecycle events.
    pub events: Vec<KnowledgeRegistryEvent>,
    /// Nonce used by deterministic ID generation.
    pub next_nonce: u64,
}

/// In-memory knowledge registry matching the contract lifecycle.
#[derive(Debug, Clone, Default)]
pub struct KnowledgeRegistry {
    config: KnowledgeRegistryConfig,
    entries: HashMap<[u8; 32], KnowledgeRegistryEntry>,
    challenges: HashMap<[u8; 32], Challenge>,
    validators: HashMap<[u8; 32], HashSet<u256>>,
    events: Vec<KnowledgeRegistryEvent>,
    next_nonce: u64,
}

impl KnowledgeRegistry {
    /// Create a registry with the requested resolution mode.
    #[must_use]
    pub fn new(config: KnowledgeRegistryConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    /// Return the configured challenge-resolution mechanism.
    #[must_use]
    pub fn resolution_mode(&self) -> ResolutionMode {
        self.config.resolution_mode
    }

    /// Publish an entry and return its stable identifier.
    ///
    /// A zero ID is deterministically replaced. A nonzero externally assigned
    /// contract ID is retained, which makes this primitive usable on either
    /// side of transaction submission.
    pub fn publish(
        &mut self,
        mut entry: KnowledgeRegistryEntry,
    ) -> Result<[u8; 32], KnowledgeRegistryError> {
        if entry.entry_id == [0; 32] {
            entry.entry_id = self.make_id(
                b"knowledge-entry-v1",
                entry.publisher_id,
                &entry.content_hash,
                entry.published_at,
            );
        }
        if self.entries.contains_key(&entry.entry_id) {
            return Err(KnowledgeRegistryError::DuplicateEntry);
        }

        // Publication always begins a fresh lifecycle; callers cannot inject
        // counts or terminal states through a draft.
        entry.state = EntryState::Active;
        entry.validation_count = 0;
        entry.challenge_count = 0;
        entry.last_refreshed = entry.published_at;
        let entry_id = entry.entry_id;
        let publisher_id = entry.publisher_id;
        self.entries.insert(entry_id, entry);
        self.events.push(KnowledgeRegistryEvent::Published {
            entry_id,
            publisher_id,
        });
        Ok(entry_id)
    }

    /// Validate an entry once per independent passport.
    pub fn validate(
        &mut self,
        entry_id: &[u8; 32],
        validator_id: u256,
    ) -> Result<ReputationEffect, KnowledgeRegistryError> {
        let entry = self
            .entries
            .get(entry_id)
            .ok_or(KnowledgeRegistryError::EntryNotFound)?;
        if entry.publisher_id == validator_id {
            return Err(KnowledgeRegistryError::SelfAttestation);
        }
        if !matches!(entry.state, EntryState::Active | EntryState::Validated) {
            return Err(KnowledgeRegistryError::InvalidState(entry.state));
        }
        if self
            .validators
            .get(entry_id)
            .is_some_and(|validators| validators.contains(&validator_id))
        {
            return Err(KnowledgeRegistryError::DuplicateValidation);
        }

        let entry = self.entries.get_mut(entry_id).expect("entry checked above");
        let previous = entry.state;
        entry.validation_count = entry.validation_count.saturating_add(1);
        entry.state = EntryState::Validated;
        let publisher_id = entry.publisher_id;
        self.validators
            .entry(*entry_id)
            .or_default()
            .insert(validator_id);
        self.events.push(KnowledgeRegistryEvent::Validated {
            entry_id: *entry_id,
            validator_id,
        });
        if previous != EntryState::Validated {
            self.events.push(KnowledgeRegistryEvent::StateChanged {
                entry_id: *entry_id,
                from: previous,
                to: EntryState::Validated,
            });
        }

        Ok(ReputationEffect {
            passport_id: publisher_id,
            domain: "knowledge".to_string(),
            delta: 0.2,
        })
    }

    /// Refresh an entry's staleness clock. Only its publisher may do so.
    pub fn refresh(
        &mut self,
        entry_id: &[u8; 32],
        caller: u256,
        now: u64,
    ) -> Result<(), KnowledgeRegistryError> {
        let entry = self
            .entries
            .get_mut(entry_id)
            .ok_or(KnowledgeRegistryError::EntryNotFound)?;
        if entry.publisher_id != caller {
            return Err(KnowledgeRegistryError::NotPublisher);
        }
        if matches!(entry.state, EntryState::Retracted | EntryState::Challenged) {
            return Err(KnowledgeRegistryError::InvalidState(entry.state));
        }
        let previous = entry.state;
        entry.last_refreshed = entry.last_refreshed.max(now);
        if entry.state == EntryState::Stale {
            entry.state = EntryState::Active;
            self.events.push(KnowledgeRegistryEvent::StateChanged {
                entry_id: *entry_id,
                from: previous,
                to: EntryState::Active,
            });
        }
        Ok(())
    }

    /// Open a challenge and move the entry to `Challenged`.
    pub fn challenge(
        &mut self,
        entry_id: &[u8; 32],
        challenger_id: u256,
        evidence_hash: [u8; 32],
        reason: impl Into<String>,
        resolution_deadline: u64,
    ) -> Result<[u8; 32], KnowledgeRegistryError> {
        if resolution_deadline == 0 {
            return Err(KnowledgeRegistryError::InvalidDeadline);
        }
        let entry = self
            .entries
            .get(entry_id)
            .ok_or(KnowledgeRegistryError::EntryNotFound)?;
        if entry.publisher_id == challenger_id {
            return Err(KnowledgeRegistryError::SelfAttestation);
        }
        if !matches!(entry.state, EntryState::Active | EntryState::Validated) {
            return Err(KnowledgeRegistryError::InvalidState(entry.state));
        }

        let challenge_id = self.make_id(
            b"knowledge-challenge-v1",
            challenger_id,
            &evidence_hash,
            resolution_deadline,
        );
        if self.challenges.contains_key(&challenge_id) {
            return Err(KnowledgeRegistryError::DuplicateChallenge);
        }
        let challenge = Challenge {
            challenge_id,
            entry_id: *entry_id,
            challenger_id,
            evidence_hash,
            reason: reason.into(),
            resolution_deadline,
            resolved: false,
            upheld: false,
        };
        self.challenges.insert(challenge_id, challenge);
        let entry = self.entries.get_mut(entry_id).expect("entry checked above");
        let previous = entry.state;
        entry.state = EntryState::Challenged;
        entry.challenge_count = entry.challenge_count.saturating_add(1);
        self.events.push(KnowledgeRegistryEvent::Challenged {
            entry_id: *entry_id,
            challenge_id,
            challenger_id,
        });
        self.events.push(KnowledgeRegistryEvent::StateChanged {
            entry_id: *entry_id,
            from: previous,
            to: EntryState::Challenged,
        });
        Ok(challenge_id)
    }

    /// Apply a governance-authorized challenge result.
    ///
    /// The caller-facing chain adapter must enforce the configured resolution
    /// mode. This state machine records the mode in the emitted event and never
    /// calls ReputationRegistry directly.
    pub fn resolve_challenge(
        &mut self,
        challenge_id: &[u8; 32],
        upheld: bool,
    ) -> Result<ReputationEffect, KnowledgeRegistryError> {
        let challenge = self
            .challenges
            .get(challenge_id)
            .ok_or(KnowledgeRegistryError::ChallengeNotFound)?;
        if challenge.resolved {
            return Err(KnowledgeRegistryError::AlreadyResolved);
        }
        let entry_id = challenge.entry_id;
        let entry = self
            .entries
            .get(&entry_id)
            .ok_or(KnowledgeRegistryError::EntryNotFound)?;
        if entry.state != EntryState::Challenged {
            return Err(KnowledgeRegistryError::InvalidState(entry.state));
        }

        let next_state = if upheld {
            EntryState::Retracted
        } else {
            EntryState::Validated
        };
        let publisher_id = entry.publisher_id;
        let challenge = self
            .challenges
            .get_mut(challenge_id)
            .expect("challenge checked above");
        challenge.resolved = true;
        challenge.upheld = upheld;
        self.entries
            .get_mut(&entry_id)
            .expect("entry checked above")
            .state = next_state;
        self.events.push(KnowledgeRegistryEvent::ChallengeResolved {
            challenge_id: *challenge_id,
            upheld,
            mode: self.config.resolution_mode,
        });
        self.events.push(KnowledgeRegistryEvent::StateChanged {
            entry_id,
            from: EntryState::Challenged,
            to: next_state,
        });

        Ok(ReputationEffect {
            passport_id: publisher_id,
            domain: "knowledge".to_string(),
            delta: if upheld { -0.3 } else { 0.2 },
        })
    }

    /// Transition refreshable entries older than 90 days to `Stale`.
    pub fn check_staleness(&mut self, now: u64) -> Vec<[u8; 32]> {
        let mut stale_ids: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(entry_id, entry)| {
                (matches!(entry.state, EntryState::Active | EntryState::Validated)
                    && now.saturating_sub(entry.last_refreshed) >= STALE_AFTER_SECS)
                    .then_some(*entry_id)
            })
            .collect();
        stale_ids.sort_unstable();
        for entry_id in &stale_ids {
            let entry = self.entries.get_mut(entry_id).expect("entry id collected");
            let previous = entry.state;
            entry.state = EntryState::Stale;
            self.events.push(KnowledgeRegistryEvent::StateChanged {
                entry_id: *entry_id,
                from: previous,
                to: EntryState::Stale,
            });
        }
        stale_ids
    }

    /// Fetch an entry by ID.
    #[must_use]
    pub fn get_entry(&self, entry_id: &[u8; 32]) -> Option<&KnowledgeRegistryEntry> {
        self.entries.get(entry_id)
    }

    /// Return all entries in stable identifier order.
    #[must_use]
    pub fn entries(&self) -> Vec<&KnowledgeRegistryEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by_key(|entry| entry.entry_id);
        entries
    }

    /// Fetch a challenge by ID.
    #[must_use]
    pub fn get_challenge(&self, challenge_id: &[u8; 32]) -> Option<&Challenge> {
        self.challenges.get(challenge_id)
    }

    /// Query entries containing an exact tag, ordered by entry ID.
    #[must_use]
    pub fn query_by_tag(&self, tag: &str) -> Vec<&KnowledgeRegistryEntry> {
        let mut entries: Vec<_> = self
            .entries
            .values()
            .filter(|entry| entry.tags.iter().any(|candidate| candidate == tag))
            .collect();
        entries.sort_by_key(|entry| entry.entry_id);
        entries
    }

    /// Events emitted since registry construction.
    #[must_use]
    pub fn events(&self) -> &[KnowledgeRegistryEvent] {
        &self.events
    }

    /// Drain emitted events for publication to Bus or an event indexer.
    pub fn drain_events(&mut self) -> Vec<KnowledgeRegistryEvent> {
        std::mem::take(&mut self.events)
    }

    /// Capture all lifecycle state in a JSON-safe, versioned snapshot.
    #[must_use]
    pub fn snapshot(&self) -> KnowledgeRegistrySnapshot {
        let mut entries: Vec<_> = self.entries.values().cloned().collect();
        entries.sort_by_key(|entry| entry.entry_id);
        let mut challenges: Vec<_> = self.challenges.values().cloned().collect();
        challenges.sort_by_key(|challenge| challenge.challenge_id);
        let mut validators: Vec<_> = self
            .validators
            .iter()
            .map(|(entry_id, validators)| {
                let mut validators: Vec<_> = validators.iter().copied().collect();
                validators.sort_unstable();
                (*entry_id, validators)
            })
            .collect();
        validators.sort_by_key(|(entry_id, _)| *entry_id);
        KnowledgeRegistrySnapshot {
            schema_version: 1,
            config: self.config,
            entries,
            challenges,
            validators,
            events: self.events.clone(),
            next_nonce: self.next_nonce,
        }
    }

    /// Restore a snapshot after validating entry, challenge, and validator links.
    pub fn from_snapshot(
        snapshot: KnowledgeRegistrySnapshot,
    ) -> Result<Self, KnowledgeRegistryError> {
        if snapshot.schema_version != 1 {
            return Err(KnowledgeRegistryError::InvalidSnapshot);
        }
        let mut entries = HashMap::with_capacity(snapshot.entries.len());
        for entry in snapshot.entries {
            if entry.entry_id == [0; 32]
                || entry.publisher_id == 0
                || entry.last_refreshed < entry.published_at
                || entries.insert(entry.entry_id, entry).is_some()
            {
                return Err(KnowledgeRegistryError::InvalidSnapshot);
            }
        }
        let mut challenges = HashMap::with_capacity(snapshot.challenges.len());
        for challenge in snapshot.challenges {
            if challenge.challenge_id == [0; 32]
                || !entries.contains_key(&challenge.entry_id)
                || challenge.resolution_deadline == 0
                || (!challenge.resolved && challenge.upheld)
                || entries
                    .get(&challenge.entry_id)
                    .is_some_and(|entry| entry.publisher_id == challenge.challenger_id)
                || challenges
                    .insert(challenge.challenge_id, challenge)
                    .is_some()
            {
                return Err(KnowledgeRegistryError::InvalidSnapshot);
            }
        }
        let mut validators = HashMap::with_capacity(snapshot.validators.len());
        for (entry_id, ids) in snapshot.validators {
            if !entries.contains_key(&entry_id) {
                return Err(KnowledgeRegistryError::InvalidSnapshot);
            }
            let unique: HashSet<_> = ids.iter().copied().collect();
            if unique.len() != ids.len()
                || entries
                    .get(&entry_id)
                    .is_some_and(|entry| unique.contains(&entry.publisher_id))
                || validators.insert(entry_id, unique).is_some()
            {
                return Err(KnowledgeRegistryError::InvalidSnapshot);
            }
        }
        for (entry_id, entry) in &entries {
            let validator_count = validators.get(entry_id).map_or(0, HashSet::len);
            let validator_count = u32::try_from(validator_count)
                .map_err(|_| KnowledgeRegistryError::InvalidSnapshot)?;
            let entry_challenges = challenges
                .values()
                .filter(|challenge| &challenge.entry_id == entry_id)
                .collect::<Vec<_>>();
            let challenge_count = u32::try_from(entry_challenges.len())
                .map_err(|_| KnowledgeRegistryError::InvalidSnapshot)?;
            let unresolved = entry_challenges
                .iter()
                .filter(|challenge| !challenge.resolved)
                .count();
            let upheld = entry_challenges
                .iter()
                .filter(|challenge| challenge.resolved && challenge.upheld)
                .count();
            if entry.validation_count != validator_count
                || entry.challenge_count != challenge_count
                || unresolved > 1
                || (unresolved == 1) != (entry.state == EntryState::Challenged)
                || upheld > 1
                || (upheld == 1) != (entry.state == EntryState::Retracted)
            {
                return Err(KnowledgeRegistryError::InvalidSnapshot);
            }
        }
        if snapshot
            .events
            .iter()
            .any(|event| !knowledge_event_links_are_valid(event, &entries, &challenges))
        {
            return Err(KnowledgeRegistryError::InvalidSnapshot);
        }
        Ok(Self {
            config: snapshot.config,
            entries,
            challenges,
            validators,
            events: snapshot.events,
            next_nonce: snapshot.next_nonce,
        })
    }

    fn make_id(
        &mut self,
        namespace: &[u8],
        actor: u256,
        content: &[u8; 32],
        timestamp: u64,
    ) -> [u8; 32] {
        self.next_nonce = self.next_nonce.saturating_add(1);
        let mut hasher = blake3::Hasher::new();
        hasher.update(namespace);
        hasher.update(&actor.to_le_bytes());
        hasher.update(content);
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&self.next_nonce.to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}

fn knowledge_event_links_are_valid(
    event: &KnowledgeRegistryEvent,
    entries: &HashMap<[u8; 32], KnowledgeRegistryEntry>,
    challenges: &HashMap<[u8; 32], Challenge>,
) -> bool {
    match event {
        KnowledgeRegistryEvent::Published {
            entry_id,
            publisher_id,
        } => entries
            .get(entry_id)
            .is_some_and(|entry| entry.publisher_id == *publisher_id),
        KnowledgeRegistryEvent::Validated {
            entry_id,
            validator_id,
        } => entries
            .get(entry_id)
            .is_some_and(|entry| entry.publisher_id != *validator_id),
        KnowledgeRegistryEvent::Challenged {
            entry_id,
            challenge_id,
            challenger_id,
        } => challenges.get(challenge_id).is_some_and(|challenge| {
            challenge.entry_id == *entry_id && challenge.challenger_id == *challenger_id
        }),
        KnowledgeRegistryEvent::ChallengeResolved {
            challenge_id,
            upheld,
            ..
        } => challenges
            .get(challenge_id)
            .is_some_and(|challenge| challenge.resolved && challenge.upheld == *upheld),
        KnowledgeRegistryEvent::StateChanged { entry_id, .. } => entries.contains_key(entry_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publish(registry: &mut KnowledgeRegistry, at: u64) -> [u8; 32] {
        registry
            .publish(KnowledgeRegistryEntry::draft(
                7,
                [1; 32],
                Some(vec![1, 2, 3]),
                vec!["rust".to_string(), "agents".to_string()],
                at,
            ))
            .unwrap()
    }

    #[test]
    fn publish_validate_and_query_emit_effects_and_events() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1_000);

        assert_eq!(registry.query_by_tag("rust").len(), 1);
        let effect = registry.validate(&entry_id, 9).unwrap();
        assert_eq!(effect.passport_id, 7);
        assert_eq!(effect.domain, "knowledge");
        assert!((effect.delta - 0.2).abs() < f64::EPSILON);
        let entry = registry.get_entry(&entry_id).unwrap();
        assert_eq!(entry.state, EntryState::Validated);
        assert_eq!(entry.validation_count, 1);
        assert!(registry.events().iter().any(|event| matches!(
            event,
            KnowledgeRegistryEvent::Validated {
                validator_id: 9,
                ..
            }
        )));
    }

    #[test]
    fn validation_rejects_self_and_duplicate_attestations() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1_000);
        assert_eq!(
            registry.validate(&entry_id, 7),
            Err(KnowledgeRegistryError::SelfAttestation)
        );
        registry.validate(&entry_id, 9).unwrap();
        assert_eq!(
            registry.validate(&entry_id, 9),
            Err(KnowledgeRegistryError::DuplicateValidation)
        );
        assert_eq!(registry.get_entry(&entry_id).unwrap().validation_count, 1);
    }

    #[test]
    fn staleness_boundary_and_owner_refresh_are_exact() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1_000);
        assert!(registry.check_staleness(999 + STALE_AFTER_SECS).is_empty());
        assert_eq!(
            registry.refresh(&entry_id, 99, 2_000),
            Err(KnowledgeRegistryError::NotPublisher)
        );
        assert_eq!(
            registry.check_staleness(1_000 + STALE_AFTER_SECS),
            vec![entry_id]
        );
        assert_eq!(
            registry.get_entry(&entry_id).unwrap().state,
            EntryState::Stale
        );
        registry.refresh(&entry_id, 7, 9_000_000).unwrap();
        let entry = registry.get_entry(&entry_id).unwrap();
        assert_eq!(entry.state, EntryState::Active);
        assert_eq!(entry.last_refreshed, 9_000_000);
    }

    #[test]
    fn challenge_resolution_has_exact_state_and_reputation_effects() {
        for (upheld, expected_state, expected_delta) in [
            (true, EntryState::Retracted, -0.3),
            (false, EntryState::Validated, 0.2),
        ] {
            let mut registry = KnowledgeRegistry::new(KnowledgeRegistryConfig {
                resolution_mode: ResolutionMode::ValidatorVote,
            });
            let entry_id = publish(&mut registry, 1_000);
            let challenge_id = registry
                .challenge(&entry_id, 11, [8; 32], "counter-evidence", 2_000)
                .unwrap();
            assert_eq!(
                registry.get_entry(&entry_id).unwrap().state,
                EntryState::Challenged
            );
            assert_eq!(registry.get_entry(&entry_id).unwrap().challenge_count, 1);

            let effect = registry.resolve_challenge(&challenge_id, upheld).unwrap();
            assert_eq!(registry.get_entry(&entry_id).unwrap().state, expected_state);
            assert_eq!(effect.passport_id, 7);
            assert!((effect.delta - expected_delta).abs() < f64::EPSILON);
            assert_eq!(
                registry.get_challenge(&challenge_id).unwrap().upheld,
                upheld
            );
            assert_eq!(
                registry.resolve_challenge(&challenge_id, upheld),
                Err(KnowledgeRegistryError::AlreadyResolved)
            );
        }
    }

    #[test]
    fn challenged_and_retracted_entries_do_not_go_stale() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1);
        let challenge_id = registry
            .challenge(&entry_id, 8, [3; 32], "bad", 100)
            .unwrap();
        assert!(registry.check_staleness(u64::MAX).is_empty());
        registry.resolve_challenge(&challenge_id, true).unwrap();
        assert!(registry.check_staleness(u64::MAX).is_empty());
    }

    #[test]
    fn snapshot_round_trip_preserves_challenge_and_validator_deduplication() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1_000);
        registry.validate(&entry_id, 9).unwrap();
        let challenge_id = registry
            .challenge(&entry_id, 10, [4; 32], "counter", 2_000)
            .unwrap();
        let encoded = serde_json::to_vec(&registry.snapshot()).unwrap();
        let snapshot: KnowledgeRegistrySnapshot = serde_json::from_slice(&encoded).unwrap();
        let mut restored = KnowledgeRegistry::from_snapshot(snapshot).unwrap();
        assert_eq!(
            restored.validate(&entry_id, 9),
            Err(KnowledgeRegistryError::InvalidState(EntryState::Challenged))
        );
        restored.resolve_challenge(&challenge_id, false).unwrap();
        assert_eq!(
            restored.validate(&entry_id, 9),
            Err(KnowledgeRegistryError::DuplicateValidation)
        );
    }

    #[test]
    fn snapshot_rejects_orphan_challenges() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1_000);
        let challenge_id = registry
            .challenge(&entry_id, 10, [4; 32], "counter", 2_000)
            .unwrap();
        let mut snapshot = registry.snapshot();
        snapshot.entries.clear();
        assert_eq!(
            KnowledgeRegistry::from_snapshot(snapshot).unwrap_err(),
            KnowledgeRegistryError::InvalidSnapshot
        );
        assert_ne!(challenge_id, [0; 32]);
    }

    #[test]
    fn snapshot_rejects_counter_and_lifecycle_inconsistency() {
        let mut registry = KnowledgeRegistry::default();
        let entry_id = publish(&mut registry, 1_000);
        registry.validate(&entry_id, 9).unwrap();

        let mut wrong_count = registry.snapshot();
        wrong_count.entries[0].validation_count = 0;
        assert_eq!(
            KnowledgeRegistry::from_snapshot(wrong_count).unwrap_err(),
            KnowledgeRegistryError::InvalidSnapshot
        );

        let mut wrong_state = registry.snapshot();
        wrong_state.entries[0].state = EntryState::Challenged;
        assert_eq!(
            KnowledgeRegistry::from_snapshot(wrong_state).unwrap_err(),
            KnowledgeRegistryError::InvalidSnapshot
        );
    }
}
