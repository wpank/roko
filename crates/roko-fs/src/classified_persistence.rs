//! Classified persistence for transcript records.
//!
//! Provides a [`ClassifiedRecord`] wrapper around transcript records that
//! adds classification levels, redaction metadata, and schema versioning.
//! This is the persistence boundary: only post-ingress, finalized objects
//! are written.
//!
//! # Classification levels
//!
//! | Level | Meaning |
//! |---|---|
//! | `Public` | Safe for logs, dashboards, sharing |
//! | `Internal` | Internal use, no external exposure |
//! | `Sensitive` | Contains PII, secrets, or credentials |
//! | `Restricted` | Highest sensitivity: audit-only access |
//!
//! # Redaction
//!
//! Fields that contain sensitive data are redacted before persistence.
//! The [`RedactedField`] struct preserves the original field name,
//! the redacted size, and a SHA-256 hash of the original value so that
//! forensic replay can verify integrity without exposing the value.

use serde::{Deserialize, Serialize};

use roko_core::tool::transcript::{TranscriptEvent, TranscriptRecord};

/// Current schema version for classified records.
pub const CLASSIFIED_SCHEMA_VERSION: u32 = 1;

// ─── Classification ─────────────────────────────────────────────────────

/// Classification level for a persisted transcript record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Classification {
    /// Safe for logs, dashboards, external sharing.
    Public,
    /// Internal use only.
    Internal,
    /// Contains PII, secrets, or credential-adjacent data.
    Sensitive,
    /// Highest sensitivity — audit access only.
    Restricted,
}

impl Classification {
    /// Stable string key for metrics and filenames.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Sensitive => "sensitive",
            Self::Restricted => "restricted",
        }
    }
}

// ─── Redaction ──────────────────────────────────────────────────────────

/// Metadata about a single redacted field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedField {
    /// The JSON path or field name that was redacted.
    pub field_path: String,
    /// Original byte size of the redacted value.
    pub original_size: usize,
    /// SHA-256 hex hash of the original value (for forensic verification).
    pub original_hash: String,
}

/// Collection of redaction metadata for a classified record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionMeta {
    /// Fields that were redacted.
    pub redacted_fields: Vec<RedactedField>,
    /// Whether any redaction was applied.
    pub was_redacted: bool,
}

// ─── Artifact descriptor ────────────────────────────────────────────────

/// Bounded descriptor for a result artifact (file path, diff summary, etc.).
///
/// This captures metadata about artifacts produced by tool calls without
/// embedding the full artifact payload in the classified record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDescriptor {
    /// Artifact kind (e.g. "file", "diff", "image").
    pub kind: String,
    /// Artifact path or identifier.
    pub path: Option<String>,
    /// Size in bytes, if known.
    pub size_bytes: Option<u64>,
    /// Content hash, if known.
    pub content_hash: Option<String>,
}

/// Optional pointer to the full payload stored elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadPointer {
    /// Storage backend (e.g. "jsonl", "cold_archive", "s3").
    pub backend: String,
    /// Opaque location key within the backend.
    pub location: String,
}

// ─── Result metadata ────────────────────────────────────────────────────

/// Bounded metadata about a tool result, safe for persistence.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultMeta {
    /// Whether the tool call succeeded.
    pub success: bool,
    /// Execution wall-clock time in milliseconds.
    pub execution_ms: Option<u64>,
    /// Bounded list of artifact descriptors.
    pub artifacts: Vec<ArtifactDescriptor>,
    /// Optional pointer to the full result payload.
    pub payload_pointer: Option<PayloadPointer>,
}

// ─── ClassifiedRecord ───────────────────────────────────────────────────

/// A transcript record with classification, redaction, and versioning.
///
/// This is the unit of persistence: only [`ClassifiedRecord`]s are written
/// to the classified JSONL log. Pre-ingress or pre-finalization objects
/// must never be persisted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifiedRecord {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// The classified transcript record.
    pub record: TranscriptRecord,
    /// Classification level.
    pub classification: Classification,
    /// Redaction metadata (empty if nothing was redacted).
    pub redaction: RedactionMeta,
    /// Optional bounded result metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_meta: Option<ResultMeta>,
}

impl ClassifiedRecord {
    /// Classify a transcript record with automatic level detection.
    ///
    /// Applies secret-pattern detection to determine the classification
    /// level and redacts sensitive content.
    #[must_use]
    pub fn classify(record: TranscriptRecord) -> Self {
        let (classification, redaction, result_record) = classify_and_redact(record);
        Self {
            schema_version: CLASSIFIED_SCHEMA_VERSION,
            record: result_record,
            classification,
            redaction,
            result_meta: None,
        }
    }

    /// Create a classified record with an explicit classification level.
    #[must_use]
    pub fn with_classification(record: TranscriptRecord, classification: Classification) -> Self {
        let (_, redaction, result_record) = classify_and_redact(record);
        Self {
            schema_version: CLASSIFIED_SCHEMA_VERSION,
            record: result_record,
            classification,
            redaction,
            result_meta: None,
        }
    }

