# 12 — Tier 2: Delete Dead Code (6 items, all OPEN)

Pure subtraction. Net negative LOC. All six items are independent and can run
in parallel. ~1 session total.

**Why this tier first**: dead code disguises real intent and inflates the
audit surface. Deleting clears the field for T3-T5 work and reduces the
chance of an agent accidentally re-wiring an obsolete module.

**Source**: doc 41 backlog T2-16..T2-21; doc 37 (learning dead code); doc 39
(config phantom fields).

---

## Cross-Cutting Notes

### Anti-patterns to enforce

1. **Don't add `#[deprecated]` and ship.** Just delete. Deprecation is for
   public APIs with external consumers; this code has none.
2. **Don't comment out files.** Delete or `git rm` them.
3. **Don't keep "for reference."** Git history retains what you delete.
4. **Don't add new wiring to "justify" a module marked dead.** If a module
   has no callers, it goes; do not "rescue" it by writing a caller in the
   same PR.
5. **One module deletion per commit.** Easier to bisect a regression. Bundle
   only obviously-cohesive groups (e.g. all 4 orphan files in T2-16, since
   none are in `lib.rs`).

### Pre-deletion safety check (mandatory for every item)

```bash
# 1. Confirm the symbol/module has no callers outside its own crate
rg '<crate_name>::<module_name>' crates/ -g '*.rs' | rg -v 'crates/<crate_name>/'

# 2. Confirm no test references
rg '<symbol>' crates/ -g '*.rs' tests/

# 3. Build clean before
cargo check --workspace

# (delete)

# 4. Build clean after
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

If step 1 produces hits, the module is **not** dead — re-evaluate the audit
claim and report the discrepancy before deleting.

---

## [ ] T2-16: Delete 4 orphan learn files

**Why**: These files exist on disk but are not in `crates/roko-learn/src/lib.rs`.
Rustc never compiles them. They mislead anyone reading the crate.

**Files to delete**:

```
crates/roko-learn/src/resonant_patterns.rs
crates/roko-learn/src/signal_metabolism.rs
crates/roko-learn/src/shapley.rs
crates/roko-learn/src/kalman.rs
```

**Pre-check (must pass; otherwise stop)**:

```bash
rg 'mod resonant_patterns|mod signal_metabolism|mod shapley|mod kalman' crates/roko-learn/src/lib.rs
# Expect: 0 matches
rg 'resonant_patterns|signal_metabolism|::shapley|::kalman' crates/ -g '*.rs' \
  | rg -v 'crates/roko-learn/src/(resonant_patterns|signal_metabolism|shapley|kalman)\.rs'
