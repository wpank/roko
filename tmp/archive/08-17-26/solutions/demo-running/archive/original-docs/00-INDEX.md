# Demo-Running: Getting Roko End-to-End

**Goal**: Make the demo-app + CLI pipeline work end-to-end, look clean, never crash, and be demo-ready.

## For Agents — How to Execute This

**Read [AGENT-GUIDE.md](AGENT-GUIDE.md) first.** It has the exact execution protocol, prompts, and parallel execution plan.

**Quick summary**:
1. Work is organized into 56 batches across 16 waves (W0-W15)
2. Batches within the same wave are independent — run them in parallel
3. Do NOT run `cargo build/test/clippy/fmt` per-batch — defer ALL compilation to the end
4. After ALL code changes are done, run a single compilation+fix phase
5. Each batch file has the exact agent prompt to use and commit instructions

**Execution rounds** (up to 20 agents per round):
- **Round 0** (W0): 7 batches — critical pipeline fixes
- **Round 1** (W1+W2+W3): 10 batches — pipeline E2E, output quality, terminal safety
- **Round 2** (W4): 3 batches — demo UI redesign (sequential)
- **Round 3** (W5): 4 batches — provider robustness
- **Round 4** (W6): 3 batches — config & boot cleanup
- **Round 5** (W7+W8): 7 batches — concurrency + code health
- **Round 6** (W9): 3 batches — **systemic pipeline quality** (highest ROI)
- **Round 7** (W10+W11): 9 batches — pipeline bugs + critical safety
- **Round 8** (W12+W13): 9 batches — runner architecture + speed/reliability
- **Round 9** (W14+W15): 9 batches — subsystem fixes + generalization
- **Phase 2**: Single agent — compilation + fix
- **Phase 3**: Single agent — demo app build

## Execution Phases

### Phase 1: Parallel Implementation (all code changes)
Spin up agents for each wave. All batches in a wave run in parallel. Move to the next wave only after the current wave's code changes are committed.

### Phase 2: Compilation + Fix (single agent)
After ALL Phase 1 code changes are done:
```bash
cargo +nightly fmt --all
cargo build --workspace 2>&1 | head -200  # fix errors iteratively
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```
Fix any compilation errors, type mismatches, or import issues that arise from parallel changes.

### Phase 3: Integration Test (single agent)
Manual verification of the demo pipeline end-to-end.

## Batch Files (Priority Order)

### Wave 0 — Critical Speed + Pipeline Fixes (7 batches; D first, then A+E parallel, then B+C+F+G parallel)
These fix the ROOT CAUSES of why the demo pipeline fails. Must be done first.

**Subwave 0a**: W0-D (routing) and W0-E (max_tokens) are independent, run in parallel.
**Subwave 0b**: W0-A (strip tools) depends on nothing. W0-F depends on W0-D.
**Subwave 0c**: W0-B, W0-C, W0-G can all run in parallel after 0a+0b.

| Batch | File | What | Est. |
|-------|------|------|------|
| W0-A | [batches/W0-A-strip-tools-prd-draft.md](batches/W0-A-strip-tools-prd-draft.md) | Strip tools from PRD/plan generation (4min → 30s) | 30m |
| W0-B | [batches/W0-B-plan-discovery-mismatch.md](batches/W0-B-plan-discovery-mismatch.md) | Fix plan discovery (prd plan writes tasks.toml, plan run looks for plan.md) | 30m |
| W0-C | [batches/W0-C-speed-optimizations.md](batches/W0-C-speed-optimizations.md) | Skip repo scan for empty workspaces, cap prompt size, better diagnostics | 1h |
| W0-D | [batches/W0-D-dispatch-routing-command-false.md](batches/W0-D-dispatch-routing-command-false.md) | Fix `roko run` dispatch when `command != "claude"` (Railway broken) | 1h |
| W0-E | [batches/W0-E-max-completion-tokens.md](batches/W0-E-max-completion-tokens.md) | Fix `max_tokens` vs `max_completion_tokens` for gpt54-mini | 15m |
| W0-F | [batches/W0-F-run-dispatch-parity.md](batches/W0-F-run-dispatch-parity.md) | Unify all 5 `roko run` dispatch paths (system prompt, tools, playbooks) | 1.5h |
| W0-G | [batches/W0-G-build-page-resilience.md](batches/W0-G-build-page-resilience.md) | BUILD page timeout increase, error detection, cancel button | 45m |

