# Feedback Item 2: CLI UX, Tool Dispatch, Image Support, Provider Gaps

Comprehensive findings from dogfooding roko in Zed with multiple providers.
Covers CLI workflow gaps, broken tool dispatch, image support, Gemini CLI backend,
provider error UX, test coverage gaps, and full system audit.

## File Index

| File | What |
|------|------|
| [01-IMAGE-SUPPORT.md](01-IMAGE-SUPPORT.md) | Image/vision support across ACP, CLI, HTTP |
| [02-GEMINI-CLI-BACKEND.md](02-GEMINI-CLI-BACKEND.md) | Adding Gemini CLI as agent backend |
| [03-PROVIDER-ERROR-UX.md](03-PROVIDER-ERROR-UX.md) | ANTHROPIC_API_KEY warnings, provider-biased errors |
| [04-TOOL-DISPATCH-BROKEN.md](04-TOOL-DISPATCH-BROKEN.md) | Tool use broken in research, partial in other paths |
| [05-CLI-WORKFLOW-GAPS.md](05-CLI-WORKFLOW-GAPS.md) | Missing pieces in note→plan→refine→execute pipeline |
| [06-SELF-DEV-UX-STATUS.md](06-SELF-DEV-UX-STATUS.md) | Status of all 23 self-developing UX docs |
| [07-TEST-COVERAGE-GAPS.md](07-TEST-COVERAGE-GAPS.md) | Tool/model/provider test matrix and gaps |
| [08-PLAN-EXECUTION-BROKEN.md](08-PLAN-EXECUTION-BROKEN.md) | Graph Engine no-op, Runner v2 gated, preflight skip, crate name bug, TUI agent display |
| [09-TUI-PANELS-BROKEN.md](09-TUI-PANELS-BROKEN.md) | Output, efficiency, diagnosis panels disconnected — push/pull mode mismatch |
| [10-FULL-SYSTEM-AUDIT.md](10-FULL-SYSTEM-AUDIT.md) | **Complete audit**: all 45 CLI commands, all TUI tabs, event pipeline, snapshot parity, orchestrate.rs subsystems |
| [11-POST-EXECUTION-AUDIT-LOOP.md](11-POST-EXECUTION-AUDIT-LOOP.md) | Auto-audit after plan run: dispatch reviewers, generate fix tasks, iterate until clean |
| [12-CLI-OUTPUT-PRETTIFICATION.md](12-CLI-OUTPUT-PRETTIFICATION.md) | Pretty CLI output: suppress log noise, wire output_format primitives, add spinners + colors |
| [13-SEARCH-COMMAND-BROKEN.md](13-SEARCH-COMMAND-BROKEN.md) | `/search` 100% broken — sends batch format but Perplexity expects flat query |
| [14-PRD-LIST-MISSING-SLUGS.md](14-PRD-LIST-MISSING-SLUGS.md) | `/prd-list` doesn't show slugs — **fixed**: slugs shown + actionable hints added |
| [15-ANALYZE-NO-TOOLS-DISPATCHED.md](15-ANALYZE-NO-TOOLS-DISPATCHED.md) | `/analyze` (and all research cmds) broken on non-Claude models — tool alias mismatch, zero tools dispatched |
| [16-PRD-DRAFT-BROKEN-AND-REDUNDANT.md](16-PRD-DRAFT-BROKEN-AND-REDUNDANT.md) | `/prd-draft` broken: no tools, no idea linkage, validation non-blocking. Use `/do` or `/plan-generate` instead. |
| [17-PRD-STATUS-DISCONNECTED.md](17-PRD-STATUS-DISCONNECTED.md) | `/prd-status` per-PRD columns always `—` — no plan↔PRD linkage |
| [18-ACP-TOOL-PERMISSION-GATE.md](18-ACP-TOOL-PERMISSION-GATE.md) | ACP has no permission gate for destructive tools (Write, Bash). No confirmation UX. |
| [19-CASCADE-ROUTER-NOT-WIRED-IN-ACP.md](19-CASCADE-ROUTER-NOT-WIRED-IN-ACP.md) | CascadeRouter (LinUCB) not consulted in ACP — no adaptive model selection. Observation key mismatch. DaimonState always default(). |
| [20-ERROR-RECOVERY-NOT-WIRED.md](20-ERROR-RECOVERY-NOT-WIRED.md) | `classify_agent_crash()` + `recovery_hint()` built but never called. Silent config fallback. Replan not in runner v2. |
| [21-PLAN-GENERATION-BUGS.md](21-PLAN-GENERATION-BUGS.md) | Graph Engine stub is default path. `develop` double-generates. No TOML validation on generated plans. |
| [22-PROVIDER-RATE-LIMIT-RETRY-BROKEN.md](22-PROVIDER-RATE-LIMIT-RETRY-BROKEN.md) | All HTTP errors → `LlmError::Network`, retry loop requires `LlmError::Provider`. 429s never retried. Gemini no streaming. |
| [23-GATE-RUNGS-3-6-NEVER-SELECTED.md](23-GATE-RUNGS-3-6-NEVER-SELECTED.md) | Both branches of `enable_advanced_rungs` return same values (0-2). Rungs 3-6 unreachable. VerifyChainGate always stub. |
| [24-SAFETY-CONTRACTS-NEVER-LOADED.md](24-SAFETY-CONTRACTS-NEVER-LOADED.md) | `AgentContract::permissive()` always used. No YAML files exist. Bash denylist can't intercept Claude CLI subprocess. |
| [25-TUI-AGENT-DATA-GAPS.md](25-TUI-AGENT-DATA-GAPS.md) | AgentOutput discarded at snapshot boundary. `current_task` never populated. No Diagnosis events from runner. |
| [26-SLASH-COMMAND-BUGS.md](26-SLASH-COMMAND-BUGS.md) | `/plan-resume` wrong flag (`--resume` vs `--resume-plan`). `/plan-run` no `--model`. `/develop` not wired. 6 commands broken by tool alias. |
| [27-RAW-EPRINTLN-ACROSS-COMMANDS.md](27-RAW-EPRINTLN-ACROSS-COMMANDS.md) | 147 raw `eprintln!` calls. output_format primitives exist but only used by run.rs. No spinners, no colors, no `--quiet`. |
| [28-KNOWLEDGE-SUBSYSTEM-AUDIT.md](28-KNOWLEDGE-SUBSYSTEM-AUDIT.md) | Neuro store works. Cold archive works (CLAUDE.md outdated). HDC fingerprints computed but never queried for similar-task lookup. |
| [29-PLAN-RUNNER-V2-PARALLELISM.md](29-PLAN-RUNNER-V2-PARALLELISM.md) | `max_parallel` from tasks.toml ignored. Runner v2 runs one agent at a time. DagExecutor supports parallelism but isn't used. |
| [30-WORKSPACE-PATH-CONFLICTS.md](30-WORKSPACE-PATH-CONFLICTS.md) | `plans/` location conflict (plan.rs vs main.rs). Session path mismatch. 11 orphaned .tmp files. |
| [31-MCP-PASSTHROUGH-GAPS.md](31-MCP-PASSTHROUGH-GAPS.md) | ACP chat drops MCP servers. AgentConfig missing `mcp_config` field. Auto-discovery not run in ACP. |
| [32-ACP-STREAMING-GAPS.md](32-ACP-STREAMING-GAPS.md) | Most slash commands buffer output (blank panel for 30s). Tool calls inside subprocess invisible to ACP. |
| [33-CONFIG-ZERO-CONFIG-BLOCKED.md](33-CONFIG-ZERO-CONFIG-BLOCKED.md) | Preflight doesn't consult builtin model registry. 7 duplicate slug warnings on every command. API key auto-detect missing. |
| [34-LEARNING-PIPELINE-ACP-GAPS.md](34-LEARNING-PIPELINE-ACP-GAPS.md) | Distillation/dream/efficiency now wired in ACP. Router selection, DaimonState, experiments still CLI-only. |
| [35-DAEMON-DEPLOYMENT-STATUS.md](35-DAEMON-DEPLOYMENT-STATUS.md) | Daemon + Railway + Fly working. Docker missing push step. Worker mode works. |

