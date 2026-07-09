# roko.toml Schema Reference

> Every table, every key, every value type, default, valid range, and example. This is
> the authoritative reference for `roko.toml`. When in doubt, this page wins.

**Status**: Shipping
**Crate**: `roko-cli`, `roko-orchestrator`
**Depends on**: [00-overview.md](00-overview.md)
**Used by**: [02-agent-config.md](02-agent-config.md), [03-gate-config.md](03-gate-config.md), [04-learn-config.md](04-learn-config.md), [05-substrate-config.md](05-substrate-config.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`roko.toml` has five top-level tables: `[agent]`, `[gate]`, `[learn]`, `[substrate]`, and
`[bus]`. All tables are optional. All keys have documented defaults. Any key absent from the
file takes its default value.

---

## Schema Notation

Each key entry uses the following format:

```
key_name
  Type:    <Rust type, as seen in TOML>
  Default: <value used when the key is absent>
  Range:   <valid values or constraints>
  Env var: <ROKO_UPPER_SNAKE equivalent for override>
  Example: <concrete TOML snippet>
  Notes:   <anything an operator needs to know>
```

---

## `[agent]` Table

Full reference: [02-agent-config.md](02-agent-config.md).

### `agent.model`

```
Type:    String
Default: "claude-sonnet-4-5"
Range:   Any model slug accepted by the configured LLM backend. Common values:
           "claude-opus-4-6"      (Anthropic, highest quality)
           "claude-sonnet-4-5"    (Anthropic, default — quality/cost balance)
           "claude-haiku-4-5"     (Anthropic, fastest/cheapest)
           "gpt-4o"               (OpenAI)
           "gpt-4o-mini"          (OpenAI, fast)
           "gemini-2.5-pro"       (Google)
           "o3"                   (OpenAI reasoning)
Env var: ROKO_AGENT_MODEL
Example: model = "claude-opus-4-6"
Notes:   This is the default model for all agents. Individual task classification
         in [learn] can override this per-task via CascadeRouter. The model slug
         must match the backend you are using; see agent.backend.
```

### `agent.mcp_config`

```
Type:    String (file path, relative to project root)
Default: ".mcp.json"
Range:   Any valid file path. The file must exist if specified and non-empty.
Env var: ROKO_MCP_CONFIG
Example: mcp_config = "config/mcp.json"
Notes:   Path to the MCP tool server discovery file. Set to empty string ("") to
         disable MCP tool discovery. See 07-mcp-config.md for the file format.
```

### `agent.max_turns`

```
Type:    Integer (u32)
Default: 25
Range:   1 – 200. Values above 100 risk very long-running tasks; use with caution.
Env var: ROKO_AGENT_MAX_TURNS
Example: max_turns = 40
Notes:   Maximum number of LLM turns an agent is allowed per task. When this limit
         is reached, the agent receives a "max turns reached" signal and must
         produce a final response or fail. Does not count tool calls as turns unless
         the backend counts them (OpenAI counts tool calls; Anthropic does not).
```

### `agent.timeout_seconds`

```
Type:    Integer (u64)
Default: 600
Range:   30 – 3600. Below 30s will cause spurious timeouts on network-heavy tasks.
Env var: ROKO_AGENT_TIMEOUT_SECONDS
Example: timeout_seconds = 900
Notes:   Wall-clock timeout per task. When exceeded, the task is cancelled and
         marked as timed out. The executor writes a partial state snapshot so the
         task can be resumed. Separate from the gate timeout (gate.timeout_seconds).
```

### `agent.backend`

```
Type:    String
Default: "anthropic"
Range:   "anthropic" | "openai" | "openrouter" | "ollama" | "bedrock" | "vertex"
Env var: ROKO_AGENT_BACKEND
Example: backend = "openrouter"
Notes:   The LLM API backend. Determines which API key environment variable is
         required (ANTHROPIC_API_KEY, OPENAI_API_KEY, OPENROUTER_API_KEY, etc.).
         "openrouter" provides access to 400+ models via a single key.
```

### `agent.base_url`

```
Type:    String (URL)
Default: "" (uses the default URL for the selected backend)
Range:   Any valid HTTPS URL ending in /
Env var: ROKO_AGENT_BASE_URL
Example: base_url = "http://localhost:4000/"
Notes:   Override the LLM API base URL. Set this to point at a local gateway
         (e.g. a caching proxy at localhost:4000) or an OpenAI-compatible endpoint
         such as Ollama or LM Studio. Must include the trailing slash.
```

### `agent.system_prompt_path`

```
Type:    String (file path)
Default: "" (uses built-in default system prompt)
Range:   Path to a .md or .txt file; must be readable.
Env var: ROKO_AGENT_SYSTEM_PROMPT_PATH
Example: system_prompt_path = "AGENTS.md"
Notes:   Path to a custom system prompt file. If set, this file replaces the
         built-in system prompt entirely. AGENTS.md in the project root is the
         conventional name. The file is read once at startup and cached.
```

### `agent.thinking`

```
Type:    Boolean
Default: false
Range:   true | false
Env var: ROKO_AGENT_THINKING
Example: thinking = true
Notes:   Enable extended thinking for backends that support it (Anthropic Claude 3.7+).
         Increases token usage significantly (budget: ~10K thinking tokens by default).
         Use for architectural decisions and complex multi-step reasoning. Controlled
         by agent.thinking_budget_tokens when enabled.
```

### `agent.thinking_budget_tokens`

```
Type:    Integer (u32)
Default: 10000
Range:   1000 – 100000
Env var: ROKO_AGENT_THINKING_BUDGET_TOKENS
Example: thinking_budget_tokens = 20000
Notes:   Maximum thinking tokens allocated when agent.thinking = true. Higher values
         allow deeper reasoning but increase latency and cost. Only used when
         agent.thinking = true.
```

---

## `[gate]` Table

Full reference: [03-gate-config.md](03-gate-config.md).

### `gate.pipeline`

```
Type:    Array of String
Default: ["compile", "test", "clippy", "diff"]
Range:   Any ordered subset of: "compile", "test", "clippy", "diff", "semantic",
         "security", "format", "coverage", "custom:<name>"
Env var: ROKO_GATE_PIPELINE (comma-separated: "compile,test,clippy")
Example: pipeline = ["compile", "test", "clippy", "diff", "semantic"]
Notes:   The ordered list of gates to run after each agent task completes. Gates
         run in the order listed. The first failure stops the pipeline unless
         gate.continue_on_failure = true. An empty array disables all verification.
```

### `gate.continue_on_failure`

```
Type:    Boolean
Default: false
Range:   true | false
Env var: ROKO_GATE_CONTINUE_ON_FAILURE
Example: continue_on_failure = true
Notes:   If true, the gate pipeline continues running remaining gates after a failure.
         All failures are collected and reported together. Useful for auditing all
         issues in a single pass. Not recommended for production; gate failures should
         stop execution by default.
```

### `gate.max_retries`

```
Type:    Integer (u32)
Default: 3
Range:   0 – 8. 0 disables retries.
Env var: ROKO_GATE_MAX_RETRIES
Example: max_retries = 5
Notes:   Maximum number of agent retry cycles when a gate fails. Each retry sends
         the gate failure back to the agent for correction. After max_retries
         exhausted, the task is marked failed. The retry uses iteration memory
         (accumulated DO-NOT-REPEAT list) to avoid repeating the same mistakes.
```

### `gate.timeout_seconds`

```
Type:    Integer (u64)
Default: 120
Range:   10 – 600
Env var: ROKO_GATE_TIMEOUT_SECONDS
Example: timeout_seconds = 180
Notes:   Per-gate wall-clock timeout. If a single gate (e.g. test runner) exceeds
         this limit, the gate is marked as timed out and treated as a failure.
         Separate from agent.timeout_seconds.
```

### `gate.adaptive_thresholds`

```
Type:    Boolean
Default: true
Range:   true | false
Env var: ROKO_GATE_ADAPTIVE_THRESHOLDS
Example: adaptive_thresholds = false
Notes:   If true, gate pass/fail thresholds adjust via EMA (exponential moving
         average) based on observed pass rates. A gate that almost always passes
         will tighten its threshold over time; a gate that is frequently failing
         will loosen it until the root cause is fixed. Disable to use fixed
         thresholds only.
```

---

## `[learn]` Table

Full reference: [04-learn-config.md](04-learn-config.md).

### `learn.cascade_router`

```
Type:    Boolean
Default: true
Range:   true | false
Env var: ROKO_LEARN_CASCADE_ROUTER
Example: cascade_router = false
Notes:   Enable the T0→T1→T2 CascadeRouter for per-task model routing. When true,
         cheap deterministic rules (T0) are tried first, then a fast cheap model
         (T1), escalating to the configured agent.model only when lower tiers fail.
         Reduces average inference cost by routing simple tasks to cheaper models.
```

### `learn.experiments`

```
Type:    Boolean
Default: true
Range:   true | false
Env var: ROKO_LEARN_EXPERIMENTS
Example: experiments = false
Notes:   Enable prompt A/B testing via Thompson Sampling bandits. When true, the
         learning runtime randomly explores prompt variants for each task category
         and promotes the best-performing variant. Disable in latency-sensitive
         or reproducibility-required environments.
```

### `learn.episode_store`

```
Type:    String (directory path)
Default: ".roko/episodes"
Range:   Any writable directory path.
Env var: ROKO_LEARN_EPISODE_STORE
Example: episode_store = "/var/roko/episodes"
Notes:   Directory where learning episodes are stored. Each episode is a JSONL
         record of: task type, model used, tokens, cost, gate pass, iterations,
         and HDC fingerprint. Episodes are the input to the playbook pattern
         extractor. On a shared server, set this to a shared path.
```

### `learn.playbook_path`

```
Type:    String (file path)
Default: ".roko/playbook.toml"
Range:   Any writable file path.
Env var: ROKO_LEARN_PLAYBOOK_PATH
Example: playbook_path = "/var/roko/playbook.toml"
Notes:   Path to the playbook rules file. Promoted patterns (rules that correctly
         predict outcomes across 5+ builds) are written here and injected into
         agent context. Commit this file to version control to share learned
         heuristics across the team.
```

### `learn.min_episodes_for_pattern`

```
Type:    Integer (u32)
Default: 5
Range:   2 – 50
Env var: ROKO_LEARN_MIN_EPISODES_FOR_PATTERN
Example: min_episodes_for_pattern = 3
Notes:   Minimum similar episodes required before a pattern is extracted. Lower
         values produce patterns faster but with less evidence. Higher values
         produce more reliable patterns but take longer to emerge. 5 is the
         tuned default based on empirical testing.
```

### `learn.distillation`

```
Type:    Boolean
Default: false
Range:   true | false
Env var: ROKO_LEARN_DISTILLATION
Example: distillation = true
Notes:   Enable knowledge distillation: periodically compress the episode store
         into a condensed playbook representation using a cheap LLM pass.
         Reduces episode store growth over time. Built but not enabled by default
         (needs more production testing). Status: Built.
```

---

## `[substrate]` Table

Full reference: [05-substrate-config.md](05-substrate-config.md).

### `substrate.backend`

```
Type:    String
Default: "jsonl"
Range:   "jsonl" | "memory" | "sqlite" (planned) | "lancedb" (planned)
Env var: ROKO_SUBSTRATE_BACKEND
Example: backend = "jsonl"
Notes:   Storage backend for the Substrate trait (Engram persistence). "jsonl"
         uses append-only JSONL files (FileSubstrate, Shipping). "memory" uses
         an in-memory store (useful for testing and ephemeral runs). "sqlite" and
         "lancedb" are planned for a future release.
```

### `substrate.data_dir`

```
Type:    String (directory path)
Default: ".roko/substrate"
Range:   Any writable directory. Created automatically if absent.
Env var: ROKO_SUBSTRATE_DATA_DIR
Example: data_dir = "/var/roko/substrate"
Notes:   Root directory for the JSONL substrate storage. Contains one file per
         Engram kind (e.g. engrams.jsonl, episodes.jsonl, playbook-rules.jsonl).
         On a server, point this to a persistent volume. For a laptop, the default
         is fine.
```

### `substrate.gc_interval_hours`

```
Type:    Integer (u32)
Default: 24
Range:   1 – 168 (1 hour to 1 week)
Env var: ROKO_SUBSTRATE_GC_INTERVAL_HOURS
Example: gc_interval_hours = 12
Notes:   How often the substrate garbage collector runs. GC removes Engrams whose
         all decay values have fallen below the configured floor and which are not
         referenced by any active provenance chain. Lower values keep disk usage
         tighter at the cost of more frequent GC scans.
```

### `substrate.max_size_gb`

```
Type:    Float (f64)
Default: 10.0
Range:   0.1 – 1000.0 (gigabytes)
Env var: ROKO_SUBSTRATE_MAX_SIZE_GB
Example: max_size_gb = 50.0
Notes:   Soft disk usage cap. When the substrate directory exceeds this size, the
         GC is triggered immediately (outside the normal schedule) and accelerated
         decay is applied to cold-tier Engrams. If the cap is still exceeded after
         GC, a warning is emitted; the runtime does not hard-fail. 0.0 disables
         the cap.
```

---

## `[bus]` Table

Full reference: [06-bus-config.md](06-bus-config.md).

**Status: Specified (target-state). The Bus abstraction is not yet required for Shipping
configurations. The shipped transport is `EventBus<E>` in `roko-runtime`, which does not
need configuration. This table is reserved for when the Bus is promoted to Shipping.**

### `bus.backend`

```
Type:    String
Default: "internal"
Range:   "internal" | "nats" | "redis" (planned)
Env var: ROKO_BUS_BACKEND
Example: backend = "nats"
Notes:   [Target-state] Transport backend for the Bus abstraction. "internal" uses
         the in-process EventBus<E> (default, no external dependencies). "nats"
         connects to a NATS server for multi-process or multi-host event distribution.
```

### `bus.url`

```
Type:    String (URL)
Default: ""
Range:   A valid connection URL for the selected backend.
Env var: ROKO_BUS_URL
Example: url = "nats://localhost:4222"
Notes:   [Target-state] Connection URL for external bus backends. Required when
         bus.backend != "internal".
```

---

## Full Example

A complete `roko.toml` with all Shipping keys explicitly set (showing their defaults):

```toml
[agent]
model              = "claude-sonnet-4-5"
mcp_config         = ".mcp.json"
max_turns          = 25
timeout_seconds    = 600
backend            = "anthropic"
base_url           = ""
system_prompt_path = ""
thinking           = false
thinking_budget_tokens = 10000

[gate]
pipeline              = ["compile", "test", "clippy", "diff"]
continue_on_failure   = false
max_retries           = 3
timeout_seconds       = 120
adaptive_thresholds   = true

[learn]
cascade_router              = true
experiments                 = true
episode_store               = ".roko/episodes"
playbook_path               = ".roko/playbook.toml"
min_episodes_for_pattern    = 5
distillation                = false

[substrate]
backend             = "jsonl"
data_dir            = ".roko/substrate"
gc_interval_hours   = 24
max_size_gb         = 10.0
```

---

## See Also

- [02-agent-config.md](02-agent-config.md) — deep dive on `[agent]`
- [03-gate-config.md](03-gate-config.md) — deep dive on `[gate]`
- [04-learn-config.md](04-learn-config.md) — deep dive on `[learn]`
- [05-substrate-config.md](05-substrate-config.md) — deep dive on `[substrate]`
- [10-config-validation.md](10-config-validation.md) — what happens when a key is wrong
- [12-examples.md](12-examples.md) — role-specific profiles

## Open Questions

- Whether `[substrate]` should support multiple named stores (e.g. `[substrate.episodes]`, `[substrate.engrams]`) with per-store backends.
- `[bus]` key set is provisional; exact API subject to change when Bus is promoted to Shipping.
- `agent.backend` registry — the full list of supported slugs is not yet auto-generated from code.