# Expect: 0 matches (no external references)
```

**Implementation**:

```bash
git rm crates/roko-learn/src/resonant_patterns.rs
git rm crates/roko-learn/src/signal_metabolism.rs
git rm crates/roko-learn/src/shapley.rs
git rm crates/roko-learn/src/kalman.rs
cargo check -p roko-learn
cargo test -p roko-learn
```

**Verify**:

```bash
ls crates/roko-learn/src/{resonant_patterns,signal_metabolism,shapley,kalman}.rs 2>&1
# Expect: 4× "No such file or directory"
cargo clippy -p roko-learn --no-deps -- -D warnings
```

**Do not**:

- Re-add `mod foo;` to `lib.rs` to "rescue" a file. They are dead by
  design.
- Move them under a `dead/` folder. Delete.
- Move them out of `roko-learn` and into a `roko-research` crate.
  No new crate; just delete.

**Rollback**: `git revert <hash>` if a downstream consumer surfaces.

**Estimated LOC removed**: ~600.

---

## [ ] T2-17: Remove ~14 unused learn modules from `lib.rs`

**Why**: These modules are exported via `pub mod foo;` so they compile, but
no caller outside `roko-learn` (or outside the module itself) uses them.
~4K LOC of "unreachable infrastructure."

**Modules to evaluate** (in `crates/roko-learn/src/lib.rs`):

| Module | Caller check command (must return 0 lines outside `roko-learn`) |
|---|---|
| `adversarial` | `rg 'roko_learn::adversarial' crates/ -g '*.rs'` |
| `adas` | `rg 'roko_learn::adas' crates/ -g '*.rs'` |
| `calibration_policy` | `rg 'roko_learn::calibration_policy' crates/ -g '*.rs'` |
| `causal` | `rg 'roko_learn::causal' crates/ -g '*.rs'` |
| `reinforce_kind` | `rg 'roko_learn::reinforce_kind' crates/ -g '*.rs'` |
| `research_pipeline` | `rg 'roko_learn::research_pipeline' crates/ -g '*.rs'` |
| `regression` | `rg 'roko_learn::regression' crates/ -g '*.rs'` |
| `bandit_research` | `rg 'roko_learn::bandit_research' crates/ -g '*.rs'` |
| `forensic_replay` | `rg 'roko_learn::forensic_replay' crates/ -g '*.rs'` |
| `drift` | `rg 'roko_learn::drift' crates/ -g '*.rs'` |
| `local_reward` | `rg 'roko_learn::local_reward' crates/ -g '*.rs'` |
| `section_outcome` | `rg 'roko_learn::section_outcome' crates/ -g '*.rs'` |
| `post_gate_reflection` | `rg 'roko_learn::post_gate_reflection' crates/ -g '*.rs'` |
| `verdict_scorer` | `rg 'roko_learn::verdict_scorer' crates/ -g '*.rs'` |

**For each module** (one commit per module):

1. Run the caller-check ripgrep above. Only proceed if **zero** external
   callers.
2. Inspect `crates/roko-learn/src/<module>.rs` (or `<module>/`) for any
   `pub use` from other modules' files (`rg 'use crate::<module>'`). If
   another in-crate module uses it, evaluate that module too — it may be
   transitively dead.
3. Remove `pub mod <module>;` from `lib.rs`.
4. `git rm` the module file (or directory).
5. `cargo check --workspace && cargo test --workspace`.
6. Commit: `T2-17a: Delete unused roko-learn module <module>` (or similar).

**Special cases**:

- `regression`: name overloaded with "test regression" in many docs; **only**
  delete if `roko_learn::regression` has no external callers. Check
  `crates/roko-learn/src/regression*.rs` and any sub-files for an
  `outcome_regression`-style helper still referenced.
- `causal`: check whether `crates/roko-learn/src/causal/mod.rs` is a
  directory with sub-modules. Delete recursively.
- `forensic_replay`: there is some dashboard/learning UI that may show
  forensic-replay records. If `rg 'forensic' crates/roko-cli/src/tui/`
  returns hits, evaluate before deleting.
- `verdict_scorer`: gate pipeline experiments. Check
  `rg 'VerdictScorer|verdict_scorer' crates/roko-gate crates/roko-cli` —
  there may be a real consumer.
- `post_gate_reflection`: check
  `rg 'PostGateReflection' crates/roko-runtime crates/roko-gate`.

If a module has callers, **do not delete it**; mark it `[--]` in this plan
and move on.

**Verify (per module)**:

```bash
rg 'pub mod <module>' crates/roko-learn/src/lib.rs
# Empty
cargo check --workspace
cargo test --workspace
```

**Verify (global, after all modules)**:

```bash
rg 'roko_learn::(adversarial|adas|calibration_policy|causal|reinforce_kind|research_pipeline|regression|bandit_research|forensic_replay|drift|local_reward|section_outcome|post_gate_reflection|verdict_scorer)' crates/ -g '*.rs'
# Should be empty
git diff --stat
# Net negative LOC; no other files modified
```

**Do not**:

- Wire a dead module into runtime "to keep it alive." If it had a use,
  it would have callers already.
- Hide the deletion behind a `feature` flag. Just delete.
- Bundle deletions across crates. One module per commit.

**Estimated LOC removed**: ~3,500–4,200 (varies by which modules survive
the caller check).

---

## [ ] T2-18: Remove 7 phantom config sections

**Why**: These sections are defined in config structs and present in
`roko.toml`, but the code never reads them. They confuse users into thinking
the system has features it doesn't.

**Sections to delete**:

| Section | Owner file | Field on `RokoConfig` |
|---|---|---|
| `OneirographyConfig` | `crates/roko-core/src/config/tools.rs` | `oneirography` |
| `DemurrageConfig` | `crates/roko-core/src/config/learning.rs` | `demurrage` |
| `AttentionConfig` | `crates/roko-core/src/config/learning.rs` | `attention` |
| `ImmuneConfig` | `crates/roko-core/src/config/learning.rs` | `immune` |
| `TemporalConfig` | `crates/roko-core/src/config/learning.rs` | `temporal` |
| `GoalsConfig` | `crates/roko-core/src/config/learning.rs` | `goals` |
| `EnergyConfig` | `crates/roko-core/src/config/budget.rs` | `energy` |

**Pre-check (per section, must pass)**:

```bash
# Replace <section> with e.g. oneirography
rg 'config\.<section>|cfg\.<section>|RokoConfig\s*\{[^}]*<section>' crates/ -g '*.rs' \
  | rg -v 'crates/roko-core/src/config/(schema|tools|learning|budget|hot_reload|compat)\.rs'
