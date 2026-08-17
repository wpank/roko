# GATE_11: Add domain profile initialization to GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-11`](../ISSUE-TRACKER.md#gate-11)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.11
- Priority: **P1**
- Effort: 3 hours
- Depends on: `GATE_03` (source 4.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ThresholdProfile` at `crates/roko-gate/src/adaptive_threshold.rs:75` defines three domain profiles (coding, research, security) with per-rung priors and sensitivity overrides. `ThresholdProfile::coding()` at line 93, `research()` at line 110, `security()` at line 127. None of these are ever instantiated at runtime (AP-8).

`AdaptiveThresholds::new()` starts all rungs at neutral priors (EMA 0.5). A security auditor and a code implementer both start with identical expectations, when their pass rates differ significantly.

## Exact Changes

1. Add `from_profile()` constructor to `AdaptiveThresholds`:
   ```rust
   pub fn from_profile(profile: &ThresholdProfile) -> Self {
       let mut at = Self::new();
       for (&rung, &prior) in &profile.rung_priors {
           let stats = at.rungs.entry(rung).or_default();
           stats.ema_pass_rate = prior;
           stats.total_observations = 5; // small but non-zero to weight priors
       }
       if let Some(sensitivity) = profile.cusum_sensitivity_override {
           at.cusum_sensitivity = sensitivity;
       }
       at
   }
   ```
2. Add `with_profile()` builder method to `GateService`:
   ```rust
   pub fn with_profile(self, profile: &ThresholdProfile) -> Self {
       let thresholds = AdaptiveThresholds::from_profile(profile);
       self.with_adaptive_thresholds(Arc::new(Mutex::new(thresholds)))
   }
   ```
3. Add `profile` field to `GateConfig`:
   ```rust
   /// Domain profile name for threshold initialization ("coding", "research", "security").
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub profile: Option<String>,
   ```
4. In `GateService::run_gates()`, when adaptive thresholds are not already set and `config.profile` is provided, initialize from the named profile:
   ```rust
   if self.adaptive.is_none() && let Some(ref profile_name) = config.profile {
       let profile = ThresholdProfile::by_name(profile_name)
           .unwrap_or_else(ThresholdProfile::coding);
       // Initialize on first use
   }
   ```
5. Add `ThresholdProfile::by_name()` helper:
   ```rust
   pub fn by_name(name: &str) -> Option<Self> {
       match name {
           "coding" => Some(Self::coding()),
           "research" => Some(Self::research()),
           "security" => Some(Self::security()),
           _ => None,
       }
   }
   ```
6. Add tests verifying profile initialization sets correct EMA priors.

## Write Scope

- `crates/roko-gate/src/adaptive_threshold.rs`
- `crates/roko-gate/src/gate_service.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `AdaptiveThresholds::from_profile(&ThresholdProfile::security())` starts rung 0 at 0.95 EMA
- [ ] GateService with profile "security" has stricter initial thresholds than default

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `AdaptiveThresholds::from_profile(&ThresholdProfile::security())` starts rung 0 at 0.95 EMA
- GateService with profile "security" has stricter initial thresholds than default
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