### Wave 1 — Pipeline Works E2E (3 batches, all parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W1-A | [batches/W1-A-plan-run-path.md](batches/W1-A-plan-run-path.md) | Fix `plan run` path resolution with `--repo` | 15m |
| W1-B | [batches/W1-B-prd-plan-extraction.md](batches/W1-B-prd-plan-extraction.md) | Fix `prd plan` silent extraction failure | 30m |
| W1-C | [batches/W1-C-plan-schema-unify.md](batches/W1-C-plan-schema-unify.md) | Unify plan schema parsing (validate vs run) | 1h |

### Wave 2 — Output Quality (5 batches, all parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W2-A | [batches/W2-A-tracing-to-file.md](batches/W2-A-tracing-to-file.md) | Route tracing to file, add `--verbose` flag | 1h |
| W2-B | [batches/W2-B-error-dedup.md](batches/W2-B-error-dedup.md) | Fix double-printed errors (80 eprintln! calls) | 1h |
| W2-C | [batches/W2-C-config-version-warn.md](batches/W2-C-config-version-warn.md) | Suppress false config version warning | 10m |
| W2-D | [batches/W2-D-spinners.md](batches/W2-D-spinners.md) | Add indicatif spinners for long operations | 1h |
| W2-E | [batches/W2-E-negative-cost.md](batches/W2-E-negative-cost.md) | Fix negative cost display | 5m |

### Wave 3 — Terminal Safety (2 batches, all parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W3-A | [batches/W3-A-ctrl-c-phases.md](batches/W3-A-ctrl-c-phases.md) | Add Ctrl+C handler to all chat phases | 20m |
| W3-B | [batches/W3-B-panic-hook.md](batches/W3-B-panic-hook.md) | Panic hook for terminal restore | 10m |

### Wave 4 — Demo UI Redesign (3 batches, sequential: A → B → C)
| Batch | File | What | Est. |
|-------|------|------|------|
| W4-A | [batches/W4-A-clickable-scenario-type.md](batches/W4-A-clickable-scenario-type.md) | Add ClickableScenario type + CommandList component | 2h |
| W4-B | [batches/W4-B-context-panel.md](batches/W4-B-context-panel.md) | Add ContextPanel component | 1h |
| W4-C | [batches/W4-C-prd-pipeline-redesign.md](batches/W4-C-prd-pipeline-redesign.md) | Refactor PRD pipeline to click-to-run | 2h |

### Wave 5 — Provider & Config Robustness (4 batches, all parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W5-A | [batches/W5-A-auth-detect-config.md](batches/W5-A-auth-detect-config.md) | Auth detection uses unified config | 1h |
| W5-B | [batches/W5-B-acp-startup.md](batches/W5-B-acp-startup.md) | ACP workspace auto-creation + log fallback | 20m |
| W5-C | [batches/W5-C-provider-preflight.md](batches/W5-C-provider-preflight.md) | Provider binary + API key pre-flight at boot | 45m |
| W5-D | [batches/W5-D-cat-fallback-refuse.md](batches/W5-D-cat-fallback-refuse.md) | Refuse silent fallback to cat agent | 10m |

### Wave 6 — Config & Boot Cleanup (3 batches; A first, then B+C parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W6-A | [batches/W6-A-config-unify.md](batches/W6-A-config-unify.md) | Unify 11 load_roko_config functions → 1 | 2h |
| W6-B | [batches/W6-B-file-locking.md](batches/W6-B-file-locking.md) | Multi-process workspace file locking | 45m |
| W6-C | [batches/W6-C-boot-sequence.md](batches/W6-C-boot-sequence.md) | RokoBootstrap struct for unified startup | 2h |

### Wave 7 — Concurrency Fixes (3 batches, all parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W7-A | [batches/W7-A-sync-mutex-serve.md](batches/W7-A-sync-mutex-serve.md) | Replace parking_lot::Mutex with tokio in serve | 20m |
| W7-B | [batches/W7-B-cancel-notify.md](batches/W7-B-cancel-notify.md) | Replace polling cancellation with Notify | 20m |
| W7-C | [batches/W7-C-playbook-locks.md](batches/W7-C-playbook-locks.md) | Fix nested async lock in PlaybookStore | 30m |

### Wave 8 — Code Health (4 batches, all parallel)
| Batch | File | What | Est. |
|-------|------|------|------|
| W8-A | [batches/W8-A-clippy-blanket.md](batches/W8-A-clippy-blanket.md) | Remove blanket clippy suppression in main.rs | 2h |
| W8-B | [batches/W8-B-rust-toolchain.md](batches/W8-B-rust-toolchain.md) | Add rust-toolchain.toml pinning 1.91+ | 2m |
| W8-C | [batches/W8-C-enrichment-backend-removal.md](batches/W8-C-enrichment-backend-removal.md) | Remove resolve_enrichment_backend() substring heuristic | 30m |
| W8-D | [batches/W8-D-toctou-fixes.md](batches/W8-D-toctou-fixes.md) | Fix TOCTOU race conditions (exists+read pattern) | 1h |

