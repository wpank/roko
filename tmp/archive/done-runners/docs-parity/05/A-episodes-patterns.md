# A — Episodes + Patterns

Audit-corrected parity view of `docs/05-learning/00-episode-logger.md` and `05-pattern-discovery-trigram.md`.

---

## What Is Already Shipped

- `EpisodeLogger` is real, append-only, crash-safe, and already supports compaction.
- HDC fingerprints are already attached to episodes today, but they live in `extra` rather than on `Engram`.
- `PatternMiner`, `EpisodeView`, cross-episode consolidation, and k-medoids clustering are all live.
- Pattern discovery already runs on a slower cadence inside `LearningRuntime`; it is not just a design note.

## What The Old Parity Material Overstated

- the live `Episode` schema is broader than the docs describe,
- the retention story is **not** "append forever"; `EpisodeLogger::compact` already exists,
- tiered hot/warm/cold storage is still a design,
- DBSCAN-style automatic clustering is still a design,
- the most important missing bridge is not "invent more episode storage" but **move HDC fingerprints into the kernel data model**.

## Corrected Gap Picture

### Shipping Contract

- keep `EpisodeLogger`, `PatternMiner`, and k-medoids in present tense,
- describe compaction as the current retention mechanism,
- describe fingerprints as shipping but living on episode records rather than `Engram`.

### Ship Soon

- add `fingerprint` to `Engram`,
- then update the docs so episode fingerprints are no longer described as a learning-only side channel.

### Deferred

- `EpisodeStorageConfig`
- `CompressedEpisodeSummary`
- zstd warm tier / HDC cold tier
- DBSCAN / `EpisodeCluster` / cluster-evolution machinery

## Practical Rewrite Guidance

When touching `docs/05-learning/00-episode-logger.md` or `05-pattern-discovery-trigram.md`:

1. update the schema examples to match the real `Episode` shape,
2. make `compact()` the present-tense retention story,
3. leave tiered storage and DBSCAN under an explicit `planned` or `target-state` heading,
4. avoid turning this into a storage-architecture roadmap.

## Batch-Ready Follow-Ups

- carry forward: add the HDC fingerprint bridge to `Engram`
- `L7`: align the episode and pattern docs with the shipped runtime

## Source Anchors

- `crates/roko-learn/src/episode_logger.rs:169` — `Episode`
- `crates/roko-learn/src/episode_logger.rs:860` — append path
- `crates/roko-learn/src/episode_logger.rs:982` — compaction path
- `crates/roko-learn/src/pattern_discovery.rs:99` — `PatternMiner`
- `crates/roko-learn/src/hdc_clustering.rs` — k-medoids implementation

## Bottom Line

The episode/pattern story is already strong enough to document honestly. The parity refresh should narrow the docs to the runtime that exists and move the tiered-storage / DBSCAN ideas into explicit future work.
