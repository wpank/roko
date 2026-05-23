//! Shared model dispatch plan data types.
//!
//! This module is intentionally data-only. Resolution and execution remain in
//! the agent/provider layers until the dispatch migration wires them together.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agent::ProviderKind;
use crate::config::schema::{ModelProfile, ProviderConfig};
use crate::foundation::{CachePolicy, ModelCallRequest, TokenBudget};

/// Request envelope for resolving an executable model dispatch plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    /// Surface that originated the request.
    pub caller: DispatchCaller,
    /// Working directory used for config, policy, and routing context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<PathBuf>,
    /// Role or persona requesting the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Existing model-call request payload.
    pub model_call: ModelCallRequest,
    /// Hard model override, if supplied by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    /// Hard provider override, if supplied by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_override: Option<String>,
    /// Required execution capabilities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<DispatchRequirement>,
    /// Cache behavior requested by the caller.
    #[serde(default)]
    pub cache_policy: CachePolicy,
    /// Per-call budget requested by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<TokenBudget>,
    /// Fallback behavior allowed for this request.
    #[serde(default)]
    pub fallback_policy: FallbackPolicy,
}

/// Caller surface that originated a dispatch request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchCaller {
    Acp,
    CliChat,
    CliOneShot,
    Runner,
    Serve,
}

/// Capability or caller contract required from a resolved dispatch plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchRequirement {
    Streaming,
    Tools,
    McpTools,
    Vision,
    Thinking,
    WebSearch,
    Resume,
    EditorMediatedAuth,
    SideEffects,
}

/// Execution-authorizing model/provider/transport plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchPlan {
    /// Original model key or slug requested by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_model: Option<String>,
    /// Original provider id or kind requested by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_provider: Option<String>,
    /// Resolved model key from config.
    pub effective_model_key: String,
    /// Concrete provider model slug sent over the transport.
    pub model_slug: String,
    /// Resolved provider registry id.
    pub provider_id: String,
    /// Resolved provider protocol family.
    pub provider_kind: ProviderKind,
    /// Provider config snapshot used to authorize execution.
    pub provider_config: ProviderConfig,
    /// Model profile snapshot used to authorize execution.
    pub model_profile: ModelProfile,
    /// Concrete transport selected for execution.
    pub transport: TransportPlan,
    /// Auth validation state for the selected transport.
    pub auth_status: DispatchAuthStatus,
    /// Capability requirements validated for this plan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<DispatchRequirement>,
    /// Fallback behavior allowed for execution.
    #[serde(default)]
    pub fallback_policy: FallbackPolicy,
    /// Ordered fallback model keys or slugs that may be attempted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_candidates: Vec<String>,
    /// Planned or completed attempts for the request.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<DispatchAttempt>,
    /// Routing/config notes that should remain visible to callers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

/// Concrete transport selected for a dispatch attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportPlan {
    Cli {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        protocol: String,
    },
    Http {
        base_url: String,
        auth: TransportAuth,
        protocol: String,
    },
    Acp {
        command_or_endpoint: String,
        protocol: String,
    },
    Unsupported {
        reason: String,
    },
    /// Harness adapter transport (Hermes, OpenClaw, etc.).
    Harness {
        /// Harness identifier (e.g. `"hermes"`, `"openclaw"`).
        harness_id: String,
        /// Transport flavor (e.g. `"http_openai"`, `"oneshot_json"`, `"acp_stdio"`).
        transport: String,
        /// Path to the harness binary, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        binary: Option<String>,
        /// Gateway endpoint URL, if applicable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        endpoint_url: Option<String>,
        /// Harness-specific configuration bag.
        #[serde(default)]
        config_bag: ConfigBag,
    },
}

/// Opaque key-value configuration bag for harness-specific settings.
///
/// Wraps `serde_json::Map<String, Value>` and manually implements `Eq`
/// (safe because config bags do not contain NaN values).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigBag(serde_json::Map<String, serde_json::Value>);

impl ConfigBag {
    /// Create an empty config bag.
    #[must_use]
    pub fn new() -> Self {
        Self(serde_json::Map::new())
    }

    /// Create a config bag from a JSON value.
    ///
    /// If the value is an object, its entries are used directly.
    /// If the value is null, the bag is empty.
    /// Otherwise, the value is wrapped under a `"_value"` key.
    #[must_use]
    pub fn from_value(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Object(map) => Self(map),
            serde_json::Value::Null => Self::new(),
            other => {
                let mut map = serde_json::Map::new();
                map.insert("_value".to_string(), other);
                Self(map)
            }
        }
    }

    /// Whether the bag is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get a value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: String, value: serde_json::Value) {
        self.0.insert(key, value);
    }
}

impl PartialEq for ConfigBag {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ConfigBag {}

/// Auth material expected by a transport plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportAuth {
    None,
    EnvVar { name: String },
    EditorMediated,
    Unknown { reason: String },
}

/// Auth validation status for a resolved dispatch plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DispatchAuthStatus {
    Validated,
    Missing { auth_method: String },
    Unvalidated { reason: String },
    NotRequired,
}

