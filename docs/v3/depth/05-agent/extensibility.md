# 05 -- Extensibility

> **Implementation status (2026-09):** All five extensibility points
> are live. The config-only path (adding an OpenAI-compatible provider)
> requires zero code changes. The four-layer SDK surface is stable.

---

## Extensibility Points

Roko's agent system has five extensibility points, each with a clear
trait or registration mechanism:

| Extension point | Trait/Interface | Location | Effort |
|---|---|---|---|
| New provider (config-only) | TOML entries | `roko.toml` | None |
| New protocol family | `ProviderAdapter` | `roko-agent/src/provider/` | Low |
| New tool translator | `Translator` | `roko-agent/src/translate/` | Medium |
| New LLM backend | `LlmBackend` | `roko-agent/src/tool_loop/` | Low |
| New tool handler | `ToolHandler` | `roko-core/src/tool/` | Low |

---

## Adding a Provider (Config-Only Path)

The simplest extension. If the provider speaks an existing protocol
(most likely OpenAI-compatible chat completions), no code is needed:

### Step 1: Add provider entry in `roko.toml`

```toml
[providers.my-provider]
kind = "openai_compat"
base_url = "https://api.my-provider.com/v1"
api_key_env = "MY_PROVIDER_API_KEY"
timeout_ms = 60000
```

### Step 2: Add model entries

```toml
[models.my-model-large]
provider = "my-provider"
slug = "my-model-large"
context_window = 128000
max_output = 4096
supports_tools = true
tool_format = "openai_json"
cost_input_per_m = 2.00
cost_output_per_m = 8.00
```

### Step 3: Use it

```bash
cargo run -p roko-cli -- run "Hello" --model my-model-large
```

The `create_agent_for_model` factory resolves the model, finds the
provider, sees `kind = "openai_compat"`, and uses the
`OpenAiCompatAdapter` to construct an agent. No code changes needed.

---

## Adding a New Protocol Family (ProviderAdapter)

If the provider uses a protocol that does not fit any existing adapter,
you need a new `ProviderAdapter` implementation:

### Step 1: Add a ProviderKind variant

In `crates/roko-core/src/agent.rs`:

```rust
pub enum ProviderKind {
    AnthropicApi,
    ClaudeCli,
    OpenAiCompat,
    // ... existing variants ...
    MyProtocol,  // NEW
}
```

### Step 2: Implement ProviderAdapter

In `crates/roko-agent/src/provider/my_protocol.rs`:

```rust
pub struct MyProtocolAdapter;

impl ProviderAdapter for MyProtocolAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::MyProtocol
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        let base_url = provider.base_url.as_deref()
            .ok_or_else(|| AgentCreationError::MissingConfig(
                "base_url".into()
            ))?;
        let api_key = provider.resolve_api_key()
            .ok_or_else(|| AgentCreationError::MissingApiKey(
                provider.api_key_env.clone().unwrap_or_default()
            ))?;

        Ok(Box::new(MyProtocolAgent::new(
            base_url, &api_key, &model.slug
        )))
    }

    fn classify_error(
        &self, status: u16, body: &Value
    ) -> ProviderError {
        match status {
            429 => ProviderError::RateLimit {
                retry_after_ms: None
            },
            401 | 403 => ProviderError::AuthFailure,
            500..=599 => ProviderError::ServerError(status),
            _ => ProviderError::Other(format!("status {status}")),
        }
    }
}
```

### Step 3: Register in adapter_for_kind

In `crates/roko-agent/src/provider/mod.rs`:

```rust
static MY_PROTOCOL_ADAPTER: MyProtocolAdapter = MyProtocolAdapter;

pub fn adapter_for_kind(kind: ProviderKind)
    -> &'static dyn ProviderAdapter
{
    match kind {
        ProviderKind::AnthropicApi => &ANTHROPIC_API_ADAPTER,
        ProviderKind::ClaudeCli    => &CLAUDE_CLI_ADAPTER,
        ProviderKind::OpenAiCompat => &OPENAI_COMPAT_ADAPTER,
        // ... existing arms ...
        ProviderKind::MyProtocol   => &MY_PROTOCOL_ADAPTER,
    }
}
```

The exhaustive `match` ensures the compiler catches any unregistered
variant.

---

## Adding a New LlmBackend

