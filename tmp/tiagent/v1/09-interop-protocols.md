# 09 - MCP, A2A, AITP, x402 Protocol Interoperability

## What This Document Covers

tiagent is a general-purpose coding agent. One of its strengths is connecting to
external tool ecosystems and collaborating with other agents. This document
describes how tiagent integrates with four interoperability protocols that serve
different audiences:

- **MCP** (Model Context Protocol) -- **Relevant to all users.** How tiagent connects to the 97M+ monthly download ecosystem of developer tools. This is the primary integration and the one most developers will use.
- **A2A** (Agent-to-Agent Protocol) -- **Relevant for agent collaboration.** Discovery and task delegation between agents built on different frameworks.
- **AITP** (Agent Interaction and Transaction Protocol) -- **Relevant for on-chain/payment use cases (optional).** Transactional messaging between agents, primarily in the NEAR ecosystem.
- **x402** -- **Relevant for on-chain/payment use cases (optional).** Pay-per-use API access via HTTP 402 payment flows.

> **What most developers need:** Most developers only need MCP. It is how
> tiagent discovers and calls tools -- the same way VS Code, Claude Desktop,
> and other AI-powered tools connect to external services. A2A is useful if you
> want tiagent to collaborate with other agents. AITP and x402 are opt-in
> extensions for blockchain-specific payment and transaction use cases.

Each protocol solves a different problem. tiagent supports all four through a
unified adapter layer that converts protocol-specific messages into its internal
Signal abstraction.

---

## 1. The Interoperability Challenge

The core challenge for any coding agent is connecting to the tools developers
actually use. MCP solves this for the vast majority of cases -- it is the
standard for connecting LLMs to external tools, and the ecosystem already has
thousands of available servers covering code analysis, databases, cloud
services, and more.

Beyond tool integration, agents increasingly need to collaborate with other
agents (A2A), and some use cases require payment flows (AITP, x402). No single
protocol covers all of these needs, so tiagent supports multiple protocols
through a unified adapter layer.

Consider the range of interactions tiagent can handle:

1. Call a code analysis tool hosted as an MCP server (MCP -- **most common**)
2. Delegate a subtask to a specialized research agent run by a different
   organization (A2A -- **agent collaboration**)
3. Pay for premium API access to a data provider (x402 -- **optional, on-chain**)
4. Execute a cross-agent financial transaction through NEAR-based
   infrastructure (AITP -- **optional, on-chain**)

Most developers will only encounter the first case. The remaining protocols are
available when needed.

tiagent addresses this by implementing adapters for each protocol and mapping
all external interactions to its internal Signal type. From the perspective of
tiagent's core loop, every inbound message -- whether it arrives as an MCP tool
call, an A2A task, an AITP thread message, or an x402 payment challenge -- is
just a Signal to be scored, routed, composed, acted upon, verified, and
persisted.

---

## 2. Protocol Comparison Matrix

| Attribute         | MCP                    | A2A                     | AITP                   | x402                   |
|-------------------|------------------------|-------------------------|------------------------|------------------------|
| **Created by**    | Anthropic              | Google                  | NEAR Protocol          | Coinbase (open spec)   |
| **Purpose**       | Tool integration       | Agent collaboration     | Agent transactions     | API payments           |
| **Transport**     | stdio / SSE / HTTP     | HTTP / SSE / Push       | Thread-based messages  | HTTP 402 responses     |
| **Adoption**      | Dominant (~97M/mo SDK) | Growing (150+ orgs)     | Early (NEAR ecosystem) | Early                  |
| **Content model** | JSON Schema tools      | Multimodal (text/files) | Typed capabilities     | Payment proofs         |
| **Statefulness**  | Session-based          | Task lifecycle          | Thread lifecycle       | Stateless per-request  |
| **tiagent role**  | Client + Server        | Peer                    | Bridge via IronClaw    | Client                 |

Key observations:

- MCP and A2A are complementary. Anthropic and Google have both acknowledged
  this publicly. MCP connects agents to tools; A2A connects agents to agents.
- AITP and x402 both address payments but at different layers. AITP embeds
  payment as one of several agent capabilities within a conversation. x402
  operates at the HTTP transport layer, invisible to the agent's reasoning.
- Only A2A defines a discovery mechanism (Agent Cards). MCP servers must be
  configured explicitly. AITP discovery relies on NEAR's infrastructure.

