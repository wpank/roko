# WU-20: Payment Receipt Ledger

**Layer**: 4 (depends on WU-17 MppClient)
**Depends on**: WU-17 (MppClient)
**Blocks**: none
**Estimated effort**: 2 hours
**Crate**: `crates/roko-chain`
**Feature gate**: `mpp`

---

## Overview

Every MPP payment and verified transfer should be recorded in a local append-only ledger. This mirrors roko's existing patterns: episodes → `.roko/episodes.jsonl`, signals → `.roko/signals.jsonl`, efficiency → `.roko/learn/efficiency.jsonl`.

The payment ledger lives at `.roko/payments.jsonl` and records every payment with full verification metadata.

---

## Pre-read

- `crates/roko-chain/src/lib.rs` — module registration for roko-chain
- `crates/roko-fs/src/substrate.rs` — `FileSubstrate` JSONL append pattern (reference implementation)
- `crates/roko-learn/src/efficiency.rs` — `.roko/learn/efficiency.jsonl` writer (closest pattern)
- `crates/roko-cli/src/orchestrate.rs` — episode/efficiency append sites (see how `.roko/` paths are resolved)
- `crates/roko-cli/src/learn.rs` — `roko learn` subcommands (where to add `payments`)
- `crates/roko-serve/src/routes/` — route registration pattern for `/api/payments/summary`

---

## Tasks

### 20.1 Create `crates/roko-chain/src/payment_ledger.rs`

```rust
//! Payment receipt ledger — append-only JSONL log of all MPP payments
//! and verified transfers.

use serde::{Deserialize, Serialize};

/// A single payment ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEntry {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Agent that initiated or received the payment.
    pub agent_id: String,
    /// Direction: "outgoing" (agent paid) or "incoming" (agent received).
    pub direction: PaymentDirection,
    /// Payment method: "mpp_one_time", "mpp_session", "direct_transfer".
    pub method: String,
    /// Service URL (for MPP payments).
    pub service_url: Option<String>,
    /// Amount in base units.
    pub amount: String,
    /// Token contract address.
    pub token: String,
    /// Settlement details.
    pub settlement: Option<SettlementRecord>,
    /// Associated task ID (if payment was part of a plan execution).
    pub task_id: Option<String>,
    /// Associated episode hash.
    pub episode_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub chain_id: u64,
    pub trust_level: String,
    pub consensus_mechanism: String,
}

/// The payment ledger writer.
pub struct PaymentLedger {
    path: std::path::PathBuf,
}

impl PaymentLedger {
    pub fn new(roko_dir: &std::path::Path) -> Self {
        Self {
            path: roko_dir.join("payments.jsonl"),
        }
    }

    /// Append a payment entry.
    pub fn record(&self, entry: &PaymentEntry) -> Result<(), std::io::Error> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let json = serde_json::to_string(entry)?;
        writeln!(file, "{json}")?;
        Ok(())
    }

    /// Read all entries (for analytics).
    pub fn read_all(&self) -> Result<Vec<PaymentEntry>, std::io::Error> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.path)?;
        let entries: Vec<PaymentEntry> = content.lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(entries)
    }

    /// Compute spending summary.
    pub fn summary(&self) -> Result<SpendingSummary, std::io::Error> {
        let entries = self.read_all()?;
        let total_outgoing: u64 = entries.iter()
            .filter(|e| matches!(e.direction, PaymentDirection::Outgoing))
            .filter_map(|e| e.amount.parse::<u64>().ok())
            .sum();
        let total_incoming: u64 = entries.iter()
            .filter(|e| matches!(e.direction, PaymentDirection::Incoming))
            .filter_map(|e| e.amount.parse::<u64>().ok())
            .sum();
        let total_payments = entries.len();
        let verified_count = entries.iter()
            .filter(|e| e.settlement.as_ref().map_or(false, |s| s.trust_level == "cryptographic"))
            .count();

        // Per-service breakdown
        let mut by_service: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for e in &entries {
            if let (Some(url), PaymentDirection::Outgoing) = (&e.service_url, &e.direction) {
                if let Ok(amount) = e.amount.parse::<u64>() {
                    *by_service.entry(url.clone()).or_default() += amount;
                }
            }
        }

        // Per-agent breakdown
        let mut by_agent: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for e in &entries {
            if matches!(e.direction, PaymentDirection::Outgoing) {
                if let Ok(amount) = e.amount.parse::<u64>() {
                    *by_agent.entry(e.agent_id.clone()).or_default() += amount;
                }
            }
        }

        Ok(SpendingSummary {
            total_outgoing,
            total_incoming,
            total_payments,
            verified_count,
            by_service,
            by_agent,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct SpendingSummary {
    pub total_outgoing: u64,
    pub total_incoming: u64,
    pub total_payments: usize,
    pub verified_count: usize,
    pub by_service: std::collections::HashMap<String, u64>,
    pub by_agent: std::collections::HashMap<String, u64>,
}
```

