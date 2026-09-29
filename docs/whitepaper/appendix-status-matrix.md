Status: reviewed · budget none · owner gap-35a614

# Appendix: status matrix

The status of every mechanism in this paper at commit ed0c33bd5 (2026-09-29), each tag re-checked against the code at that commit. The matrix is a dated snapshot: the work graph in work/items stays the live status, and the matrix is refreshed at each review. Sections cite rows by id, such as EX3.

Each row gives the mechanism, its status at the pinned commit, what runs today, the code that implements it, the evidence for the tag and the next step. A code anchor names a file and a symbol in it; `tools/status_matrix.py` fails if either is missing at the pinned commit. Evidence is a commit, a test, or a work item whose recorded state supports the tag. The next step is a verdict and, where one is filed, the work item that would change the tag.

| Status | Meaning |
|---|---|
| WIRED@ed0c33bd5 | Runs on the production path (plan runs through the Graph engine) and does its job: a sensor records, a regulator changes behaviour |
| PARTIAL@ed0c33bd5 | On the path but incomplete, masked, or fed wrong data |
| BROKEN@ed0c33bd5 | Wired, but does the wrong thing |
| ORPHANED@ed0c33bd5 | Worked under Runner-v2, the event loop deleted in `6b5da8616`, and was not re-attached |
| BUILT-UNWIRED@ed0c33bd5 | Code and tests exist, with no production caller |
| MISSING@ed0c33bd5 | Not built |
| REMOVED@ed0c33bd5 | Deliberately deleted |
| UNPROVEN@ed0c33bd5 | Might be true; no measurement exists |

| Verdict | Meaning |
|---|---|
| keep | It works; keep it |
| wire | The code exists; connect it to the production path |
| fix | It is connected but wrong or incomplete; correct it |
| build | Nothing exists yet; build it |
| redesign | Keep the goal, change the mechanism |
| park | Move it behind a feature flag, out of the default build, the core docs and the pitch |
| remove | Delete it, or the claim made for it |

## Summary

Rows per status and group at ed0c33bd5.

| Status | AU | EX | IS | QA | RC | LM | RG | SS | DM | Total |
|---|---|---|---|---|---|---|---|---|---|---|
| WIRED@ed0c33bd5 | 4 | 5 | 1 | 2 | 3 | 1 | 1 | 2 | 4 | 23 |
| PARTIAL@ed0c33bd5 | 2 | 1 | 3 | 2 | 3 | 6 | 1 | 2 | 2 | 22 |
| BROKEN@ed0c33bd5 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 2 |
| ORPHANED@ed0c33bd5 | 0 | 1 | 1 | 1 | 0 | 3 | 1 | 0 | 0 | 7 |
| BUILT-UNWIRED@ed0c33bd5 | 1 | 1 | 0 | 1 | 1 | 1 | 0 | 0 | 2 | 7 |
| MISSING@ed0c33bd5 | 0 | 1 | 1 | 2 | 0 | 0 | 4 | 1 | 0 | 9 |
| REMOVED@ed0c33bd5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 |
| Rows | 7 | 9 | 6 | 8 | 7 | 12 | 7 | 6 | 9 | 71 |

## AU · Authoring and specs

