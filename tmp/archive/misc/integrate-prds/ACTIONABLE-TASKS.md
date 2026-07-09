# Actionable Task Checklist

Ordered tasks you can hand to Claude or Codex. Each task is self-contained.
Work top-to-bottom. Check off as you go.

Every task has **Verify** commands that MUST all pass (exit 0) before marking done.

Reference docs are in this same directory if agents need context.

---

## Batch 1: Mechanical Renames (no logic changes)

### [x] 1.1 Rename bardo-runtime → roko-runtime

```
Rename crates/bardo-runtime/ to crates/roko-runtime/.
Update the crate name in crates/roko-runtime/Cargo.toml to "roko-runtime".
Update the workspace Cargo.toml members list.
Update all Cargo.toml files that depend on bardo-runtime to use roko-runtime.
Find-replace "bardo_runtime" → "roko_runtime" in all .rs files.
Do NOT change any logic, function signatures, or behavior.
```

Files to touch: Cargo.toml, crates/roko-cli/Cargo.toml, crates/roko-core/Cargo.toml, crates/roko-serve/Cargo.toml, apps/mirage-rs/Cargo.toml, all .rs files with `bardo_runtime`

**Verify:**
```bash
# No old name in any Cargo.toml
! grep -rn 'bardo.runtime' crates/ Cargo.toml --include='*.toml' | grep -v target/
# No old import in any .rs file
! grep -rn 'bardo_runtime' crates/ --include='*.rs' | grep -v target/
# New crate directory exists
test -d crates/roko-runtime
# Old crate directory gone
! test -d crates/bardo-runtime
# Workspace compiles
cargo check --workspace
# Existing tests still pass
cargo test -p roko-runtime
```

### [x] 1.2 Rename bardo-primitives → roko-primitives

```
Same pattern as 1.1 but for bardo-primitives → roko-primitives.
Update crate directory, Cargo.toml name, workspace members, all dependent Cargo.toml files,
and all "bardo_primitives" → "roko_primitives" in .rs files.
Do NOT change any logic, function signatures, or behavior.
```

Files to touch: Cargo.toml, crates/roko-core/Cargo.toml, crates/roko-compose/Cargo.toml, crates/roko-dreams/Cargo.toml, crates/roko-fs/Cargo.toml, crates/roko-learn/Cargo.toml, crates/roko-neuro/Cargo.toml, crates/roko-serve/Cargo.toml

**Verify:**
```bash
! grep -rn 'bardo.primitives' crates/ Cargo.toml --include='*.toml' | grep -v target/
! grep -rn 'bardo_primitives' crates/ --include='*.rs' | grep -v target/
test -d crates/roko-primitives
! test -d crates/bardo-primitives
cargo check --workspace
cargo test -p roko-primitives
```

### [x] 1.3 Update workspace metadata

```
In the root Cargo.toml, update:
- authors from "engineering@bardo.run" to "engineering@roko.dev"
- repository from "github.com/wpank/bardo" to "https://github.com/nunchi/roko"
- homepage same as repository
Do the same in crates/roko-serve/Cargo.toml if it has its own authors field.
```

**Verify:**
```bash
! grep -n 'bardo.run' Cargo.toml crates/roko-serve/Cargo.toml
! grep -n 'wpank/bardo' Cargo.toml crates/roko-serve/Cargo.toml
grep -q 'roko.dev' Cargo.toml
cargo check --workspace
```

---

## Batch 2: Dissolve roko-golem (structural)

### [x] 2.1 Move hypnagogia from roko-golem to roko-dreams

```
Copy crates/roko-golem/src/hypnagogia.rs to crates/roko-dreams/src/hypnagogia.rs.
Add "pub mod hypnagogia;" to crates/roko-dreams/src/lib.rs.
Update the copied file's imports to reference roko-dreams or roko-core directly
(not roko-golem). Do NOT delete the original yet.
```

**Verify:**
```bash
test -f crates/roko-dreams/src/hypnagogia.rs
grep -q 'pub mod hypnagogia' crates/roko-dreams/src/lib.rs
! grep -q 'roko_golem' crates/roko-dreams/src/hypnagogia.rs
cargo check -p roko-dreams
```

### [x] 2.2 Move chain_witness from roko-golem to roko-chain

