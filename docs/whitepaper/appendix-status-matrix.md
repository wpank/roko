Status: reviewed · budget none · owner gap-35a614

# Appendix: status matrix

The status of every mechanism in this paper at commit a17d4dadd (2026-09-29), each tag re-checked against the code at that commit. The matrix is a dated snapshot: the work graph in work/items stays the live status, and the matrix is refreshed at each review. Sections cite rows by id, such as EX3.

Each row gives the mechanism, its status at the pinned commit, what runs today, the code that implements it, the evidence for the tag and the next step. A code anchor names a file and a symbol in it; `tools/status_matrix.py` fails if either is missing at the pinned commit. Evidence is a commit, a test, or a work item whose recorded state supports the tag. The next step is a verdict and, where one is filed, the work item that would change the tag.

| Status | Meaning |
|---|---|
| WIRED@a17d4dadd | Runs on the production path (plan runs through the Graph engine) and does its job: a sensor records, a regulator changes behaviour |
| PARTIAL@a17d4dadd | On the path but incomplete, masked, or fed wrong data |
| BROKEN@a17d4dadd | Wired, but does the wrong thing |
| ORPHANED@a17d4dadd | Worked under Runner-v2, the event loop deleted in `6b5da8616`, and was not re-attached |
| BUILT-UNWIRED@a17d4dadd | Code and tests exist, with no production caller |
| MISSING@a17d4dadd | Not built |
| REMOVED@a17d4dadd | Deliberately deleted |
| UNPROVEN@a17d4dadd | Might be true; no measurement exists |

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

Rows per status and group at a17d4dadd.

| Status | AU | EX | IS | QA | RC | LM | RG | SS | DM | Total |
|---|---|---|---|---|---|---|---|---|---|---|
| WIRED@a17d4dadd | 3 | 5 | 0 | 2 | 3 | 1 | 1 | 2 | 4 | 21 |
| PARTIAL@a17d4dadd | 2 | 1 | 4 | 2 | 3 | 6 | 1 | 2 | 2 | 23 |
| BROKEN@a17d4dadd | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 2 |
| ORPHANED@a17d4dadd | 0 | 1 | 1 | 1 | 0 | 3 | 1 | 0 | 0 | 7 |
| BUILT-UNWIRED@a17d4dadd | 1 | 1 | 0 | 1 | 1 | 1 | 0 | 0 | 2 | 7 |
| MISSING@a17d4dadd | 1 | 1 | 1 | 2 | 0 | 0 | 4 | 1 | 0 | 10 |
| REMOVED@a17d4dadd | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 |
| Rows | 7 | 9 | 6 | 8 | 7 | 12 | 7 | 6 | 9 | 71 |

## AU · Authoring and specs

Turning a request into a plan whose tasks a cheap model can execute and a machine can check.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| AU1 | Plan generator | WIRED@a17d4dadd | Turns a PRD or prompt into a validated plan; one pipeline serves roko prd plan, the portal and the complex band of roko do, while roko plan generate keeps its own prompt. The planner model is selectable only from the CLI. | `crates/roko-cli/src/prd.rs`: `generate_plan_from_prd_with_outcome`, `validate_and_fix_generated_plan` | gap-2623b2, gap-853b31 | keep · gap-853b31 |
| AU2 | Plan validation | WIRED@a17d4dadd | About fifty deterministic lint codes check schema, dependency graph, context and budget before a run. Plans started through the server skip them. | `crates/roko-cli/src/plan_validate.rs`: `validate_plans_dir`; `crates/roko-cli/src/plan_policy.rs`: `PlanExecutionPolicy` | gap-655d19 | keep · gap-46ab3f |
| AU3 | Spec-quality lint and gate | MISSING@a17d4dadd | Nothing scores a task's spec, and nothing blocks a vague or unverifiable task, or a verify step too weak to fail, before dispatch. | none | find-70edcb, gap-1cd8d3 | build · gap-1cd8d3 |
| AU4 | Context packs | WIRED@a17d4dadd | The files, symbols and line ranges a task declares are rendered into its prompt; a declared file that is missing fails the plan load. | `crates/roko-cli/src/plan_policy.rs`: `render_declared_context` | bug-7c8a57 | keep |
| AU5 | Code index | BUILT-UNWIRED@a17d4dadd | The parsed code graph reaches agents only as an optional MCP server. The CLI uses it for gate impact analysis, which plan runs do not execute; it never feeds context packs. | `crates/roko-index/src/lib.rs`; `crates/roko-mcp-code/src/lib.rs` | gap-a247a4 | wire · spec-1ced1d |
| AU6 | Acceptance criteria | PARTIAL@a17d4dadd | Criteria are rendered into the task prompt as text. The typed acceptance contract is schema-checked by plan validation, and its evaluator has no production caller. | `crates/roko-gate/src/acceptance_contract.rs`: `AcceptanceContract`, `validate_evidence` | gap-1cd8d3, gap-d14a43 | redesign · spec-e57870 |
| AU7 | Pinned acceptance tests | PARTIAL@a17d4dadd | A plan-authoring convention, not harness code: a verify step copies planner-written tests over the agent's copy and pins the passing count. The agent can still read and edit the source tests. | `plans/portal-programme/08f-final-polish/accept` | gap-d14a43 | redesign · gap-d14a43 |

