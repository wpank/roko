//! Payload contracts carried by Roko's MCP, A2A, and x402 exoskeleton.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Result, RokoError};

/// Cell input and execution context carried inside an MCP `tools/call` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpCellPayload {
    pub input: Value,
    pub context: Value,
}

/// Cell output, persistence candidates, and metrics returned through MCP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpCellResponse {
    pub output: Value,
    #[serde(default)]
    pub persist: Vec<Value>,
    pub metrics: Value,
}

/// A2A agent card extended with Roko capability-discovery metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCardV2 {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hdc_fingerprint: Option<String>,
    #[serde(default)]
    pub protocols: Vec<String>,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vitality: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

impl AgentCardV2 {
    /// Reject malformed cards before publishing them to A2A discovery.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(RokoError::invalid("agent card name must not be empty"));
        }
        if self.version.trim().is_empty() {
            return Err(RokoError::invalid("agent card version must not be empty"));
        }
        if self
            .vitality
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(RokoError::invalid(
                "agent card vitality must be finite and within 0..=1",
            ));
        }
        Ok(())
    }
}

/// Budget-bounded payment request carried by an x402 interaction.
///
/// Addresses and amounts remain strings so the kernel does not depend on a
/// specific chain SDK or fixed-width integer implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub payer: String,
    pub payee: String,
    pub max_amount: String,
    pub denomination: String,
    pub purpose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_ref: Option<String>,
}

impl PaymentIntent {
    /// Validate the transport-level shape; budget accounting remains the
    /// responsibility of the payment runtime.
    pub fn validate(&self) -> Result<()> {
        if self.payer.trim().is_empty() || self.payee.trim().is_empty() {
            return Err(RokoError::invalid(
                "payment payer and payee must not be empty",
            ));
        }
        if self.denomination.trim().is_empty() || self.purpose.trim().is_empty() {
            return Err(RokoError::invalid(
                "payment denomination and purpose must not be empty",
            ));
        }
        let amount = self.max_amount.as_bytes();
        if amount.is_empty()
            || amount.iter().any(|byte| !byte.is_ascii_digit())
            || amount.iter().all(|byte| *byte == b'0')
        {
            return Err(RokoError::invalid(
                "payment max_amount must be a positive decimal integer",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a2a_card_round_trips_hdc_metadata() {
        let card = AgentCardV2 {
            name: "coder-1".to_owned(),
            description: "Rust coding agent".to_owned(),
            url: Some("https://agent.example".to_owned()),
            capabilities: vec!["code-review".to_owned()],
            hdc_fingerprint: Some("base64:abc".to_owned()),
            protocols: vec!["mcp".to_owned(), "a2a".to_owned()],
            version: "0.1.0".to_owned(),
            vitality: Some(0.85),
            profile: Some("coding".to_owned()),
        };
        card.validate().expect("valid card");
        let json = serde_json::to_string(&card).expect("serialize card");
        assert_eq!(
            serde_json::from_str::<AgentCardV2>(&json).expect("restore card"),
            card
        );
    }

    #[test]
    fn a2a_card_rejects_non_finite_or_out_of_range_vitality() {
        let mut card = AgentCardV2 {
            name: "agent".to_owned(),
            description: String::new(),
            url: None,
            capabilities: Vec::new(),
            hdc_fingerprint: None,
            protocols: Vec::new(),
            version: "1".to_owned(),
            vitality: Some(f64::NAN),
            profile: None,
        };
        assert!(card.validate().is_err());
        card.vitality = Some(1.01);
        assert!(card.validate().is_err());
        card.vitality = Some(0.0);
        assert!(card.validate().is_ok());
    }

    #[test]
    fn payment_intent_rejects_ambiguous_or_zero_amounts() {
        let mut intent = PaymentIntent {
            payer: "0xpayer".to_owned(),
            payee: "0xpayee".to_owned(),
            max_amount: "1000000".to_owned(),
            denomination: "USDC".to_owned(),
            purpose: "feed:blocks".to_owned(),
            expiry: None,
            budget_ref: Some("budget-1".to_owned()),
        };
        assert!(intent.validate().is_ok());
        for invalid in ["", "0", "-1", "1.5", " 1"] {
            intent.max_amount = invalid.to_owned();
            assert!(intent.validate().is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn mcp_payload_preserves_untyped_cell_data() {
        let payload = McpCellPayload {
            input: json!({"signals": [{"kind": "text"}]}),
            context: json!({"budget_remaining": 1.5}),
        };
        let value = serde_json::to_value(&payload).expect("serialize payload");
        assert_eq!(value["context"]["budget_remaining"], 1.5);
    }
}
