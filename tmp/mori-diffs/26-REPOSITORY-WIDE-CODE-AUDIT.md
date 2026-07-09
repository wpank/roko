# Repository-Wide Code Audit

This extends `25-CODE-ONLY-LEGACY-AUDIT.md`. The earlier file focused on the main runner/provider/gate implementation. This pass widened the scan to the tracked repository outside docs and design drafts.

## Proof of scope

- Total tracked files:
  - `git ls-files | wc -l`
  - result: `1798`
- Tracked implementation/config/script/UI files used as audit evidence:
  - `git ls-files | rg -v '(^|/)(docs|tmp/mori-diffs|tmp/unified|tmp/unified-depth)(/|$)|\.(md|mdx|rst|txt)$' | wc -l`
  - result: `1312`
- Directory coverage in the non-doc scope:
  - `1088` files under `crates`
  - `90` files under `apps`
  - `50` files under `demo`
  - `24` files under `contracts`
  - `10` files under `docker`
  - `8` files under `examples`
  - `7` files under `tests`
  - `5` files under `tools`
  - plus root config files such as `Cargo.toml`, `roko.toml`, `Dockerfile`, `fly.toml`, and `railway.toml`
- Marker query used across the non-doc scope:
  - `TODO|FIXME|HACK|stub|placeholder|unimplemented!|todo!|noop|no-op|fake|dummy|mock|legacy|compat|fallback|hardcoded|hard-code|bypass|one-off|special case|temporary|test-only|not yet supported|not wired|disabled`
- Non-doc files with at least one marker:
  - `484`

## New high-priority findings beyond the earlier runner/provider/gate audit

### Chain layer still ships large phase-2 stub surfaces

- [ ] Replace `crates/roko-chain/src/phase2.rs` or move it behind an explicit experimental feature. The file opens with "Phase 2 chain-layer stubs" and says the items intentionally avoid production logic.
- [ ] Implement or remove `HdcPrecompile` methods in `crates/roko-chain/src/phase2.rs:849`, `:854`, and `:859`. They call `todo!()` for similarity, bind, and bundle, which means this code path can panic if reached.
- [ ] Replace deterministic signature stubs in `crates/roko-chain/src/identity_economy_identity.rs:1342` through `:1364`. ERC-3009 authorization currently creates deterministic fake `r`/`s` bytes instead of cryptographic signatures.
- [ ] Replace signature-validation stubs in `crates/roko-chain/src/x402.rs:85` and `:355`. The state-channel path documents that signatures are present but not cryptographically verified.
- [ ] Convert `crates/roko-chain/src/identity_economy_markets.rs` from compile-clean type shells into real marketplace/settlement/compliance behavior, or feature-gate it. The module declares itself "Phase 2+ job-market, settlement, futures, and compliance stubs."
- [ ] Audit `crates/roko-chain/src/mock.rs` so `MockChainClient` and `MockChainWallet` cannot be selected by production config. The deterministic placeholder wallet address is useful for tests but should not be reachable through normal runtime wiring.

### Dream/runtime feedback loop is still mostly shell wiring

- [ ] Wire `crates/roko-runtime/src/delta_consumer.rs:297`, `:313`, and `:327` to real `roko-dreams` and `roko-neuro` functions. NREM replay, REM imagination, and integration currently return empty reports.
- [ ] Move `crates/roko-dreams/src/phase2/*` behind an explicit experimental feature or implement the exported surfaces. `phase2/mod.rs` says these modules are compile-clean type shells, not shipped runtime.
- [ ] Implement contradiction detection in `crates/roko-dreams/src/staging.rs:236`. Validation currently says "Not contradicted" but the placeholder always passes.
- [ ] Replace the grouped replay scheduler stub in `crates/roko-dreams/src/phase2/replay.rs:195` with real replay scheduling, or keep it out of production exports.
- [ ] Decide whether `crates/roko-daimon/src/phase2_stubs.rs` is production API or parked design surface. It exposes legacy modulation aliases and phase-2 state types through `roko-daimon/src/lib.rs`.

### Serve and gateway surfaces still expose placeholders

