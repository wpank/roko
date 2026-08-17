# 06 — Tool Calling, MCP Integration, and Extensible Dev Tools

tiagent gives your agents the same tools a developer uses — file editing,
shell commands, code search, git operations — plus any MCP-compatible tool
server. Unlike Claude Code or Codex where the tool set is fixed, tiagent's
tool system is fully extensible. You can connect any MCP server to add
domain-specific capabilities, from database clients to blockchain tools to
internal APIs.

This document describes how tiagent agents interact with the outside world
through tools — the structured functions an LLM can invoke during a
conversation — and how the tool system is extended through MCP.

---

## 1. What is Tool Calling?

Large language models generate text. On their own, they cannot read files,
query APIs, submit transactions, or modify anything. They can only produce
the next token. Tool calling is the mechanism that bridges that gap.

**The basic cycle works like this:**

1. The harness sends the LLM a list of available tools, each described by a
   name, a natural-language description, and a JSON schema for its
   parameters.
2. The LLM, instead of producing plain text, outputs a structured tool
   request — a JSON object naming the tool and supplying arguments that
   conform to the schema.
3. The harness intercepts that request, validates the arguments, executes
   the tool (a real function — reading a file, calling an API, submitting a
   transaction), and captures the result.
4. The result is fed back to the LLM as a new message in the conversation.
5. The LLM decides whether to make another tool call or to produce a final
   text response.

This loop repeats until the model is satisfied or hits a guard limit.

Tool calling is what separates a chatbot from an agent. Without tools, the
model can reason about what to do. With tools, it can actually do it. The
quality of the tool set — how well tools are described, how reliably they
execute, and how tightly they map to the domain — determines the ceiling of
what the agent can accomplish.

---

## 2. MCP (Model Context Protocol)

### What it is

MCP is an open standard, originally created by Anthropic, for connecting
LLMs to external tools, data sources, and capabilities. It defines a
wire protocol so that any tool provider can expose tools in a way any
LLM harness can consume, without bespoke integration per provider.

As of mid-2026, the MCP SDK ecosystem sees roughly 97 million monthly
downloads across its JavaScript and Python packages. It has become the
de facto standard for LLM tool integration.

### Architecture

MCP defines three roles:

```
MCP Server          MCP Client          LLM
(exposes tools)  <-->  (harness)    <-->  (model)
```

- **MCP Server**: A process that exposes one or more tools. Each tool is
  described by a JSON schema. The server handles execution when a tool is
  called.
- **MCP Client**: The agent harness (tiagent, in our case). It connects to
  one or more MCP servers, discovers available tools, translates tool calls
  from the LLM into server requests, and returns results.
- **LLM**: The language model. It sees tool schemas as part of its context
  and generates structured tool-call outputs.

### Transport mechanisms

MCP supports three transport modes:

| Transport        | How it works                                    | When to use                    |
|------------------|-------------------------------------------------|--------------------------------|
| **stdio**        | Server runs as a subprocess; communication over stdin/stdout | Local tools, dev environments  |
| **SSE**          | Server exposes an HTTP endpoint with Server-Sent Events      | Remote servers, long-lived     |
| **Streamable-HTTP** | Newer HTTP-based transport with bidirectional streaming    | Production deployments         |

For tiagent, **stdio** is the primary transport. Celestia tools run as a
local subprocess (`tiagent-mcp-celestia`), communicating with the harness
over stdin/stdout. This avoids network overhead and keeps the trust
boundary tight — the tool process runs with the same permissions as the
agent.

### tiagent as both client and server

tiagent operates in two MCP roles:

- **As a client**: tiagent discovers and calls tools from external MCP
  servers. This is the normal mode — the agent uses tools.
- **As a server**: tiagent can expose its own capabilities (plan execution,
  knowledge queries, agent orchestration) as an MCP server. This allows
  other agents, IDEs, or tools to call into tiagent programmatically.

---

## 3. tiagent Tool Architecture

### Core components

The tool system has four main components:

```
ToolRegistry
    |
    v
HandlerResolver  --->  ToolDispatcher  --->  Execution
    ^                       |
    |                       v
Tool Sources            Result
(built-in, MCP,
 Celestia, plugin)
```

**ToolRegistry** is the central catalog of every tool available to the
agent at runtime. It holds the tool name, description, parameter schema,
return schema, and metadata (risk tier, source, aliases). Tools are
registered at startup and can be added or removed dynamically as MCP
servers connect or disconnect.

