# Configuration Examples

> Ready-to-use `roko.toml` profiles for common deployment scenarios. Copy, paste, and
> adjust to your needs.

**Status**: Shipping
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md)
**Last reviewed**: 2026-04-19

---

## Profile Index

| Profile | Use case |
|---------|---------|
| [Laptop Developer](#laptop-developer) | Daily driver; coding tasks on a personal machine |
| [Server / Team](#server--team) | Shared deployment; multiple users; persistent learning |
| [CI Pipeline](#ci-pipeline) | Automated runs; deterministic; no learning noise |
| [Cluster](#cluster) | Multi-host; shared gateway and substrate |
| [Coding Agent](#coding-agent) | Optimised for software implementation tasks |
| [Research Agent](#research-agent) | Optimised for information gathering and synthesis |
| [Quick Exploration](#quick-exploration) | Fast, cheap, no gates — for prototyping |
| [Audit / Compliance](#audit--compliance) | Maximum verification; no adaptive behaviour |

---

## Laptop Developer

Balanced quality/cost for daily Roko use. Uses Sonnet for most tasks; learning enabled
to build up the local playbook over time.

```toml
[agent]
model              = "claude-sonnet-4-5"
mcp_config         = ".mcp.json"
max_turns          = 25
timeout_seconds    = 300
backend            = "anthropic"
system_prompt_path = "AGENTS.md"

[gate]
pipeline              = ["compile", "test", "clippy", "diff"]
max_retries           = 3
timeout_seconds       = 120
adaptive_thresholds   = true

[learn]
cascade_router           = true
experiments              = true
episode_store            = ".roko/episodes"
playbook_path            = ".roko/playbook.toml"
min_episodes_for_pattern = 5

[substrate]
backend           = "jsonl"
data_dir          = ".roko/substrate"
gc_interval_hours = 24
max_size_gb       = 5.0
```

Required env vars:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

---

## Server / Team

Shared deployment for a small team. Uses Opus for maximum quality; shared learning store
so all users' work improves the team's playbook. Experiments disabled for reproducibility.
Gateway set to route through a shared caching proxy.

```toml
[agent]
model              = "claude-opus-4-6"
mcp_config         = ".mcp.json"
max_turns          = 40
timeout_seconds    = 900
backend            = "anthropic"
base_url           = "http://roko-gateway.internal:4000/"

[gate]
pipeline              = ["compile", "test", "clippy", "diff", "semantic"]
max_retries           = 4
timeout_seconds       = 180
adaptive_thresholds   = false

[learn]
cascade_router           = true
experiments              = false
episode_store            = "/var/roko/shared/episodes"
playbook_path            = "/var/roko/shared/playbook.toml"
min_episodes_for_pattern = 8

[substrate]
backend           = "jsonl"
data_dir          = "/var/roko/substrate"
gc_interval_hours = 12
max_size_gb       = 100.0
```

Required env vars:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

---

## CI Pipeline

Reproducible, deterministic, minimal. Haiku for speed and cost; no learning noise;
fixed thresholds; short timeouts; fails fast.

```toml
[agent]
model           = "claude-haiku-4-5"
mcp_config      = ""
max_turns       = 15
timeout_seconds = 120
backend         = "anthropic"

[gate]
pipeline              = ["compile", "test", "clippy"]
max_retries           = 1
timeout_seconds       = 90
adaptive_thresholds   = false

[learn]
cascade_router = false
experiments    = false

[substrate]
backend = "memory"
```

Required env vars:
```bash
export ANTHROPIC_API_KEY=$CI_ANTHROPIC_API_KEY
```

---

## Cluster

Multi-host deployment with shared gateway. Each host runs a Roko instance; all share the
same gateway, episode store, and playbook via shared storage.

```toml
[agent]
model              = "claude-opus-4-6"
mcp_config         = ".mcp.json"
max_turns          = 50
timeout_seconds    = 1200
backend            = "anthropic"
base_url           = "http://roko-gateway.cluster.internal:4000/"

[gate]
pipeline              = ["compile", "test", "clippy", "format", "security", "diff", "semantic"]
max_retries           = 5
timeout_seconds       = 300
adaptive_thresholds   = true

[learn]
cascade_router           = true
experiments              = false
episode_store            = "/mnt/shared/roko/episodes"
playbook_path            = "/mnt/shared/roko/playbook.toml"
min_episodes_for_pattern = 10

[substrate]
backend           = "jsonl"
data_dir          = "/mnt/shared/roko/substrate"
gc_interval_hours = 6
max_size_gb       = 500.0
```

Required env vars:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

---

## Coding Agent

Optimised for software implementation. Full gate pipeline; aggressive retries; extended
thinking for architectural tasks; system prompt references project AGENTS.md.

```toml
[agent]
model                  = "claude-opus-4-6"
mcp_config             = ".mcp.json"
max_turns              = 35
timeout_seconds        = 900
backend                = "anthropic"
system_prompt_path     = "AGENTS.md"
thinking               = true
thinking_budget_tokens = 12000

[gate]
pipeline              = ["compile", "test", "clippy", "format", "diff", "semantic"]
max_retries           = 5
timeout_seconds       = 240
adaptive_thresholds   = true

[learn]
cascade_router           = true
experiments              = true
episode_store            = ".roko/episodes"
playbook_path            = ".roko/playbook.toml"
min_episodes_for_pattern = 4

[substrate]
backend           = "jsonl"
data_dir          = ".roko/substrate"
gc_interval_hours = 48
max_size_gb       = 15.0
```

Required env vars:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

---

## Research Agent

Optimised for information gathering and synthesis. No compile/test gates (no code output
expected); semantic gate for quality; longer turn and timeout budget; web tools enabled.

```toml
[agent]
model           = "claude-sonnet-4-5"
mcp_config      = ".mcp.json"
max_turns       = 40
timeout_seconds = 600
backend         = "anthropic"

[gate]
pipeline          = ["semantic"]
max_retries       = 2
timeout_seconds   = 60

[learn]
cascade_router = true
experiments    = true
episode_store  = ".roko/episodes"
playbook_path  = ".roko/playbook.toml"

[substrate]
backend   = "jsonl"
data_dir  = ".roko/substrate"
max_size_gb = 3.0
```

`.mcp.json` for the research agent:
```json
{
  "mcpServers": {
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "env": { "BRAVE_API_KEY": "${BRAVE_API_KEY}" }
    }
  }
}
```

Required env vars:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
export BRAVE_API_KEY=...
```

---

## Quick Exploration

No gates, no learning, in-memory storage. Use this for trying things out quickly. Not
for production.

```toml
[agent]
model           = "claude-sonnet-4-5"
max_turns       = 10
timeout_seconds = 120

[gate]
pipeline = []

[learn]
cascade_router = false
experiments    = false

[substrate]
backend = "memory"
```

---

## Audit / Compliance

Maximum verification. No adaptive behaviour. Full gate pipeline. Fixed thresholds.
No experiments. Fully deterministic across runs.

```toml
[agent]
model              = "claude-opus-4-6"
mcp_config         = ".mcp.json"
max_turns          = 30
timeout_seconds    = 600
backend            = "anthropic"

[gate]
pipeline              = ["compile", "test", "clippy", "format", "security", "diff", "semantic"]
continue_on_failure   = true   # collect ALL failures in one pass
max_retries           = 0      # no automatic retries in audit mode
timeout_seconds       = 300
adaptive_thresholds   = false  # fixed thresholds for reproducibility

[learn]
cascade_router = false
experiments    = false

[substrate]
backend           = "jsonl"
data_dir          = ".roko/audit-substrate"
gc_interval_hours = 168   # weekly GC — preserve audit data
max_size_gb       = 50.0
```

---

## See Also

- [01-roko-toml-schema.md](01-roko-toml-schema.md) — all available keys
- [08-environment-variables.md](08-environment-variables.md) — setting API keys
- [13-security-considerations.md](13-security-considerations.md) — secrets handling
