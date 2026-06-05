# ACP Current State

## Implementation Status: COMPLETE at protocol layer

### What the acp-runner produced

The runner executed 5 runs. The final run (`run-20260427-002955`) succeeded:
- **8/8 batches passed** (ACP01–ACP08: scaffold, types, transport, handler, session, streaming, CLI, conformance tests)
- **Total runtime:** ~31 minutes using gpt-5.4
- **All gates passed:** scope, diff, required_terms, cargo check, clippy, tests
- **Commit style:** `acp(ACPnn): <title>`

### What exists in crates/roko-acp/ (7,168 LOC)

| File | LOC | Role |
|------|-----|------|
| types.rs | 750 | JSON-RPC 2.0 protocol types |
| transport.rs | 307 | Stdio framing |
| handler.rs | 388 | Method dispatch |
| session.rs | 1,546 | Session lifecycle + 49 slash commands + 9 config options |
| bridge_events.rs | 1,757 | Provider dispatch + ACP streaming |
| acp_adapter.rs | 196 | EventConsumer → ACP notifications |
| pipeline.rs | 532 | Pure state machine (10 phases, 3 templates) |
| runner.rs | 1,467 | Effect driver (spawn agents, run gates, commit) |
| workflow.rs | 143 | WorkflowRun metadata |
| config.rs | 63 | AcpConfig |
| lib.rs | 19 | Module facade |
| tests/protocol_conformance.rs | 393 | 8 integration tests |

### What works

- Full ACP 0.12.2 JSON-RPC protocol over stdio
- Editor integration (Cursor, Zed confirmed)
- 6 LLM providers, 22 model aliases
- 3 workflow templates (Express/Standard/Full) with auto-selection
- Gate pipeline (compile + test + clippy) with adaptive thresholds
- Session persistence to `.roko/sessions/`
- Real-time streaming (message chunks, thought chunks, tool calls, plan updates)
- 49 slash commands mapped to roko CLI
- 9 config options in editor status bar
- Architecture runner wired AcpAdapter + WorkflowEngine (phases 3.1, 4B)

### What does NOT work (subsystem isolation)

| System | Status in ACP | Status in orchestrate.rs |
|--------|---------------|--------------------------|
| Episode logging | Not connected | Full (per-turn) |
| CascadeRouter learning | Not connected | Full (persists decisions) |
| Safety contracts | Not connected | Full (role auth + checks) |
| SystemPromptBuilder (9-layer) | Not used (static strings) | Full |
| Playbook injection | Not connected | Full (queried at dispatch) |
| Knowledge/neuro queries | Not connected | Full |
| MCP tool routing | Declared but dead | Full passthrough |
| Budget enforcement | Dead code (always 0) | Full |
| Prompt experiments (A/B) | Not connected | Full |
| Efficiency events | Not connected | Full (per-turn) |

### acp-features spec coverage

| Category | Implemented | Specified | Coverage |
|----------|-------------|-----------|----------|
| Core Protocol | 10 | 10 | 100% |
| Provider System | 6 | 8 | 75% |
| Config Options | 5 | 10 | 50% |
| Slash Commands | 31 | 35 | 89% |
| Session Streaming | 6 | 8 | 75% |
| Modes | 3 | 5 | 60% |
| Conversation History | 1 | 4 | 25% |
| Context & Enrichment | 1 | 4 | 25% |
| Pipeline/Workflow | 3 | 9 scenarios | 33% |
| **Overall** | **66** | **96+** | **~69%** |
