# Env Var Manifest

Generated on 2026-07-07, re-verified 2026-07-08 @ HEAD 5852c93c, from direct env reads (`env::var`/`env::var_os`), Clap `env =` bindings, hierarchical `ROKO__SECTION__FIELD` overrides (`roko-core/src/config/loader.rs:338-382`), the `ROKO_SECRET_<CATEGORY>_<PROVIDER>` secret-store convention (`roko-core/src/secrets/{env,namespace,resolve}.rs`), and build-time `cargo:rustc-env` declarations under `crates/`, `apps/`, and `demo/`. This intentionally excludes arbitrary docs/examples and broad string matches.

**Re-verification delta (2026-07-08, HEAD 5852c93c):** the table was re-grepped end-to-end. Additions/corrections: **`CI`** was entirely missing — it is read in production-adjacent code and tests for timeout scaling (`roko-agent/src/codex_agent.rs:451`, `roko-agent/src/openai_compat_backend.rs:898`, `roko-gate/src/{shell.rs:197,integration_gate.rs:550,verify_chain_gate.rs:534}`, `roko-agent-server/src/features/logs.rs:115`; all inside `#[cfg(test)]`/test helpers → `test-only`), now added below. **`ROKO_SECRET_*`** family (dynamic prefix, not a fixed var) documented as a note. Newly-catalogued *additional locations* for already-listed vars: `GITHUB_TOKEN` also `roko-serve/src/templates.rs:430`; `ANTHROPIC_API_KEY` also `roko-serve/src/routes/{templates.rs:275,deployments.rs:115}`, `roko-cli/src/run.rs:{2384,2402,3727}`, `roko-cli/src/doctor.rs:722`, `roko-cli/src/chat_inline.rs:3233`; `SHELL` also `roko-serve/src/terminal.rs:755`; `PATH` also `roko-core/src/config/schema.rs:47`, `roko-gate/src/payload.rs:{293,535}`, `roko-cli/src/config.rs:3378`; `ROKO_CONFIG` also `roko-core/src/config/loader.rs:{549,581}`, `roko-cli/src/doctor.rs:234`; `ROKO_MIRAGE_URL` also `apps/mirage-rs/src/rpc.rs` relay path; `ANTHROPIC_API_KEY` clap-env also `roko-core/src/config/loader.rs:{1006,1047}`. No env var was removed. Matrix-only aspirational vars still **not found in code**: `ROKO_MODEL_SLUG` (loader doc-comment only, `loader.rs:31`), `ROKO_WORKDIR`, `ROKO_STATE_ROOT`, `ROKO_BIND`, `ROKO_PORT`, `CEREBRAS_API_KEY`, `OPENROUTER_API_KEY`, `MIRAGE_STATE_DIR` (only a child-process env *key set* in `commands/server.rs:337`, never read here), `MIRAGE_BLOCK_INTERVAL_MS`, `MIRAGE_SNAPSHOT_INTERVAL_SECS` — flagged in 61's High-Impact table but with no `env::var` read in this tree (documented as forward-looking / passed-through only).

## Counts

| Category | Variables |
|---|---:|
| `app-specific` | 32 |
| `build-info` | 4 |
| `chain-mirage-isfr` | 7 |
| `core-config-override` | 8 |
| `mcp-script` | 5 |
| `runtime-deploy` | 16 |
| `secret/auth` | 17 |
| `test-only` | 4 |
| `ui-accessibility` | 6 |
| Total | 99 |

> Counts exclude the dynamic `ROKO_SECRET_<CATEGORY>_<PROVIDER>` family (one runtime-computed prefix, see note above) and the hierarchical `ROKO__SECTION__FIELD` override mechanism (arbitrary keys, not a fixed var).

## Manifest

