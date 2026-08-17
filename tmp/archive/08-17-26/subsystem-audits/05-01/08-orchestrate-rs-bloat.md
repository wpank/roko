# 08 — orchestrate.rs: The 22,635-Line God File

## Summary

`crates/roko-cli/src/orchestrate.rs` is 22,635 lines (861KB). This single file was the primary target of all 5 runner batches (arch, converge, converge-followup, mega-parity, post-parity — totaling ~661 batches of parallel codex agents). Each batch added more logic without extracting responsibilities.

The file has **138 functions**, of which **14 exceed 300 lines**. The largest function is **2,059 lines**.

---

## CRITICAL: `dispatch_agent_with` — 2,059 lines, 8 parameters

**Lines 14561-16620**

This single function does ~15 different things:
1. Budget validation
2. Task definition loading/parsing
3. Model selection via cascade routing
4. Gateway role calculations
5. System prompt building (9-layer)
6. Context assembly (neuro, enrichment, daimon, research, playbook, tool manifest)
7. Section effectiveness scoring
8. Three separate agent dispatch paths (Claude CLI, Ollama, subprocess)
9. Context attribution tracking
10. Gate pipeline execution
11. Episode recording
12. Cost anomaly detection
13. Efficiency event emission
14. Feedback sink writes
15. HDC fingerprint computation

**Why this happened:** Each runner batch added "one more thing" to dispatch. The arch runner added service traits; converge wired CascadeRouter; mega-parity added knowledge queries, attribution, and effectiveness scoring; post-parity added daimon modulation and cost anomaly detection. Nobody extracted.

**Fix:** Extract into 4-5 focused functions: `resolve_dispatch_model()`, `build_dispatch_context()`, `execute_dispatch()`, `record_dispatch_outcome()`.

---

## HIGH: 4 identical `spawn_agent_with_layer` blocks

**Lines 1584-1791**

Four identical 28-line blocks differing only in `SpawnAgentSpec` construction:
1. Has_routing path (lines 1584-1630)
2. Claude CLI path (lines 1645-1691)
3. Known protocol path (lines 1695-1741)
4. Fallback subprocess path (lines 1745-1791)

All follow the same pattern:
```rust
match spawn_agent_with_layer(..., SpawnAgentSpec {...}) {
    Ok(agent) => { ... DispatchOutcome { ... } }
    Err(err) => DispatchOutcome { backend_id: "unknown", ... }
}
```

**Why this happened:** Each dispatch path was added as a separate match arm in the converge runner, then mega-parity added routing, and nobody consolidated.

---

## HIGH: 16 hardcoded model names

Scattered throughout the file:
- `"claude-opus-4-6"` at lines 5369, 9934, 11816, 14660
- `"claude-sonnet-4-6"` at lines 10338, 12927, 14641, 15120
- `"claude-haiku-4-5"` at line 14218
- Escalation models array at line 14218: `["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"]`

These should be in `RokoConfig` or at minimum a constants module.

---

## HIGH: 40+ `unwrap()` calls, including panics in hot paths

Most critical: **Line 15952** in the dispatch path:
```rust
Arc::clone(self.chain_client.as_ref().unwrap())
```
Will crash the entire plan runner if `chain_client` is None. This is in the hot path of every agent dispatch.

---

## HIGH: Parameter explosion signals missing builder pattern

`dispatch_agent_with` takes 8 parameters with 3 `Option` overrides:
```rust
async fn dispatch_agent_with(
    &mut self,
    plan_id: &str,
    role: AgentRole,
    task: &str,
    prompt_override: Option<String>,
    model_override: Option<String>,
    exec_dir_override: Option<PathBuf>,
    system_prompt_override: Option<String>,
) -> Result<DispatchOutcome>
```

Similar bloat in `attempt_replan` (735 lines), `dispatch_action` (699 lines), `build_context_assembler_sections` (736 lines).

---

## MEDIUM: Inconsistent error handling

Three competing patterns used inconsistently across the file:
1. `let _ = ...` — silently ignore (daimon, conductor, observer: lines 7677, 7929, 9266, 9315, 13024, 14795, 15872)
2. `.ok()` — pragmatic swallowing (lines 407, 460, 509)
3. `.map_err(|e| anyhow!(...))` — proper propagation (lines 777, 829, 832)

The `let _ =` pattern is used on learning/feedback operations, which means **learning pipeline failures are invisible**. If the daimon, conductor, or observer fail, roko silently stops learning without any log message.

---

## MEDIUM: String-based dispatch instead of enums

7 locations use `.as_str()` matching where enum variants would be type-safe:
- `if cfg.command == "claude"` (line 1631)
- `match task_def.map(|td| td.tier.as_str())` (line 2825)
- `match verdict.gate.as_deref()` (line 13731) — should be `GatePhase` enum
- `match kind.as_str()` (lines 16278, 16334) — should be `SourceKind` enum

---

## ROOT CAUSE: Runner batch execution model

The 5 runners executed 661 batches of parallel codex agents. Each agent was given a focused task ("wire X into Y", "add Z to dispatch"), optimizing for local correctness. No agent had the mandate or context to refactor globally.

The runners used anti-pattern checks (no trait duplication, no dead imports) but nothing checked:
- Function length
- Parameter count
- Code duplication within a file
- Cross-function responsibility overlap

**Result:** Each batch added 50-200 lines to the existing god-file functions, creating a 22K-line file where no single developer can hold the full dispatch flow in their head.
