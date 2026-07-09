# Demurrage + Tier Progression — Implementation Prompt

> **Goal**: Add demurrage (attention-weighted decay) and tier progression to Signals.
> This is the economic foundation the unified spec builds on — Signals cost to hold,
> forcing the system to prioritize actively useful knowledge.
>
> **Spec**: `tmp/unified/01-SIGNAL.md` §6 (Demurrage), `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md`

## Context

Currently `Engram` has a `decay: Decay` field with simple time-based exponential decay.
The unified spec replaces this with economic demurrage (Gesell 1916):

- Signals have a `balance` that decreases over time
- Retrieving/citing a Signal **reinforces** it (increases balance)
- Gate passes referencing a Signal reinforce it
- When balance reaches zero, the Signal is eligible for cold storage/archival
- Signals progress through 4 tiers: Transient → Working → Consolidated → Persistent

### Files to read first
```
crates/roko-core/src/engram.rs           — current Engram struct
crates/roko-core/src/kind.rs             — Kind enum
tmp/unified/01-SIGNAL.md §6              — demurrage spec
tmp/unified/11-MEMORY-AND-KNOWLEDGE.md   — tier progression, cold storage
crates/roko-neuro/src/knowledge_store.rs — current knowledge store
crates/roko-neuro/src/tier.rs            — existing tier types (if any)
```

---

## Tasks

### DT001 — Add demurrage fields to Engram

**File**: `crates/roko-core/src/engram.rs`

**Steps**:
1. Add new fields (with serde defaults for backwards compat):
   ```rust
   /// Economic balance. Decreases via demurrage, increases via reinforcement.
   #[serde(default = "default_balance")]
   pub balance: f64,

   /// Cumulative demurrage paid since creation.
   #[serde(default)]
   pub demurrage_paid: f64,

   /// Last time this Signal was retrieved, cited, or reinforced.
   #[serde(default)]
   pub last_touched_at: Option<DateTime<Utc>>,

   /// Knowledge tier: Transient → Working → Consolidated → Persistent.
   #[serde(default)]
   pub tier: SignalTier,
   ```
2. Define `SignalTier` enum:
   ```rust
   #[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
   pub enum SignalTier {
       #[default]
       Transient,
       Working,
       Consolidated,
       Persistent,
   }
   ```
3. Default balance: 1.0 for new Signals
4. Ensure existing JSONL files deserialize correctly (serde defaults handle this)

**Verification**: `cargo check --workspace && cargo test --workspace`

---

### DT002 — Implement demurrage rate law

**File**: `crates/roko-core/src/demurrage.rs` (create new)

**Steps**:
1. Implement the rate law from spec §6:
   ```rust
   /// Apply demurrage to a signal over elapsed time.
   ///
   /// balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt
   ///
   /// Where:
   /// - r = base_rate (constant drain, e.g., 0.001 per hour)
   /// - beta = proportional_rate (scales with balance, e.g., 0.01 per hour)
   /// - dt = elapsed hours since last_touched_at
   pub fn apply_demurrage(signal: &mut Engram, now: DateTime<Utc>, config: &DemurrageConfig) {
       let dt = hours_since(signal.last_touched_at.unwrap_or(signal.created_at), now);
       let cost = config.base_rate * dt + config.proportional_rate * signal.balance * dt;
       let cost = cost.min(signal.balance);  // can't go negative
       signal.balance -= cost;
       signal.demurrage_paid += cost;
   }
   ```
2. Define `DemurrageConfig`:
   ```rust
   pub struct DemurrageConfig {
       pub base_rate: f64,          // per hour, default 0.001
       pub proportional_rate: f64,  // per hour, default 0.01
   }
   ```
3. Implement `pub fn reinforce(signal: &mut Engram, amount: f64, now: DateTime<Utc>)`:
   - Adds `amount` to balance
   - Updates `last_touched_at` to `now`

**Verification**: unit tests with known time intervals

---

### DT003 — Implement tier progression

**File**: `crates/roko-core/src/demurrage.rs`

**Steps**:
1. Define promotion criteria (from spec):
   - Transient → Working: cited 3+ times, balance > 0.5
   - Working → Consolidated: cited 10+ times, balance > 0.3, age > 24h
   - Consolidated → Persistent: cited 25+ times, gate-verified, balance > 0.2
2. Define demotion criteria:
   - Any tier → Transient: balance < 0.05
3. Implement `pub fn evaluate_tier(signal: &Engram) -> SignalTier`
4. Implement `pub fn maybe_promote(signal: &mut Engram) -> bool`

---

### DT004 — Wire demurrage into Store operations

**File**: `crates/roko-fs/src/` (FileSubstrate / FileStore)

**Steps**:
1. On `get()`: call `reinforce()` (retrieving a Signal keeps it alive)
2. On `query()`: apply demurrage to all returned Signals (lazy evaluation)
3. On `put()`: set initial balance = 1.0, tier = Transient
4. Add `prune_by_balance(threshold: f64)` — archive Signals with balance below threshold

---

### DT005 — Wire into knowledge store

**File**: `crates/roko-neuro/src/knowledge_store.rs`

**Steps**:
1. On knowledge query: reinforce matching entries
2. On gate pass citing knowledge: reinforce cited entries with bonus
3. Periodic: apply demurrage to all entries (can be triggered by plan runner or cron)
4. Archive entries with balance < 0.05 to cold storage

---

### DT006 — Add demurrage config to roko.toml

**File**: `crates/roko-core/src/config/schema.rs` (or the extracted config module)

**Steps**:
1. Add `[demurrage]` section:
   ```toml
   [demurrage]
   base_rate = 0.001        # per hour
   proportional_rate = 0.01  # per hour
   reinforce_amount = 0.1   # per retrieval
   prune_threshold = 0.05   # archive below this
   ```
2. Wire into DemurrageConfig

**Verification**:
```bash
cargo check --workspace
cargo test --workspace
```

---

## Expected Result

- Engram has `balance`, `demurrage_paid`, `last_touched_at`, `tier` fields
- Retrieving knowledge reinforces it (balance goes up)
- Unused knowledge decays (balance goes down via demurrage)
- Knowledge below threshold gets archived
- Tier progression tracks confidence level
- Existing JSONL files load cleanly (serde defaults)
