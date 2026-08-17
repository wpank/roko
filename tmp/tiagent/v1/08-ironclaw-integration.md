# 08 --- IronClaw Agent Runtime Integration

> **This document describes an optional integration.** tiagent works as a fully
> functional coding agent without IronClaw. IronClaw adds WASM sandboxing for
> untrusted tool execution and TEE support for verifiable agent computation.

tiagent is a self-improving agent harness for Celestia, the modular blockchain
network. This document describes how tiagent integrates with IronClaw --- an
open-source Rust framework for building and deploying AI agents, developed by
NEAR AI. IronClaw provides capabilities that tiagent needs but should not
reimplement: WASM-sandboxed tool execution, hardware-backed trusted execution
environments (TEEs), and battle-tested agent communication protocols.

This document is written for a reader encountering both tiagent and IronClaw for
the first time. It explains what IronClaw is, why tiagent integrates with it,
how the integration works at a technical level, and what the implementation plan
looks like.

### Why a developer might care

You do not need to care about blockchain to benefit from this integration.
Regular developers benefit from WASM sandboxing whenever they run untrusted MCP
tools or community-contributed tool packages --- it is the same concept as
running npm packages in a sandbox. If you install a third-party MCP server and
want to guarantee it cannot read your SSH keys, exfiltrate source code, or make
network requests to arbitrary endpoints, IronClaw's WASM sandbox provides that
isolation. The TEE and cross-chain protocol features (sections 5 and 6) are
relevant for on-chain use cases and can be ignored entirely if you are using
tiagent as a coding agent.

If you have not read the preceding documents:

- **01-vision-and-overview.md** explains what tiagent is: a Rust toolkit for
  building self-improving AI agents on the Celestia blockchain ecosystem.
- **02-architecture.md** explains the core abstractions: one noun (Signal), six
  verb traits (Substrate, Scorer, Gate, Router, Composer, Policy), and a
  universal loop (query, score, route, compose, act, verify, write, react).
- **06-tool-system.md** explains how tiagent agents call tools and integrate
  with MCP servers.

---

## Table of Contents

