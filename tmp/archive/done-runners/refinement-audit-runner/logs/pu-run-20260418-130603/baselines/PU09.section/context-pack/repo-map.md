# Repo Map — 09 Daimon

High-value paths for batch `09`.

## Primary code anchors

- `crates/roko-core/src/affect.rs`
- `crates/roko-daimon/src/lib.rs`
- `crates/roko-neuro/src/context.rs`
- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-compose/src/context_assembler.rs`
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/diagnosis.rs`

## Primary docs

- `docs/09-daimon/01-pad-vector.md`
- `docs/09-daimon/02-alma-three-layer-temporal.md`
- `docs/09-daimon/09-mood-congruent-memory.md`
- `docs/09-daimon/10-integration-points.md`
- `docs/09-daimon/11-coding-agent-integration.md`
- `docs/09-daimon/12-collective-emotional-contagion.md`
- `docs/09-daimon/13-current-status-and-gaps.md`
- `docs/09-daimon/INDEX.md`

## Fastest verification searches

```bash
rg -n "AffectOctant|roko-golem|Plutchik|discovery_emotion" docs/09-daimon
rg -n "BehavioralState::classify|select_with_hysteresis|DaimonPolicy" crates
rg -n "ContextAssembler|PromptComposer|externality|EmotionalTag" crates/roko-neuro crates/roko-compose
rg -n "per-crate confidence|fatigue|contagion|C-Factor" docs/09-daimon crates
```
