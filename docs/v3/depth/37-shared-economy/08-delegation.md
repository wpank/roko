# Depth: Delegation with Caveats

> Capability narrowing invariant, expiry enforcement, spend limits, scope
> restriction, and automatic revocation on ownership transfer.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 4.3
**Source:** `crates/roko-chain/src/agent_registry.rs`

---

## Caveat Structure

```rust
pub struct DelegationCaveat {
    pub delegatee: AgentId,         // Who receives the delegation
    pub allowed_capabilities: u64,  // Narrowed bitmask
    pub expiry_block: u64,          // When it expires
    pub max_spend: Option<u64>,     // Budget cap
    pub scope: Option<String>,      // Application context
}
```

---

## Narrowing Invariant

The `allowed_capabilities` bitmask must be a strict subset of the delegator's
own capabilities:

```
allowed_capabilities & ~delegator_capabilities == 0
```

A delegation can never grant capabilities the delegator does not possess.
This is enforced at creation time:

```rust
if caveat.allowed_capabilities & !passport.capabilities != 0 {
    return Err(RegistryError::InvalidDelegation);
}
```

### Transitivity

Delegations are not transitive. If A delegates to B, B cannot delegate those
capabilities to C. Only the passport owner can create delegations. This
prevents delegation chains that could circumvent the narrowing invariant.

---

## Expiry

Every delegation has an explicit `expiry_block`. After this timestamp, the
delegation is invalid and the delegatee can no longer act on behalf of the
delegator.

There is no mechanism to extend an expired delegation -- the delegator must
issue a new one. This ensures that stale delegations do not persist
indefinitely.

---

## Spend Limits

The optional `max_spend` field caps the total budget the delegate can commit
on behalf of the delegator. Once the cumulative spend reaches this limit,
the delegation is effectively exhausted.

This provides fine-grained budget authority: a passport owner can delegate
inference capability to a sub-agent with a spend limit, ensuring the
sub-agent cannot consume unlimited resources.

---

## Scope Restriction

The optional `scope` field restricts the delegation to a specific application
context (e.g., a particular plan ID, a specific domain, or a workspace).
Enforcement of scope semantics is application-level -- the registry stores
the scope but does not interpret it.

---

## Transfer Revocation

When passport ownership transfers, ALL delegation caveats are automatically
revoked:

```rust
// On transfer:
self.caveats.remove(&passport_id);
// Emit CaveatRevoked events for each removed delegation
```

This is a critical safety property: delegations are authority grants from a
specific owner. When ownership changes, those authority grants must not survive.
The new owner can issue fresh delegations with full knowledge of what they are
granting.

---

## Events

Delegation mutations emit registry events:

```rust
pub enum AgentRegistryEvent {
    CaveatUpdated { passport_id, delegatee },
    CaveatRevoked { passport_id, delegatee },
    // ...
}
```

These events provide an audit trail of all delegation changes.