**HandlerResolver** maps a tool name to its concrete implementation. When
the LLM emits a tool call for `celestia_blob_submit`, the resolver looks
up which handler owns that name. Aliases are supported — if the model
outputs `submit_blob` instead, the resolver can map it to the canonical
name.

**ToolDispatcher** is the execution engine. It receives a validated tool
call, runs pre-execution checks (authorization, rate limiting, safety
policy), invokes the handler, captures the result or error, runs
post-execution checks, and returns the result to the conversation loop.

### Tool sources

Tools come from four places:

1. **Built-in tools** — Compiled into tiagent. These are general-purpose
   tools (file I/O, shell, search) that every agent needs regardless of
   domain.
2. **MCP tools** — Discovered dynamically from connected MCP servers. These
   are loaded at runtime when the harness connects to a server and calls
   `tools/list`.
3. **Celestia tools** — Native tools for interacting with the Celestia
   network. These are implemented in the `tiagent-mcp-celestia` server but
   are logically a first-class part of tiagent.
4. **Plugin tools** — User-provided tool packages installed via the plugin
   system. A plugin is an MCP server distributed as a binary or script.

---

## 4. Built-in Tool Set

These are the core tools that ship with tiagent. They are always
available, regardless of configuration, and they cover the same
operations a developer performs every day: reading and writing code,
running builds and tests, searching for patterns, and managing source
control. If you have used Claude Code, Cursor, or Codex, these will
feel familiar — the difference is that tiagent's set is open, not locked
to a vendor.

### File tools — read, write, and edit code

| Tool              | Description                                      |
|-------------------|--------------------------------------------------|
| `read_file`       | Read contents of a file, with optional line range |
| `write_file`      | Write content to a file (create or overwrite)     |
| `edit_file`       | Apply targeted edits to a file (find/replace)     |
| `list_directory`  | List files and directories at a path              |
| `search_files`    | Search file contents by regex pattern             |

These are the tools agents use most. An agent working on a bug fix will
`read_file` the relevant source, `edit_file` to apply a patch, and
`read_file` the test to confirm the change is correct — the same
workflow a human developer follows in an editor.

### Shell tools — run tests, build, deploy

| Tool      | Description                                          |
|-----------|------------------------------------------------------|
| `bash`    | Execute a bash command and return stdout/stderr       |
| `command` | Execute a command with structured arguments           |

Shell access is what turns an agent from a code reviewer into a
developer. The agent can run `cargo test`, `npm run build`,
`docker compose up`, `git diff`, or any other command. Gate pipelines
use shell tools to validate that code compiles, tests pass, and linters
are clean.

### Search tools — find relevant code

| Tool          | Description                                      |
|---------------|--------------------------------------------------|
| `grep`        | Search file contents with regex, return matches   |
| `glob`        | Find files by glob pattern                        |
| `web_search`  | Search the web and return results                 |

Before writing code, agents search for existing implementations,
patterns, and conventions. `grep` finds usages across a codebase,
`glob` locates files by naming convention, and `web_search` pulls in
documentation or examples from the wider ecosystem.

### Git tools — commit, diff, branch

| Tool          | Description                                      |
|---------------|--------------------------------------------------|
| `git_status`  | Show working tree status                         |
| `git_diff`    | Show staged and unstaged changes                 |
| `git_commit`  | Create a commit with a message                   |
| `git_log`     | Show recent commit history                       |

Agents that produce code need to manage source control. These tools
let an agent check what has changed, stage work, create commits, and
inspect history — the same git workflow a developer uses from the
command line.

### Agent tools — spawn sub-agents for parallel work

| Tool            | Description                                      |
|-----------------|--------------------------------------------------|
| `spawn_agent`   | Launch a sub-agent with a specific task           |
| `message_agent` | Send a message to a running agent                 |

Complex tasks benefit from decomposition. An agent can spawn sub-agents
to work on independent subtasks in parallel — one agent writes the
implementation while another writes the tests — then collect and
integrate results.

### Memory tools — persist learning across sessions

| Tool             | Description                                     |
|------------------|-------------------------------------------------|
| `store_memory`   | Persist a fact or insight to durable memory      |
| `recall_memory`  | Query memory for relevant stored information     |

Agents that learn from past runs store insights (which patterns work,
which configurations fail, what the codebase conventions are) and
recall them in future sessions. This is how agents improve over time
rather than starting from scratch each run.

Each built-in tool is implemented as a Rust function with a typed
parameter struct and a typed result struct. The JSON schema is derived
from the struct definition, so the schema and implementation are always
in sync.

