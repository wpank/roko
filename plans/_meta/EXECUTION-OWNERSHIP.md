# Executable-plan ownership ledger

> Canonical CTRL-15 disposition for the 29-plan, 120-task executable baseline sealed in
> `plans/INDEX.md` before the recovered architecture queue was incorporated.
>
> Baseline reconciled at integration base `ebcc3add020af2a3ff2f3041f721839c16463be2`.
> Status is intentionally unchanged: all 120 rows remain `ready` until their own
> implementation or acceptance proof is independently reviewed, merged, and reverified.

## Authority and count boundary

- The sealed import baseline is exactly 29 plans and 120 tasks: P08-P34 contain 115 tasks; `architecture-defi-critical-path` and `e2e-smoke` contain 5.
- This ledger is a 120-row bijection over that baseline: 99 retained implementation/verification owners and 21 zero-write acceptance roll-ups.
- The recovered `architecture-core-queue` is a separate executable plan with 24 retained tasks. It is not silently folded into the 120 rows. Current generated index truth is therefore 30 executable plans and 144 tasks.
- `self-dev-ux` (55) and `self-dev-extras` (11) remain excluded superseded plans; their tasks are not executable and are outside this 120-row ledger.
- A roll-up is executable only after every named owner plan is complete. `ownership` and `superseded_by` are audit metadata; `depends_on_plan` is the scheduler-recognized edge.
- A retained row keeps its original write/public-API owner. A roll-up has `files = []`, role `quick-reviewer`, task-scoped acceptance, and no implementation authority.

## Per-plan disposition

| Plan | Tasks | Retained | Roll-ups |
|---|---:|---:|---:|
| `P08-search-command-fix` | 4 | 4 | 0 |
| `P09-tool-alias-fix` | 3 | 3 | 0 |
| `P10-slash-command-flags` | 5 | 5 | 0 |
| `P11-runner-v2-default` | 5 | 1 | 4 |
| `P12-runner-parallelism` | 5 | 0 | 5 |
| `P13-rate-limit-retry` | 4 | 4 | 0 |
| `P14-gate-rung-fix` | 3 | 0 | 3 |
| `P15-error-recovery-wiring` | 5 | 5 | 0 |
| `P16-safety-contracts` | 5 | 5 | 0 |
| `P17-cli-output-format` | 6 | 6 | 0 |
| `P18-tui-agent-data` | 5 | 0 | 5 |
| `P19-cascade-router-acp` | 6 | 6 | 0 |
| `P20-zero-config` | 5 | 5 | 0 |
| `P21-acp-streaming` | 5 | 5 | 0 |
| `P22-acp-tool-permission` | 5 | 5 | 0 |
| `P23-prd-pipeline-fix` | 6 | 6 | 0 |
| `P24-workspace-paths` | 4 | 4 | 0 |
| `P25-mcp-acp-passthrough` | 4 | 4 | 0 |
| `P26-hdc-similarity-lookup` | 4 | 4 | 0 |
| `P27-provider-error-ux` | 4 | 4 | 0 |
| `P28-image-support` | 5 | 5 | 0 |
| `P29-develop-command-wire` | 3 | 1 | 2 |
| `P30-onboarding-doctor` | 4 | 2 | 2 |
| `P31-note-and-context` | 3 | 3 | 0 |
| `P32-cli-polish` | 2 | 2 | 0 |
| `P33-model-ux` | 1 | 1 | 0 |
| `P34-verification-sweep` | 4 | 4 | 0 |
| `architecture-defi-critical-path` | 3 | 3 | 0 |
| `e2e-smoke` | 2 | 2 | 0 |

**Baseline total:** 29 plans, 120 tasks, 99 retained, 21 roll-ups.

## Exact 120-row disposition