## Audit Summary (doc 10 + docs 18-35)

| Area | Verdict |
|------|---------|
| CLI Commands (45) | **30 working, 3 broken, 6 partial** (tool alias), 1 missing (`/develop`) |
| TUI Panels (~30) | **~75% working** — 5 broken in push mode, 4 stubs, agent data gaps |
| Event Pipeline (52 call sites) | **~80% wired** — 5 event types never published |
| DashboardSnapshot (22 fields) | **12 gaps** vs DashboardData |
| orchestrate.rs (19 subsystems) | **17/19 wired** — VCG dead, safety permissive |
| ACP Learning Pipeline | **~50% wired** — episodes+distill+dream yes, router+daimon+experiments no |
| Provider Backends | **Rate limit retry broken** — 429→Network, never retried |
| Gate Pipeline | **Rungs 0-2 only** — advanced rungs unreachable even with config |
| Safety Layer | **Permissive default** — contracts never loaded, bash filter unwired |
| Plan Execution | **Sequential only** — max_parallel ignored |

## Priority Summary

### P0 — Blocks core functionality
1. **Graph Engine is default but is a no-op** (docs 08, 21) — feature flag `legacy-runner-v2` required for actual execution
2. **`/search` command 100% broken** (doc 13) — wrong API format for Perplexity
3. **Tool alias mismatch** (doc 15) — zero tools on non-Claude models for all research commands
4. **`/plan-resume` wrong flag** (doc 26) — `--resume` vs `--resume-plan`, plan restarts from scratch
5. **Tool dispatch broken in research** (doc 04) — raw JSON tool calls, no loop

