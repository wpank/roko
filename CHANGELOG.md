# Changelog

All notable changes to Roko will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- R01 supervised HTTP JSON connectors with real bounded probes/query/execute/health,
  generation-safe restart/replacement/shutdown, secret-safe status, and authenticated-or-
  loopback lifecycle routes.
- R02 bounded `agent-relay` replay/delivery, atomic `Subscribe(last_seq)` recovery, and a
  supervised durable client with capped reconnect, ACK-after-handler-commit, snapshot
  reconciliation, and supersession handling. `roko serve` now executes exact-room
  subscriptions through a bounded fsync journal with durable terminal receipts/cursors and
  authenticated reconciliation status.
- R03 authorized durable local arena HTTP lifecycle with principal-bound attempts, external
  scoring evidence, atomic settlement/leaderboard effects, and restart-safe outbox projection.
- R04 owner-scoped durable meta-agent proposal, activation, morph/rollback, and deactivation;
  exact five-head safety evidence; non-widening grants/lineage limits; and single-use R03
  evidence bound to the complete activation artifact.
- Parent-linked, capability-narrowed relay bearer delegation with bounded chain depth,
  full-chain validation, and cascading root/intermediate revocation (E35-T06)
- §37.c subscription surface (SubscriptionSink trait, PheromoneSubscription, InsightSubscription)
- §38.c introspection methods (`chain_version`, `chain_listKinds`, `chain_methodSchema`)
- §38.e per-method / per-author rate limiting
- §33.4.1-2 `roko-chain` crate with `ChainClient` + `ChainWallet` traits + mocks
- §40.a+c `roko-core::obs` Prometheus metrics + health/readiness probes
- §41.a Cross-subsystem `RokoError` variants + `ErrorKind` discriminant + `is_transient()`
- §42.a Multi-arch container images (roko, mirage, gateway) + GHCR publish workflow
- §43.a `SecretStore` trait + `EnvVarStore` + `FileStore` backends
- §39.a API stability policy doc + `schema_version` field on `RokoConfig`

### Changed
- Connectivity, arena, safety, roadmap, architecture, and API documentation now distinguish
  the accepted R01-R04 implementation boundaries from remaining additional transports,
  discovery/MCP/A2A/x402/finality/reorg/dashboard work; arena eval/flywheel/on-chain/token/
  transfer work; and Loop 4/ADAS/HGM/autonomous generated execution.
- Upgrades invalidate and remove recognized pre-T06 `.roko/relay-tokens.json` records
  because the legacy format has no verifiable parent chain. Reissue active relay
  credentials from a valid `roko_agent_...` bearer after upgrading.

### Unreleased deprecations
_(none)_

---

## [0.1.0] - 2026-04-05

### Added
- Initial Roko kernel (`roko-core`) with 7 traits + Signal type
- Memory/Filesystem substrates (`roko-std`, `roko-fs`)
- Gates: Compile, Test, Clippy, Symbol, VerifyChain, GeneratedTest, PropertyTest, Integration, LlmJudge, Diff
- Agent backends (stubs): MockAgent, ExecAgent, ClaudeAgent, OllamaAgent, OpenAiAgent
- `mirage-rs` EVM fork simulator + chain extensions (HDC index + knowledge + stigmergy)
- `mirage-rs` roko bridge: HdcSubstrate, ChainSubstrate, SimulationGate
