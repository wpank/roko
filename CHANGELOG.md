# Changelog

All notable changes to Roko will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- §37.c subscription surface (SubscriptionSink trait, PheromoneSubscription, InsightSubscription)
- §38.c introspection methods (`chain_version`, `chain_listKinds`, `chain_methodSchema`)
- §38.e per-method / per-author rate limiting
- §33.4.1-2 `roko-chain` crate with `ChainClient` + `ChainWallet` traits + mocks
- §40.a+c `roko-core::obs` Prometheus metrics + health/readiness probes
- §41.a Cross-subsystem `RokoError` variants + `ErrorKind` discriminant + `is_transient()`
- §42.a Multi-arch container images (roko, mirage, gateway) + GHCR publish workflow
- §43.a `SecretStore` trait + `EnvVarStore` + `FileStore` backends
- §39.a API stability policy doc + `schema_version` field on `RokoConfig`

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
