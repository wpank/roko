# 15 — Operational Surface: CLI Commands, Testing, Validation, Dashboard, Logging

> **Priority**: 🟡 P1 — Needed for developers to debug, operate, and build on the provider system
> **Status**: Not started
> **Depends on**: 02 (provider registry), 03 (adapters)
> **Blocks**: None (can be done in parallel with model integrations)

> **Cross-references**:
> - Doc 16 (2N.11–2N.16) adds HTTP API routes for providers/models — complement, don't duplicate
> - The `/api/learn/cascade` endpoint already exposes CascadeRouter state (routes/learning.rs)
> - The `roko provider test` CLI (2M.03) and `POST /api/providers/{id}/test` (2N.14) do the same
>   thing via different interfaces — share the underlying test logic
> - Config migration (2M.15) should generate the new `[providers.*]` / `[models.*]` sections
>   that docs 02 (2A.04) defines

## Problem Statement

The entire provider/model/routing system has zero CLI surface area. Searching the CLI codebase for `provider`, `model`, `route`, or `backend` returns **zero matches**. There's no way to:

- List configured providers or their health status
- List available models or test a model connection
- See why the router picked a particular model
- Validate a config before running a plan
- Test provider integrations without real API keys
- View routing decisions in the dashboard

The dashboard has 13 pages defined but the `Learning` and `Experiments` pages would need extension for provider health, model comparison, and routing visualization.

## What Exists

| Feature | Status | Location |
|---|---|---|
| `roko config show` | ✓ | `config_cmd.rs` — shows merged config |
| `roko config init` | ✓ | `config_cmd.rs` — interactive wizard |
| `roko config check-secrets` | ✓ | `config_cmd.rs` — validates env var refs |
| `roko status` | ✓ | `status.rs` — signal/episode counts |
| `roko dashboard` | ✓ | `tui/dashboard.rs` — 13 pages, text-only |
| Dashboard `Learning` page | ✓ | `tui/pages/efficiency.rs` — routing feedback |
| Dashboard `Experiments` page | ✓ | `tui/pages/efficiency.rs` — prompt experiments |
| roko-serve `/api/learning` | ✓ | `routes/learning.rs` — routing metrics API |
| e2e test harness | ✓ | `tests/e2e.rs` — 4 test cases |
| Structured logging | ✓ | `tracing` crate throughout |
| **Provider CLI** | ✗ | Not found |
| **Model CLI** | ✗ | Not found |
| **Route debug CLI** | ✗ | Not found |
| **Config validation** | ✗ | Partial (secrets only) |
| **Provider health dashboard** | ✗ | Not found |
| **Mock test infrastructure** | ✗ | No provider mocks |

---

## A. CLI Commands for Provider Management

### 2M.01 — Add `roko provider list` command

**File**: `crates/roko-cli/src/main.rs` (add ProviderCmd to Subcommand enum)
**What**: List all configured providers with connection status:

```
$ roko provider list

Provider     Kind           Base URL                              Status
anthropic    claude_cli     (cli: claude)                         ok (cli found)
zai          openai_compat  https://api.z.ai/api/paas/v4         ok (key set)
moonshot     openai_compat  https://api.moonshot.ai/v1            warn (key missing)
openrouter   openai_compat  https://openrouter.ai/api/v1          ok (key set)
ollama       openai_compat  http://localhost:11434                 warn (unreachable)
```

Checks:
- CLI exists in PATH (for `claude_cli` kind)
- API key env var is set (for HTTP providers)
- Base URL is reachable (quick HEAD request with 2s timeout)

**Acceptance**: `roko provider list` shows all providers from `[providers.*]` with status.
**Verification**: `cargo run -p roko-cli -- provider list`

---

### 2M.02 — Add `roko provider health` command

**File**: `crates/roko-cli/src/main.rs`
**What**: Show circuit breaker state from persisted provider health:

```
$ roko provider health

Provider     State      Fails   Cooldown   Latency p50  Error Rate  Last Check
anthropic    CLOSED     0/3     —          1.2s         0.1%        2m ago
zai          HALF-OPEN  3/3     12s left   —            15.0%       30s ago
openrouter   CLOSED     0/3     —          0.8s         0.0%        5m ago
ollama       OPEN       5/3     45s left   —            100.0%      1m ago
```