## EX · Execution

Running a plan's tasks as a dependency graph, with retries, resume and supervision.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| EX1 | Graph engine | WIRED@a17d4dadd | Runs every plan as a dependency graph of cells. It is the only plan executor: it became the default in `bcd1c8624`, and the Runner-v2 event loop was deleted in `6b5da8616`. | `crates/roko-graph/src/engine.rs`: `GraphEngine` | `bcd1c8624`, `6b5da8616` | keep |
| EX2 | Durable checkpoints and resume | WIRED@a17d4dadd | Each plan's checkpoint is replayed on resume, and a fingerprint of the authored plan rejects a changed plan. Failed, blocked and unstarted tasks are recorded and re-run on resume since `abc1f4b27`. | `crates/roko-cli/src/graph_checkpoint.rs`: `prepare_graph_checkpoint` | `abc1f4b27`, gap-7147bb | keep |
| EX3 | Parallel task scheduling | PARTIAL@a17d4dadd | A ready queue starts each task once its own dependencies settle, and a failed task skips only its dependants (the default failure policy since `abc1f4b27`). But `max_parallel` still defaults to 1, and nothing admits tasks by write set. | `crates/roko-graph/src/engine.rs`: `execute_ready_queue`; `crates/roko-cli/src/task_parser.rs`: `default_max_parallel` | `445a60d0d`, `3e7552acd`, `abc1f4b27`, test `crates/roko-graph/src/engine.rs`: `a_failed_node_blocks_only_its_dependants` | fix · gap-272448 |
| EX4 | Plan-set scheduler | WIRED@a17d4dadd | Runs several plans at once, admitting only plans whose file footprints don't overlap. The default is still one plan at a time. | `crates/roko-cli/src/graph_execution/plan_set.rs`: `PlanSetScheduler` | `725f21e05`, gap-7c9e48 | keep · gap-7c9e48 |
| EX5 | Sibling settle | WIRED@a17d4dadd | In the shared checkout, a verify step that failed while a sibling task was mid-edit waits for the sibling and re-runs once. It is a narrow mitigation until isolation lands. | `crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs`: `InFlightTasks` | `3049b7fcf` | keep |
| EX6 | Retry with gate feedback | WIRED@a17d4dadd | A failed task is re-dispatched with its parsed gate output, up to `max_retries` (3 by default). Since `ce3bdcbb8` the retry feedback survives a resume and the failing step names its rung. | `crates/roko-cli/src/task_parser.rs`: `default_max_retries`; `crates/roko-graph/src/cells/task_executor.rs`: `TaskExecutorCell` | `ce3bdcbb8`, `41c7ffbd6` | keep |
| EX7 | Escalation to a stronger model | ORPHANED@a17d4dadd | Runner-v2 moved a failing task up a model ladder. On the Graph path a retry reruns the same model, and the ladder constant and the complexity-band escalation have no caller. | `crates/roko-core/src/defaults.rs`: `MODEL_ESCALATION_LADDER`; `crates/roko-core/src/task.rs`: `TaskComplexityBand` | `6b5da8616`, gap-b62e95 | wire · gap-460230 |
| EX8 | Split or replan on failure | BUILT-UNWIRED@a17d4dadd | The replan controller has no caller, and the per-task split and replan fields are parsed and ignored. Gate-failure replanning was deleted with Runner-v2. | `crates/roko-execution/src/replan_controller.rs`: `ReplanController` | `6b5da8616` | wire · gap-3b170b |
| EX9 | Progress watchdog | MISSING@a17d4dadd | Only a wall-clock timeout per attempt and per-tier turn caps stop a hung agent; nothing detects an agent that has stopped making progress. | none | spec-a0403b, spec-edda86 | build · spec-a0403b |

