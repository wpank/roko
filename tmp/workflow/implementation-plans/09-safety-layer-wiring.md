# 09 — SafetyLayer: Wire Into the Unified Engine

> Phase 3 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/17-safety-agent-system-audit.md`.

---

## Status (2026-05-01)

**PARTIAL.** SafetyLayer is rich and used by ToolDispatcher. Not threaded through the unified `EffectDriver`. `dangerously_skip_permissions` partially per-role but inconsistent. Per-turn cumulative cost not enforced. Audit's "fail-open" claim is **stale** — current code falls back to `AgentContract::restricted`, not `permissive`.

**What's done:**

- `SafetyLayer` — `crates/roko-agent/src/safety/mod.rs:~800 LOC`
- 8 bundled YAML contracts in `crates/roko-agent/src/safety/contracts/`
- `ToolDispatcher::dispatch` runs all 10 pre-execution checks — `crates/roko-agent/src/dispatcher/mod.rs:~456`
- 13 secret-scrubbing patterns — `crates/roko-agent/src/safety/scrub.rs`
- `applicable_recovery()` exists (`contract.rs`)
- `safety.check_recovery()` invoked after tool execution (`dispatcher/mod.rs:~456-462`)
- Contract loading falls back to `AgentContract::restricted` on missing YAML — **not permissive** as audit feared
- `role_allows_dangerous_skip_permissions(role)` exists in `crates/roko-cli/src/run.rs:~2766`

**What's not:**

- `EffectDriver` does not call `safety.pre_dispatch_check(...)` or `safety.post_dispatch_check(...)`. So `WorkflowEngine`-driven model calls bypass safety entirely.
- `dangerously_skip_permissions: true` is hardcoded in many places (`agent_exec.rs`, `roko-serve/src/dispatch.rs`, parts of `dispatch_v2.rs`)
- `MaxCostPerTurn` checks **estimated** cost only; cumulative spend is a TODO (`contract.rs:~471-482`)
- Safety budget tracker is `Optional` and `None` by default in `SafetyLayer::with_defaults`
- `MCP` config silent failure (per audit § 1)
- Recovery actions only invoked from `ToolDispatcher`; agent-level `applicable_recovery` (e.g. consecutive failure → abort) never fires
- Per-turn rate limiter is global per (role, tool); no per-task scoping

---

## Goal

Every model call dispatched by `EffectDriver` (i.e. by `WorkflowEngine`-driven runs) is bracketed by safety checks. `dangerously_skip_permissions` is opt-in per role, sourced from contracts. Cumulative per-turn spend is enforced. Contract recovery actions fire at the agent level. MCP misconfiguration is loud, not silent.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#4 Features in Wrong Layer** — safety today wraps tools but not the unified driver
- **#3 Build Another Runtime** — `roko-cli/src/run.rs` has its own per-role permission check; `agent_exec.rs` hardcodes; `roko-serve/src/dispatch.rs` independently chooses

---

## Existing Code — Read These First

```rust
// crates/roko-agent/src/safety/mod.rs (sketch)
pub struct SafetyLayer {
    role_tools: HashMap<String, ToolAllowlist>,
    rate_limiter: RateLimiter,
    bash_policy: BashPolicy,
    git_policy: GitPolicy,
    network_policy: NetworkPolicy,
    path_policy: PathPolicy,
    safety_budget: Option<SafetyBudgetTracker>,
    temporal_monitor: TemporalMonitor,
    contracts: ContractRegistry,
}

impl SafetyLayer {
    pub fn pre_dispatch_check(&self, plan_id: &str, task: &TaskDef, role: &str, exec_dir: &Path) -> SafetyResult;
    pub fn post_dispatch_check(&self, plan_id: &str, task: &TaskDef, role: &str, output: &str, changed_files: &[PathBuf]) -> SafetyResult;
    pub fn check_pre_execution(&self, /* tool ctx */) -> SafetyResult;
    pub fn check_recovery(&self, /* outcome */) -> Option<RecoveryAction>;
}