```
Copy crates/roko-golem/src/chain_witness.rs to crates/roko-chain/src/witness.rs.
Add "pub mod witness;" to crates/roko-chain/src/lib.rs.
Update the copied file's imports to reference roko-chain or roko-core directly.
```

**Verify:**
```bash
test -f crates/roko-chain/src/witness.rs
grep -q 'pub mod witness' crates/roko-chain/src/lib.rs
! grep -q 'roko_golem' crates/roko-chain/src/witness.rs
cargo check -p roko-chain
```

### [x] 2.3 Remove roko-golem dependency from roko-dreams

```
In crates/roko-dreams/Cargo.toml, remove the roko-golem dependency line.
In crates/roko-dreams/src/lib.rs, remove any "pub use roko_golem::" re-exports.
Replace with local definitions or imports from roko-daimon/roko-chain as needed.
If types from roko-golem (DreamsEngine, HypnagogiaEngine) are re-exported,
either define them locally or remove the re-export.
```

**Verify:**
```bash
! grep -q 'roko.golem\|roko_golem' crates/roko-dreams/Cargo.toml
! grep -q 'roko_golem' crates/roko-dreams/src/lib.rs
! grep -rn 'roko_golem' crates/roko-dreams/src/ --include='*.rs'
cargo check -p roko-dreams
```

### [x] 2.4 Remove roko-golem dependency from roko-learn

```
In crates/roko-learn/Cargo.toml, remove the roko-golem dependency line.
Fix any imports — replace roko_golem references with roko_daimon or roko_dreams.
```

**Verify:**
```bash
! grep -q 'roko.golem\|roko_golem' crates/roko-learn/Cargo.toml
! grep -rn 'roko_golem' crates/roko-learn/src/ --include='*.rs'
cargo check -p roko-learn
```

### [x] 2.5 Remove roko-golem dependency from roko-serve

```
In crates/roko-serve/Cargo.toml, remove the roko-golem dependency line and any
scaffold feature reference.
Fix any imports in roko-serve source files.
```

**Verify:**
```bash
! grep -q 'roko.golem\|roko_golem' crates/roko-serve/Cargo.toml
! grep -rn 'roko_golem' crates/roko-serve/src/ --include='*.rs'
cargo check -p roko-serve
```

### [x] 2.6 Delete roko-golem crate

```
Remove "crates/roko-golem" from workspace Cargo.toml members list.
Delete the crates/roko-golem/ directory entirely.
```

**Verify:**
```bash
! test -d crates/roko-golem
! grep -q 'roko-golem' Cargo.toml
! grep -rn 'roko.golem\|roko_golem' crates/ --include='*.toml' --include='*.rs' | grep -v target/
cargo check --workspace
cargo test --workspace 2>&1 | tail -5
# Should show "test result: ok" or warnings, NOT errors
```

---

## Batch 3: Wire dormant safety layer

### [x] 3.1 Wire SafetyLayer into orchestrate.rs agent dispatch

```
In crates/roko-cli/src/orchestrate.rs, the SafetyLayer and ToolDispatcher are imported
but never instantiated. Wire them:

1. In PlanRunner::new() or init, create a SafetyLayer from the default config
2. Before each agent dispatch (in dispatch_agent_with), run safety pre-checks:
   - ScrubPolicy on the prompt (redact any leaked secrets)
3. After each agent result, run safety post-checks:
   - ScrubPolicy on the output (redact any secrets in agent response)

The safety modules are at crates/roko-agent/src/safety/ (bash.rs, git.rs, network.rs,
path.rs, scrub.rs, rate_limit.rs). They're fully tested — just need to be called.

Start with ScrubPolicy only (simplest, highest value). Other guards can be wired later.
```

**Verify:**
```bash
# SafetyLayer or ScrubPolicy is constructed (not just imported)
grep -n 'ScrubPolicy::new\|SafetyLayer::new\|Scrub.*::default' crates/roko-cli/src/orchestrate.rs
# It's called before or after dispatch
grep -n 'scrub\|safety.*check\|safety.*pre\|safety.*post' crates/roko-cli/src/orchestrate.rs
cargo check -p roko-cli
cargo test -p roko-agent --lib -- scrub
```

### [x] 3.2 Wire Conductor into executor loop