- [ ] Implement `roko-gateway` as a real crate or remove the gateway container from release workflows. `docker/gateway.Dockerfile` currently builds the `roko` CLI binary and labels it as a gateway placeholder.
- [ ] Replace placeholder gateway cache metrics in `crates/roko-serve/src/routes/gateway.rs:437`. `cache_hits` is hardcoded to `0` because no inference cache is wired.
- [ ] Fix the duplicate event bridge in `crates/roko-serve/src/lib.rs:1025`. REST-originated events can appear twice on the EventBus, and unmapped dashboard variants are dropped.
- [ ] Replace cross-repo context stub generation in `crates/roko-serve/src/dispatch.rs:1865`. It currently lists repositories but does not inject recent commits, open PRs, diffs, or other useful context.
- [ ] Finish Privy auth integration or remove the config knob from the production schema. `crates/roko-core/src/config/serve.rs:49` labels `privy_app_id` as a Phase 1b stub.
- [ ] Replace `AlwaysUpProbe` usage where it is acting as production health. `crates/roko-core/src/obs/health.rs:188` explicitly calls it a placeholder while real probes are implemented.

### TUI and dashboard parity still include scaffold-only views

- [ ] Replace `DashboardScaffold` in `crates/roko-cli/src/tui/dashboard.rs:112` with pages backed by real state/query contracts. It is described as an in-memory scaffold of placeholder dashboard pages.
- [ ] Wire marketplace job creation in `crates/roko-cli/src/surface_inventory.rs:1324` and `:1354`. `CreateJob` is marked as a stub with backend submission not wired.
- [ ] Wire inline PRD editing in `crates/roko-cli/src/surface_inventory.rs:1366`. The TUI Atelier view is read-only while the CLI has a fuller lifecycle.
- [ ] Resolve `surface_inventory` partial/stub entries before claiming UI parity. The inventory itself records stub, partial, and missing counts, so parity is not complete by the code's own model.

### Mirage compatibility layer is broader than the first audit captured

- [ ] Decide whether `apps/mirage-rs/src/chain/agent.rs:79` should remain. It is a runtime-local legacy agent entity for `chain_*` and `/api/agents/*` flows.
- [ ] Replace the POC bypass path in `apps/mirage-rs/src/chain/projection.rs:8` if projection is supposed to be canonical.
- [ ] Implement or isolate stubbed precompile methods in `apps/mirage-rs/src/precompiles/mod.rs:14`. `insert` and `remove` return `NotImplemented`.
- [ ] Replace the minimal trace stub in `apps/mirage-rs/src/rpc.rs:1662` with full trace-level observability if the UI/debugging path depends on it.
- [ ] Remove Anvil/Hardhat mining no-op stubs in `apps/mirage-rs/src/rpc.rs:2480` from the production API surface or make them explicitly compatibility-only.
- [ ] Revisit `apps/mirage-rs/src/http_api/isfr.rs:27`, `:44`, `:88`, and `:119`. HTTP ISFR falls back to local snapshots when RPC calls fail, which can hide broken chain-side behavior.

### Demo and Docker defaults can mask missing production wiring

- [ ] Replace `demo/demo-resources/roko.toml:9` or mark the whole demo config as non-production. It passes `args = ["demo-stub"]`.
- [ ] Move `crates/roko-demo/src/scenarios/llm.rs` behind a demo-only binary boundary. The deterministic `StubLlm` is the default backend so CI is reproducible, but that can make "end to end" demos look healthier than production provider wiring.
- [ ] Audit `demo/demo-old/index.html:2459` and related static JS compatibility handlers. The old dashboard still parses legacy JSON-RPC formats and can diverge from the current serve API.
- [ ] Replace Docker gateway placeholders before publishing images as gateway artifacts. The Dockerfile currently exercises topology rather than delivering a real gateway runtime.

### Learning, routing, and conductor policy still have bootstrap fallbacks

- [ ] Replace the hardcoded routing bootstrap in `crates/roko-learn/src/cascade/types.rs:22`. Stage 1 uses a hardcoded role-to-model table until enough observations exist.
- [ ] Replace fallback shift logic in `crates/roko-learn/src/cascade/helpers.rs:257`. Behavioral-state routing uses a hardcoded per-state shift when thresholds are missing.
- [ ] Implement the learned conductor policy behind `crates/roko-agent/src/task_runner.rs:334`. It is currently labeled as a placeholder.
- [ ] Implement `crates/roko-conductor/src/federation.rs:199`. Fleet-level cross-agent coordination is still a passthrough stub.
- [ ] Finish wiring `crates/roko-cli/src/run_inline.rs:86` so capture episodes include actual cost from the run report.