Turning a request into a plan whose tasks a cheap model can execute and a machine can check.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| AU1 | Plan generator | WIRED@ed0c33bd5 | Turns a PRD or prompt into a validated plan; one pipeline serves roko prd plan, the portal and the complex band of roko do, while roko plan generate keeps its own prompt. Since `28db9c789` the authoring config's `planner_model` picks the planner on every generate and revise path but one serve route (bug-8b1bf8). | `crates/roko-cli/src/prd.rs`: `generate_plan_from_prd_with_outcome`, `validate_and_fix_generated_plan`; `crates/roko-cli/src/model_selection.rs`: `resolve_planner_model` | `28db9c789`, gap-853b31, bug-8b1bf8, gap-2623b2 | keep · gap-2623b2 |
| AU2 | Plan validation | WIRED@ed0c33bd5 | About fifty deterministic lint codes check schema, dependency graph, context and budget before a run. Plans started through the server skip them. | `crates/roko-cli/src/plan_validate.rs`: `validate_plans_dir`; `crates/roko-cli/src/plan_policy.rs`: `PlanExecutionPolicy` | gap-655d19 | keep · gap-655d19 |
| AU3 | Spec-quality lint and gate | PARTIAL@ed0c33bd5 | Since `31bc6ba3d`, roko plan validate with its spec-quality flag scores each task against the speclint rules and hard fails, and a hard fail fails validation under the strict flag (gap-46ab3f). It is opt-in: plan generation and plan runs never call it, nothing blocks a weak spec before dispatch, and only the benchmark's Python speclint proves a check red on the base. | `crates/roko-gate/src/spec_quality.rs`: `lint_files`; `crates/roko-cli/src/commands/plan.rs`: `cmd_plan_validate` | `31bc6ba3d`, gap-1cd8d3, gap-46ab3f, test `crates/roko-gate/src/spec_quality.rs`: `spec_quality_matches_speclint_golden_fixtures`, gap-b3fa0a, find-70edcb | wire · spec-e57870 |
| AU4 | Context packs | WIRED@ed0c33bd5 | The files, symbols and line ranges a task declares are rendered into its prompt; a declared file that is missing fails the plan load. | `crates/roko-cli/src/plan_policy.rs`: `render_declared_context` | bug-7c8a57 | keep |
| AU5 | Code index | BUILT-UNWIRED@ed0c33bd5 | The parsed code graph reaches agents only as an optional MCP server. The CLI uses it for gate impact analysis, which plan runs do not execute; it never feeds context packs. | `crates/roko-index/src/lib.rs`; `crates/roko-mcp-code/src/lib.rs` | gap-a247a4 | wire · spec-1ced1d |
| AU6 | Acceptance criteria | PARTIAL@ed0c33bd5 | Criteria are rendered into the task prompt as text. The typed acceptance contract is schema-checked by plan validation, and its evaluator has no production caller. | `crates/roko-gate/src/acceptance_contract.rs`: `AcceptanceContract`, `validate_evidence` | gap-1cd8d3, gap-d14a43 | redesign · spec-e57870 |
| AU7 | Pinned acceptance tests | WIRED@ed0c33bd5 | Since `af51b7a61`, a task's accept table names its planner-written tests. A plan run pins each outside every working tree and puts a verify step first that checks the pinned copy's hash, copies it over the agent's copy and requires an exact passing count. The store is tamper-evident, not tamper-proof, and one tracked plan uses it so far. | `crates/roko-cli/src/task_accept.rs`: `pin_plans`; `crates/roko-cli/src/graph_execution/plan_runner.rs`: `pin_plans`; `plans/portal-programme/08f-final-polish/accept` | `af51b7a61`, gap-d14a43, test `crates/roko-cli/src/task_accept.rs`: `accept_store_rejects_a_changed_source`, gap-ba4d01 | keep · gap-ba4d01 |

## EX · Execution

Running a plan's tasks as a dependency graph, with retries, resume and supervision.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| EX1 | Graph engine | WIRED@ed0c33bd5 | Runs every plan as a dependency graph of cells. It is the only plan executor: it became the default in `bcd1c8624`, and the Runner-v2 event loop was deleted in `6b5da8616`. | `crates/roko-graph/src/engine.rs`: `GraphEngine` | `bcd1c8624`, `6b5da8616` | keep |
| EX2 | Durable checkpoints and resume | WIRED@ed0c33bd5 | Each plan's checkpoint is replayed on resume, and a fingerprint of the authored plan rejects a changed plan. Failed, blocked and unstarted tasks are recorded and re-run on resume since `abc1f4b27`. | `crates/roko-cli/src/graph_checkpoint.rs`: `prepare_graph_checkpoint` | `abc1f4b27`, gap-7147bb | keep |
| EX3 | Parallel task scheduling | PARTIAL@ed0c33bd5 | A ready queue starts each task once its own dependencies settle, and a failed task skips only its dependants (the default failure policy since `abc1f4b27`). Since `1697fea53` tasks whose declared files overlap never run at the same time (gap-439794). But `max_parallel` still defaults to 1. | `crates/roko-graph/src/engine.rs`: `execute_ready_queue`; `crates/roko-graph/src/exclusion.rs`: `first_overlap`; `crates/roko-cli/src/task_parser.rs`: `default_max_parallel` | `445a60d0d`, `3e7552acd`, `abc1f4b27`, test `crates/roko-graph/src/engine.rs`: `a_failed_node_blocks_only_its_dependants`, `1697fea53`, gap-439794, test `crates/roko-graph/src/convert.rs`: `task_files_become_exclusive_paths` | fix · gap-272448 |
| EX4 | Plan-set scheduler | WIRED@ed0c33bd5 | Runs several plans at once, admitting only plans whose file footprints don't overlap. The default is still one plan at a time. | `crates/roko-cli/src/graph_execution/plan_set.rs`: `PlanSetScheduler` | `725f21e05`, gap-7c9e48 | keep · gap-7c9e48 |
| EX5 | Sibling settle | WIRED@ed0c33bd5 | In the shared checkout, a verify step that failed while a sibling task was mid-edit waits for the sibling and re-runs once. It is a narrow mitigation until isolation lands. | `crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs`: `InFlightTasks` | `3049b7fcf` | keep |
| EX6 | Retry with gate feedback | WIRED@ed0c33bd5 | A failed task is re-dispatched with its parsed gate output, up to `max_retries` (3 by default). Since `ce3bdcbb8` the retry feedback survives a resume and the failing step names its rung. | `crates/roko-cli/src/task_parser.rs`: `default_max_retries`; `crates/roko-graph/src/cells/task_executor.rs`: `TaskExecutorCell` | `ce3bdcbb8`, `41c7ffbd6` | keep |
| EX7 | Escalation to a stronger model | ORPHANED@ed0c33bd5 | Runner-v2 moved a failing task up a model ladder. On the Graph path a retry reruns the same model, and the ladder constant and the complexity-band escalation have no caller. | `crates/roko-core/src/defaults.rs`: `MODEL_ESCALATION_LADDER`; `crates/roko-core/src/task.rs`: `TaskComplexityBand` | `6b5da8616`, gap-b62e95 | wire · gap-460230 |
| EX8 | Split or replan on failure | BUILT-UNWIRED@ed0c33bd5 | The replan controller has no caller, and the per-task split and replan fields are parsed and ignored. Gate-failure replanning was deleted with Runner-v2. | `crates/roko-execution/src/replan_controller.rs`: `ReplanController` | `6b5da8616` | wire · gap-3b170b |
| EX9 | Progress watchdog | MISSING@ed0c33bd5 | Only a wall-clock timeout per attempt and per-tier turn caps, enforced beyond the Claude CLI since `3e35aca4a`, stop a hung agent; nothing detects an agent that has stopped making progress. | none | spec-a0403b, spec-edda86, gap-a791b4 | build · spec-a0403b |