```
In crates/roko-cli/src/orchestrate.rs, instantiate the Conductor from roko-conductor
and call it after each task completes.

1. Create a Conductor instance in PlanRunner (or where the executor loop runs)
2. After each task result (success or failure), feed the result to the conductor
3. Before dispatching the next task, check CircuitBreaker::is_broken(plan_id)
   — if broken, skip remaining tasks and fail the plan

See crates/roko-conductor/src/lib.rs for the Conductor API.
See crates/roko-conductor/src/circuit_breaker.rs for CircuitBreaker.
```

**Verify:**
```bash
# Conductor is constructed
grep -n 'Conductor::new\|conductor.*=.*Conductor' crates/roko-cli/src/orchestrate.rs
# CircuitBreaker is checked
grep -n 'is_broken\|circuit_breaker' crates/roko-cli/src/orchestrate.rs
cargo check -p roko-cli
cargo test -p roko-conductor
```

---

## Batch 4: Wire dormant subsystems into orchestrator

### [x] 4.1 Wire task.verify commands after agent completion

```
In orchestrate.rs, after an agent completes a task, run the [[task.verify]] commands
from tasks.toml. The task_parser already parses these into a verify field on TaskDef.

For each verify entry:
1. Run the shell command via std::process::Command
2. Check exit code — 0 = pass, nonzero = fail
3. If any verify fails, mark the task as failed with the fail_msg
4. Log the verify results

This is the most impactful single wiring change — it makes the executor actually
check that tasks did what they claimed.
```

**Verify:**
```bash
# Verify commands are executed (look for Command::new or similar with verify)
grep -n 'verify\|task\.verify\|verify.*command\|verify.*phase' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# The task_parser's verify field is accessed
grep -n '\.verify' crates/roko-cli/src/orchestrate.rs | head -5
cargo check -p roko-cli
# Create a test tasks.toml with a verify command and confirm it runs
```

### [x] 4.2 Wire model_hint from tasks.toml into agent dispatch

```
The task parser reads model_hint from tasks.toml but it's never passed to dispatch.
In dispatch_agent_with(), check if the task has a model_hint field and use it as
the model override. Fall back to the default model from roko.toml if not set.
```

**Verify:**
```bash
# model_hint is read from task and passed to dispatch
grep -n 'model_hint' crates/roko-cli/src/orchestrate.rs
# It's used in agent creation or model selection
grep -n 'model_hint\|model_override' crates/roko-cli/src/orchestrate.rs | grep -v '//'
cargo check -p roko-cli
```

### [x] 4.3 Wire task.context.read_files into system prompt

```
The task parser reads [task.context].read_files but they're never injected into the
agent's context. In the system prompt assembly (RoleSystemPromptSpec or the context
layer in dispatch_agent_with):

1. For each read_files entry, read the file at the specified path
2. If lines are specified (e.g., "40-80"), extract only those lines
3. Include as inline code blocks in the context layer, prefixed with the "why" field
4. Respect token budget — truncate if total exceeds context tier budget

This is the key change that gives fresh agents the surgical context they need.
```

**Verify:**
```bash
# read_files are accessed from task context
grep -n 'read_files\|context.*read\|context.*files' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# File content is read and injected
grep -n 'std::fs::read_to_string\|read_file\|inline.*context' crates/roko-cli/src/orchestrate.rs | head -5
cargo check -p roko-cli
# Create a test tasks.toml with read_files and verify agent prompt includes file content
```

### [x] 4.4 Wire SkillLibrary: extract on success, inject on dispatch

```
roko-learn has a SkillLibrary that exists but is never used.
1. Add a SkillLibrary field to PlanRunner (or create one in the executor init)
2. After a task succeeds all gates, call skill_library.extract_skill(task, result)
3. Before dispatching a task, call skill_library.query(task_description) and inject
   any matching skills into the system prompt context layer
```

**Verify:**
```bash
# SkillLibrary is constructed
grep -n 'SkillLibrary\|skill_library' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# It's used for both extract and query
grep -c 'skill_library' crates/roko-cli/src/orchestrate.rs
# Should be >= 3 (construction + extract + query)
cargo check -p roko-cli
```

### [x] 4.5 Wire CascadeRouter feedback after task completion

```
CascadeRouter exists in roko-learn with an observe() method for bandit feedback.
After each task succeeds or fails:
1. Get the model that was used for the task
2. Call cascade_router.observe(model, outcome) with success/failure + cost + latency
This enables the router to learn which models work best for which task types.
```