pub fn role_allows_dangerous_skip_permissions(role: &str) -> bool;   // crates/roko-cli/src/run.rs:~2766
```

`AgentContract::restricted` is the safe fallback, **not** `permissive` (audit was based on older code).

---

## Implementation Steps

### Step 1 — Add `SafetyLayer` to `EffectServices`

**File:** `crates/roko-runtime/src/effect_driver.rs`

```rust
pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<dyn AffectPolicy>>,
    pub persistence: Arc<dyn PersistenceService>,           // plan 04
    pub merge_service: Arc<dyn MergeService>,               // plan 07
    pub worktree_service: Option<Arc<dyn WorktreeService>>, // plan 07
    pub safety: Arc<SafetyLayer>,                            // NEW
}
```

The safety field is `SafetyLayer` directly (not behind a trait) because there's only one implementation and the layer is stateful with rate-limit state.

### Step 2 — Bracket every agent spawn with safety checks

```rust
// crates/roko-runtime/src/effect_driver.rs
async fn spawn_for_role(&self, role: &str, task_id: Option<String>, extra_spec: PromptSpec) -> EffectOutcome {
    let task = self.task_for(task_id.as_deref())?;
    let exec_dir = self.workdir.clone();

    // PRE-DISPATCH
    if let Err(safety_err) = self.services.safety.pre_dispatch_check(
        &self.plan_id().unwrap_or_default(),
        &task,
        role,
        &exec_dir,
    ) {
        self.emit(RuntimeEvent::AgentBlocked {
            run_id: self.run_id.clone(),
            agent_id: task_id.unwrap_or(role.into()),
            reason: safety_err.to_string(),
        });
        return EffectOutcome::Failed { error: format!("safety pre-dispatch: {safety_err}") };
    }

    // SCRUB PROMPT BEFORE DISPATCH (not just after)
    let mut spec = self.base_prompt_spec(task_id.clone());
    spec.merge(extra_spec);
    let assembled = self.services.prompt_assembler.assemble(spec).await?;
    let scrubbed_system = self.services.safety.scrub(&assembled.system);

    let req = self.build_model_request(role, task_id.clone(), &assembled).with_system(scrubbed_system);
    let response = self.services.model_caller.call(req).await?;

    // POST-DISPATCH
    let scrubbed_output = self.services.safety.scrub(&response.content);
    let changed_files = detect_files_changed(&scrubbed_output, &exec_dir);
    if let Err(safety_err) = self.services.safety.post_dispatch_check(
        &self.plan_id().unwrap_or_default(),
        &task,
        role,
        &scrubbed_output,
        &changed_files,
    ) {
        // Currently warnings-only per audit; emit but do not fail
        self.emit(RuntimeEvent::SafetyWarning {
            run_id: self.run_id.clone(),
            agent_id: task_id.unwrap_or(role.into()),
            warning: safety_err.to_string(),
        });
    }

    // RECOVERY CHECK (Step 4)
    if let Some(recovery) = self.services.safety.check_agent_recovery(role, &response, &task) {
        return self.apply_recovery(recovery).await;
    }

    EffectOutcome::AgentDone {
        agent_id: task_id.unwrap_or(role.into()),
        output: scrubbed_output,
        tokens_used: response.usage.total_tokens,
        cost_usd: response.usage.cost_usd,
        files_changed: changed_files,
    }
}
```

### Step 3 — Make `dangerously_skip_permissions` opt-in via contract

Today this flag is set ad-hoc per call. Centralize:

```rust
// crates/roko-agent/src/safety/contract.rs
#[derive(Debug, Clone, Deserialize)]
pub struct AgentContract {
    // existing fields
    pub dangerously_skip_permissions: bool,    // NEW: opt-in per contract
    pub max_cumulative_cost_per_turn_usd: Option<f64>,
}
```

YAML example:

```yaml
# crates/roko-agent/src/safety/contracts/scribe.yaml
role: scribe
dangerously_skip_permissions: false        # default
governance:
  - MaxToolCallsPerTurn: 10
  - ForbiddenTools: ["bash"]
  - MaxCostPerTurn: 0.10                   # cumulative: enforced
```

```yaml
# crates/roko-agent/src/safety/contracts/implementer.yaml
role: implementer
dangerously_skip_permissions: true         # explicit opt-in
governance:
  - MaxToolCallsPerTurn: 50
  - MaxCostPerTurn: 1.00
```

Replace ad-hoc reads:

```rust
// crates/roko-cli/src/run.rs
let skip = role_allows_dangerous_skip_permissions(role);   // OLD
// becomes:
let contract = safety.contract_for_role(role);
let skip = contract.dangerously_skip_permissions;          // NEW
```

Then delete `role_allows_dangerous_skip_permissions` once all callers migrated.

### Step 4 — Implement cumulative per-turn spend enforcement

Today `MaxCostPerTurn` is checked against **estimated** cost. The TODO in `contract.rs:~471-482` is to track actual spend.

```rust
// crates/roko-agent/src/safety/budget.rs
pub struct PerTurnSpend {
    by_role: HashMap<String, f64>,                   // role → total spend in current turn
    turn_started_at_ms: u64,
}

