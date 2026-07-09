# C — Runtime Guards (Docs 05, 06, 07)

Parity of the runtime-guard chapters: loop detection (rate limiter +
circuit breaker + ghost-turn detection), sandboxing (PathPolicy +
ProcessSupervisor + worktree isolation), prompt security (XML delimiters
+ CaMeL dual-LLM + Ventriloquist).

All three chapters are substantially shipping: `roko-agent/src/safety/
rate_limit.rs` (508 LOC) + `conductor` circuit breaker (batch 07) +
`conductor` diagnosis engine + `roko-agent/src/safety/path.rs` (487
LOC) + `roko-orchestrator/src/safety/sandboxing.rs` (651 LOC) +
`roko-orchestrator/src/safety/loop_guard.rs` (364 LOC). Prompt security
is mostly present via compose + chain witness surfaces; CaMeL
dual-LLM is frontier.

Generated: 2026-04-16.

---

## C.01 — RateLimiter sliding window ships (Doc 05 §"RateLimiter sliding window")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 specifies a sliding-window rate limiter keyed by (role, tool), default 60 calls/60s.
**Reality**: `roko-agent/src/safety/rate_limit.rs:1-508` ships. `RateLimitKey` + `RateLimiter`. `SafetyLayer` at `safety/mod.rs:89` holds `Option<Arc<RateLimiter>>`. `with_defaults()` constructor configures the default 60-calls-per-60-seconds window (per Doc 16). Shared across calls via `Arc`.

---

## C.02 — Circuit breaker (conductor) with persistent state (Doc 05 §"Circuit Breaker (conductor)")

**Status**: DONE (per batch 07)
**Severity**: —
**Doc claim**: Plan-level circuit breaker via conductor. Persistent failure state across restarts.
**Reality**: Cross-ref batch 07 A.04 — `CircuitBreaker` at `crates/roko-conductor/src/circuit_breaker.rs` with `MAX_PLAN_FAILURES = 2`, `DashMap`-backed `FailureRecord`. Consumed by orchestrate.rs (batch 07 C.08). Batch 07 C.09 notes persistence-across-restart is PARTIAL — tripped state does not survive executor snapshot (that gap owned by batch 07, not 11).

---

## C.03 — DiagnosisEngine ghost-turn detection (Doc 05 §"DiagnosisEngine ghost turn detection")

**Status**: DONE (per batch 07)
**Severity**: —
**Doc claim**: Ghost-turn detection: the agent emits nothing for N consecutive turns → diagnosis intervention.
**Reality**: Cross-ref batch 07 A.03 + A.07 — `GhostTurnWatcher` in `roko-conductor/src/watchers/ghost_turn.rs` ships as 1 of 10 watchers. `StuckPatternWatcher` and `StuckDetector` also ship. Batch 07 A.07 confirms six stuck heuristics + `MetaCognitionHook`.

---

## C.04 — LoopGuard with n-gram repetition detection (Doc 05 §"Loop Detection")

**Status**: DONE (additional shipping surface)
**Severity**: —
**Doc claim**: Doc 05 does not fully enumerate LoopGuard but the mechanism is described.
**Reality**: `crates/roko-orchestrator/src/safety/loop_guard.rs:1-364` ships as an **orchestrator-layer loop guard** distinct from conductor watchers. `LoopGuardConfig` at `:33`, `LoopVerdict` enum at `:57`, `LoopGuard` at `:141`. This is a separate surface from conductor `GhostTurnWatcher` — operates at the orchestrator level rather than per-agent-turn.
**Fix sketch**: Doc 05 should cite both the conductor-layer watchers (`roko-conductor`) and the orchestrator-layer `LoopGuard` (`roko-orchestrator/src/safety/loop_guard.rs`) as distinct complementary loop-detection surfaces.

---

## C.05 — Secret zeroization via ScrubPolicy (Doc 05 §"Secret Zeroization")

**Status**: DONE
**Severity**: —
**Doc claim**: Secrets are scrubbed from tool outputs. Doc 16 cites 9 default regex patterns.
**Reality**: `crates/roko-agent/src/safety/scrub.rs:1-472` ships `ScrubPolicy`. `SafetyLayer.scrub_policy: ScrubPolicy` at `safety/mod.rs:87`. `ScrubPolicy::default()` activates 9 regex patterns (API keys, JWTs, private keys, env assignments — per Doc 16). Invoked as stage 7 (`safety scrub`) of the dispatcher pipeline per Doc 16.

---

## C.06 — Adaptive gate thresholds (Doc 05 §"Adaptive Gate Thresholds")

**Status**: DONE (per CLAUDE.md)
**Severity**: —
**Doc claim**: Gate thresholds adapt to historical performance.
**Reality**: CLAUDE.md "Adaptive gate thresholds | Wired | EMA per rung in `.roko/learn/gate-thresholds.json`". Shipping.

---

## C.07 — PathPolicy canonicalization algorithm ships (Doc 06 §"PathPolicy Canonicalization")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 specifies path canonicalization + escape prevention + optional symlink denial.
**Reality**: `crates/roko-agent/src/safety/path.rs:1-487` ships `PathPolicy`. `SafetyLayer.path_policy: PathPolicy` at `safety/mod.rs:85`. Doc 16 confirms: "Worktree sandbox via canonicalization, escape prevention, optional symlink denial".

---

## C.08 — SandboxPolicy / SandboxEnforcer ships (Doc 06 §"Future Container Sandboxing")

**Status**: DONE (additional shipping surface; Doc 06 marks "future")
**Severity**: —
**Doc claim**: Doc 06 §"Future Container Sandboxing" marks container-level sandboxing as future work.
**Reality**: `crates/roko-orchestrator/src/safety/sandboxing.rs:1-651` ships:
- `SandboxError` enum at `:59`
- `SandboxPolicy` at `:107`
- `SandboxPolicyBuilder` at `:138`
- `SandboxEnforcer<'p>` at `:217`