**Verify:**
```bash
# cascade_router.observe or similar feedback call exists
grep -n 'cascade.*observe\|router.*feedback\|router.*observe' crates/roko-cli/src/orchestrate.rs | grep -v '//'
cargo check -p roko-cli
cargo test -p roko-learn -- cascade
```

---

## Batch 5: Executor improvements

### [x] 5.1 Wire re-planning on gate failure with model escalation

```
ReplanStrategy enum exists in roko-orchestrator/src/replan.rs but is never used.
When a task fails gates:
1. Check if retries remain (task.max_retries, default 3)
2. On retry, escalate the model: haiku → sonnet → opus
3. Include the gate failure message in the retry prompt as context
4. If max retries exhausted, mark task as permanently failed
Do NOT change ReplanStrategy enum — just use the escalation path.
```

**Verify:**
```bash
# Retry logic exists with model escalation
grep -n 'retry\|escalat\|max_retries\|fallback.*model' crates/roko-cli/src/orchestrate.rs | grep -v '//' | head -10
# Gate failure message is included in retry context
grep -n 'fail.*message\|gate.*error\|error.*context\|retry.*prompt' crates/roko-cli/src/orchestrate.rs | grep -v '//' | head -5
cargo check -p roko-cli
```

### [x] 5.2 Wire auto plan generation on PRD promote

```
In crates/roko-cli/src/main.rs, find the PrdDraftCmd::Promote handler.
After successfully moving the file from drafts/ to published/:
1. Check if roko.toml has [prd] auto_plan = true (or default to true)
2. If so, call the same plan generation logic as "roko prd plan <slug>"
3. Print a message: "Auto-generating plan for {slug}..."
If auto_plan is false or the plan generation fails, just print the promotion success.
```

**Verify:**
```bash
# Promote handler references plan generation
grep -A 20 'Promote' crates/roko-cli/src/main.rs | grep -i 'plan\|auto\|generate'
# auto_plan config is checked
grep -n 'auto_plan' crates/roko-cli/src/main.rs crates/roko-core/src/config/schema.rs
cargo check -p roko-cli
# Manual test: promote a draft and see if plan generation is attempted
```

### [x] 5.3 Implement worktree isolation per task

```
roko-orchestrator/src/worktree.rs has git worktree management but is never called.
In the dispatch loop in orchestrate.rs:
1. Before dispatching a task, call worktree::create(task_id) to get an isolated worktree
2. Set the agent's working directory to the worktree path
3. After the task succeeds all gates, merge the worktree back to the main branch
4. On failure, remove the worktree (discard changes)
5. Make this opt-in via roko.toml [executor] use_worktrees = true (default false for now)
```

**Verify:**
```bash
# Worktree module is imported and called
grep -n 'worktree' crates/roko-cli/src/orchestrate.rs | grep -v '//' | head -5
# Config option exists
grep -n 'use_worktrees\|worktree' crates/roko-core/src/config/schema.rs | head -3
cargo check -p roko-cli
# When disabled (default), existing behavior unchanged
cargo test -p roko-orchestrator -- worktree
```

---

## Batch 6: Core type extensions

### [x] 6.1 Add 3 extended score axes

```
Completed in `crates/roko-core/src/score.rs` and downstream scorers.

Final state:
- `Score` now carries `precision`, `salience`, and `coherence`
- The score formula already incorporates the new axes where appropriate
- Defaults preserve backward compatibility for existing code paths
```

### [x] 6.2 Add knowledge tier field to KnowledgeEntry

```
Completed in `crates/roko-neuro/src/lib.rs` and `crates/roko-neuro/src/knowledge_store.rs`.

Final state:
- `KnowledgeTier` exists with Transient, Working, Consolidated, Persistent
- `KnowledgeEntry` carries a `tier` field with a transient default
- Tier multiplier and effective half-life helpers are implemented
```

### [x] 6.3 Reconcile knowledge types

```
Completed in `crates/roko-neuro/src/lib.rs`, `distiller.rs`, `tier_progression.rs`,
`knowledge_store.rs`, plus downstream callers in `roko-cli`, `roko-compose`,
and `roko-dreams`.

Final state:
- Canonical code-facing variants now match the PRD: `Insight`, `Heuristic`,
  `Warning`, `CausalLink`, `StrategyFragment`, `AntiKnowledge`
- Legacy names (`Fact`, `Procedure`, `Playbook`, `Constraint`) were retired
  from the enum itself and are preserved only as serde aliases for backward
  compatibility with old JSONL data
- Default half-lives are wired for the PRD-native kinds
- Distillation, tier progression, dream synthesis, context assembly, and CLI
  retrieval now emit/query the PRD-native kinds
```