### 20.2 Wire ledger into MppClient

After every successful payment in `MppClient`, record the result to the ledger. The `MppClient` should accept an optional `PaymentLedger` reference and call `ledger.record()` after settlement completes.

Modify `MppClient::new()` (or add a builder method) to accept a ledger:

```rust
impl MppClient {
    pub fn with_ledger(mut self, ledger: Arc<PaymentLedger>) -> Self {
        self.ledger = Some(ledger);
        self
    }
}
```

After each successful `pay_one_time()`, `pay_session()`, or `direct_transfer()`, build a `PaymentEntry` from the result and call `self.ledger.as_ref().map(|l| l.record(&entry))`.

### 20.3 Add `roko learn payments` CLI subcommand

**File**: `crates/roko-cli/src/learn.rs`

Add a `payments` arm to the `roko learn` subcommand that reads the ledger and prints a summary table:

```
Payment Ledger Summary
━━━━━━━━━━━━━━━━━━━━━━
Total outgoing:  125.50 USDC (126 payments)
Total incoming:    0.00 USDC (0 payments)
Verified:        119/126 (94.4% cryptographic)

By Service:
  api.openai.com/v1          82.30 USDC  (89 payments)
  api.perplexity.ai           31.20 USDC  (28 payments)
  browserbase.com             12.00 USDC   (9 payments)

By Agent:
  researcher                  93.50 USDC
  planner                     32.00 USDC
```

Implementation:

```rust
fn show_payment_summary(roko_dir: &Path) -> anyhow::Result<()> {
    let ledger = PaymentLedger::new(roko_dir);
    let summary = ledger.summary()?;

    println!("Payment Ledger Summary");
    println!("{}", "━".repeat(22));
    println!(
        "Total outgoing:  {:.2} (base units) ({} payments)",
        summary.total_outgoing, summary.total_payments
    );
    println!(
        "Total incoming:  {:.2} (base units)",
        summary.total_incoming
    );
    let pct = if summary.total_payments > 0 {
        (summary.verified_count as f64 / summary.total_payments as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "Verified:        {}/{} ({:.1}% cryptographic)",
        summary.verified_count, summary.total_payments, pct
    );

    if !summary.by_service.is_empty() {
        println!("\nBy Service:");
        let mut services: Vec<_> = summary.by_service.iter().collect();
        services.sort_by(|a, b| b.1.cmp(a.1));
        for (url, amount) in services {
            println!("  {:<30} {} (base units)", url, amount);
        }
    }

    if !summary.by_agent.is_empty() {
        println!("\nBy Agent:");
        let mut agents: Vec<_> = summary.by_agent.iter().collect();
        agents.sort_by(|a, b| b.1.cmp(a.1));
        for (agent, amount) in agents {
            println!("  {:<30} {} (base units)", agent, amount);
        }
    }

    Ok(())
}
```

### 20.4 Add `/api/payments/summary` route to roko-serve

**File**: `crates/roko-serve/src/routes/`

Add a GET route that returns the `SpendingSummary` as JSON:

```rust
async fn payments_summary(
    State(state): State<AppState>,
) -> Result<Json<SpendingSummary>, StatusCode> {
    let roko_dir = state.roko_dir();
    let ledger = PaymentLedger::new(roko_dir);
    let summary = ledger.summary()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(summary))
}
```

Register at `/api/payments/summary` in the router.