### Standard library defaults can produce false confidence if used in production

- [ ] Confirm no production constructor defaults to `roko-std::NoOpGate`. `crates/roko-std/src/noop.rs` includes a gate that always passes.
- [ ] Confirm `roko-std::NoOpRouter`, `NoOpComposer`, and related no-op trait implementations are test/bring-up only and cannot satisfy real acceptance criteria.
- [ ] Confirm `crates/roko-std/src/tool/mock_dispatcher.rs` is never reachable through production tool dispatch config. It returns canned tool results and records calls, which is correct for tests but wrong for live agent work.

## Marker hotspot proof

Highest marker-count files in `crates`:

- `crates/roko-cli/src/orchestrate.rs`: `74`
- `crates/roko-agent/src/multi_pool.rs`: `67`
- `crates/roko-std/src/tool/mock_dispatcher.rs`: `48`
- `crates/roko-cli/src/config_cmd.rs`: `43`
- `crates/roko-cli/src/config.rs`: `41`
- `crates/roko-serve/src/parity.rs`: `39`
- `crates/roko-agent/src/provider/mod.rs`: `29`
- `crates/roko-gate/src/rung_dispatch.rs`: `17`
- `crates/roko-cli/src/runner/event_loop.rs`: `7`

Highest marker-count files in `apps`:

- `apps/mirage-rs/src/provider.rs`: `87`
- `apps/mirage-rs/src/rpc.rs`: `43`
- `apps/mirage-rs/src/fork.rs`: `25`
- `apps/mirage-rs/src/chain_rpc.rs`: `23`
- `apps/mirage-rs/src/replay.rs`: `16`
- `apps/mirage-rs/src/integration.rs`: `13`
- `apps/mirage-rs/src/scenario.rs`: `12`

## Read this with `25-CODE-ONLY-LEGACY-AUDIT.md`

This file does not replace the earlier audit. Together they say:

- the runtime/orchestrator/provider/gate path is still not fully reconciled
- several exported crates still include deliberate phase-2 stubs
- the server/gateway/UI surfaces have placeholders and partial parity entries
- demos and Docker images can exercise topology without proving production behavior
- no-op/mock defaults exist and must be kept out of acceptance-critical runtime paths

## 2026-04-27 Deepening Pass - Stub/Fallback Classification And Owner Mapping

### Self-grade for this deepening pass

Initial rating: `9.90 / 10`.

Rationale: this pass upgrades the repository-wide marker audit from a loose grep report into an implementation-grade retirement plan. It now records current scope evidence, separates production blockers from expected test/demo markers, assigns every major marker family to an architectural owner document, defines a generated inventory artifact, and gives no-context checklists with proof gates. The remaining gap is that the generated inventory is specified here but not yet produced by a checked-in scanner.

### Current source refresh

Run these commands from the repository root when refreshing this doc:

```bash
git ls-files | wc -l
git ls-files | rg -v '(^|/)(docs|tmp/mori-diffs|tmp/unified|tmp/unified-depth)(/|$)|\.(md|mdx|rst|txt)$' | wc -l
git ls-files | rg -v '(^|/)(docs|tmp/mori-diffs|tmp/unified|tmp/unified-depth)(/|$)|\.(md|mdx|rst|txt)$' | cut -d/ -f1 | sort | uniq -c | sort -nr
rg -n "TODO|FIXME|HACK|stub|placeholder|unimplemented!|todo!|noop|no-op|fake|dummy|mock|legacy|compat|fallback|hardcoded|bypass|one-off|temporary|test-only|not yet supported|not wired|disabled" crates apps tests demo examples docker contracts -g '!**/target/**' --stats
```

Observed on 2026-04-27:

- [ ] Treat the older `1798` tracked-file count as historical only; the current checkout reports `1862` tracked files.
- [ ] Treat the older `1312` non-doc implementation/config/script/UI count as historical only; the current checkout reports `1375`.
- [ ] Use this top-level non-doc scope as the current broad coverage set: `1146 crates`, `90 apps`, `56 demo`, `25 contracts`, `10 docker`, `8 examples`, `7 tests`, `7 .github`, `6 plans`, `5 tools`, plus root configs.
- [ ] Treat the old `484` marker-file count as a narrower historical scan. A broad 2026-04-27 marker pass across `crates`, `apps`, `tests`, `demo`, `examples`, `docker`, and `contracts` found `1347` files with at least one marker-bearing term.
- [ ] Do not interpret `1347` as `1347 production bugs`. The scan intentionally includes tests, demos, generated bundles, contract fixtures, compatibility shims, and docs-adjacent READMEs.
- [ ] Do interpret source markers in production runtime paths as blockers until classified and either retired, gated, or explicitly proven safe.

### Classification taxonomy

Every marker finding must be assigned exactly one classification before it can be closed:

- [ ] `production_blocker`: reachable from `roko`, `roko serve`, provider dispatch, runner, HTTP, TUI, gateway, merge, gate, workspace, or artifact paths without an explicit test/demo/experimental switch.
- [ ] `runtime_risk`: not on the default path, but reachable through normal config, feature flags, public HTTP endpoints, CLI subcommands, or documented user workflows.
- [ ] `experimental_surface`: intentionally parked phase-2 or future design code that is compiled/exported but must not be reachable from production defaults.
- [ ] `compatibility_surface`: legacy/Mirage/EVM/Anvil/Hardhat compatibility behavior that is acceptable only if labeled, isolated, and excluded from Mori parity claims.
- [ ] `test_fixture`: marker exists only in `tests`, test-only fixtures, ignored parity tests, or `#[cfg(test)]` blocks.
- [ ] `demo_only`: marker exists only under demo binaries/resources and cannot affect production binaries, provider proof, or user acceptance claims.
- [ ] `generated_or_vendor`: marker comes from generated frontend bundles, package-lock files, ABI outputs, or vendored snapshots and should be excluded from implementation triage.
- [ ] `acceptable_fallback`: fallback is safe, typed, observable, explicitly emitted in durable events, and covered by proof.
- [ ] `forbidden_fallback`: fallback hides missing production behavior, silently converts errors into success, routes to `cat`/`ExecAgent`/mock/no-op behavior, or weakens gates without explicit user-visible status.

### Current high-signal anchors

These anchors are the first-pass queue. An implementation agent should update this list with the generated inventory instead of relying on memory:

- [ ] `docker/gateway.Dockerfile:4` and `docker/gateway.Dockerfile:32`: gateway image still states that `roko-gateway` crate does not exist and copies `target/release/roko` into `/roko-gateway`.
- [ ] `docker/docker-compose.yml:6`: compose still labels the gateway service as placeholder.
- [ ] `crates/roko-chain/src/phase2.rs:18`, `:849`, `:854`, and `:859`: phase-2 chain stubs and `todo!()` HDC precompile methods.
- [ ] `crates/roko-gate/src/rung_dispatch.rs:132`, `:146`, `:173`, `:186`, `:201`, `:220`, and `:237`: rung dispatch returns typed stub verdicts when critical gate inputs/oracles are absent.
- [ ] `crates/roko-std/src/noop.rs:30` and `crates/roko-std/src/lib.rs:29`: `NoOpGate`, `NoOpRouter`, `NoOpComposer`, `NoOpPolicy`, and `NoOpScorer` are exported and must not satisfy production acceptance.
- [ ] `crates/roko-agent/src/provider/mod.rs:173`: provider construction can fall back to `ExecAgent` with no tool support if no provider is found.
- [ ] `crates/roko-cli/src/runner/agent_stream.rs:131` and `crates/roko-cli/src/dispatch_v2.rs:122`: legacy runner-program conversion remains available and must be explicitly classified.
- [ ] `crates/roko-cli/src/unified.rs:95` and `crates/roko-cli/src/chat_inline.rs:1475`: direct dispatch bypasses the canonical model-call service planned in [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] `crates/roko-cli/src/runner/gate_dispatch.rs:49`: `RungExecutionInputs::default()` can make missing gate inputs look like a legitimate execution path.
- [ ] `crates/roko-cli/src/commands/util.rs:140`, `crates/roko-cli/src/chat_inline.rs:2314`, `crates/roko-cli/src/agent_serve.rs:1324`, and `crates/roko-fs/src/layout.rs:177`: `signals.jsonl` compatibility still appears beside `engrams.jsonl` paths and needs a single migration/alias policy.
- [ ] `crates/roko-core/src/obs/health.rs:190`, `crates/roko-cli/src/orchestrate.rs:5504`, and `crates/roko-cli/src/commands/util.rs:657`: `AlwaysUpProbe` can report health without proving runtime dependencies.
- [ ] `crates/roko-serve/src/routes/gateway.rs:439`: `cache_hits` is hardcoded to `0`; gateway/cache claims cannot be considered proven.
- [ ] `crates/roko-cli/src/tui/dashboard.rs:114`, `crates/roko-cli/src/serve_runtime.rs:109`, and `crates/roko-cli/src/commands/dashboard.rs:56`: `DashboardScaffold` remains an adapter/scaffold surface rather than a pure projection reader.
- [ ] `crates/roko-cli/src/surface_inventory.rs:711`, `:869`, `:906`, `:1324`, and `:1349`: `CreateJob` is still listed as stub/partial by the code's own inventory, even though core/server job request types exist.
- [ ] `crates/roko-serve/src/job_runner.rs:413`: job runner instantiates `MockChainClient::local()` and must be treated as demo/runtime-risk until chain dependency injection is explicit.
- [ ] `demo/demo-resources/roko.toml:9` and `crates/roko-demo/src/scenarios/llm.rs:4`: demo defaults can make deterministic stub LLM behavior look like provider proof.
- [ ] `crates/roko-learn/src/model_experiment.rs:751`, `crates/roko-serve/src/routes/providers.rs:150`, `crates/roko-serve/src/routes/gateway.rs:854`, and `crates/roko-cli/src/runner/event_loop.rs:2432`: `DaimonPolicy::default()` appears in live services and should become a resolved runtime policy record.

### Owner mapping

Use this routing table to avoid one-off fixes:

- [ ] Provider, model, direct dispatch, `ExecAgent`, cache, cost, batch, and prompt-diagnostics findings belong to [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] Runner, gate, retry, replan, merge, task state, `RungExecutionInputs`, and legacy `orchestrate.rs` execution findings belong to [39-RUNNER-EXECUTION-POLICY-AUDIT.md](39-RUNNER-EXECUTION-POLICY-AUDIT.md).
- [ ] HTTP route, TUI, dashboard scaffold, operation, projection, and adapter findings belong to [40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md](40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md).
- [ ] Event store, `signals.jsonl`, `engrams.jsonl`, durable event, projection, runtime query, and proof bundle findings belong to [34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md](34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md).
- [ ] Config, secret, provider registry, fallback policy, default policy, health policy, and `DaimonPolicy::default()` findings belong to [33-CONFIGURATION-PROVIDER-POLICY-AUDIT.md](33-CONFIGURATION-PROVIDER-POLICY-AUDIT.md).
- [ ] Crate layering, exported no-op types, dependency inversion, app-service boundary, and mock leakage findings belong to [32-DEPENDENCY-LAYERING-AUDIT.md](32-DEPENDENCY-LAYERING-AUDIT.md).
- [ ] Workspace, artifact location, clean-clone proof, Docker, CI, proof harness, generated inventory, and file layout findings belong to [27-FILESYSTEM-RUNTIME-CI-AUDIT.md](27-FILESYSTEM-RUNTIME-CI-AUDIT.md) and [37-WORKSPACE-LAYOUT-ARTIFACT-STORE-AUDIT.md](37-WORKSPACE-LAYOUT-ARTIFACT-STORE-AUDIT.md).
- [ ] Learning, routing, dreams, feedback, conductor, cognitive policy, and memory-loop findings belong to [38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md](38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md).
- [ ] Side-effect duplication, direct filesystem/process/network/git writes, scattered stores, and hidden mutation findings belong to [30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md](30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md).
- [ ] Feature parity claims, dogfood issues, UX counts, stale matrices, and source-doc correction belong to [28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md](28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT.md).
- [ ] Global priority, stop conditions, and final closure status belong to [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md).