**Verify:**
```bash
# New variants exist
grep -E 'Warning|CausalLink|StrategyFragment' crates/roko-neuro/src/lib.rs crates/roko-neuro/src/knowledge_store.rs
# Old variants still compile (serde alias or kept)
grep -E 'Fact|Procedure|Playbook|Constraint' crates/roko-neuro/src/lib.rs crates/roko-neuro/src/knowledge_store.rs
cargo check --workspace
cargo test -p roko-neuro
# Deserialization backward compat: old JSON with "Fact" still works
```

---

## Batch 7: Signal → Engram rename (BIG, do last in Phase A)

### [x] 7.1 Rename Signal → Engram in roko-core with compat alias

```
Completed in `crates/roko-core/src/engram.rs` / `crates/roko-core/src/lib.rs`.

Original plan:
- In crates/roko-core/src/signal.rs:
- Rename struct Signal → Engram
- Rename SignalBuilder → EngramBuilder
- Add compat aliases: pub type Signal = Engram; pub type SignalBuilder = EngramBuilder;
- Rename file: signal.rs → engram.rs
- Update mod declaration in lib.rs: mod signal → mod engram
- Update pub use: pub use engram::{Engram, EngramBuilder, Signal, SignalBuilder};
Do NOT touch any other crates — the compat aliases keep everything compiling.
```

**Verify:**
```bash
# Engram struct exists
grep 'pub struct Engram' crates/roko-core/src/engram.rs
# File renamed
test -f crates/roko-core/src/engram.rs
! test -f crates/roko-core/src/signal.rs
# Module declaration updated
grep 'mod engram' crates/roko-core/src/lib.rs
# ENTIRE workspace still compiles via compat alias
cargo check --workspace
cargo test -p roko-core -- --nocapture
```

### [x] 7.2 Update all consumer crates to use Engram

```
For each crate that imports Signal, replace with Engram:
- use roko_core::Signal → use roko_core::Engram
- Signal:: → Engram::
- SignalBuilder:: → EngramBuilder::
- fn signatures: Signal → Engram
- Variable names: signal → engram (where it's a type reference)

Completed across the Rust workspace. The public type is now `Engram` end-to-end.
```

**Verify (per crate):**
```bash
# No remaining Signal references (except test assertions comparing strings)
! grep -n 'use roko_core::Signal\b' crates/<CRATE>/src/**/*.rs 2>/dev/null
! grep -n 'Signal::builder\|Signal::' crates/<CRATE>/src/**/*.rs 2>/dev/null | grep -v 'Kind::Signal\|"Signal"'
cargo check -p <CRATE>
```

**Verify (all done):**
```bash
# No remaining Signal type references in Rust code
REFS=$(grep -rn '\bSignal\b' crates/ apps/ --include='*.rs' | grep -v target/ | grep -v 'Kind::Signal' | grep -v '"Signal"' | grep -v '// ')
test -z "$REFS" || echo "Remaining Signal references: $REFS"
cargo check --workspace
```

### [x] 7.3 Remove compat alias

```
In crates/roko-core/src/engram.rs, remove:
  pub type Signal = Engram;
  pub type SignalBuilder = EngramBuilder;
In crates/roko-core/src/lib.rs, remove Signal and SignalBuilder from pub use.
```

**Verify:**
```bash
! grep 'pub type Signal' crates/roko-core/src/engram.rs
! grep 'pub type SignalBuilder' crates/roko-core/src/engram.rs
cargo check --workspace
cargo test --workspace 2>&1 | tail -3
```

---

## Batch 8: Wire Neuro + Daimon into runtime

### [x] 8.1 Wire Neuro query into context assembly

```
Completed in `crates/roko-cli/src/orchestrate.rs`.

Current behavior:
When assembling context for a task (in orchestrate.rs dispatch_agent_with):
1. If a NeuroStore is available, query it with the task title/description
2. Take the top 3-5 results (sorted by relevance × tier_multiplier)
3. Format each as "## Learned: {kind}\n{content}\n(confidence: {confidence})"
4. Append to the context layer in the system prompt
5. If no NeuroStore or no results, skip silently

See crates/roko-neuro/src/lib.rs for the NeuroStore trait and query() method.
```