| # | Stable task | Disposition | Canonical implementation owner | Write scope | Outcome |
|---:|---|---|---|---|---|
| 1 | `P08-search-command-fix#T1` | **RETAINED** | `P08-search-command-fix#T1` | `crates/roko-agent/src/perplexity/search.rs` | Rewrite PerplexitySearchClient to use single-query API format |
| 2 | `P08-search-command-fix#T2` | **RETAINED** | `P08-search-command-fix#T2` | `crates/roko-cli/src/commands/research.rs` | Replace date_range with native recency_filter in search command |
| 3 | `P08-search-command-fix#T3` | **RETAINED** | `P08-search-command-fix#T3` | `crates/roko-agent/src/perplexity/search.rs` | Update search tests to match real Perplexity API response format |
| 4 | `P08-search-command-fix#T4` | **RETAINED** | `P08-search-command-fix#T4` | `crates/roko-agent/src/perplexity/types.rs` | Add serde snippet alias to SearchResult.content field |
| 5 | `P09-tool-alias-fix#T1` | **RETAINED** | `P09-tool-alias-fix#T1` | `crates/roko-agent/src/provider/openai_compat.rs` | Resolve Claude aliases to canonical names in parse_allowed_tools_csv |
| 6 | `P09-tool-alias-fix#T2` | **RETAINED** | `P09-tool-alias-fix#T2` | `crates/roko-agent/src/provider/openai_compat.rs` | Add unit tests for alias resolution in tool CSV parser |
| 7 | `P09-tool-alias-fix#T3` | **RETAINED** | `P09-tool-alias-fix#T3` | `crates/roko-agent/src/provider/openai_compat.rs`, `crates/roko-agent/src/provider/gemini.rs`, `crates/roko-agent/src/provider/anthropic_api.rs` | Audit other provider backends for same alias resolution gap |
| 8 | `P10-slash-command-flags#T1` | **RETAINED** | `P10-slash-command-flags#T1` | `crates/roko-acp/src/bridge_events.rs` | Fix /plan-resume to use --resume-plan flag |
| 9 | `P10-slash-command-flags#T2` | **RETAINED** | `P10-slash-command-flags#T2` | `crates/roko-acp/src/bridge_events.rs` | Add --model flag passthrough to /plan-run command |
| 10 | `P10-slash-command-flags#T3` | **RETAINED** | `P10-slash-command-flags#T3` | `crates/roko-acp/src/session.rs` | Register /develop as ACP slash command in session.rs |
| 11 | `P10-slash-command-flags#T4` | **RETAINED** | `P10-slash-command-flags#T4` | `crates/roko-acp/src/bridge_events.rs` | Add /develop dispatch handler to bridge_events.rs |
| 12 | `P10-slash-command-flags#T5` | **RETAINED** | `P10-slash-command-flags#T5` | `crates/roko-acp/src/session.rs` | Add test asserting /plan-resume uses --resume-plan flag |
| 13 | `P11-runner-v2-default#T1` | **ROLL-UP** | `E12-T03` | zero-write | Acceptance roll-up: retire the legacy-runner-v2 feature facade |
| 14 | `P11-runner-v2-default#T2` | **ROLL-UP** | `E01-T01` | zero-write | Acceptance roll-up: canonical runner-v2 default |
| 15 | `P11-runner-v2-default#T3` | **ROLL-UP** | `E12-T03` | zero-write | Acceptance roll-up: remove the legacy-runner-v2 source facade |
| 16 | `P11-runner-v2-default#T4` | **ROLL-UP** | `E12-T03` | zero-write | Acceptance roll-up: compile formerly gated runner tests unconditionally |
| 17 | `P11-runner-v2-default#T5` | **RETAINED** | `P11-runner-v2-default#T5` | `crates/roko-cli/src/commands/plan.rs` | Add TOML validation after plan generate agent completes |
| 18 | `P12-runner-parallelism#T1` | **ROLL-UP** | `SH02-T01 + E01-T05` | zero-write | Acceptance roll-up: enforce effective per-plan concurrency |
| 19 | `P12-runner-parallelism#T2` | **ROLL-UP** | `SH02-T01 + E01-T04 + E01-T05` | zero-write | Acceptance roll-up: attribute concurrent active tasks per plan |
| 20 | `P12-runner-parallelism#T3` | **ROLL-UP** | `SH02-T01 + E01-T04 + E01-T05` | zero-write | Acceptance roll-up: own concurrent agent handles per task |
| 21 | `P12-runner-parallelism#T4` | **ROLL-UP** | `SH04-T01 + SH04-T02` | zero-write | Acceptance roll-up: preserve per-task output attribution |
| 22 | `P12-runner-parallelism#T5` | **ROLL-UP** | `SH02-T01 + E01-T04 + E01-T05` | zero-write | Acceptance roll-up: dispatch independent tasks within effective capacity |
| 23 | `P13-rate-limit-retry#T1` | **RETAINED** | `P13-rate-limit-retry#T1` | `crates/roko-agent/src/openai_compat_backend.rs` | Classify HTTP errors in send_turn (non-streaming path) |
| 24 | `P13-rate-limit-retry#T2` | **RETAINED** | `P13-rate-limit-retry#T2` | `crates/roko-agent/src/openai_compat_backend.rs` | Classify HTTP errors in stream_turn (streaming path) |
| 25 | `P13-rate-limit-retry#T3` | **RETAINED** | `P13-rate-limit-retry#T3` | `crates/roko-agent/src/openai_compat_backend.rs` | Classify HTTP errors in send_turn_streaming path |
| 26 | `P13-rate-limit-retry#T4` | **RETAINED** | `P13-rate-limit-retry#T4` | `crates/roko-agent/src/openai_compat_backend.rs` | Add unit test for classify_http_error helper |
| 27 | `P14-gate-rung-fix#T1` | **ROLL-UP** | `E05-T05 + E05-T07` | zero-write | Acceptance roll-up: execute advanced rungs on the live runner path |
| 28 | `P14-gate-rung-fix#T2` | **ROLL-UP** | `E05-T07` | zero-write | Acceptance roll-up: remove legacy advanced-rung activation logging |
| 29 | `P14-gate-rung-fix#T3` | **ROLL-UP** | `E05-T05` | zero-write | Acceptance roll-up: prove the canonical complex pipeline |
| 30 | `P15-error-recovery-wiring#T1` | **RETAINED** | `P15-error-recovery-wiring#T1` | `crates/roko-cli/src/runner/event_loop.rs` | Wire classify_agent_crash into runner-v2 agent failure handler |
| 31 | `P15-error-recovery-wiring#T2` | **RETAINED** | `P15-error-recovery-wiring#T2` | `crates/roko-cli/src/commands/do_cmd.rs` | Wire classify_agent_crash into do_cmd.rs agent failure paths |
| 32 | `P15-error-recovery-wiring#T3` | **RETAINED** | `P15-error-recovery-wiring#T3` | `crates/roko-cli/src/commands/do_cmd.rs` | Warn on silent roko.toml parse fallback in do_cmd.rs |
| 33 | `P15-error-recovery-wiring#T4` | **RETAINED** | `P15-error-recovery-wiring#T4` | `crates/roko-cli/src/runner/event_loop.rs` | Add crash_class field to runner-v2 task failure ledger entries |
| 34 | `P15-error-recovery-wiring#T5` | **RETAINED** | `P15-error-recovery-wiring#T5` | `crates/roko-cli/src/runner/event_loop.rs` | Enrich agent exit handler and output sink messages with crash diagnosis |
| 35 | `P16-safety-contracts#T1` | **RETAINED** | `P16-safety-contracts#T1` | `crates/roko-cli/src/dispatch_v2.rs` | Add disallowed_tools to CliDispatchRequest and wire into CLI args |
| 36 | `P16-safety-contracts#T2` | **RETAINED** | `P16-safety-contracts#T2` | `crates/roko-cli/src/runner/agent_stream.rs` | Add disallowed_tools to AgentSpawnConfig and propagate to CliDispatchRequest |
| 37 | `P16-safety-contracts#T3` | **RETAINED** | `P16-safety-contracts#T3` | `crates/roko-agent/src/safety/contract.rs` | Add forbidden_tool_names() helper to AgentContract |
| 38 | `P16-safety-contracts#T4` | **RETAINED** | `P16-safety-contracts#T4` | `crates/roko-cli/src/runner/event_loop.rs` | Load AgentContract for task role and set disallowed_tools on spawn config |
| 39 | `P16-safety-contracts#T5` | **RETAINED** | `P16-safety-contracts#T5` | `crates/roko-cli/src/runner/event_loop.rs` | Wire contract forbidden tools into bridge dispatch path |
| 40 | `P17-cli-output-format#T1` | **RETAINED** | `P17-cli-output-format#T1` | `crates/roko-cli/src/cli_output.rs`, `crates/roko-cli/src/lib.rs` | Create CliOutput wrapper struct in cli_output.rs |
| 41 | `P17-cli-output-format#T2` | **RETAINED** | `P17-cli-output-format#T2` | `crates/roko-core/src/config/loader.rs` | Downgrade duplicate model slug warnings to debug level |
| 42 | `P17-cli-output-format#T3` | **RETAINED** | `P17-cli-output-format#T3` | `crates/roko-cli/src/commands/do_cmd.rs` | Replace eprintln! calls in run_simple_path with CliOutput |
| 43 | `P17-cli-output-format#T4` | **RETAINED** | `P17-cli-output-format#T4` | `crates/roko-cli/src/commands/do_cmd.rs` | Replace eprintln! calls in standard and complex paths with CliOutput |
| 44 | `P17-cli-output-format#T5` | **RETAINED** | `P17-cli-output-format#T5` | `crates/roko-cli/src/commands/do_cmd.rs` | Replace plan completion summary and failure details with output_format |
| 45 | `P17-cli-output-format#T6` | **RETAINED** | `P17-cli-output-format#T6` | `crates/roko-cli/src/commands/do_cmd.rs` | Replace remaining eprintln! calls in do_cmd helper functions |
| 46 | `P18-tui-agent-data#T1` | **ROLL-UP** | `SH04-T01` | zero-write | Acceptance roll-up: use structured task attribution |
| 47 | `P18-tui-agent-data#T2` | **ROLL-UP** | `SH04-T03 + SH04-T05` | zero-write | Acceptance roll-up: publish attributed live usage |
| 48 | `P18-tui-agent-data#T3` | **ROLL-UP** | `SH04-T02 + SH04-T03` | zero-write | Acceptance roll-up: preserve typed live agent output |
| 49 | `P18-tui-agent-data#T4` | **ROLL-UP** | `SH04-T04` | zero-write | Acceptance roll-up: publish one diagnosis for failed gates |
| 50 | `P18-tui-agent-data#T5` | **ROLL-UP** | `SH04-T04` | zero-write | Acceptance roll-up: expose diagnoses through the canonical TUI bridge |
| 51 | `P19-cascade-router-acp#T1` | **RETAINED** | `P19-cascade-router-acp#T1` | `crates/roko-acp/src/bridge_events.rs` | Add cascade_select_model() function to bridge_events.rs |
| 52 | `P19-cascade-router-acp#T2` | **RETAINED** | `P19-cascade-router-acp#T2` | `crates/roko-acp/src/bridge_events.rs` | Wire cascade selection into handle_session_prompt_inner |
| 53 | `P19-cascade-router-acp#T3` | **RETAINED** | `P19-cascade-router-acp#T3` | `crates/roko-acp/src/bridge_events.rs` | Fix model key mismatch in cascade observation recording |
| 54 | `P19-cascade-router-acp#T4` | **RETAINED** | `P19-cascade-router-acp#T4` | `crates/roko-acp/src/bridge_events.rs` | Load DaimonState from disk in acp_routing_context |
| 55 | `P19-cascade-router-acp#T5` | **RETAINED** | `P19-cascade-router-acp#T5` | `crates/roko-acp/src/bridge_events.rs` | Record cascade routing decision in ACP episode metadata |
| 56 | `P19-cascade-router-acp#T6` | **RETAINED** | `P19-cascade-router-acp#T6` | `crates/roko-acp/src/bridge_events.rs` | Add tests for cascade selection and key normalization |
| 57 | `P20-zero-config#T1` | **RETAINED** | `P20-zero-config#T1` | `crates/roko-cli/src/commands/util.rs` | Consult builtin registry in preflight_provider_for_model |
| 58 | `P20-zero-config#T2` | **RETAINED** | `P20-zero-config#T2` | `crates/roko-acp/src/session.rs` | Auto-detect builtin models in ACP session config initialization |
| 59 | `P20-zero-config#T3` | **RETAINED** | `P20-zero-config#T3` | `crates/roko-core/src/config/loader.rs` | Suppress false-positive duplicate slug warnings in config loader |
| 60 | `P20-zero-config#T4` | **RETAINED** | `P20-zero-config#T4` | `crates/roko-core/src/agent.rs` | Propagate use_max_completion_tokens from builtin registry |
| 61 | `P20-zero-config#T5` | **RETAINED** | `P20-zero-config#T5` | `crates/roko-core/src/agent.rs` | Add tests for zero-config builtin model resolution |
| 62 | `P21-acp-streaming#T1` | **RETAINED** | `P21-acp-streaming#T1` | `crates/roko-acp/src/bridge_events.rs` | Stream non-progress stdout lines in run_slash_command |
| 63 | `P21-acp-streaming#T2` | **RETAINED** | `P21-acp-streaming#T2` | `crates/roko-acp/src/bridge_events.rs` | Stream stderr lines immediately in run_slash_command |
| 64 | `P21-acp-streaming#T3` | **RETAINED** | `P21-acp-streaming#T3` | `crates/roko-acp/src/bridge_events.rs` | Remove dead output accumulator from run_slash_command |
| 65 | `P21-acp-streaming#T4` | **RETAINED** | `P21-acp-streaming#T4` | `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/commands/do_cmd.rs` | Wire AcpProgressSink as output_sink when ROKO_ACP_PROGRESS is set |
| 66 | `P21-acp-streaming#T5` | **RETAINED** | `P21-acp-streaming#T5` | `crates/roko-acp/src/bridge_events.rs` | Set ROKO_ACP_PROGRESS=1 on subprocess in run_slash_command |
| 67 | `P22-acp-tool-permission#T1` | **RETAINED** | `P22-acp-tool-permission#T1` | `crates/roko-acp/src/bridge_events.rs` | Replace ToolContext::testing() with proper ToolContext in ACP dispatch |
| 68 | `P22-acp-tool-permission#T2` | **RETAINED** | `P22-acp-tool-permission#T2` | `crates/roko-acp/src/bridge_events.rs` | Add denied_tools check to AcpBuiltinToolHandler |
| 69 | `P22-acp-tool-permission#T3` | **RETAINED** | `P22-acp-tool-permission#T3` | `crates/roko-acp/src/builtin_tools.rs` | Add slash_command_allowed_tools function |
| 70 | `P22-acp-tool-permission#T4` | **RETAINED** | `P22-acp-tool-permission#T4` | `crates/roko-acp/src/bridge_events.rs` | Wire slash_command_allowed_tools into ToolContext at dispatch sites |
| 71 | `P22-acp-tool-permission#T5` | **RETAINED** | `P22-acp-tool-permission#T5` | `crates/roko-acp/src/bridge_events.rs` | Add test for tool permission enforcement in AcpBuiltinToolHandler |
| 72 | `P23-prd-pipeline-fix#T1` | **RETAINED** | `P23-prd-pipeline-fix#T1` | `crates/roko-cli/src/commands/prd.rs` | Give prd draft-new agent read-only codebase tools |
| 73 | `P23-prd-pipeline-fix#T2` | **RETAINED** | `P23-prd-pipeline-fix#T2` | `crates/roko-cli/src/commands/prd.rs` | Update draft-new system prompt to instruct tool usage |
| 74 | `P23-prd-pipeline-fix#T3` | **RETAINED** | `P23-prd-pipeline-fix#T3` | `crates/roko-cli/src/commands/prd.rs` | Block draft write when validation detects errors |
| 75 | `P23-prd-pipeline-fix#T4` | **RETAINED** | `P23-prd-pipeline-fix#T4` | `crates/roko-cli/src/prd.rs` | Link plans to PRDs by slug match in cmd_status |
| 76 | `P23-prd-pipeline-fix#T5` | **RETAINED** | `P23-prd-pipeline-fix#T5` | `crates/roko-cli/src/prd.rs` | Infer PRD status from directory path when frontmatter is missing |
| 77 | `P23-prd-pipeline-fix#T6` | **RETAINED** | `P23-prd-pipeline-fix#T6` | `crates/roko-cli/src/task_parser.rs`, `crates/roko-cli/src/prd.rs` | Add source_prd field to TaskMeta and set it in plan generation |
| 78 | `P24-workspace-paths#T1` | **RETAINED** | `P24-workspace-paths#T1` | `crates/roko-cli/src/main.rs` | Align resolve_plans_dir in main.rs to prefer top-level plans/ |
| 79 | `P24-workspace-paths#T2` | **RETAINED** | `P24-workspace-paths#T2` | `crates/roko-cli/src/main.rs` | Update doc strings referencing .roko/plans/ to plans/ |
| 80 | `P24-workspace-paths#T3` | **RETAINED** | `P24-workspace-paths#T3` | `crates/roko-cli/src/doctor.rs` | Add orphaned tmp file detection to roko doctor |
| 81 | `P24-workspace-paths#T4` | **RETAINED** | `P24-workspace-paths#T4` | `crates/roko-cli/src/doctor.rs` | Add plans directory conflict detection to roko doctor |
| 82 | `P25-mcp-acp-passthrough#T1` | **RETAINED** | `P25-mcp-acp-passthrough#T1` | `crates/roko-core/src/config/agent.rs` | Add mcp_config field to roko-core AgentConfig |
| 83 | `P25-mcp-acp-passthrough#T2` | **RETAINED** | `P25-mcp-acp-passthrough#T2` | `crates/roko-acp/src/runner.rs` | Wire MCP config into ACP workflow runner ServiceConfig |
| 84 | `P25-mcp-acp-passthrough#T3` | **RETAINED** | `P25-mcp-acp-passthrough#T3` | `crates/roko-acp/src/session.rs` | Run MCP auto-discovery during ACP session creation |
| 85 | `P25-mcp-acp-passthrough#T4` | **RETAINED** | `P25-mcp-acp-passthrough#T4` | `crates/roko-acp/src/bridge_events.rs` | Wire session MCP config into bridge_events tool-loop dispatch |
| 86 | `P26-hdc-similarity-lookup#T1` | **RETAINED** | `P26-hdc-similarity-lookup#T1` | `crates/roko-learn/src/episode_logger.rs` | Add query_similar_episodes method to EpisodeLogger |
| 87 | `P26-hdc-similarity-lookup#T2` | **RETAINED** | `P26-hdc-similarity-lookup#T2` | `crates/roko-learn/src/episode_logger.rs` | Add test for query_similar_episodes method |
| 88 | `P26-hdc-similarity-lookup#T3` | **RETAINED** | `P26-hdc-similarity-lookup#T3` | `crates/roko-cli/src/orchestrate.rs` | Query similar episodes before dispatch in orchestrate.rs |
| 89 | `P26-hdc-similarity-lookup#T4` | **RETAINED** | `P26-hdc-similarity-lookup#T4` | `crates/roko-cli/src/orchestrate.rs` | Format similar-episode results into a PromptSection |
| 90 | `P27-provider-error-ux#T1` | **RETAINED** | `P27-provider-error-ux#T1` | `crates/roko-cli/src/doctor.rs` | Make doctor API key check conditional on configured providers |
| 91 | `P27-provider-error-ux#T2` | **RETAINED** | `P27-provider-error-ux#T2` | `crates/roko-cli/src/unified.rs` | Make auth failure error message provider-agnostic in unified.rs |
| 92 | `P27-provider-error-ux#T3` | **RETAINED** | `P27-provider-error-ux#T3` | `crates/roko-cli/src/auth_detect.rs` | Make auth_detect setup instructions provider-agnostic |
| 93 | `P27-provider-error-ux#T4` | **RETAINED** | `P27-provider-error-ux#T4` | `crates/roko-cli/src/doctor.rs` | Add detected-providers summary check to doctor output |
| 94 | `P28-image-support#T1` | **RETAINED** | `P28-image-support#T1` | `crates/roko-acp/src/handler.rs` | Set ACP image capability from model vision support |
| 95 | `P28-image-support#T2` | **RETAINED** | `P28-image-support#T2` | `crates/roko-acp/src/bridge_events.rs` | Add image placeholder text in extract_prompt_text |
| 96 | `P28-image-support#T3` | **RETAINED** | `P28-image-support#T3` | `crates/roko-acp/src/bridge_events.rs` | Extract image injection helper and apply in both dispatch paths |
| 97 | `P28-image-support#T4` | **RETAINED** | `P28-image-support#T4` | `crates/roko-agent/src/provider/anthropic_api.rs` | Verify and fix anthropic_api provider image passthrough |
| 98 | `P28-image-support#T5` | **RETAINED** | `P28-image-support#T5` | `crates/roko-core/src/config/model_registry.rs` | Add test asserting all vision-capable builtins have supports_vision |
| 99 | `P29-develop-command-wire#T1` | **ROLL-UP** | `P10-slash-command-flags#T3` | zero-write | Acceptance roll-up: P10 owns develop registration |
| 100 | `P29-develop-command-wire#T2` | **ROLL-UP** | `P10-slash-command-flags#T4` | zero-write | Acceptance roll-up: P10 owns develop dispatch |
| 101 | `P29-develop-command-wire#T3` | **RETAINED** | `P29-develop-command-wire#T3` | `crates/roko-cli/src/commands/develop.rs` | Skip re-generation in develop interactive mode when plan exists |
| 102 | `P30-onboarding-doctor#T1` | **ROLL-UP** | `P27-provider-error-ux#T1` | zero-write | Acceptance roll-up: provider-aware doctor key checks |
| 103 | `P30-onboarding-doctor#T2` | **ROLL-UP** | `P27-provider-error-ux#T1` | zero-write | Acceptance roll-up: configured-provider credential validation |
| 104 | `P30-onboarding-doctor#T3` | **RETAINED** | `P30-onboarding-doctor#T3` | `crates/roko-cli/src/commands/util.rs` | Print next-step hints after roko init succeeds |
| 105 | `P30-onboarding-doctor#T4` | **RETAINED** | `P30-onboarding-doctor#T4` | `crates/roko-cli/src/commands/setup.rs` | Update roko setup next-step hints to include develop and models |
| 106 | `P31-note-and-context#T1` | **RETAINED** | `P31-note-and-context#T1` | `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/commands/plan.rs` | Route unrecognized `roko plan` args to plan generate |
| 107 | `P31-note-and-context#T2` | **RETAINED** | `P31-note-and-context#T2` | `crates/roko-cli/src/note_cluster.rs`, `crates/roko-cli/src/lib.rs` | Create note_cluster module with word-overlap clustering |
| 108 | `P31-note-and-context#T3` | **RETAINED** | `P31-note-and-context#T3` | `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/commands/plan.rs` | Add --from-notes flag to plan generate command |
| 109 | `P32-cli-polish#T1` | **RETAINED** | `P32-cli-polish#T1` | `crates/roko-core/src/config/provider.rs`, `crates/roko-core/src/lib.rs` | Add skip_serializing_if to ModelProfile bool fields |
| 110 | `P32-cli-polish#T2` | **RETAINED** | `P32-cli-polish#T2` | `crates/roko-cli/src/inline/symbols.rs` | Replace PENDING hourglass emoji with circle in symbols.rs |
| 111 | `P33-model-ux#T1` | **RETAINED** | `P33-model-ux#T1` | `crates/roko-agent/src/codex_agent.rs` | Add max_tokens auto-recovery retry in CodexAgent::run |
| 112 | `P34-verification-sweep#T1` | **RETAINED** | `P34-verification-sweep#T1` | `Cargo.toml` | Verify full workspace compiles with cargo check |
| 113 | `P34-verification-sweep#T2` | **RETAINED** | `P34-verification-sweep#T2` | `Cargo.toml` | Run clippy on workspace with zero warnings |
| 114 | `P34-verification-sweep#T3` | **RETAINED** | `P34-verification-sweep#T3` | `Cargo.toml` | Run workspace tests and fix any failures |
| 115 | `P34-verification-sweep#T4` | **RETAINED** | `P34-verification-sweep#T4` | `Cargo.toml` | Build release binary and verify CLI starts |
| 116 | `architecture-defi-critical-path#D01-chain-registry-indexer-foundation` | **RETAINED** | `architecture-defi-critical-path#D01-chain-registry-indexer-foundation` | `crates/roko-chain/src/knowledge_registry.rs`, `crates/roko-chain/src/indexer.rs`, `crates/roko-chain/src/lib.rs` | Implement chain registry client and event indexer foundation |
| 117 | `architecture-defi-critical-path#D02-serve-registry-passport-routes` | **RETAINED** | `architecture-defi-critical-path#D02-serve-registry-passport-routes` | `crates/roko-serve/src/routes/registries.rs`, `crates/roko-serve/src/routes/agents.rs`, `crates/roko-serve/src/routes/mod.rs` | Add serve routes for registries and passport lifecycle |
| 118 | `architecture-defi-critical-path#D03-defi-critical-path-verification` | **RETAINED** | `architecture-defi-critical-path#D03-defi-critical-path-verification` | `crates/roko-chain/src/lib.rs`, `crates/roko-serve/src/routes/mod.rs` | Verify the DeFi critical path integration surface |
| 119 | `e2e-smoke#S01` | **RETAINED** | `e2e-smoke#S01` | `crates/roko-core/src/lib.rs` | Add #[must_use] to generate_share_token in roko-core |
| 120 | `e2e-smoke#S02` | **RETAINED** | `e2e-smoke#S02` | `crates/roko-core/src/lib.rs` | Add unit test for generate_share_token in roko-core |

