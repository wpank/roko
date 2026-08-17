# Task Checklist

Status: `[ ]` not started, `[~]` partial/in-progress, `[x]` done

All items have full technical context in [01-BLOCKERS.md](01-BLOCKERS.md) and [05-REMAINING-AUDIT.md](05-REMAINING-AUDIT.md).

---

## Wave 1: Pipeline Path Fixes — UNBLOCKS DEMO (1-2h)

- [x] **1.1** Fix `plan run` plans_dir resolution with `--repo`
  - `plan.rs:220-227` — resolved plans_dir relative to workdir BEFORE validate_before_run()
  - Also fixed validate_before_run() to accept workdir param instead of using CWD
  - Verified: pipeline passes with `--repo /tmp/workspace`

- [x] **1.2** Fix `prd plan` silent extraction failure
  - Tools stripped from plan-generation dispatch (W0-A)
  - Post-gen TOML validation: `validate_and_fix_generated_plan()` (R2)
  - Verified: `prd plan` consistently produces tasks.toml

- [~] **1.3** Unify plan schema parsing (validate vs run)
  - `plan validate` and `plan run` now both use `TasksFile::parse()` from `task_parser.rs`
  - `plan_loader.rs` calls `TasksFile::parse()` directly
  - Remaining: some edge cases where validate is more lenient than run

- [x] **1.4** Demo scenario consistent --repo usage
  - `roko_cmd()` in dev.sh always passes `--repo $PIPELINE_WORKSPACE`
  - Terminal session uses `--repo` consistently
  - Verified: pipeline completes all 7 steps

---

## Wave 2: Output Quality — MAKES DEMO PRESENTABLE (2-3h)

- [x] **2.1** Route tracing to file by default
  - Was already implemented — tracing goes to `.roko/roko.log` by default
  - stderr only with `--verbose` or `RUST_LOG`

- [x] **2.2** Suppress false config version warning
  - Config loader checks explicit config_version text, not [providers] table presence
  - Fixed in session 4 batch

- [x] **2.3** Fix error deduplication
  - Removed duplicate eprintln from dry_run_fs.rs, event_loop.rs
  - Fixed in session 4 batch

- [x] **2.4** Add spinners for long operations → replaced with line output (W2-D)
  - Line-based progress output instead of spinners (better for CI/logging)

- [x] **2.5** Fix negative cost display
  - `.max(0.0)` in costs_log.rs, cli_progress.rs, learn.rs
  - Fixed in session 4 batch

---

## Wave 3: Terminal Safety — PREVENTS CRASHES (1h) — DONE

- [x] **3.1** Ctrl+C in all chat phases — W3
- [x] **3.2** RAII terminal cleanup guard — RawModeGuard with Drop, session 4
- [x] **3.3** Panic hook for terminal restore — session 4

---

## Wave 4: Demo UI Redesign (3-5h) — DONE

- [x] **4.1-4.5** Demo UI redesign with clickable scenarios (committed in W4)

---

## Wave 5: Provider & Config Quality (3-5h)

- [x] **5.1** Startup provider validation → `preflight_provider_for_model()` in plan.rs, agent.rs, prd.rs
- [x] **5.2** Auth detection uses config (W5-A)
- [x] **5.3** ACP workspace auto-creation (W5-B)
- [x] **5.4** ACP log file fallback (W5-B)
- [x] **5.5** ACP JSON-RPC error on startup failure (W5-B)
- [ ] **5.6** Home dir workspace collision — warn when CWD=~ (not in batch scope)

---

## Wave 6: Remaining P0 Items

- [x] **6.1** Unified config loading (W6-A, W14-D — deprecated loaders delegate to unified)
- [x] **6.2** Multi-process file locking — `acquire_workspace_lock()` added in plan.rs
- [x] **6.3** `prd plan` — handle case where agent uses write_file — tools stripped (W0-A)

---

## Pipeline-Specific Fixes (Sessions 2-4)

- [x] **P.1** Targeted provider preflight (plan.rs, agent.rs, prd.rs)
- [x] **P.2** scaffold_missing_crates() in plan_loader.rs (crate dirs + workspace Cargo.toml)
- [x] **P.3** Per-task failure reasons in run summary
- [x] **P.4** Gate skip for greenfield workspaces (no Cargo.toml → skip rung 0-2)
- [x] **P.5** Read-only role gate bypass (researcher/strategist/quick-reviewer auto-pass gates)
- [x] **P.6** Plan generation prompt: concrete files, role-appropriate verify, no placeholders
- [x] **P.7** Post-gen TOML validation (typos, model hints, slug, placeholders)
- [x] **P.8** Config propagation to ephemeral workspaces (server-side)
- [x] **P.9** --model injection in terminal session
- [x] **P.10** Deadlock fix: gate auto-pass via tokio::spawn (not inline send)
- [x] **P.11** dev.sh SIGKILL escalation on timeout
- [x] **P.12** dev.sh progress indicator during long steps

---

## Session 5: Model Quality + Streaming (uncommitted)

- [x] **S5.1** Model hint fallback to default
  - When model_hint doesn't resolve to a known provider model, fall back to default instead of erroring
- [x] **S5.2** TOML validation expanded (19 new typos, status/role validation)
  - Added 19 common LLM typo corrections (e.g. `implmentation`, `dependecies`)
  - Status and role values validated against known enums
- [x] **S5.3** Plan gen prompt: never set model_hint
  - Prompt explicitly instructs LLM to omit the model_hint field