Reads from `.roko/learn/provider-health.json` and `.roko/learn/latency-stats.json`.

**Acceptance**: Shows per-provider circuit breaker state, failure counts, latency.
**Verification**: `cargo run -p roko-cli -- provider health`

---

### 2M.03 — Add `roko provider test <provider>` command

**File**: `crates/roko-cli/src/main.rs`
**What**: Send a minimal test request to verify provider connectivity:

```
$ roko provider test zai

Testing provider 'zai' (openai_compat)...
  Endpoint: https://api.z.ai/api/paas/v4/chat/completions
  API Key:  set (ZAI_API_KEY)
  Model:    glm-5.1

  Sending: {"model":"glm-5.1","messages":[{"role":"user","content":"Say hello"}],"max_tokens":10}
  Response: 200 OK (1.2s)
  Content:  "Hello! How can I assist you?"
  Tokens:   input=12, output=8
  Cost:     $0.000052

  ✓ Provider 'zai' is working
```

**Acceptance**: Sends a real request and shows the response. Errors show the full HTTP error.
**Verification**: `cargo run -p roko-cli -- provider test zai` (requires API key)

---

### 2M.04 — Add `roko model list` command

**File**: `crates/roko-cli/src/main.rs`
**What**: List all configured models with provider and capabilities:

```
$ roko model list

Model          Provider     Slug            Context  Tools  Thinking  Vision  Cost (in/out)
claude-opus    anthropic    claude-opus-4-6  200K     ✓      ✗         ✗       $15.00/$75.00
glm-5-1        zai          glm-5.1          200K     ✓      ✓         ✗       $1.40/$4.40
kimi-k2-5      moonshot     kimi-k2.5        256K     ✓      ✓         ✓       $0.60/$3.00
glm-5-1-or     openrouter   z-ai/glm-5.1     200K     ✓      ✓         ✗       $1.26/$3.96
```

**Acceptance**: Shows all models from `[models.*]` with capabilities summary.
**Verification**: `cargo run -p roko-cli -- model list`

---

### 2M.05 — Add `roko model route <model> --explain` command

**File**: `crates/roko-cli/src/main.rs`
**What**: Show the routing decision trace for a model:

```
$ roko model route glm-5-1 --explain --role implementer --complexity integrative

Routing decision for 'glm-5-1':
  Stage: UCB (423 observations)
  Alpha: 0.058 (mostly exploitation)

  Candidate Scores:
    glm-5-1         0.847  (pass: 82%, cost: 0.04, latency: 0.12)  ← selected
    kimi-k2-5       0.791  (pass: 78%, cost: 0.02, latency: 0.15)
    claude-sonnet    0.723  (pass: 88%, cost: 0.42, latency: 0.08)

  Provider Health:
    zai: CLOSED (healthy)

  Cache Affinity:
    Previous model: glm-5-1 (+0.15 bonus applied)

  Pareto Status:
    glm-5-1: ON frontier (not dominated)

  Final: glm-5-1 via zai
```

**Context**: This is the single most important debugging command. When a developer asks "why did roko pick this model?", this command gives the full answer.

**Acceptance**: Shows scoring, health, affinity, and Pareto status for the routing decision.
**Verification**: `cargo run -p roko-cli -- model route glm-5-1 --explain`

---

## B. Configuration Validation

### 2M.06 — Add `roko config validate` command

**File**: `crates/roko-cli/src/config_cmd.rs`
**What**: Three-phase validation of `roko.toml`:

```
$ roko config validate

Phase 1: TOML syntax ............... ✓
Phase 2: Schema validation ......... ✓
Phase 3: Semantic validation:
  ✓ All model providers exist in [providers.*]
  ✓ Fallback chain models exist
  ✓ Tier model keys exist
  ✓ API key env vars are set
  ⚠ Provider 'moonshot' base_url unreachable (timeout 2s)
  ⚠ Model 'kimi-k2-5' references provider 'moonshot' which is unreachable

Result: 2 warnings, 0 errors
```