### Generated inventory contract

Add a tracked scanner that writes `tmp/mori-diffs/generated/repository-marker-inventory.json`. The file should be reproducible from a clean clone and should not require network or credentials.

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-27T00:00:00Z",
  "repository_root": ".",
  "tracked_files": 1862,
  "non_doc_scope_files": 1375,
  "scan_terms": ["TODO", "FIXME", "stub", "placeholder", "unimplemented!", "todo!", "noop", "mock", "legacy", "fallback", "bypass"],
  "findings": [
    {
      "id": "repo-marker-0001",
      "path": "crates/roko-gate/src/rung_dispatch.rs",
      "line": 132,
      "term": "stub",
      "snippet_hash": "sha256:...",
      "classification": "production_blocker",
      "owner_doc": "39-RUNNER-EXECUTION-POLICY-AUDIT.md",
      "owner_gap": "gate-input-contract",
      "reachable_from": ["roko plan run", "runner v2 gate dispatch"],
      "allowed_reason": null,
      "required_action": "replace stub verdict path with explicit missing-input failure or wire the required input/oracle",
      "proof_required": ["unit", "runtime_event", "http_projection", "proof_bundle"],
      "status": "open"
    }
  ],
  "summary": {
    "production_blocker": 0,
    "runtime_risk": 0,
    "experimental_surface": 0,
    "compatibility_surface": 0,
    "test_fixture": 0,
    "demo_only": 0,
    "generated_or_vendor": 0,
    "acceptable_fallback": 0,
    "forbidden_fallback": 0
  }
}
```

Implementation requirements:

- [ ] Add a scanner command or script under a tracked path, preferably `tests/proof/mori-diffs/scan-repository-markers.sh` or a Rust `xtask` if one is introduced.
- [ ] Exclude generated frontend bundles, lockfiles, ABI artifacts, and test fixtures only by explicit rules recorded in the JSON output.
- [ ] Include all scanned roots in the JSON output: `crates`, `apps`, `tests`, `demo`, `examples`, `docker`, `contracts`, `.github`, `deploy`, `tools`, root configs, and `plans`.
- [ ] Include a stable `snippet_hash` instead of relying only on line numbers so moved code is detectable.
- [ ] Require every non-excluded finding to have `classification`, `owner_doc`, `required_action`, `proof_required`, and `status`.
- [ ] Fail the proof script if any source-path finding outside tests/demos/generated files is unclassified.
- [ ] Fail the proof script if a `production_blocker` finding lacks an owner doc and proof requirement.
- [ ] Append inventory summary into [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md) so closure status is not split across files.

### Implementation batch RF-01 - Marker scanner and inventory

- [ ] Create the tracked scanner.
- [ ] Make the scanner deterministic: sorted file order, sorted findings, stable IDs by path/line/term/hash, no absolute paths in output.
- [ ] Record the exact query terms and root paths in output.
- [ ] Implement explicit exclusions for generated/vendor/test/demo categories and include exclusion reasons in output.
- [ ] Generate `tmp/mori-diffs/generated/repository-marker-inventory.json`.
- [ ] Add a proof command that exits non-zero when unclassified production-path markers exist.
- [ ] Add a README entry showing the scanner command and the expected output path.
- [ ] Update this doc with the generated summary counts.

Proof:

- [ ] `bash tests/proof/mori-diffs/scan-repository-markers.sh`
- [ ] `jq '.summary' tmp/mori-diffs/generated/repository-marker-inventory.json`
- [ ] `jq -e '.findings[] | select(.classification == null or .owner_doc == null)' tmp/mori-diffs/generated/repository-marker-inventory.json` must return no rows for production-path findings.

### Implementation batch RF-02 - Production fallback firewall

- [ ] Add a single runtime fallback policy type that classifies fallback behavior as `disabled`, `explicit_user_requested`, `safe_degraded`, or `forbidden`.
- [ ] Route provider fallback, `ExecAgent` fallback, no-op gate fallback, health fallback, chain mock fallback, cache fallback, and legacy file fallback through that policy.
- [ ] Emit a durable runtime event for every fallback decision.
- [ ] Make forbidden fallbacks fail fast with an actionable error instead of silently succeeding.
- [ ] Require explicit config for `ExecAgent` fallback and mark it as no-tool-support in provider status/projections.
- [ ] Prevent `NoOpGate` and other no-op standard implementations from being constructed by production runtime builders.
- [ ] Add a config validation error when production service paths select mock/no-op/default policy types.
- [ ] Add proof that a misconfigured provider no longer falls back to `cat` or generic `ExecAgent` without an explicit status.

Proof:

- [ ] Provider missing config produces `provider_unavailable` or `missing_credentials`, not success.
- [ ] Gate missing input produces a typed gate failure, not pass or stub success.
- [ ] Health endpoint reports degraded/unready when real probes are absent, not always-up.
- [ ] Runtime event query exposes the fallback decision and policy source.

### Implementation batch RF-03 - Gateway and model-call surface

- [ ] Decide whether `roko-gateway` is a real binary or remove the gateway image from release workflows.
- [ ] If real, create the gateway binary/service around the model-call service described in [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] Replace `docker/gateway.Dockerfile` placeholder copy of the CLI binary with the real gateway artifact.
- [ ] Replace hardcoded gateway cache metrics with the model-call cache/cost ledger.
- [ ] Route `unified.rs` and `chat_inline.rs` through the same gateway/dispatcher path as runner execution.
- [ ] Publish provider lifecycle, prompt diagnostics, cache outcome, token/cost, retry, and model-selection events.
- [ ] Add HTTP query endpoints that expose model-call runs, provider status, cache stats, and prompt diagnostics.
- [ ] Add provider matrix proof covering Anthropic, OpenAI, Moonshot, Z.AI, Perplexity, Claude CLI, and Codex CLI with the status taxonomy from [27-FILESYSTEM-RUNTIME-CI-AUDIT.md](27-FILESYSTEM-RUNTIME-CI-AUDIT.md).

Proof:

- [ ] `docker build -f docker/gateway.Dockerfile .` builds the real gateway binary or the Dockerfile is removed from release scope.
- [ ] A real provider call through CLI and HTTP produces the same model-call event shape.
- [ ] Gateway metrics show real cache hit/miss counters instead of fixed zeroes.
- [ ] Provider matrix output records `proved`, `missing_credentials`, `auth_failed`, `rate_limited`, or `unsupported` for every provider.

### Implementation batch RF-04 - Gate and runner input integrity

- [ ] Replace `RungExecutionInputs::default()` in runner gate dispatch with a real input assembly record.
- [ ] Make absent symbols, source roots, generated tests, verify scripts, fact-check content, judge payloads, and judge oracles explicit typed failures.
- [ ] Emit `gate_input_missing`, `gate_skipped_by_policy`, `gate_stub_blocked`, and `gate_executed` runtime events.
- [ ] Wire gate retry and replan decisions through the runner reducer described in [39-RUNNER-EXECUTION-POLICY-AUDIT.md](39-RUNNER-EXECUTION-POLICY-AUDIT.md).
- [ ] Ensure stub verdicts cannot be interpreted as successful verification.
- [ ] Add HTTP and TUI projections for skipped/missing/stub-blocked gates.
- [ ] Prove merge is blocked when acceptance-critical gates are missing.

Proof:

- [ ] A plan with missing fact-check content reports a typed missing-input gate result.
- [ ] A plan with all gate inputs wired reaches real gate execution.
- [ ] A gate failure triggers the configured retry/replan path.
- [ ] HTTP query and TUI projection show the same gate status.

### Implementation batch RF-05 - Dashboard/TUI scaffold retirement

- [ ] Treat `DashboardScaffold` as an adapter-only compatibility shim until it is removed.
- [ ] Move dashboard data reads to query services owned by [40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md](40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md).
- [ ] Replace scaffold-created page state with projection-backed pages.
- [ ] Replace `CreateJob` stub/partial paths with one command path that writes through the job service and emits durable events.
- [ ] Ensure TUI and HTTP job creation share validation, storage, events, and error shapes.
- [ ] Update `surface_inventory` so each migrated view is marked `wired` only after proof.
- [ ] Remove or clearly label remaining placeholder pages as non-parity.

Proof:

- [ ] Creating a job in TUI results in the same durable job artifact and event as `POST /api/jobs`.
- [ ] Dashboard/TUI pages can be rebuilt from runtime query endpoints after process restart.
- [ ] `surface_inventory` has no `stub` or `partial` status for pages claimed as Mori parity.

### Implementation batch RF-06 - Chain/mock/demo boundary

- [ ] Feature-gate or remove exported phase-2 chain stub modules from production defaults.
- [ ] Replace HDC `todo!()` methods or make them unreachable without an experimental feature.
- [ ] Move `MockChainClient::local()` construction behind test/demo-only providers or explicit local-sim config.
- [ ] Ensure `roko-serve` job runner receives a chain client through dependency injection rather than constructing a mock internally.
- [ ] Label Mirage compatibility endpoints and local fallbacks as compatibility surfaces, not Mori parity.
- [ ] Keep deterministic `StubLlm` demo behavior under demo-only binaries/configs.
- [ ] Add a proof failure if production provider/chain proof uses demo stub LLM or mock chain without explicit `demo_only` classification.

Proof:

- [ ] Production build cannot reach HDC `todo!()` methods through default features.
- [ ] Serve job runner config without a real chain client is degraded or disabled, not silently backed by a mock.
- [ ] Demo proof reports `demo_only` and cannot be used as provider proof.

### Implementation batch RF-07 - File-event compatibility cleanup

- [ ] Define one canonical event log name and migration policy for `signals.jsonl` versus `engrams.jsonl`.
- [ ] Keep legacy read aliases only in a repository/file-store layer, never in feature code.
- [ ] Replace scattered `signals.jsonl` direct reads with event-store/query calls.
- [ ] Emit a migration event when legacy signals are renamed/imported.
- [ ] Update docs, CLI help, and demo READMEs so they do not advertise stale paths as canonical.
- [ ] Add a proof that a workspace containing only legacy `signals.jsonl` is migrated or queried correctly.
- [ ] Add a proof that new runs write only the canonical path unless legacy export is explicitly requested.

Proof:

- [ ] Fresh run writes the canonical runtime event log.
- [ ] Legacy workspace resumes with migration evidence.
- [ ] HTTP/TUI queries use projection services and do not read legacy paths directly.

### Implementation batch RF-08 - Status and archive hygiene

- [ ] Mark every item in this file as one of `open`, `implemented_unproven`, `proved`, `superseded`, `demo_only`, `test_only`, `compatibility_only`, or `wont_fix_with_reason`.
- [ ] Do not archive this doc while any `production_blocker` or `runtime_risk` item remains open.
- [ ] If a finding is superseded by a deeper doc, add the exact target checklist item and proof gate.
- [ ] Update [README.md](README.md), [23-HANDOFF-OPEN-ITEMS.md](23-HANDOFF-OPEN-ITEMS.md), and [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md) when status changes.
- [ ] Regenerate the marker inventory after every major architecture refactor.
- [ ] Keep old grep counts only as historical evidence; use generated JSON for current status.

Proof:

- [ ] `tmp/mori-diffs/generated/repository-marker-inventory.json` exists and matches the current tree.
- [ ] Every production-path marker has an owner and status.
- [ ] README latest-pass entry lists the current self-grade and scope.
- [ ] No archived doc contains unchecked production blockers not represented in the active ledger.

### Definition of complete

- [ ] A clean clone can run the marker scanner and produce the same inventory schema.
- [ ] Every marker outside tests/demos/generated/vendor paths is classified.
- [ ] Every `production_blocker` has an owning architecture doc, implementation checklist, and proof gate.
- [ ] Every `forbidden_fallback` is either removed or converted to explicit degraded/failure behavior with durable events.
- [ ] No default production path uses mock, no-op, stub, placeholder, `todo!()`, `unimplemented!()`, hidden `ExecAgent`, hardcoded always-up health, hardcoded cache metrics, or demo-stub provider behavior.
- [ ] HTTP and TUI parity claims are backed by runtime query/projection data, not scaffolds.
- [ ] Provider, gate, merge, resume, job, workspace, and gateway proof scripts fail if a stub/demo/mock path is used as success evidence.
- [ ] [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md) records this doc as either fully proved or blocked by specific open P0/P1 items.