# Expect: 0 matches (only schema definitions and reload diff machinery)
```

**Implementation (per section, one commit each)**:

1. **Delete the struct**: e.g. `pub struct OneirographyConfig` in `tools.rs`,
   plus its `Default` impl.
2. **Remove the field on `RokoConfig`**: `crates/roko-core/src/config/schema.rs`
   has `pub oneirography: OneirographyConfig` and the matching `Default`
   line. Remove both.
3. **Remove from `roko.toml`**: delete the `[oneirography]` block at the
   workspace root.
4. **Remove from hot-reload diff**: `crates/roko-core/src/config/hot_reload.rs`
   has equality checks per section. Remove those that compare the deleted
   field.
5. **Remove from `compat.rs`** if a migration path mentions it.
6. `cargo check --workspace && cargo test --workspace`.
7. Commit: `T2-18a: Remove phantom OneirographyConfig section`.

**Special cases**:

- `[conductor]`: **DO NOT remove this entire section**. Some fields are real;
  T2-19 trims the dead fields specifically.
- `[learning]` (the parent group): there are real children alongside the
  dead ones (`DemurrageConfig`, `AttentionConfig` etc. are dead;
  `LearningConfig` itself has used fields). Only the seven named structs
  go.

**Verify (after all 7 land)**:

```bash
rg 'OneirographyConfig|DemurrageConfig|AttentionConfig|ImmuneConfig|TemporalConfig|GoalsConfig|EnergyConfig' crates/ -g '*.rs'
# Should match only:
#   - The deletion commit's removed lines (visible in git log -p only)
#   - Possibly a CHANGELOG or migration doc

grep -E '^\[oneirography\]|^\[demurrage\]|^\[attention\]|^\[immune\]|^\[temporal\]|^\[goals\]|^\[energy\]' roko.toml
# Empty

cargo test --workspace
```

**Do not**:

- Mark deleted sections `#[deprecated]`. Just delete.
- Add a "this section is reserved for future use" doc-comment to keep a
  placeholder. If we want it later, we'll add it back.
- Remove `[conductor]` (T2-19 handles the trim-down case).
- Worry about user-facing breaking changes; the config has no external
  consumers using these sections.

**Estimated LOC removed**: ~600.

---

## [ ] T2-19: Remove 6 phantom `ConductorConfig` fields

**Why**: 6 of 12 `ConductorConfig` fields exist purely as TUI display rows
or example-TOML entries. Orchestration / runner code reads neither.

**Fields to remove**: `auto_advance_batch`, `auto_merge_on_complete`,
`pre_plan`, `conductor_model`, `warm_implementers_per_plan`, `enabled_roles`.

**Fields to keep (verified used)**: `max_agents`, `max_parallel_plans`,
`parallel_enabled`, `express_mode`, `max_auto_fix_attempts`, `auto_fix_model`,
`watchers`.