Phase 1: TOML parses correctly
Phase 2: All required fields present, types correct
Phase 3: Cross-references valid, env vars set, endpoints reachable

**Acceptance**: Invalid config produces actionable error messages. Valid config passes.
**Verification**: `cargo run -p roko-cli -- config validate`

---

### 2M.07 — Add semantic validation for provider/model references

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Validate that model → provider references are consistent:

```rust
pub fn validate_references(config: &RokoConfig) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();
    let provider_keys: HashSet<_> = config.providers.keys().collect();

    for (model_key, profile) in &config.models {
        if !provider_keys.contains(&profile.provider) {
            warnings.push(ValidationWarning::UnknownProvider {
                model: model_key.clone(),
                provider: profile.provider.clone(),
                similar: find_similar(&profile.provider, &provider_keys),
            });
        }
    }

    // Check fallback models exist
    if let Some(ref fallback) = config.agent.fallback_model {
        if !config.models.contains_key(fallback) {
            warnings.push(ValidationWarning::UnknownModel {
                field: "agent.fallback_model".to_string(),
                model: fallback.clone(),
            });
        }
    }

    // Check tier models exist
    // ...

    warnings
}
```

**Context**: The `find_similar()` function uses edit distance to suggest "did you mean 'openrouter' instead of 'openruoter'?" — critical for developer experience.

**Acceptance**: Missing provider produces warning with suggestion.
**Verification**: `cargo test -p roko-core -- validate_references`

---

## C. Testing Infrastructure

### 2M.08 — Add wiremock-based provider mock for tests

**File**: `crates/roko-agent/tests/mock_provider.rs` (new)
**What**: Reusable mock HTTP server for provider tests:

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

pub async fn mock_openai_compat() -> (MockServer, String) {
    let server = MockServer::start().await;

    // Mount default chat completion response
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "mock-1",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Mock response"
                },
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&server)
        .await;

    let base_url = server.uri();
    (server, base_url)
}

pub async fn mock_openai_with_tool_calls() -> (MockServer, String) {
    // ... mock that returns tool_calls on first request, final answer on second
}
```

**Deps**: Add `wiremock = "0.6"` to `roko-agent/Cargo.toml` as `[dev-dependencies]`.

**Context**: Every provider integration test should use this mock. No real API keys needed for CI.

**Acceptance**: `mock_openai_compat()` returns a server that responds to chat completion requests.
**Verification**: `cargo test -p roko-agent -- mock_provider`

---

### 2M.09 — Add recorded fixture system for provider responses

**File**: `crates/roko-agent/tests/fixtures/` (new directory)
**What**: JSON fixture files for each provider's response format:

```
fixtures/
├── glm-5.1/
│   ├── simple_response.json
│   ├── tool_call_response.json
│   ├── thinking_response.json
│   ├── web_search_response.json
│   └── error_rate_limit.json
├── kimi-k2.5/
│   ├── simple_response.json
│   ├── tool_call_response.json
│   ├── thinking_response.json
│   ├── partial_truncated.json
│   └── vision_response.json
├── openrouter/
│   ├── glm_via_openrouter.json
│   └── fallback_different_model.json
└── common/
    ├── 429_rate_limit.json
    ├── 401_auth_failure.json
    └── 500_server_error.json
```

Each fixture is a complete HTTP response body. Tests load these and feed them to mock servers.

**Acceptance**: Fixtures exist for all major response types per provider.
**Verification**: `cargo test -p roko-agent -- fixture_loading`

---

### 2M.10 — Add contract test: verify provider response parsing

**File**: `crates/roko-agent/tests/contract_tests.rs` (new)
**What**: For each fixture, verify that the translator correctly parses it:

```rust
#[test]
fn glm_tool_call_fixture_parses() {
    let fixture = include_str!("fixtures/glm-5.1/tool_call_response.json");
    let json: Value = serde_json::from_str(fixture).unwrap();
    let response = BackendResponse::Json(json);
    let translator = OpenAiTranslator;

    let calls = translator.parse_calls(&response).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "Read");
}