    /// Attach result metadata.
    #[must_use]
    pub fn with_result_meta(mut self, meta: ResultMeta) -> Self {
        self.result_meta = Some(meta);
        self
    }

    /// Whether this record was redacted.
    #[must_use]
    pub fn was_redacted(&self) -> bool {
        self.redaction.was_redacted
    }
}

// ─── Secret patterns ────────────────────────────────────────────────────

/// Known secret prefixes that trigger Sensitive classification.
const SECRET_PATTERNS: &[&str] = &[
    "sk-ant-",     // Anthropic API keys
    "sk-",         // OpenAI API keys
    "ghp_",        // GitHub personal access tokens
    "gho_",        // GitHub OAuth tokens
    "ghu_",        // GitHub user-to-server tokens
    "ghs_",        // GitHub server-to-server tokens
    "github_pat_", // GitHub fine-grained PATs
    "xoxb-",       // Slack bot tokens
    "xoxp-",       // Slack user tokens
    "AIza",        // Google API keys
    "AKIA",        // AWS access key IDs
    "eyJ",         // JWT tokens (base64 JSON)
];

/// Redaction placeholder.
const REDACTED: &str = "[REDACTED]";

fn classify_and_redact(
    record: TranscriptRecord,
) -> (Classification, RedactionMeta, TranscriptRecord) {
    let mut redaction = RedactionMeta::default();
    let mut classification = classify_event(&record.event);

    let redacted_event = redact_event(record.event, &mut redaction);

    if redaction.was_redacted && classification < Classification::Sensitive {
        classification = Classification::Sensitive;
    }

    let result_record = TranscriptRecord {
        meta: record.meta,
        event: redacted_event,
    };

    (classification, redaction, result_record)
}

fn classify_event(event: &TranscriptEvent) -> Classification {
    match event {
        // Error events might contain stack traces with sensitive paths.
        TranscriptEvent::Error { .. } => Classification::Internal,
        // Usage data is internal.
        TranscriptEvent::Usage { .. } => Classification::Internal,
        // Provider changes reveal infrastructure details.
        TranscriptEvent::ProviderChanged { .. } => Classification::Internal,
        // Most events are public by default.
        _ => Classification::Public,
    }
}

fn redact_event(event: TranscriptEvent, meta: &mut RedactionMeta) -> TranscriptEvent {
    match event {
        TranscriptEvent::AssistantDelta { text } => {
            let (redacted, field) = redact_string("event.text", &text);
            if let Some(f) = field {
                meta.redacted_fields.push(f);
                meta.was_redacted = true;
            }
            TranscriptEvent::AssistantDelta { text: redacted }
        }
        TranscriptEvent::ToolOutputDelta { call_id, text } => {
            let (redacted, field) = redact_string("event.text", &text);
            if let Some(f) = field {
                meta.redacted_fields.push(f);
                meta.was_redacted = true;
            }
            TranscriptEvent::ToolOutputDelta {
                call_id,
                text: redacted,
            }
        }
        TranscriptEvent::Error {
            code,
            message,
            recoverable,
        } => {
            let (redacted, field) = redact_string("event.message", &message);
            if let Some(f) = field {
                meta.redacted_fields.push(f);
                meta.was_redacted = true;
            }
            TranscriptEvent::Error {
                code,
                message: redacted,
                recoverable,
            }
        }
        // All other events pass through unchanged.
        other => other,
    }
}