### Wave 9 — Systemic Pipeline Quality (3 batches, all parallel) — HIGHEST ROI
Fixes THE root cause of poor pipeline output. ImplementerTemplate wiring, PRD injection, cross-task awareness, cost tracking.

| Batch | File | What | Est. |
|-------|------|------|------|
| W9-A | [batches/W9-A-wire-implementer-template.md](batches/W9-A-wire-implementer-template.md) | Wire ImplementerTemplate to dispatch + inject PRD excerpt | 3h |
| W9-B | [batches/W9-B-cross-task-output.md](batches/W9-B-cross-task-output.md) | Cross-task output injection (T2 sees T1's files) | 2h |
| W9-C | [batches/W9-C-cost-tracking.md](batches/W9-C-cost-tracking.md) | Fix cost tracking: model_profile → ToolLoop | 15m |

### Wave 10 — Pipeline Bug Fixes (5 batches, all parallel)
Data path bugs found in real pipeline run analysis (gpt54-mini, BTC Funding Alert CLI).

| Batch | File | What | Est. |
|-------|------|------|------|
| W10-A | [batches/W10-A-pipeline-quick-fixes.md](batches/W10-A-pipeline-quick-fixes.md) | 8 quick fixes: JSON leak, model_hint, dream path, sentinels, INDEX, slug, mcp examples, workdir | 1.5h |
| W10-B | [batches/W10-B-episode-data-wiring.md](batches/W10-B-episode-data-wiring.md) | Episode data zeros + gate verdict substrate writes | 2h |
| W10-C | [batches/W10-C-memory-learn-unify.md](batches/W10-C-memory-learn-unify.md) | Unify .roko/memory/ and .roko/learn/, cascade router slugs, INDEX, PRD update | 2h |
| W10-D | [batches/W10-D-scaffold-and-gates.md](batches/W10-D-scaffold-and-gates.md) | Scaffold with inter-crate deps, max_loc gate, threshold schema, git commits | 3h |
| W10-E | [batches/W10-E-plan-prompt-quality.md](batches/W10-E-plan-prompt-quality.md) | Playbook ID fix, plan.md generation, PRD types in prompts, config version | 2h |

### Wave 11 — Critical Bugs (4 batches, all parallel)
Crash, hang, and injection prevention.

| Batch | File | What | Est. |
|-------|------|------|------|
| W11-A | [batches/W11-A-gate-channel-and-fatal.md](batches/W11-A-gate-channel-and-fatal.md) | Gate channel send fallback + Fatal event fallback | 1h |
| W11-B | [batches/W11-B-unwrap-lock-safety.md](batches/W11-B-unwrap-lock-safety.md) | Chain client unwrap + lock poisoning fix | 30m |
| W11-C | [batches/W11-C-config-validation.md](batches/W11-C-config-validation.md) | Config reference validation on load + synthesized profile fix | 30m |
| W11-D | [batches/W11-D-shell-injection.md](batches/W11-D-shell-injection.md) | Shell injection in demo terminal (shellQuote) | 10m |

### Wave 12 — Runner Architecture (4 batches, all parallel)
Event loop restructuring for correctness and concurrency.

| Batch | File | What | Est. |
|-------|------|------|------|
| W12-A | [batches/W12-A-gate-semaphore-per-run.md](batches/W12-A-gate-semaphore-per-run.md) | Global gate semaphore → per-run | 1h |
| W12-B | [batches/W12-B-multi-agent-handle.md](batches/W12-B-multi-agent-handle.md) | Single agent_handle → per-plan HashMap + FailPlan fix + iteration fix | 2h |
| W12-C | [batches/W12-C-event-loop-safety.md](batches/W12-C-event-loop-safety.md) | DAG sort, agent_output cap, epoch timestamp, timeout guard, hook role | 2h |
| W12-D | [batches/W12-D-runner-config-fixes.md](batches/W12-D-runner-config-fixes.md) | MCP wiring, dream logic, Permanent retryable, budget enforcement, feedback bound | 1.5h |

### Wave 13 — Speed & Reliability (5 batches, all parallel)
Performance optimization and safety mechanisms.

| Batch | File | What | Est. |
|-------|------|------|------|
| W13-A | [batches/W13-A-toml-repair-pipeline.md](batches/W13-A-toml-repair-pipeline.md) | Deterministic TOML repair (eliminates 80% of LLM retries) | 2h |
| W13-B | [batches/W13-B-cache-warm-and-gates.md](batches/W13-B-cache-warm-and-gates.md) | Warm cargo cache + dynamic gate channel buffer | 1h |
| W13-C | [batches/W13-C-connection-pooling.md](batches/W13-C-connection-pooling.md) | Connection pooling docs/observability (already implemented) | 15m |
| W13-D | [batches/W13-D-atomic-state-writes.md](batches/W13-D-atomic-state-writes.md) | Checkpoint file for crash recovery + --fresh expansion | 1h |
| W13-E | [batches/W13-E-error-taxonomy.md](batches/W13-E-error-taxonomy.md) | Error classification, schema validation, scaffold safety | 2h |

### Wave 14 — Subsystem Fixes (4 batches, all parallel)
Compose, serve, learning, config architecture improvements.

| Batch | File | What | Est. |
|-------|------|------|------|
| W14-A | [batches/W14-A-compose-sections.md](batches/W14-A-compose-sections.md) | Budget caps, O(N²) fix, SectionSpec table, DRY, budget conflicts | 3h |
| W14-B | [batches/W14-B-serve-fixes.md](batches/W14-B-serve-fixes.md) | Health status codes, RwLock, SSE, WS, lock ordering | 2h |
| W14-C | [batches/W14-C-learning-fixes.md](batches/W14-C-learning-fixes.md) | LinUCB persistence, nested mutex, episode IDs, CostsLog, scoring | 3h |
| W14-D | [batches/W14-D-config-fixes.md](batches/W14-D-config-fixes.md) | ROKO__* docs, deprecated loader, global merge, diagnostics, interpolation | 2h |

### Wave 15 — Prompt, Design, Code Health, Generalization (5 batches, all parallel)
Extensibility, testability, and code quality improvements.

| Batch | File | What | Est. |
|-------|------|------|------|
| W15-A | [batches/W15-A-prompt-quality.md](batches/W15-A-prompt-quality.md) | Workspace context, model_hint removal, recovery guidance, few-shot, role-tools | 2h |
| W15-B | [batches/W15-B-design-patterns.md](batches/W15-B-design-patterns.md) | dispatch_and_record helper, log errors, SafetyLayer, output sinks, env warnings | 3h |
| W15-C | [batches/W15-C-code-health.md](batches/W15-C-code-health.md) | unwrap replacements, hardcoded models → defaults, TimeoutConfig | 4h |
| W15-D | [batches/W15-D-demo-app.md](batches/W15-D-demo-app.md) | TimeoutConfig, CommandFailureReason, command templates, AbortController | 2h |
| W15-E | [batches/W15-E-generalization.md](batches/W15-E-generalization.md) | Data-driven gate rungs, Workspace abstraction, AdaptiveBudget | 4h |

## What's Already Done (DO NOT re-implement)

See [DONE.md](DONE.md) for the full list of completed work (47 batches, B10/B11 resolved, provider synthesis removal, learning feedback wiring, safety boundaries, etc.).

## Supporting Documents

| File | Purpose |
|------|---------|
| [STATUS.md](STATUS.md) | **Current status** — what's fixed, what's broken, latest demo run, open issues (START HERE) |
| [AGENT-GUIDE.md](AGENT-GUIDE.md) | Execution protocol, agent prompts, parallel plan |
| [DONE.md](DONE.md) | Completed work — do not re-implement |
| [01-BLOCKERS.md](01-BLOCKERS.md) | Original blocker analysis with root causes |
| [04-DEMO-UI-REDESIGN.md](04-DEMO-UI-REDESIGN.md) | Full UI redesign spec (layout, components, interactions) |
| [05-REMAINING-AUDIT.md](05-REMAINING-AUDIT.md) | Complete inventory of all open audit items |
| [06-STREAMING-DESIGN.md](06-STREAMING-DESIGN.md) | Streaming stderr output design for `plan run` (session 5) |
| [PIPELINE-RUN-AUDIT.md](PIPELINE-RUN-AUDIT.md) | Full audit of 5-pipeline run (2026-05-04) with cross-cutting issues |
| [TERMINAL-SESSION-REDESIGN.md](TERMINAL-SESSION-REDESIGN.md) | Terminal session layer diagnosis and fixes |
| [IMPROVEMENTS.md](IMPROVEMENTS.md) | **Improvement recommendations** — 75 items across 13 categories: critical bugs, runner architecture, compose/templates, serve/SSE/WS, learning, config, speed, reliability, prompts, design patterns, code health, demo app, generalization. All with mechanical implementation steps, file paths, and line numbers. |
