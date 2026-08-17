# PR #24 Review — Consolidated Chat Layer

**Branch:** `jl/chat-consolidated`
**Supersedes:** #11, #13, #14, #17, #19

## TL;DR

The PR works (53/53 tests, 3-agent e2e passes) but uses the wrong architecture. commonware-p2p
is designed for validator consensus, not agent coordination. A WebSocket relay (~1,150 lines)
provides dramatically more capability while being simpler and language-agnostic. The relay
aligns with roko's v2 Bus/Store fabric model: chain = Store, relay = Bus.

## Document structure

### [review/](review/) — PR #24 as-is
- [current-design.md](review/current-design.md) — What the PR does: participation, architecture, coordination patterns
- [test-results.md](review/test-results.md) — Local validation: builds, tests, 3-agent e2e
- [gaps-and-questions.md](review/gaps-and-questions.md) — Security, reliability, architecture gaps; open questions

### [context/](context/) — Research and background
- [protocol-comparison.md](context/protocol-comparison.md) — vs Matrix, Waku, GossipSub, OpenClaw A2A; commonware-p2p constraints
- [erc-standards.md](context/erc-standards.md) — ERC-8004 (identity) + ERC-8183 (jobs): what they are, combined flow
- [roko-alignment.md](context/roko-alignment.md) — Roko sketches assessment + v2 architecture alignment

### [critique/](critique/) — Why the current approach doesn't work
- [why-redesign.md](critique/why-redesign.md) — P2P mesh problems, what jobs actually look like, why relay-first

### [redesign/](redesign/) — V2-aligned relay architecture
- [core-concept.md](redesign/core-concept.md) — The relay IS the Bus; Signal/Pulse duality; three-layer architecture
- [wire-protocol.md](redesign/wire-protocol.md) — Frames, envelope format, subscribe/publish/resume, HTTP endpoints
- [feeds.md](redesign/feeds.md) — Continuous data streams: registration, discovery, composition, paid feeds
- [groups-and-coordination.md](redesign/groups-and-coordination.md) — 4 coordination modes, pheromones, ERC-8183 job lifecycle
- [chain-and-discovery.md](redesign/chain-and-discovery.md) — Chain as Global Store, agent topologies, 4-source discovery
- [architecture.md](redesign/architecture.md) — Full synthesis: diagram, file structure, capabilities, migration

### [verdict.md](verdict.md) — Ship recommendation and priorities