#[test]
fn kimi_thinking_fixture_parses() {
    let fixture = include_str!("fixtures/kimi-k2.5/thinking_response.json");
    let json: Value = serde_json::from_str(fixture).unwrap();
    let reasoning = json.pointer("/choices/0/message/reasoning_content")
        .and_then(|v| v.as_str());
    assert!(reasoning.is_some());
}
```

**Context**: Contract tests catch API format changes. When GLM or Kimi updates their API, re-record the fixture and the test will show what parsing broke.

**Acceptance**: All fixtures parse without errors.
**Verification**: `cargo test -p roko-agent -- contract_tests`

---

## D. Dashboard Extensions

### 2M.11 — Add Provider Health dashboard page

**File**: `crates/roko-cli/src/tui/pages/mod.rs` (add `ProviderHealth` to `PageId`)
**What**: New dashboard page showing provider status:

```
╔══════════════════════════════════════════════════════════════╗
║  Provider Health                                             ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  anthropic    ● CLOSED    p50: 1.2s  err: 0.1%  cost: $4.20 ║
║  zai          ○ HALF-OPEN p50: —     err: 15%   cost: $0.89 ║
║  openrouter   ● CLOSED    p50: 0.8s  err: 0.0%  cost: $1.50 ║
║  ollama       ✗ OPEN      p50: —     err: 100%  cost: $0.00 ║
║                                                              ║
║  Last 24h: 423 requests, 12 failures, 3 fallbacks            ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

Reads from `.roko/learn/provider-health.json` and `.roko/learn/latency-stats.json`.

**Acceptance**: Page renders with provider health data.
**Verification**: `cargo run -p roko-cli -- dashboard --page provider-health`

---

### 2M.12 — Add Model Comparison dashboard page

**File**: `crates/roko-cli/src/tui/pages/mod.rs`
**What**: New page showing cost/quality comparison across models:

```
╔══════════════════════════════════════════════════════════════╗
║  Model Comparison (last 7 days)                              ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  Model            Pass%   Avg Cost  $/Success  Observations  ║
║  kimi-k2.5         78%    $0.08     $0.10      145           ║
║  glm-5.1           82%    $0.19     $0.23      203           ║
║  claude-sonnet      88%    $0.42     $0.48      312           ║
║  claude-opus        94%    $2.10     $2.23       47           ║
║                                                              ║
║  Pareto frontier: kimi-k2.5, glm-5.1, claude-opus            ║
║  (claude-sonnet dominated by glm-5.1)                        ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

**Acceptance**: Page renders with model comparison data from CascadeRouter stats.
**Verification**: `cargo run -p roko-cli -- dashboard --page model-comparison`

---

## E. Structured Routing Decision Log

### 2M.13 — Add routing decision log

**File**: `crates/roko-learn/src/routing_log.rs` (new)
**What**: Log every routing decision to `.roko/learn/routing.jsonl`:

```rust
#[derive(Serialize)]
pub struct RoutingDecisionLog {
    pub timestamp: String,
    pub trace_id: String,
    pub task_id: String,

    // Request
    pub requested_model: String,
    pub role: String,
    pub task_complexity: String,

    // Decision
    pub selected_provider: String,
    pub selected_model: String,
    pub routing_stage: String,        // "static", "confidence", "ucb"
    pub routing_reason: String,       // "highest_ucb_score", "experiment_override", "fallback"

    // Candidates
    pub candidates: Vec<CandidateEntry>,

