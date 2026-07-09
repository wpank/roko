# 04-21-26: Cybernetic Self-Learning Architecture

Documents from the April 21 2026 session exploring how roko's learning infrastructure
connects to benchmarks, HuggingFace, and multi-domain agent workflows.

## Files

| File | What |
|------|------|
| [01-swe-bench-current-state.md](01-swe-bench-current-state.md) | What exists today for SWE-bench (Python scripts, gaps, learning loop disconnect) |
| [02-huggingface-integration.md](02-huggingface-integration.md) | HuggingFace API surface and what each layer enables for roko |
| [03-native-bench-crate.md](03-native-bench-crate.md) | Design for `roko bench` — native Rust benchmark harness that closes the learning loop |
| [04-generalized-arenas.md](04-generalized-arenas.md) | The generalized architecture: domain-agnostic arenas, self-learning loops, network effects |
| [05-domain-catalog.md](05-domain-catalog.md) | Concrete domains and what arena + gate + learning config looks like for each |
| [06-hdc-deep-integration.md](06-hdc-deep-integration.md) | HDC integration map: where vectors connect to arenas, chain, learning, and each other |
| [07-implementation-direction.md](07-implementation-direction.md) | Decisions and parallel workstreams (Korai + privacy-first + wire existing + build new) |
| [08-korai-narrative-and-gaps.md](08-korai-narrative-and-gaps.md) | The full Korai narrative, 7 specific gaps, and how to measurably prove it works |
| [09-unified-narrative.md](09-unified-narrative.md) | Everything unified: HDC + chain + context + arenas + Golem runtime + performance analysis |

| [10-knowledge-publishing-and-privacy.md](10-knowledge-publishing-and-privacy.md) | When/how agents publish to chain, 7-layer defense (earlier draft, more conventional) |
| [11-geometric-knowledge-sharing.md](11-geometric-knowledge-sharing.md) | **The clean version**: zero-LLM privacy via algebraic erasure, standalone doc for newcomers |

## Related

- `generalizations/` — AgentRuntime redesign, extension model, domain specialization (synergistic with chain integration)