## IS · Isolation and integration

Keeping concurrent agents out of each other's way and out of the operator's secrets, then combining their results.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| IS1 | Worktrees | PARTIAL@a17d4dadd | Tasks edit the operator's checkout by default. The opt-in per-task worktree forks from HEAD, and its successful edits are never merged back. | `crates/roko-cli/src/graph_execution/plan_runner.rs`: `worktree_per_task`; `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `WorktreeManager` | gap-4ec59f | redesign · gap-4ec59f |
| IS2 | Merge queue, plan merger and attempt acceptance | ORPHANED@a17d4dadd | Runner-v2 integrated task results through these. On the Graph path nothing merges or commits a result, and `accept_attempt` has no production caller. | `crates/roko-cli/src/orchestrator/merge_queue.rs`: `MergeQueue`; `crates/roko-cli/src/runner/merge.rs`: `PlanMerger`; `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `accept_attempt` | `6b5da8616`, gap-3b5361, bug-a3760a | wire · gap-3b5361 |
| IS3 | Whole-plan verify on the integrated result | MISSING@a17d4dadd | Verify steps are per task. No plan-level gate checks that the parts work together. | none | gap-60233f, gap-af00b1 | build · gap-60233f |
| IS4 | Git guard hook | PARTIAL@a17d4dadd | The Claude CLI hook blocks recursive deletes anywhere in a command, but git checkout, switch and push only at its start; git reset, stash and clean pass, and the hook fails open without python3. | `crates/roko-agent/src/claude_cli_agent.rs`: `build_settings_json` | bug-7de5df | fix · bug-7de5df |
| IS5 | Environment isolation | PARTIAL@a17d4dadd | Since `dc99a9e81` (bug-7d7200), gates start from an allowlisted environment, and provider CLIs have the key variables they recognise scrubbed, so a key under an unrecognised name still reaches the agent. Agent tool shells, MCP servers and provider probes inherit everything, and the key files stay readable. | `crates/roko-core/src/child_env.rs`: `gate_env`; `crates/roko-gate/src/gate_env.rs`; `crates/roko-agent/src/process/env.rs`: `apply_credential_scrub` | `dc99a9e81`, bug-7d7200, test `crates/roko-core/src/child_env.rs`: `gate_env_drops_secret_looking_names_even_under_allowed_prefixes`, bug-0d9ac4, bug-0eb8e2, bug-a66941 | fix · spec-ba7bea |
| IS6 | Sandbox levels | PARTIAL@a17d4dadd | A sandbox-level setting and resource limits exist, but no OS sandbox. The pre- and post-dispatch safety checks run in the ACP server, not on plan runs. | `crates/roko-agent/src/safety/sandbox.rs`: `SandboxLevel`; `crates/roko-agent/src/safety/mod.rs`: `pre_dispatch_check` | bug-7debad | fix · no item filed |

## QA · Verification and QA

Deciding whether a task is done, and whether that decision can be trusted.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| QA1 | Task verify commands | WIRED@a17d4dadd | After the agent finishes, the task's own shell checks run fail-fast in its working directory. Each outcome carries a typed verdict (since `725f21e05`), and learning is fed only after the verify steps settle. | `crates/roko-cli/src/graph_task_dispatch.rs`: `settle_task_verification`; `crates/roko-gate/src/shell.rs`: `ShellGate` | `725f21e05` | keep |
| QA2 | Typed verdicts | PARTIAL@a17d4dadd | Passed, Unverified and ForcedAccept are stamped on every outcome and checkpointed. But learning receives a bool in which Unverified counts as a pass, plan success ignores verdicts, and the dashboard counts unverified and skipped tasks as passed. | `crates/roko-graph/src/cells/task_executor.rs`: `TaskGateVerdict` | `725f21e05`, bug-7e1b6b, gap-29a84b | fix · spec-e9d7ec |
| QA3 | Seven-rung gate pipeline | ORPHANED@a17d4dadd | Compile, lint, test, symbol, generated-test, property-test and integration rungs exist, but plan tasks run only their authored verify steps; the pipeline's entry point is reachable only from tests. | `crates/roko-cli/src/runner/gate_dispatch.rs`: `run_gate_once`, `spawn_gate` | `6b5da8616`, gap-85f102 | redesign · spec-6ac537 |
| QA4 | Tamper and scope diff | MISSING@a17d4dadd | Nothing flags an attempt that edits tests, verify scripts or planner-written acceptance tests, or that edits files outside its task. | none | spec-9230a9 | build · gap-abbd22 |
| QA5 | Hidden tests and random audits | MISSING@a17d4dadd | No tests are held back from the agent, and no sample of accepted passes is re-checked, so the false-green rate is not estimated. | none | spec-6ac537, gap-89f393 | build · spec-6ac537 |
| QA6 | LLM judge and reviewer verdicts | BUILT-UNWIRED@a17d4dadd | The judge rung has no production oracle and is always skipped. Reviewer BLOCK and REVISE verdicts do not gate Graph tasks. | `crates/roko-gate/src/llm_judge_gate.rs`: `JudgeOracle`; `crates/roko-gate/src/review_verdict.rs`: `parse_structured_review_verdict` | gap-85f102, gap-f4b935 | wire · gap-f4b935 |
| QA7 | Adaptive gate thresholds | WIRED@a17d4dadd | Per-rung pass rates are updated after every verify and, since `99adacd6d`, bound the retry budget of tasks without an authored `max_retries`. Concurrent runs can lose updates to the threshold file (bug-e0f472). | `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs`: `suggested_max_retries`; `crates/roko-cli/src/runner/persist.rs`: `suggested_max_retries` | `99adacd6d`, `41c7ffbd6`, `ce3bdcbb8`, bug-e0f472 | keep |
| QA8 | Gaming detector and holdout | PARTIAL@a17d4dadd | The gaming detector has no audited quality labels and no actuator, and the holdout split only logs. Both wait for audits to supply ground truth. | `crates/roko-learn/src/gate_gaming.rs`: `GateGamingDetector`; `crates/roko-learn/src/holdout.rs`: `HoldoutExperiment` | find-4b4344 | park · find-4b4344 |

## RC · Routing and cost

Choosing a model for each attempt and accounting for what it costs.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| RC1 | Provider adapters | WIRED@a17d4dadd | Twelve provider kinds: the Claude, Codex, Cursor and Gemini CLIs, API adapters and OpenAI-compatible endpoints for cheap models. The Hermes kind carries open parser bugs. | `crates/roko-core/src/agent.rs`: `ProviderKind` | bug-7567eb, bug-b14145 | keep |
| RC2 | Learned model router | PARTIAL@a17d4dadd | Picks a model only when neither the task nor the run pins one. Its learned stage updates on successes only: router-chosen failures never reach it, and manual overrides count as successes. | `crates/roko-learn/src/cascade_router.rs`: `CascadeRouter`; `crates/roko-cli/src/dispatch/model_routing.rs`: `ModelRouter` | bug-8da8ba, bug-f68404, bug-c34782 | fix · bug-8da8ba |
| RC3 | Provider-health circuit breaker | WIRED@a17d4dadd | Routes away from providers whose recent calls failed, a closed loop. Attempt timeouts and turn-cap stops also count as provider failures (bug-7cdce7). | `crates/roko-learn/src/cascade_router.rs`: `route_with_health_scored` | bug-7cdce7 | keep |
| RC4 | Provider failover | PARTIAL@a17d4dadd | On a refusal or an exhausted quota the call moves to other keys and fallback models, sideways rather than up, and the record keeps the model that was originally chosen. | `crates/roko-cli/src/graph_task_dispatch.rs`: `run_bridge_with_failover`, `failover_candidates` | bug-35379d | fix · bug-35379d |
| RC5 | Budgets | WIRED@a17d4dadd | A plan ceiling with a per-call reservation, and a per-task retry ceiling; since `abc1f4b27` no new task starts once settled spend reaches the ceiling. Daily and agent-lifetime limits are not enforced. | `crates/roko-cli/src/graph_task_dispatch.rs`: `GraphPlanBudgetPolicy`, `GraphTaskSpendLedger` | `abc1f4b27`, bug-ae28ac, gap-34b2ed | keep |
| RC6 | Cost accounting | PARTIAL@a17d4dadd | Since `d4be4e872`, timed-out and failed attempts are priced from the usage they streamed and marked as estimates (bug-690dc6). Cost rows still carry no source, and prices come from undated tables. | `crates/roko-core/src/usage.rs`: `UsageSource`; `crates/roko-cli/src/dispatch_v2.rs`: `fill_cost_from_profile` | `d4be4e872`, bug-690dc6, gap-288e38, gap-ad0d39 | fix · gap-288e38 |
| RC7 | Inference gateway | BUILT-UNWIRED@a17d4dadd | Caching, batching, fallback and cost accounting run only under roko serve; no plan-run path depends on the gateway crate. It stays serve-only until an API-model arm needs caching. | `crates/roko-gateway/src/gateway.rs`: `InferenceGateway` | gap-cdf3fc | keep |

## LM · Learning and memory

Records of past attempts, and the loops that feed them back into prompts and routing.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| LM1 | Episode and efficiency records | PARTIAL@a17d4dadd | Each attempt leaves records for learning; since 2026-09-29 episodes keep the full failure reason and classify timeouts. Many efficiency fields stay empty on the live path, and tool events are counted as tool calls. | `crates/roko-cli/src/runtime_feedback/episodes.rs`; `crates/roko-learn/src/efficiency.rs`: `AgentEfficiencyEvent` | `e39b5519a`, `d4be4e872`, gap-7a8474, bug-f9ae3e | fix · spec-b7303f |
| LM2 | Playbooks | WIRED@a17d4dadd | Since `763596768` outcomes are credited to the playbooks a prompt actually used (reg-3f5969), and selection adds each playbook's net successes (successes minus failures) to its relevance. A floor still puts three playbooks into every prompt, and nothing measures whether they help. | `crates/roko-cli/src/dispatch/prompt_builder.rs`: `collect_playbooks_cached` | `763596768`, reg-3f5969 | fix · spec-6ac537 |
| LM3 | Knowledge store | BROKEN@a17d4dadd | Since `189a14e65`, gate-verified attempts grow and reinforce the store (reg-06ae9f). But the prompt cache queries it with an empty topic, which matches nothing, so plan-run prompts carry no knowledge (bug-86117a). | `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs`: `VerifiedKnowledgeSink`; `crates/roko-cli/src/dispatch/prompt_cache.rs`: `PromptCache` | `189a14e65`, reg-06ae9f, bug-86117a | fix · bug-86117a |
| LM4 | Error patterns and post-gate lessons | ORPHANED@a17d4dadd | Runner-v2 fed discovered error patterns and retry lessons back into prompts. On the Graph path the pattern reader loads a file that nothing writes, and each gate failure pays for a lesson that no retry prompt reads. | `crates/roko-learn/src/error_pattern_store.rs`: `ErrorPatternStore`; `crates/roko-learn/src/post_gate_reflection.rs`: `PostGateReflectionStore` | `6b5da8616`, gap-e483e7 | wire · spec-6ac537 |
| LM5 | Prompt experiments | PARTIAL@a17d4dadd | Since `ebf274ada`, prompt variants are assigned per attempt, bound to the hash of the final prompt and settled with the attempt's outcome, on both dispatch paths (gap-fdd27f). But arms are assigned adaptively (UCB1) and the winner is declared by a chi-squared test, whose error rate is not controlled under adaptive assignment. | `crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs`: `LaunchedTreatments`; `crates/roko-learn/src/prompt_experiment.rs`: `ExperimentStore` | `ebf274ada`, gap-fdd27f | fix · spec-6ac537 |
| LM6 | Section selection by bandit and auction | PARTIAL@a17d4dadd | Prompt composition reads per-section posteriors and bids, but nothing on the plan path records section outcomes, so the posteriors never move. | `crates/roko-learn/src/section_outcome.rs`: `SectionOutcomeStore`; `crates/roko-runtime/src/heartbeat_attention.rs`: `ContextBidder` | `6b5da8616` | redesign · spec-6ac537 |
| LM7 | Offline consolidation | ORPHANED@a17d4dadd | Runner-v2 ran a paid consolidation job after plans. The Graph path registers a plan-completion sink for it, but no production code emits the plan-completed event, so plan runs never start it. | `crates/roko-cli/src/runtime_feedback/plan_completion.rs`: `DreamConsolidationSink` | q-6b7cca, bug-470de8, bug-b9be1d | park · q-6b7cca |
| LM8 | Affect state | PARTIAL@a17d4dadd | An affect state is appraised after every task and shifts the routing tier, with no record of its influence. The sink that saves it fires only on the plan-completed event, so each run's updates are lost. | `crates/roko-cli/src/graph_task_dispatch.rs`: `build_routing_context`; `crates/roko-cli/src/runtime_feedback/plan_completion.rs` | q-6b7cca | park · q-6b7cca |
| LM9 | Reflex shortcut | ORPHANED@a17d4dadd | Nothing on the plan path promotes reflex rules, and a rule hit would record a gate pass without running any gate. | `crates/roko-learn/src/reflex_store.rs`: `record_gate_pass_for`; `crates/roko-cli/src/graph_task_dispatch.rs`: `record_gate_pass_for` | bug-94151f | remove · bug-94151f |
| LM10 | Similarity and collective-intelligence metrics | BUILT-UNWIRED@a17d4dadd | Episode fingerprints are hash-expanded vectors that nothing reads; the active-inference router has no caller; the collective-intelligence prompt block reads a file that plan runs never write; compounding metrics are never computed. | `crates/roko-cli/src/runtime_feedback/episodes.rs`: `attach_episode_hdc_fingerprint`; `crates/roko-learn/src/active_inference.rs`: `EfeRouter`; `crates/roko-cli/src/dispatch/prompt_builder.rs`: `generate_cfactor_context` | gap-14f08e | park |
| LM11 | Second learning pipeline | PARTIAL@a17d4dadd | A separate learning runtime settles roko do, roko prd and plan-authoring runs, not plan runs, and updates router and section state on its own. | `crates/roko-learn/src/runtime_feedback/mod.rs`: `LearningRuntime`; `crates/roko-cli/src/agent_exec.rs`: `record_completed_run` | spec-b7303f | redesign · spec-b7303f |
| LM12 | Hindsight relabelling | PARTIAL@a17d4dadd | Since `33e107da1`, a verify failure that names a sibling task relabels that sibling's latest success as a failure. The corrections go to an adjustments file that nothing reads, so no learner sees them. | `crates/roko-cli/src/runtime_feedback/hindsight.rs`: `HindsightSink`; `crates/roko-cli/src/graph_execution/plan_runner.rs`: `HindsightSink` | `fb87e3738`, `33e107da1`, gap-5fb9a7, gap-5be28d | fix · gap-5be28d |

## RG · Regulation and audits

The second-order loops that watch the first-order ones.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| RG1 | Telemetry lenses and state hub | WIRED@a17d4dadd | Sensors: per-task model, cost, tool steps and gate output flow to the TUI and the HTTP event streams. The plan event stream still lacks run completion, heartbeats and timestamps. | `crates/roko-runtime/src/state_hub.rs`: `StateHub`; `crates/roko-cli/src/runner/graph_tui_bridge.rs`: `StateHubTelemetrySink`; `crates/roko-runtime/src/telemetry_projection_aggregator.rs`: `LensPayload` | gap-8a1fb3 | keep |
| RG2 | Conductor watchers and circuit breaker | ORPHANED@a17d4dadd | The watcher engine has no caller on the plan path since Runner-v2's supervisor tick was deleted; that tick had caused restart storms. The ultrastable controller (RG3) is meant to replace it. | `crates/roko-conductor/src/conductor.rs`: `Conductor`; `crates/roko-cli/src/runner/conductor_adapter.rs`: `ConductorRingSink` | `6b5da8616`, gap-ebd656 | park · gap-ebd656 |
| RG3 | Ultrastable controller (M1) | MISSING@a17d4dadd | Nothing changes a harness parameter when an essential variable, such as the verified pass rate or the cost per verified task, leaves its bounds. | none | spec-6ac537 | build · spec-6ac537 |
| RG4 | Loop-liveness audit (M2) | MISSING@a17d4dadd | No learning loop shows its exposure, influence and benefit: there is no decision log, no withhold arm and no way to freeze learning for a comparison. | none | gap-1f2661, gap-644040 | build · spec-6ac537 |
| RG5 | Calibrated self-model (M3) | MISSING@a17d4dadd | Nothing forecasts a task's chance of passing on a given model, so routing and escalation cannot be judged against a forecast. | none | spec-6ac537 | build · spec-6ac537 |
| RG6 | Guarded commit with rollback | MISSING@a17d4dadd | No self-modification, whether to the router, memory or a controller, is checked on held-out work before it is accepted, and none can be rolled back. | none | gap-644040 | build · spec-6ac537 |
| RG7 | Cross-cut functors | PARTIAL@a17d4dadd | Only a routing-bias arbitration runs on the plan path, turning the affect state and stale consolidation advice into a prefer-cheaper routing bias whose effect is not recorded. | `crates/roko-cli/src/graph_task_dispatch.rs`: `arbitrate_cross_cut_routing_bias` | `725f21e05` | park |

## SS · Safety, steering and surfaces

Who may do what, and how an operator watches and steers a run.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| SS1 | Auth, roles and API keys on the control plane | WIRED@a17d4dadd | Role-based permissions and API keys protect roko serve. An open p0 lets a Privy sign-in act as admin on a publicly bound server. | `crates/roko-serve/src/rbac.rs`: `Role`, `Permission` | bug-7eef96, find-95ef81 | fix · bug-7eef96 |
| SS2 | Tool policy | PARTIAL@a17d4dadd | A role's allowed and forbidden tools become Claude CLI flags, so Claude enforces tool names. Count-based contract rules never fire, and the Codex policy is advisory. | `crates/roko-agent/src/safety/contract.rs`: `forbidden_tool_names` | bug-1948c9, gap-baab0a | fix · bug-1948c9 |
| SS3 | Output screening and operator deference | PARTIAL@a17d4dadd | Outputs and memory writes are screened before they are stored, but nothing measures how often the screen wrongly denies, and the quarantine routes read a file the runtime never writes. | `crates/roko-core/src/immune.rs`: `ImmunePipeline`; `crates/roko-core/src/corrigibility.rs`: `CorrigibilityHead` | bug-4cb029, gap-2f69e9 | fix · gap-f75dc8 |
| SS4 | TUI, portal, HTTP API and event streams | WIRED@a17d4dadd | Operators watch each task's model, cost, gate output and transcript live. Cancel works from the TUI, REST, the portal and Ctrl-C, but lands only between nodes. | `crates/roko-serve/src/routes/plans.rs`: `cancel_plan`; `apps/portal/src` | `d9efb9338`, gap-b367bf | keep |
| SS5 | Pause, resume, retry, skip and approve | BROKEN@a17d4dadd | TUI pause sets a flag nothing reads while the status bar shows paused; the CLI controls write a file nothing reads; TUI retry and skip are acknowledged and dropped. REST pause cancels and later restarts from the checkpoint. | `crates/roko-graph/src/cell.rs`: `is_paused`; `crates/roko-cli/src/runner/types.rs`: `ControlCommand` | bug-8208a6, gap-c002bb, gap-1555ac | fix · bug-8208a6 |
| SS6 | Per-task diff, approve-before-merge and budget alarms | MISSING@a17d4dadd | There is no approval hold before integration and no budget threshold event. The task-diff route looks for branch names that Graph runs never create, so it finds nothing. | `crates/roko-serve/src/routes/plans.rs`: `find_agent_branch`; `crates/roko-cli/src/graph_execution/control_adapter.rs`: `GraphExecutionControlAdapter` | code read at `a17d4dadd` | build · gap-25065c |

## DM · Domains and automation

Work beyond code, and work started by events rather than by a person.

| Row | Mechanism | Status | Today | Code | Evidence | Next |
|---|---|---|---|---|---|---|
| DM1 | Shell-command verifiers | WIRED@a17d4dadd | Any command can be a gate, as a custom rung in roko.toml or a task's verify step. It is the only verifier not tied to code, and roko run refuses to start without one in a workspace it cannot build. | `crates/roko-core/src/config/gates.rs`: `effective_rungs`; `crates/roko-cli/src/runner/gate_dispatch.rs`: `has_custom_rungs` | bug-1410e8 | keep |
| DM2 | Task domain label | PARTIAL@a17d4dadd | A task can be labelled code, research or docs. Routing reads the label; gate selection does not. | `crates/roko-core/src/task.rs`: `TaskDomain`; `crates/roko-cli/src/dispatch/model_routing.rs`: `task_domain` | code read at `a17d4dadd` | wire · gap-7a3527 |
| DM3 | Triggers | WIRED@a17d4dadd | Cron, webhook and file-watch events start a graph through roko serve. The graph cell registry has no agent or shell cell, so a trigger cannot start agent work. | `crates/roko-serve/src/trigger_runtime.rs`: `ensure_trigger_runtime`; `crates/roko-graph/src/engine.rs`: `default_registry` | code read at `a17d4dadd` | wire |
| DM4 | Feeds and recipes | PARTIAL@a17d4dadd | Since `310860465`, roko serve registers three built-in feeds at startup, but they only publish events: no plan or agent consumes them, and recipes only compute scores. | `crates/roko-serve/src/state.rs`: `EpisodeOutcomeFeed`; `crates/roko-core/src/feeds/provider_health.rs` | `310860465` | park |
| DM5 | MCP client and built-in tools | WIRED@a17d4dadd | Every dispatch gets the built-in tools and the configured MCP servers. MCP servers still inherit the full environment, provider keys included. | `crates/roko-agent/src/mcp/client.rs`: `McpClient` | bug-0eb8e2 | keep |
| DM6 | Plugin install | WIRED@a17d4dadd | Signed install, admission and dependency resolution for plugin manifests work. | `crates/roko-plugin/src/lib.rs`: `PluginManifest` | code read at `a17d4dadd` | keep |
| DM7 | WASM plugin hooks | REMOVED@a17d4dadd | The hook runtime was deleted with Runner-v2, and loading a WASM extension returns an error. The docs still call the hooks live. | `crates/roko-cli/src/runner/extension_loader.rs`: `WasmExtension` | `6b5da8616`, bug-0e13d0 | remove · gap-cdf3fc |
| DM8 | Messaging channel adapters | BUILT-UNWIRED@a17d4dadd | Adapters for six messaging platforms and a channel router exist, but they are not declared as modules, so they are not compiled; the Telegram adapter is a stub. | `crates/roko-runtime/src/channel_binding.rs`: `ChannelBindingRouter`; `crates/roko-runtime/src/adapters/telegram.rs` | gap-8d80d3, spec-743a7e | wire · gap-8d80d3 |
| DM9 | Agent groups, coordination markers and relay | BUILT-UNWIRED@a17d4dadd | Group membership, coordination markers and cross-machine relay have storage and REST routes, but plan runs neither join groups nor leave markers; they belong to a separate multi-user product. | `crates/roko-core/src/groups.rs`: `PheromoneDeposit`; `crates/roko-serve/src/group_runtime.rs`; `crates/roko-runtime/src/connector_runtime.rs`: `HttpJsonConnector` | code read at `a17d4dadd` | park |

## Vision claims

The paper's ten vision claims, each tagged at the same commit and traced to the rows it rests on.

| Claim | Status | Rests on | Main blocker |
|---|---|---|---|
| V1 Domain-agnostic orchestration | PARTIAL@a17d4dadd | DM1, DM2, DM3, DM4, QA6 | Shell commands are the only verifier for work that is not code, and triggers cannot start agent work. |
| V2 A fast authoring loop with a frontier planner | PARTIAL@a17d4dadd | AU1, AU2, AU3 | The planner model is selectable only from the CLI, and there is no spec-quality lint and no plan diff. |
| V3 Granular tasks executed by the cheapest capable models | MISSING@a17d4dadd | AU1, EX7, RC2 | Nothing routes by task difficulty, and generated plans are coarse and lose their model hints. |
| V4 Parallel execution of the task graph | PARTIAL@a17d4dadd | EX3, EX4, EX5 | The ready queue runs independent tasks, but `max_parallel` defaults to 1 and nothing admits tasks by write set. |
| V5 Safe isolation and correct integration | PARTIAL@a17d4dadd | IS1, IS2, IS3, IS4, IS5, IS6 | The shared checkout is the default, nothing merges, no whole-plan check exists, and agent tool shells still see provider keys. |
| V6 Trust from gates and acceptance criteria | PARTIAL@a17d4dadd | QA1, QA2, QA3, QA4, QA5, QA6, AU6, AU7 | Acceptance criteria are prompt text, and there are no hidden, tamper, scope or whole-plan checks. |
| V7 Cheaper and faster than top models | UNPROVEN@a17d4dadd | RC2, RC6, EX7 | No head-to-head run exists, escalation is not wired, and cost rows carry no source. |
| V8 Improves over time | PARTIAL@a17d4dadd | LM1, LM2, LM3, LM4, LM5, LM6, LM11, LM12, RC2 | Several loops were re-wired on 2026-09-29, but no loop has a measured benefit, and the knowledge read path injects nothing. |
| V9 Cybernetic throughout | PARTIAL@a17d4dadd | RG1, RG2, RG3, RG4, RG5, RG6, RC3, RC5, QA7 | Provider health, the plan budget and adaptive retry budgets close loops, but nothing audits the gates or the regulators. |
| V10 Observable and controllable | PARTIAL@a17d4dadd | RG1, SS4, SS5, SS6 | Watching works; pause is cosmetic, the CLI controls are no-ops, and there is no approval step or diff view. |
