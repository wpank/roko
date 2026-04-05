# roko-std

Standard trait implementations for Roko. Default backends and composite helpers that every deployment needs.

## Install

```toml
[dependencies]
roko-std = { path = "../roko-std" }
roko-core = { path = "../roko-core" }
```

## What's inside

| Type | Purpose |
| --- | --- |
| `MemorySubstrate` | In-memory `Substrate` — tests, ephemeral state, hot indexes |
| `NoOpScorer`, `NoOpGate`, `NoOpRouter`, `NoOpComposer`, `NoOpPolicy` | Identity defaults for the six traits |
| `SumScorer`, `MulScorer`, `ConstScorer` | Compose other scorers linearly or multiplicatively |
| `FirstRouter`, `HighestScoreRouter`, `RoundRobinRouter` | Simple `Router` impls |
| `InMemoryTraceSink` | Swallowable trace buffer for tests |
| `StaticToolRegistry`, `ROKO_BUILTIN_TOOLS` | Read-only tool registry |

## Example

```rust
use roko_std::{MemorySubstrate, NoOpScorer, HighestScoreRouter};
use roko_core::{Context, Query, Substrate};

let sub = MemorySubstrate::new();
sub.put(signal).await?;

let hits = sub.query(&Query::all(), &Context::now()).await?;
let pick = HighestScoreRouter.pick(&hits, &Context::now());
```

## Positioning

Everything here is trivial-but-necessary: the scaffold you reach for to wire a Roko system together before committing to concrete I/O backends. For filesystem persistence use `roko-fs`, for real verification use `roko-gate`, for LLM spawning use `roko-agent`.
