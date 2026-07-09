# Configuration — roko.toml, Secrets, Providers, Models

> Status-quo audit · re-verified 2026-07-08 against HEAD `5852c93c05` · sources: ~35 code files read/verified · 5 design docs · git history (`2b9515938` "test coverage, config validation, and schema improvements", `bfe0f82d6` "…cold archival config, registry cleanup", `0a6684b07` "~200 tests") · 3 parallel code sweeps + 1 re-verification pass

**Re-verify note (2026-07-08):** paths and claims re-checked against current HEAD. Corrections: (a) secrets code lives at `crates/roko-core/src/secrets/` **not** `config/secrets/`; (b) `model_registry.rs` is at `config/model_registry.rs`, `BUILTIN_MODELS` has **15 entries** (not "50+", not 13); (c) `[project]` is **consumed** (`fresh_base_branch`/`name`/`default_domain` — Open Q#1 resolved, not an orphan); (d) `preflight_provider_for_model` was **rewritten** to take `&RokoConfig` + single-model check (util.rs:1883) — the tmp-feedback/33 "zero-config blocked" doc is now **half-stale** (its cited signature and aggregate-warning bug are gone) but its **core P1 gap survives**: preflight still does NOT consult `BUILTIN_MODELS`, so `model = "gpt-4o-mini"` with no explicit `[providers.openai]` still fails.

## Summary

Configuration is one of roko's **most mature subsystems**. A unified loader (`crates/roko-core/src/config/loader.rs`) replaced "12+ separate `load_roko_config` functions" (loader.rs:3-6) with a single 5-layer precedence chain, provenance tracking, diagnostics, and strict-mode validation; ~50 callsites across roko-cli/serve/dreams now use `load_config_unified`. Hot reload is **actually wired**: `roko-serve` spawns a config watcher (2s poll, 500ms debounce) that diffs sections via `hot_reload::config_diff`, applies hot-reloadable sections live, and publishes `ConfigReloaded` events (config_watcher.rs:30-80, routes/config.rs:167-211). Provider `health`/`test` make **real HTTP calls**; `config validate` even probes base_url reachability. Two migration paths exist (Mori `.mori/config.toml` → RokoConfig; config v1 → v2 provider extraction).

For the condensed config/env provenance matrix and env-var checklist, see [61-CONFIG-ENV-MATRIX.md](61-CONFIG-ENV-MATRIX.md).

The debt is structural, not missing features: (1) **two parallel schemas parse the same roko.toml** — `roko_core::RokoConfig` (32 sections, authoritative) and `roko_cli::config::Config` + legacy `ConfigLayer` merge, kept alive for CLI-only fields (`auto_plan`, `repos`, `[[gate]]`, `dreams`, `daimon`); `load_resolved_config` loads the core config **and discards it** (`let _core_validated`, config.rs:2904) before re-parsing with legacy layers. (2) **Two secrets systems**: `config set-secret` → plaintext `~/.roko/.env` vs `config secrets set/get/list/rotate` → `.roko/secrets.toml` FileStore (0600 enforced) — no keychain/vault, `rotate` == `set`. (3) The v2 "**config-as-signal**" design (Kind::Config, lineage, demurrage) is **doc-only**: no `Kind::Config` exists anywhere in crates/. (4) CLI `config show --effective` prints secrets unredacted while the serve HTTP API masks them.

## Schema census (RokoConfig, schema.rs:90-163 — consumers verified; corrections to sweep noted)

| Section | Consumer evidence | Verdict |
|---|---|---|
| `config_version`/`schema_version` | migrate warning schema.rs:309-324; `config doctor` config_cmd.rs:271-274 | ✅ |
| `project` | **CONSUMED** (re-verified): commands/server.rs:285 `project.fresh_base_branch`, :308/343/380 `project.name`; orchestrate.rs:15238/18183 `project.default_domain`; tui/config_meta.rs:531 editable | ✅ (was mislabeled orphan) |
| `prd` (`auto_plan`) | roko-serve/routes/prds.rs:165,587,1023 (auto-plan on publish) | ✅ |
| `agent` | roko-serve/dispatch.rs:331,1835; roko-agent/safety/mod.rs:319 (role_tools); resolve_model | ✅ |
| `providers` | agent.rs:1362 resolution; serve/routes/providers.rs:732; synthesis schema.rs:369-402 | ✅ |
| `models` | agent.rs:324 profile lookup; cascade keys schema.rs:656-724 | ✅ |
| `gates` | orchestrate.rs:8371,18146; roko-acp/session.rs:258; env ROKO_SKIP_TESTS schema.rs:540-545 | ✅ |
| `graduation` | roko-graph/cells/graduation.rs:50-71 (Pulse→Signal promotion) | ✅ |
| `routing` | orchestrate.rs:15754 (tier models); hot_reload.rs:224 | ✅ |
| `pipeline` | roko-cli/run.rs:603,656 (band/stage config) | ✅ |
| `budget` | orchestrate.rs:15066+ tracking; serve/dispatch.rs:453; env ROKO_BUDGET_USD | ✅ |
| `conductor` | loader.rs:724 (max_agents); **`context_pressure_enabled` self-documented dead** (schema.rs:1270-1276: "nothing… subscribes yet") | 🟡 |
| `watcher` | serve/fswatcher.rs:17-21 (FileWatchEventSource) | ✅ |
| `learning` | runner/event_loop.rs:5479 (dream_on_completion); commands/util.rs:257 (replan) | ✅ |
| `tui` | tui uses refresh_rate; light | ✅ |
| `timeouts` | dispatch/orchestration timeout enforcement | ✅ |
| `serve` | serve/lib.rs:316,394,424 (auth, otlp); prds.rs:165 auto_orchestrate | ✅ |
| `scheduler` | serve/scheduler.rs:32-36 (CronEventSource); cli/event_sources.rs:41 | ✅ |
| `webhooks` | serve/routes/webhooks.rs:58 (github.secret) | ✅ |
| `subscriptions` | **NOT orphaned** (sweep corrected): serve/dispatch.rs:642,694,808 SubscriptionRegistry seeded from config; routes/subscriptions.rs CRUD | ✅ |
| `server` | serve/lib.rs:178-191 (bind/port/CORS); main.rs:2536 | ✅ |
| `deploy` | commands/server.rs:278,507 (worker_image); serve/lib.rs:2848 | ✅ |
| `perplexity` / `gemini` | commands/research.rs:47,346 / :164-260 + orchestrate.rs:7532 (context caching) | ✅ |
| `tools` | orchestrator/service_factory.rs:132; orchestrate.rs:4225 (prefer_mcp) | ✅ |
| `chain` | serve/state.rs:1039-1066; orchestrate.rs:4568 (ChainClient, wallet_key) | ✅ |
| `relay` | serve/lib.rs:404,2579; commands/isfr.rs:71 | ✅ |
| `isfr` | serve/routes/isfr.rs:79-87; keeper spawn serve/lib.rs:2212 | ✅ |
| `feed_agents` | serve/feed_agents/mod.rs:111; epoch_tracker.rs:49 | ✅ |
| `runner` | runner/types.rs:26,1423 (plan_timeout, dangerously_skip_permissions) | ✅ |
| `agents` (Vec\<AgentDefinition\>) | commands/server.rs:24 (`roko up` discovery) | ✅ |
| `validation.strict_validation` | loader.rs:310-334 (hard errors on dangling model→provider refs) | ✅ |
| `cold_storage` | **NOT orphaned** (sweep corrected): orchestrate.rs:3875 `post_plan_cold_archival` + :8601-8604; serve/lib.rs:2097 | ✅ (commit `bfe0f82d6`) |
| *(no `[experiments]` section)* | A/B experiments live in `.roko/learn/experiments.json` (ExperimentStore), not roko.toml; `config experiments` reads that | by design |

**Duplicate schema**: `roko_cli::config::Config` (config.rs:26-70) re-declares `agent/gates/budget/providers/models/learning/tools` and adds CLI-only `auto_plan/dreams/daimon/prompt/repos/executor/runner/runtime` — parsed from the *same* roko.toml via `ConfigLayer` (config.rs:927-1050).

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Unified RokoConfig schema (32 sections, serde defaults) | v1/00-architecture/20-configuration-schema.md | roko-core/src/config/schema.rs:90-163 + 13 submodules | ✅ | `config_version=2`, example_toml() |
| Layered resolution | v1/12-interfaces/04-…:16,41 (CLI > env > file > defaults) | loader.rs:8-14: named env > `ROKO_CONFIG` > project roko.toml > global `~/.roko/config.toml` > defaults | 🟡 | No CLI-flag layer in the resolver (flags are per-command ad hoc); otherwise richer than design |
| Hierarchical env overrides `ROKO__SECTION__FIELD` | v1 layered doc | loader.rs:33-43, applied via serde roundtrip | ✅ | e.g. `ROKO__GATES__SKIP_TESTS=true` |
| Deployment profiles `[profile.<shape>]` (laptop/single-server/container/clustered/edge) | v1/12-interfaces/04-…:107-147,313 | none | ❌ | doc claims "fully implemented" (…:345) — **stale**: `load_layered` is deprecated shim (config.rs:2959) |
| Per-field provenance | v1 layered doc | provenance.rs:25-92 (File/Default/Migration/Env/LocalOverride/CliOverride); ConfigSources config.rs:2831-2872; `config show` source tags | ✅ | `load_config_validated` loader.rs:142-209 |
| Hot reload (watch + diff + apply) | v2-depth/14-config/layered-resolution-and-reload (500ms debounce) | serve/config_watcher.rs:22,30-80 (2s poll, 500ms debounce, spawned lib.rs:335,790) + hot_reload.rs:114,208 + routes/config.rs:167-211; `POST /config/reload` | ✅ | Hot sections: budget/tools/learning/gates/conductor/routing (hot_reload.rs:331-343); others stored + restart warning |
| ConfigCache (ArcSwap + notify watcher) | — | cache.rs:22; wired **only** into roko-acp (config_watch.rs:62) | 🔌 | serve uses its own poll loop instead |
| Config-as-signal (`Kind::Config`, content-addressed, demurrage) | v2/19-CONFIG.md:3,17 | **nothing** — no `Kind::Config` in crates/ (only `ErrorKind::Config`) | ❌ | Closest: `ServerEvent::ConfigReloaded` on serve bus (routes/config.rs:200) |
| Config Compose/Verify/Trigger cells | v2/19-CONFIG.md:101,172 | functional equivalents exist (merge/validate/watch) but not cell-shaped | ❌ | paradigm gap, behavior parity ok |
| Secrets trait + backends | v1/19-deployment/10-secret-management.md | `roko-core/src/secrets/mod.rs:32-48` (SecretStore trait), env.rs (`ROKO_SECRET_<NS>`), file.rs (0600 enforced open+write, atomic) + audit.rs + resolve.rs + namespace.rs | ✅ | FileStore.open fails on loose perms (secrets/file.rs:14-16,55,72) |
| Keychain / 1Password / Vault / AWS backends | design §43.4-43.6 | doc comment only (secrets/mod.rs:28 lists "env, file, vault, 1Password, AWS" but only env+file impl'd) | ❌ | |
| Secret rotation (versioned) | v1 secret-management | `rotate` defaults to `set` in trait; no backend overrides | 🟡 | |
| `config set-secret` / `check-secrets` | CLI ref | config_cmd.rs:496-502 → `~/.roko/.env` (0600, atomic, config_cmd.rs:555-600); check-secrets does **real HTTP** validation of GITHUB_TOKEN / SLACK_BOT_TOKEN (config_cmd.rs:302-372) | ✅ | duplicate of FileStore path |
| `config secrets set/get/list/rotate` | CLI ref | cli/secrets.rs:44-134 → `.roko/secrets.toml` FileStore, `namespace.key` dotted | ✅ | "profile-aware" = namespace-aware; no dev/prod profiles |
| Secrets over HTTP | — | serve/routes/secrets.rs:22-28 — CRUD + test; **values never returned** | ✅ | |
| Provider registry + env synthesis | v1/02-agents/01-provider-registry | schema.rs:236-296 (`synthesize_standard_providers`), `effective_providers` schema.rs:369-402; availability checks (binary-on-PATH / api_key_env) schema.rs:578-615 | ✅ | explicit `[providers.*]` wins over synthesized |
| `config providers list/health/test` | CLI ref | commands/config_cmd.rs:381-726; list probes base_url via **HEAD** (:1479-1484); `test` sends real per-kind requests (:468-642); `health` reads `.roko/provider-health.json` written by roko-learn/src/provider_health.rs | ✅ | |
| Model profiles + builtin registry | v1 agents docs | ModelProfile (config/provider.rs); BUILTIN_MODELS `config/model_registry.rs:33` (15 entries), `resolve_slug` :209 alias-aware; ModelRegistry wrapper config/registry.rs (🔌 not shared as Arc, NOT consulted by preflight) | ✅/🔌 | commit `bfe0f82d6` "registry cleanup" |
| `config models list/route` | CLI ref | commands/config_cmd.rs:728-817 / :915-1053 — `route` explains via persisted CascadeRouter state (explain-only, no dispatch) | ✅ | |
| `config validate` (3-phase + reachability) | commit `2b9515938` | config_cmd.rs:375-446 → semantic_validate_config :1132-1321 incl. HTTP HEAD probes; reference validation schema.rs:1190-1251 | ✅ | |
| `config migrate` | — | v1→v2: extracts `[providers]`/`[models]` from legacy flat agent config (config_cmd.rs:449-493, `--dry-run`, prompt before write); Mori: `from_mori_toml` compat.rs:85-129 (one-way, drops providers/models by design) | ✅ | 9 compat tests compat.rs:272-447 |
| Serve config API | — | routes/config.rs: GET (masked :306-336), GET /config/toml (masked), PUT (deep-merge + write-back + ephemeral-workspace propagation :94-107), POST /reload | ✅ | |
| Tests | v1 32-comprehensive-test-strategy | roko-core/tests/config_loader_integration.rs; inline tests in schema/hot_reload/cache/compat/loader/secrets/routes-config | ✅ | commit `0a6684b07` (~200 tests) |

## V2-aligned

- **Single entry point** `load_config_unified` adopted at ~50 callsites (loader.rs:117; grep across roko-cli/serve/dreams) — kills the "12+ loaders" era.
- Hot-reload diff engine with section classification + event emission is the config *loop* (query→diff→apply→emit) even if not literally cell-shaped: hot_reload.rs + config_watcher.rs + `ServerEvent::ConfigReloaded`.
- Provenance (`ValidatedConfig{raw, migrated, diagnostics, provenance}`) matches v2 lineage intent (provenance.rs:122-208).
- Strict validation mode (`LoadOptions::strict()` loader.rs:102-109; `validation.strict_validation` schema.rs:170-176) + `StrictConfigSource` rejecting `dangerously_skip_permissions` in shared configs (validation.rs:36-82 via loader.rs:231-241).
- Real-world provider probing (HEAD + live `test` requests) beats the design's aspiration.
- Secret hygiene: 0600 enforcement on open *and* every write (file.rs:14-16,125-129); serve never returns secret values.

## Old paradigm & tech debt

- 🕰️ **Dual roko.toml schema**: `roko_cli::config::Config`+`ConfigLayer` (config.rs:26-70,927-1050) vs `RokoConfig`. `load_resolved_config` loads the authoritative core config then **discards it** (`let _core_validated =`, config.rs:2904-2908) and re-merges legacy layers for `auto_plan/repos/[[gate]]/dreams/daimon/runner.plan_timeout_secs` (compat notes config.rs:2886-2895). Same-file two-parser risk: fields valid in one schema are silently ignored by the other.
- 🕰️ Two secrets CLI systems (`~/.roko/.env` vs `.roko/secrets.toml`), both plaintext; EnvVarStore's `ROKO_SECRET_*` convention is a third read-path (secrets/env.rs:5).
- 🕰️ `RokoConfig::classify_changes` (schema.rs:794-849) duplicates `hot_reload::config_diff` with different section lists; only tests call it.
- CLI `config show` / `show --effective` print secrets **unredacted** (config_cmd.rs:215-229) while serve masks (routes/config.rs:306-336) — inconsistent redaction policy.
- Restart-warning formatting bug: routes/config.rs:185-191 interpolates `change.section.is_hot_reloadable()` (a bool) into the user-facing message.
- Scattered legacy wrappers remain: orchestrate.rs:867 `load_roko_config`, commands/isfr.rs:44, serve_runtime.rs:500, roko-acp/config.rs:209 (own loader + `LoadOptions::acp()` == default, loader.rs:96-98).
- `example_toml()` is hand-maintained string assembly (schema.rs:855-1105) — drifts from schema; `extract_config_version_from_text` is line-based parsing (schema.rs:72-84).
- `conductor.context_pressure_enabled` ships disabled with a comment admitting no subscriber (schema.rs:1270-1276).
- v1 layered-resolution doc **stale**: claims `load_layered` "fully implemented" (04-…:345) but it's a deprecated shim (config.rs:2954-2961); profiles never built.
- Hardcoded-not-config: claude command fallback `"claude"` (schema.rs:390-394), `auto_fix_model = "claude-haiku-4-5"` default (schema.rs:1290-1292), watcher poll cadence 2s (config_watcher.rs:46).

## Zero-config gap (tmp-feedback/2/33 — re-verified P1)

The "install roko, set one API key, everything works" path is **still blocked at the preflight boundary**, though the referenced code has since been refactored:

- `preflight_provider_for_model(config: &RokoConfig, model_key: &str)` (util.rs:1883, called from prd.rs:372/763, agent.rs:25, plan.rs:327, do_cmd.rs:189/318) resolves **only** via `config.models[key].provider` → `config.providers[provider]`. If either is absent it hard-bails. It never falls back to `BUILTIN_MODELS`.
- `BUILTIN_MODELS` (config/model_registry.rs:33, **15 entries**) + `resolve_slug` (:209, alias-aware) already know each model's provider and `api_key_env`, but nothing wires the registry into preflight or into provider synthesis for an *unlisted* model.
- No `env_api_key_for_provider` helper exists in `config/mod.rs` (tmp-feedback fix #3 unimplemented). `synthesize_standard_providers` (schema.rs) seeds the known set but a user who names a model not present in their `[models]` table gets no auto-config.
- **Stale in tmp-feedback/33:** the old `&Config`/`.providers.iter()` signature, the `preflight_providers_aggregate` "7 duplicate slug warnings on every command" bug (removed — see util.rs:1973-1975 NOTE), and the "50+ models" count. **Live:** the missing builtin-registry consultation and missing env-key auto-detect.
- Fix shape unchanged: in preflight, after the explicit-config miss, `resolve_slug(model_key)` → synthesize a `ProviderConfig` from the builtin entry when its `api_key_env` is set in the environment. ~1 file, ~15 lines.

## Not implemented

- ❌ **Config-as-signal** (v2/19-CONFIG.md:17: "content-addressed, carry lineage… subject to demurrage") — no `Kind::Config`, no config Signal in Store/Bus, no L4 "evolved" lowest-priority layer (19-CONFIG.md:101).
- ❌ Deployment-shape profiles `[profile.laptop|single-server|container|clustered|edge]` (v1/12-interfaces/04-…:132-147).
- ❌ CLI-flags as a true top resolution layer (design 04-…:41).
- ❌ OS keychain / 1Password / Vault / AWS secret backends (secrets/mod.rs:8-9 "later waves").
- ❌ Versioned secret rotation; SecretAuditLog exists (audit.rs) but no runtime writer found.
- ❌ Hot reload for CLI long-lived processes (TUI watches `.roko` data via fs_watch.rs, not roko.toml; orchestrate loads once per run). ConfigCache is ACP-only.
- ❌ `.roko/config` dir — not present at repo root (only inside stale `.roko/worktrees/*` copies); `.roko/GAPS.md` contains zero config entries.

## Migration checklist

- [ ] **[P0]** Collapse the dual schema: port `auto_plan/repos/[[gate]]/dreams/daimon` into `RokoConfig` (or a `[cli]` section), make `roko_cli::config::Config` a projection of it, delete the `ConfigLayer` merge path and the discarded `_core_validated` load — verify: `grep -rn 'ConfigLayer\|_core_validated' crates/roko-cli/src/config.rs | wc -l` → ~0; `cargo run -p roko-cli -- config show`
- [ ] **[P0]** Redact secrets in CLI output by reusing serve's `mask_secret_fields` — verify: `ANTHROPIC_API_KEY=sk-test cargo run -p roko-cli -- config show --effective | grep -c 'sk-test'` → 0
- [ ] **[P1]** Zero-config: consult `BUILTIN_MODELS`/`resolve_slug` in `preflight_provider_for_model` (util.rs:1883) and synthesize a provider from the registry entry when its `api_key_env` is present — verify: with only `OPENAI_API_KEY` set and no `[providers.openai]`, `cargo run -p roko-cli -- do "hi" --model gpt-4o-mini` gets past preflight
- [ ] **[P1]** Consolidate secrets onto `SecretStore` (secrets live at `crates/roko-core/src/secrets/`; migrate `~/.roko/.env` entries into FileStore or an env-file provider of `SecretResolver`); document one blessed path — verify: `cargo run -p roko-cli -- config secrets list` shows migrated namespaces
- [ ] **[P1]** Emit a config Signal on reload (start config-as-signal): persist `ConfigReloaded` + config hash to substrate in `reload_config_from_disk` — verify: edit roko.toml under `roko serve`, then `cargo run -p roko-cli -- status` / `roko replay <hash>` shows a config signal
- [ ] **[P1]** Share `ConfigCache` in roko-serve (replace 2s mtime poll) and expose reload to TUI/orchestrate — verify: `grep -rn 'ConfigCache::new' crates/roko-serve/src`
- [ ] **[P2]** Implement `[profile.<shape>]` merge before user overrides per v1/12-interfaces/04 — verify: roko.toml with `profile = "laptop"` changes `server.bind` default; integration test in config_loader_integration.rs
- [ ] **[P2]** Wire `SecretAuditLog` into set/rotate/delete in CLI + serve routes; add versioned rotate to FileStore — verify: `printf v2 | cargo run -p roko-cli -- config secrets rotate llm anthropic` then audit entry exists
- [ ] **[P2]** Fix restart-warning message (routes/config.rs:185-191) to name the section instead of printing a bool — verify: `curl -X POST :6677/api/config/reload` after `[agent]` edit → warning contains "agent"
- [ ] **[P3]** Delete or delegate `RokoConfig::classify_changes` to `hot_reload::config_diff` — verify: `grep -rn classify_changes crates/ --include='*.rs' | grep -v test` → empty
- [ ] **[P3]** Generate `example_toml()` from the schema via serde (roundtrip test) — verify: new test parses `RokoConfig::example_toml()` cleanly
- [ ] **[P3]** Update stale v1/12-interfaces/04 §"fully implemented" claim; add keychain backend behind a feature flag — verify: `cargo build -p roko-core --features secrets-keychain`

## Deep pass 2 — 32-section consumer table + verdicts (verified HEAD `5852c93c05`, 2026-07-08)

`RokoConfig` has exactly **32 section fields** (schema.rs:90-163, excluding the two `u32` version fields `config_version`/`schema_version`). Enumerated below with **parsed?** (serde deserializes it — universally yes, all `#[serde(default)]`), **live consumer** (a file:line that *reads the value and acts on it at runtime*), and a **dead-config verdict**. "Live runtime" here = the CLI + runner-v2 (`commands/plan.rs` → `runner/event_loop.rs`) + `roko serve` daemon; **legacy `orchestrate.rs` is 🕰️ (still compiled/reachable but the v2 runner is the blessed path)**.

| # | Section | Parsed? | Live consumer (file:line) | Verdict |
|---|---|---|---|---|
| 1 | `project` | ✅ | commands/server.rs:285/308/343/380 (`fresh_base_branch`,`name`); orchestrate.rs:15238/18183 (`default_domain`) | ✅ live |
| 2 | `prd` | ✅ | serve/routes/prds.rs:165,587,1023 (`auto_plan` on publish) | ✅ live (serve) |
| 3 | `agent` | ✅ | serve/dispatch.rs:331,1835; roko-agent/safety/mod.rs:319 (`role_tools`); resolve_model | ✅ live |
| 4 | `providers` | ✅ | roko-agent agent.rs:1362; serve/routes/providers.rs:732; synth schema.rs:369-402 | ✅ live |
| 5 | `models` | ✅ | agent.rs:324 (profile lookup); cascade keys schema.rs:656-724 | ✅ live |
| 6 | `gates` | ✅ | orchestrate.rs:8371,18146; roko-acp/session.rs:258 | ✅ live |
| 7 | `graduation` | ✅ | roko-graph/cells/graduation.rs:50-71 | ✅ live |
| 8 | `routing` | ✅ | orchestrate.rs:15754 (tier models); hot_reload.rs:224 | ✅ live |
| 9 | `pipeline` | ✅ | roko-cli/run.rs:603,656 (band/stage) | ✅ live |
| 10 | `budget` | ✅ | orchestrate.rs:15066; serve/dispatch.rs:453 | ✅ live |
| 11 | `conductor` | ✅ | **split**: `max_agents` → loader.rs:724 (✅ live); `context_pressure_enabled`+`watchers` → conductor.rs:111/181 **only, and the Conductor is constructed only from 🕰️ orchestrate.rs:6318 — runner-v2 `event_loop.rs` never builds one** | 🟡 half-dead (see below) |
| 12 | `watcher` (`[[watcher.paths]]`) | ✅ | serve/fswatcher.rs:17-21 (FileWatchEventSource) | ✅ live (serve only) |
| 13 | `learning` | ✅ | runner/event_loop.rs:5479 (`dream_on_completion`); commands/util.rs:257 (replan) | ✅ live |
| 14 | `tui` | ✅ | tui refresh_rate (light) | ✅ live |
| 15 | `timeouts` | ✅ | dispatch/orchestration timeout enforcement | ✅ live |
| 16 | `serve` | ✅ | serve/lib.rs:316,394,424 (auth, otlp, relay) | ✅ live (serve only) |
| 17 | `scheduler` | ✅ | serve/scheduler.rs:32-36 (CronEventSource); cli/event_sources.rs:41 | ✅ live (serve only) |
| 18 | `webhooks` | ✅ | serve/routes/webhooks.rs:58 (`github.secret`) | ✅ live (serve only) |
| 19 | `subscriptions` (Vec) | ✅ | serve/dispatch.rs:642,694,808 (SubscriptionRegistry seed) | ✅ live (serve only) |
| 20 | `server` | ✅ | serve/lib.rs:178-191 (bind/port/CORS); main.rs:2536 | ✅ live (serve only) |
| 21 | `deploy` | ✅ | commands/server.rs:278,507 (`worker_image`); serve/lib.rs:2848 | ✅ live |
| 22 | `perplexity` | ✅ | commands/research.rs:47,346 | ✅ live |
| 23 | `gemini` | ✅ | commands/research.rs:164-260; orchestrate.rs:7532 (context caching) | ✅ live |
| 24 | `tools` | ✅ | orchestrator/service_factory.rs:132; orchestrate.rs:4225 (`prefer_mcp`) | ✅ live |
| 25 | `chain` | ✅ | serve/state.rs:1039-1066; orchestrate.rs:4568 (ChainClient) | ✅ live |
| 26 | `relay` | ✅ | serve/lib.rs:404,2579; commands/isfr.rs:71 | ✅ live (serve only) |
| 27 | `isfr` | ✅ | serve/routes/isfr.rs:79-87; keeper spawn lib.rs:2212 | ✅ live (serve only) |
| 28 | `feed_agents` | ✅ | serve/feed_agents/mod.rs:111; epoch_tracker.rs:49 | ✅ live (serve only) |
| 29 | `runner` | ✅ | runner/types.rs:26,1423 (`plan_timeout`, `dangerously_skip_permissions`) | ✅ live |
| 30 | `agents` (Vec) | ✅ | commands/server.rs:24 (`roko up` discovery) | ✅ live |
| 31 | `validation` | ✅ | loader.rs:310-334 (`strict_validation` → hard errors) | ✅ live |
| 32 | `cold_storage` | ✅ | orchestrate.rs:3875 (`post_plan_cold_archival`) + 8601-8604; serve/lib.rs:2097 | 🟡 legacy-path only (no runner-v2 or cron trigger — see 14 in CLAUDE.md "Cold substrate archival … not instantiated at runtime") |

### Dead-config verdict (count)

**Zero top-level sections are fully orphaned** — every one of the 32 deserializes into a value that *some* code reads. The dead config lives at the **subfield / runtime-path** granularity:

1. **`conductor.watchers.*` (WatcherThresholds, 10 sub-tables, schema.rs:1315-1336)** — parsed → `let thresholds = &config.watchers` (conductor.rs:111) → but the `Conductor` is only constructed/run from 🕰️ `orchestrate.rs:6318`; runner-v2 `event_loop.rs` has *no* `Conductor::` construction (only an `AgentRole::Conductor` string map at :5145 and a hardcoded `conductor_load: 0.0` at :4258). **Dead in the v2 runtime.**
2. **`conductor.context_pressure_enabled` (schema.rs:1276)** — self-documented dead: the doc comment at schema.rs:1272-1274 admits "the watcher emits `conductor.intervention` signals but nothing in the runner event loop subscribes to them yet." Even when `true`, output is dropped.
3. **`cold_storage` (§32)** — parsed and consumed, but only on the legacy `orchestrate.rs` post-plan path; no runner-v2 hook and no cron/scheduler trigger, so under the blessed runtime it is inert (matches CLAUDE.md remaining-work item #14).

Plus the previously-catalogued dead/stale bits: `[profile.<shape>]` (parsed by the deprecated `load_layered` shim, config.rs:2954-2961, but profiles never merged — ❌), and no `[experiments]` section at all (lives in `.roko/learn/experiments.json`).

**Net: 0 dead top-level sections / ~3 runtime-dead config surfaces (conductor.watchers.*, conductor.context_pressure_enabled, cold_storage-in-v2).** The far bigger config hazard is not deadness but the **dual-parser silent drop** (below).

### Config load precedence + the two-parser silent-drop trace

**Authoritative loader precedence** (`roko_core::config::loader::load_config_validated_with_options`, loader.rs:8-14), highest → lowest:
1. **Named env overrides** — `ROKO_MODEL`, `ROKO_BUDGET_USD`, etc. (loader.rs:33-43 also applies hierarchical `ROKO__SECTION__FIELD` via serde roundtrip).
2. **`ROKO_CONFIG`** env-pointed file.
3. **Project `roko.toml`** (ancestor walk from workdir).
4. **Global `~/.roko/config.toml`**.
5. **`RokoConfig::default()`** serde defaults.
   *(No CLI-flag layer in the resolver — flags are applied per-command ad hoc; the v1 "CLI > env > file > defaults" design's top layer is unbuilt.)*

**The silent-drop path** (`roko_cli::config::load_resolved_config`, config.rs:2896-2911): the CLI calls the authoritative core loader at **config.rs:2904** — `let _core_validated = ...load_config_validated_with_options(...)` — which walks ancestors, applies env + `ROKO__*` overrides, interpolates, and resolves file secrets. **Then it throws the result away** (bound to `_core_validated`, never read; comment at config.rs:2900-2903 lists everything it does before discarding) and re-parses the *same* `roko.toml` through the legacy `ConfigLayer` merge (config.rs:927-1050) to recover CLI-only fields (`auto_plan`, `repos`, `[[gate]]`, `dreams`, `daimon`, `runner.plan_timeout_secs` — compat notes config.rs:2886-2895). 

Consequence: **two independent deserializers read one file.** A key that is valid in `RokoConfig` but absent from `roko_cli::config::Config` (config.rs:26-70) — or vice-versa — is silently ignored by whichever parser owns the runtime path for that command, with no "unknown key" warning (neither parser is `deny_unknown_fields`). The `_core_validated` call is pure validation theater on the CLI path: it can *reject* a config (map_err at config.rs:2908) but its parsed values never reach dispatch.

### The secret systems (there are effectively four read-paths)

| # | System | Write | Read | Backing | Notes |
|---|---|---|---|---|---|
| A | `config set-secret` / `check-secrets` | config_cmd path → `~/.roko/.env` (0600, atomic) | dotenv loader main.rs:3191-3193 | plaintext `~/.roko/.env` | global, **does NOT override** already-set process env (main.rs:3191) |
| B | project dotenv | manual edit | main.rs:3201-3203 | plaintext `{workdir}/.roko/.env` | **higher priority, DOES override** existing env vars |
| C | `config secrets set/get/list/rotate` | cli secrets cmd → `SecretStore::FileStore` | resolve.rs / namespace.rs | `.roko/secrets.toml` (0600 enforced on open+write, file.rs:14-16,125-129, atomic) | dotted `namespace.key`; `rotate` == `set` (no versioning); values never printed by serve |
| D | `ROKO_SECRET_<NS>_<KEY>` env convention | export | secrets/env.rs:5 (EnvVarStore) | process env | third read-path into the `SecretStore` trait |

A/B (`.env`, two locations) and C (`secrets.toml`) are **separate stores with no bridge** — a secret set via `set-secret` is invisible to `config secrets list` and vice-versa. All are plaintext; no OS keychain/Vault/1Password backend exists (secrets/mod.rs:28 lists them as doc-only). `SecretAuditLog` (audit.rs) exists but has no runtime writer. **CLI `config show --effective` prints secret values UNREDACTED** (config_cmd.rs:215-229) while serve's HTTP API masks them (routes/config.rs:306-336) — inconsistent redaction policy = the P0 leak.

## Open questions

1. ~~Is `[project]` truly consumed at runtime?~~ **RESOLVED (2026-07-08): yes** — `fresh_base_branch` (server.rs:285), `name` (server.rs:308/343/380), `default_domain` (orchestrate.rs:15238/18183), editable via tui/config_meta.rs:531. Not an orphan.
2. What is the staleness contract between `.roko/provider-health.json` (written by roko-learn/src/provider_health.rs) and `config providers health` reads — should `health` fall back to live probes when the snapshot is old?
3. Should `PUT /api/config`'s write-back serialize the **full** merged config to roko.toml (it does today, routes/config.rs:87-92) — this bakes env-derived values into the file, defeating layering. Intended?
4. `LoadOptions::acp()` is identical to default (loader.rs:96-98) — is a workspace-scoped divergence still planned, or should roko-acp drop its bespoke loader (roko-acp/src/config.rs:209)?
5. The task brief said "`.roko/config` exists" — nothing at the repo root matches; only `.roko/worktrees/*` copies. Was this a stale reference, or is a `.roko/config/` split (à la Claude Code settings dirs) planned?