1. [What is IronClaw?](#1-what-is-ironclaw)
2. [Why Integrate with IronClaw?](#2-why-integrate-with-ironclaw)
3. [Integration Modes](#3-integration-modes)
4. [WASM Tool Sandboxing](#4-wasm-tool-sandboxing)
5. [TEE Integration](#5-tee-integration)
6. [Protocol Support: ACP and AITP](#6-protocol-support-acp-and-aitp)
7. [Implementation Plan](#7-implementation-plan)

---

## 1. What is IronClaw?

### 1.1 Overview

IronClaw is an open-source Rust project maintained by NEAR AI. It describes
itself as an "Agent OS" --- not just a library for building agents, but a full
runtime environment for managing agent lifecycles, executing tools safely, and
communicating across trust boundaries. The project repository is at
[github.com/nearai/ironclaw](https://github.com/nearai/ironclaw). As of
mid-2026, it has over 14,000 GitHub stars and ships weekly releases (v0.29.x at
the time of writing).

IronClaw is the security-hardened counterpart to OpenClaw (NEAR AI's
accessibility-focused agent framework). Where OpenClaw prioritizes rapid
prototyping and ease of use, IronClaw is built from the ground up around a
security-first architecture: credentials never touch the LLM, untrusted tools
run inside WASM sandboxes, and production deployments can run inside
hardware-isolated Trusted Execution Environments.

### 1.2 Architecture

IronClaw ships as a single Rust binary. When started with `ironclaw serve`, it
launches a local web interface where users interact with agents, manage
projects, and configure settings. Under the hood, the system has several layers:

```
+---------------------------------------------------------------+
|                       IronClaw Binary                         |
+---------------------------------------------------------------+
|  Channels (REPL, HTTP webhooks, Web UI, SSE/WebSocket)        |
+---------------------------------------------------------------+
|  Agent Loop (prompt -> LLM -> tool calls -> result -> repeat) |
+---------------------------------------------------------------+
|  Tool Dispatch                                                |
|  +-------------------+  +------------------+  +-----------+   |
|  | Native Rust tools |  | WASM sandboxed   |  | MCP       |   |
|  | (full host access)|  | tools (isolated) |  | servers   |   |
|  +-------------------+  +------------------+  +-----------+   |
+---------------------------------------------------------------+
|  Security Layer                                               |
|  +----------------+  +--------------+  +------------------+   |
|  | Credential     |  | Prompt       |  | Network          |   |
|  | vault (AES-256)|  | injection    |  | allowlisting     |   |
|  |                |  | defense      |  |                  |   |
|  +----------------+  +--------------+  +------------------+   |
+---------------------------------------------------------------+
|  Storage (PostgreSQL or LibSQL, hybrid full-text + vector)    |
+---------------------------------------------------------------+
```

The workspace is organized into approximately 14 Rust crates. The ones most
relevant to tiagent integration are:

| Crate | What it does |
|-------|-------------|
| `ironclaw_engine` | The v2 execution model: agent loop, tool dispatch, state management |
| `ironclaw_wasm` | WASM sandbox runtime, capability system, resource limiting |
| `ironclaw_safety` | Pattern detection, content sanitization, output redaction |
| `ironclaw_common` | Shared types, configuration, error handling |
| `ironclaw_cli` | The binary entry point, channel setup, server lifecycle |

### 1.3 Key Features

**WASM sandboxing.** IronClaw executes untrusted tools inside isolated
WebAssembly containers using the Wasmtime runtime. Each tool invocation creates
a fresh WASM instance with explicit, capability-based permissions. A tool can
only access the filesystem paths, network domains, and credentials that are
explicitly granted to it. When a tool finishes, its instance is discarded ---
no state persists between invocations. This provides defense-in-depth: even if
a tool is malicious, it cannot read agent secrets, access the host filesystem,
or exfiltrate data to unauthorized endpoints.

**Credential protection.** Secrets are encrypted at rest with AES-256-GCM, keys
stored in the OS keychain. When a tool needs a credential (for example, an API
key to call a service), the credential is injected at the host boundary
immediately before the HTTP request leaves the system. The WASM guest code never
sees the raw secret. After use, credential material is zeroized in memory.

**Trusted Execution Environments (TEEs).** When deployed on NEAR AI Cloud,
IronClaw instances run inside hardware-isolated TEEs. The TEE encrypts all data
from the moment the instance boots. Every inference produces a cryptographic
attestation --- a hardware-signed certificate proving that specific code ran on
specific data inside the enclave. Third parties can independently verify these
attestations without trusting the cloud provider.

**Protocol support.** IronClaw implements MCP (Model Context Protocol) for tool
integration, ACP (Agent Client Protocol) for IDE-to-agent communication and
agent-to-agent delegation, and has growing support for AITP (Agent Interaction
and Transaction Protocol) for structured cross-agent transactions.

---

## 2. Why Integrate with IronClaw?

tiagent is an agent *harness* --- it orchestrates the loop of prompting LLMs,
calling tools, validating results, and persisting state. It does not need to
reimplement everything from scratch. IronClaw provides four capabilities that
tiagent would otherwise need to build:

### 2.1 WASM sandboxing solves tool security

tiagent's tool system (described in **06-tool-system.md**) currently trusts all
tools to behave correctly. A tool that reads files, for example, can read any
file the process has access to. A tool that makes HTTP requests can call any
endpoint. This is acceptable during development but unacceptable in production,
especially when running untrusted third-party tools.

IronClaw's WASM sandbox provides exactly the isolation tiagent needs:
capability-based permissions, network allowlisting, filesystem scoping, and
resource limits. Rather than building a sandbox from scratch, tiagent can
delegate untrusted tool execution to IronClaw.

### 2.2 TEE support enables verifiable agent execution

tiagent publishes agent traces and state to Celestia's DA layer (described in
**04-celestia-integration.md** and **05-da-storage-patterns.md**). Today, anyone
reading those traces must trust that the agent actually ran the code it claims
to have run. There is no cryptographic proof of execution.

IronClaw's TEE support closes that gap. If an agent runs inside a TEE, the
hardware produces an attestation certificate for every computation. tiagent can
attach that attestation to the blob it publishes to Celestia. Now anyone reading
the trace can verify --- without trusting tiagent, the agent operator, or the
cloud provider --- that the stated computation actually happened.

This is particularly important for on-chain use cases: a smart contract or
rollup could validate the TEE attestation before accepting an agent's output.

### 2.3 ACP and AITP provide battle-tested protocols

tiagent supports multiple agent communication protocols (described in
**09-interop-protocols.md**): MCP, A2A, AITP, and x402. Implementing these
protocols correctly --- especially the security and session management layers
--- is substantial engineering work.

IronClaw already has working implementations of MCP and ACP, with growing AITP
support. By integrating with IronClaw, tiagent gets tested protocol
implementations rather than building from specification documents.

### 2.4 Avoiding the "built but never connected" trap

The roko codebase (tiagent's predecessor) has a documented pattern of building
capabilities that never get wired into the runtime. IronClaw is a running
system with weekly releases and thousands of users. Integrating with it means
tiagent gets battle-tested infrastructure rather than yet another internal
implementation that may or may not get connected.

---

## 3. Integration Modes

There are three ways tiagent can integrate with IronClaw. They are not mutually
exclusive --- each mode addresses a different need, and a mature deployment
would use all three.

### 3.1 Mode A: IronClaw as Tool Sandbox (relevant to all developers)

```
+---------------------------------------------------+
|  tiagent (orchestrator)                           |
|                                                   |
|  Agent Loop: prompt -> LLM -> tool calls          |
|                   |                               |
|                   v                               |
|  Tool Dispatch:                                   |
|  +------------------+  +----------------------+   |
|  | Trusted tools    |  | IronClaw WASM runner |   |
|  | (native, in-     |  | (untrusted tools     |   |
|  |  process)        |  |  sandboxed via WASM) |   |
|  +------------------+  +----------------------+   |
|                              |                    |
|                              v                    |
|                         IronClaw process          |
|                         (wasmtime runtime)        |
+---------------------------------------------------+
```

In this mode, tiagent remains the orchestrator. It runs its own agent loop,
manages its own state, and handles its own LLM dispatch. When the agent loop
produces a tool call, the tool dispatcher checks whether the target tool is
trusted (native) or untrusted (sandboxed). Untrusted tools are forwarded to an
IronClaw process that executes them in a WASM sandbox and returns the result.

**This is the recommended starting point** and the mode most relevant to general
development use. It requires the least integration work and provides the most
immediate benefit: security isolation for tool execution. Any developer running
third-party MCP tools or community tool packages benefits from this mode,
regardless of whether they use any blockchain features.

### 3.2 Mode B: IronClaw as Agent Runtime (on-chain / production deployments)

```
+---------------------------------------------+
|  IronClaw (runtime)                         |
|                                             |
|  Agent Loop (IronClaw's engine)             |
|  System Prompt (tiagent-generated)          |
|  Tools (IronClaw-managed)                   |
|  State (IronClaw PostgreSQL/LibSQL)         |
|                                             |
|  +---------------------------------------+  |
|  | tiagent plugin                        |  |
|  | - Celestia DA writes                  |  |
|  | - TraceCommons scoring                |  |
|  | - Self-improvement loop               |  |
|  +---------------------------------------+  |
+---------------------------------------------+
```

In this mode, tiagent agents deploy into IronClaw's managed runtime. IronClaw
handles the agent loop, tool dispatch, and state persistence. tiagent injects
its Celestia-specific capabilities (DA writes, trace scoring, self-improvement)
as IronClaw plugins.

This mode provides the full security surface (WASM sandboxing, credential
protection, TEE execution) but requires deeper integration. It is appropriate
for production deployments where security and verifiability are paramount.

### 3.3 Mode C: Protocol Bridge (cross-ecosystem / on-chain use cases)

```
+-------------------+         +--------------------+
|  tiagent agent A  |  ACP/   |  IronClaw agent B  |
|  (Celestia-native)|  AITP   |  (NEAR-native)     |
|                   | <-----> |                    |
|  Publishes to     |         |  Publishes to      |
|  Celestia DA      |         |  NEAR              |
+-------------------+         +--------------------+
```

In this mode, tiagent uses IronClaw's ACP and AITP implementations to
communicate with agents running in other runtimes. A tiagent agent on
Celestia can delegate tasks to an IronClaw agent on NEAR (or vice versa),
exchange structured data, and even conduct financial transactions through AITP's
payment capabilities.

This mode does not require running tiagent inside IronClaw or vice versa. It
only requires that both systems speak the same protocols.

### 3.4 Recommendation

Start with **Mode A** (tool sandbox). It provides immediate security benefits
with minimal coupling. Expand to **Mode C** (protocol bridge) when cross-runtime
agent communication is needed. Adopt **Mode B** (full runtime) for production
deployments that require TEE attestation.

---

## 4. WASM Tool Sandboxing

This section covers the technical details of Mode A: using IronClaw's WASM
sandbox as a tool execution backend for tiagent.

### 4.1 How IronClaw's WASM Sandbox Works

IronClaw uses the Wasmtime WebAssembly runtime to execute tools in isolated
containers. The execution model follows a "compile once at registration,
instantiate fresh per execution" lifecycle:

1. **Registration**: When a WASM tool is installed, IronClaw compiles the
   `.wasm` binary into machine code once and caches the compiled module.
2. **Invocation**: Each time the tool is called, IronClaw creates a fresh
   Wasmtime `Store` with its own `ToolStoreData`. This store is the tool's
   entire world --- it cannot see anything outside of it.
3. **Execution**: The tool runs with a fuel budget (default: 100,000,000 units,
   roughly 1 CPU-second). If it exceeds the budget, Wasmtime traps the
   execution with `TrapCode::OutOfFuel`.
4. **Cleanup**: After the tool returns, the store is discarded. All memory is
   freed. Any staged credentials are zeroized.

### 4.2 Capability-Based Permissions

Each WASM tool has a capabilities file (`{tool_name}.capabilities.json`) that
defines exactly what the tool is allowed to do. The permission model is
deny-by-default --- anything not explicitly granted is forbidden.

```json
{
  "name": "celestia-blob-reader",
  "permissions": {
    "network": {
      "allow_domains": ["celestia-node.example.com"],
      "allow_methods": ["GET", "POST"]
    },
    "filesystem": {
      "workspace_read": ["/workspace/data/*.json"]
    },
    "credentials": {
      "allow": ["CELESTIA_AUTH_TOKEN"]
    }
  },
  "resource_limits": {
    "max_fuel": 100000000,
    "max_memory_bytes": 67108864,
    "max_output_bytes": 1048576
  }
}
```

The capability categories and what they control:

| Category | What it controls | Default (no grant) |
|----------|------------------|--------------------|
| `network.allow_domains` | Which hosts the tool can make HTTP requests to | No network access |
| `network.allow_methods` | Which HTTP methods are permitted | None |
| `filesystem.workspace_read` | Which paths the tool can read (glob patterns) | No filesystem access |
| `credentials.allow` | Which secrets can be injected into outbound requests | No credentials |
| `resource_limits.max_fuel` | CPU budget (Wasmtime fuel units) | 100,000,000 |
| `resource_limits.max_memory_bytes` | Maximum heap size | 64 MB |
| `resource_limits.max_output_bytes` | Maximum size of tool output | 1 MB |

### 4.3 Credential Injection Without Exposure

One of IronClaw's most important security properties is that WASM tools never
see raw credentials. The injection flow works like this:

```
Tool says:              "I need to call celestia-node.example.com"
                             |
                             v
Host checks:            Is this domain in allow_domains?
                             |  yes
                             v
Host stages credential: RuntimeSecretInjectionStore.stage(
                          scope: "celestia-node.example.com",
                          capability_id: "CELESTIA_AUTH_TOKEN",
                          ttl: 5 minutes
                        )
                             |
                             v
Tool makes HTTP request: GET /blob/...
                             |
                             v
Host egress proxy:      Intercepts request, injects Authorization header
                        from staged credential, forwards to destination
                             |
                             v
After use:              Credential material is zeroized in memory
```

The WASM guest code constructs the HTTP request without any authentication
headers. The host intercepts the request at the egress boundary, injects the
credential, and forwards it. The tool never handles, logs, or even sees the raw
secret.

### 4.4 tiagent Tool Dispatch to IronClaw

Here is how tiagent's tool dispatcher routes a call to IronClaw's WASM sandbox.
The `IronClawToolRunner` implements tiagent's `ToolExecutor` trait:

```rust
/// A tool executor that delegates to IronClaw's WASM sandbox.
/// Untrusted tools are compiled to .wasm and executed in isolation.
pub struct IronClawToolRunner {
    /// HTTP client for communicating with the IronClaw process.
    client: reqwest::Client,
    /// Base URL of the running IronClaw instance.
    base_url: Url,
}

impl ToolExecutor for IronClawToolRunner {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        // 1. Build the tool invocation request
        let request = IronClawToolRequest {
            tool: tool_name.to_string(),
            arguments,
        };

        // 2. POST to IronClaw's tool execution endpoint
        let response = self.client
            .post(self.base_url.join("/api/tools/execute")?)
            .json(&request)
            .send()
            .await?;

        // 3. Parse the sandboxed execution result
        let result: IronClawToolResponse = response.json().await?;

        // 4. Map back to tiagent's ToolResult type
        Ok(ToolResult {
            content: result.output,
            is_error: result.error.is_some(),
            error_message: result.error,
        })
    }
}
```

tiagent's tool dispatcher selects between native execution and sandboxed
execution based on tool metadata:

```rust
pub async fn dispatch_tool_call(
    &self,
    tool_call: &ToolCall,
) -> Result<ToolResult, ToolError> {
    let tool_meta = self.registry.get(&tool_call.name)?;

    match tool_meta.execution_mode {
        // Trusted tools run in-process with full host access
        ExecutionMode::Native => {
            self.native_executor.execute(
                &tool_call.name,
                tool_call.arguments.clone(),
            ).await
        }
        // Untrusted tools are sandboxed via IronClaw WASM
        ExecutionMode::WasmSandboxed => {
            self.ironclaw_runner.execute(
                &tool_call.name,
                tool_call.arguments.clone(),
            ).await
        }
    }
}
```

### 4.5 Security Benefits Summary

| Threat | Without IronClaw | With IronClaw WASM sandbox |
|--------|------------------|---------------------------|
| Malicious tool reads agent secrets | Tool has full process memory access | WASM isolation: tool sees only its own store |
| Tool exfiltrates data to attacker server | Tool can make arbitrary HTTP requests | Network allowlisting: only approved domains |
| Tool consumes unbounded CPU | No limit; agent hangs | Fuel metering: execution traps at budget |
| Tool exhausts memory | Process-level OOM | Per-instance memory ceiling (default 64 MB) |
| Tool persists state between calls | Shared process memory | Fresh instance per invocation; no carryover |
| API key leaked in tool logs | Tool sees raw credentials | Credentials injected at egress; tool never handles them |

---

## 5. TEE Integration

### 5.1 What are Trusted Execution Environments?

A Trusted Execution Environment (TEE) is a hardware feature provided by modern
CPUs that creates an isolated region of memory and computation. Code running
inside a TEE is protected from everything outside it --- including the operating
system, the hypervisor, other processes, and even the physical machine operator.

The major TEE implementations are:

| Technology | Vendor | Isolation unit | Key property |
|------------|--------|----------------|-------------|
| Intel SGX | Intel | Enclave (application-level) | Code + data encrypted in memory; attestable |
| Intel TDX | Intel | VM (virtual machine-level) | Full VM isolation; no host OS trust needed |
| AMD SEV-SNP | AMD | VM (virtual machine-level) | Memory encryption with integrity; nested paging protection |
| ARM CCA | ARM | Realm (VM-level) | Hardware-enforced realm isolation |

The common property is **remote attestation**: the CPU can produce a
cryptographic certificate proving:

1. What code was loaded into the enclave/VM
2. That it was not tampered with
3. That the enclave/VM is running on genuine hardware

A third party can verify this certificate without trusting the machine operator,
the cloud provider, or the software stack outside the TEE.

### 5.2 Why TEE Matters for tiagent

tiagent publishes agent execution traces to Celestia's DA layer. These traces
are valuable for shared learning --- other agents can read them and learn from
prior runs. But there is a trust problem: how does a consuming agent know that
the trace is authentic? How does it know the publishing agent actually ran the
code it claims to have run, with the inputs it claims to have used?

Without TEE, the answer is "you trust the publisher." With TEE, the answer is
"you verify the attestation."

The pattern is:

```
+----------------------------------------------+
|  TEE Enclave                                 |
|                                              |
|  1. Load tiagent + agent code                |
|  2. Run agent loop (prompt, tool calls, etc) |
|  3. Produce execution trace                  |
|  4. Sign trace with enclave key              |
|  5. Generate attestation certificate         |
|                                              |
+----------------------------------------------+
         |
         v
  Attestation = {
    enclave_measurement: "sha256:abc...",  // hash of loaded code
    trace_hash: "sha256:def...",           // hash of execution trace
    hardware_signature: "...",             // CPU-signed proof
    timestamp: "2026-08-13T10:30:00Z"
  }
         |
         v
  Celestia DA blob = {
    namespace: "tiagent/traces",
    data: {
      trace: <execution trace>,
      attestation: <TEE attestation>
    }
  }
```

A consumer reads the blob from Celestia and verifies:

1. The `hardware_signature` is valid (signed by genuine TEE hardware)
2. The `enclave_measurement` matches the expected agent code
3. The `trace_hash` matches the hash of the included trace data

If all three checks pass, the consumer knows the trace is authentic --- it was
produced by the stated code on the stated inputs, inside hardware isolation.

### 5.3 IronClaw TEE Deployment

IronClaw supports TEE execution when deployed on NEAR AI Cloud. The deployment
model works as follows:

1. The IronClaw instance boots inside a hardware-isolated enclave (Intel TDX or
   AMD SEV-SNP, depending on the cloud region).
2. Data encryption begins at boot --- there is no unencrypted phase.
3. Every inference request produces a cryptographic attestation.
4. The attestation is a hardware-signed certificate that any third party can
   independently verify.

For tiagent, this means: deploy a tiagent agent inside an IronClaw TEE instance,
and every tool call, LLM interaction, and state transition is attestable. The
attestation can be attached to Celestia DA blobs as proof of authentic execution.

### 5.4 Attestation-to-DA Pipeline

```rust
/// Wraps an execution trace with its TEE attestation
/// before publishing to Celestia.
pub struct AttestedTrace {
    /// The raw execution trace (tool calls, LLM responses, etc.)
    pub trace: ExecutionTrace,
    /// TEE attestation proving the trace was produced in a secure enclave
    pub attestation: TeeAttestation,
}

pub struct TeeAttestation {
    /// Hash of the code loaded into the enclave
    pub enclave_measurement: String,
    /// Hash of the execution trace data
    pub trace_hash: String,
    /// Hardware-signed proof (CPU-specific format)
    pub hardware_signature: Vec<u8>,
    /// When the attestation was produced
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AttestedTrace {
    /// Serialize and submit to Celestia DA as a blob.
    pub async fn publish_to_celestia(
        &self,
        celestia: &CelestiaClient,
        namespace: &Namespace,
    ) -> Result<BlobCommitment, CelestiaError> {
        let blob_data = serde_json::to_vec(self)?;
        celestia.submit_blob(namespace, &blob_data).await
    }

    /// Verify the attestation against known enclave measurements.
    pub fn verify(&self, trusted_measurements: &[String]) -> bool {
        // 1. Check that the enclave measurement is in the trusted set
        if !trusted_measurements.contains(&self.enclave_measurement) {
            return false;
        }

        // 2. Verify the trace hash matches the actual trace
        let computed_hash = sha256(&serde_json::to_vec(&self.trace).unwrap());
        if computed_hash != self.trace_hash {
            return false;
        }

        // 3. Verify the hardware signature (CPU-specific verification)
        verify_tee_signature(&self.hardware_signature, &self.trace_hash)
    }
}
```

---

## 6. Protocol Support: ACP and AITP

### 6.1 ACP --- Agent Client Protocol

ACP is a protocol for structured communication between editors (or other
clients) and coding agents. It is built as an extension of JSON-RPC 2.0.
Originally designed for IDE-to-agent communication (for example, VS Code
controlling a coding agent), ACP has expanded to support agent-to-agent
delegation as well.

IronClaw implements ACP for delegating jobs to external agents. When IronClaw
receives a task it cannot handle locally, it can spawn an ACP-compliant agent as
a subprocess and communicate via the protocol. This works bidirectionally ---
IronClaw can also act as an ACP server, accepting tasks from external clients.

**How tiagent uses ACP:**

tiagent can use IronClaw's ACP implementation to delegate specialized tasks to
IronClaw-hosted agents. For example, a tiagent agent running a self-improvement
cycle might delegate a code refactoring subtask to an IronClaw agent that has
access to a specialized code analysis tool:

```
tiagent agent                    IronClaw (ACP server)
     |                                |
     |  ACP: submit_task({           |
     |    prompt: "Refactor this     |
     |     function for clarity",    |
     |    context: { code: "..." }   |
     |  })                           |
     | -----------------------------> |
     |                                |  [IronClaw runs agent loop
     |                                |   with sandboxed tools]
     |                                |
     |  ACP: task_result({           |
     |    status: "completed",       |
     |    output: "..."              |
     |  })                           |
     | <----------------------------- |
     |                                |
```

### 6.2 AITP --- Agent Interaction and Transaction Protocol

AITP is a protocol developed by NEAR AI for enabling AI agents to communicate
securely across trust boundaries. Where ACP focuses on client-to-agent
communication, AITP is designed for agent-to-agent and agent-to-user
interactions that involve structured data exchange and financial transactions.

AITP defines three core concepts:

| Concept | What it is | Example |
|---------|-----------|---------|
| **Threads** | Units of dialogue and work; the communication channel | A conversation between a tiagent agent and a NEAR agent about a data analysis task |
| **Transports** | Mechanisms that carry thread messages between agents | HTTP-based Threads API (AITP-T01) |
| **Capabilities** | Standardized message formats for specific interaction types | Payments (AITP-01), Decisions (AITP-02), Data Requests (AITP-03) |

The current AITP capability set includes:

- **AITP-01: Payments** --- structured financial transactions between agents
- **AITP-02: Decisions** --- presenting choices and capturing selections
- **AITP-03: Data Requests** --- structured information gathering
- **AITP-04: NEAR Wallet** --- NEAR blockchain wallet operations
- **AITP-05: EVM Wallet** --- Ethereum-compatible wallet operations

**How tiagent uses AITP:**

AITP is particularly valuable for tiagent because it bridges the Celestia and
NEAR ecosystems. A tiagent agent (Celestia-native) can use AITP to:

1. **Request data from NEAR agents**: A tiagent agent analyzing cross-chain
   patterns can request data from a NEAR-native agent via AITP-03 (Data
   Request).
2. **Conduct transactions**: An agent that discovers a useful dataset on NEAR
   can pay for access via AITP-01 (Payments).
3. **Coordinate decisions**: Multi-agent workflows can use AITP-02 (Decisions)
   to reach consensus on action plans.

### 6.3 Mapping tiagent Signals to External Protocols

tiagent's internal data model is Signal-based (see **02-architecture.md**).
Every piece of data flowing through the system is a Signal with a `kind` field.
When communicating with IronClaw via ACP or AITP, tiagent needs to translate
between its Signal model and the external protocol's message format:

```rust
/// Convert a tiagent Signal into an ACP task submission.
pub fn signal_to_acp_task(signal: &Signal) -> AcpTask {
    AcpTask {
        prompt: signal.content.to_string(),
        context: AcpContext {
            metadata: signal.metadata.clone(),
            parent_hash: signal.parent.map(|h| h.to_string()),
        },
    }
}

/// Convert an ACP task result back into a tiagent Signal.
pub fn acp_result_to_signal(result: &AcpTaskResult, parent: &Signal) -> Signal {
    Signal {
        kind: SignalKind::ToolResult,
        content: result.output.clone().into(),
        parent: Some(parent.hash()),
        score: None,
        metadata: result.metadata.clone(),
    }
}

/// Convert a tiagent Signal into an AITP thread message.
pub fn signal_to_aitp_message(signal: &Signal) -> AitpMessage {
    AitpMessage {
        thread_id: signal.metadata.get("thread_id")
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        content: signal.content.to_string(),
        capabilities: extract_required_capabilities(signal),
    }
}
```

---

## 7. Implementation Plan

The integration is phased to deliver incremental value. Each phase is
self-contained --- it can be shipped and used independently.

### Phase 1: IronClaw WASM Tool Runner

**Goal**: Untrusted tools execute in IronClaw's WASM sandbox instead of
in-process.

**Deliverables**:
- `tiagent-ironclaw` crate with `IronClawToolRunner` implementing `ToolExecutor`
- Capability file generation for tiagent tools (`.capabilities.json`)
- Tool registry metadata: `execution_mode: native | wasm_sandboxed`
- Integration tests: tool execution, permission denial, resource limit traps
- Documentation: how to mark tools as sandboxed, how to write capabilities files

**Dependencies**:
- IronClaw binary available on the host (installable via `cargo install ironclaw`)
- tiagent tool system (doc 06) implemented

**Effort estimate**: 2--3 weeks

### Phase 2: TEE Execution Mode

**Goal**: tiagent agents produce cryptographic attestations of their execution,
published alongside traces to Celestia DA.

**Deliverables**:
- `AttestedTrace` and `TeeAttestation` types in `tiagent-core`
- TEE deployment configuration (target enclave type, measurement allowlist)
- Attestation verification in the trace consumer path
- `AttestationGate` implementing tiagent's `Gate` trait (rejects unverified
  traces)
- Integration with Celestia DA blob publishing (doc 05 pipeline)

**Dependencies**:
- Phase 1 (WASM tool runner) completed
- Access to TEE-capable deployment environment (NEAR AI Cloud or equivalent)

**Effort estimate**: 3--4 weeks

### Phase 3: ACP and AITP Protocol Bridge

**Goal**: tiagent agents can communicate with IronClaw agents (and any other
ACP/AITP-compliant agents) through standardized protocols.

**Deliverables**:
- ACP client in `tiagent-ironclaw` (submit tasks, receive results)
- AITP client in `tiagent-ironclaw` (threads, capabilities, transactions)
- Signal-to-ACP and Signal-to-AITP translation layer
- AITP capability support: Payments (AITP-01), Data Requests (AITP-03)
- Cross-runtime integration tests (tiagent agent talks to IronClaw agent)

**Dependencies**:
- Phase 1 (WASM tool runner) completed
- AITP specification stabilized (currently at v0.1.0 draft)

**Effort estimate**: 4--5 weeks

### Phase Summary

```
Phase 1                    Phase 2                Phase 3
WASM Tool Runner           TEE Execution          Protocol Bridge
(weeks 1-3)                (weeks 4-7)            (weeks 8-12)

+-------------------+  +-------------------+  +-------------------+
| IronClawToolRunner|  | AttestedTrace     |  | ACP client        |
| Capability files  |  | TeeAttestation    |  | AITP client       |
| Execution mode    |  | AttestationGate   |  | Signal translation|
| registry          |  | DA blob w/ proof  |  | Cross-runtime     |
|                   |  |                   |  | tests             |
| Benefit:          |  | Benefit:          |  | Benefit:          |
| Tool security     |  | Verifiable exec   |  | Cross-ecosystem   |
|                   |  |                   |  | communication     |
+-------------------+  +-------------------+  +-------------------+
```

---

## Summary

IronClaw provides three capabilities that tiagent should use rather than
reimplement:

1. **WASM sandboxing** for executing untrusted tools with capability-based
   permissions, resource limits, and credential protection.
2. **TEE execution** for producing hardware-signed attestations of agent
   computation, enabling verifiable traces on Celestia DA.
3. **ACP/AITP protocols** for structured agent-to-agent communication across
   runtime boundaries.

The integration is layered: start with WASM tool sandboxing (immediate security
benefit, minimal coupling), add TEE attestation (verifiable execution for
on-chain use cases), and expand to protocol bridges (cross-ecosystem agent
communication). Each layer builds on the previous one but can be adopted
independently.