---

## 3. MCP Integration (Primary -- relevant to all users)

> **This is the most important section for regular developers.** MCP is how
> tiagent connects to the ecosystem of developer tools -- code analysis,
> database access, cloud services, GitHub, and thousands more. Every developer
> using tiagent benefits from MCP, whether or not they use any other protocol
> described in this document.

MCP is tiagent's primary interoperability protocol. It is the most widely
adopted standard for connecting LLMs to external tools, with roughly 97 million
monthly SDK downloads across the npm and PyPI ecosystems.

### 3.1 Architecture

MCP follows a client-server architecture:

```
+------------------+          +------------------+
|                  |  stdio   |                  |
|  tiagent         | -------> |  MCP Server      |
|  (MCP Client)    | <------- |  (tool provider) |
|                  |          |                  |
+------------------+          +------------------+
```

The client (tiagent) sends JSON-RPC requests to MCP servers. Each server
exposes three types of primitives:

- **Tools**: Functions the agent can call. Described by JSON Schema with name,
  description, and input schema. Examples: `read_file`, `search_code`,
  `submit_blob`.
- **Resources**: Read-only data the agent can access. Identified by URI.
  Examples: `celestia://block/12345`, `file:///path/to/config`.
- **Prompts**: Reusable prompt templates. Parameterized text that the client
  can fill in and send to the LLM.

Transport options:

- **stdio**: Server runs as a subprocess. Communication over stdin/stdout.
  Simplest to configure. Used for local tools.
- **SSE (Server-Sent Events)**: Server runs as an HTTP service. Client connects
  via SSE for server-to-client streaming, POST for client-to-server messages.
  Used for remote tools.
- **Streamable HTTP**: Newer transport that uses standard HTTP with optional
  streaming. Preferred for new deployments.

### 3.2 tiagent as MCP Client

tiagent discovers and connects to MCP servers through two mechanisms:

**Explicit configuration** in `tiagent.toml`:

```toml
[mcp.servers.celestia-tools]
command = "tiagent-mcp-celestia"
args = ["--network", "mocha"]

[mcp.servers.code-intel]
command = "tiagent-mcp-code"
args = ["--workspace", "."]

[mcp.servers.github]
command = "tiagent-mcp-github"
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }
```

**Auto-discovery** of binaries matching the `tiagent-mcp-*` naming convention.
tiagent scans `$PATH` for executables with this prefix and registers them as
available MCP servers. This allows installing an MCP server (e.g.,
`tiagent-mcp-celestia`) and having it appear automatically without manual
configuration.

When tiagent starts, it:

1. Reads the `[mcp.servers]` table from `tiagent.toml`
2. Scans `$PATH` for `tiagent-mcp-*` binaries not already configured
3. Spawns each server as a subprocess (stdio transport) or connects via HTTP
4. Calls `initialize` to negotiate protocol version and capabilities
5. Calls `tools/list` to discover available tools
6. Merges MCP-discovered tools into the built-in tool registry

The merged registry is what the LLM sees when it asks "what tools are
available?" Built-in tools and MCP tools coexist in the same namespace.

### 3.3 tiagent as MCP Server

tiagent can also expose its own capabilities as an MCP server, allowing other
MCP clients (Claude Desktop, VS Code extensions, other agents) to use tiagent
as a tool provider.

Exposed tools include:

| Tool                  | Description                                    |
|-----------------------|------------------------------------------------|
| `submit_blob`         | Submit a blob to a Celestia namespace          |
| `get_blob`            | Retrieve a blob by height and namespace        |
| `query_knowledge`     | Search tiagent's durable knowledge store       |
| `run_agent`           | Dispatch a task to a tiagent agent             |
| `get_signal`          | Retrieve a signal by hash                      |
| `list_namespaces`     | List known Celestia namespaces                 |

This bidirectional capability means tiagent can both consume and provide tools
within the MCP ecosystem.

### 3.4 Celestia-Specific MCP Server

`tiagent-mcp-celestia` is a dedicated MCP server that wraps Celestia node
operations as MCP tools. It connects to a Celestia light node or bridge node
and exposes:

- Blob submission and retrieval
- Namespace management
- Block and header queries
- State queries (balances, delegations)
- Data availability sampling status

