# Config Schema -- Specified Sections Not in Code

The architecture docs (primarily 20-configuration-schema.md and docs 25-29) specify config sections that do not exist in `RokoConfig` (`crates/roko-core/src/config/schema.rs`).

## Checklist

### CS-01: [demurrage] config section
- [x] Add `DemurrageConfig` to `RokoConfig`

**Spec** (doc 04, doc 18): Demurrage (time-based value decay) configuration.
**Current code**: No `[demurrage]` section in RokoConfig.
**Depends on**: Demurrage trait implementation (see `07-advanced-systems.md`)
**Accept when**:
- [x] `pub demurrage: DemurrageConfig` field on RokoConfig
- [x] Parses from `[demurrage]` in roko.toml
- [x] `cargo test -p roko-core`
**Priority**: P1

### CS-02: [attention] config section
- [x] Add `AttentionConfig` to `RokoConfig`

**Spec** (doc 25): Attention token budgets, auction parameters.
**Current code**: No `[attention]` section.
**Depends on**: Attention types (see `07-advanced-systems.md`)
**Accept when**:
- [x] `pub attention: AttentionConfig` field on RokoConfig
- [x] `cargo test -p roko-core`
**Priority**: P2

### CS-03: [immune] config section
- [x] Add `ImmuneConfig` to `RokoConfig`

**Spec** (doc 26): Quarantine thresholds, taint classification rules.
**Current code**: No `[immune]` section.
**Depends on**: Cognitive Immune System types (see `07-advanced-systems.md`)
**Accept when**:
- [x] `pub immune: ImmuneConfig` field on RokoConfig
- [x] `cargo test -p roko-core`
**Priority**: P2

### CS-04: [temporal] config section
- [x] Add `TemporalConfig` to `RokoConfig`

**Spec** (doc 27): Allen relations, epoch configuration.
**Current code**: No `[temporal]` section.
**Depends on**: Temporal types (see `07-advanced-systems.md`)
**Accept when**:
- [x] `pub temporal: TemporalConfig` field on RokoConfig
- [x] `cargo test -p roko-core`
**Priority**: P2

### CS-05: [goals] config section
- [x] Add `GoalsConfig` to `RokoConfig`

**Spec** (doc 28): Goal seed parameters, tree pruning thresholds.
**Current code**: No `[goals]` section.
**Depends on**: Goal types (see `07-advanced-systems.md`)
**Accept when**:
- [x] `pub goals: GoalsConfig` field on RokoConfig
- [x] `cargo test -p roko-core`
**Priority**: P2

### CS-06: [energy] config section
- [x] Add `EnergyConfig` to `RokoConfig`

**Spec** (doc 29): Energy pool sizes, metabolism rates.
**Current code**: No `[energy]` section.
**Depends on**: Cognitive Energy types (see `07-advanced-systems.md`)
**Accept when**:
- [x] `pub energy: EnergyConfig` field on RokoConfig
- [x] `cargo test -p roko-core`
**Priority**: P2

### CS-07: Budget field naming
- [x] Align Budget field names with spec

**Spec** (doc 17): Budget struct uses `max_pulses`.
**Current code**: `Budget` in query.rs uses `max_signals`. `BudgetConfig` in config/schema.rs uses `max_plan_usd`, `max_turn_usd`, `prompt_token_budget`.
**What to change**: Once Pulse exists, rename `max_signals` to `max_pulses` on the Budget query struct. Keep the BudgetConfig USD fields as they are (different concern).
**Depends on**: K-01 (Pulse type)
**Accept when**:
- [x] `Budget` struct uses `max_pulses` field name
- [x] All call sites updated
- [x] `cargo test --workspace`
**Priority**: P1

## Notes

CS-01 and CS-07 are P1 and can be done independently. CS-02 through CS-06 are all P2 and depend on their respective advanced system types being built first.
