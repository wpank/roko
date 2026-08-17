# TraceCommons x IronClaw Integration

## Document purpose

This document describes how to integrate IronClaw (NEAR AI's agent framework)
with TraceCommons (TC). It is self-contained: the reader needs no prior context
on either system. Each section includes Rust code sketches, PostgreSQL schema
changes, API additions, configuration examples, and priority/complexity
estimates.

---

## 0. System overviews

### 0.1 TraceCommons (TC)

TraceCommons is a server-side control plane for collecting, scoring, and
curating AI agent traces. Its purpose is to build a shared corpus of
high-quality agent interaction data -- tool calls, LLM turns, human feedback --
that downstream consumers can use for evaluation, benchmarking, and training.

**Six crates:**

| Crate | Role |
|---|---|
| `trace-commons-protocol` | Wire types: `TraceContributionEnvelope`, event schema, redaction pipeline, consent/privacy metadata, value scoring, credit events |
| `trace-commons-contributor` | Client-side CLI: discovers local sessions (Claude Code, Codex, trajectory files), assembles envelopes, redacts secrets, uploads to ingest |
| `trace-commons-gate-api` | Trait contracts for the quality gate: `PerplexityScorer`, `Embedder`, `VectorIndex`, plus decision/outcome types |
| `trace-commons-gate-enclave` | Implementations of the gate traits: chunker, mean-pooled embedder, uSearch vector index, perplexity scorers, `EnclaveGateOrchestrator` pipeline |
| `trace-commons-operator-client` | HTTP client with host allowlisting and format helpers for operator tooling |
| `trace-commons-server` | Hosted ingest/admin/worker binary: PostgreSQL-backed corpus, encrypted artifact store, NEAR credit settlement, score attestation (EdDSA JWTs), audit hash chain, dedup, and ~60k LOC of route handlers |

**Key abstractions:**

- `TraceContributionEnvelope` -- the canonical unit of data. Contains: metadata
  (`IronclawTraceMetadata`, consent, contributor pseudonym, privacy report),
  events (user messages, assistant messages, tool calls, tool results, routing
  decisions, feedback), outcome (success/failure classification), replay
  metadata, value scorecard, and optional process evaluation labels.

- `TraceSource` trait -- the client-side adapter interface:
  ```rust
  pub trait TraceSource {
      fn name(&self) -> &'static str;
      fn discover(&self) -> anyhow::Result<Vec<SessionRef>>;
      fn load(&self, r: &SessionRef) -> anyhow::Result<SessionTranscript>;
  }
  ```
  Today there are three implementations: `ClaudeCodeSource`, `CodexSource`,
  and `TrajectorySource`. IronClaw would be the fourth.

- Quality gate -- the `EnclaveGateOrchestrator` pipelines a trace through
  chunking, perplexity scoring, embedding, and novelty assessment. The result
  is an `OrchestrationDecision` with pass/fail flags, numeric scores, and
  per-chunk vector index entries. This runs inside a TEE (enclave) on the
  server side.

- Credit system -- accepted traces earn non-transferable credit points based on
  a `CreditQualityScore` (multiplicative, log-concave, anti-Goodhart). Credits
  settle on-chain via `NearCreditReceiptCall` to a NEAR smart contract.

- Score attestation -- the server signs EdDSA JWTs attesting to a contributor's
  scored submissions, allowing external collectors to verify scores without
  trusting an API response.

### 0.2 IronClaw

IronClaw is NEAR AI's open-source, security-first AI agent runtime. It runs
agents that can use tools, call LLMs, operate across multiple communication
channels, and deploy inside TEEs on NEAR AI Cloud.

**Ten crate families (62 workspace crates):**

| Family | Crate count | Role |
|---|---|---|
| `contracts/` | 6 | Shared vocabulary and port definitions: `host_api`, `common`, `prompt_envelope`, `loop_contracts`, `extension_contracts`, `product_contracts` |
| `substrates/` | 6 | Privileged host services: `filesystem`, `libsql_runtime`, `secrets`, `network`, `safety`, `observability` |
| `events/` | 4 | Append-only event log, event store, projections, streams |
| `domains/` | 12 | Typed record owners: `threads`, `conversations`, `triggers`, `memory`, `skills`, `auth`, `attachments`, `extractors`, `identity`, `llm`, `trace_commons`, `outbound` |
| `kernel/` | 9 | Authority perimeter: `trust`, `authorization`, `approvals`, `resources`, `runtime_policy`, `capabilities`, `processes`, `turns`, `host_runtime` |
| `lanes/` | 4 | Isolated execution environments: `wasm`, `wasm_limiter`, `mcp`, `sandbox` |
| `loop/` | 4 | Agent behavior: `agent_loop`, `loop_host`, `turn_runner`, `hooks` |
| `extensions/` | 8 + 14 packages | Integration packages: Slack, Telegram, GitHub, Gmail, Google services, etc. |
| `product/` | 5 | User-facing: `assistant`, `operator`, `openai_compat`, `webui`, `host_ingress` |
| `app/` | 4 | Assembly: `composition`, `cli` (the `ironclaw` binary), `config`, `architecture_tests` |

**WASM sandboxing:**

- Runtime: Wasmtime.
- Isolation model: fresh Wasmtime instance per tool invocation -- no state
  persists across calls.
- Fuel metering: each invocation receives a bounded fuel allowance. Execution
  terminates when fuel exhausts (no wall-clock timeout as primary mechanism).
- Memory: 16 MB default per instance, with WebAssembly linear memory isolated
  from the host. Data exchange is serialized messages only.
- Capability-based permissions: `ironclaw_host_api` defines what filesystem
  paths, network endpoints, and host functions each tool can access.

**Credential firewall (staged injection):**

1. Storage: API keys encrypted at rest via AES-256-GCM (`ironclaw_secrets`).
2. Resolution: at tool invocation time, `HostRuntime` looks up the credential.
3. Injection: secret injected into the sandbox just-in-time at the host
   boundary. The LLM and the tool code never see the raw secret simultaneously.
4. Zeroization: after tool execution, all credential copies in memory are wiped
   using constant-time operations to prevent timing attacks.
5. Leak detection: all outbound traffic is scanned in real-time. Anything that
   matches a secret pattern heading outbound is blocked.

**26 LLM providers:**

Native: NEAR AI, Anthropic (Claude), OpenAI, Google Gemini, GitHub Copilot,
Ollama, MiniMax, AWS Bedrock, io.net, Mistral, Yandex AI Studio, Cloudflare
Workers AI. OpenAI-compatible: OpenRouter, Together AI, Fireworks AI, vLLM,
LiteLLM, LM Studio, plus custom endpoints. Routing is handled by
`RoutedLlmProviderModelGateway`.

**Multi-channel:**

CLI/REPL, WebChat (React SPA + SSE), and WASM channel adapters for Telegram,
Slack, Discord, Signal, and WhatsApp. Each channel adapter runs as a WASM
component receiving inbound messages and forwarding responses through the
unified `RebornRuntime`.

**TEE deployment on NEAR AI Cloud:**

Agents execute within Intel SGX or AMD SEV enclaves. Memory is encrypted at
rest and in use. Remote attestation verifies enclave code integrity. User data
(prompts, credentials, conversation history) never leaves the TEE boundary --
only computation results are exported.

---

## 1. Trace capture from IronClaw agents

### 1.1 The gap

IronClaw has no built-in trace export pipeline. Its `events/` family
(`event_log`, `event_store`, `event_projections`, `event_streams`) provides
internal event persistence for crash recovery and state reconstruction, but
these events are not structured for external consumption, are not redacted, and
carry no consent or privacy metadata.

TraceCommons fills this gap: it defines a privacy-preserving envelope format,
runs a multi-layer redaction pipeline, scores traces for quality, and stores
them in a curated corpus.

### 1.2 IronClawTraceAdapter

The adapter implements TC's `TraceSource` trait:

```rust
// crate: trace-commons-contributor/src/source/ironclaw.rs

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use crate::source::{
    SessionRef, SessionTranscript, SessionEvent, SessionEventKind,
    TraceSource, session_hash,
};

pub const SOURCE_IRONCLAW: &str = "ironclaw";

/// Discovers and loads IronClaw agent sessions from the local event store.
pub struct IronClawSource {
    /// Root of the IronClaw data directory, typically `~/.ironclaw/`.
    root: PathBuf,
}

impl IronClawSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Walk the event store directory, find completed conversation threads.
    fn discover_threads(&self) -> anyhow::Result<Vec<PathBuf>> {
        let threads_dir = self.root.join("data/threads");
        if !threads_dir.exists() {
            return Ok(Vec::new());
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&threads_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                paths.push(path);
            }
        }
        paths.sort();
        Ok(paths)
    }
}

impl TraceSource for IronClawSource {
    fn name(&self) -> &'static str {
        SOURCE_IRONCLAW
    }

    fn discover(&self) -> anyhow::Result<Vec<SessionRef>> {
        let threads = self.discover_threads()?;
        let mut refs = Vec::with_capacity(threads.len());
        for path in threads {
            let meta = std::fs::metadata(&path)?;
            refs.push(SessionRef {
                source: SOURCE_IRONCLAW,
                path,
                project: None,
                cwd: None,
                started_at: None,
                size_bytes: meta.len(),
            });
        }
        Ok(refs)
    }

    fn load(&self, r: &SessionRef) -> anyhow::Result<SessionTranscript> {
        let bytes = std::fs::read(&r.path)?;
        let hash = session_hash(&bytes);
        let raw: Vec<IronClawEventRecord> =
            serde_json::Deserializer::from_slice(&bytes)
                .into_iter()
                .filter_map(|r| r.ok())
                .collect();

        let mut events = Vec::with_capacity(raw.len());
        let mut model = None;
        let mut agent_version = None;
        let mut started_at: Option<DateTime<Utc>> = None;

        for record in &raw {
            if started_at.is_none() {
                started_at = record.timestamp;
            }
            match record.event_type.as_str() {
                "user_message" => events.push(SessionEvent {
                    kind: SessionEventKind::User,
                    timestamp: record.timestamp,
                    content: record.content.clone(),
                    structured: record.metadata.clone(),
                    tool_name: None,
                    token_counts: None,
                }),
                "assistant_message" => {
                    if let Some(m) = record.model_name.as_ref() {
                        model = Some(m.clone());
                    }
                    events.push(SessionEvent {
                        kind: SessionEventKind::Assistant,
                        timestamp: record.timestamp,
                        content: record.content.clone(),
                        structured: record.metadata.clone(),
                        tool_name: None,
                        token_counts: record.token_counts(),
                    });
                }
                "tool_call" => events.push(SessionEvent {
                    kind: SessionEventKind::ToolCall,
                    timestamp: record.timestamp,
                    content: record.content.clone(),
                    structured: record.metadata.clone(),
                    tool_name: record.tool_name.clone(),
                    token_counts: None,
                }),
                "tool_result" => events.push(SessionEvent {
                    kind: SessionEventKind::ToolResult,
                    timestamp: record.timestamp,
                    content: record.content.clone(),
                    structured: record.metadata.clone(),
                    tool_name: record.tool_name.clone(),
                    token_counts: None,
                }),
                "system_init" => {
                    agent_version = record.agent_version.clone();
                }
                _ => events.push(SessionEvent {
                    kind: SessionEventKind::Opaque,
                    timestamp: record.timestamp,
                    content: None,
                    structured: serde_json::json!({
                        "record_type": record.event_type,
                    }),
                    tool_name: None,
                    token_counts: None,
                }),
            }
        }

        Ok(SessionTranscript {
            source: std::borrow::Cow::Borrowed(SOURCE_IRONCLAW),
            agent_version,
            model,
            project: None,
            cwd: None,
            started_at,
            session_hash: hash,
            events,
        })
    }
}

/// Internal deserialization target for IronClaw event records.
#[derive(serde::Deserialize)]
struct IronClawEventRecord {
    event_type: String,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    agent_version: Option<String>,
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
    #[serde(default)]
    metadata: serde_json::Value,
}

impl IronClawEventRecord {
    fn token_counts(&self) -> Option<(u32, u32)> {
        match (self.input_tokens, self.output_tokens) {
            (Some(i), Some(o)) => Some((i, o)),
            _ => None,
        }
    }
}
```

### 1.3 Hook points

IronClaw events that map to TC event types:

| IronClaw event | TC event type | Notes |
|---|---|---|
| `user_message` | `UserMessage` | Direct mapping |
| `assistant_message` | `AssistantMessage` | Extract `model_name`, token counts |
| `tool_call` | `ToolCall` | WASM tool name, arguments in `structured_payload` |
| `tool_result` | `ToolResult` | Tool output, success/failure, latency |
| `llm_request` / `llm_response` | (folded into `AssistantMessage`) | Provider name, model, token counts |
| `channel_message_received` | `UserMessage` | With channel metadata in `structured_payload` |
| `channel_message_sent` | `AssistantMessage` | Outbound channel response |
| `wasm_sandbox_event` | `ToolCall` / `ToolResult` | Fuel consumed, memory used |
| `credential_access` | (metadata on `ToolCall`) | Which credential was accessed |
| `routing_decision` | `RoutingDecision` | Model selection from 26 providers |

### 1.4 Registration in the source registry

```rust
// In trace-commons-contributor/src/source/mod.rs

pub mod ironclaw;

pub const SOURCE_IRONCLAW: &str = "ironclaw";

pub fn all_sources(
    claude_root: Option<PathBuf>,
    codex_root: Option<PathBuf>,
    trajectory_path: Option<PathBuf>,
    ironclaw_root: Option<PathBuf>,
) -> Vec<Box<dyn TraceSource>> {
    let mut sources: Vec<Box<dyn TraceSource>> = vec![
        Box::new(claude_code::ClaudeCodeSource::new(/* ... */)),
        Box::new(codex::CodexSource::new(/* ... */)),
    ];
    if let Some(path) = trajectory_path {
        sources.push(Box::new(trajectory::TrajectorySource::new(path)));
    }
    // IronClaw: default to ~/.ironclaw/ when not overridden
    let ironclaw_root = ironclaw_root.unwrap_or_else(|| {
        dirs::home_dir().unwrap_or_default().join(".ironclaw")
    });
    if ironclaw_root.exists() {
        sources.push(Box::new(ironclaw::IronClawSource::new(ironclaw_root)));
    }
    sources
}
```

**Priority:** P0 (prerequisite for everything else)
**Complexity:** Medium -- the adapter is straightforward; the complexity is in
mapping IronClaw's internal event format (which may evolve) to TC's stable
schema.

---

## 2. Schema extensions for IronClaw traces

### 2.1 Protocol extension

IronClaw produces metadata that no other trace source generates. Rather than
polluting the generic `TraceContributionEvent` with IronClaw-specific fields,
define an extension type that rides in the event's `structured_payload`:

```rust
// In trace-commons-protocol/src/ironclaw_extension.rs

use serde::{Deserialize, Serialize};

/// IronClaw-specific metadata attached to trace events via
/// `structured_payload`. TC's generic pipeline passes this through
/// unmodified; IronClaw-aware consumers (analytics, scoring) can
/// deserialize it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IronClawTraceExtension {
    /// WASM fuel consumed by this tool invocation. None for non-tool events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_fuel_consumed: Option<u64>,

    /// WASM fuel limit that was configured for this invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_fuel_limit: Option<u64>,

    /// Peak linear memory usage in bytes during tool execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_memory_peak_bytes: Option<u64>,

    /// Whether this tool ran in a fresh WASM instance (should always be true
    /// under IronClaw's security model).
    #[serde(default)]
    pub wasm_fresh_instance: bool,

    /// Credential access pattern: which credential names were accessed
    /// during this tool invocation (redacted to category, never raw values).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credential_categories_accessed: Vec<String>,

    /// Whether credentials were properly zeroized after use.
    #[serde(default)]
    pub credential_zeroized: bool,

    /// Communication channel through which this event originated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<IronClawChannel>,

    /// Conversation thread ID within the channel (redacted/hashed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_thread_hash: Option<String>,

    /// Sandbox isolation level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_isolation: Option<SandboxIsolation>,

    /// LLM provider used for this turn (from IronClaw's 26-provider catalog).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,

    /// Specific model within the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_model: Option<String>,

    /// Whether this event occurred inside a TEE.
    #[serde(default)]
    pub tee_attested: bool,

    /// TEE attestation report hash, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tee_attestation_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IronClawChannel {
    Cli,
    Web,
    Telegram,
    Slack,
    Discord,
    Signal,
    Whatsapp,
    Webhook,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxIsolation {
    WasmFull,
    WasmRestricted,
    Docker,
    Native,
    Unknown,
}
```

### 2.2 Extending `IronclawTraceMetadata`

The existing `IronclawTraceMetadata` struct in `trace-commons-protocol` already
has a `channel` field (`TraceChannel` enum) and a `feature_flags` map. Extend
`TraceChannel` with IronClaw-specific variants:

```rust
// Extend the existing enum:
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceChannel {
    Web,
    Cli,
    Telegram,
    Slack,
    Discord,
    Signal,
    Whatsapp,
    Routine,
    Other,
}
```

And populate `feature_flags` with IronClaw-specific entries during envelope
assembly:

```rust
feature_flags.insert("sandbox_type".to_string(), "wasm".to_string());
feature_flags.insert("fuel_metering".to_string(), "enabled".to_string());
feature_flags.insert("credential_firewall".to_string(), "staged_injection".to_string());
feature_flags.insert("tee_deployment".to_string(), tee_status.to_string());
```

### 2.3 PostgreSQL schema

Add an `ironclaw_metadata` JSONB column to the trace corpus table:

```sql
-- Migration: add_ironclaw_metadata
-- Adds IronClaw-specific metadata to traces without breaking the generic schema.

ALTER TABLE trace_contributions
    ADD COLUMN ironclaw_metadata JSONB DEFAULT NULL;

COMMENT ON COLUMN trace_contributions.ironclaw_metadata IS
    'IronClaw-specific metadata: WASM fuel, credential patterns, channel, '
    'sandbox isolation, provider info. NULL for non-IronClaw traces.';

-- Index for querying by provider or channel
CREATE INDEX CONCURRENTLY idx_trace_contributions_ironclaw_provider
    ON trace_contributions ((ironclaw_metadata->>'provider_name'))
    WHERE ironclaw_metadata IS NOT NULL;

CREATE INDEX CONCURRENTLY idx_trace_contributions_ironclaw_channel
    ON trace_contributions ((ironclaw_metadata->>'channel'))
    WHERE ironclaw_metadata IS NOT NULL;

-- Aggregate table for IronClaw-specific analytics
CREATE TABLE ironclaw_trace_analytics (
    submission_id UUID PRIMARY KEY REFERENCES trace_contributions(submission_id),
    tenant_id TEXT NOT NULL,
    provider_name TEXT,
    provider_model TEXT,
    channel TEXT,
    total_wasm_fuel_consumed BIGINT DEFAULT 0,
    total_wasm_invocations INT DEFAULT 0,
    credential_access_count INT DEFAULT 0,
    credential_zeroized_count INT DEFAULT 0,
    tee_attested BOOLEAN DEFAULT FALSE,
    sandbox_isolation TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- RLS policy (required by TC convention)
ALTER TABLE ironclaw_trace_analytics ENABLE ROW LEVEL SECURITY;

CREATE POLICY ironclaw_trace_analytics_tenant_policy
    ON ironclaw_trace_analytics
    FOR ALL
    USING (tenant_id = trace_current_tenant_id());
```

**Priority:** P1 (needed before any IronClaw-specific analytics)
**Complexity:** Low -- JSONB is flexible; the dedicated analytics table
is optional until query volume justifies it.

---

## 3. TEE attestation bridge

### 3.1 The opportunity

Both systems use TEEs independently:
- IronClaw runs agents inside SGX/SEV enclaves on NEAR AI Cloud.
- TraceCommons runs the quality gate (`EnclaveGateOrchestrator`) inside a TEE.

Together they can form an end-to-end attestation chain: the TEE that ran the
agent proves the trace is authentic, and the TEE that scored the trace proves
the scores are tamper-proof.

### 3.2 Attestation chain

```
[IronClaw TEE]            [TC Ingest]            [TC Gate TEE]
     |                         |                       |
     |-- agent execution -->   |                       |
     |   attestation_report_1  |                       |
     |                         |                       |
     |-- trace envelope -----> |                       |
     |   (includes att_1 hash) |                       |
     |                         |-- forward to gate --> |
     |                         |                       |-- score trace
     |                         |                       |   attestation_report_2
     |                         |                       |
     |                         |<-- decision + att_2 --|
     |                         |                       |
     |                         |-- chain_hash = H(att_1 || att_2)
     |                         |-- store in audit row
```

### 3.3 Data structures

```rust
/// A TEE attestation report from an IronClaw agent execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IronClawTeeAttestation {
    /// Attestation format: "sgx_dcap_v4" or "sev_snp_v1".
    pub format: String,
    /// SHA-256 of the enclave measurement (MRENCLAVE for SGX, LAUNCH_DIGEST
    /// for SEV-SNP).
    pub measurement_hash: String,
    /// SHA-256 of the signer identity (MRSIGNER for SGX, signer for SEV).
    pub signer_hash: String,
    /// The raw attestation report, base64-encoded.
    pub report_b64: String,
    /// SHA-256 hash of report_b64 for compact references.
    pub report_hash: String,
    /// Timestamp when the attestation was generated.
    pub attested_at: DateTime<Utc>,
}

/// Chained attestation linking agent execution to trace scoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainedAttestation {
    pub schema_version: String,
    /// IronClaw agent execution attestation.
    pub agent_attestation: IronClawTeeAttestation,
    /// TC gate scoring attestation (from EnclaveGateOrchestrator).
    pub gate_attestation_chain_hash: String,
    /// H(agent_attestation.report_hash || gate_attestation_chain_hash)
    pub chained_hash: String,
    pub created_at: DateTime<Utc>,
}

impl ChainedAttestation {
    pub fn build(
        agent_att: IronClawTeeAttestation,
        gate_chain_hash: String,
        now: DateTime<Utc>,
    ) -> Self {
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"trace_commons.chained_attestation.v1\n");
        hasher.update(agent_att.report_hash.as_bytes());
        hasher.update(b"\n");
        hasher.update(gate_chain_hash.as_bytes());
        let chained_hash = format!("sha256:{}", hex::encode(hasher.finalize()));

        Self {
            schema_version: "trace_commons.chained_attestation.v1".to_string(),
            agent_attestation: agent_att,
            gate_attestation_chain_hash: gate_chain_hash,
            chained_hash,
            created_at: now,
        }
    }

    /// Verify the chained hash is correctly derived from its components.
    pub fn verify_chain_integrity(&self) -> bool {
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"trace_commons.chained_attestation.v1\n");
        hasher.update(self.agent_attestation.report_hash.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.gate_attestation_chain_hash.as_bytes());
        let expected = format!("sha256:{}", hex::encode(hasher.finalize()));
        self.chained_hash == expected
    }
}
```

### 3.4 PostgreSQL schema

```sql
CREATE TABLE tee_attestation_chain (
    submission_id UUID PRIMARY KEY REFERENCES trace_contributions(submission_id),
    tenant_id TEXT NOT NULL,
    agent_measurement_hash TEXT NOT NULL,
    agent_signer_hash TEXT NOT NULL,
    agent_report_hash TEXT NOT NULL,
    agent_attested_at TIMESTAMPTZ NOT NULL,
    gate_attestation_chain_hash TEXT NOT NULL,
    chained_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE tee_attestation_chain ENABLE ROW LEVEL SECURITY;
CREATE POLICY tee_attestation_chain_tenant_policy
    ON tee_attestation_chain FOR ALL
    USING (tenant_id = trace_current_tenant_id());

-- The raw attestation reports are stored in the encrypted artifact store,
-- not in PostgreSQL. Only hashes live in the DB.
```

### 3.5 API endpoint

```
POST /v1/attestation/verify-chain
Authorization: Bearer <upload-claim>
Content-Type: application/json

{
    "submission_id": "uuid",
    "agent_report_hash": "sha256:...",
    "gate_attestation_chain_hash": "sha256:..."
}

Response 200:
{
    "verified": true,
    "chained_hash": "sha256:...",
    "agent_measurement_hash": "sha256:...",
    "gate_policy_version": "enclave_v3"
}
```

**Priority:** P2 (high value but requires IronClaw TEE deployment to be
production-ready first)
**Complexity:** High -- attestation verification requires parsing vendor-
specific report formats (SGX DCAP, SEV-SNP) and maintaining a root-of-trust
for each.

---

## 4. WASM-specific trace types

### 4.1 Fuel metering as a quality signal

IronClaw's WASM fuel metering produces a unique signal: how efficiently a tool
accomplished its task. Efficient tools (low fuel for high-value results) produce
better traces because they demonstrate clean, minimal tool usage patterns that
are more useful for training.

```rust
/// WASM-specific signals extracted from IronClaw tool invocations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WasmToolMetrics {
    pub tool_name: String,
    pub fuel_consumed: u64,
    pub fuel_limit: u64,
    /// fuel_consumed / fuel_limit -- closer to 0 means more efficient.
    pub fuel_efficiency: f32,
    pub memory_peak_bytes: u64,
    pub memory_limit_bytes: u64,
    pub execution_time_ms: u64,
    /// True if execution was terminated due to fuel exhaustion.
    pub fuel_exhausted: bool,
    /// True if the tool attempted to access capabilities it was not granted.
    pub capability_violation_attempted: bool,
}

/// Aggregate WASM metrics for a complete trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WasmTraceAggregate {
    pub total_fuel_consumed: u64,
    pub total_fuel_limit: u64,
    pub tool_invocation_count: u32,
    pub fuel_exhaustion_count: u32,
    pub capability_violation_count: u32,
    pub mean_fuel_efficiency: f32,
    pub peak_memory_bytes: u64,
}
```

### 4.2 Scoring integration

The `CreditQualityScore` formula can use WASM metadata as a bonus/penalty
signal. Fuel efficiency maps naturally to the existing multiplicative,
log-concave framework:

```rust
/// WASM-aware credit quality adjustment. Multiplied into the base q score.
/// Returns a value in [0.8, 1.2] -- bounded to prevent WASM metadata from
/// dominating the score.
pub fn wasm_quality_factor(wasm: &WasmTraceAggregate) -> f64 {
    let mut factor = 1.0_f64;

    // Penalty for fuel exhaustion: indicates the tool hit limits, likely
    // producing incomplete results.
    if wasm.fuel_exhaustion_count > 0 {
        let exhaustion_rate =
            wasm.fuel_exhaustion_count as f64 / wasm.tool_invocation_count.max(1) as f64;
        factor *= 1.0 - (exhaustion_rate * 0.2).min(0.2);
    }

    // Penalty for capability violations: indicates the agent tried to
    // escape the sandbox, a safety signal.
    if wasm.capability_violation_count > 0 {
        factor *= 0.8;
    }

    // Small bonus for fuel efficiency (efficient tools = cleaner traces).
    if wasm.mean_fuel_efficiency < 0.3 && wasm.tool_invocation_count >= 3 {
        factor *= 1.05;
    }

    factor.clamp(0.8, 1.2)
}
```

### 4.3 Safety scoring for sandbox compliance

```rust
/// Score a trace's sandbox compliance. Returns [0, 1].
pub fn sandbox_compliance_score(wasm: &WasmTraceAggregate) -> f32 {
    let mut score = 1.0_f32;

    // Each capability violation is a strong negative signal.
    let violation_penalty = (wasm.capability_violation_count as f32 * 0.25).min(0.75);
    score -= violation_penalty;

    // Fuel exhaustion is a mild negative (agent was too aggressive).
    let exhaustion_penalty = (wasm.fuel_exhaustion_count as f32 * 0.1).min(0.25);
    score -= exhaustion_penalty;

    score.clamp(0.0, 1.0)
}
```

**Priority:** P1 (unique value that no other trace source provides)
**Complexity:** Medium -- requires IronClaw to expose fuel/memory metrics in
its event records, which is a change on their side.

---

## 5. Multi-channel trace unification

### 5.1 The problem

An IronClaw agent operates across CLI, Telegram, Slack, Discord, Signal, and
WebChat. The same agent, with the same model, produces traces from different
channels. TC needs to:

1. Unify traces from the same agent across channels.
2. Correlate sessions that are part of the same logical conversation but span
   channels (e.g., user starts on Slack, continues on CLI).
3. Apply channel-specific redaction rules (Slack workspace IDs, Telegram user
   IDs, Discord server names).

### 5.2 Agent fingerprinting

```rust
/// Unique identifier for an IronClaw agent instance, stable across channels.
/// Derived from the agent's configuration hash, not from runtime state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentFingerprint {
    /// SHA-256 of the agent's configuration (skills, tools, system prompt,
    /// model routing rules). Changes when the agent is reconfigured.
    pub config_hash: String,
    /// Semantic version of the IronClaw runtime.
    pub runtime_version: String,
    /// The agent's self-declared name (from SKILL.md or config).
    pub agent_name: String,
}

impl AgentFingerprint {
    pub fn compute(
        config_bytes: &[u8],
        runtime_version: &str,
        agent_name: &str,
    ) -> Self {
        let config_hash = format!(
            "sha256:{}",
            hex::encode(sha2::Sha256::digest(config_bytes))
        );
        Self {
            config_hash,
            runtime_version: runtime_version.to_string(),
            agent_name: agent_name.to_string(),
        }
    }
}
```

### 5.3 Cross-channel session correlation

```rust
/// Correlate traces from the same logical conversation across channels.
/// The correlation key is a hash of the agent fingerprint + thread context,
/// so it is stable across channels but does not leak user identity.
pub fn cross_channel_correlation_key(
    agent_fingerprint: &AgentFingerprint,
    thread_context: &str,
) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"ironclaw.cross_channel.v1\n");
    hasher.update(agent_fingerprint.config_hash.as_bytes());
    hasher.update(b"\n");
    hasher.update(thread_context.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
```

### 5.4 Channel-specific redaction rules

Each channel leaks different PII through its metadata:

| Channel | Leaked identifiers | Redaction strategy |
|---|---|---|
| Slack | Workspace ID, channel name, user display name | Hash workspace + channel; strip display names |
| Telegram | User ID, chat ID, group name | Hash user ID + chat ID; strip group names |
| Discord | Server ID, channel ID, user tag | Hash server + channel; strip user tags |
| Signal | Phone number (in thread context) | Strip all phone-number patterns |
| WhatsApp | Phone number, group name | Strip phone numbers; hash group name |
| CLI | Local username, hostname | Strip via existing path redaction |
| Web | Session cookie (in headers) | Strip via existing header redaction |

```rust
/// Channel-aware redaction rules applied before the deterministic pipeline.
pub fn channel_specific_redaction(
    channel: IronClawChannel,
    content: &str,
) -> String {
    match channel {
        IronClawChannel::Telegram => {
            // Telegram user IDs are numeric, 6-12 digits
            let re = regex::Regex::new(r"\b\d{6,12}\b").unwrap();
            re.replace_all(content, "[TELEGRAM_ID]").to_string()
        }
        IronClawChannel::Signal | IronClawChannel::Whatsapp => {
            // Phone numbers: +1234567890 or variants
            let re = regex::Regex::new(r"\+?\d[\d\s\-]{7,15}\d").unwrap();
            re.replace_all(content, "[PHONE]").to_string()
        }
        IronClawChannel::Discord => {
            // Discord user tags: Username#1234
            let re = regex::Regex::new(r"\w+#\d{4}").unwrap();
            re.replace_all(content, "[DISCORD_USER]").to_string()
        }
        _ => content.to_string(),
    }
}
```

### 5.5 PostgreSQL schema for channel analytics

```sql
CREATE TABLE ironclaw_channel_sessions (
    session_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id TEXT NOT NULL,
    submission_id UUID NOT NULL REFERENCES trace_contributions(submission_id),
    agent_config_hash TEXT NOT NULL,
    channel TEXT NOT NULL,
    cross_channel_key TEXT,
    event_count INT NOT NULL,
    started_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE ironclaw_channel_sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY ironclaw_channel_sessions_tenant_policy
    ON ironclaw_channel_sessions FOR ALL
    USING (tenant_id = trace_current_tenant_id());

CREATE INDEX idx_ironclaw_channel_sessions_cross_channel
    ON ironclaw_channel_sessions (cross_channel_key)
    WHERE cross_channel_key IS NOT NULL;
```

**Priority:** P1 (core differentiator for IronClaw traces)
**Complexity:** Medium -- redaction rules require ongoing maintenance as
channel APIs evolve.

---

## 6. NEAR ecosystem integration

### 6.1 Shared identity

Both TC and IronClaw operate in the NEAR ecosystem. NEAR account IDs can serve
as the shared identity layer:

```rust
/// Map a NEAR account ID to a TC contributor identity. The NEAR account
/// is the canonical identity; the TC pseudonym is derived from it.
pub fn near_account_to_contributor_pseudonym(
    near_account_id: &str,
) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"trace_commons.near_identity.v1\n");
    hasher.update(near_account_id.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Verify that a NEAR account ID is well-formed (2-64 chars, lowercase
/// alphanumeric with `.`, `-`, `_` separators, no consecutive separators,
/// no leading/trailing separators).
pub fn validate_near_account_id(account_id: &str) -> bool {
    if account_id.len() < 2 || account_id.len() > 64 {
        return false;
    }
    let mut prev_sep = false;
    for (i, b) in account_id.bytes().enumerate() {
        let is_alnum = b.is_ascii_lowercase() || b.is_ascii_digit();
        let is_sep = matches!(b, b'.' | b'-' | b'_');
        if !is_alnum && !is_sep {
            return false;
        }
        if i == 0 && is_sep {
            return false;
        }
        if is_sep && prev_sep {
            return false;
        }
        prev_sep = is_sep;
    }
    !prev_sep
}
```

### 6.2 On-chain trace provenance

Submit trace hashes to a NEAR smart contract for tamper-proof provenance:

```rust
/// Build a NEAR function call to record a trace hash on-chain.
/// Uses the existing NearCreditReceiptCall machinery for validation.
pub fn build_trace_provenance_call(
    contract_id: &str,
    submission_id: uuid::Uuid,
    trace_hash: &str,
    gate_decision_hash: &str,
) -> anyhow::Result<serde_json::Value> {
    anyhow::ensure!(
        trace_hash.starts_with("sha256:") && trace_hash.len() == 71,
        "trace_hash must be sha256-prefixed hex"
    );
    anyhow::ensure!(
        gate_decision_hash.starts_with("sha256:") && gate_decision_hash.len() == 71,
        "gate_decision_hash must be sha256-prefixed hex"
    );
    Ok(serde_json::json!({
        "method": "record_trace_provenance",
        "contract_id": contract_id,
        "args": {
            "submission_id": submission_id.to_string(),
            "trace_hash": trace_hash,
            "gate_decision_hash": gate_decision_hash,
            "recorded_at": chrono::Utc::now().to_rfc3339(),
        }
    }))
}
```

### 6.3 Credit flow

TC credits earned from IronClaw traces can settle to the NEAR contract:

```
IronClaw agent produces trace
    -> TC contributor submits trace
    -> TC gate scores trace (perplexity + novelty)
    -> CreditQualityScore computed
    -> NearCreditReceiptCall::settle() called
    -> NEAR contract records non-transferable credit
    -> Credits usable for IronClaw compute on NEAR AI Cloud
```

The existing `NearCreditReceiptCall` in `trace-commons-server` already handles
settlement. The integration point is mapping IronClaw NEAR account IDs to TC
credit account hashes:

```rust
/// Derive the TC credit account hash from an IronClaw user's NEAR account.
pub fn ironclaw_credit_account_hash(near_account_id: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"trace_commons.credit_account.ironclaw.v1\n");
    hasher.update(near_account_id.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
```

### 6.4 Configuration

```toml
# In the TC contributor config or IronClaw's config.toml

[trace_commons]
enabled = true
ingest_url = "https://ingest.tracecommons.ai"
issuer_url = "https://issuer.tracecommons.ai"
audience = "trace-commons-ingest"
consent_scopes = ["debugging_evaluation", "ranking_training"]

[trace_commons.near]
account_id = "alice.near"
credit_contract_id = "trace-credits.near"
provenance_contract_id = "trace-provenance.near"
```

**Priority:** P2 (valuable but depends on NEAR contract deployment)
**Complexity:** Medium for identity mapping, High for contract deployment.

---

## 7. Specific integration improvements

### 7.1 Agent fingerprinting

**Goal:** Identify which IronClaw agent version produced a trace.

**Implementation:** The `AgentFingerprint` (section 5.2) is computed at
envelope assembly time and stored in `ironclaw_metadata.agent_config_hash`.
This enables:
- Tracking trace quality across agent versions.
- Detecting regressions when an agent update degrades trace quality.
- Correlating agent configurations with scoring outcomes.

```rust
/// Store the agent fingerprint in the envelope's feature_flags.
fn embed_agent_fingerprint(
    feature_flags: &mut BTreeMap<String, String>,
    fingerprint: &AgentFingerprint,
) {
    feature_flags.insert(
        "ironclaw_agent_config_hash".to_string(),
        fingerprint.config_hash.clone(),
    );
    feature_flags.insert(
        "ironclaw_runtime_version".to_string(),
        fingerprint.runtime_version.clone(),
    );
    feature_flags.insert(
        "ironclaw_agent_name".to_string(),
        fingerprint.agent_name.clone(),
    );
}
```

**API endpoint:**
```
GET /v1/analytics/agent-versions?agent_config_hash=sha256:...
Authorization: Bearer <upload-claim>

Response 200:
{
    "agent_config_hash": "sha256:...",
    "trace_count": 142,
    "mean_credit_quality_micros": 650000,
    "gate_pass_rate": 0.87,
    "first_seen": "2026-08-01T00:00:00Z",
    "last_seen": "2026-08-10T12:00:00Z"
}
```

**Priority:** P1 | **Complexity:** Low

---

### 7.2 Tool usage analytics

**Goal:** Which of IronClaw's tools are most effective?

**Implementation:** Aggregate `tool_name` from `TraceContributionEvent` where
the source is IronClaw, cross-referenced with trace quality scores.

```sql
-- Materialized view for tool effectiveness
CREATE MATERIALIZED VIEW ironclaw_tool_effectiveness AS
SELECT
    e.tool_name,
    COUNT(DISTINCT tc.submission_id) AS trace_count,
    AVG(gd.credit_quality_micros) / 1000000.0 AS mean_quality,
    COUNT(*) FILTER (WHERE gd.gate_passed) * 100.0
        / NULLIF(COUNT(*), 0) AS gate_pass_pct,
    AVG((ic.ironclaw_metadata->>'wasm_fuel_consumed')::bigint)
        AS mean_fuel_consumed
FROM trace_contribution_events e
JOIN trace_contributions tc ON tc.submission_id = e.submission_id
LEFT JOIN trace_gate_decisions gd ON gd.submission_id = tc.submission_id
LEFT JOIN trace_contributions ic ON ic.submission_id = tc.submission_id
WHERE e.event_type = 'tool_call'
  AND tc.ironclaw_metadata IS NOT NULL
  AND e.tool_name IS NOT NULL
GROUP BY e.tool_name
ORDER BY mean_quality DESC;
```

**Priority:** P1 | **Complexity:** Low

---

### 7.3 Provider comparison

**Goal:** Compare trace quality across IronClaw's 26 LLM providers.

```sql
CREATE MATERIALIZED VIEW ironclaw_provider_quality AS
SELECT
    ic.ironclaw_metadata->>'provider_name' AS provider,
    ic.ironclaw_metadata->>'provider_model' AS model,
    COUNT(*) AS trace_count,
    AVG(gd.credit_quality_micros) / 1000000.0 AS mean_quality,
    AVG(gd.perplexity_micros) / 1000000.0 AS mean_perplexity,
    AVG(gd.novelty_score_micros) / 1000000.0 AS mean_novelty,
    COUNT(*) FILTER (WHERE gd.gate_passed) * 100.0
        / NULLIF(COUNT(*), 0) AS gate_pass_pct
FROM trace_contributions ic
LEFT JOIN trace_gate_decisions gd ON gd.submission_id = ic.submission_id
WHERE ic.ironclaw_metadata IS NOT NULL
  AND ic.ironclaw_metadata->>'provider_name' IS NOT NULL
GROUP BY provider, model
ORDER BY mean_quality DESC;
```

**API endpoint:**
```
GET /v1/analytics/providers
Authorization: Bearer <upload-claim>

Response 200:
{
    "providers": [
        {
            "provider": "anthropic",
            "model": "claude-sonnet-4",
            "trace_count": 89,
            "mean_quality": 0.72,
            "gate_pass_rate": 0.91
        },
        ...
    ]
}
```

**Priority:** P1 | **Complexity:** Low

---

### 7.4 Safety scoring

**Goal:** Rate IronClaw traces on sandbox compliance.

The `sandbox_compliance_score` function (section 4.3) produces a [0,1] score.
Store it alongside the gate decision:

```rust
/// Extend the gate decision audit row with sandbox compliance.
pub struct IronClawSafetyAudit {
    pub submission_id: uuid::Uuid,
    pub sandbox_compliance: f32,
    pub capability_violations: u32,
    pub fuel_exhaustions: u32,
    pub credential_hygiene: f32,
}
```

**Priority:** P1 | **Complexity:** Low

---

### 7.5 Credential hygiene scoring

**Goal:** Score traces on credential handling practices.

```rust
/// Score credential hygiene for a trace. Returns [0, 1].
pub fn credential_hygiene_score(events: &[IronClawTraceExtension]) -> f32 {
    let mut score = 1.0_f32;
    let mut total_accesses = 0u32;
    let mut zeroized_accesses = 0u32;

    for ext in events {
        if !ext.credential_categories_accessed.is_empty() {
            total_accesses += 1;
            if ext.credential_zeroized {
                zeroized_accesses += 1;
            }
        }
    }

    if total_accesses > 0 {
        let zeroize_rate = zeroized_accesses as f32 / total_accesses as f32;
        // Non-zeroized credentials are a moderate safety concern.
        score *= 0.5 + 0.5 * zeroize_rate;
    }

    score.clamp(0.0, 1.0)
}
```

**Priority:** P2 | **Complexity:** Low

---

### 7.6 Channel effectiveness

**Goal:** Which channels produce the best agent interactions?

```sql
CREATE MATERIALIZED VIEW ironclaw_channel_effectiveness AS
SELECT
    ic.ironclaw_metadata->>'channel' AS channel,
    COUNT(*) AS trace_count,
    AVG(gd.credit_quality_micros) / 1000000.0 AS mean_quality,
    AVG(array_length(
        string_to_array(
            COALESCE(tc.outcome->>'error_taxonomy', ''),
            ','
        ), 1
    )) AS mean_errors,
    COUNT(*) FILTER (WHERE tc.outcome->>'task_success' = 'success')
        * 100.0 / NULLIF(COUNT(*), 0) AS success_pct
FROM trace_contributions ic
JOIN trace_contributions tc ON tc.submission_id = ic.submission_id
LEFT JOIN trace_gate_decisions gd ON gd.submission_id = ic.submission_id
WHERE ic.ironclaw_metadata IS NOT NULL
  AND ic.ironclaw_metadata->>'channel' IS NOT NULL
GROUP BY channel
ORDER BY mean_quality DESC;
```

**Priority:** P2 | **Complexity:** Low

---

### 7.7 Cost optimization

**Goal:** Correlate WASM fuel + LLM tokens with trace quality.

```rust
/// Compute a cost-quality ratio for an IronClaw trace.
pub struct CostQualityRatio {
    pub total_fuel: u64,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub credit_quality_micros: i64,
    /// quality_per_dollar: credit_quality / estimated_cost.
    /// Higher is better. Measures how much quality each dollar buys.
    pub quality_per_dollar: f64,
}

impl CostQualityRatio {
    pub fn compute(
        wasm: &WasmTraceAggregate,
        total_input_tokens: u32,
        total_output_tokens: u32,
        token_cost_per_million: f64,
        credit_quality_micros: i64,
    ) -> Self {
        let token_cost = (total_input_tokens + total_output_tokens) as f64
            / 1_000_000.0
            * token_cost_per_million;
        // WASM fuel cost is negligible but tracked for completeness.
        let fuel_cost = wasm.total_fuel_consumed as f64 * 1e-12;
        let estimated_cost_usd = token_cost + fuel_cost;
        let quality_per_dollar = if estimated_cost_usd > 0.0 {
            (credit_quality_micros as f64 / 1_000_000.0) / estimated_cost_usd
        } else {
            0.0
        };
        Self {
            total_fuel: wasm.total_fuel_consumed,
            total_input_tokens,
            total_output_tokens,
            estimated_cost_usd,
            credit_quality_micros,
            quality_per_dollar,
        }
    }
}
```

**Priority:** P2 | **Complexity:** Medium (requires token cost lookup per
provider/model)

---

### 7.8 Federated scoring

**Goal:** Distributed scoring where IronClaw's TEE and TC's TEE each handle
part of the evaluation.

**Architecture:**

```
[IronClaw TEE]                    [TC Gate TEE]
     |                                 |
     |-- local pre-score:              |
     |   - WASM metrics               |
     |   - credential hygiene          |
     |   - sandbox compliance          |
     |                                 |
     |-- PreScoreReport ------------> |
     |                                 |-- perplexity scoring
     |                                 |-- novelty scoring
     |                                 |-- credit quality
     |                                 |
     |                                 |-- merge pre-score + gate score
     | <-- FederatedDecision ---------|
```

```rust
/// Pre-score computed inside the IronClaw TEE, sent to TC for merging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IronClawPreScore {
    pub schema_version: String,
    pub submission_id: uuid::Uuid,
    pub sandbox_compliance: f32,
    pub credential_hygiene: f32,
    pub wasm_efficiency: f32,
    pub channel_quality_hint: f32,
    /// TEE attestation proving this pre-score was computed in an enclave.
    pub attestation_hash: String,
}

/// Combined decision merging IronClaw pre-score with TC gate score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedDecision {
    pub gate_decision: OrchestrationDecisionSummary,
    pub ironclaw_pre_score: IronClawPreScore,
    /// Weighted combination: 0.7 * gate_quality + 0.3 * ironclaw_quality
    pub federated_quality_micros: i64,
    pub chained_attestation_hash: String,
}

/// Minimal summary of the gate decision for the federated response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationDecisionSummary {
    pub perplexity_passed: bool,
    pub novelty_passed: bool,
    pub credit_quality_micros: i64,
    pub gate_policy_version: String,
}
```

**API endpoint:**
```
POST /v1/gate/federated-evaluate
Authorization: Bearer <upload-claim>
Content-Type: application/json

{
    "submission_id": "uuid",
    "pre_score": { ... },
    "envelope": { ... }
}

Response 200:
{
    "federated_quality_micros": 720000,
    "gate_passed": true,
    "chained_attestation_hash": "sha256:..."
}
```

**Priority:** P3 (requires both TEE deployments to be mature)
**Complexity:** Very High -- distributed scoring introduces consensus and
availability challenges.

---

## 8. Implementation roadmap

| Phase | Deliverable | Priority | Complexity | Dependencies |
|---|---|---|---|---|
| 1a | `IronClawSource` adapter in `trace-commons-contributor` | P0 | Medium | IronClaw event format stability |
| 1b | `IronClawTraceExtension` in `trace-commons-protocol` | P0 | Low | None |
| 1c | `TraceChannel` enum extension (Discord, Signal, WhatsApp) | P0 | Low | None |
| 2a | `ironclaw_metadata` JSONB column migration | P1 | Low | Phase 1 |
| 2b | WASM quality signals (`wasm_quality_factor`, `sandbox_compliance_score`) | P1 | Medium | Phase 1 + IronClaw fuel metrics |
| 2c | Channel-specific redaction rules | P1 | Medium | Phase 1 |
| 2d | Agent fingerprinting in feature_flags | P1 | Low | Phase 1 |
| 2e | Tool usage analytics materialized view | P1 | Low | Phase 2a |
| 2f | Provider comparison materialized view | P1 | Low | Phase 2a |
| 3a | TEE attestation chain types | P2 | High | IronClaw TEE attestation API |
| 3b | On-chain trace provenance contract | P2 | High | NEAR contract deployment |
| 3c | Credit account mapping (NEAR account -> TC credit hash) | P2 | Medium | Phase 3b |
| 3d | Credential hygiene scoring | P2 | Low | Phase 2b |
| 3e | Channel effectiveness analytics | P2 | Low | Phase 2a |
| 4a | Federated scoring protocol | P3 | Very High | Phases 3a + 3b |
| 4b | Cost-quality optimization analytics | P2 | Medium | Phase 2e + provider cost data |

### Phase 1 (Weeks 1-3): Foundation

Ship the `IronClawSource` adapter and protocol extensions. At the end of
Phase 1, IronClaw traces flow into TC through the existing `trace-commons
submit` pipeline. No server-side changes required -- the extension metadata
rides in `structured_payload` and `feature_flags`, which the server already
stores as-is.

### Phase 2 (Weeks 4-8): Analytics

Add the PostgreSQL schema, materialized views, and WASM-specific scoring.
This phase makes IronClaw traces queryable and scoreable with IronClaw-
specific signals. The channel analytics and agent fingerprinting enable
per-agent and per-channel quality tracking.

### Phase 3 (Weeks 9-16): Ecosystem

Wire TEE attestation bridging, on-chain provenance, and NEAR identity
mapping. This phase requires coordination with the NEAR contract team and
IronClaw's TEE deployment pipeline.

### Phase 4 (Weeks 17+): Federated

Implement federated scoring with pre-scores computed in IronClaw's TEE and
merged with TC gate scores. This is the most complex phase and should only
begin after Phases 1-3 are stable in production.

---

## 9. Configuration reference

### 9.1 IronClaw side (`~/.ironclaw/config.toml`)

```toml
[trace_commons]
enabled = true
ingest_url = "https://ingest.tracecommons.ai"
issuer_url = "https://issuer.tracecommons.ai"
audience = "trace-commons-ingest"

# Consent: which uses are permitted for traces from this agent.
# Options: debugging_evaluation, benchmark_only, ranking_training, model_training
consent_scopes = ["debugging_evaluation", "ranking_training"]

# PII filter backend: "near-ai" uses NEAR AI's privacy filter,
# null/omitted uses deterministic-only redaction.
pii_filter = "near-ai"

# NEAR identity for credit settlement.
near_account_id = "alice.near"

# Include WASM fuel metrics in trace metadata.
include_wasm_metrics = true

# Include credential access patterns (redacted to categories).
include_credential_patterns = true
```

### 9.2 TC server side (environment variables)

```bash
# Enable IronClaw-specific analytics (materialized view refresh).
TRACE_COMMONS_IRONCLAW_ANALYTICS_ENABLED=true

# Refresh interval for materialized views (seconds).
TRACE_COMMONS_IRONCLAW_ANALYTICS_REFRESH_INTERVAL=3600

# NEAR contract for credit settlement.
TRACE_COMMONS_NEAR_CREDIT_CONTRACT_ID=trace-credits.near

# NEAR contract for trace provenance.
TRACE_COMMONS_NEAR_PROVENANCE_CONTRACT_ID=trace-provenance.near

# TEE attestation verification.
TRACE_COMMONS_TEE_ATTESTATION_ENABLED=true
TRACE_COMMONS_TEE_ATTESTATION_ALLOWED_MEASUREMENTS=sha256:abc...,sha256:def...
```

### 9.3 TC contributor side (`~/.config/trace-commons/contributor.json`)

```json
{
    "schema_version": "trace_commons.contributor_config.v1",
    "issuer_url": "https://issuer.tracecommons.ai",
    "ingest_url": "https://ingest.tracecommons.ai",
    "audience": "trace-commons-ingest",
    "tenant_id": "tenant-abc",
    "instance_id": "ironclaw-alice-macbook",
    "user_subject": "alice@near",
    "device_key_id": "sha256:...",
    "consent_scopes": ["debugging_evaluation", "ranking_training"],
    "pii_filter": "near-ai",
    "allowed_hosts": "ingest.tracecommons.ai,issuer.tracecommons.ai"
}
```

---

## 10. Open questions

1. **IronClaw event format stability.** The `IronClawSource` adapter depends on
   the structure of IronClaw's event log files. If these change between
   releases, the adapter breaks. Should the adapter target a versioned export
   format, or should IronClaw ship a `trace-commons` extension that handles
   export natively?

2. **Fuel metering granularity.** IronClaw's fuel metering is per-invocation.
   Should TC track fuel at the per-tool-call level (more useful for analytics)
   or per-session level (simpler, lower storage)?

3. **Channel consent scope.** Should different channels be allowed different
   consent scopes? A user might consent to `model_training` for CLI traces but
   only `debugging_evaluation` for Telegram traces, since Telegram traces may
   contain more sensitive conversational content.

4. **TEE attestation verification latency.** Verifying SGX DCAP or SEV-SNP
   attestation reports requires contacting Intel/AMD verification services.
   Should this be synchronous (blocking ingest) or asynchronous (verify after
   acceptance, quarantine on failure)?

5. **Credit fungibility.** The current TC credit system is non-transferable.
   Should credits earned from IronClaw traces be usable for IronClaw compute
   directly, or should they remain strictly TC-internal with a separate
   redemption mechanism?