---

## 5. Celestia-Native Tools

These tools are available when Celestia integration is enabled. They are
provided by the `tiagent-mcp-celestia` MCP server, which is an optional
add-on — tiagent works as a full-featured agent harness without it. When
connected, these tools give the agent direct, structured access to
Celestia's data availability layer and node operations. An agent
equipped with these tools can submit data, verify inclusion proofs,
manage namespaces, and interact with the Celestia network — all through
the same tool-calling interface it uses for everything else.

### Blob operations

#### `celestia_blob_submit`

Submit a blob of data to a Celestia namespace.

**Parameters:**

| Name        | Type     | Required | Description                              |
|-------------|----------|----------|------------------------------------------|
| `namespace` | string   | yes      | Namespace ID (hex-encoded, 29 bytes)     |
| `data`      | string   | yes      | Blob data (base64-encoded)               |
| `gas_price` | number   | no       | Gas price in utia (default: estimated)   |
| `signer`    | string   | no       | Key name to sign with (default: default) |

**Returns:**

```json
{
  "height": 142857,
  "commitment": "0xabc123...",
  "namespace": "0x...",
  "tx_hash": "0xdef456..."
}
```

**Example usage by the LLM:**

```json
{
  "tool": "celestia_blob_submit",
  "arguments": {
    "namespace": "0x746961676e65740000000000000000000000000000000000000000000000",
    "data": "eyJldmVudCI6ICJ0YXNrX2NvbXBsZXRlZCJ9"
  }
}
```

#### `celestia_blob_get`

Retrieve a specific blob by height, namespace, and commitment.

| Name         | Type   | Required | Description                |
|--------------|--------|----------|----------------------------|
| `height`     | number | yes      | Block height               |
| `namespace`  | string | yes      | Namespace ID (hex-encoded) |
| `commitment` | string | yes      | Blob commitment (hex)      |

Returns the blob data (base64), the namespace, and the share version.

#### `celestia_blob_get_all`

Retrieve all blobs in a namespace at a given height.

| Name        | Type   | Required | Description                |
|-------------|--------|----------|----------------------------|
| `height`    | number | yes      | Block height               |
| `namespace` | string | yes      | Namespace ID (hex-encoded) |

Returns an array of blobs, each with data, commitment, and share version.

#### `celestia_blob_get_proof`

Get a Namespaced Merkle Tree (NMT) inclusion proof for a blob.

| Name         | Type   | Required | Description                |
|--------------|--------|----------|----------------------------|
| `height`     | number | yes      | Block height               |
| `namespace`  | string | yes      | Namespace ID (hex-encoded) |
| `commitment` | string | yes      | Blob commitment (hex)      |

Returns the NMT proof structure: start index, end index, and the set of
sibling hashes needed to verify inclusion against the data root.

### Namespace operations

#### `celestia_namespace_create`

Register a namespace for use. This is a local operation — namespaces on
Celestia are permissionless and do not require on-chain registration. This
tool records the namespace in tiagent's local configuration for convenient
reuse.

| Name          | Type   | Required | Description                      |
|---------------|--------|----------|----------------------------------|
| `name`        | string | yes      | Human-readable label             |
| `namespace_id`| string | no       | Specific ID (hex); auto-generated if omitted |
| `version`     | number | no       | Namespace version (default: 0)   |

#### `celestia_namespace_list`

List all locally registered namespaces with their IDs and labels.
No parameters. Returns an array of namespace entries.

### Header and node operations

#### `celestia_header_get`

Get a block header by height.

| Name     | Type   | Required | Description                              |
|----------|--------|----------|------------------------------------------|
| `height` | number | no       | Block height (default: latest)           |

Returns the header: height, hash, time, data root, validator set hash,
and proposer address.

#### `celestia_node_info`

Query the local Celestia light node for its status.

No parameters. Returns: node type (light/bridge/full), network,
sync status (head height, synced height, sync percentage), peer count,
and uptime.

### Balance and transfer

#### `celestia_balance_get`

Check TIA balance for an address.

| Name      | Type   | Required | Description                            |
|-----------|--------|----------|----------------------------------------|
| `address` | string | no       | Celestia address (default: own node)   |

Returns balance in TIA and utia.

#### `celestia_transfer`

Transfer TIA tokens to another address. This is a financial operation
and is classified as a Dangerous-tier tool (see Section 9).