**Pre-check (per field, must pass)**:

```bash
rg 'conductor\.<field>|c\.conductor\.<field>' crates/ -g '*.rs' \
  | rg -v 'crates/roko-core/src/config/(schema|compat)\.rs' \
  | rg -v 'crates/roko-cli/src/tui/' \
  | rg -v 'crates/roko-cli/src/config_cmd' \
  | rg -v 'crates/roko-cli/src/commands/config_cmd'
# Expect: 0 matches (TUI display + compat migration are OK; orchestration must not use)
```

**Files to edit**:

- `crates/roko-core/src/config/schema.rs:1145+` — remove the field
  declarations and their `Default` defaults inside `impl Default for ConductorConfig`.
- `crates/roko-core/src/config/compat.rs` — remove migration entries for the
  removed fields.
- `crates/roko-cli/src/tui/views/config_meta.rs` (or wherever the TUI shows
  these values) — remove display rows.
- `roko.toml` — remove the lines.

**Implementation** (one commit per ~3 fields, since they share the same
file edits):

1. Edit `schema.rs`: delete the field, delete the line in `Default`.
2. Edit `compat.rs`: remove migration entries.
3. Edit TUI: remove rows.
4. Edit `roko.toml`: remove lines.
5. `cargo check --workspace`.
6. Repeat for next batch.

**Verify**:

```bash
rg 'auto_advance_batch|auto_merge_on_complete|pre_plan|conductor_model|warm_implementers_per_plan|enabled_roles' crates/ -g '*.rs' roko.toml
# Should be empty
cargo test --workspace
```

**Do not**:

- Remove the entire `ConductorConfig` (kept fields are real).
- Migrate logic from `enabled_roles` into a different schema. The field is
  unused; deletion is the right answer.

**Estimated LOC removed**: ~80.

---

## [ ] T2-20: Remove write-only sinks (conductor, dreams)

**Why**: `ConductorObservationSink` and `DreamTriggerSink` write to JSONL files
that nothing reads. They consume cycles and disk for no benefit.

**Files to delete**:

- `crates/roko-cli/src/runtime_feedback/conductor.rs`
- `crates/roko-cli/src/runtime_feedback/dreams.rs`

**Files to edit**:

- `crates/roko-cli/src/runtime_feedback/mod.rs` — remove `pub mod conductor;`,
  `pub mod dreams;`, and any re-exports of `ConductorObservationSink` /
  `DreamTriggerSink`.
- `crates/roko-cli/src/commands/plan.rs` (around line 392-396) — remove the
  two `.with_sink(...)` calls that construct these sinks. Also remove the
  `conductor_path` and `dream_path` setup above (lines ~370-373) since they
  become unused.

**Pre-check**:

```bash
rg 'ConductorObservationSink|DreamTriggerSink' crates/ -g '*.rs' tests/
# Expect: matches only in conductor.rs, dreams.rs, runtime_feedback/mod.rs, commands/plan.rs
# (no other consumers)
```

**Implementation**:

1. Delete the two sink files.
2. Edit `runtime_feedback/mod.rs` to remove the module declarations and any
   re-exports.
3. Edit `commands/plan.rs` to remove the construction calls. The `let
   conductor_path = ...` and `let dream_path = ...` lines also go.
4. The `let _ = std::fs::create_dir_all(...).join(".roko/conductor")` line
   can go too.
5. `cargo check --workspace && cargo test --workspace`.

**Verify**:

```bash
rg 'ConductorObservationSink|DreamTriggerSink|conductor::ObservationSink|dreams::TriggerSink' crates/ -g '*.rs' tests/
# Empty

ls crates/roko-cli/src/runtime_feedback/{conductor,dreams}.rs 2>&1
# 2× "No such file or directory"
```

**Do not**:

- Delete `ConductorService` / `ConductorEvent` / dream-trigger types
  outside `roko-cli`. The conductor and dream subsystems exist; only their
  empty observation sinks go.
