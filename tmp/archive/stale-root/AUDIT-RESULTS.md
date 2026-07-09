# Parity Audit Results (2026-04-10)

Systematic code audit of all 27 MASTER-PLAN sections.
Verdict per item: **DONE** (real code, wired) | **STUB** (exists but incomplete) | **MISSING** (not implemented)

## Score Summary

| Tier | Section | Done | Stub | Missing | Total | % Done |
|------|---------|------|------|---------|-------|--------|
| 1 | 1A Executor Phases | 21 | 1 | 0 | 22 | 95% |
| 1 | 1B Conductor Watchers | 15 | 0 | 0 | 15 | 100% |
| 1 | 1C MCP Tool Registry | 10 | 2 | 0 | 12 | 83% |
| 1 | 1D Observability | 10 | 1 | 0 | 11 | 91% |
| 1 | 1E Re-Planning | 10 | 0 | 0 | 10 | 100% |
| 1 | 1F Auto Plan Generation | 9 | 2 | 0 | 11 | 82% |
| 1 | 1G Gate/Learn/API | 13 | 0 | 0 | 13 | 100% |
| 1 | 1H TUI Dashboard | 14 | 6 | 0 | 20 | 70% |
| 1 | 1I Skill/Playbook | 7 | 1 | 0 | 8 | 88% |
| 1 | 1J LinUCB Bandit | 11 | 0 | 0 | 11 | 100% |
| 2 | 2A roko-serve | 9 | 0 | 0 | 9 | 100% |
| 2 | 2B roko-plugin SDK | 11 | 0 | 0 | 11 | 100% |
| 2 | 2C Webhooks+Dispatch | 21 | 0 | 0 | 21 | 100% |
| 2 | 2D MCP Servers | 37 | 0 | 0 | 37 | 100% |
| 3 | 3A Agent Templates | 7 | 2 | 0 | 9 | 78% |
| 3 | 3B Subscriptions | 5 | 2 | 0 | 7 | 71% |
| 3 | 3C Cron/FileWatch | 10 | 1 | 0 | 11 | 91% |
| 4 | 4A Daemon Mode | 4 | 5 | 0 | 9 | 44% |
| 4 | 4B Multi-Repo | 1 | 5 | 0 | 6 | 17% |
| 4 | 4C Cloud Deploy | 3 | 9 | 0 | 12 | 25% |
| 4 | 4D Secrets | 4 | 3 | 0 | 7 | 57% |
| 5 | 5A roko-neuro | 14 | 0 | 3 | 17 | 82% |
| 5 | 5B Context Assembly | 9 | 0 | 0 | 9 | 100% |
| 5 | 5C Daimon/Affect | 12 | 1 | 0 | 13 | 92% |
| 5 | 5D Dreams | 15 | 1 | 0 | 16 | 94% |
| 5 | 5E Operating Freq | 10 | 0 | 0 | 10 | 100% |
| 5 | 5F C-Factor | 16 | 3 | 1 | 20 | 80% |
| **TOTAL** | | **313** | **45** | **4** | **362** | **86%** |

## All STUB + MISSING Items (45 remediation tasks)

### Tier 1 — Mori Parity (13 items)

```
1A.16 | STUB | Clean stale .git/index.lock before worktree ops — no lock cleanup code found
1C.05 | STUB | Role-based tool profiles — RESEARCHER_TOOL_PROFILE imported but not auto-applied to tasks by role
1C.06 | STUB | Role profiles auto-population — conditional wiring from role to denied_tools incomplete
1D.09 | STUB | Secret redaction in logs — no pattern matching for sk-*/xoxb-*/ghp_* in tracing layer
1F.03 | STUB | Plan circular dependency validation — basic TOML parse but no cycle detection
1F.04 | STUB | Plan quality heuristics — no task desc length, read_files presence, or verify field checks
1H.07 | STUB | TUI Page 1 Overview — 3-column layout with plan table, health indicators, alerts not rendered
1H.08 | STUB | TUI Page 2 Plan Execution — task table, live agent output, detail panel not rendered
1H.09 | STUB | TUI Page 3 Agent Activity — agents table, model distribution, cost breakdown not rendered
1H.10 | STUB | TUI Page 4 Gate Results — gate summary, thresholds, failures list not rendered
1H.11 | STUB | TUI Page 5 Learning — router table, experiments, efficiency sparklines not rendered
1H.12 | STUB | TUI Page 6 Signals — signals table, distribution chart, DAG explorer not rendered
1I.04 | STUB | Skill query before dispatch — SkillLibrary loaded but query() not called to inject guidance
```