| Name       | Type   | Required | Description                          |
|------------|--------|----------|--------------------------------------|
| `to`       | string | yes      | Recipient address                    |
| `amount`   | string | yes      | Amount in TIA (e.g., "1.5")          |
| `gas_price`| number | no       | Gas price in utia                    |

Returns the transaction hash and final balance.

### Verification

#### `celestia_prove_inclusion`

Verify that a blob is included in a specific block using its NMT proof.

| Name         | Type   | Required | Description                    |
|--------------|--------|----------|--------------------------------|
| `height`     | number | yes      | Block height                   |
| `namespace`  | string | yes      | Namespace ID (hex-encoded)     |
| `commitment` | string | yes      | Blob commitment (hex)          |
| `data_root`  | string | no       | Expected data root (verifies header too) |

Returns a boolean `included` field and, if verification fails, a
`reason` string explaining what did not match.

---

## 6. Celestia Development Tools

These tools are for developers building applications on top of Celestia —
deploying rollups, compiling and deploying contracts, and working with
IBC. They are implemented in dedicated MCP servers or as extensions to
the `tiagent-mcp-celestia` server. Like the Celestia-native tools above,
these are optional and only available when the relevant MCP servers are
configured.

### Rollup operations

#### `rollup_deploy`

Deploy a new rollup that uses Celestia for data availability.

| Name         | Type   | Required | Description                              |
|--------------|--------|----------|------------------------------------------|
| `name`       | string | yes      | Rollup identifier                        |
| `framework`  | string | yes      | Framework: `rollkit`, `sovereign`, `op-stack`, `arbitrum-orbit` |
| `da_config`  | object | no       | DA configuration overrides               |
| `namespace`  | string | no       | Celestia namespace (auto-created if omitted) |

Returns deployment details: rollup ID, RPC endpoint, namespace, and
genesis block info.

#### `rollup_status`

Check the status of a deployed rollup.

| Name   | Type   | Required | Description       |
|--------|--------|----------|-------------------|
| `name` | string | yes      | Rollup identifier |

Returns: synced height, DA height, peer count, health status.

### Contract operations

#### `contract_compile`

Compile a smart contract for deployment to a rollup.

| Name       | Type   | Required | Description                              |
|------------|--------|----------|------------------------------------------|
| `path`     | string | yes      | Path to contract source                  |
| `language` | string | yes      | `cosmwasm` or `solidity`                 |
| `optimize` | bool   | no       | Optimize output (default: true)          |

Returns: compiled artifact path, code size, ABI path.

#### `contract_deploy`

Deploy a compiled contract to a rollup.

| Name       | Type   | Required | Description                              |
|------------|--------|----------|------------------------------------------|
| `artifact` | string | yes      | Path to compiled contract                |
| `rollup`   | string | yes      | Target rollup name                       |
| `init_msg` | object | no       | Instantiation message (CosmWasm) or constructor args (Solidity) |

Returns: contract address, transaction hash, code ID (for CosmWasm).

### IBC operations

#### `ibc_transfer`

Initiate an IBC token transfer between chains.

| Name          | Type   | Required | Description                          |
|---------------|--------|----------|--------------------------------------|
| `source`      | string | yes      | Source chain/rollup                  |
| `destination` | string | yes      | Destination chain/rollup             |
| `token`       | string | yes      | Token denomination                   |
| `amount`      | string | yes      | Amount to transfer                   |
| `channel`     | string | no       | IBC channel (auto-resolved if known) |

Returns: packet sequence, source tx hash, timeout height.

#### `ibc_channel_query`

Query IBC channels for a chain.

| Name    | Type   | Required | Description                       |
|---------|--------|----------|-----------------------------------|
| `chain` | string | yes      | Chain/rollup to query             |
| `state` | string | no       | Filter by state: `open`, `closed` |

Returns an array of channels with port, channel ID, counterparty,
connection, and state.

---

## 7. General Development Workflow

Most tiagent usage involves no blockchain tools at all. Here is a
typical interaction for everyday software development:

**Developer runs:**

```bash
tiagent run "add user authentication with JWT"
```

**What the agent does, using built-in tools:**

1. **Reads existing code** — `read_file` on route handlers, models, and
   middleware to understand the current architecture.
2. **Searches for patterns** — `grep` for existing auth references,
   `glob` to find test files, `web_search` for JWT best practices.
3. **Writes new code** — `edit_file` to add auth middleware,
   `write_file` for new modules (JWT validation, user model, login
   route).
4. **Runs tests** — `bash` to execute `cargo test` or `npm test` and
   verify the new code works.