| Env var | Category | Kind | Code owners / first locations |
|---|---|---|---|
| `ANTHROPIC_API_KEY` | `secret/auth` | `clap-env` | `crates/roko-agent/src/provider/anthropic_api.rs:34`<br>`crates/roko-cli/src/agent_serve.rs:449`<br>`crates/roko-cli/src/auth_detect.rs:153`<br>`crates/roko-cli/src/bootstrap.rs:92` |
| `ANTHROPIC_MODEL` | `app-specific` | `direct` | `crates/roko-demo/src/scenarios/llm.rs:95` |
| `BARDO_AVAILABLE_MEMORY_BYTES` | `app-specific` | `direct` | `apps/mirage-rs/src/resources.rs:142` |
| `CARGO_MANIFEST_DIR` | `app-specific` | `direct` | `crates/roko-serve/build.rs:8` |
| `CI` | `test-only` | `direct` | `crates/roko-agent/src/codex_agent.rs:451`<br>`crates/roko-agent/src/openai_compat_backend.rs:898`<br>`crates/roko-gate/src/shell.rs:197`<br>`crates/roko-gate/src/integration_gate.rs:550`<br>`crates/roko-gate/src/verify_chain_gate.rs:534`<br>`crates/roko-agent-server/src/features/logs.rs:115` (all `#[cfg(test)]`/test-timeout scaling) |
| `CLICOLOR` | `ui-accessibility` | `direct` | `crates/roko-cli/src/inline/terminal.rs:237`<br>`crates/roko-cli/src/main.rs:183` |
| `CLICOLOR_FORCE` | `ui-accessibility` | `direct` | `crates/roko-cli/src/inline/terminal.rs:240`<br>`crates/roko-cli/src/main.rs:180` |
| `EDITOR` | `app-specific` | `direct` | `crates/roko-cli/src/config_cmd.rs:622` |
| `ETH_RPC_URL` | `chain-mirage-isfr` | `clap-env` | `apps/roko-chain-watcher/src/config.rs:53`<br>`crates/roko-chain/src/isfr_keeper.rs:195` |
| `FLY_APP_NAME` | `runtime-deploy` | `direct` | `crates/roko-serve/src/relay.rs:181` |
| `GEMINI_API_KEY` | `secret/auth` | `direct` | `crates/roko-cli/src/chat_inline.rs:3236` |
| `GH_TOKEN` | `secret/auth` | `direct` | `crates/roko-cli/src/commands/server.rs:559`<br>`crates/roko-serve/src/feedback.rs:459` |
| `GITHUB_TOKEN` | `secret/auth` | `direct` | `crates/roko-cli/src/commands/server.rs:559`<br>`crates/roko-cli/src/worker/cloud.rs:120`<br>`crates/roko-mcp-github/src/main.rs:1244`<br>`crates/roko-serve/src/feedback.rs:459` |
| `GLOBAL_KEY` | `app-specific` | `clap-env` | `crates/roko-cli/src/config.rs:3699` |
| `HOME` | `app-specific` | `direct` | `crates/roko-agent/src/mcp/config.rs:96`<br>`crates/roko-agent/src/openclaw/gateway_service.rs:74`<br>`crates/roko-agent/src/openclaw/infer_agent.rs:91`<br>`crates/roko-cli/src/chat_session.rs:1858` |
| `ISFR_SERVICE_URL` | `chain-mirage-isfr` | `direct` | `apps/mirage-rs/src/http_api/isfr.rs:49` |
| `ISFR_STRICT_PROXY` | `chain-mirage-isfr` | `direct` | `apps/mirage-rs/src/http_api/isfr.rs:83` |
| `MIRAGE_DASHBOARD_DIR` | `chain-mirage-isfr` | `direct` | `apps/mirage-rs/src/rpc.rs:915` |
| `MIRAGE_RPC_URL` | `chain-mirage-isfr` | `clap-env` | `apps/roko-chain-watcher/src/config.rs:16`<br>`crates/roko-cli/tests/chain_integration.rs:35` |
| `MIRAGE_SHUTDOWN_SECRET` | `secret/auth` | `direct` | `apps/mirage-rs/src/rpc.rs:3011` |
| `MIRAGE_TEST_PORT` | `chain-mirage-isfr` | `direct` | `apps/mirage-rs/src/rpc.rs:3805` |
| `MOONSHOT_API_KEY` | `secret/auth` | `clap-env` | `crates/roko-cli/src/chat_inline.rs:3239`<br>`demo/demo-resources/provider-routing/roko.toml:40` |
| `NO_COLOR` | `ui-accessibility` | `direct` | `crates/roko-cli/src/inline/terminal.rs:234`<br>`crates/roko-cli/src/main.rs:177`<br>`crates/roko-cli/src/tui/theme.rs:129` |
| `NUNCHI_DASHBOARD_URL` | `app-specific` | `clap-env` | `crates/roko-cli/src/main.rs:760` |
| `OLLAMA_MODEL` | `app-specific` | `direct` | `crates/roko-demo/src/scenarios/llm.rs:121` |
| `OLLAMA_URL` | `app-specific` | `direct` | `crates/roko-demo/src/scenarios/llm.rs:122` |
| `OPENAI_API_BASE` | `app-specific` | `direct` | `crates/roko-cli/src/auth_detect.rs:174` |
| `OPENAI_API_KEY` | `secret/auth` | `clap-env` | `crates/roko-agent/src/openai_compat_backend.rs:1647`<br>`crates/roko-cli/src/auth_detect.rs:172`<br>`crates/roko-cli/src/bootstrap.rs:93`<br>`crates/roko-cli/src/chat_inline.rs:3235` |
| `OPENAI_BASE_URL` | `app-specific` | `direct` | `crates/roko-cli/src/auth_detect.rs:175` |
| `PATH` | `app-specific` | `direct` | `crates/roko-acp/src/session.rs:928`<br>`crates/roko-agent/src/provider/pre_flight.rs:202`<br>`crates/roko-cli/src/config.rs:3155`<br>`crates/roko-cli/src/daemon/launchd.rs:53` |
| `PERPLEXITY_API_KEY` | `secret/auth` | `direct` | `crates/roko-cli/src/chat_inline.rs:3243`<br>`crates/roko-cli/src/commands/research.rs:731`<br>`crates/roko-cli/src/orchestrate.rs:4702`<br>`crates/roko-std/src/tool/builtin/web_search.rs:347` |
| `PORT` | `runtime-deploy` | `clap-env` | `crates/roko-cli/src/worker/mod.rs:62`<br>`crates/roko-serve/src/lib.rs:275` |
| `RAILWAY_PUBLIC_DOMAIN` | `runtime-deploy` | `direct` | `crates/roko-serve/src/relay.rs:178` |
| `RAILWAY_VOLUME_MOUNT_PATH` | `runtime-deploy` | `direct` | `apps/mirage-rs/src/main.rs:208` |
| `ROKO_ACP_LEGACY` | `app-specific` | `direct` | `crates/roko-acp/src/bridge_events.rs:1352` |
| `ROKO_ACP_TEST_UNSET_PROVIDER_KEY` | `app-specific` | `clap-env` | `crates/roko-acp/src/session.rs:1874` |
| `ROKO_AGENT_RELAY_BIND` | `runtime-deploy` | `clap-env` | `apps/agent-relay/src/main.rs:23` |
| `ROKO_AGENT_RELAY_CHAIN_ID` | `runtime-deploy` | `clap-env` | `apps/agent-relay/src/main.rs:34` |
| `ROKO_AGENT_RELAY_RPC_WS_URL` | `runtime-deploy` | `clap-env` | `apps/agent-relay/src/main.rs:29` |
| `ROKO_AGENT_RELAY_URL` | `runtime-deploy` | `direct` | `apps/mirage-rs/src/rpc.rs:118`<br>`crates/roko-serve/src/state.rs:1023` |
| `ROKO_API_KEY_ENV_NAME` | `secret/auth` | `clap-env` | `crates/roko-core/src/config/schema.rs:2060` |
| `ROKO_ATTEST_SIGNING_KEY_HEX` | `secret/auth` | `direct` | `crates/roko-cli/src/orchestrate.rs:20917`<br>`crates/roko-cli/src/task_helpers.rs:698` |
| `ROKO_BACKEND` | `core-config-override` | `direct` | `crates/roko-core/src/config/loader.rs:547` |
| `ROKO_BUDGET_USD` | `core-config-override` | `clap-env` | `crates/roko-core/src/config/schema.rs:527` |
| `ROKO_CONFIG` | `app-specific` | `direct` | `crates/roko-acp/src/config.rs:93`<br>`crates/roko-acp/src/config_watch.rs:158`<br>`crates/roko-cli/src/config.rs:2812`<br>`crates/roko-cli/src/doctor.rs:220` |
| `ROKO_CONTEXT_LIMIT_K` | `core-config-override` | `clap-env` | `crates/roko-core/src/config/schema.rs:505` |
| `ROKO_CONTROL_PLANE_URL` | `runtime-deploy` | `direct` | `crates/roko-cli/src/worker/mod.rs:50` |
| `ROKO_DEBUG` | `app-specific` | `direct` | `crates/roko-agent/src/claude_cli_agent.rs:366`<br>`crates/roko-agent/src/harness/claude_parser.rs:27` |
| `ROKO_DEMO_CACHE` | `app-specific` | `direct` | `crates/roko-agent/src/file_cache.rs:51` |
| `ROKO_DEPLOYMENT_ID` | `runtime-deploy` | `direct` | `crates/roko-cli/src/worker/mod.rs:51` |
| `ROKO_DISPATCHER` | `app-specific` | `direct` | `crates/roko-agent/src/provider/mod.rs:279` |
| `ROKO_EFFORT` | `core-config-override` | `direct` | `crates/roko-cli/src/main.rs:2905` |
| `ROKO_EXPECT_API_KEY` | `secret/auth` | `direct` | `crates/roko-core/src/config/schema.rs:2061` |
| `ROKO_GIT_HASH` | `build-info` | `build-time` | `crates/roko-cli/build.rs:19` |
| `ROKO_HIGH_CONTRAST` | `ui-accessibility` | `direct` | `crates/roko-cli/src/tui/theme.rs:127` |
| `ROKO_LOG` | `app-specific` | `direct` | `crates/roko-cli/src/main.rs:2293` |
| `ROKO_LOG_FORMAT` | `app-specific` | `direct` | `crates/roko-cli/src/main.rs:2942` |
| `ROKO_LOG_RAW` | `app-specific` | `direct` | `crates/roko-cli/src/main.rs:2086` |
| `ROKO_MAX_AGENTS` | `core-config-override` | `clap-env` | `crates/roko-core/src/config/schema.rs:516` |
| `ROKO_MCP_CONFIG` | `mcp-script` | `direct` | `crates/roko-agent/src/process/mcp.rs:150` |
| `ROKO_MCP_SCRIPTS_DIR` | `mcp-script` | `direct` | `crates/roko-mcp-scripts/src/main.rs:602` |
| `ROKO_MCP_SCRIPTS_ENV_ALLOWLIST` | `mcp-script` | `direct` | `crates/roko-mcp-scripts/src/main.rs:528` |
| `ROKO_MCP_SCRIPTS_TIMEOUT_SECS` | `mcp-script` | `direct` | `crates/roko-mcp-scripts/src/main.rs:522` |
| `ROKO_MIRAGE_URL` | `runtime-deploy` | `direct` | `crates/roko-demo/src/deploy.rs:446`<br>`crates/roko-serve/src/state.rs:1020` |
| `ROKO_MOCK_STATE_PATH` | `app-specific` | `direct` | `crates/roko-agent/src/provider/mod.rs:292` |
| `ROKO_MODEL` | `core-config-override` | `direct` | `crates/roko-acp/src/runner.rs:462`<br>`crates/roko-cli/src/main.rs:2897`<br>`crates/roko-core/src/config/loader.rs:546` |
| `ROKO_PROVIDER` | `core-config-override` | `direct` | `crates/roko-core/src/config/loader.rs:548` |
| `ROKO_QUIET` | `app-specific` | `direct` | `crates/roko-cli/src/main.rs:2929` |
| `ROKO_REDUCED_MOTION` | `ui-accessibility` | `direct` | `crates/roko-cli/src/tui/effects_config.rs:138` |
| `ROKO_RESOLVE_API_KEY_CHILD` | `secret/auth` | `direct` | `crates/roko-core/src/config/schema.rs:2057` |
| `ROKO_ROLE` | `core-config-override` | `direct` | `crates/roko-cli/src/main.rs:2921` |
| `ROKO_RUSTC_VERSION` | `build-info` | `build-time` | `crates/roko-cli/build.rs:34` |
| `ROKO_SCRIPTS_DIR` | `mcp-script` | `direct` | `crates/roko-mcp-scripts/src/main.rs:593` |
| `ROKO_SERVER_AUTH_TOKEN` | `secret/auth` | `direct` | `crates/roko-cli/src/tui/app.rs:3428`<br>`crates/roko-runtime/src/http_event_sink.rs:29`<br>`crates/roko-serve/src/routes/event_ingest.rs:137`<br>`crates/roko-serve/src/terminal.rs:171` |
| `ROKO_SERVER_URL` | `runtime-deploy` | `direct` | `crates/roko-cli/src/tui/app.rs:3420` |
| `ROKO_SERVE_URL` | `runtime-deploy` | `direct` | `crates/roko-cli/src/commands/feed.rs:35`<br>`crates/roko-cli/src/tui/app.rs:3416`<br>`crates/roko-runtime/src/http_event_sink.rs:23` |
| `ROKO_SPA_DIR` | `runtime-deploy` | `direct` | `crates/roko-serve/src/embedded.rs:26` |
| `ROKO_TARGET` | `build-info` | `build-time` | `crates/roko-cli/build.rs:38` |
| `ROKO_TEMPLATE_JSON` | `runtime-deploy` | `direct` | `crates/roko-cli/src/worker/mod.rs:41` |
| `ROKO_TEST_MISSING_KEY_XYZ_DOES_NOT_EXIST` | `test-only` | `clap-env` | `crates/roko-acp/tests/protocol_conformance.rs:489` |
| `ROKO_TEST_OLLAMA` | `test-only` | `direct` | `crates/roko-agent/tests/ollama_tool_loop.rs:28`<br>`crates/roko-cli/tests/ollama_e2e.rs:17` |
| `ROKO_TEST_RPC_URL` | `chain-mirage-isfr` | `direct` | `crates/roko-chain/tests/alloy_live.rs:15` |
| `ROKO_TIMING` | `app-specific` | `direct` | `crates/roko-cli/src/main.rs:2066` |
| `ROKO_VERBOSE` | `app-specific` | `direct` | `crates/roko-cli/src/model_selection.rs:84` |
| `ROKO_VIEWPORT_HEIGHT` | `ui-accessibility` | `direct` | `crates/roko-cli/src/inline/terminal.rs:78` |
| `ROKO_WATCHER_ID` | `app-specific` | `clap-env` | `apps/roko-chain-watcher/src/config.rs:20` |
| `ROKO_WATCHER_QUERY` | `app-specific` | `clap-env` | `apps/roko-chain-watcher/src/config.rs:35` |
| `ROKO_WORKSPACE_ROOT` | `runtime-deploy` | `direct` | `crates/roko-mcp-code/src/lib.rs:197` |
| `RUST_LOG` | `app-specific` | `direct` | `crates/roko-cli/src/daemon/systemd.rs:32`<br>`crates/roko-cli/src/main.rs:2124` |
| `SHELL` | `app-specific` | `direct` | `crates/roko-serve/src/terminal.rs:533` |
| `SKIP_FRONTEND_BUILD` | `test-only` | `direct` | `crates/roko-serve/build.rs:19` |
| `SLACK_BOT_TOKEN` | `secret/auth` | `direct` | `crates/roko-mcp-slack/src/main.rs:645`<br>`crates/roko-serve/src/feedback.rs:763` |
| `SLACK_SIGNING_SECRET` | `secret/auth` | `direct` | `crates/roko-serve/src/routes/webhooks.rs:123` |
| `SLACK_TOKEN` | `secret/auth` | `direct` | `crates/roko-serve/src/feedback.rs:763` |
| `TARGET` | `build-info` | `direct` | `crates/roko-cli/build.rs:37` |
| `TEST_KEY` | `app-specific` | `clap-env` | `crates/roko-core/tests/config_loader_integration.rs:71` |
| `XDG_CONFIG_HOME` | `app-specific` | `direct` | `crates/roko-core/src/config/loader.rs:626` |
| `ZAI_API_KEY` | `secret/auth` | `clap-env` | `crates/roko-agent/tests/provider_integration.rs:538`<br>`crates/roko-cli/src/auth_detect.rs:160`<br>`crates/roko-cli/src/bootstrap.rs:94`<br>`crates/roko-cli/src/chat_inline.rs:3230` |
| `ZAI_MODEL` | `app-specific` | `direct` | `crates/roko-cli/src/auth_detect.rs:162` |

