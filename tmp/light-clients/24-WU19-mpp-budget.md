# WU-19: MPP Budget Controller

**Layer**: 4 (depends on WU-17 MppClient)
**Depends on**: WU-17 (MppClient)
**Blocks**: none (standalone safety layer)
**Estimated effort**: 2-3 hours
**Crate**: `crates/roko-chain`
**Feature gate**: `mpp` (same as MppClient)

---

## Overview

Agents with MPP access can drain a wallet without spending controls. This WU adds `MppBudgetPolicy` — a per-agent spending policy that wraps MppClient and enforces limits before any payment executes.

This integrates with roko's existing `AgentContract` safety layer pattern. If a payment would exceed any configured limit, the request is denied with a clear error — no fallback, no bypass.

---

## Pre-read

- `crates/roko-chain/src/mpp_client.rs` — `MppClient`, `pay_one_time()`, session flows (WU-17)
- `crates/roko-chain/src/types.rs` — `ChainError`
- `crates/roko-agent/src/safety/` — `AgentContract` safety layer pattern
- `crates/roko-core/src/config/chain.rs` — `ChainConfig` struct (where `MppBudgetConfig` will live)
- `crates/roko-chain/src/lib.rs` — module registration pattern

---

## Tasks

### 19.1 Create `crates/roko-chain/src/mpp_budget.rs`

**Imports needed**:
```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use crate::types::ChainError;
```

**Types to define (in this order)**:

#### `MppBudgetPolicy` struct
```rust
/// Spending policy for an agent's MPP usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppBudgetPolicy {
    /// Maximum amount per single payment (base units). None = unlimited.
    pub max_per_payment: Option<u64>,
    /// Maximum total spend per hour (base units). None = unlimited.
    pub max_per_hour: Option<u64>,
    /// Maximum total spend per day (base units). None = unlimited.
    pub max_per_day: Option<u64>,
    /// Maximum total lifetime spend (base units). None = unlimited.
    pub max_total: Option<u64>,
    /// Allowlisted service URL prefixes. Empty = allow all.
    pub allowed_services: Vec<String>,
    /// Blocklisted service URL prefixes. Checked after allowlist.
    pub blocked_services: Vec<String>,
}
```

#### `SpendingRecord` struct
```rust
/// Tracks spending for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingRecord {
    /// Agent identifier.
    pub agent_id: String,
    /// Total spent (lifetime, base units).
    pub total_spent: u64,
    /// Spending entries for rolling window calculations.
    pub entries: Vec<SpendingEntry>,
}
```

#### `SpendingEntry` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingEntry {
    pub timestamp_ms: u64,
    pub amount: u64,
    pub service_url: String,
    pub tx_hash: Option<String>,
}
```

#### `MppBudgetController` struct
```rust
/// Budget controller that tracks and enforces spending limits.
pub struct MppBudgetController {
    /// Per-agent spending policies.
    policies: HashMap<String, MppBudgetPolicy>,
    /// Default policy for agents without explicit policy.
    default_policy: MppBudgetPolicy,
    /// Per-agent spending records (mutable, behind RwLock).
    records: Arc<RwLock<HashMap<String, SpendingRecord>>>,
    /// Path to persistence file.
    persist_path: std::path::PathBuf,
}