    // Outcome (filled after completion)
    pub outcome_success: Option<bool>,
    pub outcome_cost_usd: Option<f64>,
    pub outcome_latency_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct CandidateEntry {
    pub model: String,
    pub provider: String,
    pub score: f64,
    pub disqualified: Option<String>,  // "provider_unhealthy", "budget_exceeded", etc.
}
```

**Context**: Enables after-the-fact queries: "why did the router pick GLM over Kimi for this task?" This is the data backing the `roko model route --explain` command and the dashboard routing visualization.

**Acceptance**: Every routing decision is logged with full candidate scoring.
**Verification**: `cargo test -p roko-learn -- routing_decision_log`

---

### 2M.14 — Wire routing log into CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: After each `route()` call, emit a `RoutingDecisionLog` entry:

```rust
impl CascadeRouter {
    pub fn route_logged(
        &self,
        ctx: &RoutingContext,
        log: &RoutingLogger,
    ) -> CascadeModel {
        let candidates = self.score_all_candidates(ctx);
        let selected = self.select_from_candidates(&candidates);

        log.append(RoutingDecisionLog {
            requested_model: ctx.preferred_model.clone(),
            selected_model: selected.primary.slug.clone(),
            candidates: candidates.into_iter().map(|c| CandidateEntry {
                model: c.slug,
                score: c.score,
                disqualified: c.disqualify_reason,
                // ...
            }).collect(),
            // ...
        });

        selected
    }
}
```

**Acceptance**: `.roko/learn/routing.jsonl` grows with each routing decision.
**Verification**: `cargo run -p roko-cli -- run "test" && wc -l .roko/learn/routing.jsonl`

---

## F. Config Migration

### 2M.15 — Add `roko config migrate` command

**File**: `crates/roko-cli/src/config_cmd.rs`
**What**: Auto-migrate old `roko.toml` to new format with `[providers.*]` and `[models.*]`:

```
$ roko config migrate

Detected roko.toml version 1 (no [providers] section)

Proposed changes:
  + [providers.claude_cli]
  +   kind = "claude_cli"
  +   command = "claude"              # from [agent.command]
  +   args = ["--print", ...]         # from [agent.args]
  +
  + [models.claude-sonnet-4-6]
  +   provider = "claude_cli"
  +   slug = "claude-sonnet-4-6"      # from [agent.model]
  +   context_window = 200000
  +   supports_tools = true
  +   tool_format = "anthropic_blocks"
  +
  + config_version = 2

Apply changes? [y/N]
```

**Context**: Reads the old `[agent]` section, generates equivalent `[providers.*]` and `[models.*]` entries, and rewrites the TOML. Always asks for confirmation.

**Acceptance**: Migration preserves all existing functionality. `roko run` works identically before and after.
**Verification**: `cargo run -p roko-cli -- config migrate --dry-run`

---

### 2M.16 — Add config_version field and version detection

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Add `config_version` to `RokoConfig`:

```rust
pub struct RokoConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,  // 1 = old style, 2 = with providers/models
    // ... rest of fields
}

fn default_config_version() -> u32 { 1 }
```

When loading a version 1 config, log a deprecation warning:
```
warning: roko.toml uses config version 1 (no [providers] section)
  hint: run `roko config migrate` to upgrade
```

**Acceptance**: Old configs load with deprecation warning. New configs load silently.
**Verification**: `cargo test -p roko-core -- config_version_detection`

---

## Summary

| Section | Tasks | IDs | Priority |
|---|---|---|---|
| **A. Provider CLI** | 5 | 2M.01–2M.05 | 🟡 P1 |
| **B. Config Validation** | 2 | 2M.06–2M.07 | 🟡 P1 |
| **C. Test Infrastructure** | 3 | 2M.08–2M.10 | 🟡 P1 |
| **D. Dashboard** | 2 | 2M.11–2M.12 | 🟢 P2 |
| **E. Routing Log** | 2 | 2M.13–2M.14 | 🟡 P1 |
| **F. Config Migration** | 2 | 2M.15–2M.16 | 🟡 P1 |
| **Total** | **16** | **2M.01–2M.16** | |

## Execution Order

```
2M.16 (config_version)  ← first: backwards compat
2M.07 (validation)      ← second: catch errors early
2M.06 (validate cmd)    ← third: expose validation
2M.08 (mock server)     ← fourth: test infrastructure
2M.09-10 (fixtures)     ← fifth: contract tests
2M.01-05 (CLI cmds)     ← sixth: developer UX
2M.13-14 (routing log)  ← seventh: observability
2M.11-12 (dashboard)    ← eighth: visualization
2M.15 (migrate cmd)     ← ninth: migration tool
```