If your provider supports tool calling and you want to use Roko's
ToolLoop (rather than the provider's internal loop), implement
`LlmBackend`:

```rust
pub struct MyBackend {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[async_trait]
impl LlmBackend for MyBackend {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
    ) -> Result<BackendResponse, LlmError> {
        let body = build_request_body(
            &self.model, messages, tools
        );
        let response = self.client
            .post(&format!("{}/chat", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send().await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        let json: Value = response.json().await
            .map_err(|e| LlmError::Backend(e.to_string()))?;

        Ok(BackendResponse::Json(json))
    }
}
```

Then wire it into the ToolLoop:

```rust
let backend = Arc::new(MyBackend { ... });
let translator = Arc::new(OpenAiTranslator);
let dispatcher = Arc::new(ToolDispatcher::new(registry, resolver));
let tool_loop = ToolLoop::new(translator, dispatcher, backend);

let output = tool_loop.run(
    system_prompt, user_prompt, &tools, &ctx
).await;
```

The existing `OllamaLlmBackend` at
`crates/roko-agent/src/ollama_backend.rs` is a working reference
implementation.

---

## Adding a New Translator

If a model uses a wire format not covered by the six existing
translators (OpenAI, Ollama, Claude, Gemini, Hermes, ReAct):

```rust
pub struct MyFormatTranslator;

impl Translator for MyFormatTranslator {
    fn format(&self) -> ToolFormat {
        ToolFormat::MyFormat  // Add to the ToolFormat enum first
    }

    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools {
        let json = tools.iter().map(|t| {
            json!({
                "tool_name": t.name,
                "tool_desc": t.description,
                "params": t.schema,
            })
        }).collect::<Vec<_>>();
        RenderedTools::JsonArray(json!(json))
    }

    fn parse_calls(
        &self, response: &BackendResponse
    ) -> Result<Vec<ToolCall>, TranslatorError> {
        let BackendResponse::Json(ref v) = *response else {
            return Ok(vec![]);
        };
        // ... parse your format ...
        Ok(calls)
    }

    fn render_results(
        &self, results: &[(ToolCall, ToolResult)]
    ) -> RenderedResults {
        RenderedResults::JsonMessages(json!([...]))
    }
}
```

Then register it in `translator_for` in `translate/capability.rs`.

---

## Adding a New Tool Handler

Tools are registered through the `ToolRegistry` and resolved through
`HandlerResolver`. To add a new tool:

### Step 1: Define the tool

```rust
pub fn my_tool_def() -> ToolDef {
    ToolDef::new("my_tool")
        .description("Does something useful")
        .add_param("input", "string", "The input", true)
        .build()
}
```

### Step 2: Implement the handler

```rust
pub struct MyToolHandler;

#[async_trait]
impl ToolHandler for MyToolHandler {
    async fn execute(
        &self,
        call: &ToolCall,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input = call.args.get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingParam(
                "input".into()
            ))?;

        Ok(ToolResult::text(format!("Result: {input}")))
    }
}
```

### Step 3: Register in the resolver

Add the tool definition to the registry and the handler to the
resolver, typically in `roko-std/src/tool/`.

---

## Four-Layer Rust SDK

The SDK is intentionally layered so developers can stop at the highest
level that fits their task:

| Layer | User | Entry point | What they own |
|---|---|---|---|
| One-liner | Application author | `roko::run(...)` | Defaults, model selection |
| Builder | Agent author | `Agent::builder()` | Roles, tools, gates, prompts |
| Trait impl | Trait implementor | `ProviderAdapter`, `Translator`, etc. | Narrow, stable contracts |
| Runtime impl | Runtime implementor | Runtime/supervisor wiring | Host process, transport |

### Guidance

- Application authors should get a working agent in under a minute.
- Agent authors should stay on the builder surface unless they are
  replacing a kernel contract.
- Trait implementors should keep dependencies narrow.
- Runtime implementors should wire execution hosts directly.

---

## 8-Step Domain Plugin Process

For adding a new domain-specific agent type (see also
`depth/05-agent/domain-profiles.md`):

1. **Define the role** -- add a variant to `AgentRole` with default
   tier, budget, and permissions.
2. **Create the role template** -- write a system prompt template in
   `roko-compose/src/templates/`.
3. **Register tools** -- define domain-specific `ToolDef` entries and
   `ToolHandler` implementations.
4. **Configure the model** -- add `[models.*]` entries for models suited
   to the domain.
5. **Wire the provider** -- ensure the provider config exists.
6. **Set gate criteria** -- define domain-specific gate checks.
7. **Add to the router** -- register the role's default tier.
8. **Test end-to-end** -- run `roko run "<domain prompt>"` and verify.

---

## Self-Evolving Agent Architecture

Beyond static extensibility, Roko supports **self-evolution**:

### Darwin Godel Machine Pattern

The Darwin Godel Machine (Sakana AI; arXiv:2505.22954, 2025) iteratively
modifies its own code and empirically validates each change using
benchmarks. SWE-bench improved from 20.0% to 50.0% (2.5x improvement).

**Mapping to Roko:** Roko already has the infrastructure for
self-modification (PRD, plan, execute, gate, persist). The DGM
pattern adds an evolutionary archive -- maintaining a population of
agent configurations and selecting for fitness.

### Voyager-Style Skill Library

The EpisodeLogger + playbook system in `roko-learn` already captures
execution traces. Skill extraction would identify reusable patterns
from successful episodes and store them as composable skills with
semantic descriptions for retrieval (Wang et al., 2023;
arXiv:2305.16291).

### Intrinsic vs. Extrinsic Metacognition

Roko's learning layer (efficiency events, cascade router, experiments,
adaptive thresholds) is extrinsic metacognition. The path to intrinsic
metacognition would require the system to modify its own learning
mechanisms (Liu & van der Schaar, 2025; arXiv:2506.05109).

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-agent/src/provider/mod.rs` | ProviderAdapter trait, adapter_for_kind, create_agent_for_model |
| `crates/roko-agent/src/translate/mod.rs` | Translator trait |
| `crates/roko-agent/src/tool_loop/mod.rs` | LlmBackend trait |
| `crates/roko-core/src/tool/` | ToolHandler trait, ToolDef, ToolRegistry |
| `crates/roko-agent/src/ollama_backend.rs` | Reference LlmBackend implementation |

---

## Citations

1. Zhang, J. et al. (2025). "Darwin Godel Machine." arXiv:2505.22954.
   -- SWE-bench 20% to 50%.
2. Wang, G. et al. (2023). "Voyager." arXiv:2305.16291.
3. Liu, T. & van der Schaar, M. (2025). "Truly Self-Improving Agents
   Require Intrinsic Metacognitive Learning." arXiv:2506.05109.
4. `crates/roko-agent/src/provider/mod.rs` -- ProviderAdapter trait.
5. `crates/roko-agent/src/tool_loop/mod.rs` -- LlmBackend trait.
6. `crates/roko-agent/src/translate/mod.rs` -- Translator trait.