## Dynamic / computed env surfaces (not single fixed vars)

| Surface | Shape | Owner | Notes |
|---|---|---|---|
| `ROKO_SECRET_<CATEGORY>_<PROVIDER>` | Runtime-built prefix (default `ROKO_SECRET_`) | `roko-core/src/secrets/{env.rs:43,namespace.rs:69,resolve.rs:111,214}` | Read-only `EnvVarStore` secret backend. e.g. `ROKO_SECRET_LLM_ANTHROPIC`, `ROKO_SECRET_RPC_ALCHEMY`. Prefix is configurable, so no single fixed var name — excluded from the count. Must be scrubbed like any provider key. |
| `ROKO__SECTION__FIELD` | Arbitrary hierarchical keys | `roko-core/src/config/loader.rs:338-382` | Any `ROKO__*` var maps to a dotted config path (e.g. `ROKO__AGENT__DEFAULT_MODEL`, `ROKO__CONDUCTOR__MAX_AGENTS`, `ROKO__GATES__SKIP_TESTS`). Collected via env scan, not enumerable statically. |

## Follow-Up Rules

- [ ] Every `secret/auth` var needs a scrub test or explicit no-log/no-front-end policy.
- [ ] Every `runtime-deploy` var needs an ops-doc owner in `77-OPERATIONS-DEPLOY-RUNBOOK.md` or `61-CONFIG-ENV-MATRIX.md`.
- [ ] Every `mcp-script` var needs a trust-boundary note in `75-SECURITY-AUTH-SCOPE-MATRIX.md`.
- [ ] Every `test-only` var must stay out of user-facing docs unless it is in a proof-gate appendix.
- [ ] New direct env reads should fail docs CI until this manifest or a generated successor is updated.
