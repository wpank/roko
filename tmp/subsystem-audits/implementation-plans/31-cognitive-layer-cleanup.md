# 31 — Cognitive Layer Cleanup

The cognitive subsystem is ~110K LOC across `roko-neuro` (~32K),
pheromones (~68K), and daimon (~40K — actually overlaps). The audit
recommends:

- **Keep**: Neuro + dreams.
- **Replace daimon (40K)** with a `FailureTracker` (~2K).
- **Delete pheromones (68K)** entirely.

This is the largest deletion in the entire backlog — ~96K LOC of net
removal.

Source: subsystem-audits/cognitive-layer/AUDIT.md, doc 36 § orchestrate
and cognitive layer.

---

## Anti-Patterns

1. **Don't preserve "for inspiration."** Git history retains anything
   you need.
2. **Don't move dead code into a `legacy` crate.** Delete.
3. **Don't migrate features from pheromones into neuro just to keep the
   ideas alive.** If a feature is real, it has callers; if it has no
   callers, it goes.

---

## Plan

### [ ] CL-1: Inventory the cognitive crates / modules

```bash
# Find pheromone-related code
rg -l 'pheromone|Pheromone' crates/ -g '*.rs'

# Find daimon-related code
rg -l 'daimon|Daimon|DaemonPolicy|AffectPolicy' crates/ -g '*.rs'
```

Map the call graph:

- Who imports `pheromone::`?
- Who imports `daimon::`?
- What do they use it for?

**Estimated effort**: 4-6 hours of investigation.

### [ ] CL-2: Replace daimon with FailureTracker

**Why**: Daimon's actual job is "track agent failures and adjust prompt
or strategy." The audit says ~2K LOC of `FailureTracker` covers the
real use cases.

#### Implementation

```rust
// crates/roko-learn/src/failure_tracker.rs (new)

pub struct FailureTracker {
    by_role: HashMap<Role, RoleFailureStats>,
    by_task_kind: HashMap<TaskKind, TaskKindFailureStats>,
}

pub struct RoleFailureStats {
    pub recent_failures: VecDeque<FailureRecord>,
    pub consecutive_count: u32,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl FailureTracker {
    pub fn record(&mut self, record: FailureRecord) {
        // Update by_role and by_task_kind
    }

    pub fn suggest_retry_strategy(&self, role: &Role) -> RetryStrategy {
        // If consecutive_count > 3, escalate model.
        // If recent failures cluster around one task kind, surface to user.
    }

    pub fn surface_alerts(&self) -> Vec<Alert> {
        // Drain pending alerts.
    }
}
```

Wire into the runner's retry decision and the orchestrator's "is the
agent stuck" check.

#### Step 2: Remove daimon

Migrate any feature in daimon that's actually used into either
`FailureTracker` or `roko-neuro`. Then:

```bash
cargo check --workspace                    # baseline
git rm -r crates/roko-cognitive/           # or wherever daimon lives
# Remove daimon mod refs from any lib.rs
cargo check --workspace                    # must build
```

If daimon is in its own crate, drop the workspace member.

### [ ] CL-3: Delete pheromones

Same procedure:

1. Confirm zero external callers (after CL-1 inventory).
2. Delete the module / crate.
3. Build clean.

The 68K LOC includes a constraint solver, a routing graph, a decay
model, and several JSONL writers. None of it has consumers per the audit.

```bash
rg 'pheromone' crates/ -g '*.rs' \
  | rg -v 'crates/roko-cognitive/pheromone/'
# Should be empty before deletion
```

### [ ] CL-4: Verify neuro + dreams retained

These stay. Verify:

- `roko-neuro` builds and tests pass.
- Dream worker still drains triggers (post-parity PK_04 wired this).
- Episode → knowledge admission pipeline works (T4-29).

### [ ] CL-5: Update `INDEX.md` and per-subsystem docs

After deletion, update:

- `tmp/subsystem-audits/INDEX.md` LOC table.
- `tmp/subsystem-audits/cognitive-layer/AUDIT.md` to reflect post-deletion state.
- `tmp/subsystem-audits/cognitive-layer/PLAN.md` close-out section.

---

## Combined Verification

```bash
# Pheromones gone
rg 'pheromone|Pheromone' crates/ -g '*.rs'   # 0 matches

# Daimon gone (or only as FailureTracker reference in docs)
rg 'daimon|Daimon|AffectPolicy' crates/ -g '*.rs'   # 0 matches in product code

# FailureTracker in place
rg 'FailureTracker' crates/   # multiple matches: definition + callers

# Build + test green
cargo check --workspace
cargo test --workspace

# LOC reduction
git diff --stat HEAD~5..HEAD   # ~-100K lines
```

---

## Status

- [ ] CL-1 — Inventory
- [ ] CL-2 — Replace daimon with FailureTracker
- [ ] CL-3 — Delete pheromones
- [ ] CL-4 — Verify neuro + dreams retained
- [ ] CL-5 — Update audit docs

**Estimated effort**: 16-30 hours, dominated by CL-2 (FailureTracker
implementation + daimon migration verification).

This is a high-impact, high-risk change. Do it in a feature branch,
verify on a fork before merging to main, and keep the diff focused on
deletion + the small replacement.