/// Fallback behavior allowed for a dispatch request or plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum FallbackPolicy {
    Disabled,
    ConfigOrdered { models: Vec<String> },
    SameProviderOnly,
    AllowCrossProvider { reason: String },
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self::Disabled
    }
}

/// A primary or fallback dispatch attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchAttempt {
    pub kind: DispatchAttemptKind,
    pub model_key: String,
    pub model_slug: String,
    pub provider_id: String,
    pub provider_kind: ProviderKind,
    pub transport: TransportPlan,
}

/// Attempt position in a dispatch plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchAttemptKind {
    Primary,
    Fallback,
}

/// Typed dispatch resolution or execution error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DispatchError {
    MissingAuth {
        provider_id: String,
        auth_method: String,
    },
    UnsupportedProvider {
        provider_id: String,
        provider_kind: ProviderKind,
    },
    CapabilityMismatch {
        requirement: DispatchRequirement,
        provider_id: String,
        model_slug: String,
    },
    AmbiguousProvider {
        requested_provider: String,
        candidates: Vec<String>,
    },
    AmbiguousModel {
        requested_model: String,
        candidates: Vec<String>,
    },
    ProviderFailure {
        provider_id: String,
        message: String,
    },
    Cancelled,
    BudgetExceeded {
        detail: String,
    },
    ConfigInvalid {
        detail: String,
    },
    /// Harness adapter crashed or disconnected mid-turn.
    HarnessCrash {
        harness_id: String,
        transport: String,
        mid_turn: bool,
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_TTFT_TIMEOUT_MS;
    use crate::foundation::{ChatMessage, MessageRole};

    fn provider_config() -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some("https://example.test/v1".to_string()),
            api_key_env: Some("EXAMPLE_API_KEY".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(120_000),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
        }
    }

    fn model_profile() -> ModelProfile {
        ModelProfile {
            provider: "example".to_string(),
            slug: "example-model".to_string(),
            context_window: 128_000,
            tool_format: "openai_json".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn dispatch_plan_serializes_core_resolution_shape() {
        let plan = DispatchPlan {
            requested_model: Some("fast".to_string()),
            requested_provider: Some("example".to_string()),
            effective_model_key: "fast".to_string(),
            model_slug: "example-model".to_string(),
            provider_id: "example".to_string(),
            provider_kind: ProviderKind::OpenAiCompat,
            provider_config: provider_config(),
            model_profile: model_profile(),
            transport: TransportPlan::Http {
                base_url: "https://example.test/v1".to_string(),
                auth: TransportAuth::EnvVar {
                    name: "EXAMPLE_API_KEY".to_string(),
                },
                protocol: "chat_completions".to_string(),
            },
            auth_status: DispatchAuthStatus::Unvalidated {
                reason: "resolver skeleton".to_string(),
            },
            requirements: vec![DispatchRequirement::Streaming],
            fallback_policy: FallbackPolicy::Disabled,
            fallback_candidates: Vec::new(),
            attempts: vec![DispatchAttempt {
                kind: DispatchAttemptKind::Primary,
                model_key: "fast".to_string(),
                model_slug: "example-model".to_string(),
                provider_id: "example".to_string(),
                provider_kind: ProviderKind::OpenAiCompat,
                transport: TransportPlan::Http {
                    base_url: "https://example.test/v1".to_string(),
                    auth: TransportAuth::EnvVar {
                        name: "EXAMPLE_API_KEY".to_string(),
                    },
                    protocol: "chat_completions".to_string(),
                },
            }],
            diagnostics: vec!["capabilities pending resolver validation".to_string()],
        };

        let value = serde_json::to_value(&plan).expect("serialize dispatch plan");

        assert_eq!(value["provider_kind"], "openai_compat");
        assert_eq!(value["transport"]["kind"], "http");
        assert_eq!(value["auth_status"]["status"], "unvalidated");
        assert_eq!(value["fallback_policy"]["policy"], "disabled");
    }

    #[test]
    fn dispatch_request_serializes_existing_model_call_payload() {
        let request = DispatchRequest {
            caller: DispatchCaller::CliOneShot,
            workdir: Some(PathBuf::from("/repo")),
            role: Some("implementer".to_string()),
            model_call: ModelCallRequest {
                model: "example-model".to_string(),
                messages: vec![ChatMessage {
                    role: MessageRole::User,
                    content: "hello".to_string(),
                }],
                cache_policy: CachePolicy::Bypass,
                ..Default::default()
            },
            model_override: Some("example-model".to_string()),
            provider_override: None,
            requirements: vec![DispatchRequirement::Tools],
            cache_policy: CachePolicy::Bypass,
            budget: Some(TokenBudget {
                max_input: Some(1000),
                max_output: Some(500),
                max_cost_usd: Some(0.25),
            }),
            fallback_policy: FallbackPolicy::SameProviderOnly,
        };

        let json = serde_json::to_string(&request).expect("serialize dispatch request");
        let decoded: DispatchRequest =
            serde_json::from_str(&json).expect("deserialize dispatch request");

        assert!(matches!(decoded.caller, DispatchCaller::CliOneShot));
        assert!(matches!(decoded.cache_policy, CachePolicy::Bypass));
        assert_eq!(decoded.model_call.messages.len(), 1);
        assert_eq!(decoded.requirements, vec![DispatchRequirement::Tools]);
    }

    #[test]
    fn config_bag_new_is_empty() {
        let bag = ConfigBag::new();
        assert!(bag.is_empty());
        assert!(bag.get("anything").is_none());
    }

    #[test]
    fn config_bag_from_value_object() {
        let obj = serde_json::json!({"key": "value", "num": 42});
        let bag = ConfigBag::from_value(obj);
        assert!(!bag.is_empty());
        assert_eq!(
            bag.get("key"),
            Some(&serde_json::Value::String("value".to_string()))
        );
        assert_eq!(bag.get("num"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn config_bag_from_value_null() {
        let bag = ConfigBag::from_value(serde_json::Value::Null);
        assert!(bag.is_empty());
    }

    #[test]
    fn config_bag_from_value_non_object() {
        // Non-object, non-null values get wrapped under "_value".
        let bag = ConfigBag::from_value(serde_json::json!("just a string"));
        assert!(!bag.is_empty());
        assert_eq!(
            bag.get("_value"),
            Some(&serde_json::Value::String("just a string".to_string()))
        );

        let bag = ConfigBag::from_value(serde_json::json!(123));
        assert_eq!(bag.get("_value"), Some(&serde_json::json!(123)));
    }

    #[test]
    fn config_bag_insert_and_get() {
        let mut bag = ConfigBag::new();
        assert!(bag.is_empty());

        bag.insert("host".to_string(), serde_json::json!("localhost"));
        bag.insert("port".to_string(), serde_json::json!(8080));

        assert!(!bag.is_empty());
        assert_eq!(
            bag.get("host"),
            Some(&serde_json::Value::String("localhost".to_string()))
        );
        assert_eq!(bag.get("port"), Some(&serde_json::json!(8080)));
        assert!(bag.get("missing").is_none());
    }

    #[test]
    fn config_bag_eq() {
        let mut a = ConfigBag::new();
        let mut b = ConfigBag::new();
        assert_eq!(a, b);

        a.insert("x".to_string(), serde_json::json!(1));
        assert_ne!(a, b);

        b.insert("x".to_string(), serde_json::json!(1));
        assert_eq!(a, b);

        // Different values for the same key.
        let mut c = ConfigBag::new();
        c.insert("x".to_string(), serde_json::json!(2));
        assert_ne!(a, c);
    }

    #[test]
    fn config_bag_serde_roundtrip() {
        let mut bag = ConfigBag::new();
        bag.insert("endpoint".to_string(), serde_json::json!("http://x"));
        bag.insert("retries".to_string(), serde_json::json!(3));

        let json = serde_json::to_string(&bag).expect("serialize");
        let back: ConfigBag = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bag, back);
    }

    #[test]
    fn transport_plan_harness_variant() {
        let plan = TransportPlan::Harness {
            harness_id: "hermes".to_string(),
            transport: "http_openai".to_string(),
            binary: Some("/usr/local/bin/hermes".to_string()),
            endpoint_url: Some("http://127.0.0.1:8642".to_string()),
            config_bag: ConfigBag::new(),
        };

        // Verify it serializes with the correct tag.
        let value = serde_json::to_value(&plan).expect("serialize");
        assert_eq!(value["kind"], "harness");
        assert_eq!(value["harness_id"], "hermes");
        assert_eq!(value["transport"], "http_openai");
        assert_eq!(value["binary"], "/usr/local/bin/hermes");
        assert_eq!(value["endpoint_url"], "http://127.0.0.1:8642");

        // Roundtrip through JSON.
        let json = serde_json::to_string(&plan).expect("serialize");
        let back: TransportPlan = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(plan, back);
    }

    #[test]
    fn dispatch_error_harness_crash() {
        let err = DispatchError::HarnessCrash {
            harness_id: "openclaw".to_string(),
            transport: "acp_stdio".to_string(),
            mid_turn: true,
            error: "SIGKILL".to_string(),
        };

        // Verify serialization roundtrip.
        let value = serde_json::to_value(&err).expect("serialize");
        assert_eq!(value["kind"], "harness_crash");
        assert_eq!(value["harness_id"], "openclaw");
        assert_eq!(value["transport"], "acp_stdio");
        assert_eq!(value["mid_turn"], true);
        assert_eq!(value["error"], "SIGKILL");

        let json = serde_json::to_string(&err).expect("serialize");
        let back: DispatchError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(err, back);

        // Verify Debug output contains key fields.
        let debug = format!("{err:?}");
        assert!(debug.contains("HarnessCrash"));
        assert!(debug.contains("openclaw"));
        assert!(debug.contains("SIGKILL"));
    }
}
