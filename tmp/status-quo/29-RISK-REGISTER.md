# Risk Register

**Date**: 2026-07-08 · HEAD `5852c93c05` on `main`. Cross-refs: [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md), [75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md), [60-STATE-PERSISTENCE-LEDGER.md](60-STATE-PERSISTENCE-LEDGER.md).

This register tracks risks that can create false confidence, data loss, security exposure, or migration churn.

| Risk | Current signal | Impact | Mitigation | Proof |
|---|---|---|---|---|
| Default plan path reports synthetic success | Clap default `graph` (`main.rs:1362`) overrides enum `#[default] RunnerV2`; `TaskExecutorCell` dry-runs → `task-output:stub:` | Users think work ran when it did not ($0, 0 agents) | Flip default or fail closed | Default smoke run detects real provider/gate output; fails on stub marker |
| Resume ignores snapshots | `roko resume` hardcodes Graph (`main.rs:2699`) while Graph warns resume unsupported | Duplicate work or lost recovery | Route to Runner v2 | Resume E2E skips completed task |
| Unauthenticated relay proxy | `/relay/*` + 2 WS merged outside `/api` (`routes/mod.rs:248`) | Remote unauthed read/write on any non-loopback deploy | Nest under auth stack | 401-without-key test |
| Read-scope authorizes writes | Mutating `/api/*` falls to `read` (`middleware.rs:385`) | Read key mutates run/jobs/dream/deploy | Deny-by-default scope + CI classifier | Read key rejected on a mutating route |
| False audit assurance | `custody verify` prints "OK" but only checks JSON-parse+timestamps (`custody.rs:206`) | Operators trust a tamper check that does not run | Wire the real hash-chained audit or relabel | Verify fails on a tampered chain |
| Secret exposure | Post-checks `Warn`-only (`safety/mod.rs:767`); `config show --effective` unredacted; worker callback unauthed | Secrets leak in logs/output/turns | `Block` post-checks; redact display; auth callback | Leaking turn denied; seeded key redacted |
| Broken command looks healthy | `research search` → Perplexity 422; mock tests false-green | Users get silent no-results; CI stays green | Fix body + live test | Live search returns results |
| Provider tool stripping | Alias casing bug (`openai_compat.rs:252,348`) strips all tools on non-Claude providers | research analyze/enhance/prd agents fail on OpenAI/Gemini/Ollama | Normalize alias casing | Non-Claude agent completes a tool call |
| State split hides regressions | Gate verdicts → `signals.jsonl` but dashboards read `engrams.jsonl`; serve reads missing `state/executor.json`; 44 MB `feed_tick` firehose | Empty dashboards, serve errors, learning disagreement | Canonical state map + migration; converge signals↔engrams | One run visible identically across stores |
| API/frontend drift | 4 frontend 404s (share/shared, bench/matrix, isfr/stream, ws/agents) + camel/snake event drift | Broken UI surfaces | Generate route manifest and test frontend calls | Route contract test passes |
| Safety coverage fragmentation | Pre-checks fail closed; post-checks advisory; ACP permission gate has zero callers | Tool or provider path can bypass policy | Cross-surface safety integration tests; wire ACP gate | Same denial across CLI, ACP, serve, agent loop |
| **Per-tool safety bypassed on default provider** ([99](99-TRACE-AGENT-TURN.md)) | `ToolDispatcher`→`SafetyLayer` 9-policy pre-check runs only on OpenAI-compat `ToolLoop`; default Claude-CLI/Codex drive own subprocess loop with `--dangerously-skip-permissions:true` | The default self-host path executes tool calls with no roko role/bash/net/path/budget/contract gating | Route Claude/Codex through a roko pre-check, or ratify [BYPASS] with `build_settings_json`-encoded policy + test | Same denial fires on CLI and `ToolLoop` for a forbidden tool |
| **Adaptive gates are not live** ([101](101-TRACE-GATE-PIPELINE.md)) | Live path uses `RungExecutionInputs::default()`, never `enrich_rung_config`; rungs 3-6 stub-pass `Verdict::pass`; EMA only updates rung 2; `GateThresholds::save` never called | False confidence: SPC/oracles/ratchet/VerdictPublisher advertised but dark; stub passes inflate the rung-2 EMA toward 1.0 | Port enrichment into `run_gate_once`; stubs=Skipped excluded from EMA; persist per-rung thresholds | Advanced rung fails a bad diff; threshold file changes across runs |
| Stub gates affect learning | Stub gate verdicts (`stub_verdict → Verdict::pass`, `rung_dispatch.rs:290`) counted by `all(passed)` | Positive learning from unproven work | Treat stub verdict as non-success everywhere | Learning tests reject stub gate pass |
| No intra-plan task parallelism ([96](96-TRACE-RUNNER-V2-EXECUTION.md)) | Flat `task_index` + per-plan FSM; `max_concurrent_plans=4`, one agent/plan; `task_dag.rs`/`UnifiedTaskDag` dead | Throughput/expectation gap — "parallel task execution" is per-plan only; scheduling redesign risk if assumed to be a DAG | Wire a real DAG or document the per-plan model and delete the dead DAG code | Two ready tasks in one plan run concurrently, or the design is documented |
| CascadeRouter learned state not durable ([96](96-TRACE-RUNNER-V2-EXECUTION.md)) | Dual writers to `cascade-router.json` (dispatch + subscriber); LinUCB arm state resets toward identity on restart | Routing never converges; A/B/learning gains lost each restart; write races | Single router owner/file-lock; persist + reload LinUCB matrices | Warm router state reused after a restart |
| `events.jsonl` unbounded write-only firehose ([97](97-TRACE-SERVE-LIFECYCLE.md)) | 44 MB / 97% `feed_tick`; no-op apply; bootstrap reads a different file; no rotation/cap | Disk growth; empty feed panels on reconnect; two schemas in one file | Cap/rotate or stop persisting no-op `DashboardEvent`s; hydrate snapshot | File bounded across runs; reconnect shows recent feed ticks |
| Cold-substrate unbounded growth | Archival copies-not-moves (`cold_substrate.rs:218`), hourly | Disk fills; duplicated engrams skew retrieval | Move/dedup on archive | Cold store bounded across cycles |
| Docs overclaim chain maturity | Chain/ISFR code is substantial but feature-gated/local/mocked in places | Misleading product roadmap | Mark local, mock, optional, live-chain paths separately | Chain smoke tests split mock vs live |
| Graph/Cell model split | `roko-core::Cell` and `roko-graph` node cells use different contracts | Adapter sprawl | Canonical cell trait or documented split | One graph can run agent/compose/gate without private adapters |
| Config schema drift | Core schema, CLI config, TUI config metadata, env overrides, migration all evolve independently | Operator surprises | Add config schema parity test | New field requires schema+CLI+TUI+docs updates |
| Route count inflation | Raw `.route` count includes tests, aliases, nested routers, and multiple apps | Bad progress metric | Track serve-only, workspace-wide, and frontend-called route sets separately | Route inventory generated with categories |
| Demo sprawl | `demo-app`, `demo-web`, `demo-old`, resources, generated dist all coexist | Users land on stale surfaces | Declare supported demos and archive rest | README and CI reference only supported demos |
| Tmp archaeology churn | Newest tmp designs may not be canonical | Rework from stale plans | Source ranking and archive policy | New implementation cites source priority |