impl SafetyLayer {
    pub fn record_call_cost(&mut self, role: &str, cost_usd: f64, contract: &AgentContract) -> SafetyResult {
        let spend = self.per_turn.by_role.entry(role.into()).or_insert(0.0);
        *spend += cost_usd;
        if let Some(limit) = contract.max_cumulative_cost_per_turn_usd {
            if *spend > limit {
                return Err(SafetyError::CostExceeded { role: role.into(), spent: *spend, limit });
            }
        }
        Ok(())
    }

    pub fn reset_turn(&mut self) {
        self.per_turn.by_role.clear();
        self.per_turn.turn_started_at_ms = now_ms();
    }
}
```

`EffectDriver::spawn_for_role` calls `safety.record_call_cost(role, response.usage.cost_usd, contract)?` after a successful call. On `EffectOutcome::Failed { CostExceeded }` the FSM transitions to `Halted`.

`reset_turn()` is called by the driver between user prompts in chat, between phase transitions in plan execution.

### Step 5 — Make MCP misconfiguration loud

**File:** `crates/roko-agent/src/provider/mod.rs` (search for `find_mcp_config`)

Today missing `.mcp.json` returns `Ok(None)`. After:

```rust
pub fn resolve_mcp_config(workdir: &Path, expected: McpExpectation) -> Result<Option<PathBuf>> {
    let found = find_mcp_config(workdir)?;
    match (found, expected) {
        (Some(path), _) => Ok(Some(path)),
        (None, McpExpectation::Required) => Err(SafetyError::McpMissing { workdir: workdir.into() }),
        (None, McpExpectation::Optional) => {
            tracing::warn!(workdir = ?workdir, "no .mcp.json found; agent will run without MCP tools");
            Ok(None)
        }
    }
}

pub enum McpExpectation { Required, Optional }
```

Set per-role: `architect` and `auditor` are `Optional`; `implementer` may be `Required` if the role manifest declares MCP tools as needed.

### Step 6 — Implement agent-level recovery actions

Today `check_recovery` runs after **tool** execution. Add an agent-level variant:

```rust
// crates/roko-agent/src/safety/contract.rs
impl SafetyLayer {
    pub fn check_agent_recovery(
        &self,
        role: &str,
        response: &ModelCallResponse,
        task: &TaskDef,
    ) -> Option<RecoveryAction> {
        let contract = self.contract_for_role(role);
        let history = self.failure_history(&task.id);

        if history.consecutive_failures >= contract.max_consecutive_failures.unwrap_or(3) {
            return Some(RecoveryAction::Alert {
                reason: format!("agent {role} failed {} times consecutively", history.consecutive_failures),
            });
        }
        if response.usage.total_tokens > contract.max_tokens_per_turn.unwrap_or(16_000) as u64 {
            return Some(RecoveryAction::Downgrade {
                from_model: contract.preferred_model.clone(),
                to_model: contract.fallback_model.clone(),
            });
        }
        None
    }
}
```

`EffectDriver::apply_recovery(action)`:

- `Retry` → return `EffectOutcome::Retry` so FSM re-emits `SpawnImplementer`
- `Downgrade` → caller passes `routing_hints: ["downgrade"]` on next call
- `Abort` → `EffectOutcome::Failed`
- `Alert` → `RuntimeEvent::SafetyAlert` + continue (warning, not blocking)

### Step 7 — Audit and lock down `dangerously_skip_permissions`

Find all `.with_dangerously_skip_permissions(true)` and `dangerously_skip_permissions: true` literals:

```bash
rg 'dangerously_skip_permissions' crates/ --type rust
```

Each call must:

- Source the value from `contract.dangerously_skip_permissions` (Step 3)
- Or be inside a test (`#[cfg(test)]`)

After audit, add a clippy lint or compile-time check that prevents reintroduction.

### Step 8 — Tests

