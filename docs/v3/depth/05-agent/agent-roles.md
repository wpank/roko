# 05-agent/agent-roles -- Agent Roles

> The 28-role AgentRole enum, per-role defaults (tier, budget, permissions),
> role-to-template mapping, graduated autonomy, and configuration override.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-core/src/agent.rs` (AgentRole, ModelTier, TurnBudget,
ToolPermissions), `crates/roko-compose/src/templates/` (role prompt templates)

---

## 1. Purpose of Roles

Every agent in Roko is assigned a **role** -- a named persona that determines:

1. **Capability scoping** -- A `QuickReviewer` gets read-only tools; an
   `Implementer` gets read + write + exec. Enforced by the `ToolDispatcher`.
2. **Model routing** -- Each role has a default `ModelTier` (Fast/Standard/Premium)
   used as the CascadeRouter's starting point.
3. **Budget control** -- Each role has a per-turn dollar ceiling (`TurnBudget`)
   that prevents runaway spending.
4. **Prompt persona** -- Each role maps to one of 11 compose templates that
   provide role-specific system prompt content through the 9-layer
   `SystemPromptBuilder`.

---

## 2. The 28 Roles

The `AgentRole` enum at `crates/roko-core/src/agent.rs` defines 28 variants.
`AgentRole::ALL_AGENTS` is a const array of all 27 working roles (everything
except `Conductor`, which is a meta-watcher).

### Meta / Orchestration

| Role | Tier | Description |
|------|------|-------------|
| **Conductor** | Premium | Meta-orchestrator; watches agents, intervenes |
| **Strategist** | Standard | Writes plan briefs, decomposes PRDs into tasks |

### Implementation

| Role | Tier | Description |
|------|------|-------------|
| **Implementer** | Standard | Writes code (the main coding agent) |
| **Refactorer** | Standard | Structural rewrite without behavior change |
| **AutoFixer** | Fast | Lightweight patcher after gate failure |

### Review / Audit

| Role | Tier | Description |
|------|------|-------------|
| **Architect** | Premium | Reviews architecture before implementation |
| **Auditor** | Premium | Post-implementation review for correctness and safety |
| **QuickReviewer** | Fast | Single-pass reviewer for Standard-complexity plans |
| **Critic** | Standard | Devil's advocate / alternative-approach reviewer |

### Research / Knowledge

| Role | Tier | Description |
|------|------|-------------|
| **Researcher** | Standard | Broad research reader (docs, code, external) |
| **Scribe** | Standard | Drafts documentation |

### Validation

| Role | Tier | Description |
|------|------|-------------|
| **PrePlanner** | Fast | Validates pre-plan artifacts before enrichment |
| **DocVerifier** | Fast | Verifies docs still match code after edits |
| **IntegrationTester** | Standard | Integration-level tests against live system |
| **TerminalValidator** | Fast | Tests CLI/terminal entry points end-to-end |
| **LifecycleTester** | Standard | Exercises agent lifecycle (spawn/tick/teardown) |
| **CrossSystemTester** | Standard | Tests cross-system flows across boundaries |
| **FullLoopValidator** | Premium | Validates end-to-end pipeline |

### Observation / Tracking

| Role | Tier | Description |
|------|------|-------------|
| **SpecDriftDetector** | Fast | Detects divergence between PRD and implementation |
| **RegressionDetector** | Fast | Watches for regression in test-pass rate and cost |
| **PerformanceSentinel** | Fast | Tracks performance metrics across runs |
| **CoverageTracker** | Fast | Tracks coverage/rung satisfaction |
| **SnapshotComparator** | Fast | Compares snapshots across runs for drift |

### Management

| Role | Tier | Description |
|------|------|-------------|
| **PlanLifecycleManager** | Standard | Manages plan lifecycle state transitions |
| **MergeResolver** | Standard | Resolves merge conflicts across workstreams |
| **ErrorDiagnoser** | Standard | Diagnoses errors into actionable root causes |
| **DependencyValidator** | Fast | Validates dependency additions/upgrades |
| **PatternExtractor** | Standard | Extracts reusable patterns from completed work |

---

## 3. ModelTier

```rust
pub enum ModelTier {
    Fast,      // Haiku-class: classification, watchers, orchestration
    Standard,  // Sonnet-class: implementation, review (the workhorse)
    Premium,   // Opus/GPT-5-class: architecture, hard debugging
}
```

The tier is a hint to the CascadeRouter's starting point. As the LinUCB bandit
learns which models succeed for which tasks, it may promote or demote a role's
effective tier. Defaults are conservative: implementation starts at Standard,
orchestration overhead starts at Fast.

---

## 4. TurnBudget

Per-turn spending cap:

```rust
pub struct TurnBudget {
    pub base_usd: f32,
    pub multiplier: f32,
}
```

The `multiplier` adjusts for model escalation: when the CascadeRouter escalates
from Standard to Premium, the budget is multiplied by the escalation ratio to
account for higher per-token cost. De-escalation reduces the multiplier.

---

## 5. ToolPermissions

Each role declares permissions checked by the `ToolDispatcher` at step 2
(authorize):

```rust
pub struct ToolPermissions {
    pub read: bool,     // File read, grep, glob
    pub write: bool,    // File write, edit, patch
    pub exec: bool,     // Bash, run_tests
    pub git: bool,      // Git operations
    pub network: bool,  // Web fetch, web search
}
```

The dispatcher checks `def.permission.satisfied_by(&role_perms)` before allowing
any tool call to proceed. A `QuickReviewer` cannot write files even if the model
requests a `write_file` call -- the dispatcher blocks it with
`ToolError::PermissionDenied`.

### Permission groups by role family

| Family | Read | Write | Exec | Git | Network |
|--------|------|-------|------|-----|---------|
| Read-only (Conductor, Reviewers, Auditor, Detectors) | Yes | No | No | No | No |
| Read-write (Implementer, Refactorer, AutoFixer, Scribe) | Yes | Yes | Yes | Yes | No |
| Research (Researcher) | Yes | No | No | No | Yes |
| Validation (Testers, Validators) | Yes | Varies | Yes | No | No |

---

## 6. Graduated Autonomy

Roles implement graduated autonomy (Meta-Harness principle #5):

- **Read-only roles** (Conductor, QuickReviewer, Auditor family): can inspect
  and report but not modify. The SafetyLayer provides a floor that even
  high-autonomy roles cannot breach.
- **Read-write roles** (Implementer, Refactorer, AutoFixer): full file
  operations within worktree boundaries.
- **Meta roles** (Conductor, Strategist): orchestrate but do not implement.

The `AutonomyLevel` enum provides finer-grained control:

```rust
pub enum AutonomyLevel {
    Observe,      // Can only watch
    Suggest,      // Can suggest actions but not execute
    ActReview,    // Can act but requires review
    Guardrails,   // Can act within guardrails
    Full,         // Full autonomy within SafetyLayer floor
}
```

Per-capability autonomy is configured via `AutonomyConfig`:

```rust
pub struct AutonomyConfig {
    pub agent_id: String,
    pub per_capability: HashMap<String, AutonomyLevel>,
    pub default_level: AutonomyLevel,
}
```

---

## 7. Role-to-Template Mapping

The 11 compose templates in `crates/roko-compose/src/templates/` group roles
into behavioral families. The `RoleSystemPromptSpec` maps each role to its
template for the 9-layer `SystemPromptBuilder`.

Templates provide:

- Role persona description
- Behavioral constraints
- Expected output format
- Tool usage guidance specific to the role family

The mapping is many-to-one: multiple roles share a template when their
behavioral requirements overlap (e.g., all validation roles share a testing
template).

---

## 8. Configuration Override

Users can override any role default in `roko.toml`:

```toml
[agent.roles.implementer]
model = "claude-opus-4-6"
tools = ["read_file", "edit_file", "git-*"]
budget = { max_cost_usd_cents_per_turn = 500 }