## IS · Isolation and integration

Keeping concurrent agents out of each other's way and out of the operator's secrets, then combining their results.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| IS1 | Worktrees | PARTIAL@ed0c33bd5 | Tasks edit the operator's checkout by default. The opt-in per-task worktree forks from HEAD, and its successful edits are never merged back. | `crates/roko-cli/src/graph_execution/plan_runner.rs`: `worktree_per_task`; `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `WorktreeManager` | gap-4ec59f | redesign · gap-4ec59f |
| IS2 | Merge queue, plan merger and attempt acceptance | ORPHANED@ed0c33bd5 | Runner-v2 integrated task results through these. On the Graph path nothing merges or commits a result, and `accept_attempt` has no production caller. | `crates/roko-cli/src/orchestrator/merge_queue.rs`: `MergeQueue`; `crates/roko-cli/src/runner/merge.rs`: `PlanMerger`; `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `accept_attempt` | `6b5da8616`, gap-3b5361, bug-a3760a | wire · gap-3b5361 |
| IS3 | Whole-plan verify on the integrated result | MISSING@ed0c33bd5 | Verify steps are per task. No plan-level gate checks that the parts work together. | none | gap-60233f, gap-af00b1 | build · gap-60233f |
| IS4 | Git guard hook | WIRED@ed0c33bd5 | Since `0728a2817` and `abc655b5e`, the Claude CLI hook checks every command in a chain, past wrappers, subshells and git aliases, and blocks git checkout, switch, restore and push, hard resets, stash, clean and recursive deletes; it also blocks when python3 is missing. It is best effort: a script file can hide a command, a few wrapper forms still pass (bug-0bc728), and other provider CLIs have no hook. | `crates/roko-agent/src/claude_cli_agent.rs`: `build_settings_json`; `crates/roko-agent/src/claude_cli_guard.py` | `0728a2817`, `abc655b5e`, bug-7de5df, bug-66f5a1, bug-f4e133, test `crates/roko-agent/src/claude_cli_agent.rs`: `settings_hook_fails_closed_without_python3`, bug-0bc728 | fix · bug-0bc728 |
| IS5 | Environment isolation | PARTIAL@ed0c33bd5 | Since `dc99a9e81` (bug-7d7200), gates start from an allowlisted environment, and provider CLIs have the key variables they recognise scrubbed, so a key under an unrecognised name still reaches the agent. Since `0728a2817` and `abc655b5e`, Claude CLI agents and roko-std's file tools refuse the provider key files, but roko-std's bash tool can still read them (bug-62e7e6), and agent tool shells, MCP servers and provider probes inherit everything. | `crates/roko-core/src/child_env.rs`: `gate_env`, `is_key_file`; `crates/roko-gate/src/gate_env.rs`; `crates/roko-agent/src/process/env.rs`: `apply_credential_scrub` | `dc99a9e81`, bug-7d7200, test `crates/roko-core/src/child_env.rs`: `gate_env_drops_secret_looking_names_even_under_allowed_prefixes`, `0728a2817`, bug-a66941, bug-63327d, test `crates/roko-agent/src/safety/path.rs`: `path_policy_denies_provider_key_files`, bug-62e7e6, bug-0d9ac4, bug-0eb8e2 | fix · spec-ba7bea |
| IS6 | Sandbox levels | PARTIAL@ed0c33bd5 | A sandbox-level setting and resource limits exist, but no OS sandbox. The pre- and post-dispatch safety checks run in the ACP server, not on plan runs. | `crates/roko-agent/src/safety/sandbox.rs`: `SandboxLevel`; `crates/roko-agent/src/safety/mod.rs`: `pre_dispatch_check` | bug-7debad, gap-8f8544 | fix · gap-8f8544 |

## QA · Verification and QA

Deciding whether a task is done, and whether that decision can be trusted.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| QA1 | Task verify commands | WIRED@ed0c33bd5 | After the agent finishes, the task's own shell checks run fail-fast in its working directory. Each outcome carries a typed verdict (since `725f21e05`), and learning is fed only after the verify steps settle. | `crates/roko-cli/src/graph_task_dispatch/verification.rs`: `settle_task_verification`; `crates/roko-gate/src/shell.rs`: `ShellGate` | `725f21e05` | keep |
| QA2 | Typed verdicts | PARTIAL@ed0c33bd5 | Passed, Unverified and ForcedAccept are stamped on every outcome and checkpointed, and since `3383bd8c0` a plan succeeds only when every task passed its verify steps (gap-29a84b). But learning receives a bool in which Unverified counts as a pass (bug-c34782), and the dashboard counts unverified and skipped tasks as passed. | `crates/roko-graph/src/cells/task_executor.rs`: `TaskGateVerdict`; `crates/roko-cli/src/graph_execution/plan_runner.rs`: `TaskVerdictCounts` | `725f21e05`, `3383bd8c0`, gap-29a84b, bug-7eb27e, test `crates/roko-cli/src/graph_execution/plan_runner.rs`: `plan_outcome_follows_task_verdicts`, bug-c34782, bug-7e1b6b | fix · spec-e9d7ec |
| QA3 | Seven-rung gate pipeline | ORPHANED@ed0c33bd5 | Compile, lint, test, symbol, generated-test, property-test and integration rungs exist, but plan tasks run only their authored verify steps; the pipeline's entry point is reachable only from tests. | `crates/roko-cli/src/runner/gate_dispatch.rs`: `run_gate_once`, `spawn_gate` | `6b5da8616`, gap-85f102 | redesign · spec-6ac537 |
| QA4 | Tamper and scope diff | MISSING@ed0c33bd5 | Nothing flags an attempt that edits tests, verify scripts or gate config, or that edits files outside its task. Only acceptance tests pinned by the harness (AU7) run unchanged whatever the agent edits. | none | spec-9230a9, gap-d14a43 | build · gap-abbd22 |
| QA5 | Hidden tests and random audits | MISSING@ed0c33bd5 | No tests are held back from the agent, and no sample of accepted passes is re-checked, so the false-green rate is not estimated. | none | spec-6ac537, gap-89f393 | build · spec-6ac537 |
| QA6 | LLM judge and reviewer verdicts | BUILT-UNWIRED@ed0c33bd5 | The judge rung has no production oracle and is always skipped. Reviewer BLOCK and REVISE verdicts do not gate Graph tasks. | `crates/roko-gate/src/llm_judge_gate.rs`: `JudgeOracle`; `crates/roko-gate/src/review_verdict.rs`: `parse_structured_review_verdict` | gap-85f102, gap-f4b935 | wire · gap-f4b935 |
| QA7 | Adaptive gate thresholds | WIRED@ed0c33bd5 | Per-rung pass rates are updated after every verify and, since `99adacd6d`, bound the retry budget of tasks without an authored `max_retries`. Concurrent runs can lose updates to the threshold file (bug-e0f472). | `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs`: `suggested_max_retries`; `crates/roko-cli/src/runner/persist.rs`: `suggested_max_retries` | `99adacd6d`, `41c7ffbd6`, `ce3bdcbb8`, bug-e0f472 | keep |
| QA8 | Gaming detector and holdout | PARTIAL@ed0c33bd5 | The gaming detector has no audited quality labels and no actuator, and the holdout split only logs. Both wait for audits to supply ground truth. | `crates/roko-learn/src/gate_gaming.rs`: `GateGamingDetector`; `crates/roko-learn/src/holdout.rs`: `HoldoutExperiment` | find-4b4344 | park · find-4b4344 |

## RC · Routing and cost

Choosing a model for each attempt and accounting for what it costs.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| RC1 | Provider adapters | WIRED@ed0c33bd5 | Twelve provider kinds: the Claude, Codex, Cursor and Gemini CLIs, API adapters and OpenAI-compatible endpoints for cheap models. The Hermes kind carries open parser bugs. | `crates/roko-core/src/agent.rs`: `ProviderKind` | bug-7567eb, bug-b14145 | keep |
| RC2 | Learned model router | PARTIAL@ed0c33bd5 | Picks a model only when neither the task nor the run pins one. Since `91cfe0467` router-chosen failures update its learned stage as successes do (bug-8da8ba), and since `b11ca807d` concurrent processes merge their learning instead of overwriting it. But it learns from the provider call's success flag before gates run, and manual overrides count as successes. | `crates/roko-learn/src/cascade_router.rs`: `CascadeRouter`; `crates/roko-cli/src/dispatch/model_routing.rs`: `ModelRouter` | `91cfe0467`, bug-8da8ba, `b11ca807d`, bug-9c88ac, test `crates/roko-cli/src/runtime_feedback/routing.rs`: `routing_sink_updates_linucb_on_failure`, bug-c34782, bug-f68404 | fix · bug-c34782 |
| RC3 | Provider-health circuit breaker | WIRED@ed0c33bd5 | Routes away from providers whose recent calls failed, a closed loop. Attempt timeouts and turn-cap stops also count as provider failures (bug-7cdce7). | `crates/roko-learn/src/cascade_router.rs`: `route_with_health_scored` | bug-7cdce7 | keep |
| RC4 | Provider failover | PARTIAL@ed0c33bd5 | On a refusal or an exhausted quota the call moves to other keys and fallback models, sideways rather than up. The records name the model that ran but never mark it as a substitute, so the router credits it as its own pick. | `crates/roko-cli/src/graph_task_dispatch/failover.rs`: `run_bridge_with_failover`, `failover_candidates` | bug-35379d | fix · bug-35379d |
| RC5 | Budgets | WIRED@ed0c33bd5 | A plan ceiling with a per-call reservation, and a per-task retry ceiling; since `abc1f4b27` no new task starts once settled spend reaches the ceiling. Daily and agent-lifetime limits are not enforced. | `crates/roko-cli/src/graph_task_dispatch/budget.rs`: `GraphPlanBudgetPolicy`, `GraphTaskSpendLedger` | `abc1f4b27`, bug-ae28ac, gap-34b2ed | keep |
| RC6 | Cost accounting | PARTIAL@ed0c33bd5 | Since `d4be4e872`, timed-out and failed attempts are priced from the usage they streamed and marked as estimates (bug-690dc6), and since `98ee1418f` plan generation and revision spend is recorded. Cost rows still carry no source, the attempt record names its usage source but no amount, and prices come from undated tables. | `crates/roko-core/src/usage.rs`: `UsageSource`; `crates/roko-cli/src/dispatch_v2.rs`: `fill_cost_from_profile` | `d4be4e872`, bug-690dc6, `98ee1418f`, gap-288e38, gap-ad0d39 | fix · gap-288e38 |
| RC7 | Inference gateway | BUILT-UNWIRED@ed0c33bd5 | Caching, batching, fallback and cost accounting run only under roko serve; no plan-run path depends on the gateway crate. It stays serve-only until an API-model arm needs caching. | `crates/roko-gateway/src/gateway.rs`: `InferenceGateway` | gap-cdf3fc | keep |

## LM · Learning and memory

Records of past attempts, and the loops that feed them back into prompts and routing.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| LM1 | Episode and efficiency records | PARTIAL@ed0c33bd5 | Since `42349d8ee` each Graph attempt opens under a durable key and settles once, into a verdict record that names the model that ran; its efficiency, cost and episode rows carry the key (gap-96f7ed). The record's tokens, cost and first-token time stay empty, as do many efficiency fields, and tool events are counted as tool calls. | `crates/roko-cli/src/graph_task_dispatch/attempt.rs`: `open_attempt`; `crates/roko-cli/src/runtime_feedback/episodes.rs`; `crates/roko-learn/src/efficiency.rs`: `AgentEfficiencyEvent` | `e39b5519a`, `d4be4e872`, `42349d8ee`, gap-528762, gap-96f7ed, gap-7a8474, bug-f9ae3e | fix · spec-b7303f |
| LM2 | Playbooks | WIRED@ed0c33bd5 | Since `763596768` outcomes are credited to the playbooks a prompt actually used (reg-3f5969), and selection adds each playbook's net successes (successes minus failures) to its relevance. A floor still puts three playbooks into every prompt, and nothing measures whether they help. | `crates/roko-cli/src/dispatch/prompt_builder.rs`: `collect_playbooks_cached` | `763596768`, reg-3f5969 | fix · spec-6ac537 |
| LM3 | Knowledge store | BROKEN@ed0c33bd5 | Since `189a14e65`, gate-verified attempts grow and reinforce the store (reg-06ae9f). But the prompt cache queries it with an empty topic, which matches nothing, so plan-run prompts carry no knowledge (bug-86117a). | `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs`: `VerifiedKnowledgeSink`; `crates/roko-cli/src/dispatch/prompt_cache.rs`: `PromptCache` | `189a14e65`, reg-06ae9f, bug-86117a | fix · bug-86117a |
| LM4 | Error patterns and post-gate lessons | ORPHANED@ed0c33bd5 | Runner-v2 fed discovered error patterns and retry lessons back into prompts. On the Graph path the pattern reader loads a file that nothing writes, and each gate failure pays for a lesson that no retry prompt reads. | `crates/roko-learn/src/error_pattern_store.rs`: `ErrorPatternStore`; `crates/roko-learn/src/post_gate_reflection.rs`: `PostGateReflectionStore` | `6b5da8616`, gap-e483e7 | wire · spec-6ac537 |
| LM5 | Prompt experiments | PARTIAL@ed0c33bd5 | Since `ebf274ada`, prompt variants are assigned per attempt, bound to the hash of the final prompt and settled with the attempt's outcome, on both dispatch paths (gap-fdd27f). But arms are assigned adaptively (UCB1) and the winner is declared by a chi-squared test, whose error rate is not controlled under adaptive assignment. | `crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs`: `LaunchedTreatments`; `crates/roko-learn/src/prompt_experiment.rs`: `ExperimentStore` | `ebf274ada`, gap-fdd27f | fix · spec-6ac537 |
| LM6 | Section selection by bandit and auction | PARTIAL@ed0c33bd5 | Prompt composition reads per-section posteriors and bids, but nothing on the plan path records section outcomes, so the posteriors never move. | `crates/roko-learn/src/section_outcome.rs`: `SectionOutcomeStore`; `crates/roko-runtime/src/heartbeat_attention.rs`: `ContextBidder` | `6b5da8616` | redesign · spec-6ac537 |
| LM7 | Offline consolidation | ORPHANED@ed0c33bd5 | Runner-v2 ran a paid consolidation job after plans. The Graph path registers a plan-completion sink for it, but no production code emits the plan-completed event, so plan runs never start it; since `d5b17759b` the sink is also off by default (bug-470de8). | `crates/roko-cli/src/runtime_feedback/plan_completion.rs`: `DreamConsolidationSink` | q-6b7cca, `d5b17759b`, bug-470de8, bug-b16d55, bug-b9be1d | park · q-6b7cca |
| LM8 | Affect state | PARTIAL@ed0c33bd5 | An affect state is appraised after every task and shifts the routing tier, with no record of its influence. The sink that saves it fires only on the plan-completed event, so each run's updates are lost. | `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`: `build_routing_context`; `crates/roko-cli/src/runtime_feedback/plan_completion.rs` | q-6b7cca | park · q-6b7cca |
| LM9 | Reflex shortcut | ORPHANED@ed0c33bd5 | Nothing on the plan path promotes reflex rules. Since `328123077` plan dispatch consults them only for tasks without verify steps and only when `t0_reflexes` is on, which it is not by default, and a rule hit credits no gate pass. | `crates/roko-learn/src/reflex_store.rs`: `ReflexStore`; `crates/roko-cli/src/graph_task_dispatch.rs`: `with_reflex_store` | `328123077`, bug-94151f, test `crates/roko-cli/src/graph_task_dispatch.rs`: `reflex_match_records_no_gate_pass_before_verify`, gap-4468bd | remove |
| LM10 | Similarity and collective-intelligence metrics | BUILT-UNWIRED@ed0c33bd5 | Episode fingerprints are hash-expanded vectors that nothing reads; the active-inference router has no caller; the collective-intelligence prompt block reads a file that plan runs never write; compounding metrics are never computed. | `crates/roko-cli/src/runtime_feedback/episodes.rs`: `attach_episode_hdc_fingerprint`; `crates/roko-learn/src/active_inference.rs`: `EfeRouter`; `crates/roko-cli/src/dispatch/prompt_builder.rs`: `generate_cfactor_context` | gap-14f08e | park |
| LM11 | Second learning pipeline | PARTIAL@ed0c33bd5 | A separate learning runtime settles roko do, roko prd and plan-authoring runs, not plan runs, and updates router and section state on its own. | `crates/roko-learn/src/runtime_feedback/mod.rs`: `LearningRuntime`; `crates/roko-cli/src/agent_exec.rs`: `record_completed_run` | spec-b7303f | redesign · spec-b7303f |
| LM12 | Hindsight relabelling | PARTIAL@ed0c33bd5 | Since `33e107da1`, a verify failure that names a sibling task relabels that sibling's latest success as a failure. The corrections go to an adjustments file that nothing reads, so no learner sees them. | `crates/roko-cli/src/runtime_feedback/hindsight.rs`: `HindsightSink`; `crates/roko-cli/src/graph_execution/plan_runner.rs`: `HindsightSink` | `fb87e3738`, `33e107da1`, gap-5fb9a7, gap-5be28d | fix · gap-5be28d |

## RG · Regulation and audits

The second-order loops that watch the first-order ones.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| RG1 | Telemetry lenses and state hub | WIRED@ed0c33bd5 | Sensors: per-task model, cost, tool steps and gate output flow to the TUI and the HTTP event streams, with run completion and agent heartbeats. The plan event stream still lacks task timestamps, dependency edges and the gate rung. | `crates/roko-runtime/src/state_hub.rs`: `StateHub`; `crates/roko-cli/src/runner/graph_tui_bridge.rs`: `StateHubTelemetrySink`; `crates/roko-runtime/src/telemetry_projection_aggregator.rs`: `LensPayload` | `5c62bf0d4`, gap-8a1fb3 | keep |
| RG2 | Conductor watchers and circuit breaker | ORPHANED@ed0c33bd5 | The watcher engine has no caller on the plan path since Runner-v2's supervisor tick was deleted; that tick had caused restart storms. The ultrastable controller (RG3) is meant to replace it. | `crates/roko-conductor/src/conductor.rs`: `Conductor`; `crates/roko-cli/src/runner/conductor_adapter.rs`: `ConductorRingSink` | `6b5da8616`, gap-ebd656 | park · gap-ebd656 |
| RG3 | Ultrastable controller (M1) | MISSING@ed0c33bd5 | Nothing changes a harness parameter when an essential variable, such as the verified pass rate or the cost per verified task, leaves its bounds. | none | spec-6ac537 | build · spec-6ac537 |
| RG4 | Loop-liveness audit (M2) | MISSING@ed0c33bd5 | No learning loop shows its exposure, influence and benefit: there is no decision log, no withhold arm and no way to freeze learning for a comparison. The keyed assignment such an arm would use exists since `1c5371f9a`, with no caller. | none | gap-1f2661, gap-644040, gap-528762 | build · spec-6ac537 |
| RG5 | Calibrated self-model (M3) | MISSING@ed0c33bd5 | Nothing forecasts a task's chance of passing on a given model, so routing and escalation cannot be judged against a forecast. | none | spec-6ac537 | build · spec-6ac537 |
| RG6 | Guarded commit with rollback | MISSING@ed0c33bd5 | No self-modification, whether to the router, memory or a controller, is checked on held-out work before it is accepted, and none can be rolled back. | none | gap-644040 | build · spec-6ac537 |
| RG7 | Cross-cut functors | PARTIAL@ed0c33bd5 | Only a routing-bias arbitration runs on the plan path, turning the affect state and stale consolidation advice into a prefer-cheaper routing bias whose effect is not recorded. | `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`: `arbitrate_cross_cut_routing_bias` | `725f21e05` | park |

## SS · Safety, steering and surfaces

Who may do what, and how an operator watches and steers a run.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| SS1 | Auth, roles and API keys on the control plane | WIRED@ed0c33bd5 | Role-based permissions and API keys protect roko serve. Since `9dc966af1` a Privy sign-in grants nothing without an explicit filter (bug-7eef96), and since `c651ddc57` serve keeps the API keys the CLI creates. The runtime socket is protected by file permissions alone. | `crates/roko-serve/src/rbac.rs`: `Role`, `Permission` | `9dc966af1`, bug-7eef96, `c651ddc57`, bug-da5b41, find-95ef81 | fix · find-95ef81 |
| SS2 | Tool policy | PARTIAL@ed0c33bd5 | A role's allowed and forbidden tools become Claude CLI flags, so Claude enforces tool names. Count-based contract rules never fire, and the Codex policy is advisory. | `crates/roko-agent/src/safety/contract.rs`: `forbidden_tool_names` | bug-1948c9, gap-baab0a | fix · bug-1948c9 |
| SS3 | Output screening and operator deference | PARTIAL@ed0c33bd5 | Outputs and memory writes are screened before they are stored, but nothing measures how often the screen wrongly denies, and the quarantine routes read a file the runtime never writes. | `crates/roko-core/src/immune.rs`: `ImmunePipeline`; `crates/roko-core/src/corrigibility.rs`: `CorrigibilityHead` | bug-4cb029, gap-2f69e9 | fix · gap-f75dc8 |
| SS4 | TUI, portal, HTTP API and event streams | WIRED@ed0c33bd5 | Operators watch each task's model, cost, gate output and transcript live. Cancel works from the TUI, REST, the portal and Ctrl-C, but lands only between nodes. | `crates/roko-serve/src/routes/plans.rs`: `cancel_plan`; `apps/portal/src` | `d9efb9338`, gap-b367bf | keep |
| SS5 | Pause, resume, retry, skip and approve | BROKEN@ed0c33bd5 | TUI pause sets a flag nothing reads while the status bar shows paused; the CLI controls write a file nothing reads; TUI retry and skip are acknowledged and dropped. REST pause cancels and later restarts from the checkpoint. | `crates/roko-graph/src/cell.rs`: `is_paused`; `crates/roko-cli/src/runner/types.rs`: `ControlCommand` | bug-8208a6, gap-c002bb, gap-1555ac | fix · bug-8208a6 |
| SS6 | Per-task diff, approve-before-merge and budget alarms | MISSING@ed0c33bd5 | There is no approval hold before integration and no budget threshold event. The task-diff route looks for branch names that Graph runs never create, so it finds nothing. | `crates/roko-serve/src/routes/plans.rs`: `find_agent_branch`; `crates/roko-cli/src/graph_execution/control_adapter.rs`: `GraphExecutionControlAdapter` | code read at `ed0c33bd5` | build · gap-0d64d5 |

## DM · Domains and automation

Work beyond code, and work started by events rather than by a person.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| DM1 | Shell-command verifiers | WIRED@ed0c33bd5 | Any command can be a gate, as a custom rung in roko.toml or a task's verify step. It is the only verifier not tied to code, and roko run refuses to start without one in a workspace it cannot build. | `crates/roko-core/src/config/gates.rs`: `effective_rungs`; `crates/roko-cli/src/runner/gate_dispatch.rs`: `has_custom_rungs` | bug-1410e8 | keep |
| DM2 | Task domain label | PARTIAL@ed0c33bd5 | A task can be labelled code, research or docs. Routing reads the label; gate selection does not. | `crates/roko-core/src/task.rs`: `TaskDomain`; `crates/roko-cli/src/dispatch/model_routing.rs`: `task_domain` | code read at `ed0c33bd5` | wire · gap-7a3527 |
| DM3 | Triggers | WIRED@ed0c33bd5 | Cron, webhook and file-watch events start a graph through roko serve. The graph cell registry has no agent or shell cell, so a trigger cannot start agent work. | `crates/roko-serve/src/trigger_runtime.rs`: `ensure_trigger_runtime`; `crates/roko-graph/src/engine.rs`: `default_registry` | code read at `ed0c33bd5` | wire |
| DM4 | Feeds and recipes | PARTIAL@ed0c33bd5 | Since `310860465`, roko serve registers three built-in feeds at startup, but they only publish events: no plan or agent consumes them, and recipes only compute scores. | `crates/roko-serve/src/state.rs`: `EpisodeOutcomeFeed`; `crates/roko-core/src/feeds/provider_health.rs` | `310860465` | park |
| DM5 | MCP client and built-in tools | WIRED@ed0c33bd5 | Every dispatch gets the built-in tools and the configured MCP servers. MCP servers still inherit the full environment, provider keys included. | `crates/roko-agent/src/mcp/client.rs`: `McpClient` | bug-0eb8e2 | keep |
| DM6 | Plugin install | WIRED@ed0c33bd5 | Signed install, admission and dependency resolution for plugin manifests work. | `crates/roko-plugin/src/lib.rs`: `PluginManifest` | code read at `ed0c33bd5` | keep |
| DM7 | WASM plugin hooks | REMOVED@ed0c33bd5 | The hook runtime was deleted with Runner-v2, and loading a WASM extension returns an error. Install still checks a hook's interface; since `cac54e574` the docs say that no hook runs (gap-cdf3fc). | `crates/roko-cli/src/runner/extension_loader.rs`: `WasmExtension` | `6b5da8616`, bug-0e13d0, `cac54e574`, gap-cdf3fc | remove |
| DM8 | Messaging channel adapters | BUILT-UNWIRED@ed0c33bd5 | Adapters for six messaging platforms and a channel router exist, but they are not declared as modules, so they are not compiled; the Telegram adapter is a stub. | `crates/roko-runtime/src/channel_binding.rs`: `ChannelBindingRouter`; `crates/roko-runtime/src/adapters/telegram.rs` | gap-8d80d3, spec-743a7e | wire · gap-8d80d3 |
| DM9 | Agent groups, coordination markers and relay | BUILT-UNWIRED@ed0c33bd5 | Group membership, coordination markers and cross-machine relay have storage and REST routes, but plan runs neither join groups nor leave markers; they belong to a separate multi-user product. | `crates/roko-core/src/groups.rs`: `PheromoneDeposit`; `crates/roko-serve/src/group_runtime.rs`; `crates/roko-runtime/src/connector_runtime.rs`: `HttpJsonConnector` | code read at `ed0c33bd5` | park |

## Vision claims

The paper's ten vision claims, each tagged at the same commit and traced to the rows it rests on.

| Claim | Status | Rests on | Main blocker |
|---|---|---|---|
| V1 Domain-agnostic orchestration | PARTIAL@ed0c33bd5 | DM1, DM2, DM3, DM4, QA6 | Shell commands are the only verifier for work that is not code, and triggers cannot start agent work. |
| V2 A fast authoring loop with a frontier planner | PARTIAL@ed0c33bd5 | AU1, AU2, AU3 | The spec-quality lint is opt-in and blocks nothing before dispatch, and there is no plan diff. |
| V3 Granular tasks executed by the cheapest capable models | MISSING@ed0c33bd5 | AU1, EX7, RC2 | Nothing routes by task difficulty, and generated plans are coarse and lose their model hints. |
| V4 Parallel execution of the task graph | PARTIAL@ed0c33bd5 | EX3, EX4, EX5 | The ready queue runs independent tasks and keeps tasks with overlapping files apart, but `max_parallel` defaults to 1. |
| V5 Safe isolation and correct integration | PARTIAL@ed0c33bd5 | IS1, IS2, IS3, IS4, IS5, IS6 | The shared checkout is the default, nothing merges, no whole-plan check exists, and agent tool shells still see provider keys. |
| V6 Trust from gates and acceptance criteria | PARTIAL@ed0c33bd5 | QA1, QA2, QA3, QA4, QA5, QA6, AU6, AU7 | Acceptance criteria are prompt text, only pinned acceptance tests resist tampering, and there are no hidden, scope or whole-plan checks. |
| V7 Cheaper and faster than top models | UNPROVEN@ed0c33bd5 | RC2, RC6, EX7 | No head-to-head run exists, escalation is not wired, and cost rows carry no source. |
| V8 Improves over time | PARTIAL@ed0c33bd5 | LM1, LM2, LM3, LM4, LM5, LM6, LM11, LM12, RC2 | Several loops were re-wired on 2026-09-29, but no loop has a measured benefit, and the knowledge read path injects nothing. |
| V9 Cybernetic throughout | PARTIAL@ed0c33bd5 | RG1, RG2, RG3, RG4, RG5, RG6, RC3, RC5, QA7 | Provider health, the plan budget and adaptive retry budgets close loops, but nothing audits the gates or the regulators. |
| V10 Observable and controllable | PARTIAL@ed0c33bd5 | RG1, SS4, SS5, SS6 | Watching works; pause is cosmetic, the CLI controls are no-ops, and there is no approval step or diff view. |