impl MppBudgetController {
    /// Check if a payment is allowed under the agent's budget policy.
    /// Returns Ok(()) if allowed, Err with reason if denied.
    pub fn check_payment(
        &self,
        agent_id: &str,
        amount: u64,
        service_url: &str,
    ) -> Result<(), ChainError> {
        let policy = self.policies.get(agent_id)
            .unwrap_or(&self.default_policy);

        // 1. Check per-payment limit
        if let Some(max) = policy.max_per_payment {
            if amount > max {
                return Err(ChainError::Other(format!(
                    "Payment {amount} exceeds per-payment limit {max} for agent {agent_id}"
                )));
            }
        }

        // 2. Check service allowlist/blocklist
        if !policy.allowed_services.is_empty() {
            if !policy.allowed_services.iter().any(|s| service_url.starts_with(s)) {
                return Err(ChainError::Other(format!(
                    "Service {service_url} not in allowlist for agent {agent_id}"
                )));
            }
        }
        if policy.blocked_services.iter().any(|s| service_url.starts_with(s)) {
            return Err(ChainError::Other(format!(
                "Service {service_url} is blocked for agent {agent_id}"
            )));
        }

        // 3. Check rolling window limits
        let records = self.records.read().unwrap();
        if let Some(record) = records.get(agent_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // Hourly check
            if let Some(max_hour) = policy.max_per_hour {
                let hour_ago = now.saturating_sub(3_600_000);
                let hourly_total: u64 = record.entries.iter()
                    .filter(|e| e.timestamp_ms >= hour_ago)
                    .map(|e| e.amount)
                    .sum();
                if hourly_total + amount > max_hour {
                    return Err(ChainError::Other(format!(
                        "Payment would exceed hourly limit: {hourly_total} + {amount} > {max_hour}"
                    )));
                }
            }

            // Daily check
            if let Some(max_day) = policy.max_per_day {
                let day_ago = now.saturating_sub(86_400_000);
                let daily_total: u64 = record.entries.iter()
                    .filter(|e| e.timestamp_ms >= day_ago)
                    .map(|e| e.amount)
                    .sum();
                if daily_total + amount > max_day {
                    return Err(ChainError::Other(format!(
                        "Payment would exceed daily limit: {daily_total} + {amount} > {max_day}"
                    )));
                }
            }

            // Lifetime check
            if let Some(max_total) = policy.max_total {
                if record.total_spent + amount > max_total {
                    return Err(ChainError::Other(format!(
                        "Payment would exceed lifetime limit: {} + {amount} > {max_total}",
                        record.total_spent
                    )));
                }
            }
        }

        Ok(())
    }

    /// Record a completed payment.
    pub fn record_payment(
        &self,
        agent_id: &str,
        amount: u64,
        service_url: &str,
        tx_hash: Option<&str>,
    ) {
        let mut records = self.records.write().unwrap();
        let record = records.entry(agent_id.to_string())
            .or_insert_with(|| SpendingRecord {
                agent_id: agent_id.to_string(),
                total_spent: 0,
                entries: Vec::new(),
            });

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        record.total_spent += amount;
        record.entries.push(SpendingEntry {
            timestamp_ms: now,
            amount,
            service_url: service_url.to_string(),
            tx_hash: tx_hash.map(String::from),
        });
    }

    /// Persist spending records to disk.
    pub fn flush(&self) -> Result<(), ChainError> {
        let records = self.records.read().unwrap();
        let json = serde_json::to_string_pretty(&*records)
            .map_err(|e| ChainError::Other(format!("Failed to serialize budget records: {e}")))?;
        std::fs::write(&self.persist_path, json)
            .map_err(|e| ChainError::Other(format!("Failed to write budget file: {e}")))?;
        Ok(())
    }