## Risk Burn-Down Order

1. Execution honesty (default engine + resume).
2. Security perimeter (relay auth, scope deny-by-default, ACP permission gate, blocking post-checks).
3. Broken commands (research search, non-Claude tool alias).
4. Resume and state safety (signals↔engrams, canonical snapshot).
5. Route/frontend contract.
6. Foundation type consolidation.
7. Safety and gate proof.
8. Docs convergence.
9. Deletion/archive.

## Risk Review Checklist

- [ ] Does this feature run on the default path?
- [ ] Can the user mistake dry-run output for real output (stub marker)?
- [ ] Is any new router inside the auth stack, and does every mutating route have an explicit scope?
- [ ] Does a "verify"/"OK"/"blocked" message reflect a check that actually runs?
- [ ] Are secrets redacted everywhere they can be displayed or logged?
- [ ] Does it write to a canonical store (`engrams.jsonl`, `state-snapshot.json`)?
- [ ] Does the frontend or TUI call the same route (and field casing) the server exposes?
- [ ] Does the docs source predate the code change, and does it use the canonical counts/noun?
- [ ] Does a stub or noop influence learning, routing, or success metrics?
- [ ] Does the per-tool safety funnel run on the provider this path actually uses (not just OpenAI-compat `ToolLoop`)?
- [ ] Does a gate "adaptive"/"oracle"/"threshold" claim run on the live runner path, or only on dead `orchestrate.rs`?
- [ ] Does learned state (router/thresholds) survive a restart, or reset toward identity?