```rust
#[tokio::test]
async fn workflow_engine_blocks_path_escape() {
    let services = test_services_with_safety();
    let outcome = effect_driver.execute(PipelineAction::SpawnImplementerForTask {
        task_id: "T1".into(),
        prompt: PromptSpec { workdir: Some(PathBuf::from("/etc/")), ..Default::default() },
    }).await;
    assert!(matches!(outcome, EffectOutcome::Failed { .. }));
    assert!(outcome.error().contains("path"));
}

#[tokio::test]
async fn cumulative_cost_per_turn_enforced() {
    let mut safety = SafetyLayer::with_defaults();
    let contract = AgentContract {
        max_cumulative_cost_per_turn_usd: Some(1.0), ..base()
    };
    safety.record_call_cost("implementer", 0.6, &contract).unwrap();
    let res = safety.record_call_cost("implementer", 0.5, &contract);
    assert!(matches!(res, Err(SafetyError::CostExceeded { .. })));
}

#[tokio::test]
async fn dangerously_skip_permissions_sourced_from_contract() {
    // Constructed via spawn pipeline; no direct call
    let outcome = run_workflow_with_role("scribe", "task: write docs").await;
    assert!(!outcome.dispatch.dangerously_skip_permissions);
    let outcome = run_workflow_with_role("implementer", "task: refactor").await;
    assert!(outcome.dispatch.dangerously_skip_permissions);
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #1 Just shell out | Bypassing safety by spawning claude directly | Plan 01 makes this impossible; Plan 09 ensures the unified path is also safety-bracketed |
| #4 Features in wrong layer | Adding cost tracking inside `ModelCallService` | Cost tracking is a safety concern; lives in `SafetyLayer::record_call_cost` |
| #5 Hardcoded role behavior | `if role == "implementer" { skip = true }` | All role-specific safety flags come from contracts |

---

## Things NOT To Do

1. **Don't make safety checks blocking on the call hot path.** Pre-dispatch is fast (<5ms typical). Post-dispatch scrubs can be slower; spawn for non-critical post-checks.
2. **Don't fail open on contract load errors.** Current code falls back to `AgentContract::restricted` (audit was wrong about `permissive`). Verify and **keep restricted** — never `permissive`.
3. **Don't allow contract `dangerously_skip_permissions: true` for `auditor`, `quick_reviewer`, `scribe`, or `architect` roles.** These are read-only by definition. Add a YAML lint that catches this.
4. **Don't expose `SafetyLayer` mutable state across threads without `Mutex`.** Rate limiter and `PerTurnSpend` are mutable; protect them.
5. **Don't put scrubber regex in multiple places.** One pattern set in `safety/scrub.rs`. Adding patterns is a single PR.
6. **Don't ignore safety post-warnings.** Even though they don't block, emit `RuntimeEvent::SafetyWarning` so they show up in dashboards and audit trails.
7. **Don't skip safety for "internal" callers like distillation or dreams.** Every model call is bracketed. Distillation processes user data; safety scrubs apply.
8. **Don't reset per-turn spend mid-task.** Plan execution turns = entire task lifecycle, not individual model calls.

---

## Tests / Proof Criteria

```bash
# 1. EffectDriver bracketed by safety
rg 'safety\.(pre_dispatch_check|post_dispatch_check|scrub|record_call_cost)' crates/roko-runtime/src/effect_driver.rs
# expected: 4+ matches

# 2. dangerously_skip_permissions only sourced from contract
rg 'dangerously_skip_permissions' crates/ --type rust | grep -v 'safety/contract' | grep -v test
# expected: 0 (or only via `contract.dangerously_skip_permissions` access)

# 3. Cumulative cost enforced
rg 'fn record_call_cost' crates/roko-agent/src/safety/
# expected: 1+ implementation

# 4. No permissive contract fallback
rg 'AgentContract::permissive' crates/ --type rust | grep -v test
# expected: 0
```

Functional proofs:

- [ ] All 3 unit tests above pass
- [ ] `roko run` with a malicious prompt that tries to write `/etc/passwd` is blocked at pre-dispatch
- [ ] After 3 consecutive failures of a role, recovery action `Alert` fires + emits `RuntimeEvent::SafetyAlert`
- [ ] Cumulative cost > `MaxCostPerTurn` triggers `CostExceeded` error halting the workflow
- [ ] Role `auditor` cannot dispatch `bash` tool (verify via prompt + observed model behavior)
- [ ] Missing `.mcp.json` for a role marked `Required` errors loudly at startup; for `Optional` warns
- [ ] Secret in agent output is scrubbed (verify in `.roko/episodes.jsonl` content field)

---

## Dependencies

- **Plan 07 (EffectDriver)** — same surface, must land together
- **Plan 03 (FeedbackService)** — for cost data flowing into safety per-turn budget

---

## Estimated Effort

**M.** ~1-1.5 weeks.

- Step 1 (services wiring) — S (1 day)
- Step 2 (driver bracketing) — M (2 days)
- Step 3 (contract opt-in) — M (2 days, lots of YAML + caller updates)
- Step 4 (cumulative spend) — S (1 day)
- Step 5 (MCP loud) — S (half day)
- Step 6 (agent recovery) — S (1 day)
- Step 7 (audit + lockdown) — M (2 days, mostly grep + replace)
- Step 8 (tests) — S (1 day)
