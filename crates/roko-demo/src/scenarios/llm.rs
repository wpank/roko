//! LLM "leaf" provider abstraction.
//!
//! The scripted spine (Rust) owns chain interaction; the LLM only returns
//! structured JSON for parameters (bid amounts, insight content, vote
//! decisions). For CI + demo reproducibility, [`StubLlm`] returns bounded,
//! deterministic-ish outputs without needing an actual model.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Request to the LLM. `slot` names the field the scenario wants filled.
#[derive(Clone, Debug)]
pub struct LlmRequest {
    /// The slot being asked about (e.g. "bid_amount", "approve", "insight_content").
    pub slot: String,
    /// Free-form context the scenario wants the LLM to reason over.
    pub context: serde_json::Value,
}

/// A structured response from the LLM.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Produce a JSON value for the requested slot.
    async fn fill(&self, req: LlmRequest) -> anyhow::Result<serde_json::Value>;
}

/// Deterministic stub: returns canned values per slot based on a seed counter.
pub struct StubLlm {
    counter: std::sync::atomic::AtomicU64,
}

impl StubLlm {
    /// Fresh stub.
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn next(&self) -> u64 {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for StubLlm {
    fn default() -> Self {
        Self::new()
    }
}

/// Shape: `{ bid: bool, submission_content: Option<String> }`
#[derive(Serialize, Deserialize)]
pub struct BidDecision {
    /// Worker decision.
    pub bid: bool,
    /// Free-text submission content.
    pub submission_content: Option<String>,
}

/// Shape: `{ approve: bool, reason: String }`
#[derive(Serialize, Deserialize)]
pub struct VoteDecision {
    /// Approve flag.
    pub approve: bool,
    /// Human-readable reason.
    pub reason: String,
}

#[async_trait]
impl LlmProvider for StubLlm {
    async fn fill(&self, req: LlmRequest) -> anyhow::Result<serde_json::Value> {
        let n = self.next();
        let out = match req.slot.as_str() {
            "bounty_amount" => {
                // Small deterministic bounty: 10 → 100 DAEJI.
                let v = 10 + (n % 10) * 10;
                serde_json::json!(v)
            }
            "job_spec" => serde_json::json!(format!("compute feature#{n}")),
            "bid_amount" | "bid" => serde_json::to_value(BidDecision {
                bid: true,
                submission_content: Some(format!("result#{n}")),
            })?,
            "submission_content" => serde_json::json!(format!("submission#{n}")),
            "approve" | "approve_decision" => serde_json::to_value(VoteDecision {
                // Alternate approvals every few votes (mostly accept).
                approve: n % 5 != 4,
                reason: "looks plausible".into(),
            })?,
            "confirm_decision" => serde_json::json!(n % 3 != 2),
            "insight_content" => {
                serde_json::json!(format!("heuristic#{n}: prefer lower slippage pools"))
            }
            "route_proposal" => serde_json::json!({
                "hops": ["0xaaa", "0xbbb"],
                "expected_output": 1000 + n * 10,
            }),
            _ => serde_json::Value::Null,
        };
        Ok(out)
    }
}