[agent.roles.conductor]
model = "claude-haiku-4-5"
budget = { max_cost_usd_cents_per_turn = 5 }

[agent.roles.researcher]
model = "sonar-pro"
```

Override hierarchy:

1. Per-task configuration in `tasks.toml` (highest priority)
2. Per-role configuration in `roko.toml`
3. Role defaults in `AgentRole` (lowest priority)

---

## 9. Role Composition

Roles compose into agent types for different task categories:

| Agent type | Roles | Purpose |
|-----------|-------|---------|
| Coding Agent | Implementer + QuickReviewer + IntegrationTester | Standard dev cycle |
| Research Agent | Researcher + Scribe | Deep investigation with citations |
| Architecture Agent | Architect + Strategist + Critic | System-level design |
| Validation Agent | Multiple validators + detectors | Quality assurance |

---

## 10. Citations

1. `crates/roko-core/src/agent.rs` -- AgentRole enum (28 variants),
   AgentBackend, ModelTier, TurnBudget, ToolPermissions, AutonomyLevel,
   AutonomyConfig.
2. `crates/roko-compose/src/templates/` -- 11 role-specific prompt templates.
3. `crates/roko-agent/src/dispatcher/mod.rs` -- Permission enforcement in
   dispatch pipeline.
4. Lee, Y. et al. (2026). "Meta-Harness." arXiv:2603.28052. -- Graduated
   autonomy (principle #5).
