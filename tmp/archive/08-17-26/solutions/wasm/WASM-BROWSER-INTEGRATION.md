# WASM Browser Integration for Roko

**Date**: 2026-04-29
**Status**: Research / design doc
**Audience**: Developer eval, product demo, dogfooding (all)

---

## Executive Summary

Roko can be partially compiled to WebAssembly to run agent conversations, prompt
composition, and plan visualization directly in the browser — no backend required.
The heaviest capabilities (gates, file I/O, process management) still need a server,
but a meaningful demo can run entirely client-side.

**What you get**: Browser calls Claude/OpenAI directly via fetch. The 9-layer system
prompt is assembled in WASM. Plans are parsed and visualized client-side. The PRD
pipeline demo works with zero `roko serve` running.

**What you don't get**: Compilation gates, file operations, process lifecycle, the
full orchestration loop. Those still need the backend.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [What Can Run in WASM](#2-what-can-run-in-wasm)
3. [What Cannot Run in WASM](#3-what-cannot-run-in-wasm)
4. [Demo Scenarios](#4-demo-scenarios)
5. [The roko-wasm Crate](#5-the-roko-wasm-crate)
6. [Implementation Details](#6-implementation-details)
7. [Integration with Demo App](#7-integration-with-demo-app)
8. [Build Pipeline](#8-build-pipeline)
9. [Work Breakdown](#9-work-breakdown)
10. [Risks and Mitigations](#10-risks-and-mitigations)
11. [Ecosystem Reference](#11-ecosystem-reference)
12. [Standalone Deployment Mode](#12-standalone-deployment-mode)
13. [Knowledge Workflows in the Browser](#13-knowledge-workflows-in-the-browser)
14. [Standalone Dashboard: Page-by-Page](#14-standalone-dashboard-page-by-page)
15. [Use Cases for the Standalone Deploy](#15-use-cases-for-the-standalone-deploy)

---

## 1. Architecture Overview

```
┌──────────────────────────────────────────────────────┐
│                   Browser Tab                         │
│                                                       │
│  ┌─────────────────┐  ┌────────────────────────────┐ │
│  │  React Frontend  │  │      roko-wasm (WASM)      │ │
│  │  (Vite + TS)     │◄─►                            │ │
│  │                  │  │  SystemPromptBuilder        │ │
│  │  PrdPipelinePanel│  │  WasmFetchPoster            │ │
│  │  Builder         │  │  PlanParser                 │ │
│  │  WorkflowConst.  │  │  HDC (roko-primitives)     │ │
│  │  Demo            │  │  DaimonState (affect)       │ │
│  └────────┬─────────┘  │  TokenBudget               │ │
│           │            └──────────┬─────────────────┘ │
│           │                       │                    │
│           │     ┌─────────────────▼──────────┐        │
│           │     │  Browser Fetch API          │        │
│           │     │  (direct LLM calls)         │        │
│           │     └─────────────┬──────────────┘        │
│           │                   │                        │
└───────────┼───────────────────┼────────────────────────┘
            │                   │
   ┌────────▼────────┐  ┌──────▼──────────┐
   │  roko-serve      │  │  Claude API      │
   │  :6677           │  │  OpenAI API      │
   │  (optional)      │  │  Gemini API      │
   │                  │  │  (direct)        │
   │  gates, files,   │  └─────────────────┘
   │  process mgmt    │
   └──────────────────┘
```

**Two modes of operation:**

| Mode | LLM Access | Backend Required | Use Case |
|---|---|---|---|
| **Direct** | Browser → LLM API | No | Local dev, standalone demo, zero-infra |
| **Proxy** | Browser → roko-serve → LLM API | Yes | Hosted demo, API key security, full features |

The WASM module provides the same Rust logic the CLI uses (prompt composition, plan
parsing, scoring) but executes in the browser. The React app calls into it via
`wasm-bindgen` exports.

---

## 2. What Can Run in WASM

### Tier 1: Ready now (minimal changes)

| Component | Crate | Blocker | Fix |
|---|---|---|---|
| HDC vectors, manifolds, scoring | `roko-primitives` | `getrandom` needs `wasm_js` feature | 1 line in app crate |
| Signal/Engram types | `roko-core` (subset) | `tokio::sync`, `parking_lot` | Feature-gate behind `#[cfg(not(wasm))]` |
| Tier routing | `roko-primitives` | Same as HDC | Same |

### Tier 2: Achievable with feature-gating

| Component | Crate | Blocker | Fix |
|---|---|---|---|
| 9-layer SystemPromptBuilder | `roko-compose` | Transitive deps (`roko-agent`, `roko-learn`) | Extract `Playbook`/`Skill` types to `roko-core`; feature-gate `TokenCounter` |
| Token budget estimation | `roko-compose` | `tiktoken-rs` uses `rayon` | Use `Heuristic` variant (already exists: `text.len() / 4`) |
| Prompt templates | `roko-compose` | None — already I/O-free by design | Just needs the dep chain fix |
| Affect engine (daimon) | `roko-daimon` | `roko-core` blocker | Fixed once core has wasm feature |
| Plan/task TOML parsing | `roko-orchestrator` | Subset only — parser is pure, executor is not | Extract parsing into shared module |

### Tier 3: New code needed

| Component | What | Effort |
|---|---|---|
| WasmFetchPoster | `HttpPoster` impl using `web_sys::fetch` | ~150 LOC |
| Agent conversation loop | Multi-turn tool-use loop running in browser | ~300 LOC (port from `tool_loop/mod.rs`) |
| IndexedDB substrate | Replace `FileSubstrate` for browser persistence | ~400 LOC |
| Plan DAG renderer data | Serialize plan DAG for WorkflowConstellation | ~100 LOC |

---

## 3. What Cannot Run in WASM

These are **fundamental** OS-level blockers, not fixable with feature flags:

| Component | Why | Alternative |
|---|---|---|
| Gates (compile, test, clippy) | Spawns `cargo` subprocesses | Show gate results from backend via SSE |
| ExecAgent / Claude CLI agent | Spawns OS processes | Use HTTP-based agents (ClaudeAgent, OpenAiAgent) |
| File substrate (roko-fs) | `tokio::fs` everywhere | IndexedDB for ephemeral browser state |
| Process supervisor | `nix` + `tokio::process` | Not applicable in browser |
| Full orchestration loop | Needs all of the above | Run client-side planning, server-side execution |
| MCP servers | Stdin/stdout IPC, TCP sockets | Not applicable in browser |
| Terminal PTY | OS-level pseudo-terminal | Keep using WebSocket to roko-serve PTY |

---

## 4. Demo Scenarios

### Demo A: "Zero-Backend Agent Chat" (standalone, no server)

**What the user sees**: A chat interface where they paste their API key, type a prompt,
and watch a Claude agent respond — with the 9-layer system prompt assembled in Rust/WASM
right in their browser, and the LLM call going directly from their browser to the API.

**Why it's impressive**: Shows that roko's prompt engineering runs at native speed in the
browser. The system prompt is not a static string — it's dynamically assembled from role,
conventions, domain context, task, tools, anti-patterns, affect state, and learned
techniques. The user sees this assembly happen.

**Components needed**:
- `roko-wasm` with `SystemPromptBuilder` + `WasmFetchPoster`
- React chat UI (new component, ~200 LOC)
- API key input + model picker (reuse existing `model-catalog.ts`)

**Data flow**:
```
User types prompt
    → WASM: SystemPromptBuilder.build() assembles 9-layer system prompt
    → WASM: WasmFetchPoster.post_json() calls Claude API via browser fetch
    → React: renders streaming response (SSE via ReadableStream)
    → WASM: (optional) DaimonState updates affect based on response quality
    → loop
```

**Backend requirement**: None. Fully standalone.

### Demo B: "Live PRD Pipeline — Browser-Native" (enhanced current demo)

**What the user sees**: The existing PrdPipelinePanel, but the early stages (idea capture,
PRD drafting, plan generation) run entirely in-browser. The pipeline transitions from
WASM-powered to server-powered when it hits execution (gates, file writes).

**Why it's impressive**: Shows graceful degradation. The demo works even if `roko serve`
isn't running — it just stops at the "Tasks" phase and shows a "connect to roko-serve
for execution" prompt. When the server is available, it seamlessly picks up.

**Components needed**:
- Everything from Demo A
- Plan TOML parser compiled to WASM (extract from `roko-orchestrator`)
- Enhanced PrdPipelinePanel that accepts WASM-generated state

**Data flow**:
```
Idea text entered
    → WASM: build system prompt for PRD-writer agent
    → WASM: WasmFetchPoster calls Claude API
    → WASM: parse PRD response into PrdDocument
    → React: PrdPipelinePanel renders PRD card
    → WASM: build system prompt for plan-generator agent
    → WASM: WasmFetchPoster calls Claude API
    → WASM: parse plan response, extract tasks.toml
    → React: renders plan + task board
    → [if roko-serve available]:
        → HTTP: POST /api/plans/execute
        → SSE/WS: stream gate results, agent progress
    → [if not]:
        → UI: "Connect roko-serve to execute"
```

### Demo C: "Prompt Workshop" (interactive prompt builder)

**What the user sees**: An interactive editor where they configure each of the 9 prompt
layers and see the assembled system prompt update in real-time. Slider controls for token
budget. Toggle layers on/off. See the final prompt, send it to an LLM, see the response.

**Why it's impressive**: Makes the composition system tangible. Users understand what roko
is doing under the hood. They can experiment with prompt engineering using roko's framework.

**Components needed**:
- `roko-wasm` with `SystemPromptBuilder` + budget estimation
- React editor with 9 section panels (new component, ~500 LOC)
- Token count display (use heuristic counter from WASM)

**Backend requirement**: None for composition. Optional for sending to LLM.

### Demo D: "Plan Visualizer" (DAG explorer)

**What the user sees**: Upload or paste a `tasks.toml` file, and see the dependency DAG
rendered as an interactive graph (reuse WorkflowConstellation). Click tasks to see details.
Simulate execution order. Identify critical paths.

**Components needed**:
- TOML parser compiled to WASM
- DAG construction logic (extract from `roko-orchestrator`)
- Enhanced WorkflowConstellation with clickable nodes

**Backend requirement**: None. Fully standalone.

### Demo E: "Cost Estimator" (pre-flight analysis)

**What the user sees**: Given a plan, estimate the token cost across all tasks by
assembling the system prompts and counting tokens — entirely in-browser. Show per-task
and total cost breakdowns by model.

**Components needed**:
- Everything from Demo D
- SystemPromptBuilder per task
- Token budget estimation
- Model pricing data (from existing `model-catalog.ts`)

**Backend requirement**: None.

---

## 5. The roko-wasm Crate

### Crate structure

```
crates/roko-wasm/
├── Cargo.toml
├── src/
│   ├── lib.rs              # wasm-bindgen exports
│   ├── fetch_poster.rs     # WasmFetchPoster: HttpPoster
│   ├── agent.rs            # Browser agent conversation loop
│   ├── prompt.rs           # SystemPromptBuilder wasm bindings
│   ├── plan.rs             # Plan/task TOML parser bindings
│   ├── storage.rs          # IndexedDB substrate (optional)
│   └── types.rs            # Shared types for JS interop
└── tests/
    └── web.rs              # wasm-bindgen-test browser tests
```

### Cargo.toml

```toml
[package]
name = "roko-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# WASM interop
wasm-bindgen = "0.2.118"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "Window", "Request", "RequestInit", "RequestMode",
    "Headers", "Response", "ReadableStream", "ReadableStreamDefaultReader",
    "AbortController", "AbortSignal",
    "console",
] }
serde-wasm-bindgen = "0.6"
console_error_panic_hook = "0.1"

# Roko deps — WASM-compatible subset only
roko-primitives = { path = "../roko-primitives" }
roko-core = { path = "../roko-core", default-features = false, features = ["wasm"] }
roko-compose = { path = "../roko-compose", default-features = false, features = ["wasm"] }
roko-daimon = { path = "../roko-daimon", optional = true }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Async
futures = "0.3"

# RNG for WASM
getrandom = { version = "0.4", features = ["wasm_js"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### Exported API (wasm-bindgen)

```rust
use wasm_bindgen::prelude::*;

// ── Prompt composition ──────────────────────────────────

#[wasm_bindgen]
pub struct PromptBuilder { /* wraps SystemPromptBuilder */ }

#[wasm_bindgen]
impl PromptBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(role_identity: &str) -> Self;

    pub fn with_conventions(&mut self, text: &str) -> &mut Self;
    pub fn with_domain(&mut self, text: &str) -> &mut Self;
    pub fn with_task(&mut self, text: &str) -> &mut Self;
    pub fn with_tools(&mut self, text: &str) -> &mut Self;
    pub fn with_anti_patterns(&mut self, patterns: JsValue) -> &mut Self; // Vec<String>
    pub fn with_gate_feedback(&mut self, text: &str) -> &mut Self;

    pub fn build(&self) -> String;
    pub fn estimate_tokens(&self) -> usize;
    pub fn sections_json(&self) -> JsValue; // serialized section breakdown
}

// ── LLM calls ───────────────────────────────────────────

#[wasm_bindgen]
pub struct AgentHandle { /* wraps WasmFetchPoster + conversation state */ }

#[wasm_bindgen]
impl AgentHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<AgentHandle, JsError>;
    // config: { provider, model, api_key, api_url?, system_prompt? }

    pub async fn send_message(&mut self, content: &str) -> Result<JsValue, JsError>;
    // returns: { role, content, usage: { input_tokens, output_tokens } }

    pub fn conversation_json(&self) -> JsValue;
    pub fn total_cost(&self) -> f64;
    pub fn total_tokens(&self) -> u64;
    pub fn clear_history(&mut self);
}

// ── Plan parsing ────────────────────────────────────────

#[wasm_bindgen]
pub fn parse_plan(toml_text: &str) -> Result<JsValue, JsError>;
// returns: { tasks: [...], dag_edges: [...], critical_path: [...] }

#[wasm_bindgen]
pub fn validate_plan(toml_text: &str) -> Result<JsValue, JsError>;
// returns: { valid: bool, errors: [...], warnings: [...] }

// ── Utilities ───────────────────────────────────────────

#[wasm_bindgen]
pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64;

#[wasm_bindgen]
pub fn hdc_fingerprint(text: &str) -> JsValue; // HDC vector as Float32Array

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
```

---

## 6. Implementation Details

### 6.1 WasmFetchPoster

The critical piece: implements `HttpPoster` using the browser Fetch API.

```rust
// crates/roko-wasm/src/fetch_poster.rs

use roko_agent::http::{HttpPostError, HttpPoster};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Headers, Response, AbortController};

pub struct WasmFetchPoster;

// NOTE: We use async_trait(?Send) because WASM is single-threaded.
// The trait requires Send + Sync, so we need a cfg-gated version
// of HttpPoster for WASM, or an unsafe impl Send/Sync (safe because
// WASM is genuinely single-threaded).
#[async_trait::async_trait(?Send)]
impl HttpPoster for WasmFetchPoster {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        // 1. Create abort controller for timeout
        let abort = AbortController::new()
            .map_err(|e| HttpPostError::transport(format!("AbortController: {e:?}")))?;
        let signal = abort.signal();

        // Set timeout via setTimeout + abort
        let timeout_closure = Closure::once(move || abort.abort());
        web_sys::window().unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                timeout_closure.as_ref().unchecked_ref(),
                timeout_ms as i32,
            ).ok();
        timeout_closure.forget(); // prevent drop

        // 2. Build request
        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.mode(RequestMode::Cors);
        opts.signal(Some(&signal));

        // Body
        let body_str = String::from_utf8_lossy(body);
        opts.body(Some(&JsValue::from_str(&body_str)));

        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| HttpPostError::transport(format!("Request: {e:?}")))?;

        // 3. Headers
        let req_headers = request.headers();
        req_headers.set("Content-Type", "application/json").ok();
        for (k, v) in headers {
            req_headers.set(k, v).ok();
        }

        // 4. Fetch
        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| HttpPostError::transport(format!("fetch: {e:?}")))?;

        let response: Response = resp_value.dyn_into()
            .map_err(|_| HttpPostError::transport("not a Response"))?;

        // 5. Read body text
        let text = JsFuture::from(response.text()
            .map_err(|e| HttpPostError::transport(format!("text(): {e:?}")))?)
            .await
            .map_err(|e| HttpPostError::transport(format!("text await: {e:?}")))?
            .as_string()
            .unwrap_or_default();

        // 6. Check status
        let status = response.status();
        if status >= 400 {
            return Err(HttpPostError::http(status, text));
        }

        Ok(text)
    }
}
```

**Key design decisions:**

- **Send + Sync**: WASM is single-threaded. `JsFuture` and `web_sys` types are `!Send`.
  The `HttpPoster` trait requires `Send + Sync`. Two approaches:
  1. **Recommended**: Add `#[cfg(target_arch = "wasm32")]` to use `async_trait(?Send)` on
     a separate trait impl block, with `unsafe impl Send for WasmFetchPoster {}` (safe
     because WASM has no threads).
  2. **Alternative**: Fork the `HttpPoster` trait for WASM. More type-safe but duplicates
     the interface.

- **Timeout**: Browser Fetch has no native timeout. We use `AbortController` + `setTimeout`.

- **Streaming**: `HttpPoster` is request/response only (returns `String`). Streaming SSE
  from Claude's API would need a separate `send_turn_streaming` impl on `LlmBackend`
  that uses `ReadableStream` from `web_sys`. This is a Tier 2 feature.

- **CORS**: The Claude API sets `Access-Control-Allow-Origin: *` for direct browser calls.
  OpenAI does too. Gemini works. This is the path that lets us skip a proxy entirely.

### 6.2 Feature flags needed in existing crates

#### roko-core

```toml
[features]
default = ["full"]
full = ["tokio-sync", "parking-lot", "forensics"]
wasm = []  # excludes tokio::sync, parking_lot, std::fs modules
tokio-sync = ["dep:tokio"]
parking-lot = ["dep:parking_lot"]
forensics = []  # gates forensic.rs which uses std::fs
```

Changes:
- Gate `tokio::sync::{Mutex, RwLock, watch}` behind `tokio-sync` feature
- Gate `parking_lot` behind its own feature
- Gate `forensic.rs`, `secrets/file.rs` behind `forensics`
- Under `wasm` feature, use `std::sync::{Mutex, RwLock}` instead (or `futures::lock::Mutex`)

#### roko-compose

```toml
[features]
default = ["full"]
full = ["tokenizer", "enrichment", "dep:roko-agent", "dep:roko-learn"]
wasm = []  # just SystemPromptBuilder + templates + budget
tokenizer = ["dep:tiktoken-rs", "dep:tokenizers"]
enrichment = ["dep:roko-agent"]
```

Changes:
- Make `roko-agent` and `roko-learn` optional deps
- Extract `Playbook`, `Skill`, `SectionEffectivenessRegistry` types:
  - Option A: Move to `roko-core` (where they arguably belong)
  - Option B: Create `roko-types` crate (minimal, no deps beyond serde)
  - Option C: Duplicate the 3 structs in `roko-wasm` (pragmatic, ugly)
- Feature-gate `token_counter.rs`, `context_provider.rs`, `enrichment/`, `prompt_assembly_service.rs`, `symbol_resolver.rs`

#### roko-agent (minimal changes)

```toml
[features]
default = ["full"]
full = ["exec", "cli", "mcp", "process"]
http-only = []  # just HttpPoster trait + claude_agent + openai_agent
exec = ["dep:tokio/process"]
cli = ["exec"]
mcp = ["dep:tokio/process"]
process = ["dep:tokio/process", "dep:nix", "dep:libc"]
```

For WASM, we only need the `HttpPoster` trait definition and the agent structs
(`ClaudeAgent`, `OpenAiAgent`). The `roko-wasm` crate wouldn't depend on `roko-agent`
at all — it would just re-implement `HttpPoster` locally and build the request JSON
directly.

### 6.3 Browser agent conversation loop

The conversation loop for a browser agent (simplified from `tool_loop/mod.rs`):

```rust
// crates/roko-wasm/src/agent.rs

pub struct BrowserAgent {
    poster: WasmFetchPoster,
    config: AgentConfig,        // provider, model, api_key, api_url
    system_prompt: String,
    messages: Vec<Message>,     // conversation history
    total_input_tokens: u64,
    total_output_tokens: u64,
}

impl BrowserAgent {
    pub async fn send_message(&mut self, content: &str) -> Result<Message, AgentError> {
        self.messages.push(Message::user(content));

        // Build request body (provider-specific)
        let (url, headers, body) = match self.config.provider {
            Provider::Anthropic => self.build_claude_request(),
            Provider::OpenAI => self.build_openai_request(),
            Provider::Google => self.build_gemini_request(),
        };

        // Call LLM via browser fetch
        let response_text = self.poster.post_json(&url, &headers, &body, 120_000).await?;

        // Parse response (provider-specific)
        let response = self.parse_response(&response_text)?;

        self.messages.push(Message::assistant(&response.content));
        self.total_input_tokens += response.usage.input_tokens;
        self.total_output_tokens += response.usage.output_tokens;

        Ok(response)
    }
}
```

**No tool execution in browser.** The browser agent is a conversation loop only.
Tool calls returned by the LLM are surfaced to the React UI for display, not executed.
Actual tool execution (Read, Write, Bash) requires the backend.

An alternative for future work: a limited tool set that works in-browser (e.g., a
"think" tool, "plan" tool, "search" tool that calls a web search API).

### 6.4 IndexedDB substrate (optional, Tier 2)

For persisting conversations, plans, and PRDs in the browser:

```rust
// crates/roko-wasm/src/storage.rs

use indexed_db::Factory;

pub struct IdbSubstrate {
    db: indexed_db::Database,
}

impl IdbSubstrate {
    pub async fn open(name: &str) -> Result<Self, StorageError> {
        let factory = Factory::get().unwrap();
        let db = factory.open(name, 1, |evt| {
            let db = evt.database();
            db.build_object_store("signals").auto_increment().create()?;
            db.build_object_store("episodes").auto_increment().create()?;
            db.build_object_store("plans").auto_increment().create()?;
            Ok(())
        }).await?;
        Ok(Self { db })
    }

    pub async fn append_signal(&self, signal: &Signal) -> Result<(), StorageError> {
        let tx = self.db.transaction("signals").rw();
        let store = tx.object_store("signals")?;
        let js_val = serde_wasm_bindgen::to_value(signal)?;
        store.add(&js_val).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn query_signals(&self, kind: &str) -> Result<Vec<Signal>, StorageError> {
        // ... indexed query
    }
}
```

This is optional for MVP. Conversations can live in React state. IndexedDB becomes
valuable when you want persistence across page reloads.

### 6.5 Streaming responses (Tier 2)

For streaming LLM responses in the browser (not through `HttpPoster`, which is
request/response):

```rust
pub async fn send_message_streaming(
    &mut self,
    content: &str,
    on_chunk: &js_sys::Function,  // JS callback for each chunk
) -> Result<Message, AgentError> {
    // ... build request with stream: true ...

    let response = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response.dyn_into()?;

    let body = response.body().ok_or("no body")?;
    let reader = body.get_reader().dyn_into::<ReadableStreamDefaultReader>()?;

    let decoder = web_sys::TextDecoder::new()?;
    let mut accumulated = String::new();

    loop {
        let result = JsFuture::from(reader.read()).await?;
        let done = js_sys::Reflect::get(&result, &"done".into())?.as_bool().unwrap();
        if done { break; }

        let value = js_sys::Reflect::get(&result, &"value".into())?;
        let chunk = decoder.decode_with_buffer_source(&value)?;

        // Parse SSE lines, extract content deltas
        for line in chunk.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                    if let Some(text) = event.content_delta() {
                        accumulated.push_str(&text);
                        let _ = on_chunk.call1(&JsValue::NULL, &text.into());
                    }
                }
            }
        }
    }

    Ok(Message::assistant(&accumulated))
}
```

---

## 7. Integration with Demo App

### Current demo architecture (no WASM)

```
demo-app/
├── src/
│   ├── hooks/
│   │   ├── useApi.ts          → fetch to localhost:6677
│   │   ├── useSSE.ts          → EventSource to localhost:6677
│   │   ├── useTerminal.ts     → WebSocket PTY to localhost:6677
│   │   └── useServerHealth.ts → polls /health every 5s
│   ├── lib/
│   │   ├── serve-url.ts       → SERVE_URL = http://localhost:6677
│   │   └── workflow-api.ts    → SSE + WS workflow subscriptions
│   ├── components/
│   │   ├── PrdPipelinePanel.tsx
│   │   └── WorkflowConstellation.tsx (Three.js)
│   └── pages/
│       ├── Builder.tsx         → terminal-based, needs server
│       └── Demo.tsx            → scripted scenarios
```

### With WASM integration

```
demo-app/
├── src/
│   ├── hooks/
│   │   ├── useApi.ts              (unchanged)
│   │   ├── useSSE.ts              (unchanged)
│   │   ├── useTerminal.ts         (unchanged)
│   │   ├── useServerHealth.ts     (unchanged)
│   │   ├── useRokoWasm.ts         NEW — lazy-loads WASM module
│   │   ├── usePromptBuilder.ts    NEW — wraps PromptBuilder WASM calls
│   │   └── useWasmAgent.ts        NEW — wraps AgentHandle for React
│   ├── lib/
│   │   ├── serve-url.ts           (unchanged)
│   │   ├── workflow-api.ts        (unchanged)
│   │   ├── wasm-loader.ts         NEW — init(), lazy load, error boundary
│   │   └── wasm-types.ts          NEW — TS types matching WASM exports
│   ├── components/
│   │   ├── PrdPipelinePanel.tsx    (unchanged for now)
│   │   ├── WorkflowConstellation.tsx (unchanged)
│   │   ├── WasmAgentChat.tsx       NEW — zero-backend chat component
│   │   ├── PromptWorkshop.tsx      NEW — interactive prompt builder
│   │   └── PlanVisualizer.tsx      NEW — upload + visualize plans
│   └── pages/
│       ├── Builder.tsx             (unchanged — still needs server)
│       ├── Demo.tsx                (enhanced — WASM fallback for pipeline)
│       ├── Explore.tsx             NEW — WASM demos page
│       └── ...
├── public/
│   └── wasm/
│       ├── roko_wasm_bg.wasm       Built artifact
│       └── roko_wasm.js            Generated JS glue
└── vite.config.ts                  Add WASM plugin (vite-plugin-wasm)
```

### New hooks

#### `useRokoWasm.ts`

```typescript
import { useState, useEffect } from 'react';

let wasmModule: typeof import('../../public/wasm/roko_wasm') | null = null;
let wasmPromise: Promise<typeof wasmModule> | null = null;

export function useRokoWasm() {
  const [ready, setReady] = useState(!!wasmModule);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (wasmModule) { setReady(true); return; }
    if (!wasmPromise) {
      wasmPromise = import('../../public/wasm/roko_wasm').then(async (mod) => {
        await mod.default(); // init WASM
        wasmModule = mod;
        return mod;
      });
    }
    wasmPromise.then(() => setReady(true)).catch(setError);
  }, []);

  return { wasm: wasmModule, ready, error };
}
```

#### `useWasmAgent.ts`

```typescript
import { useState, useCallback, useRef } from 'react';
import { useRokoWasm } from './useRokoWasm';

interface AgentConfig {
  provider: 'anthropic' | 'openai' | 'google';
  model: string;
  apiKey: string;
  apiUrl?: string;
  systemPrompt?: string;
}

interface Message {
  role: 'user' | 'assistant';
  content: string;
  usage?: { input_tokens: number; output_tokens: number };
}

export function useWasmAgent(config: AgentConfig | null) {
  const { wasm, ready } = useRokoWasm();
  const agentRef = useRef<any>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCost, setTotalCost] = useState(0);

  const sendMessage = useCallback(async (content: string) => {
    if (!wasm || !config) return;
    if (!agentRef.current) {
      agentRef.current = new wasm.AgentHandle(config);
    }
    setLoading(true);
    setMessages(prev => [...prev, { role: 'user', content }]);

    try {
      const response = await agentRef.current.send_message(content);
      setMessages(prev => [...prev, {
        role: 'assistant',
        content: response.content,
        usage: response.usage,
      }]);
      setTotalCost(agentRef.current.total_cost());
    } finally {
      setLoading(false);
    }
  }, [wasm, config]);

  return { messages, sendMessage, loading, totalCost, ready };
}
```

### Vite configuration

```typescript
// vite.config.ts additions
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
  ],
  optimizeDeps: {
    exclude: ['roko-wasm'], // don't pre-bundle WASM
  },
});
```

---

## 8. Build Pipeline

### Building the WASM module

```bash
# From workspace root

# 1. Build WASM (release, optimized for size)
cargo build \
  --target wasm32-unknown-unknown \
  --release \
  -p roko-wasm

# 2. Generate JS bindings
wasm-bindgen \
  --target web \
  --out-dir demo/demo-app/public/wasm \
  target/wasm32-unknown-unknown/release/roko_wasm.wasm

# 3. Optimize (reduces size 10-30%)
wasm-opt -Oz \
  -o demo/demo-app/public/wasm/roko_wasm_bg.wasm \
  demo/demo-app/public/wasm/roko_wasm_bg.wasm
```

### Expected bundle size

| Component | Estimated size (optimized) |
|---|---|
| Baseline WASM overhead | ~50 KB |
| SystemPromptBuilder + templates | ~30 KB |
| WasmFetchPoster + agent loop | ~20 KB |
| serde_json | ~80 KB |
| TOML parser | ~40 KB |
| HDC primitives | ~60 KB |
| **Total estimate** | **~280 KB** |

This is comparable to a medium JS library. Loaded lazily, it won't affect initial
page load. The `.wasm` file compresses well with gzip/brotli (~60% reduction).

### CI integration

Add to CI pipeline:

```yaml
# .github/workflows/wasm.yml
wasm-build:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: wasm32-unknown-unknown
    - run: cargo install wasm-bindgen-cli@0.2.118
    - run: cargo install wasm-opt
    - run: cargo build --target wasm32-unknown-unknown --release -p roko-wasm
    - run: wasm-bindgen --target web --out-dir dist/ target/wasm32-unknown-unknown/release/roko_wasm.wasm
    - run: wasm-opt -Oz -o dist/roko_wasm_bg.wasm dist/roko_wasm_bg.wasm
    - uses: actions/upload-artifact@v4
      with:
        name: roko-wasm
        path: dist/
```

### WASM tests

```bash
# Run browser tests via wasm-pack test (or manually)
wasm-pack test --headless --chrome -p roko-wasm
```

Or without wasm-pack:

```bash
cargo test --target wasm32-unknown-unknown -p roko-wasm
```

---

## 9. Work Breakdown

### Phase 0: Foundation (prerequisite refactoring)

| # | Task | Effort | Files touched |
|---|---|---|---|
| 0.1 | Add `wasm` feature to `roko-core` — gate `tokio::sync`, `parking_lot`, fs modules | 1-2 days | `roko-core/Cargo.toml`, ~5 source files |
| 0.2 | Extract `Playbook`, `Skill`, `SectionEffectivenessRegistry` to `roko-core` (or `roko-types`) | 1 day | `roko-learn/src/`, `roko-core/src/`, `roko-compose/src/` |
| 0.3 | Add `wasm` feature to `roko-compose` — gate tokenizer, enrichment, fs modules | 1 day | `roko-compose/Cargo.toml`, ~8 source files |
| 0.4 | Verify `roko-primitives` compiles to `wasm32-unknown-unknown` | 0.5 day | `roko-primitives/Cargo.toml` |

**Total Phase 0**: ~4 days

### Phase 1: roko-wasm crate (MVP)

| # | Task | Effort | Notes |
|---|---|---|---|
| 1.1 | Create `roko-wasm` crate scaffold | 0.5 day | Cargo.toml, lib.rs, wasm-bindgen setup |
| 1.2 | Implement `WasmFetchPoster` | 1 day | ~150 LOC, web-sys fetch |
| 1.3 | Implement `PromptBuilder` wasm-bindgen exports | 1 day | Wraps SystemPromptBuilder |
| 1.4 | Implement `AgentHandle` (non-streaming) | 1.5 days | Claude + OpenAI providers, conversation loop |
| 1.5 | Implement `parse_plan` / `validate_plan` | 1 day | TOML parsing, DAG construction |
| 1.6 | Build pipeline: cargo build + wasm-bindgen + wasm-opt | 0.5 day | Script + CI |
| 1.7 | Browser tests (wasm-bindgen-test) | 1 day | Test prompt build, plan parse, mock fetch |

**Total Phase 1**: ~6.5 days

### Phase 2: Demo app integration

| # | Task | Effort | Notes |
|---|---|---|---|
| 2.1 | `useRokoWasm` hook + lazy loader | 0.5 day | Init, error boundary, loading state |
| 2.2 | Vite WASM plugin setup | 0.5 day | vite-plugin-wasm, config |
| 2.3 | `WasmAgentChat` component (Demo A) | 2 days | Chat UI, API key input, model picker, message list |
| 2.4 | `PromptWorkshop` component (Demo C) | 2 days | 9 section editors, live preview, token count |
| 2.5 | `PlanVisualizer` component (Demo D) | 1.5 days | TOML upload, DAG view, reuse WorkflowConstellation |
| 2.6 | Enhanced PrdPipelinePanel (Demo B fallback) | 2 days | WASM-first with server upgrade path |
| 2.7 | Add "Explore" page/tab hosting WASM demos | 0.5 day | Route, navigation |

**Total Phase 2**: ~9 days

### Phase 3: Polish and streaming (optional)

| # | Task | Effort | Notes |
|---|---|---|---|
| 3.1 | Streaming responses via ReadableStream | 2 days | SSE parsing, chunk callbacks |
| 3.2 | IndexedDB substrate for persistence | 2 days | Signal + episode + plan stores |
| 3.3 | Cost estimator (Demo E) | 1 day | Per-task cost breakdown |
| 3.4 | Affect engine integration (roko-daimon) | 1 day | Optional, compile daimon to WASM |
| 3.5 | HDC fingerprint visualization | 1 day | Show vector similarity in plan view |
| 3.6 | Bundle size optimization + profiling | 1 day | wasm-opt flags, tree analysis |

**Total Phase 3**: ~8 days

### Summary

| Phase | Effort | What you get |
|---|---|---|
| Phase 0 (foundation) | ~4 days | Existing crates compile to WASM |
| Phase 1 (roko-wasm) | ~6.5 days | WASM module with prompt builder + LLM calls + plan parsing |
| Phase 2 (demo integration) | ~9 days | 4 new demo components in the React app |
| Phase 3 (polish) | ~8 days | Streaming, persistence, cost estimation |
| **Total** | **~27.5 days** | |

MVP (Phase 0 + 1 + Demo A from Phase 2) is achievable in **~13 days**.

---

## 10. Risks and Mitigations

### R1: CORS on LLM APIs

**Risk**: Some LLM providers may block browser-origin requests.

**Status**: Claude API (`api.anthropic.com`) allows CORS. OpenAI allows CORS.
Gemini allows CORS. Ollama (local) allows CORS by default.

**Mitigation**: The proxy mode (through roko-serve) is the fallback for any
provider that blocks direct browser access.

### R2: API key exposure in browser

**Risk**: API keys are visible in browser dev tools / memory.

**Mitigation**:
- Direct mode is for local dev only — key never leaves the machine
- Proxy mode for hosted demos — key stays on server
- Clear warning in UI: "Your API key is stored in browser memory only"
- Never persist keys to localStorage/IndexedDB

### R3: tokio Send/Sync bounds

**Risk**: `HttpPoster` requires `Send + Sync`. WASM types are `!Send`.

**Mitigation**: `unsafe impl Send + Sync for WasmFetchPoster` is sound because
WASM is single-threaded. Document this clearly. Long-term, the trait could be
split with a `#[cfg(target_arch = "wasm32")]` variant.

### R4: Bundle size creep

**Risk**: Pulling in too many Rust dependencies inflates the WASM binary.

**Mitigation**:
- Aggressive feature-gating (only compile what's needed)
- `opt-level = "z"`, `lto = true`, `panic = "abort"`, `strip = true`
- `wasm-opt -Oz` post-processing
- Track bundle size in CI, alert on >500 KB
- Use `twiggy` for size profiling

### R5: Transitive dependency hell

**Risk**: A single rogue `dep:tokio` in the dependency tree breaks the WASM build.

**Mitigation**:
- The `roko-wasm` crate should depend on as few workspace crates as possible
- Use `cargo tree -p roko-wasm --target wasm32-unknown-unknown` to audit
- Add a CI job that verifies the WASM build on every PR

### R6: tiktoken-rs WASM incompatibility

**Risk**: `tiktoken-rs` depends on `rayon` which doesn't compile to WASM.

**Mitigation**: Already solved — the `Heuristic` token counter
(`text.len() / 4`) is used in WASM mode. Alternatively, call a JS tokenizer
(like `tiktoken` npm package) from WASM via `js_sys`.

### R7: Feature flag maintenance burden

**Risk**: Having `wasm` features across multiple crates increases maintenance.

**Mitigation**: Keep it to 3 crates max (`roko-core`, `roko-compose`, `roko-wasm`).
The WASM CI build catches regressions. Feature flags are additive (default
features are unchanged).

---

## 11. Ecosystem Reference

### Crate versions (current as of 2026-04)

| Crate | Version | Notes |
|---|---|---|
| `wasm-bindgen` | 0.2.118 | Actively maintained under new org |
| `wasm-bindgen-futures` | 0.4.x | Standard async bridge |
| `web-sys` | 0.3.x | Feature-gated browser API bindings |
| `js-sys` | 0.3.x | JS built-in bindings |
| `serde-wasm-bindgen` | 0.6.x | Efficient Rust↔JS type conversion |
| `console_error_panic_hook` | 0.1.x | Better panic messages in browser console |
| `getrandom` | 0.4.2 | Feature: `wasm_js` (renamed from `js` in 0.3→0.4) |
| `reqwest` (on WASM) | 0.12.x | Works, no `blocking`, uses browser fetch internally |
| `indexed-db` | latest | Thread-safe IndexedDB wrapper |
| `vite-plugin-wasm` | latest | Vite WASM support |
| `vite-plugin-top-level-await` | latest | Required for WASM init |

### Tools

| Tool | Replaces | Notes |
|---|---|---|
| `cargo build --target wasm32-unknown-unknown` | — | Standard Rust cross-compilation |
| `wasm-bindgen-cli` | wasm-pack (archived) | Version must match crate exactly |
| `wasm-opt` | — | 10-30% size reduction |
| `twiggy` | — | WASM binary size profiler |
| `Trunk` | wasm-pack | Full build tool, optional (we use Vite) |

### Key constraints

- **No tokio**: Use `wasm-bindgen-futures::spawn_local` for async
- **No std::fs**: Use IndexedDB or in-memory structures
- **No std::process**: No subprocess spawning
- **No std::net**: Use browser fetch/WebSocket via web-sys
- **No threads**: Single-threaded (unless using wasm-bindgen-rayon + nightly + COOP/COEP)
- **No native TLS**: Browser handles TLS transparently

---

## Appendix: Decision Log

| Decision | Choice | Rationale |
|---|---|---|
| WASM target | `wasm32-unknown-unknown` | Browser target; WASI is server-only |
| Build tool | `cargo build` + `wasm-bindgen-cli` + `wasm-opt` | wasm-pack archived; this is the modern approach |
| Async runtime | `wasm-bindgen-futures` | Standard; `tokio_with_wasm` adds unnecessary complexity |
| Fetch impl | Custom `WasmFetchPoster` via `web_sys` | Matches existing `HttpPoster` trait; no extra deps |
| Token counting | Heuristic (`len/4`) in WASM | `tiktoken-rs` uses `rayon` (WASM-hostile) |
| Type extraction | Move to `roko-core` | Simpler than a new `roko-types` crate |
| JS interop style | `serde-wasm-bindgen` for complex types | Avoids JSON round-trip overhead |
| Streaming | Phase 3 (optional) | Non-streaming MVP is simpler and covers core demos |
| Standalone mode | Build flag + env vars | Same React app, different entry point; no new repo |
| Knowledge persistence | IndexedDB | JSON-serializable data model maps 1:1 |
| Dream cycle trigger | User-initiated or timer | No cron in browser; explicit "consolidate" button |

---

## 12. Standalone Deployment Mode

### The idea

A deployable version of the dashboard that runs entirely in the browser — no `roko serve`,
no Rust backend, no terminal. Deploy to Vercel/Netlify/Cloudflare Pages with a few API key
env vars and it just works.

This is **not** a stripped-down version. It's the full dashboard with a different data
source: instead of REST calls to `localhost:6677`, the WASM module handles agent dispatch,
knowledge storage (IndexedDB), prompt composition, and plan parsing directly.

### How it works

```
┌────────────────────────────────────────────────────────────┐
│                Static hosting (Vercel / Netlify / CF Pages) │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  React app (same codebase, standalone mode)          │  │
│  │                                                      │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │  │
│  │  │ Dashboard    │  │ Knowledge   │  │ Agent Chat  │  │  │
│  │  │ (Cost,Fleet, │  │ (Graph,     │  │ (Multi-     │  │  │
│  │  │  Routing)    │  │  Entries,   │  │  model,     │  │  │
│  │  │             │  │  Dreams)    │  │  streaming) │  │  │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │  │
│  │         │                │                │         │  │
│  │  ┌──────▼────────────────▼────────────────▼──────┐  │  │
│  │  │           roko-wasm (WASM module)              │  │  │
│  │  │                                                │  │  │
│  │  │  WasmFetchPoster    → browser fetch to LLM APIs│  │  │
│  │  │  SystemPromptBuilder → 9-layer prompt assembly │  │  │
│  │  │  KnowledgeStore     → IndexedDB-backed         │  │  │
│  │  │  DreamCycle (partial)→ distill, tier, replay   │  │  │
│  │  │  EpisodeLogger      → IndexedDB-backed         │  │  │
│  │  │  CascadeRouter      → model selection logic    │  │  │
│  │  │  PlanParser         → TOML → DAG               │  │  │
│  │  └───────────────────────┬────────────────────────┘  │  │
│  │                          │                           │  │
│  └──────────────────────────┼───────────────────────────┘  │
│                             │                               │
└─────────────────────────────┼───────────────────────────────┘
                              │
                    ┌─────────▼─────────┐
                    │  LLM APIs (CORS)  │
                    │  Claude / OpenAI  │
                    │  Gemini / etc.    │
                    └───────────────────┘
```

### Startup flag / build mode

The same React app serves both modes. A build-time flag controls the data source:

```bash
# Full mode (default) — connects to roko-serve at :6677
VITE_MODE=full npm run build

# Standalone mode — WASM-only, no backend
VITE_MODE=standalone \
  VITE_ANTHROPIC_KEY=sk-ant-... \
  VITE_OPENAI_KEY=sk-... \
  npm run build
```

In code, a single provider swap:

```typescript
// src/lib/data-provider.ts

export type DataMode = 'full' | 'standalone';
export const MODE: DataMode = (import.meta.env.VITE_MODE || 'full') as DataMode;

// Every hook checks this:
export function useKnowledgeEntries() {
  if (MODE === 'standalone') {
    return useWasmKnowledge();   // IndexedDB via WASM
  }
  return useApiKnowledge();      // fetch to /api/knowledge/entries
}
```

### Environment variables

| Var | Required | What |
|---|---|---|
| `VITE_MODE` | Yes | `full` or `standalone` |
| `VITE_ANTHROPIC_KEY` | For Claude agents | Anthropic API key |
| `VITE_OPENAI_KEY` | For OpenAI agents | OpenAI API key |
| `VITE_GEMINI_KEY` | For Gemini agents | Google AI API key |
| `VITE_DEFAULT_MODEL` | No | Default model slug (e.g. `claude-sonnet-4-5`) |
| `VITE_DEFAULT_PROVIDER` | No | Default provider (e.g. `anthropic`) |

**Security note**: API keys are embedded at build time into the JS bundle. This is
appropriate for:
- Personal/team deploys behind auth (Vercel password protection, Cloudflare Access)
- Internal tooling
- Local development

For public-facing deploys, use a runtime prompt (user pastes their own key) or add a
thin API proxy (Cloudflare Worker / Vercel Edge Function).

### Deployment targets

```bash
# Vercel
vercel env add VITE_MODE standalone
vercel env add VITE_ANTHROPIC_KEY sk-ant-...
vercel deploy

# Netlify
netlify env:set VITE_MODE standalone
netlify env:set VITE_ANTHROPIC_KEY sk-ant-...
netlify deploy --prod

# Cloudflare Pages
# Set env vars in dashboard, then:
npx wrangler pages deploy dist/

# GitHub Pages
# Set secrets in repo settings, use in GitHub Actions build step

# Docker (static nginx)
docker build --build-arg VITE_MODE=standalone \
             --build-arg VITE_ANTHROPIC_KEY=sk-ant-... \
             -t roko-dashboard .
```

### What the standalone build includes

The standalone build bundles:
- The React app (~500 KB gzipped, current)
- The WASM module (~280 KB gzipped, estimated)
- Three.js for WorkflowConstellation (~200 KB gzipped, current)
- xterm.js excluded (no terminal in standalone mode)
- **Total: ~980 KB gzipped** — comparable to a medium SPA

---

## 13. Knowledge Workflows in the Browser

The knowledge system is the strongest case for standalone WASM. The data model is pure
JSON, the store logic is arithmetic on flat structs, and the LLM calls are standard
HTTP POSTs.

### What the data model looks like in IndexedDB

```
IndexedDB "roko-standalone"
├── knowledge          (KnowledgeEntry[])  → keyPath: "id"
│   indexes: kind, tier, confidence, created_at, tags (multiEntry)
├── confirmations      (KnowledgeConfirmationRecord[])
├── episodes           (Episode[])  → keyPath: "id"
│   indexes: plan_id, task_id, created_at, kind
├── staging            (StagingEntry[])  → keyPath: "id"
│   indexes: stage, created_at
├── playbooks          (Playbook[])  → keyPath: "id"
│   indexes: domain, success_rate
├── efficiency         (EfficiencyEvent[])  → keyPath: auto-increment
│   indexes: timestamp, model, cost
├── cascade_router     (single doc)  → keyPath: "singleton"
├── experiments        (single doc)  → keyPath: "singleton"
└── settings           (single doc)  → keyPath: "singleton"
```

Every type is already `#[derive(Serialize, Deserialize)]` in the Rust source. The
IndexedDB schema mirrors the JSONL files 1:1.

### Knowledge operations ported to browser

| Operation | Backend version | Browser version | LLM needed? |
|---|---|---|---|
| **Query** | Scan JSONL, score by keyword+recency+HDC | Scan IndexedDB, same scoring math | No |
| **Ingest** | Append to JSONL, dedup by tag overlap | Put to IndexedDB, same dedup | No |
| **Decay** | Rewrite JSONL with decayed confidences | Update-in-place in IndexedDB | No |
| **GC** | Filter JSONL, remove below threshold | Delete from IndexedDB by confidence index | No |
| **Stats** | Aggregate over JSONL | Aggregate over IndexedDB cursor | No |
| **Tier progression (D1)** | Pattern mine episodes → insights | Same math over IndexedDB episodes | No |
| **Tier progression (D2)** | Promote insights with 5+ episodes → heuristics | Same | No |
| **Tier progression (D3)** | High-confidence heuristics → playbook | Same | No |
| **Distillation** | LLM call (Claude Haiku) over episode batch | Same LLM call via WasmFetchPoster | **Yes** |
| **Dream replay selection** | Mattar-Daw scoring, episode selection | Same math | No |
| **Staging buffer** | Confidence ladder, GC after 7 days | Same logic, browser timer | No |
| **Strategy fragments** | Query by kind=StrategyFragment | Same IndexedDB query | No |
| **Anti-knowledge** | Query by kind=AntiKnowledge | Same IndexedDB query | No |
| **Routing advice** | Generate from cluster data | Same math | No |

### The dream cycle in the browser

A "Consolidate" button (or periodic timer) triggers a browser-side dream cycle:

```
┌──────────────────────────────────────────────────────┐
│  Dream Cycle (browser)                                │
│                                                       │
│  1. Replay selection (Mattar-Daw)                     │
│     IndexedDB episodes → score → select top-N        │
│     [pure math, no LLM]                               │
│                                                       │
│  2. Distillation                                      │
│     Selected episodes → WasmFetchPoster → Claude API  │
│     → parse structured response → KnowledgeEntry[]   │
│     [1 LLM call per batch, ~$0.01-0.05]              │
│                                                       │
│  3. Staging                                           │
│     New entries → staging buffer (confidence=0.20)    │
│     [pure logic]                                      │
│                                                       │
│  4. Tier progression (D1/D2/D3)                       │
│     Pattern mine confirmed entries                    │
│     Promote to Working/Consolidated/Persistent tiers  │
│     [pure math]                                       │
│                                                       │
│  5. Write results                                     │
│     → IndexedDB knowledge store                      │
│     → IndexedDB playbook store                       │
│     → Update dashboard state                          │
│                                                       │
│  Total: ~2-5 seconds + 1 LLM round-trip              │
└──────────────────────────────────────────────────────┘
```

### What knowledge workflows look like for the user

**Workflow 1: Research → Distill → Knowledge Graph**

User opens the standalone dashboard. Navigates to Agent Chat. Starts a research
conversation with Claude: "Explain the tradeoffs between WebSocket and SSE for
real-time updates." The conversation runs (3-4 turns). Each turn is logged as an
episode in IndexedDB.

User clicks "Consolidate knowledge." The dream cycle runs:
- Selects the research conversation episodes
- Calls Claude Haiku to extract insights ("SSE is simpler for unidirectional...",
  "WebSocket enables bidirectional but adds connection management overhead...")
- New `KnowledgeEntry` items appear in the Knowledge Graph tab
- Entries start at `Transient` tier with `confidence: 0.20`

User does more research on the same topic. Subsequent distillation finds overlapping
entries, increments `confirmation_count`, and confidence rises. After 5+
confirmations, entries auto-promote to `Working` tier.

**Workflow 2: Multi-Agent Knowledge Transfer**

User creates Agent A (Claude Sonnet, domain: "frontend") and Agent B (GPT-4o,
domain: "backend"). Each agent has conversations that produce episodes. Distillation
runs per-agent. The knowledge store merges insights from both, tracking
`source_model` and `model_generality`.

Agent A's conversation: "What's the best React state management for real-time data?"
Agent B's conversation: "Design a WebSocket server for streaming updates."

After consolidation, the knowledge graph shows cross-domain links: Agent A's
"use React Query for server state" connects to Agent B's "WebSocket message
format should be JSON with type discriminators."

**Workflow 3: Knowledge-Informed Prompt Engineering**

User opens Prompt Workshop. The 9-layer builder pulls `relevant_skills` and
`relevant_playbooks` from IndexedDB. The system prompt now includes learned
knowledge from previous conversations. The user can see which knowledge entries
influenced the prompt and toggle them on/off.

**Workflow 4: Export/Import Knowledge**

User downloads their IndexedDB knowledge store as JSON. Shares it with a teammate.
Teammate imports it into their standalone dashboard. The import applies a
confidence discount (0.7x by default — same as `ingest_with_source` in the Rust
code) since the knowledge wasn't generated locally.

---

## 14. Standalone Dashboard: Page-by-Page

Which pages work, which change, which are removed.

### Pages that work as-is

These already have demo data fallbacks and don't need the backend:

| Page | Route | What changes in standalone mode |
|---|---|---|
| **Landing** | `/` | Nothing — already falls back to demo metrics |
| **Cost Dashboard** | `/dashboard` | Metrics come from IndexedDB instead of `/api/learn/efficiency` |
| **Fleet** | `/dashboard/fleet` | Agent cards reflect WASM agents created in-session |
| **Knowledge Graph** | `/dashboard/knowledge` | Live data from IndexedDB knowledge store (not demo data) |
| **Knowledge Entries** | `/dashboard/entries` | Live data from IndexedDB |
| **Cascade Router** | `/dashboard/routing` | CascadeRouter state from WASM, persisted to IndexedDB |
| **Chain** | `/dashboard/chain` | No change — already a Phase 2 teaser |
| **Explorer** | `/explorer` | Episodes from IndexedDB; health tab shows WASM module info |
| **Bench Showroom** | `/bench/showroom` | No change — already static demo data |

### Pages that are enhanced with WASM

| Page | Route | What's new |
|---|---|---|
| **Demo** | `/demo` | New scenarios that don't need PTY: "Knowledge Distillation", "Prompt Workshop", "Plan Visualizer". PTY scenarios show "requires roko-serve" badge |
| **Bench** | `/bench` | "Configure" tab can run benchmarks via WASM agents (same model, same prompt, different providers) — cost/quality comparison without a backend |

### Pages that are removed or gated

| Page | Route | Why |
|---|---|---|
| **Builder** | `/builder` | Requires PTY. Hidden in standalone mode. |
| **Terminal** | `/terminal` | Requires PTY. Hidden in standalone mode. |
| **Share** | `/share/:token` | Requires server state. Hidden in standalone mode. |
| **Dashboard Share** | `/dashboard/share/:token` | Same. Hidden. |

### New pages for standalone mode

| Page | Route | What |
|---|---|---|
| **Agent Chat** | `/chat` | Multi-model chat. Pick provider/model, paste key or use env var. 9-layer system prompt assembled via WASM. Conversations logged as episodes to IndexedDB. |
| **Prompt Workshop** | `/workshop` | Interactive prompt builder. 9 layers with live editing. See assembled prompt, send it. |
| **Knowledge Console** | `/knowledge` | Unified knowledge management: query, browse, consolidate (dream cycle), import/export, stats, decay visualization. |
| **Plan Studio** | `/plans` | Upload/paste `tasks.toml`, see DAG, simulate execution, estimate cost per task. |

### Route gating

```typescript
// src/main.tsx
import { MODE } from './lib/data-provider';

const routes = [
  // Always present
  { path: '/', element: <Landing /> },
  { path: '/dashboard', element: <DashboardLayout />, children: [...] },
  { path: '/explorer', element: <Explorer /> },
  { path: '/demo', element: <Demo /> },

  // Standalone-only
  ...(MODE === 'standalone' ? [
    { path: '/chat', element: <AgentChat /> },
    { path: '/workshop', element: <PromptWorkshop /> },
    { path: '/knowledge', element: <KnowledgeConsole /> },
    { path: '/plans', element: <PlanStudio /> },
  ] : []),

  // Full-only (need server)
  ...(MODE === 'full' ? [
    { path: '/builder', element: <Builder /> },
    { path: '/terminal', element: <Terminal /> },
    { path: '/share/:token', element: <Share /> },
    { path: '/bench', element: <Bench /> },
  ] : []),

  // Both modes, bench differs
  ...(MODE === 'standalone' ? [
    { path: '/bench', element: <BenchStandalone /> },
  ] : []),
];
```

---

## 15. Use Cases for the Standalone Deploy

### UC1: "Team Knowledge Base"

Deploy to Vercel behind Vercel password protection. Team members access the dashboard,
each with their own browser-local knowledge store. They can:

- Chat with Claude/GPT to research topics
- Distill conversations into knowledge entries
- Browse the knowledge graph to see what the team has learned
- Export/import knowledge between team members
- The knowledge graph grows organically from real conversations

**Why this is interesting**: It's a knowledge management tool that builds itself from
your actual research conversations. No manual curation — the dream cycle handles
distillation and tier progression automatically.

### UC2: "Prompt Engineering Workbench"

A deployed tool for prompt engineers to experiment with roko's 9-layer composition
system. They can:

- Configure each layer independently
- See token budgets and how layers compete for space
- A/B test different system prompts against the same user message
- Track which prompt configurations produce better results (via model routing feedback)
- Share prompt configurations as JSON exports

**Why this is interesting**: Makes roko's prompt engineering framework accessible to
anyone, not just people who can run the Rust CLI.

### UC3: "Agent Benchmark Suite"

Deploy the bench system with WASM agents. Configure runs against multiple models,
track cost/quality/latency per provider. No infrastructure needed — the user's
browser does all the work, calling LLM APIs directly.

- Run the same task through Claude Sonnet, GPT-4o, Gemini Pro
- See Pareto frontier (cost vs quality)
- CascadeRouter learns from results, suggests optimal model routing
- Export results as JSON for team review

### UC4: "Self-Improving Research Assistant"

The standalone dashboard as a long-lived research companion:

- Day 1: Chat with Claude about a topic. Knowledge store is empty.
- Day 5: After several conversations, run consolidation. Knowledge graph populates.
- Day 10: New conversations benefit from stored knowledge (appears in system prompt
  via `relevant_skills` and `relevant_playbooks` layers).
- Day 30: Tier progression has promoted reliable insights to `Consolidated`.
  The assistant's context is now enriched with 30 days of distilled knowledge.
- The affect engine (daimon) modulates agent behavior based on recent success/failure
  patterns — cautious after failures, exploratory when things are going well.

**Why this is interesting**: Demonstrates roko's learning loop without any infrastructure.
The browser IS the agent runtime. Knowledge accumulates across sessions via IndexedDB.

### UC5: "Investor / Conference Demo"

A URL you can give to anyone. They open it, see the dashboard with live-looking data,
and can actually interact with it:

- Chat with agents (using a prepaid API key set at deploy time)
- See the knowledge graph grow in real-time as they converse
- Watch the dream cycle distill their conversation into structured knowledge
- Explore plans and DAGs
- All without installing anything or running any backend

**Why this is interesting**: The demo IS the product. Not a recording, not a mock — the
actual agent coordination system running in their browser.

### UC6: "Offline-First Agent Notebook"

A version that works without internet after initial load:

- WASM module cached by service worker
- Previous conversations and knowledge in IndexedDB
- Plan parsing and prompt composition work fully offline
- LLM calls queue until network returns, or use local Ollama

Deploy with a service worker for PWA capability:

```bash
VITE_MODE=standalone VITE_PWA=true npm run build
```

---

## Additional Work for Standalone Mode

Beyond the roko-wasm crate work (Phases 0-3 from section 9), standalone mode needs:

### Phase S1: Data provider layer

| # | Task | Effort | Notes |
|---|---|---|---|
| S1.1 | Create `data-provider.ts` with `MODE` switch | 0.5 day | Single import, every hook checks it |
| S1.2 | `useWasmKnowledge` hook (IndexedDB-backed) | 1 day | CRUD + query + decay |
| S1.3 | `useWasmEpisodes` hook (IndexedDB-backed) | 0.5 day | Append + query + filter |
| S1.4 | `useWasmRouter` hook (cascade router state) | 0.5 day | Model selection + learning |
| S1.5 | `useWasmEfficiency` hook (cost tracking) | 0.5 day | Per-turn cost aggregation |
| S1.6 | Update all existing hooks to check `MODE` | 1 day | ~12 hooks to add fallback path |

**Total Phase S1**: ~4 days

### Phase S2: Knowledge console + dream cycle

| # | Task | Effort | Notes |
|---|---|---|---|
| S2.1 | `KnowledgeConsole` page | 2 days | Query, browse, stats, decay viz |
| S2.2 | Dream cycle WASM impl (distill + tier progression) | 2.5 days | Port DreamCycle core logic |
| S2.3 | "Consolidate" UI (progress, results, staging view) | 1 day | Show what the cycle found |
| S2.4 | Knowledge export/import | 0.5 day | JSON download/upload |
| S2.5 | Knowledge graph with live IndexedDB data | 1 day | Replace demo fallback with real data |

**Total Phase S2**: ~7 days

### Phase S3: New standalone pages

| # | Task | Effort | Notes |
|---|---|---|---|
| S3.1 | `AgentChat` page | 2 days | Multi-model, episode logging |
| S3.2 | `PromptWorkshop` page | 2 days | 9 layers, live preview |
| S3.3 | `PlanStudio` page | 1.5 days | Upload, DAG view, cost estimate |
| S3.4 | `BenchStandalone` page | 2 days | WASM agents, multi-model comparison |
| S3.5 | Route gating + navigation updates | 0.5 day | Mode-aware routing |
| S3.6 | Deploy pipeline (Vercel/Netlify configs) | 0.5 day | Build scripts, env vars |

**Total Phase S3**: ~8.5 days

### Combined work breakdown (all phases)

| Phase | What | Days |
|---|---|---|
| Phase 0: Foundation refactoring | Feature flags in roko-core, roko-compose | 4 |
| Phase 1: roko-wasm crate | WASM module with prompt builder + LLM calls + plan parsing | 6.5 |
| Phase 2: Demo integration | WASM demos in existing demo app | 9 |
| Phase 3: Polish | Streaming, IndexedDB persistence, cost estimation | 8 |
| Phase S1: Data provider | Mode switch, WASM-backed hooks | 4 |
| Phase S2: Knowledge console | Dream cycle, knowledge management UI | 7 |
| Phase S3: Standalone pages | Chat, workshop, plan studio, bench | 8.5 |
| **Total** | | **~47 days** |

**MVP paths:**

| MVP | Phases | Days | What you get |
|---|---|---|---|
| WASM demo only | 0 + 1 + partial 2 | ~13 | Agent chat + prompt builder in existing demo app |
| Standalone dashboard | 0 + 1 + 3 + S1 + S3.1 | ~23 | Deployable dashboard with agent chat + knowledge |
| Full standalone | All | ~47 | Complete standalone product with knowledge workflows + dreams |
