# ACP Protocol: Goals

## End State

ACP is the primary editor integration surface. It uses the same unified WorkflowEngine, PromptAssemblyService, and FeedbackService as all other entry points. Supports multi-agent parallel execution, architect review mode, agent following, and dynamic slash commands — all from within the editor.

## Key Properties

- **Shared workflow engine**: ACP workflow templates come from the same config as `roko plan run`.
- **Learning feedback**: ACP sessions produce episodes, feed CascadeRouter, update thresholds.
- **Safety contracts**: Role-based contracts applied to ACP agent dispatch.
- **Full tool dispatch**: MCP tools accessible from ACP, not just file I/O.
- **Session concurrency safety**: Proper locking for multi-editor scenarios.
- **Parallel agents**: Spawn N agents (Claude/Codex/Gemini) in isolated worktrees, stream progress per-thread, synthesize results.
- **Architect review mode**: Read-only analysis with embedded resources, structured review output, mode-switch handoff to implementation.
- **Agent following**: Live cursor tracking as agent reads files, call graph synthesis from traversal, AGENT NAVIGATES markers.
- **Dynamic slash commands**: User-defined and workflow-generated `/commands` from TOML config.
- **Dynamic re-planning**: Mid-execution plan revision when conditions change (P1 triage, gate failures).
- **Permission scoping**: Per-action approve/deny with auto-approve toggles ("auto-approve reads").
- **Synthesis view**: Benchmark comparison across parallel approaches, merge prompt with quantitative data.

## What Exists Today

- Pure FSM pipeline (10 phases, 3 templates, auto-selection)
- Multi-role sequential dispatch (Strategist → Implementer → AutoFixer → Reviewer)
- 49 registered slash commands (all returned from `build_slash_commands()` in session.rs)
- 9 config options (model, effort, temperament, routing_mode, clippy, tests, workflow, review_strictness, max_iterations) — mode is set via `session/set_mode`, not a config option
- Tool call streaming (start/complete per phase)
- SharedWorkflowRun for live queries
- 6 provider kinds (AnthropicApi, ClaudeCli, OpenAiCompat, CursorAcp, PerplexityApi, GeminiApi) — model aliases come from roko.toml [models.*] entries
- Session persistence to disk

## Gap

### Integration (existing code, not wired)
- Wire to ModelCallService instead of raw CLI subprocess
- Wire FeedbackService for episode logging
- Wire SafetyLayer for contract enforcement
- Wire SystemPromptBuilder (9-layer) instead of simple mode prompts (requires adding roko-compose to Cargo.toml — not a dependency today)
- Wire MCP server passthrough (stored but not forwarded)
- Connect to shared workflow config
- File context injection (editor sends open files, not used)
- Knowledge-informed prompts (neuro store not queried)

### Parallel Agents (new infrastructure)
- Multi-thread execution (tokio::join for N agents)
- Per-agent worktree isolation (git worktree per thread)
- Per-agent progress streaming (independent tool call streams)
- Synthesis / benchmark comparison across approaches
- Merge prompt UX (select best approach or custom merge)

### Advanced Modes (new infrastructure)
- Architect review mode (read-only + structured output + mode switch)
- Agent following mode (cursor tracking + traversal trace)
- Debug mode (step-through execution)
- Auto-approve scoping (per-action permission toggles)

### Dynamic Slash Commands (new infrastructure)
- User-defined commands from TOML
- Workflow-installed commands
- Composable commands (chain existing commands)
- Context-sensitive command suggestions

See also: `FEATURES.md` for full feature inventory with status.

---

## Sources

- `crates/roko-acp/src/session.rs` — `build_slash_commands` (49 commands), `build_config_options` (9 options), `SessionConfigState`, `AcpSession` struct
- `crates/roko-acp/src/pipeline.rs` — `PipelinePhase` (10 states), `WorkflowTemplate` (3 templates + auto-select)
- `crates/roko-acp/src/runner.rs` — multi-role dispatch (Strategist, Implementer, AutoFixer, Reviewer, Architect, Auditor), `SharedWorkflowRun`
- `crates/roko-acp/Cargo.toml` — dependency list (roko-compose absent, confirms SystemPromptBuilder gap)