## Roll-up mapping rationale

- **P11 T1/T3/T4 → E12-T03; P11 T2 → E01-T01.** The current canonical end state removes the obsolete feature facade and compiles its tests unconditionally; restoring the feature as P11 originally requested would contradict E12. E01 owns the actual runner-v2 default.
- **P12 T1-T3/T5 → SH02-T01 plus E01-T04/T05; P12 T4 → SH04-T01/T02.** The canonical scheduler owns effective per-plan capacity and live DAG dispatch, while structured telemetry owns parallel output identity. Container-shape prescriptions are not separate product outcomes.
- **P14 T1-T3 → E05-T05/T07.** P14 targets the dead legacy orchestrator. E05 ports advanced-rung inputs into the live runner, proves the canonical seven-rung pipeline, and deletes the inert legacy toggle.
- **P18 T1-T5 → SH04-T01-T05.** SH04 owns structured identity, typed output/usage, the connected approval TUI, liveness reconciliation, and exactly-once gate diagnosis. Delimiter parsing and ad-hoc bridge helpers cannot remain separate mechanisms.
- **P29 T1/T2 → P10-slash-command-flags#T3 and P10-slash-command-flags#T4.** These are literal duplicate edits to the same ACP registration and dispatch sites. P29-T3 remains the unique develop interactive-regeneration behavior.
- **P30 T1/T2 → P27-provider-error-ux#T1.** Unconditional OpenAI/Gemini warnings conflict with the stronger provider-aware credential check. P27-provider-error-ux#T1 owns one effective-provider iteration; P30 T3/T4 retain unique onboarding output.

## Scheduling and preservation invariants

- Every roll-up remains `ready`; no unexecuted work was marked done or skipped.
- Every roll-up has at least one nonempty `depends_on_plan`; the combined execution graph must resolve all owner plan IDs and remain acyclic.
- Roll-ups have no file or public-API ownership. Their acceptance commands inspect/test the named canonical implementation.
- All 99 retained rows preserve their original IDs, order, status, files, role, context, verification, and task-local dependencies.
- The 24 recovered architecture-core tasks remain retained exactly as authored. Their separate count is visible in the regenerated index instead of being hidden behind the sealed 120-task baseline.