### 20.5 Tests: record + read_all roundtrip, summary computation

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(agent: &str, direction: PaymentDirection, amount: u64) -> PaymentEntry {
        PaymentEntry {
            timestamp: "2026-05-04T12:00:00Z".to_string(),
            agent_id: agent.to_string(),
            direction,
            method: "mpp_one_time".to_string(),
            service_url: Some("https://api.openai.com/v1".to_string()),
            amount: amount.to_string(),
            token: "0xUSDC".to_string(),
            settlement: Some(SettlementRecord {
                tx_hash: "0xabc".to_string(),
                block_number: 100,
                chain_id: 42431,
                trust_level: "cryptographic".to_string(),
                consensus_mechanism: "threshold_bls".to_string(),
            }),
            task_id: None,
            episode_hash: None,
        }
    }

    #[test]
    fn record_and_read_all_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let ledger = PaymentLedger::new(tmp.path());

        let entry = make_entry("agent-1", PaymentDirection::Outgoing, 1000);
        ledger.record(&entry).unwrap();
        ledger.record(&entry).unwrap();

        let entries = ledger.read_all().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].agent_id, "agent-1");
        assert_eq!(entries[0].amount, "1000");
    }

    #[test]
    fn read_all_empty_file() {
        let tmp = TempDir::new().unwrap();
        let ledger = PaymentLedger::new(tmp.path());
        let entries = ledger.read_all().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn summary_computation() {
        let tmp = TempDir::new().unwrap();
        let ledger = PaymentLedger::new(tmp.path());

        ledger.record(&make_entry("researcher", PaymentDirection::Outgoing, 500)).unwrap();
        ledger.record(&make_entry("researcher", PaymentDirection::Outgoing, 300)).unwrap();
        ledger.record(&make_entry("planner", PaymentDirection::Outgoing, 200)).unwrap();
        ledger.record(&make_entry("planner", PaymentDirection::Incoming, 100)).unwrap();

        let summary = ledger.summary().unwrap();
        assert_eq!(summary.total_outgoing, 1000);
        assert_eq!(summary.total_incoming, 100);
        assert_eq!(summary.total_payments, 4);
        assert_eq!(summary.verified_count, 3); // 3 outgoing have cryptographic settlement
        assert_eq!(*summary.by_agent.get("researcher").unwrap(), 800);
        assert_eq!(*summary.by_agent.get("planner").unwrap(), 200);
        assert_eq!(
            *summary.by_service.get("https://api.openai.com/v1").unwrap(),
            1000
        );
    }

    #[test]
    fn summary_empty_ledger() {
        let tmp = TempDir::new().unwrap();
        let ledger = PaymentLedger::new(tmp.path());
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.total_outgoing, 0);
        assert_eq!(summary.total_incoming, 0);
        assert_eq!(summary.total_payments, 0);
        assert_eq!(summary.verified_count, 0);
    }
}
```

### 20.6 Register module in lib.rs

**File**: `crates/roko-chain/src/lib.rs`

```rust
/// Payment receipt ledger — append-only JSONL log of MPP payments.
#[cfg(feature = "mpp")]
pub mod payment_ledger;

#[cfg(feature = "mpp")]
pub use payment_ledger::{PaymentEntry, PaymentDirection, PaymentLedger, SettlementRecord, SpendingSummary};
```

---

## Persistence

File: `.roko/payments.jsonl` (append-only JSONL, same pattern as `episodes.jsonl`)

Each line is a JSON-serialized `PaymentEntry`. Example:

```json
{"timestamp":"2026-05-04T12:34:56Z","agent_id":"researcher","direction":"outgoing","method":"mpp_one_time","service_url":"https://api.openai.com/v1","amount":"82300000","token":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","settlement":{"tx_hash":"0xabc...","block_number":142857,"chain_id":42431,"trust_level":"cryptographic","consensus_mechanism":"threshold_bls"},"task_id":"T-42","episode_hash":"sha256:abc123"}
```

---

## Verification Checklist

- [ ] `PaymentEntry`, `PaymentDirection`, `SettlementRecord`, `SpendingSummary` structs defined
- [ ] `PaymentLedger::new()` constructs with `.roko/payments.jsonl` path
- [ ] `PaymentLedger::record()` appends JSONL entries
- [ ] `PaymentLedger::read_all()` reads and deserializes all entries
- [ ] `PaymentLedger::summary()` computes correct totals, verified count, by-service, by-agent breakdowns
- [ ] `read_all()` returns empty vec when file does not exist
- [ ] Module registered in `lib.rs` behind `#[cfg(feature = "mpp")]`
- [ ] Ledger wired into `MppClient` — records after every successful payment
- [ ] `roko learn payments` subcommand prints formatted summary table
- [ ] `/api/payments/summary` route returns `SpendingSummary` as JSON
- [ ] `cargo test -p roko-chain` passes (roundtrip + summary tests)
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` passes
