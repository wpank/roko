# WASM Hooks (All 23)

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- the 23
> validated WASM hook points available to plugin extensions, their lifecycle
> positions, and the bounded execution contract.

---

## 1. Overview

The plugin registry (`crates/roko-plugin/src/registry.rs`) validates that WASM
extension modules only export functions from a fixed set of 23 named hook
points. Any export outside this set causes admission rejection. This bounded
contract prevents plugins from defining arbitrary entry points that could bypass
the sandbox.

---

## 2. The 23 Hook Points

### 2.1 Lifecycle Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `on_init` | Plugin load | Initialize state, validate config |
| `on_shutdown` | Plugin unload | Clean up resources, flush state |

### 2.2 Observation Hooks (3)

| Hook | When | Purpose |
|---|---|---|
| `on_observe` | Signal observed | React to new signals in substrate |
| `on_filter` | Pre-routing | Filter signals before scoring |
| `filter_input` | Pre-composition | Transform or filter input data |

### 2.3 Storage Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `on_retrieve` | Substrate read | Intercept or augment retrieved data |
| `on_store` | Substrate write | Validate or transform stored data |

### 2.4 Inference Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `pre_inference` | Before LLM call | Modify prompt, add context, check budget |
| `post_inference` | After LLM response | Process output, extract metadata |

### 2.5 Gate Hooks (1)

| Hook | When | Purpose |
|---|---|---|
| `on_gate` | Gate verdict | React to gate pass/fail decisions |

### 2.6 Action Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `pre_action` | Before tool execution | Validate parameters, check policy |
| `post_action` | After tool execution | Process results, record outcomes |

### 2.7 Tool Hooks (1)

| Hook | When | Purpose |
|---|---|---|
| `on_tool_call` | Tool dispatch | Intercept tool calls for logging/policy |

### 2.8 Communication Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `on_message_send` | Outbound message | Transform or validate outgoing messages |
| `on_message_receive` | Inbound message | Process incoming messages |

### 2.9 Cognitive Hooks (1)

| Hook | When | Purpose |
|---|---|---|
| `on_reflect` | Meta-cognition step | Agent self-assessment and adaptation |

### 2.10 Budget Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `on_cost_update` | Cost recorded | React to cost changes |
| `on_budget_exceeded` | Ceiling hit | Handle budget exhaustion |

### 2.11 Tick Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `on_tick_start` | Tick begins | Setup per-tick state |
| `on_tick_end` | Tick completes | Cleanup, emit summaries |

### 2.12 Slot Hooks (2)

| Hook | When | Purpose |
|---|---|---|
| `on_slot_assigned` | Task slot assigned | Prepare for task execution |
| `on_slot_completed` | Task slot finished | Record completion, release resources |

---

## 3. Hook Validation

The registry validates WASM modules at admission time:

```rust
const WASM_HOOK_NAMES: [&str; 23] = [
    "on_init", "on_shutdown", "on_observe", "on_filter",
    "filter_input", "on_retrieve", "on_store",
    "pre_inference", "post_inference", "on_gate",
    "pre_action", "post_action", "on_tool_call",
    "on_message_send", "on_message_receive", "on_reflect",
    "on_cost_update", "on_error", "on_budget_exceeded",
    "on_tick_start", "on_tick_end",
    "on_slot_assigned", "on_slot_completed",
];
```

Any exported function whose name is not in this set triggers a validation error
and the package is rejected. This is a hard boundary -- there is no mechanism to
register custom hook names.

---

## 4. Execution Constraints

WASM hooks execute under strict resource bounds:

| Constraint | Default | Enforcement |
|---|---|---|
| **Fuel metering** | 10,000,000 fuel units | `wasmi` fuel counter |
| **Module size** | 32 MB max | Checked before compilation |
| **Memory** | 256 MB max | `wasmi` memory limits |
| **No filesystem** | Blocked | No host imports for FS |
| **No network** | Blocked | No host imports for network |

When fuel runs out, execution halts immediately. The hook returns an error
result and the plugin is flagged for health degradation.

---

## 5. Hook Contract

Each hook function has a standard signature:

- Takes serialized input (pointer + length to linear memory)
- Returns a status code (0 = success, non-zero = error)
- Side effects are limited to the declared host imports

Hooks that need to produce output write to a pre-allocated output buffer in
linear memory. The host reads the buffer after the hook returns.

---

## 6. Relationship to the Immune Graph

Hooks at the `pre_action` and `post_action` positions interact with the
five-stage immune Graph (E34). Tool results traverse the immune pipeline
regardless of whether they originate from a built-in tool or a plugin hook.
The `on_tool_call` hook fires before the immune check, allowing plugins to
log or modify parameters, but the actual execution decision is made by the
immune system.

---

## 7. Current Integration Targets

The E32 8/8 manifest confirms bounded all-23-hook WASM integration with the
following live targets:

| Target | Integration |
|---|---|
| Claude CLI MCP | Plugin-declared MCP tools via CLI passthrough |
| Codex CLI MCP | Plugin-declared MCP tools via Codex provider |
| Cursor ACP | Plugin tools through ACP sidecar |
| Hermes ACP | Plugin tools through Hermes provider |
| Native Gemini CLI MCP | Authenticated Gemini MCP tool dispatch |

Component-model Store/Bus hostcalls and OpenClaw/legacy one-shot parity remain
open.

---

## 8. Source Locations

| Component | Path |
|---|---|
| Hook name constants | `crates/roko-plugin/src/registry.rs` (line 28) |
| WASM module validation | `crates/roko-plugin/src/registry.rs` |
| Extension hook points | `crates/roko-core/src/extension.rs` |
| Extension loader | `crates/roko-cli/src/runner/extension_loader.rs` |
| Serve runtime hooks | `crates/roko-cli/src/serve_runtime.rs` |

---

*New file for v3. The 23 hook names are extracted from the
`WASM_HOOK_NAMES` constant in `crates/roko-plugin/src/registry.rs`.*
