# Daeji Analysis — Agent Infrastructure Assessment

## Documents

### PR #24 Review
- [pr24-review/](pr24-review/) — Complete review of the consolidated chat layer PR
  - [README.md](pr24-review/README.md) — Index and TL;DR
  - [verdict.md](pr24-review/verdict.md) — Don't merge as-is; redesign as relay
  - [review/](pr24-review/review/) — What the PR does, test results, gaps
  - [context/](pr24-review/context/) — Protocol comparison, ERC standards, roko alignment
  - [critique/](pr24-review/critique/) — Why the current approach doesn't work
  - [redesign/](pr24-review/redesign/) — V2-aligned relay architecture proposal

### Contracts & Spec Analysis
- [contracts-analysis.md](contracts-analysis.md) — Inventory of 17 contracts-core contracts, which overlap ERC-8004/8183, what to keep/replace/drop
- [superseded-prs.md](superseded-prs.md) — What PRs #11/#13/#14/#17/#19 contributed, what was preserved, what was lost
- [isfr-analysis.md](isfr-analysis.md) — ISFR implementations across 4 repos, current vs spec target, ISFR as first use case
- [spec-gap-analysis.md](spec-gap-analysis.md) — How far current state is from agent-chainv2 spec across all dimensions

### Roko Relay Reality Check
- [roko-relay-current-state.md](roko-relay-current-state.md) — What roko's relay actually is today: flat request/response broker, no pub/sub, no feeds, PulseBus exists but isn't connected to relay
- [daeji-relay-practical.md](daeji-relay-practical.md) — Practical relay design given current state: Phase 1 at ~600 lines (pub/sub + chain events), feeds add ~90 lines, groups add ~100 lines
- [isfr-integration-comparison.md](isfr-integration-comparison.md) — ISFR with PR #24 vs relay: language barriers, protocol coupling, concrete code examples both approaches, ~430 total lines for relay path

### Redesign
- [minimal-redesign.md](minimal-redesign.md) — Elegant minimal version: 7 contracts + relay, vanilla ERC-8004/8183, ISFR as first use case

### ISFR Implementation
- [isfr-implementation-guide.md](isfr-implementation-guide.md) — End-to-end guide: canonical contracts (demo-ide v3.0), mirage deployment, relay integration, feed topics, keeper agents, roko-serve API endpoints, demo-app UI (IsfrFeedTile, workspace templates, WebSocket live feeds), agent discovery, full checklist

## TL;DR

**Current state:** 17 custom contracts that reinvent ERC-8004 and ERC-8183 poorly, plus a chat layer (PR #24) using the wrong transport (commonware-p2p mesh for agent coordination). Roko's relay is flat request/response with no pub/sub.

**Target:** 7 contracts using vanilla ERC standards + a standalone WebSocket relay. Less code, dramatically more capability, spec-aligned.

**Practical relay:** Phase 1 is ~600 lines (topic pub/sub + ring buffer + chain watcher). Feeds are ~90 lines on top of that — they're just topics with metadata. The PulseBus infrastructure already exists in roko; it just needs a WebSocket bridge.

**First use case:** ISFR keeper coordination via relay feed topics. ~430 lines total across relay + keeper changes. Python and Rust keepers coexist on the same topics. Compare to PR #24 which requires rewriting the Python keeper in Rust and can't express range coordination in its typed message enum.

**Distance from spec:**
- Done: consensus, BLS, QMDB, block finality, ISFR source weights, reputation decay model
- Close (2 weeks): vanilla contracts, relay v1, chain timestamp/blockhash fixes
- Medium-term (1-2 months): 7-domain reputation, validation registry, relay v2 with feeds+groups
- Long-term (3-6 months): validator-computed oracle, precompiles, TEE clearing