### Tier 3 — Templates & Events (5 items)

```
3A.07 | STUB | Experiment variant assignment — TemplateExperiment schema exists but ExperimentStore.assign_variant() not called in dispatch
3A.08 | STUB | Feedback→experiment outcome conversion — feedback collected but not routed to ExperimentStore outcomes
3B.05 | STUB | Default 21 subscriptions — only template-triggered subscriptions exist, not full enumeration
3B.07 | STUB | CLI subscription commands — roko subscription list/add/remove/enable/disable not in main.rs subcommands
3C.11 | STUB | CLI event-sources command — roko event-sources list not in main.rs subcommands
```

### Tier 4 — Daemon & Ops (22 items)

```
4A.01 | STUB | Daemon subcommands not in main.rs — DaemonState enum exists but Start/Stop/Status/Logs not wired to CLI
4A.06 | STUB | daemon_stop/status/logs/reload — function signatures defined but bodies not implemented
4A.09 | STUB | daemon install/uninstall — launchd.rs exists but subcommands not registered
4B.03 | STUB | Per-repo data isolation — .roko/repos/{name}/signals.jsonl not implemented, uses global
4B.04 | STUB | Repo context in dispatch — webhook dispatch doesn't set working directory per repo
4B.05 | STUB | Repo-local config override — ./{repo}/.roko/roko.toml merging not implemented
4B.06 | STUB | Cross-repo references — repo listing in agent prompt context not implemented
4C.02 | STUB | fly.toml template — only Railway and manual backends exist
4C.06 | STUB | /api/health endpoint — not found in routes (needed for cloud health checks)
4C.07 | STUB | --cloud flag on roko init — cloud-optimized config generation not implemented
4C.08 | STUB | register_github_webhook() — auto-registration after deploy not implemented
4C.09 | STUB | Post-deploy webhook registration loop not wired
4C.10 | STUB | Cloud execution flow — clone/branch/execute/gate/commit/push/PR not orchestrated
4C.11 | STUB | Git helper functions — git_clone, git_checkout_new_branch, git_commit, git_push not found
4C.12 | STUB | Volume mount path docs not found
4D.02 | STUB | .env file loading — dotenvy not wired into config initialization
4D.03 | STUB | Secret masking in logs — tracing-subscriber layer for redaction not found
4D.04 | STUB | Secret masking in API responses — raw values not replaced with ***
```

### Tier 5 — Cognitive (5 items)

```
5A.15 | MISSING | HDC signal fingerprinting — no calls to bardo_primitives fingerprint in signal handlers
5A.16 | MISSING | HDC episode fingerprinting — no episode metadata fingerprinting in orchestrate.rs
5A.17 | MISSING | Similarity-based template suggestion — no HDC matching for unmatched signals
5C.05 | STUB | Affect events incomplete — only GateResult fires; TaskOutcome/Blocked/TimePressure/QueueWait/DreamFailure never triggered
5D.05 | STUB | Auto-dream scheduling — DreamLoopConfig exists but orchestrate.rs never calls DreamRunner::schedule()
5F.11 | STUB | J3 Social sensitivity — returns 0.0, context-attribution.jsonl never written
5F.12 | STUB | J4 Knowledge integration rate — KnowledgeConfirmationRecord never produced by distiller
5F.14 | STUB | J6 Convergence velocity — same data gap as J4
5F.18 | MISSING | J10 C-factor metrics endpoint — GET /api/metrics/c_factor not created
5F.23 | STUB | AntiKnowledge auto-generation — halves confidence works but not auto-generated from gate failures
```

## Also: Test Compilation Failures (8 crates)

These are separate from the above — tests that don't compile:

| Crate | Error Count | Root Cause |
|-------|-------------|------------|
| roko-daimon | 1 | Missing `tempfile` dev-dep |
| roko-conductor | 1 | `TaskTimingEvent` missing `Serialize` |
| roko-dreams | 12 | Missing `tempfile`, `ChronoDuration` undefined, private `read_all()` |
| roko-learn (lib) | 1 | `&&str` → `String` type mismatch |
| roko-learn (integration) | 11 | `LinUCBRouter` API changed (3→4 args) |
| roko-plugin | 2 | `notify` API changed + lifetime issue |
| roko-serve | 4 | Duplicate `Uuid`, missing `CascadeRouter`, private fields |
| roko-cli | 1 | Missing `workspace_dir` field |
| roko-agent (integration) | 9 | `SafetyPolicy`/`with_safety_policy` API removed |
| roko-mcp-slack | 1 | Temporary value dropped while borrowed |