5. **Checks compilation and lint** — `bash` to run the compiler and
   linter, fixing any issues the tools surface.

**Gate pipeline validates automatically:**

- Compile gate: does it build? Yes.
- Test gate: do all tests pass? Yes.
- Lint gate: is the code clean? Yes.

The entire workflow uses file tools, shell tools, and search tools. No
MCP servers need to be configured. No blockchain tools are involved.
This is the default tiagent experience for most developers.

---

## 8. Tool Discovery and Configuration

### Static configuration

Tools are configured in `tiagent.toml` under the `[tools]` section.

A minimal configuration needs nothing more than a code intelligence
server:

```toml
[tools.mcp]
servers = [
  { name = "code", command = "tiagent-mcp-code" },
]
```

This gives the agent built-in tools plus code intelligence (AST-aware
search, symbol lookup, cross-reference navigation). For most software
projects, this is sufficient.

To add more capabilities, add more MCP servers. For example, to add
GitHub integration and Celestia tools:

```toml
[tools.mcp]
servers = [
  { name = "code", command = "tiagent-mcp-code" },
  { name = "github", command = "tiagent-mcp-github", env = { GITHUB_TOKEN = "$GITHUB_TOKEN" } },
  { name = "celestia", command = "tiagent-mcp-celestia", args = ["--node", "http://localhost:26658"] },
]
```

Each server entry specifies:

- `name` — A label for the server (used in logs and tool namespacing)
- `command` — The binary to execute
- `args` — Arguments passed to the binary
- `env` — Environment variables (supports `$VAR` expansion from the shell)

When tiagent starts, it launches each configured MCP server as a
subprocess, connects over stdio, calls `tools/list` to discover available
tools, and registers them in the `ToolRegistry`.

### Auto-discovery

In addition to explicit configuration, tiagent scans the system PATH for
binaries matching the pattern `tiagent-mcp-*`. Any matching binary is
treated as a potential MCP server. tiagent launches it, attempts a
handshake, and if the handshake succeeds, registers the server's tools.

This means installing a tiagent MCP plugin can be as simple as placing a
binary named `tiagent-mcp-postgres` on the PATH. No configuration file
changes required.

Auto-discovered servers run with default arguments. To pass specific
arguments (like a node URL or credentials), use the explicit config in
`tiagent.toml`.

### Tool aliasing

Different MCP servers may expose tools under different names. The LLM
might also use variations of a tool name. tiagent supports aliasing:

```toml
[tools.aliases]
submit_blob = "celestia_blob_submit"
get_blob = "celestia_blob_get"
run_command = "bash"
```

The `HandlerResolver` checks aliases before reporting a tool as unknown.
This prevents failures when the model uses a plausible but non-canonical
name.

### Namespacing

When multiple MCP servers are connected, tool names could collide. tiagent
prefixes each tool with its server name when a collision is detected:

- Server "celestia" exposes `submit` and server "rollup" exposes `submit`
- Both are registered as `celestia.submit` and `rollup.submit`
- If there is no collision, the bare name `submit` also works

The LLM sees the fully qualified names in its tool list, but aliases
allow shorthand use.

---

## 9. Safety and Authorization

Tool calling without safety controls is dangerous. An agent that can
execute shell commands and transfer tokens needs guardrails.

### Risk tiers

Every tool is assigned a risk tier:

| Tier          | Description                                     | Examples                                   |
|---------------|-------------------------------------------------|--------------------------------------------|
| **Safe**      | Read-only, no side effects                      | `read_file`, `celestia_blob_get`, `glob`   |
| **Moderate**  | Local side effects (file writes, process spawn) | `write_file`, `bash`, `contract_compile`   |
| **Dangerous** | Network/financial side effects, irreversible    | `celestia_transfer`, `contract_deploy`, `celestia_blob_submit` |

Risk tiers are declared in the tool's metadata when it is registered.

### Policy-based authorization

Before every tool call, the `ToolDispatcher` checks it against the active
`Policy`. The policy is a trait with a single method:

```
fn authorize(tool_name, arguments, context) -> Allow | Deny(reason) | Confirm(prompt)
```

The three outcomes:

- **Allow** — Execute immediately.
- **Deny** — Block execution. The LLM receives an error message explaining
  why the call was denied.
- **Confirm** — Pause execution and present the user with a confirmation
  prompt. The user sees the tool name, arguments, and a description of the
  risk. They approve or reject.

The default policy maps directly from risk tiers:

- Safe tools: Allow
- Moderate tools: Allow (but logged)
- Dangerous tools: Confirm

Users can override the default policy in `tiagent.toml`:

```toml
[tools.policy]
# Auto-approve everything (use only in CI/automated pipelines)
mode = "auto-approve"

# Or set per-tool overrides
[tools.policy.overrides]
celestia_transfer = "confirm"
bash = "confirm"
celestia_blob_submit = "allow"
```

### Audit logging

Every tool call — whether allowed, denied, or confirmed — is logged to
the agent's episode log. Each entry records:

- Timestamp
- Tool name and arguments
- Risk tier
- Authorization decision (allow/deny/confirm + user response)
- Execution result (success/error)
- Execution duration

This provides a complete audit trail of everything the agent did.

---

## 10. Tool Loop

The tool loop is the core execution cycle of the agent. It is the process
by which the agent iteratively uses tools to accomplish a task.

### Flow

```
User prompt
    |
    v
[Build messages: system prompt + conversation history + tool schemas]
    |
    v
[Send to LLM] <------------------------------------+
    |                                                |
    v                                                |
[Parse LLM response]                                 |
    |                                                |
    +---> Text response? ---> Done (return to user)  |
    |                                                |
    +---> Tool call(s)?                              |
              |                                      |
              v                                      |
         [Validate arguments against schema]         |
              |                                      |
              v                                      |
         [Check authorization (Policy)]              |
              |                                      |
              +---> Denied? ---> Error message ------+
              |                                      |
              +---> Confirmed? ---> Ask user         |
              |         |                            |
              |         +---> Rejected ---> Error ---+
              |         |                            |
              |         +---> Approved               |
              |                  |                   |
              v                  v                   |
         [Execute tool(s)]                           |
              |                                      |
              v                                      |
         [Capture result or error]                   |
              |                                      |
              v                                      |
         [Append tool result to conversation] -------+
```

### Parallel execution

When the LLM returns multiple tool calls in a single response, tiagent
checks whether they are independent (no data dependencies between them).
Independent calls are executed concurrently using async tasks. Dependent
calls are executed sequentially in the order returned.

This matters for performance. An agent that needs to read five files can
do so in parallel rather than one at a time, reducing wall-clock latency
by up to 5x.

### Iteration limits

To prevent runaway agents, the tool loop enforces a maximum iteration
count. The default is 50 iterations per task. If the agent exceeds this
limit, the loop terminates and returns an error to the caller.

The limit is configurable:

```toml
[agent]
max_tool_iterations = 100
```

This is a hard stop. It does not matter how close the agent is to
finishing — if it hits the limit, it stops. This prevents resource
exhaustion from agents stuck in loops.

### Error handling

When a tool call fails (network error, invalid arguments, permission
denied), the error is formatted as a structured message and fed back to
the LLM as the tool's result. The LLM then decides how to proceed. It
might:

- Retry with corrected arguments
- Try a different approach
- Report the error to the user and ask for guidance
- Give up and return a partial result

The harness does not automatically retry failed tool calls. Retry logic
is the LLM's responsibility — it has the context to decide whether a
retry makes sense and how to adjust the approach.

### Timeout

Individual tool calls have a configurable timeout (default: 120 seconds).
Long-running operations like contract compilation or rollup deployment
get extended timeouts:

```toml
[tools.timeouts]
default = 120
contract_compile = 600
rollup_deploy = 900
```

If a tool call exceeds its timeout, the harness kills the operation and
returns a timeout error to the LLM.

---

## Summary

The tool system is tiagent's hands. The LLM provides reasoning; tools
provide action. The built-in tool set — file operations, shell commands,
code search, git, agent spawning, memory — covers the same workflow a
developer follows every day. Most tiagent users will never need anything
beyond these tools and a code intelligence MCP server.

MCP gives us a standard protocol for extending beyond that baseline.
tiagent can consume tools from any MCP-compatible server and expose its
own capabilities to other systems. When Celestia integration is enabled,
the `tiagent-mcp-celestia` server adds first-class access to data
availability, namespace management, and inclusion verification. The
Celestia development tools (`rollup_deploy`, `contract_compile`,
`contract_deploy`, `ibc_transfer`) extend that further into the builder
workflow.

Safety is enforced at the dispatch layer through risk tiers, policy-based
authorization, and confirmation prompts. Every tool call is logged. The
iteration limit and timeout system prevent runaway execution. These
controls apply uniformly — built-in tools, MCP tools, and Celestia tools
all go through the same authorization and audit path.
