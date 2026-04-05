# `mirage-rs` examples

Runnable demos of the §37 chain extensions and roko-bridge surface. Each
example is gated behind the feature it exercises.

| Example | Feature | Section |
|---|---|---|
| `seed_chain_fixtures` | `chain` | §37.19 |
| `roko_chain_watcher` | `roko` | §37.16 |
| `persona_chain_native` | `roko` | §37.17 |

## `seed_chain_fixtures`

Populates a **running** mirage-rs JSON-RPC server with 50 `InsightEntry`s and
20 pheromones covering AMMs, lending, liquidations, MEV, oracles, bridges,
restaking, and governance. Used to pre-warm the browser demo and the
chain-watcher.

**Prereq**: mirage-rs must be running with chain subsystems enabled:

```bash
cargo run -p mirage-rs --features chain --bin mirage-rs -- \
    --enable-hdc --enable-knowledge --enable-stigmergy
```

**Run**:

```bash
cargo run -p mirage-rs --features chain --example seed_chain_fixtures -- \
    --rpc-url http://127.0.0.1:8545
```

The `--rpc-url` flag defaults to `http://127.0.0.1:8545`. If no mirage is
listening the seeder prints a friendly error and exits with code 1.

**Expected output** (trimmed):

```
seed_chain_fixtures: target = http://127.0.0.1:8545
seed_chain_fixtures: connected. stats={"insights":0,"pheromones":0,"toggles":{...}}
seed_chain_fixtures: done.
  insights:   50 accepted / 0 failed  (of 50)
  pheromones: 20 deposited / 0 failed (of 20)
```

## `roko_chain_watcher`

Pure in-process demo of the push-based subscription surface (§37.11–§37.14).
Spins up a `PheromoneBus` + `InsightBus`, registers a `VecSink` subscriber on
each, seeds 12 pheromones and 8 insights, then loops 5 times emitting a new
pheromone + decay event per tick. Prints a summary table with subscription
stats and the observed pheromone mix.

No network, no HTTP server, no LLM. Runs in under a second.

**Run**:

```bash
cargo run -p mirage-rs --features roko --example roko_chain_watcher
```

**Expected output** (trimmed):

```
== roko_chain_watcher ==
registered subscribers: pheromone=sub#1, insight=sub#1
seeded 12 pheromones (field size = 12)
seeded 8 insight events
initial drain: 12 pheromone events, 8 insight events

tick 0 (t+45m): field=13 evap=0 | new events: 1 pheromones, 1 insights
    pher#13 kind=Threat intensity=0.73
    insight: decayed #0100 -> weight 0.900
...
== summary ==
  pheromone field: 17 live entries
  pheromone sub #sub#1: delivered=17 dropped_oldest=0 dropped_newest=0 closed=false
  insight   sub #sub#1: delivered=13 dropped_oldest=0 dropped_newest=0 closed=false
  observed pheromone mix: threat=6 opportunity=6 wisdom=5
```

## `persona_chain_native`

Builds an `HdcSubstrate` for a chain-native Uniswap-analyst persona, puts 3
insights about Uniswap behaviour, then runs a semantic `Substrate::query` and
prints the top-3 hits ranked by HDC similarity × effective score.

Deterministic: no LLM, no RNG, no network. The text projection is a stable
hash of input tokens.

**Run**:

```bash
cargo run -p mirage-rs --features roko --example persona_chain_native
```

**Expected output**:

```
== persona_chain_native ==
persona = chain-native/uniswap-analyst

writing 3 insights into HdcSubstrate
  put [uniV3-stf-revert] -> hash <hex>
  put [uniV3-twap-depth] -> hash <hex>
  put [uniV4-hook-gas] -> hash <hex>

query (text_query) = "uniswap v3 STF reverts on low allowance"
top-3 hits:
  #1: effective_score=2.720  body="uniswap v3 STF revert typically means insufficient allowance on the input token"
  #2: effective_score=2.720  body="uniswap v3 TWAP oracle accuracy depends on pool liquidity depth; thin pools are manipulable"
  #3: effective_score=2.720  body="uniswap v4 hook invocations add ~20k gas when hooks are permissionless and untrusted"

ok: top hit matches the expected STF-revert insight.
persona_chain_native: done (3 entries in substrate).
```
