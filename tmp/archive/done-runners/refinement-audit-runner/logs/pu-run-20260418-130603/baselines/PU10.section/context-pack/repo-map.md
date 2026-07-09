# Repo Map — 10 Dreams

High-value paths for batch `10`.

## Primary code anchors

- `crates/roko-dreams/src/runner.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-dreams/src/replay.rs`
- `crates/roko-dreams/src/imagination.rs`
- `crates/roko-dreams/src/hypnagogia.rs`
- `crates/roko-dreams/src/threat.rs`
- `crates/roko-dreams/src/lib.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/daemon.rs`
- `crates/roko-learn/src/pattern_discovery.rs`
- `crates/roko-learn/src/hdc_clustering.rs`

## Primary docs

- `docs/10-dreams/07-hypnagogia-engine.md`
- `docs/10-dreams/09-threat-simulation.md`
- `docs/10-dreams/13-scheduling-and-triggers.md`
- `docs/10-dreams/15-cross-system-integration.md`
- `docs/10-dreams/16-implementation-status.md`
- `docs/10-dreams/INDEX.md`

## Fastest verification searches

```bash
rg -n "DreamTrigger|scheduled_cron|manual_enabled|dream run|DreamHeartbeatPolicy" crates/roko-dreams crates/roko-cli docs/10-dreams
rg -n "CounterfactualQuery|ImaginationMode|HypnagogiaEngine|ThreatScenario" crates/roko-dreams docs/10-dreams
rg -n "roko-golem|Oneirography|Sleepwalker|nightmare|lucid" docs/10-dreams
rg -n "PatternMiner|CrossEpisodeConsolidator|k_medoids" crates/roko-learn docs/10-dreams
```