- Delete `EpisodeSink`, `RoutingObservationSink`, or `KnowledgeIngestionSink`.
  Those have real consumers (T4-29 wires the knowledge one).

**Estimated LOC removed**: ~150.

---

## [ ] T2-21: Remove phantom agent config fields

**Why**: `AgentConfig.policy_manifests` (line 70) and `AgentConfig.domain`
(line 90) are never read. `AgentConfig.data_llm` has a real implementation
(`DataLlmRouter`) but is never wired from `orchestrate.rs`.

**File**: `crates/roko-core/src/config/agent.rs`

**Pre-check**:

```bash
rg 'policy_manifests' crates/ -g '*.rs' \
  | rg -v 'crates/roko-core/src/config/agent\.rs'
# Expect: 0 matches

rg '\.domain\s*[=,)]' crates/ -g '*.rs' | rg 'agent\.|AgentConfig'
# Inspect manually; the audit says "never read" — verify no real consumer.

rg 'data_llm' crates/ -g '*.rs'
# Should show: agent.rs (definition), data_llm/* (impl), no orchestrate.rs wiring.
```

**Implementation**:

1. **Delete `policy_manifests`** from `AgentConfig`:
   - Remove the field declaration.
   - Remove its `Default::default()` line.
   - Remove the `roko.toml` line if any (`grep policy_manifests roko.toml`).

2. **Delete `domain`** from `AgentConfig`:
   - Same steps as above.

3. **Keep `data_llm`** but document its status:
   - Add a doc-comment:
     ```rust
     /// Reserved for future CaMeL dual-LLM isolation (privileged + quarantined).
     /// `DataLlmRouter` implementation exists in `crates/roko-agent/src/data_llm/`,
     /// but `orchestrate.rs` does not currently route prompts through it.
     /// See plan 22 for the dispatch migration that re-enables this path.
     pub data_llm: Option<DataLlmConfig>,
     ```
   - Do **not** delete `data_llm/`.

4. `cargo check --workspace && cargo test --workspace`.

**Verify**:

```bash
rg 'policy_manifests|AgentConfig\s*\{[^}]*\bdomain\b' crates/ -g '*.rs'
# Should be empty for both removed fields

rg 'data_llm' crates/roko-core/src/config/agent.rs
# Field declaration plus the new doc-comment
```

**Do not**:

- Delete `data_llm.rs` — the implementation is substantial and may be wired
  in plan 22.
- Delete `mcp_config`, `backends`, or other real `AgentConfig` fields.
- Touch `policy_manifests` in any unrelated crate (it's unique to
  `AgentConfig`).

**Estimated LOC removed**: ~30.

---

## Combined Verification (after all of T2-16..T2-21)

```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
git diff --stat HEAD~6..HEAD   # 6 commits, ~5K LOC net negative

# Spot-check the audit's claimed "dead" symbols are gone
rg 'OneirographyConfig|DemurrageConfig|AttentionConfig|ImmuneConfig|TemporalConfig|GoalsConfig|EnergyConfig' crates/ -g '*.rs' roko.toml
# 0 matches

rg 'mod resonant_patterns|mod signal_metabolism|mod shapley|mod kalman' crates/roko-learn/src/lib.rs
# 0 matches

rg 'ConductorObservationSink|DreamTriggerSink' crates/ -g '*.rs'
# 0 matches

rg 'policy_manifests' crates/roko-core/src/config/agent.rs
# 0 matches
```

---

## Status

- [ ] T2-16 — Delete 4 orphan learn files
- [ ] T2-17 — Remove ~14 unused learn modules from `lib.rs`
- [ ] T2-18 — Remove 7 phantom config sections
- [ ] T2-19 — Remove 6 phantom `ConductorConfig` fields
- [ ] T2-20 — Remove write-only sinks (conductor, dreams)
- [ ] T2-21 — Remove phantom agent config fields

**After completion**: ~5,000–6,000 LOC removed, no behavior change. The
codebase is smaller, the audit surface is smaller, and Tiers 3-5 work on a
cleaner slate.

Move on to Tier 3 (`13-tier3-security-hardening.md`).
