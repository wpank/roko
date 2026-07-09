# Repo Map — 09 Daimon

High-value paths and searches for batch `09`.

## Audit surface snapshot

Checked on `2026-04-18`.

- `docs/09-daimon`: 15 files
- `crates/roko-core`: 69 files
- `crates/roko-daimon`: 2 files
- `crates/roko-neuro`: 8 files
- `crates/roko-compose`: 71 files
- `crates/roko-learn`: 47 files
- `crates/roko-cli`: 123 files
- `crates/roko-conductor`: 20 files
- total audited surface above: 355 files

## Primary code anchors

- `crates/roko-core/src/affect.rs`
- `crates/roko-daimon/src/lib.rs`
- `crates/roko-neuro/src/context.rs`
- `crates/roko-neuro/src/distiller.rs`
- `crates/roko-neuro/src/lib.rs`
- `crates/roko-compose/src/context_assembler.rs`
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/diagnosis.rs`

## Primary docs

- `docs/09-daimon/01-pad-vector.md`
- `docs/09-daimon/04-six-behavioral-states.md`
- `docs/09-daimon/09-mood-congruent-memory.md`
- `docs/09-daimon/10-integration-points.md`
- `docs/09-daimon/11-coding-agent-integration.md`
- `docs/09-daimon/12-collective-emotional-contagion.md`
- `docs/09-daimon/13-current-status-and-gaps.md`
- `docs/09-daimon/INDEX.md`

## Fastest verification searches

```bash
rg -n "roko-golem|AffectOctant|Plutchik|discovery_emotion" docs/09-daimon tmp/docs-parity/09/context-pack
rg -n "pub struct EmotionalTag|BehavioralState::classify|pub struct DaimonPolicy" crates/roko-core/src/affect.rs crates/roko-daimon/src/lib.rs
rg -n "ContextAssembler|with_affect_state|apply_somatic_bias|discovery_emotion" crates/roko-neuro/src/context.rs crates/roko-neuro/src/distiller.rs crates/roko-neuro/src/lib.rs
rg -n "DaimonPolicy|select_with_hysteresis" crates/roko-cli/src/orchestrate.rs crates/roko-learn/src/cascade_router.rs
rg -n "per-crate confidence|fatigue|contagion|C-Factor" docs/09-daimon/11-coding-agent-integration.md docs/09-daimon/12-collective-emotional-contagion.md crates/roko-core crates/roko-learn crates/roko-cli
```

## Reading note

`roko-golem` is useful for finding stale language, not for defining the live
runtime contract.