**Verify:**
```bash
# NeuroStore is queried during dispatch
grep -n 'neuro.*query\|NeuroStore\|knowledge.*query' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# Results are injected into context/prompt
grep -n 'Learned\|neuro.*context\|knowledge.*context' crates/roko-cli/src/orchestrate.rs | grep -v '//'
cargo check -p roko-cli
```

### [x] 8.2 Wire Neuro write on task success

```
Completed in `crates/roko-cli/src/orchestrate.rs`.

Current behavior:
After a task passes all gates in orchestrate.rs:
1. Create a KnowledgeEntry from the task result:
   - kind: Insight (for successful patterns) or Heuristic (for reusable approaches)
   - content: summary of what the agent did and why it worked
   - tier: KnowledgeTier::Transient (start low, promote on reuse)
   - source_episodes: link to the episode ID
2. Call neuro_store.ingest(entry)
3. If ingest fails, log warning but don't fail the task

This closes the learn-from-success loop.
```

**Verify:**
```bash
# neuro_store.ingest or similar write call exists after gate success
grep -n 'neuro.*ingest\|knowledge.*write\|knowledge.*store.*put' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# KnowledgeEntry is constructed
grep -n 'KnowledgeEntry' crates/roko-cli/src/orchestrate.rs | grep -v 'use\|//'
cargo check -p roko-cli
```

### [x] 8.3 Wire Daimon appraise on gate results

```
Completed in `crates/roko-cli/src/orchestrate.rs`.

Current behavior:
After each gate verdict in orchestrate.rs:
1. If a DaimonState is available, call daimon.appraise(event) with:
   - AffectEvent::GateResult { passed: verdict.passed, confidence: verdict.score }
2. On task completion, call:
   - AffectEvent::TaskOutcome { success: all_gates_passed }
3. Persist daimon state after appraisal (daimon.persist())
4. If no daimon configured, skip silently

See crates/roko-daimon/src/lib.rs for AffectEvent variants and appraise().
```

**Verify:**
```bash
# Daimon appraise is called
grep -n 'daimon.*appraise\|affect.*appraise\|AffectEvent' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# Both GateResult and TaskOutcome events are sent
grep -c 'AffectEvent' crates/roko-cli/src/orchestrate.rs
# Should be >= 2
cargo check -p roko-cli
```

### [x] 8.4 Wire Daimon modulate into model selection

```
Completed in `crates/roko-cli/src/orchestrate.rs`.

Current behavior:
Before selecting a model for a task in orchestrate.rs:
1. If a DaimonState is available, call daimon.modulate(base_params)
2. base_params comes from the task's model_hint or default config
3. The modulated result may change the model tier:
   - Low confidence/dominance → escalate to stronger model
   - High pleasure/confidence → use cheaper model
4. Use the modulated model for dispatch
5. If no daimon, use base_params unchanged

See crates/roko-daimon/src/lib.rs for DispatchParams and modulate().
```

**Verify:**
```bash
# Daimon modulate is called before dispatch
grep -n 'daimon.*modulate\|affect.*modulate\|DispatchParams' crates/roko-cli/src/orchestrate.rs | grep -v '//'
# Model selection uses modulated params
grep -B 5 -A 5 'modulate' crates/roko-cli/src/orchestrate.rs | grep 'model'
cargo check -p roko-cli
```

---

## How to Use This

### With Claude Code (this tool):
Copy a task description and say "do this". Claude will read the relevant files, make changes, and run the verify commands.

### With Claude in a separate terminal:
```bash
claude -p "$(cat <<'EOF'
[paste task description AND verify section here]

Workspace: /Users/will/dev/nunchi/roko/roko
After changes, run every Verify command. All must pass (exit 0).
EOF
)" --dangerously-skip-permissions
```

### Batch parallelism:
- Batch 1: sequential (1.1 → 1.2 → 1.3)
- Batch 2: 2.1-2.5 parallel, then 2.6
- Batch 3-5: mostly independent
- Batch 6: independent of each other
- Batch 7: sequential (7.1 → 7.2 → 7.3)
- Batch 8: independent of each other, depends on Batch 6.2
