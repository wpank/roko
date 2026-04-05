# roko-core

The Roko kernel. One noun (`Signal`) and six trait verbs compose into every capability the system expresses.

## Install

```toml
[dependencies]
roko-core = { path = "../roko-core" }
```

Depends on `serde`, `serde_json`, `blake3`, `chrono`, `thiserror`. No async runtime. No I/O. `#![deny(unsafe_code)]`.

## The shape

```text
Signal  = content-addressed (BLAKE3), decaying, scored, traced, composable
Traits  = Substrate · Scorer · Gate · Router · Composer · Policy
Loop    = query substrate → score → route/compose → gate → write back → policy fires
```

Every capability in Roko — agent spawning, compilation gates, context assembly, model routing, memory retrieval, chain participation, bounty markets — reduces to one of these six verbs operating on Signals.

## Signal

A `Signal` is the only data type Roko stores. It carries:

- **`id: ContentHash`** — BLAKE3 of the canonical body + metadata. Two signals with identical meaning share an id.
- **`kind: Kind`** — discriminator (`Prompt`, `AgentOutput`, `GateVerdict`, `Episode`, `Task`, `PromptSection`, …).
- **`body: Body`** — the payload (text, JSON, binary, or empty).
- **`provenance: Provenance`** — author tag + trust level.
- **`score: Score`** — multi-axis rating (confidence, recency, relevance, cost).
- **`decay: Decay`** — half-life settings for automatic score erosion.
- **`lineage: Vec<ContentHash>`** — parent signals, forming a replayable DAG.
- **`tags: BTreeMap<String, String>`** — structured metadata queryable from `Substrate`.

Signals are built via a typed builder:

```rust
use roko_core::{Body, Kind, Provenance, Signal};

let sig = Signal::builder(Kind::Prompt)
    .body(Body::text("Write a haiku."))
    .provenance(Provenance::trusted("cli"))
    .tag("lang", "en")
    .build();
```

## The six traits

```rust
trait Substrate {
    async fn put(&self, sig: Signal) -> Result<()>;
    async fn get(&self, id: &ContentHash) -> Result<Option<Signal>>;
    async fn query(&self, q: &Query, ctx: &Context) -> Result<Vec<Signal>>;
}

trait Scorer        { fn score(&self, sig: &Signal, ctx: &Context) -> Score; }
trait Gate          { async fn verify(&self, sig: &Signal, ctx: &Context) -> Verdict; }
trait Router        { fn pick<'a>(&self, candidates: &'a [Signal], ctx: &Context) -> Option<&'a Signal>; }
trait Composer      { fn compose(&self, parts: &[Signal], budget: &Budget, scorer: &dyn Scorer, ctx: &Context) -> Result<Signal>; }
trait Policy        { fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>; }
```

Each trait has multiple concrete impls elsewhere in the workspace. The core never commits to a backend.

## Universal loop

`loop_tick` runs one step of the `query → score → route/compose → gate → write-back → policy` cycle, parametric over trait objects. It's the canonical example of how every Roko subsystem wires its primitives:

```rust
let outcome = loop_tick(&substrate, &composer, &gate, &scorer, &policy, &query, &budget, &ctx).await?;
```

## Downstream crates

| Crate | Role |
| --- | --- |
| `roko-std` | `MemorySubstrate`, NoOp impls, simple routers/scorers |
| `roko-fs` | `FileSubstrate` (JSONL persistence) |
| `roko-gate` | Shell/Compile/Clippy/Test/Diff gates |
| `roko-compose` | `PromptComposer`, `SectionScorer` |
| `roko-agent` | `Agent` trait extension + MockAgent/ExecAgent |
| `roko-cli` | `roko` binary wiring everything |

`roko-core` has no dependency on any of these — it is the abstract contract every backend implements.
