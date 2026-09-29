# Safety Hooks

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- the tool
> execution safety chain, AgentContract policy, provider isolation, tool
> cooldown, immune Graph screening, and the trust-origin lattice integration.

---

## 1. Overview

Safety in Roko's tool system operates at two levels:

1. **Structural** -- role-based access control ensures agents can only see
   tools permitted by their role. An Auditor role structurally lacks write
   tools in its registry.

2. **Runtime** -- the safety hook chain in `roko-agent/src/safety/` applies a
   pipeline of checks before and after tool execution.

These levels are complementary. The structural system provides the guarantee
("this tool CANNOT exist in the registry"). The runtime system provides policy
enforcement ("this tool SHOULD NOT run under these conditions").

---

## 2. Tool Execution Safety Chain

All tool calls pass through the safety layer:

```
LLM proposes tool call
       |
       v
1. Role authorization (structural)
       |
       v
2. AgentContract tool policy (intersection)
       |
       v
3. Pre-call validation (params, rate limits, budget)
       |
       v
4. Tool executes
       |
       v
5. Post-call verification (output sanitization)
       |
       v
6. Immune Graph screening (five-stage)
       |
       v
7. Evidence persistence (content-addressed)
```

If any step rejects, the chain halts and the rejection is recorded in the
audit trail.

---

## 3. AgentContract Tool Policy

Role/task allowlists intersect to determine which tools an agent may call:

- A tool must be allowed by **both** the agent's role and the task's policy
- Denials win over allows (deny-by-default)
- Unknown roles deny all tools
- Unsupported policy-bearing dispatches are rejected

```rust
// Intersection semantics:
// allowed = role_allowlist INTERSECT task_allowlist
// if tool in denied_list -> reject (regardless of allow)
// if role unknown -> deny all
```

This intersection model prevents privilege escalation: a task cannot grant
tools that the agent's role does not permit.

---

## 4. Provider Isolation and Tool Cooldown

The E34 safety closure enforces provider-level isolation:

**Provider isolation:** Each provider session operates within bounded
workspace-rooted authority. Provider-owned internal calls and results stay
within the provider boundary. Controls persist independently of disposable
attempt worktrees.

**Tool cooldown/isolation:** Rapid-fire tool calls are rate-limited per
provider. The cooldown prevents:
- Runaway loops from hallucinating agents
- Resource exhaustion from tight retry loops
- Timing-based side channels from rapid tool sequences

**Content-addressed evidence:** Tool results are checksummed with BLAKE3 and
persisted with security metadata. This creates tamper-evident evidence chains.

**Reciprocal incident links:** Quarantine incidents link back to the
triggering tool call and forward to remediation. A quarantined tool call
records which safety check triggered it and which subsequent actions
addressed the issue.

---

## 5. Immune Graph Screening

Canonical provider primary outputs and every host-visible `ToolDispatcher`
result traverse the fixed five-stage immune Graph:

```
1. Deference  -- Does this violate corrigibility constraints?
2. Switch     -- Should we escalate to a human?
3. Truth      -- Is the output truthful/consistent?
4. Impact     -- What is the potential harm?
5. Task       -- Does this advance the assigned task?
```

This is the same immune pipeline described in Chapter 12 (Safety). Tool
results are one of its primary inputs. All five stages must pass for a
tool result to be accepted into the cognitive loop.

---

## 6. TaintedString: Sensitive Data Handling

Sensitive data (private keys, API keys, session tokens) uses taint tracking
via the trust-origin lattice. Flow control rules determine where tainted
data may propagate:

| Taint Label | LLM Context | Event Bus | Collective Mesh |
|---|---|---|---|
| WalletSecret | BLOCKED | BLOCKED | BLOCKED |
| OwnerSecret | BLOCKED | Allowed | BLOCKED |
| StrategyConfidential | Allowed | Allowed | BLOCKED |
| UserPII | Allowed | Allowed | BLOCKED |
| UntrustedExternal | After validation | Allowed | After validation |

The TaintTracker in `roko-agent/src/safety/` enforces these flow rules at
every boundary crossing.

---

## 7. Audit Trail

Every safety decision produces an audit record persisted as a Signal with
lineage linking back to the triggering tool call:

```
Tool Call Signal
  -> Safety Check Signal (role auth)
  -> Safety Check Signal (policy check)
  -> Tool Execution Signal
  -> Post-Call Check Signal
  -> Outcome Verification Signal
```

This audit chain supports forensic replay: every action can be causally
traced from Signal lineage for debugging or compliance.

---

## 8. WASM Sandbox Security

Untrusted tools (plugin-provided, marketplace-purchased, third-party MCP)
can run inside a WASM sandbox via Wasmtime:

- **No filesystem access** -- cannot read or write host files
- **No network access** -- cannot make HTTP requests or open sockets
- **No key access** -- cannot access signing keys
- **Fuel metering** -- bounded computation (default 10M fuel units)
- **Epoch interruption** -- wall-clock timeout (default 5s)
- **Memory cap** -- configurable RSS limit (default 256 MB)

Sandboxed tools receive a restricted interface -- read operations only. Any
write must be returned as a request that the host validates through the
normal safety chain before executing.

---

## 9. Source Locations

| Component | Path |
|---|---|
| Safety layer | `crates/roko-agent/src/safety/` |
| Tool dispatcher | `crates/roko-agent/src/dispatcher/mod.rs` |
| Trust-origin lattice | `crates/roko-agent/src/safety/` |
| Sandbox config | `crates/roko-std/src/tool/sandbox_config.rs` |
| Immune Graph | `crates/roko-graph/` |

---

*Derived from: v1/18-tools/04-safety-hooks.md. Chain-specific safety hooks
(PolicyCage, AllowlistGuard, SpendingLimiter, RevmSimulator,
HallucinationDetector) moved to chain domain plugin documentation.
Capability<T> compile-time tokens are chain-domain-specific. The E34 closure
additions (provider isolation, tool cooldown, evidence checksums, incident
links) are integrated from the 2026-08-17 acceptance.*
