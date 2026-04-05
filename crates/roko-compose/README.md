# roko-compose

`Composer` implementations for Roko — assemble signals into structured outputs under a resource budget.

## Install

```toml
[dependencies]
roko-compose = { path = "../roko-compose" }
roko-core = { path = "../roko-core" }
```

## What's inside

| Type | Purpose |
| --- | --- |
| `PromptSection` | One labeled, priority-tagged, placement-tagged prompt fragment |
| `PromptComposer` | Assembles sections into a final `Prompt` signal under a token budget |
| `SectionScorer` | Ranks sections by priority × recency × relevance |
| `PromptBuild` | Return struct carrying the assembled prompt + dropped-section bookkeeping |
| `ContextStrategy` | Knobs: U-shape placement, priority-based dropping, cache-friendly ordering |
| `CacheLayer` | Stable prefix for prompt-caching across runs |

## Example

```rust
use roko_compose::{Placement, PromptComposer, PromptSection, SectionPriority};
use roko_core::{Budget, Composer, Context};
use roko_std::NoOpScorer;

let role = PromptSection::new("role", "You are a Rust engineer.")
    .with_priority(SectionPriority::Critical)
    .with_placement(Placement::Start)
    .into_signal()?;
let task = PromptSection::new("task", user_request)
    .with_priority(SectionPriority::Critical)
    .with_placement(Placement::End)
    .into_signal()?;

let prompt = PromptComposer::new()
    .compose(&[role, task], &Budget::tokens(8000), &NoOpScorer, &Context::now())?;
```

## Design principle

**Composers do not read files.** The app layer reads files, wraps their contents in `Signal<PromptSection>`s, and passes them to the composer. This enforces the strict I/O boundary — composers are pure functions of their inputs.

## Token estimation

`estimate_tokens(text)` uses a 4-bytes-per-token heuristic. Swap in a real tokenizer (tiktoken/huggingface) upstream if you need precision; the composer only cares that the function is monotonic.

## U-shape placement

`PromptComposer` honors `Placement::Start`, `Placement::End`, and `Placement::Body`. Under budget pressure, `Body`-placed sections get dropped first (lowest priority), preserving the U-shape attention pattern (critical info at start and end of the prompt).
