# Repo Map - 10 Dreams

High-value paths and current numbers for batch `10`.

## Workspace context

- total Rust LOC: 322,088
- workspace members: 36
- `roko-learn`: 35,847 LOC across 42 modules
- `roko-cli` + TUI: 58K LOC total
- `roko-dreams`: 5,964 LOC across 7 source files

## Primary code anchors

- `crates/roko-dreams/src/lib.rs`
- `crates/roko-dreams/src/runner.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-dreams/src/replay.rs`
- `crates/roko-dreams/src/imagination.rs`
- `crates/roko-dreams/src/hypnagogia.rs`
- `crates/roko-dreams/src/threat.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/daemon.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-learn/src/pattern_discovery.rs`
- `crates/roko-learn/src/hdc_clustering.rs`
- `crates/roko-learn/src/playbook.rs`

## Highest-value live docs

- `docs/10-dreams/07-hypnagogia-engine.md`
- `docs/10-dreams/09-threat-simulation.md`
- `docs/10-dreams/13-scheduling-and-triggers.md`
- `docs/10-dreams/15-cross-system-integration.md`
- `docs/10-dreams/16-implementation-status.md`
- `docs/10-dreams/17-advanced-dream-concepts.md`
- `docs/10-dreams/INDEX.md`

## Fast verification searches

```bash
rg -n "DreamTrigger|DreamSchedulePolicy|manual_enabled|scheduled_cron|DreamHeartbeatPolicy|dream run" crates/roko-dreams crates/roko-cli docs/10-dreams
rg -n "DreamReplayMode|utility_score|CounterfactualQuery|ImaginationMode|HypnagogiaEngine|ThreatScenario" crates/roko-dreams docs/10-dreams
rg -n "PatternMiner|CrossEpisodeConsolidator|k_medoids|PlaybookStore" crates/roko-learn crates/roko-dreams docs/10-dreams
rg -n "roko-golem|Sleepwalker|Oneirography|nightmare|lucid" docs/10-dreams
```
