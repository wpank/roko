# 11 — Inconsistencies & Corrections in roko-progress

> **Purpose**: Cross-reference `MORI-PARITY-CHECKLIST.md` and `CURRENT-STATE.md`
> against the **active codebase** at `/Users/will/dev/nunchi/roko/roko/crates/`.
>
> **Audited**: 2026-04-08
>
> ⚠ **NOTE**: The bardo repo (`/Users/will/dev/uniswap/bardo/roko/`) is stale.
> The active repo is `/Users/will/dev/nunchi/roko/roko/`. The nunchi copy has
> diverged with meaningful progress that the roko-progress docs don't reflect.

---

## 🟢 Progress in nunchi NOT reflected in roko-progress

### 1. `orchestrate.rs` — the runtime harness EXISTS now

`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (766 lines)

This is a **major addition** not tracked in `MORI-PARITY-CHECKLIST.md`:
- Plan-driven loop: reads plans → builds executor → dispatches agents → runs gates → persists
- Per-plan tracking (agent calls, phase, success/failure)
- Gate pipeline integration (CompileGate, TestGate, ClippyGate)
- `OrchestrateReport` with per-plan stats
- `role_system_prompt(role)` — role-specific system prompts (basic but functional)

**Items that should be updated in checklist**:
- §14 (Plan execution) — several items now partially done
- I.2 (Orchestrator wiring) — harness exists, partially wired

### 2. `SafetyLayer` — wired into dispatcher

`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` now has a `SafetyLayer`
struct (256 lines) that composes all guards (bash, git, network, path, scrub, rate_limit) and
the `ToolDispatcher` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs`
integrates it via `.with_safety(layer)`.

**Items that should be updated**:
- I.1 (Safety wiring) — partially done: SafetyLayer→Dispatcher wired, but Dispatcher not yet called from CLI/orchestrate.rs
- §28 — more items should move from `[ ]` to `[~]`

### 3. `bardo-runtime` — new crate with process management

`/Users/will/dev/nunchi/roko/roko/crates/bardo-runtime/src/process.rs` has:
- `ProcessHandle` wrapping `tokio::process::Child`
- `ProcessSupervisor` pool with bulk kill/reap
- `ProcessId` unique identity
- Cooperative shutdown with grace period
- Stdout/stderr stream capture

**Items that should be updated**:
- §8 (Process management) — §8.1-8.4 partially covered by ProcessSupervisor
- §8.6 (kill_all_descendants) — `ProcessSupervisor::shutdown_all()` exists

### 4. `bardo-primitives` — new crate

`/Users/will/dev/nunchi/roko/roko/crates/bardo-primitives/src/` has:
- `hdc.rs` — HDC fingerprint primitives
- `tier.rs` — tiering primitives

### 5. `roko-std` — new crate with standard impls

`/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/` has:
- `MemorySubstrate` — in-memory Substrate for tests
- NoOp impls of all 6 traits
- Composite scorers (Sum, Mul, Const)
- Routers (First, HighestScore, RoundRobin)
- `trace_sink.rs` — TraceSink impls

### 6. Episode logging removed from run.rs

The nunchi copy of `run.rs` has **removed** the EpisodeLogger integration that existed
in the bardo copy. This means §I.3.1 may need to be re-implemented, or it was
intentionally removed during refactoring.

---

## 🔴 CRITICAL: Still-present gaps in nunchi codebase

Despite the progress above, the core "agent wiring" gap persists:

### 1. `orchestrate.rs` still uses `ExecAgent` (line 545)

```rust
let mut agent = ExecAgent::new(
    &self.config.agent.command,
    self.config.agent.args.clone(),
)
```

No Claude-specific flags. No `--tools`, `--settings`, `--bare`, `--mcp-config`,
`--fallback-model`, `--effort`, `--resume`. The role system prompt goes into the
**user message** via `PromptComposer`, not into `--append-system-prompt`.

### 2. Role system prompts are minimal (line 660-720)

```rust
AgentRole::Implementer => "You are an expert Rust software engineer. Implement the task
  precisely, writing clean, well-tested code that follows the existing codebase conventions."
```

vs Mori's ~2K token prompt with coding standards, tool guidance, artifact hints, rules.
The elaborate `SystemPromptBuilder` and 9 templates in `roko-compose` are still unused.

### 3. `ClaudeAgent` (HTTPS path) still has no system prompt

`MessagesRequest` is still just `{model, max_tokens, messages}`. No `system` field.

### 4. SafetyLayer wired to Dispatcher but Dispatcher not called from CLI

The `ToolDispatcher.with_safety(layer)` connection exists, but `orchestrate.rs` never
creates a `ToolDispatcher` — it just calls `ExecAgent::run()`.

### 5. ProcessSupervisor exists but not used by orchestrate.rs

`bardo-runtime::ProcessSupervisor` is built but `orchestrate.rs` doesn't use it.

---

## 🟡 MISLEADING items in roko-progress docs

### CURRENT-STATE.md

This doc references `/Users/will/dev/uniswap/bardo/roko/` which is now stale.
It should be updated to reference `/Users/will/dev/nunchi/roko/roko/` and
re-verified against the active codebase.

Key claims that need re-verification:
- LOC counts (may have changed)
- Test counts (some tests removed in nunchi copy)
- "✅" status on crates (needs "wired?" column)

### MORI-PARITY-CHECKLIST.md §5 (Per-role prompt templates)

All 8 items marked `[x]`. Template files exist in both bardo and nunchi copies.
But `orchestrate.rs` has its own inline `role_system_prompt()` function that
**doesn't use** these templates — it has hardcoded 1-sentence prompts.

**Recommendation**: Downgrade §5.1-5.8 to `[~]` and add note: "templates exist
in roko-compose but orchestrate.rs uses inline prompts instead".

### 08-gap-inventory.md, 09-refactor-gaps.md

These predate the nunchi fork. Items they list as gaps may now be partially
addressed by the new crates (bardo-runtime, bardo-primitives, roko-std).
Should be re-checked or marked "SUPERSEDED BY MORI-PARITY-CHECKLIST.md".

---

## Recommended actions

1. **Update CURRENT-STATE.md** to reference nunchi paths, re-verify all claims
2. **Downgrade §5.1-5.8** in checklist from `[x]` to `[~]`
3. **Add new items** to checklist for orchestrate.rs, bardo-runtime, bardo-primitives, roko-std
4. **Upgrade §I.1** partially — SafetyLayer→Dispatcher is wired
5. **Upgrade §I.2** partially — orchestrate.rs harness exists
6. **Upgrade §8** partially — ProcessSupervisor exists
7. **Add new § or I.* item** for "Wire SystemPromptBuilder/templates into orchestrate.rs"
8. **Add new § or I.* item** for "Replace ExecAgent with ClaudeCliAgent in orchestrate.rs"
9. **Mark episode logging** as regressed (removed from nunchi run.rs)
