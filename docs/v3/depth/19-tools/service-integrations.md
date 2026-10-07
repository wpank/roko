# Service Integrations

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- three-layer
> integration architecture, operations platform adapters, and structural vs.
> decorative classification.

---

## 1. Overview

Service integrations connect Roko agents to external platforms. They fall into
two categories:

1. **Operations platform adapters** -- collaboration and productivity tools
   (GitHub, Slack, Linear) used for code review, issue triage, and team
   coordination.

2. **Infrastructure services** -- LLM providers, compute backends, and
   payment protocols that agents depend on for operation.

Operations adapters are accessed via MCP servers: `roko-mcp-github` is the
only one roko builds. Slack reaches roko through its webhook alone, with no MCP
tool server, and no Slack or scripts MCP crate exists. Infrastructure services
are accessed through the provider layer in `roko-agent`.

---

## 2. Three-Layer Architecture

All service integrations follow a common three-layer pattern:

```
Layer 1: Event Reception
    +-- Webhook endpoints (GitHub, Slack)
    +-- Polling adapters (Linear, external events)
    +-- WebSocket streams (Slack Socket Mode)

Layer 2: Agent Execution
    +-- Event -> Signal conversion
    +-- Template matching (which agent handles this?)
    +-- Agent spawn with ToolContext

Layer 3: MCP Tool Adapters
    +-- github.* tools (via roko-mcp-github)
```

Events arrive at Layer 1, are converted to Signals, matched to agent
templates via subscription configuration, and the agent executes using tools
from Layer 3.

---

## 3. Operations Platform Adapters

### GitHub Integration

| Aspect | Details |
|---|---|
| **Protocol** | REST API v3 + GraphQL v4 + Webhooks |
| **Authentication** | GitHub App (installation token) or PAT |
| **MCP server** | `roko-mcp-github` (19 tools in v3 catalog) |
| **Webhook events** | push, pull_request, pull_request_review, issues |

Webhook events consumed:
- `push` -- file changes trigger enrichment and the subscribed templates
- `pull_request` -- PR open/update triggers review agents
- `pull_request_review` -- review submission triggers response
- `issues` -- issue open triggers triage

The `roko github status` CLI command exposes GitHub configuration, auth
state, plan PR/CI state, and failure issues.

### Slack Integration

| Aspect | Details |
|---|---|
| **Protocols** | Socket Mode (preferred), HTTP webhook (fallback) |
| **Authentication** | Bot token + signing secret |
| **MCP server** | None: Slack has webhook reception only (`POST /webhooks/slack`) |
| **Rate limits** | 1 message/second per channel (Tier 3) |

Socket Mode is preferred for self-hosted deployments (no public endpoint
needed). HTTP mode requires a public URL.

### Linear Integration

| Aspect | Details |
|---|---|
| **Protocol** | GraphQL API + Webhooks |
| **Authentication** | API key |
| **Use case** | Issue tracking, sprint management, PM sync |

---

## 4. Infrastructure Services

### LLM Providers

Roko supports 12 provider kinds through `roko-agent`:

| Provider | Transport | Authentication |
|---|---|---|
| AnthropicApi | HTTPS | API key |
| OpenAiCompat | HTTPS | API key |
| GeminiApi | HTTPS (stream-JSON) | API key |
| GeminiCli | CLI subprocess | OAuth |
| PerplexityApi | HTTPS | API key |
| CerebrasApi | HTTPS | API key |
| ClaudeCli | CLI subprocess | OAuth |
| CodexCli | CLI subprocess | OAuth |
| CursorAcp | ACP protocol | Per-call auth |
| CursorCli | CLI subprocess | Local |
| Hermes | ACP protocol | Per-call auth |
| OpenClaw | HTTPS | API key |

Provider outcome feedback updates a persisted health registry. Unhealthy
providers are filtered during learned routing.

### Payment Protocols

| Protocol | Use Case |
|---|---|
| x402 | Micropayments for API access and knowledge sales |
| MPP | Session-based billing for sustained operations |
| Server-backed USD | Internal cost accounting (default) |

---

## 5. Event Conversion

Events from platform webhooks are converted to Signals:

```rust
pub enum ServicePlatformCommand {
    // GitHub events
    GitHubPush { repo: String, ref_: String, commits: Vec<CommitInfo> },
    GitHubPullRequest { repo: String, number: u64, action: String },
    GitHubIssue { repo: String, number: u64, action: String },
    GitHubReview { repo: String, pr_number: u64, action: String },

    // Slack events
    SlackMessage { channel: String, user: String, text: String },
    SlackSlashCommand { channel: String, command: String, text: String },

    // Generic
    WebhookPayload { source: String, payload: serde_json::Value },
    CronTick { schedule: String, template: String },
    FileChanged { paths: Vec<PathBuf>, watch_root: PathBuf },
}
```

Each command is converted to a Signal with appropriate `Kind` and `Body`,
then matched against subscription patterns.

---

## 6. Structural vs. Decorative Classification

Integrations are classified by impact on agent capabilities:

**Structural** -- change what the agent CAN do:
- LLM providers (enable inference)
- MCP servers (add new tools)
- Wallet providers (enable signing)

**Decorative** -- change how the agent PRESENTS its work:
- Slack notifications (report results)
- TUI rendering (display progress)
- Telemetry export (emit metrics)

Structural integrations are required for function. Decorative integrations
enhance observability but the agent works without them.

---

## 7. Configuration

```toml
# roko.toml -- service integration configuration
[[agent.mcp_servers]]
name = "github"
command = "roko-mcp-github"
args = ["--repo", "nunchi/roko"]
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

[[subscription]]
pattern = "github.pull_request"
agent_template = "pr-review-agent"
```

---

*Derived from: v1/18-tools/08-service-integrations.md. Chain-specific services
(MetaMask, Uniswap Trading API, Venice, Bankr, AgentCash) moved to chain
domain plugin documentation. Bounty program section removed (not v3 scope).*
