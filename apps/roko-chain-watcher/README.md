# roko-chain-watcher

A long-running roko agent that observes a `mirage-rs` chain and posts insights.

It implements the §33.26.9 checklist item: a real agent loop that subscribes
(via polling for now — see §37.c for the WS upgrade path) to chain pheromones
and insights, pattern-matches against them, and acts on the chain.

## What a watcher does

```
  mirage-rs (RPC) ──poll──▶ rpc_client ──▶ reactions::decide ──▶ rpc_client ──post──▶ mirage-rs
```

Every `--poll-interval-ms` tick, the watcher:

1. **Observes** — calls `chain_queryPheromones` + `chain_searchInsights` with
   the configured `--query`.
2. **Decides** — runs the pattern rules in `reactions.rs`, producing a list
   of desired reactions.
3. **Reacts** — executes each reaction via JSON-RPC, subject to the
   `--max-reactions-per-min` rate limit and the `--dry-run` flag.

## Reaction rules

| # | Trigger                                        | Action                                   |
|---|------------------------------------------------|------------------------------------------|
| 1 | `threat` pheromone w/ intensity > 0.7 AND no existing `warning` insight | post a `warning` insight |
| 2 | `opportunity` pheromone w/ intensity > 0.6     | post a `strategy_fragment` insight       |
| 3 | `wisdom` pheromone with a matching insight     | `chain_confirmInsight` on the top match  |
| 4 | insight content contains `WRONG` / `BUG` / `INCORRECT` with ≥1 confirmation | `chain_challengeInsight` |
| 5 | any observations this tick                     | deposit a `wisdom` summary pheromone     |

Thresholds live as constants at the top of `src/reactions.rs`.

## Quick start

```bash
# Terminal 1: start mirage with chain extensions
cargo run --bin mirage-rs --features binary,chain -- \
  --enable-hdc --enable-knowledge --enable-stigmergy

# Terminal 2: start roko-chain-watcher
cargo run --bin roko-chain-watcher -- \
  --rpc-url http://127.0.0.1:8545 \
  --watcher-id alice-watcher-01
```

## Tuning knobs

| Flag                        | Env                    | Default                      | Purpose                                         |
|-----------------------------|------------------------|------------------------------|-------------------------------------------------|
| `--rpc-url`                 | `MIRAGE_RPC_URL`       | `http://127.0.0.1:8545`      | mirage-rs JSON-RPC endpoint                     |
| `--watcher-id`              | `ROKO_WATCHER_ID`      | `roko-watcher-001`           | identity reported as `author` on posts          |
| `--poll-interval-ms`        | —                      | `2000`                       | how often to poll                               |
| `--poll-k`                  | —                      | `25`                         | top-k pheromones/insights per poll              |
| `--query`                   | `ROKO_WATCHER_QUERY`   | `threat opportunity wisdom`  | HDC query used for both searches                |
| `--dry-run`                 | —                      | `false`                      | log reactions but don't execute                 |
| `--max-reactions-per-min`   | —                      | `30`                         | per-minute reaction rate limit                  |
| `--max-events`              | —                      | `0` (forever)                | exit after this many observations               |

## Structured logs

The watcher emits `tracing` events with stable field names — pipe stderr
through `jq` after enabling JSON output (`ROKO_LOG=info`):

```
method=chain_postInsight outcome=accepted id=insight:... latency_ms=4 "post_insight ok"
method=chain_depositPheromone id=42 latency_ms=3 "deposit_pheromone ok"
method=chain_confirmInsight target=insight:... latency_ms=2 "confirm ok"
```

Useful filters: `method=chain_postInsight`, `kind=PostInsight`, `dry_run=true`.

## Hard limits

- HTTP polling only (no WS subscriptions; that lands with §37.15).
- Out-of-process — the watcher never embeds mirage as a library.
- No LLM — all rules are pattern-matching.
- Never exceeds `--max-reactions-per-min`.