This is a substantial orchestrator-layer sandboxing surface (651 LOC) beyond the agent-layer `PathPolicy`. Whether it is full container-level (bwrap / firejail / docker) or a richer in-process sandbox requires deeper reading — but the enforcer surface is real.
**Fix sketch**: Doc 06 should cite `roko-orchestrator/src/safety/sandboxing.rs` as a shipping sandbox surface and downgrade the "future" claim to "orchestrator-layer sandbox ships; container integration frontier".

---

## C.09 — ProcessSupervisor lifecycle management (Doc 06 §"ProcessSupervisor Lifecycle Management")

**Status**: DONE (per CLAUDE.md)
**Severity**: —
**Doc claim**: ProcessSupervisor tracks agent processes + enforces shutdown timeouts.
**Reality**: CLAUDE.md "ProcessSupervisor (lifecycle mgmt) | Wired | PlanRunner tracks + shuts down agents". Shipping.

---

## C.10 — WorktreeManager isolation (Doc 06 §"WorktreeManager Isolation")

**Status**: DONE
**Severity**: —
**Doc claim**: Git worktrees isolate agent runs.
**Reality**: Worktree creation happens via `.claude/worktrees/` and `.roko/worktrees/` — grep-verified in the repo (batch 08 found worktree directories). Shipping.

---

## C.11 — XML-delimited prompt architecture (Doc 07 §"Prompt Architecture with XML Delimiters")

**Status**: DONE (per compose layer)
**Severity**: —
**Doc claim**: Prompts use XML delimiters (`<task>`, `<context>`, `<system>`) for structural robustness against injection.
**Reality**: `crates/roko-compose/` (batch 09 D.11) owns `SystemPromptBuilder` (6-layer prompt; CLAUDE.md "Wired"). XML delimiter usage is a compose-layer concern, shipping as prompt-template convention.

---

## C.12 — CaMeL dual-LLM pattern is absent (Doc 07 §"CaMeL dual-LLM", Debenedetti et al. 2025)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: CaMeL (Debenedetti et al. 2025) separates privileged-LLM (trusted) from quarantined-LLM (untrusted) for prompt-injection defense.
**Reality**: `Grep 'CaMeL\|dual[_-]LLM\|quarantined_llm' crates/ --include=*.rs` returns zero matches. Frontier academic pattern.

---

## C.13 — Ventriloquist defense (Doc 07 §"Ventriloquist Defense")

**Status**: NOT DONE (per batch 08)
**Severity**: LOW
**Doc claim**: System-prompt-hash commitment on Korai chain with 24h timelock for updates.
**Reality**: Cross-ref batch 08 B.08 — Ventriloquist defense on-chain surface is absent (no `system_prompt_hash` commitment, no timelock). The concept is design-only pending Korai Passport / Tier-6 deployment.

---

## C.14 — Tool-Guard pattern + MCP avoidance (Doc 07 §"Tool-Guard Pattern", §"MCP Avoidance")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Tools are gated by permissions (Tool-Guard). MCP is avoided as an attack surface.
**Reality**: Tool-Guard ships via `SafetyLayer` (A.01) + `ToolDispatcher` 7-stage pipeline (Doc 16). MCP is used in Roko (CLAUDE.md: "MCP config passthrough | Wired"; `roko-mcp-code` is a shipping MCP server). Doc 07's "MCP avoidance" framing is outdated vs the shipping MCP-integrated design.
**Fix sketch**: Doc 07 §"MCP Avoidance" should be updated — MCP is embraced with safety gating, not avoided. Cite `roko-mcp-code` + the `SafetyLayer` invariants that apply equally to MCP-sourced tools.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 11 (C.01 RateLimiter, C.02 circuit breaker, C.03 ghost turn, C.04 LoopGuard, C.05 ScrubPolicy, C.06 adaptive gate thresholds, C.07 PathPolicy, C.08 SandboxPolicy, C.09 ProcessSupervisor, C.10 WorktreeManager, C.11 XML prompts) |
| PARTIAL | 1 (C.14 MCP avoidance outdated) |
| NOT DONE | 2 (C.12 CaMeL dual-LLM, C.13 Ventriloquist on-chain) |

Section C is the **strongest shipping section of topic 11**.
Runtime guards are comprehensive: rate limiter, circuit breaker,
ghost-turn watcher, loop guard, secret scrubbing, path
canonicalization, orchestrator-layer sandbox enforcer, process
supervisor, worktree isolation. The two NOT-DONE items are frontier
academic patterns (CaMeL) and Tier-6 chain features (Ventriloquist).

## Agent Execution Notes

### C.04 / C.08 — Two shipping loop+sandbox surfaces

Docs 05 and 06 should cite both the agent-layer and
orchestrator-layer shipping surfaces. They are complementary, not
redundant.

### C.14 — Update MCP framing

Doc 07 still frames MCP as an attack surface to avoid. Roko ships
MCP-integrated + gated. Update the framing.

Doc 16's remaining integration gap should not be described as "MCP
doesn't fit the safety architecture". The real residual gap is
subprocess-owned or specialty execution paths that bypass the central
dispatcher, not MCP as a category.

Acceptance criteria:

- Doc 05 cites `rate_limit.rs` + `LoopGuard`,
- Doc 06 cites `PathPolicy` + `SandboxEnforcer` + `ProcessSupervisor`,
- Doc 07 §"MCP Avoidance" updated to reflect shipping MCP integration,
- remaining integration caveats are framed as dispatcher-coverage gaps,
  not MCP-avoidance doctrine.