    /// Load spending records from disk.
    pub fn load(path: &std::path::Path) -> Result<HashMap<String, SpendingRecord>, ChainError> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let json = std::fs::read_to_string(path)
            .map_err(|e| ChainError::Other(format!("Failed to read budget file: {e}")))?;
        serde_json::from_str(&json)
            .map_err(|e| ChainError::Other(format!("Failed to parse budget file: {e}")))
    }
}
```

**Tests**:
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_policy() -> MppBudgetPolicy {
        MppBudgetPolicy {
            max_per_payment: Some(10_000_000),
            max_per_hour: Some(50_000_000),
            max_per_day: Some(200_000_000),
            max_total: Some(1_000_000_000),
            allowed_services: vec![],
            blocked_services: vec![],
        }
    }

    fn test_controller(policy: MppBudgetPolicy) -> MppBudgetController {
        let mut policies = HashMap::new();
        policies.insert("test-agent".to_string(), policy.clone());
        MppBudgetController {
            policies,
            default_policy: policy,
            records: Arc::new(RwLock::new(HashMap::new())),
            persist_path: PathBuf::from("/tmp/test-mpp-budgets.json"),
        }
    }

    #[test]
    fn allows_payment_within_per_payment_limit() {
        let ctrl = test_controller(test_policy());
        assert!(ctrl.check_payment("test-agent", 5_000_000, "https://api.example.com").is_ok());
    }

    #[test]
    fn rejects_payment_exceeding_per_payment_limit() {
        let ctrl = test_controller(test_policy());
        let err = ctrl.check_payment("test-agent", 15_000_000, "https://api.example.com");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("per-payment limit"));
    }

    #[test]
    fn rejects_payment_exceeding_hourly_limit() {
        let ctrl = test_controller(test_policy());
        // Record payments just under the hourly limit
        for _ in 0..5 {
            ctrl.record_payment("test-agent", 9_000_000, "https://api.example.com", None);
        }
        // Next payment should push over the 50M hourly limit
        let err = ctrl.check_payment("test-agent", 9_000_000, "https://api.example.com");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("hourly limit"));
    }

    #[test]
    fn rejects_payment_exceeding_lifetime_limit() {
        let ctrl = test_controller(test_policy());
        // Manually set total_spent near the limit
        {
            let mut records = ctrl.records.write().unwrap();
            records.insert("test-agent".to_string(), SpendingRecord {
                agent_id: "test-agent".to_string(),
                total_spent: 999_000_000,
                entries: vec![],
            });
        }
        let err = ctrl.check_payment("test-agent", 5_000_000, "https://api.example.com");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("lifetime limit"));
    }

    #[test]
    fn allowlist_blocks_unlisted_service() {
        let policy = MppBudgetPolicy {
            allowed_services: vec!["https://api.openai.com".into()],
            ..test_policy()
        };
        let ctrl = test_controller(policy);
        let err = ctrl.check_payment("test-agent", 1_000, "https://api.evil.com");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("not in allowlist"));
    }

    #[test]
    fn allowlist_permits_listed_service() {
        let policy = MppBudgetPolicy {
            allowed_services: vec!["https://api.openai.com".into()],
            ..test_policy()
        };
        let ctrl = test_controller(policy);
        assert!(ctrl.check_payment("test-agent", 1_000, "https://api.openai.com/v1/chat").is_ok());
    }

    #[test]
    fn blocklist_overrides_allowlist() {
        let policy = MppBudgetPolicy {
            allowed_services: vec!["https://api.example.com".into()],
            blocked_services: vec!["https://api.example.com/admin".into()],
            ..test_policy()
        };
        let ctrl = test_controller(policy);
        let err = ctrl.check_payment("test-agent", 1_000, "https://api.example.com/admin/delete");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("blocked"));
    }

    #[test]
    fn unknown_agent_uses_default_policy() {
        let ctrl = test_controller(test_policy());
        // "unknown-agent" has no explicit policy, should use default
        assert!(ctrl.check_payment("unknown-agent", 5_000_000, "https://api.example.com").is_ok());
    }

    #[test]
    fn record_payment_increments_total() {
        let ctrl = test_controller(test_policy());
        ctrl.record_payment("test-agent", 1_000, "https://api.example.com", Some("0xabc"));
        ctrl.record_payment("test-agent", 2_000, "https://api.example.com", None);

        let records = ctrl.records.read().unwrap();
        let record = records.get("test-agent").unwrap();
        assert_eq!(record.total_spent, 3_000);
        assert_eq!(record.entries.len(), 2);
        assert_eq!(record.entries[0].tx_hash.as_deref(), Some("0xabc"));
        assert!(record.entries[1].tx_hash.is_none());
    }

    #[test]
    fn spending_record_serde_roundtrip() {
        let record = SpendingRecord {
            agent_id: "test".into(),
            total_spent: 42,
            entries: vec![SpendingEntry {
                timestamp_ms: 1700000000000,
                amount: 42,
                service_url: "https://api.example.com".into(),
                tx_hash: Some("0xdeadbeef".into()),
            }],
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: SpendingRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_spent, 42);
        assert_eq!(back.entries.len(), 1);
    }

    #[test]
    fn policy_serde_roundtrip() {
        let policy = test_policy();
        let json = serde_json::to_string(&policy).unwrap();
        let back: MppBudgetPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_per_payment, Some(10_000_000));
        assert_eq!(back.max_total, Some(1_000_000_000));
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = std::env::temp_dir().join("roko-mpp-budget-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mpp-budgets.json");

        let ctrl = test_controller(test_policy());
        ctrl.record_payment("agent-a", 1_000, "https://api.example.com", None);
        ctrl.record_payment("agent-b", 2_000, "https://api.other.com", Some("0x123"));
        ctrl.flush().unwrap();

        // Simulate restart: load from disk
        let loaded = MppBudgetController::load(&ctrl.persist_path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("agent-a").unwrap().total_spent, 1_000);
        assert_eq!(loaded.get("agent-b").unwrap().total_spent, 2_000);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

---

### 19.2 Add `MppBudgetConfig` to chain config

**File**: `crates/roko-core/src/config/chain.rs`

Add to `ChainConfig` or as a nested struct under `[chain.mpp.budget]`:

```rust
/// Budget controls for MPP spending.
///
/// ```toml
/// [chain.mpp.budget]
/// max_per_payment = 10000000
/// max_per_hour = 50000000
/// max_per_day = 200000000
/// max_total = 1000000000
/// allowed_services = ["https://api.openai.com", "https://api.anthropic.com"]
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MppBudgetConfig {
    /// Maximum amount per single payment (base units). None = unlimited.
    #[serde(default)]
    pub max_per_payment: Option<u64>,
    /// Maximum total spend per hour (base units). None = unlimited.
    #[serde(default)]
    pub max_per_hour: Option<u64>,
    /// Maximum total spend per day (base units). None = unlimited.
    #[serde(default)]
    pub max_per_day: Option<u64>,
    /// Maximum total lifetime spend (base units). None = unlimited.
    #[serde(default)]
    pub max_total: Option<u64>,
    /// Allowlisted service URL prefixes. Empty = allow all.
    #[serde(default)]
    pub allowed_services: Vec<String>,
    /// Blocklisted service URL prefixes. Checked after allowlist.
    #[serde(default)]
    pub blocked_services: Vec<String>,
}
```

Wire into the existing MPP config section (wherever `[chain.mpp]` lives):
```rust
/// Budget enforcement for agent MPP spending.
#[serde(default)]
pub budget: MppBudgetConfig,
```

Per-agent overrides go through the agent config:
```toml
[agents.researcher.mpp_budget]
max_per_payment = 5000000
allowed_services = ["https://api.perplexity.ai"]
```

**Tests**:
```rust
#[test]
fn mpp_budget_config_deserialize() {
    let toml = r#"
        max_per_payment = 10000000
        max_per_hour = 50000000
        max_per_day = 200000000
        max_total = 1000000000
        allowed_services = ["https://api.openai.com", "https://api.anthropic.com"]
    "#;
    let config: MppBudgetConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.max_per_payment, Some(10_000_000));
    assert_eq!(config.allowed_services.len(), 2);
}