This server is distributed as a standalone binary so that any MCP client -- not
just tiagent -- can interact with Celestia through standard MCP tooling.

---

## 4. A2A Integration (relevant for agent collaboration)

A2A (Agent-to-Agent Protocol) by Google enables agents built on different
frameworks to discover each other, communicate, and collaborate on tasks. Where
MCP connects an agent to tools, A2A connects an agent to other agents.

### 4.1 Agent Cards

Every A2A-compatible agent publishes an Agent Card at a well-known URL:

```
GET https://agent.example.com/.well-known/agent.json
```

tiagent publishes its own Agent Card describing its capabilities:

```json
{
  "name": "tiagent",
  "description": "Self-improving agent harness for Celestia",
  "url": "https://tiagent.example.com",
  "version": "0.1.0",
  "capabilities": {
    "streaming": true,
    "pushNotifications": true
  },
  "skills": [
    {
      "id": "celestia-da",
      "name": "Celestia Data Availability",
      "description": "Submit and retrieve blobs from Celestia namespaces",
      "tags": ["celestia", "data-availability", "blobs"]
    },
    {
      "id": "code-analysis",
      "name": "Code Analysis",
      "description": "Analyze codebases using tree-sitter parsing and HDC indexing",
      "tags": ["code", "analysis", "rust"]
    }
  ],
  "authentication": {
    "schemes": ["bearer"]
  }
}
```

Other agents can discover tiagent by fetching this card and deciding whether
its skills match their needs.

### 4.2 Task Lifecycle

A2A defines a task lifecycle that maps naturally to tiagent's Signal lifecycle:

```
A2A Task States          tiagent Signal States
-----------------        ---------------------
submitted       ------>  created
working         ------>  processing
input-required  ------>  blocked (awaiting input)
completed       ------>  resolved
failed          ------>  failed
canceled        ------>  canceled
```

When tiagent receives an A2A task:

1. The A2A adapter creates a Signal from the task payload
2. The Signal enters tiagent's core loop (score, route, compose, act, verify)
3. State transitions in the Signal are reflected back as A2A task updates
4. The final result is returned as an A2A task artifact

When tiagent sends an A2A task to another agent:

1. tiagent discovers the target agent's capabilities via its Agent Card
2. It creates an A2A task with the appropriate skill ID
3. It monitors the task via SSE streaming or polling
4. The result is converted back to a Signal for further processing

### 4.3 Streaming

For long-running tasks, A2A supports SSE streaming. tiagent uses this for:

- Sending incremental progress updates to the requesting agent
- Receiving incremental results from agents it has delegated to
- Maintaining real-time awareness of multi-agent workflows

---

## 5. AITP Integration (optional -- on-chain/payment use cases)

AITP (Agent Interaction and Transaction Protocol) by NEAR Protocol provides
thread-based messaging with typed capabilities. tiagent integrates with AITP
through its IronClaw bridge rather than implementing AITP natively.

### 5.1 AITP Capabilities

AITP defines five capability types:

| Capability           | Purpose                              | tiagent Use Case                      |
|----------------------|--------------------------------------|---------------------------------------|
| Data Requests        | Ask for structured data              | Query external data sources           |
| Payments             | Transfer value between agents        | Pay for compute, data, API access     |
| Agent Delegation     | Pass tasks to specialized agents     | Delegate Celestia-specific work       |
| Decisions            | Request approval or choices          | Human-in-the-loop checkpoints         |
| Identity Verification| Verify agent or user identity        | Authenticate before sensitive ops     |

### 5.2 IronClaw Bridge

tiagent does not implement AITP directly. Instead, it connects to IronClaw (a
NEAR-ecosystem agent framework) which provides AITP support. The integration
works through IronClaw's API:

```
tiagent  ---HTTP--->  IronClaw  ---AITP--->  NEAR Agent
Signal                Thread                  Thread
```

tiagent converts outbound Signals to IronClaw API calls. IronClaw handles
the AITP protocol mechanics: thread management, capability negotiation, and
NEAR token transfers. Responses flow back through the same path and are
converted to Signals.

This bridge approach avoids duplicating AITP protocol logic and leverages
IronClaw's existing integration with the NEAR ecosystem.

### 5.3 Thread-to-Signal Mapping

AITP conversations happen within threads. Each thread maps to a Signal chain
in tiagent:

- Thread creation maps to a new root Signal
- Thread messages map to child Signals linked by parent hash
- Thread resolution maps to a terminal Signal with the result
- Payment events within threads create associated payment Signals

---

## 6. x402 Payment Integration (optional -- on-chain/payment use cases)

x402 uses the HTTP 402 ("Payment Required") status code to enable automatic
micropayments for API access. When an agent calls an API that requires payment,
the server responds with 402 and payment instructions. The agent pays and
retries.

### 6.1 Payment Flow

```
tiagent                          API Server
   |                                |
   |  GET /api/data                 |
   |------------------------------->|
   |                                |
   |  402 Payment Required          |
   |  X-Payment: {amount, chain,    |
   |    recipient, token}           |
   |<-------------------------------|
   |                                |
   |  [evaluate cost vs budget]     |
   |  [sign payment transaction]    |
   |                                |
   |  GET /api/data                 |
   |  X-Payment-Proof: {tx_hash,    |
   |    chain, signature}           |
   |------------------------------->|
   |                                |
   |  200 OK + data                 |
   |<-------------------------------|
```

### 6.2 Budget Management

Uncontrolled payments are dangerous. tiagent enforces spending limits:

```toml
[protocols.x402]
enabled = true
max_per_request = "0.01"     # Maximum payment per individual request
max_per_session = "1.00"     # Maximum total spend per agent session
max_per_day = "10.00"        # Maximum daily spend across all sessions
currency = "TIA"             # Default payment currency
auto_approve_below = "0.001" # Auto-approve payments below this threshold
```

Payments above `auto_approve_below` require explicit approval through the
decision pipeline (which can be automated or human-in-the-loop depending on
configuration).

### 6.3 Settlement on Celestia

While x402 supports EVM chains and Bitcoin Lightning for settlement, tiagent
can also settle on Celestia. Payment proofs are submitted as blobs to a
dedicated namespace, creating a transparent audit trail of agent spending.

This is optional. For low-value micropayments, settling on a cheaper chain
may be more practical. The settlement chain is configurable per API endpoint.

---

## 7. Unified Protocol Layer

All four protocols are integrated through a common adapter layer. The adapter
converts protocol-specific messages to and from tiagent's internal Signal
representation.

### 7.1 Architecture

```
                    +-------------------------------------------+
                    |              tiagent core                  |
                    |                                           |
                    |   Signal -> Score -> Route -> Compose ->  |
                    |   Act -> Verify -> Write -> React         |
                    |                                           |
                    +-------------------+-----------------------+
                                        |
                              Internal Signals
                                        |
                    +-------------------+-----------------------+
                    |          ProtocolAdapter layer             |
                    |                                           |
                    |  +--------+  +--------+  +------+  +---+ |
                    |  |  MCP   |  |  A2A   |  | AITP |  |402| |
                    |  |Adapter |  |Adapter |  |Bridge|  |Adp| |
                    |  +---+----+  +---+----+  +--+---+  +-+-+ |
                    +------|-----------|---------|---------|-----+
                           |           |         |         |
                    stdio/HTTP    HTTP/SSE   IronClaw   HTTP 402
                           |           |         |         |
                    MCP Servers   A2A Agents  NEAR Agents  APIs
```

### 7.2 The ProtocolAdapter Trait

Each protocol implements a common trait:

```rust
trait ProtocolAdapter {
    /// Convert an inbound protocol message to a Signal
    fn inbound(&self, raw: &[u8]) -> Result<Signal>;

    /// Convert an outbound Signal to a protocol message
    fn outbound(&self, signal: &Signal) -> Result<Vec<u8>>;

    /// Check if this adapter handles the given message format
    fn can_handle(&self, raw: &[u8]) -> bool;

    /// Protocol-specific health check
    fn health_check(&self) -> Result<ProtocolHealth>;
}
```

The adapter layer sits between the transport layer and the core loop. Inbound
messages are deserialized by the appropriate adapter and converted to Signals.
Outbound Signals are serialized by the adapter into the target protocol format.

### 7.3 Signal Mapping

Every protocol message maps to a Signal with protocol-specific metadata:

| Protocol Field        | Signal Field          | Notes                              |
|-----------------------|-----------------------|------------------------------------|
| MCP tool call         | Signal.payload        | Tool name + args in payload        |
| MCP tool result       | Signal.payload        | Result data in payload             |
| A2A task              | Signal (root)         | Task ID stored in metadata         |
| A2A task update       | Signal (child)        | Linked by parent hash              |
| AITP thread message   | Signal (chain)        | Thread ID in metadata              |
| x402 payment request  | Signal (payment type) | Amount, recipient in metadata      |
| x402 payment proof    | Signal (receipt type)  | Tx hash in metadata                |

The `origin` field on each Signal records which protocol produced it, enabling
protocol-aware routing downstream. For example, a Signal originating from A2A
might be routed to a different model than one originating from an MCP tool
call.

### 7.4 Protocol Routing

When tiagent needs to send a message externally, it determines the correct
protocol based on the target:

1. If the target is a known MCP server, use MCP
2. If the target has an A2A Agent Card, use A2A
3. If the target is an IronClaw/NEAR agent, use AITP via the bridge
4. If the target is an HTTP API requiring payment, use x402
5. If none match, fall back to direct HTTP

This routing is configured in `tiagent.toml` and can be overridden per-target.

---

## 8. Configuration

All protocol settings live under the `[protocols]` section of `tiagent.toml`.

### 8.1 Full Configuration Example

```toml
# ─── MCP ────────────────────────────────────────────────────────
[mcp]
auto_discover = true          # Scan $PATH for tiagent-mcp-* binaries

[mcp.servers.celestia]
command = "tiagent-mcp-celestia"
args = ["--network", "mocha"]
env = { CELESTIA_NODE_AUTH = "${CELESTIA_AUTH_TOKEN}" }

[mcp.servers.code-intel]
command = "tiagent-mcp-code"
args = ["--workspace", "."]

[mcp.servers.github]
command = "tiagent-mcp-github"
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

[mcp.server_mode]
enabled = true                # Expose tiagent as an MCP server
bind = "127.0.0.1:9100"
transport = "streamable-http"

# ─── A2A ────────────────────────────────────────────────────────
[protocols.a2a]
enabled = true
agent_card_path = "/.well-known/agent.json"
bind = "0.0.0.0:9200"

[protocols.a2a.discovery]
# Known agent directories to query for Agent Cards
directories = [
  "https://agents.example.com",
]
cache_ttl_secs = 3600         # Cache discovered Agent Cards for 1 hour

[protocols.a2a.authentication]
scheme = "bearer"
token_env = "A2A_AUTH_TOKEN"

# ─── AITP (via IronClaw bridge) ─────────────────────────────────
[protocols.aitp]
enabled = true
ironclaw_url = "http://localhost:8080"
ironclaw_api_key_env = "IRONCLAW_API_KEY"

[protocols.aitp.capabilities]
data_requests = true
payments = true
delegation = true
decisions = true
identity_verification = false  # Not yet implemented

# ─── x402 ───────────────────────────────────────────────────────
[protocols.x402]
enabled = true
max_per_request = "0.01"
max_per_session = "1.00"
max_per_day = "10.00"
currency = "TIA"
auto_approve_below = "0.001"

[protocols.x402.settlement]
default_chain = "celestia"
fallback_chain = "base"       # Use Base L2 for sub-cent payments
namespace = "x402-receipts"   # Celestia namespace for payment proofs
```

### 8.2 Enabling and Disabling Protocols

Each protocol can be independently enabled or disabled. When disabled, the
corresponding adapter is not loaded and no resources are allocated for it.

MCP is always available (it is the primary tool integration mechanism). The
other three protocols are opt-in.

### 8.3 Environment Variables

Secrets (API keys, auth tokens) are referenced via environment variable
interpolation (`${VAR_NAME}`) rather than stored directly in the config file.
tiagent resolves these at startup and refuses to start if required variables
are missing.

---

## Summary

tiagent integrates with four interoperability protocols to participate in the
broader agent ecosystem:

- **MCP** for tool integration (primary, always enabled)
- **A2A** for agent-to-agent collaboration and discovery
- **AITP** for transactional messaging via the IronClaw bridge
- **x402** for automatic API micropayments

All protocols are unified through a `ProtocolAdapter` layer that converts
external messages to and from tiagent's internal Signal representation. This
means the core agent loop does not need protocol-specific logic -- it operates
on Signals regardless of their origin. Protocol selection for outbound messages
is handled by routing rules in `tiagent.toml`.