### P1 — Degrades daily experience
6. **ACP streaming** (doc 32) — blank panel for 30s during long commands
7. **Rate limit retry broken** (doc 22) — single 429 kills entire plan execution
8. **Raw eprintln! everywhere** (doc 27) — 147 calls, output_format primitives unused
9. **ACP tool permission gate** (doc 18) — no confirmation for file writes or bash
10. **Cascade router not in ACP** (doc 19) — no adaptive model selection for 50%+ of usage
11. **Safety contracts permissive** (doc 24) — every agent can do anything
12. **Gate rungs 3-6 unreachable** (doc 23) — advanced validation broken
13. **TUI agent data gaps** (docs 09, 25) — output discarded, tasks empty, diagnosis missing
14. **Error recovery not wired** (doc 20) — crash classification built but never called
15. **Runner v2 parallelism** (doc 29) — max_parallel ignored, sequential only
16. **Zero-config blocked** (doc 33) — builtin registry not consulted in preflight
17. **Learning pipeline ACP gaps** (doc 34) — router/daimon/experiments CLI-only
18. **ANTHROPIC_API_KEY warnings** (doc 03) — unconditional, provider-biased
19. **Image support** (doc 01) — ACP hardcodes `image: false`
20. **CLI output prettification** (doc 12) — no spinners, no colors

### P2 — Broken workflows / missing features
21. **`/prd-draft` broken** (doc 16) — zero tools, no idea linkage, non-blocking validation
22. **`/prd-status` disconnected** (doc 17) — per-PRD columns always `—`
23. **MCP passthrough gaps** (doc 31) — ACP chat drops MCP servers
24. **Workspace path conflicts** (doc 30) — plans/ directory ambiguity
25. **Knowledge HDC lookup** (doc 28) — fingerprints computed but never queried
26. **Post-execution audit loop** (doc 11) — auto-audit after plan run
27. **Docker push missing** (doc 35) — `--push` flag defined but not wired
28. **Gemini CLI backend** (doc 02)
29. **Context sources / plan refinement** (doc 05)
30. **Cross-provider test matrix** (doc 07)

### Known Dead Code
- **VCG auction** — built in roko-core, zero references in orchestrate.rs
- **DashboardSnapshot.errors** ring buffer — populated, never read by TUI
- **DashboardSnapshot.agent_topology** — read by TUI, never populated by any event

### Quick Wins (< 10 min each)
1. `/plan-resume` flag fix: `--resume` → `--resume-plan` (doc 26, 2 min)
2. `/plan-run` model passthrough: add `--model` arg (doc 26, 5 min)
3. Config parse warning: warn on roko.toml syntax error (doc 20, 5 min)
4. VerifyChainGate warning log: mark as Phase 2 stub (doc 23, 2 min)
5. Gate rung selection fix: use 3-6 when advanced enabled (doc 23, 5 min)
6. Docker push wire: connect `--push` flag (doc 35, 5 min)