- [x] **S5.4** Cost tracking wired in dispatch_v2
  - `fill_cost_from_profile()` called at 3 dispatch sites
- [x] **S5.5** Research tools added (Read,Write,Edit)
  - All 5 research subcommands pass `allowed_tools: Some("Read,Write,Edit")`
- [x] **S5.6** TOML retry logic (2 retries)
  - On parse failure, retries up to 2 times with progressively stricter prompt
- [x] **S5.7** Timing instrumentation
  - Per-step timing in pipeline execution
- [~] **S5.8** Streaming output for plan run
  - RunOutputSink trait created (W15-B) with StderrSink and NoopSink implementations
  - Wiring into agent_events.rs is follow-up (trait + impls ready)
- [x] **S5.9** Health check false offline fix (W14-B — try_read + 503 on down)
- [x] **S5.10** Task agent prompt improvements (W15-A — workspace_context() injected + failure recovery)

---

## Session 6: Deep Architecture Audit — 75 Items

Full audit across 5 subsystems (runner, compose, serve, learning, config).
See [IMPROVEMENTS.md](IMPROVEMENTS.md) for all items with mechanical steps.

### Critical (fix immediately)
- [x] **S6.1** MCP config never wired to `plan run` (W10-A — config passthrough wired)
- [ ] **S6.2** Dream consolidation inverted logic — 120s CI delay (not in batch scope)
- [x] **S6.3** Health endpoint HTTP 200 on "down" status (W14-B — returns 503)
- [x] **S6.4** `RunnerFailureKind::Permanent` retryable (W10-A — fixed)
- [x] **S6.5** `Fatal` event result swallowed — plans hang (W11-A — gate channel fallback)

### High (before next demo)
- [x] **S6.6** LinUCB state not persisted — routing resets on restart (W14-C — LinUCBSnapshot added)
- [x] **S6.7** Global gate semaphore singleton → per-run (W12-A — per-run semaphore)
- [x] **S6.8** Single agent_handle → per-plan map (W12-B — per-plan agent handles)
- [ ] **S6.9** Per-turn budget enforcement (not in batch scope — warning-only currently)
- [x] **S6.10** `ROKO__*` env override: implement or remove doc (W14-D — doc corrected)

### Medium (next sprint)
- [x] **S6.11** FailPlan → wrong plan attribution (W12-C — fixed in event loop)
- [x] **S6.12** Sentinel task sorts by string not DAG (W12-C — fixed)
- [x] **S6.13** agent_output unbounded growth (W12-C — bounded)
- [x] **S6.14** Plan timeout fires twice (W12-C — deduplicated)
- [x] **S6.15** Section budget caps: 5/11 → 11/11 covered (W14-A — all sections capped)
- [x] **S6.16** Blocking RwLock in relay_health (W14-B — uses try_read)
- [x] **S6.17** Nested mutex in cascade_router (W14-C — 3-phase sequential locking)
- [x] **S6.18** Deprecated config loader divergence (W14-D — delegates to unified loader)
- [ ] **S6.19** Feedback facade unbounded tasks (not in batch scope)
- [x] **S6.20** 13 RwLock maps, no lock ordering (W14-B — lock-ordering comment block)

---

## Session 7: Pipeline Run Analysis (gpt54-mini, BTC Funding Alert CLI)

Real end-to-end run: 4/4 tasks, 161s. Code quality gaps identified.

### Tier 0 — Systemic (highest ROI)
- [x] **S7.1** Wire ImplementerTemplate to runtime dispatch (W9-A — template wired)
- [ ] **S7.2** Inject PRD excerpt into implementer prompts (not in batch scope — follows from template wiring)
- [x] **S7.3** Cost tracking: pass model_profile to ToolLoop (W9-C — wired)
- [x] **S7.4** Cross-task output injection (W9-B — predecessor output in prompts)

### Tier 1 — Pipeline Bugs
- [ ] **S7.5** Dream path `.roko/.roko/` double-nesting (not in batch scope)
- [x] **S7.6** Episode data all zeros (W10-B — episode data wired)
- [ ] **S7.7** Gate verdicts not written to substrate (not in batch scope)
- [ ] **S7.8** Unconditionally strip model_hint (W15-A removed method; field stays for user override)
- [ ] **S7.9** Suppress JSON tool_uses leak in prd plan (not in batch scope)
- [x] **S7.10** Unify `.roko/memory/` and `.roko/learn/` (W10-C — unified)

### Tier 2 — Quality
- [x] **S7.11** Scaffold Cargo.toml with inter-crate deps (W10-D — scaffold enhanced)
- [ ] **S7.12** max_loc gate enforcement (not in batch scope — advisory only)
- [ ] **S7.13** Gate rung sentinel constants (not in batch scope)
- [ ] **S7.14** Gate threshold schema mismatch (not in batch scope)
- [x] **S7.15** Playbook ID mismatch fix (W10-E — queries by real playbook IDs)
- [ ] **S7.16** Cascade router auto-register slugs (not in batch scope)
- [ ] **S7.17** INDEX.md path fixes (not in batch scope)
- [ ] **S7.18** Git commit after task gates pass (not in batch scope)
- [ ] **S7.19** Inject slug into plan prompt (not in batch scope)
- [ ] **S7.20** Remove mcp_servers from examples (not in batch scope)

---

## Wave 7-9: Lower Priority Items

See [05-REMAINING-AUDIT.md](05-REMAINING-AUDIT.md) for full inventory.
Most items are P1-P3 and not blocking the demo pipeline.
