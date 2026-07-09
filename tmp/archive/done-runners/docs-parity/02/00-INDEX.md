# 02 — Agents Parity Refresh

Parity refresh for `docs/02-agents/` against the live codebase and the 2026-04-17 audit.

Generated: 2026-04-18

---

## Batch Posture

- Treat this pack as a docs calibration pass, not a code-implementation plan.
- Describe only what is wired today.
- Mark research-heavy or zero-user proposals as `DEFERRED`.
- Keep recommendations small enough for one agent to verify in a single sitting.

---

## Reality Snapshot

| Area | Current state | Evidence |
|---|---|---|
| Agent surface | `roko-agent` ships 19 `Agent` impls spanning Claude CLI/API, Codex, Cursor, OpenAI, Ollama, Gemini, and Perplexity families | `crates/roko-agent/src/` |
| Roles | `AgentRole` is live with 28 variants and default backend/tier/budget mappings | `crates/roko-core/src/agent.rs` |
| Provider layer | 6 `ProviderKind` variants and 6 registered adapters are wired | `crates/roko-core/src/agent.rs`, `crates/roko-agent/src/provider/mod.rs` |
| Tool runtime | `ToolLoop`, `ToolDispatcher`, and `SafetyLayer` are real; `roko-std` currently exports a 16-tool built-in registry | `crates/roko-agent/src/tool_loop/`, `crates/roko-agent/src/dispatcher/`, `crates/roko-std/src/tool/builtin/mod.rs` |
| MCP | `agent.mcp_config` passthrough works from CLI config through agent spawn, with discovery fallback via `find_mcp_config` | `crates/roko-cli/src/config.rs`, `crates/roko-cli/src/orchestrate.rs`, `crates/roko-agent/src/mcp/config.rs` |
| Sidecar | `roko-agent-server` is wired as a per-agent HTTP sidecar with dispatcher-backed messaging and relay registration tests | `crates/roko-agent-server/` |
| Lifecycle | `PlanRunner` owns a live `ProcessSupervisor`; pools exist but are not the main runtime owner | `crates/roko-cli/src/orchestrate.rs`, `crates/roko-runtime/src/process.rs` |
| Routing | `CascadeRouter` is live and persisted; `active_inference.rs` exists as an optional path, not the default orchestration path | `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/active_inference.rs` |
| Event seams | Two duplicate `AgentEvent` enums still exist, one in `roko-agent` and one in `roko-learn` | `crates/roko-agent/src/task_runner.rs`, `crates/roko-learn/src/events.rs` |

---

## What This Refresh Changes

| File | Verdict | Refresh goal |
|---|---|---|
| [A-core-abstractions.md](A-core-abstractions.md) | `rewrite` | Present the live agent surface and narrow remaining gaps to ownership and event duplication issues |
| [B-provider-system.md](B-provider-system.md) | `rewrite` | Replace stale adapter/provider counts with the current provider families and clarify what rides through `OpenAiCompat` |
| [C-tool-loop.md](C-tool-loop.md) | `rewrite` | State clearly that the tool runtime is real, but not every backend uses the same shared path |
| [D-lifecycle-infrastructure.md](D-lifecycle-infrastructure.md) | `rewrite` | Emphasize `PlanRunner` + `ProcessSupervisor`, MCP passthrough, and the live sidecar instead of speculative pool adoption work |
| [E-routing-temperament.md](E-routing-temperament.md) | `narrow` | Keep the real routing stack, but stop describing temperament and active inference as fully propagated runtime policy |
| [F-advanced-capabilities.md](F-advanced-capabilities.md) | `defer` | Move domain profiles, six-domain packaging, and plugin SPI tiers 4-5 into explicit future work |

---

## Narrow Gap Set

These are the only issues this parity pack should treat as active near-term gaps:

1. `AgentEvent` is duplicated across `roko-agent` and `roko-learn`.
2. Provider and tool-loop docs still lag the current backend surface.
3. MCP and sidecar wiring are real but underrepresented in the old parity notes.
4. Temperament, domain profiles, and plugin tiers 4-5 were described too far ahead of code and usage.

Everything else should be framed as either shipped or explicitly deferred.

---

## Explicit Deferrals

Do not describe these as present-tense agent capabilities in this batch:

- domain profiles from doc 16 as a live runtime product surface
- six-domain agent bundles
- plugin SPI tiers 4-5
- shared agent memory, Darwin/Godel, archive systems, or self-evolving agent frameworks
- meta-router and other research-only routing overlays

These may remain as target-state notes, but they are not parity blockers for `02-agents`.

---

## Success Criteria

- Every file under `tmp/docs-parity/02/` reflects the current codebase instead of the earlier implementation plan.
- The pack distinguishes live wiring from target-state ideas.
- Deferred material is labeled as deferred, not implied to exist.
- `bash -n tmp/docs-parity/02/run-docs-parity.sh` passes.
