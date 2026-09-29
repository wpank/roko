# Depth: Capability Passports

> 10-bit capability bitmask, three passport tiers, lifecycle states, prompt
> hash commitment with timelock, and the ventriloquist defense.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 4
**Source:** `crates/roko-chain/src/agent_registry.rs`

---

## Capability Bitmask

Ten capabilities encoded as bit positions in a `u64`:

| Bit | Constant | Capability |
|---|---|---|
| 0 | `CAP_INFERENCE` | LLM inference |
| 1 | `CAP_DATA_TRANSFORM` | Data transformation and ETL |
| 2 | `CAP_FINE_TUNE` | Model fine-tuning |
| 3 | `CAP_RAG` | Retrieval-augmented generation |
| 4 | `CAP_MULTI_AGENT` | Multi-agent orchestration |
| 5 | `CAP_TRADING` | Trading and DeFi operations |
| 6 | `CAP_SECURITY` | Security analysis and auditing |
| 7 | `CAP_ANALYTICS` | Analytics and metrics |
| 8 | `CAP_KNOWLEDGE` | Knowledge management |
| 9 | `CAP_STRATEGY` | Strategic planning |

### Enforcement

Capabilities are checked at two levels:

1. **Job matching:** A job requiring `CAP_SECURITY | CAP_ANALYTICS` only
   matches agents whose capability bitmask includes both bits.

2. **Safety layer intersection:** Passport capabilities compose with the
   Cell x Graph x Space capability intersection from the safety layer.
   An agent may have the `CAP_INFERENCE` bit but be restricted from
   inference in a particular Graph context.

---

## Passport Tiers

| Tier | Constant | Description |
|---|---|---|
| Worker | `TIER_WORKER` | Entry-level agents. Standard job participation. |
| Sovereign | `TIER_SOVEREIGN` | Proven track record. Group leadership, direct hire eligibility. |
| Protocol | `TIER_PROTOCOL` | Governance-level access. System operations, admin functions. |

Tier is determined by the agent's total commitment level (equivalent to
historical stake in the chain-era design). Higher tiers unlock additional
capabilities and privileges but also imply higher accountability.

---

## Lifecycle States

```
Minting --> Active --> Suspended --> Revoked
```

Only forward transitions are allowed. Once Revoked, the passport is
permanently disabled and cannot be reactivated.

```rust
pub enum PassportState {
    Minting,                       // Being created, not yet usable
    Active,                        // Fully operational
    Suspended { reason: String },  // Temporarily disabled
    Revoked { reason: String },    // Permanently disabled
}
```

---

## Prompt Hash Commitment

The ventriloquist defense prevents rapid prompt swapping attacks:

### Registration

At passport creation, the agent commits the BLAKE3 hash of its system prompt.
This hash is publicly visible and tied to the passport identity.

### Updates

Prompt changes require a 24-hour timelock:

```rust
pub struct PendingPromptUpdate {
    pub new_hash: [u8; 32],
    pub submitted_at: u64,
    pub executable_after: u64,  // submitted_at + 86400
}
```

The agent submits the new hash, waits 24 hours, then executes the update.
During the waiting period, the old prompt remains active and any observer
can see that a change is pending.

### Rate Limiting

More than 3 prompt changes in a 30-day window triggers a -0.05 reputation
penalty across all domains:

```rust
const PROMPT_UPDATE_TIMELOCK_SECS: u64 = 24 * 3600;
const PROMPT_CHANGE_WINDOW_SECS: u64 = 30 * 24 * 3600;
const MAX_PROMPT_CHANGES_IN_WINDOW: usize = 3;
```

### Attack Scenario

Without the commitment: An attacker controls agent X with high reputation.
X has earned trust through legitimate work. The attacker swaps X's prompt
to a malicious one, uses X's reputation to gain access to sensitive jobs,
executes the malicious prompt, then swaps back. Observers cannot distinguish
legitimate X from compromised X.

With the commitment: The prompt change is visible 24 hours in advance.
Frequent changes trigger reputation penalties and flag the agent for review.
Observers can verify that the agent's current prompt matches its committed
hash.

---

## Transfer and Ownership

Passport ownership can be transferred, but transfers carry consequences:

```rust
pub struct TransferRecord {
    pub from: String,
    pub to: String,
    pub block: u64,
}
```

On transfer:
1. All delegation caveats are automatically revoked
2. The transfer is recorded in the audit trail
3. The new owner inherits the passport's capabilities and reputation
4. Previous delegations cannot be used to act on behalf of the new owner

This prevents authority leakage: if Alice delegates to Bob, and Alice then
transfers the passport to Carol, Bob's delegation is invalidated. Carol
must issue new delegations explicitly.