fn redact_string(field_path: &str, value: &str) -> (String, Option<RedactedField>) {
    for pattern in SECRET_PATTERNS {
        if value.contains(pattern) {
            let hash = sha256_hex(value);
            let field = RedactedField {
                field_path: field_path.to_string(),
                original_size: value.len(),
                original_hash: hash,
            };
            return (REDACTED.to_string(), Some(field));
        }
    }
    (value.to_string(), None)
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

// ─── hex encoding (minimal, no extra dep) ──────────────────────────────

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().fold(String::new(), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::transcript::{TranscriptEvent, TranscriptEventMeta, TranscriptRecord};

    fn make_meta(seq: u64) -> TranscriptEventMeta {
        TranscriptEventMeta {
            run_id: "run-1".into(),
            turn_id: 0,
            agent_id: "agent-1".into(),
            sequence: seq,
            timestamp_ms: 1_700_000_000_000,
            provider: "test".into(),
            model: "test-model".into(),
            parent_event_id: None,
        }
    }

    fn make_record(seq: u64, event: TranscriptEvent) -> TranscriptRecord {
        TranscriptRecord {
            meta: make_meta(seq),
            event,
        }
    }

    #[test]
    fn classification_ordering() {
        assert!(Classification::Public < Classification::Internal);
        assert!(Classification::Internal < Classification::Sensitive);
        assert!(Classification::Sensitive < Classification::Restricted);
    }

    #[test]
    fn classify_public_event() {
        let record = make_record(
            1,
            TranscriptEvent::AssistantDelta {
                text: "hello world".into(),
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert_eq!(classified.classification, Classification::Public);
        assert!(!classified.was_redacted());
        assert_eq!(classified.schema_version, CLASSIFIED_SCHEMA_VERSION);
    }

    #[test]
    fn classify_error_event_as_internal() {
        let record = make_record(
            1,
            TranscriptEvent::Error {
                code: "TEST".into(),
                message: "something broke".into(),
                recoverable: false,
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert_eq!(classified.classification, Classification::Internal);
    }

    #[test]
    fn redact_anthropic_api_key() {
        let record = make_record(
            1,
            TranscriptEvent::AssistantDelta {
                text: "my key is sk-ant-api-abc123-def456".into(),
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert!(classified.was_redacted());
        assert_eq!(classified.classification, Classification::Sensitive);

        // The text should be redacted.
        if let TranscriptEvent::AssistantDelta { text } = &classified.record.event {
            assert_eq!(text, REDACTED);
        } else {
            panic!("expected AssistantDelta");
        }

        // Redaction metadata should exist.
        assert_eq!(classified.redaction.redacted_fields.len(), 1);
        let field = &classified.redaction.redacted_fields[0];
        assert_eq!(field.field_path, "event.text");
        assert!(field.original_size > 0);
        assert!(!field.original_hash.is_empty());
    }

    #[test]
    fn redact_openai_api_key() {
        let record = make_record(
            1,
            TranscriptEvent::ToolOutputDelta {
                call_id: "c1".into(),
                text: "export OPENAI_API_KEY=sk-proj-abc123".into(),
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert!(classified.was_redacted());
    }

    #[test]
    fn redact_github_token() {
        let record = make_record(
            1,
            TranscriptEvent::AssistantDelta {
                text: "token: ghp_1234567890abcdef".into(),
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert!(classified.was_redacted());
    }

    #[test]
    fn no_redaction_for_clean_text() {
        let record = make_record(
            1,
            TranscriptEvent::AssistantDelta {
                text: "This is perfectly safe text with no secrets".into(),
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert!(!classified.was_redacted());
        assert_eq!(classified.redaction.redacted_fields.len(), 0);
    }

    #[test]
    fn explicit_classification_override() {
        let record = make_record(
            1,
            TranscriptEvent::AssistantDelta {
                text: "normal text".into(),
            },
        );
        let classified = ClassifiedRecord::with_classification(record, Classification::Restricted);
        assert_eq!(classified.classification, Classification::Restricted);
    }

    #[test]
    fn with_result_meta() {
        let record = make_record(
            1,
            TranscriptEvent::RunFinished {
                success: true,
                total_turns: 5,
                total_tool_calls: 10,
                wall_ms: 30_000,
            },
        );
        let classified = ClassifiedRecord::classify(record).with_result_meta(ResultMeta {
            success: true,
            execution_ms: Some(30_000),
            artifacts: vec![ArtifactDescriptor {
                kind: "file".into(),
                path: Some("src/main.rs".into()),
                size_bytes: Some(1234),
                content_hash: None,
            }],
            payload_pointer: None,
        });
        assert!(classified.result_meta.is_some());
        assert_eq!(classified.result_meta.as_ref().unwrap().artifacts.len(), 1);
    }

    #[test]
    fn serde_roundtrip() {
        let record = make_record(
            1,
            TranscriptEvent::AssistantDelta {
                text: "hello".into(),
            },
        );
        let classified = ClassifiedRecord::classify(record);
        let json = serde_json::to_string(&classified).unwrap();
        let decoded: ClassifiedRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.schema_version, classified.schema_version);
        assert_eq!(decoded.classification, classified.classification);
    }

    #[test]
    fn schema_version_is_set() {
        let record = make_record(
            1,
            TranscriptEvent::RunStarted {
                system_prompt_hash: None,
                tools_offered: 0,
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert_eq!(classified.schema_version, 1);
    }

    #[test]
    fn classification_as_str() {
        assert_eq!(Classification::Public.as_str(), "public");
        assert_eq!(Classification::Internal.as_str(), "internal");
        assert_eq!(Classification::Sensitive.as_str(), "sensitive");
        assert_eq!(Classification::Restricted.as_str(), "restricted");
    }

    #[test]
    fn redaction_hash_is_deterministic() {
        let input = "sk-ant-api-key-12345";
        let h1 = sha256_hex(input);
        let h2 = sha256_hex(input);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars.
    }

    #[test]
    fn error_message_with_secret_is_redacted() {
        let record = make_record(
            1,
            TranscriptEvent::Error {
                code: "AUTH_FAIL".into(),
                message: "failed with key AKIA1234567890ABCDEF".into(),
                recoverable: false,
            },
        );
        let classified = ClassifiedRecord::classify(record);
        assert!(classified.was_redacted());
        assert_eq!(classified.classification, Classification::Sensitive);
    }
}
