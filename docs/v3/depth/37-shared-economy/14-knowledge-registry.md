# Depth: Knowledge Registry

> Publish/validate/challenge lifecycle, 90-day staleness auto-transition,
> challenge resolution modes, reputation effects, and durable event outbox.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 9
**Source:** `crates/roko-chain/src/knowledge_registry.rs`

---

## Entry Structure

```rust
pub struct KnowledgeRegistryEntry {
    pub entry_id: [u8; 32],
    pub publisher_id: AgentId,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: Option<Vec<u64>>,
    pub tags: Vec<String>,
    pub state: EntryState,
    pub validation_count: u32,
    pub challenge_count: u32,
    pub published_at: u64,
    pub last_refreshed: u64,
}
```

### HDC Fingerprint

The optional hyperdimensional computing fingerprint enables semantic
discovery without full content disclosure. Two entries with similar HDC
vectors are likely semantically related. The full content remains private
-- only the fingerprint is published.

---

## Lifecycle States

```rust
pub enum EntryState {
    Active,      // Published, available for validation
    Challenged,  // Under challenge, awaiting resolution
    Validated,   // At least one independent validator attested
    Retracted,   // Withdrawn after upheld challenge
    Stale,       // Not refreshed within 90 days
}
```

### Transition Rules

| From | To | Trigger |
|---|---|---|
| Active | Validated | Independent validator attests |
| Active | Challenged | Challenger submits counter-evidence |
| Challenged | Active | Challenge rejected by governance |
| Challenged | Retracted | Challenge upheld by governance |
| Active/Validated | Stale | 90 days without refresh |

---

## Self-Attestation Prevention

The publisher cannot validate their own entry:

```rust
if entry.publisher_id == validator_id {
    return Err(KnowledgeRegistryError::SelfAttestation);
}
```

This prevents trivial reputation inflation through self-validation. Each
validation must come from an independent passport.

### Distinct-Validator Counting

Each validator is counted once per entry. Validating the same entry twice
produces an error:

```rust
if entry_validators.contains(&validator_id) {
    return Err(KnowledgeRegistryError::DuplicateValidation);
}
```

---

## Challenge Mechanism

Any agent can challenge an entry by submitting counter-evidence:

```rust
pub struct Challenge {
    pub challenge_id: [u8; 32],
    pub entry_id: [u8; 32],
    pub challenger_id: AgentId,
    pub evidence_hash: [u8; 32],
    pub reason: String,
    pub resolution_deadline: u64,
    pub resolved: bool,
    pub upheld: bool,
}
```

### Resolution Modes

The registry supports three governance mechanisms for resolving challenges:

```rust
pub enum ResolutionMode {
    Multisig,       // N-of-M designated signers
    Arbitrator,     // A designated domain arbitrator
    ValidatorVote,  // Reputation-weighted validator voting
}
```

The mode is configured at registry creation time. The registry itself does
not implement the governance logic -- it defines the transition (Challenged
-> Active or Challenged -> Retracted) and expects an authorized caller to
invoke the resolution.

---

## Reputation Effects

Challenge resolution produces reputation mutations:

```rust
pub struct ReputationEffect {
    pub passport_id: AgentId,
    pub domain: String,
    pub delta: f64,
}
```

| Outcome | Publisher Effect | Challenger Effect |
|---|---|---|
| Challenge upheld | Negative delta (entry was wrong) | Positive delta (correct challenge) |
| Challenge rejected | No change | Negative delta (frivolous challenge) |

These effects are emitted as data -- the caller is responsible for applying
them to the reputation registry.

---

## Staleness Policy

Entries not refreshed within 90 days automatically transition to Stale:

```rust
pub const STALE_AFTER_SECS: u64 = 90 * 24 * 60 * 60;

if now - entry.last_refreshed > STALE_AFTER_SECS {
    entry.state = EntryState::Stale;
}
```

This prevents the registry from accumulating outdated knowledge. Stale entries
can be refreshed by their publisher to return to Active state, but they must
be explicitly maintained.

---

## Event Outbox

Every lifecycle transition emits a durable event:

```rust
pub enum KnowledgeRegistryEvent {
    Published { entry_id, publisher_id },
    Validated { entry_id, validator_id },
    Challenged { entry_id, challenge_id, challenger_id },
    ChallengeResolved { challenge_id, upheld, mode },
    StateChanged { entry_id, from, to },
}
```

Events accumulate in the registry's event buffer and can be drained by the
server layer for persistence, notification, and downstream processing.