#[test]
fn mpp_budget_config_defaults_to_unlimited() {
    let config = MppBudgetConfig::default();
    assert!(config.max_per_payment.is_none());
    assert!(config.max_per_hour.is_none());
    assert!(config.allowed_services.is_empty());
}
```

---

### 19.3 Wire budget check into MppClient

**File**: `crates/roko-chain/src/mpp_client.rs` (from WU-17)

In `pay_one_time()` and session payment flows, add budget enforcement:

```rust
// Before executing any payment:
if let Some(budget) = &self.budget_controller {
    budget.check_payment(agent_id, amount, service_url)?;
}

// After successful settlement:
if let Some(budget) = &self.budget_controller {
    budget.record_payment(agent_id, amount, service_url, tx_hash.as_deref());
    // Flush periodically (every 10 payments or on error)
    if let Err(e) = budget.flush() {
        tracing::warn!(error = %e, "failed to persist budget records");
    }
}
```

Add `budget_controller` field to `MppClient`:
```rust
pub struct MppClient {
    // ... existing fields ...
    /// Optional budget controller for spending limits.
    budget_controller: Option<Arc<MppBudgetController>>,
}
```

Add constructor that accepts a budget controller:
```rust
impl MppClient {
    /// Create with budget enforcement.
    pub fn with_budget(
        mut self,
        controller: Arc<MppBudgetController>,
    ) -> Self {
        self.budget_controller = Some(controller);
        self
    }
}
```

---

### 19.4 Add `roko learn payments` CLI subcommand

**File**: `crates/roko-cli/src/learn.rs` (or wherever learn subcommands live)

Add a `payments` subcommand that reads `.roko/state/mpp-budgets.json` and displays spending analytics:

```rust
/// Show MPP spending analytics per agent.
fn learn_payments(data_dir: &Path) -> Result<()> {
    let budget_path = data_dir.join("state/mpp-budgets.json");
    let records = MppBudgetController::load(&budget_path)?;

    if records.is_empty() {
        println!("No MPP spending records found.");
        return Ok(());
    }

    for (agent_id, record) in &records {
        println!("Agent: {agent_id}");
        println!("  Total spent: {} base units", record.total_spent);
        println!("  Transactions: {}", record.entries.len());

        if !record.entries.is_empty() {
            // Last 24h spending
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let day_ago = now_ms.saturating_sub(86_400_000);
            let daily: u64 = record.entries.iter()
                .filter(|e| e.timestamp_ms >= day_ago)
                .map(|e| e.amount)
                .sum();
            println!("  Last 24h: {} base units", daily);

            // Top services
            let mut service_totals: HashMap<&str, u64> = HashMap::new();
            for entry in &record.entries {
                *service_totals.entry(&entry.service_url).or_default() += entry.amount;
            }
            println!("  Services:");
            for (url, total) in service_totals {
                println!("    {url}: {total} base units");
            }
        }
        println!();
    }

    Ok(())
}
```

---

### 19.5 Tests: limit enforcement, rolling windows, persistence roundtrip

All tests are defined inline in task 19.1 above. Summary of coverage:

| Test | What it verifies |
|------|-----------------|
| `allows_payment_within_per_payment_limit` | Happy path: payment under limit |
| `rejects_payment_exceeding_per_payment_limit` | Per-payment cap enforcement |
| `rejects_payment_exceeding_hourly_limit` | Rolling 1h window enforcement |
| `rejects_payment_exceeding_lifetime_limit` | Lifetime cap enforcement |
| `allowlist_blocks_unlisted_service` | Service allowlist deny |
| `allowlist_permits_listed_service` | Service allowlist accept (prefix match) |
| `blocklist_overrides_allowlist` | Blocklist takes priority |
| `unknown_agent_uses_default_policy` | Default policy fallback |
| `record_payment_increments_total` | Record keeping correctness |
| `spending_record_serde_roundtrip` | SpendingRecord serialization |
| `policy_serde_roundtrip` | MppBudgetPolicy serialization |
| `persistence_roundtrip` | Write to disk, reload, verify |

---

### 19.6 Register module in `lib.rs`

**File**: `crates/roko-chain/src/lib.rs`

Add after existing module declarations:
```rust
/// MPP budget enforcement — per-agent spending controls.
#[cfg(feature = "mpp")]
pub mod mpp_budget;
```

Add to the pub use section:
```rust
#[cfg(feature = "mpp")]
pub use mpp_budget::{MppBudgetController, MppBudgetPolicy, SpendingRecord, SpendingEntry};
```

---

## Config Surface

### Global defaults

```toml
[chain.mpp.budget]
max_per_payment = 10000000   # 10 USDC
max_per_hour = 50000000      # 50 USDC
max_per_day = 200000000      # 200 USDC
max_total = 1000000000       # 1000 USDC
allowed_services = ["https://api.openai.com", "https://api.anthropic.com"]
```

### Per-agent overrides

```toml
[agents.researcher.mpp_budget]
max_per_payment = 5000000    # researcher gets lower limits
allowed_services = ["https://api.perplexity.ai"]
```

---

## Persistence

**File**: `.roko/state/mpp-budgets.json`

Contains a JSON map of agent ID to `SpendingRecord`. Written on every successful payment (via `flush()`), loaded on startup.

---

## Integration with MppClient

The `MppClient` (WU-17) should call `budget.check_payment()` BEFORE executing any payment, and `budget.record_payment()` AFTER successful settlement. This is wired in WU-17's `pay_one_time()` method.

Flow:
```
Agent requests payment
  → MppClient.pay_one_time(agent_id, amount, service_url)
    → budget.check_payment(agent_id, amount, service_url)  ← DENY if over limit
    → execute on-chain payment
    → budget.record_payment(agent_id, amount, service_url, tx_hash)
    → budget.flush()
```

---

## Verification Checklist

- [ ] `mpp_budget.rs` exists with `MppBudgetPolicy`, `SpendingRecord`, `SpendingEntry`, `MppBudgetController`
- [ ] `MppBudgetConfig` added to chain config (deserializes from `[chain.mpp.budget]`)
- [ ] `MppBudgetController::check_payment()` enforces per-payment, hourly, daily, and lifetime limits
- [ ] `check_payment()` enforces service allowlist and blocklist
- [ ] `record_payment()` tracks spending with timestamps
- [ ] `flush()` persists to `.roko/state/mpp-budgets.json`
- [ ] `load()` restores records from disk
- [ ] `MppClient` calls `check_payment()` before and `record_payment()` after every payment
- [ ] Default policy applies to agents without explicit policy
- [ ] Per-agent policy overrides work
- [ ] `roko learn payments` subcommand shows spending analytics
- [ ] Module registered in `lib.rs` under `#[cfg(feature = "mpp")]`
- [ ] All types serialize/deserialize correctly
- [ ] `cargo test -p roko-chain --features mpp` — all tests pass
- [ ] `cargo clippy -p roko-chain --features mpp --no-deps -- -D warnings` — no warnings
- [ ] `cargo test --workspace` — no breakage
