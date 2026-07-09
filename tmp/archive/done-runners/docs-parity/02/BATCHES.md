# Batch Plan — 02 Agents Parity Refresh

These are documentation-refresh batches only.

They do **not** authorize code changes outside `tmp/docs-parity/02/`.

---

## Execution Contract

- Read the live `docs/02-agents/` docs and the current code before rewriting parity copy.
- Prefer `keep`, `narrow`, `defer`, or `rewrite` decisions over new prescriptions.
- Keep every batch small enough for a single agent to finish in under 90 minutes.
- When a claim depends on code, anchor it with a file path or line reference.
- If an idea has no clear runtime owner or no external user pressure, defer it.

---

## Recommended Order

`G1 -> G2 -> G3 -> G4 -> G5 -> G6`

This order locks the batch posture first, then refreshes the shipped runtime surfaces before touching the more speculative sections.

---

## Batch Overview

| Batch | Scope | Goal | Primary files | Verify focus |
|---|---|---|---|---|
| G1 | Index + source anchors | Reset the batch posture around audited scope and current evidence | `00-INDEX.md`, `BATCHES.md`, `SOURCE-INDEX.md`, `run-docs-parity.sh` | `bash -n` + anchor spot-checks |
| G2 | Core abstractions | Refresh the live agent surface, role coverage, and narrow the true shared-type/event seams | `A-core-abstractions.md`, `context-pack/agents-summary.md` | agent count, role count, duplicate-event notes |
| G3 | Provider system | Update the provider/backend story to match current adapters and runtime families | `B-provider-system.md`, `context-pack/repo-map.md` | provider kinds, adapters, backend families |
| G4 | Tool runtime + lifecycle | Show the real tool loop, MCP passthrough, sidecar, and `PlanRunner` lifecycle ownership | `C-tool-loop.md`, `D-lifecycle-infrastructure.md`, `context-pack/agent-runbook.md` | tool/runtime anchors, MCP path, sidecar tests |
| G5 | Routing + active gaps | Narrow routing copy to what is actually shipped and isolate the small remaining gap set | `E-routing-temperament.md`, `context-pack/gaps-summary.md` | `CascadeRouter`, `active_inference`, event duplication |
| G6 | Advanced + deferred | Keep the real extension points and move domain/plugin overreach into explicit future work | `F-advanced-capabilities.md`, `context-pack/carry-forward-map.md` | deferred labels and handoff notes |

---

## Dependencies

| Batch | Depends on | Why |
|---|---|---|
| G1 | — | Establishes the refreshed posture and source anchors |
| G2 | G1 | Core claims should follow the refreshed anchor set |
| G3 | G1 | Provider copy depends on corrected terminology and anchors |
| G4 | G1 | Lifecycle and tool-runtime refresh should cite the corrected anchor set |
| G5 | G1 | Routing refresh depends on the narrowed posture |
| G6 | G1 | Deferral language should inherit the same posture |

---

## Batch Details

### G1 — Reset The Pack

**Owns**

- refresh the top-level index
- replace the old execution-plan framing
- correct stale line references
- update the runner script text and batch descriptions

**Out of scope**

- changing code
- expanding the pack into a roadmap

**Verify**

```bash
bash -n tmp/docs-parity/02/run-docs-parity.sh
rg -n "docs calibration|duplicate `AgentEvent`|16-tool built-in registry|ProcessSupervisor" tmp/docs-parity/02/00-INDEX.md tmp/docs-parity/02/SOURCE-INDEX.md
```

### G2 — Core Abstractions

**Owns**

- current `Agent` surface
- role coverage
- live backend families
- the small remaining core seams: response-type ownership and duplicate `AgentEvent` enums

**Out of scope**

- new shared crates
- full event unification plans

**Verify**

```bash
rg -n "19 `Agent` impls|28 variants|Claude CLI/API|duplicate `AgentEvent`" tmp/docs-parity/02/A-core-abstractions.md tmp/docs-parity/02/context-pack/agents-summary.md
```

### G3 — Provider System

**Owns**

- current provider kinds and adapters
- mapping from runtime families to provider adapters
- clear wording for Codex/OpenAI/Ollama/OpenRouter-style traffic

**Out of scope**

- new provider SPI work
- speculative optimization layers

**Verify**

```bash
rg -n "6 `ProviderKind` variants|OpenAiCompat|ClaudeCliAdapter|PerplexityAdapter|GeminiAdapter" tmp/docs-parity/02/B-provider-system.md tmp/docs-parity/02/context-pack/repo-map.md
```

### G4 — Tool Runtime And Lifecycle

**Owns**

- live `ToolLoop` / `ToolDispatcher` / `SafetyLayer` narrative
- built-in tool registry reality
- MCP passthrough path
- `roko-agent-server` and `PlanRunner` lifecycle ownership

**Out of scope**

- universalizing every backend onto one runtime path
- pushing pools into the main orchestrator narrative

**Verify**

```bash
rg -n "ToolLoop|ToolDispatcher|16-tool built-in registry|agent.mcp_config|AgentServer|ProcessSupervisor" tmp/docs-parity/02/C-tool-loop.md tmp/docs-parity/02/D-lifecycle-infrastructure.md tmp/docs-parity/02/context-pack/agent-runbook.md
```

### G5 — Routing And Gaps

**Owns**

- `CascadeRouter` reality
- active inference as an existing but optional path
- temperament as partial rather than fully propagated runtime policy
- the narrowed post-audit gap list

**Out of scope**

- meta-router proposals
- research-heavy routing overlays

**Verify**

```bash
rg -n "CascadeRouter|active inference|temperament|duplicate `AgentEvent`" tmp/docs-parity/02/E-routing-temperament.md tmp/docs-parity/02/context-pack/gaps-summary.md
```

### G6 — Advanced And Deferred

**Owns**

- real extension points and advanced primitives that already ship
- explicit deferral of domain profiles and plugin SPI tiers 4-5
- carry-forward notes for items that belong elsewhere

**Out of scope**

- presenting target-state ideas as live capabilities
- adding new roadmap commitments

**Verify**

```bash
rg -n "DEFERRED|domain profiles|plugin SPI tiers 4-5|CompositeAgent|MorphableAgent" tmp/docs-parity/02/F-advanced-capabilities.md tmp/docs-parity/02/context-pack/carry-forward-map.md
```
