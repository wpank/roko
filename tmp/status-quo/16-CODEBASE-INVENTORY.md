# 16 · Codebase Inventory

> Status-quo audit · **deeper second pass** · verified **HEAD `5852c93c05`** · `main` · **2026-07-08**
> Method: every number below computed by shell over the working tree, not estimated.
> - Files/LOC: `find <pkg> -name '*.rs' -not -path '*/target/*'` + `wc -l`
> - Public API: `rg -c 'pub fn |pub struct |pub enum |pub trait '` (declaration count, summed per package)
> - Tests: `rg -c '#\[(tokio::)?test\]|#\[test\]|#\[rstest\]'`
> - Big files: `find … -exec wc -l {} + | sort -rn`
> - Artifacts: `du -sh .roko/*`
> Supersedes the 2026-07-07 draft (thin, ~3 KB). Scope = 31 `crates/` + 3 `apps/` + `tests/` = **35 Cargo members**.

---

## 1. Workspace totals

| Metric | Value | Note |
|---|---:|---|
| Cargo members | **35** | 31 `crates/*`, 3 `apps/*`, 1 `tests/`; `default-members` = cli + mcp-code + mcp-github only (`Cargo.toml:84`) |
| Rust files | **1,272** | excludes `target/`; excludes 12 stale root `src/*.rs` (not a workspace package) |
| Rust LOC | **727,276** | raw line count incl. blanks/comments/tests |
| Public-API declarations | **10,671** | `pub fn/struct/enum/trait` sites |
| Test attributes | **9,968** | attribute hits, **not** a passing-test count |
| Avg LOC / file | **572** | 727,276 / 1,272 |
| Avg tests / crate | **~285** | skewed by roko-agent (1,715) and roko-cli (1,648) |

The workspace is **top-heavy**: `roko-cli` (188 K) + `roko-agent` (87 K) + `roko-serve` (66 K) + `roko-learn` (60 K) + `roko-core` (54 K) = **455 K LOC = 63 %** of the tree lives in 5 packages.

---

## 2. Per-package inventory (all 35)

Sorted by LOC descending. **Role** = one-line responsibility. **API** = pub-decl count, **Tst** = test attrs, density columns are LOC-normalized signals of surface bloat / test coverage.

| Package | Layer | rs | LOC | API | Tst | API/kLOC | Role |
|---|:--:|--:|--:|--:|--:|--:|---|
| roko-cli | 4 | 281 | 188,494 | 1,479 | 1,648 | 7.8 | CLI binary: all subcommands + ratatui TUI + runner v2 + legacy orchestrate |
| roko-agent | 2 | 192 | 87,187 | 1,180 | 1,715 | 13.5 | 8 LLM backends, pools, MCP, tool loop, safety layer |
| roko-serve | 4 | 109 | 65,640 | 456 | 451 | 6.9 | HTTP control plane (~85 routes) + SSE + WS on :6677 |
| roko-learn | 2 | 71 | 59,645 | 1,003 | 918 | 16.8 | Episodes, playbooks, bandits, cascade router, experiments, efficiency |
| roko-core | 1 | 121 | 53,653 | 1,312 | 1,173 | 24.5 | Signal + 6 verb traits, types, config, tools, errors (kernel) |
| mirage-rs (app) | — | 52 | 38,376 | 540 | 385 | 14.1 | EVM/chain simulator app (revm, JSON-RPC, dashboard API) |
| roko-compose | 2 | 55 | 26,786 | 447 | 430 | 16.7 | Prompt assembly, 9 templates, enrichment, context providers |
| roko-chain | 2 | 41 | 23,436 | 708 | 282 | 30.2 | Chain witness primitives + optional alloy backend |
| roko-gate | 3 | 47 | 22,653 | 425 | 550 | 18.8 | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-orchestrator | 3 | 32 | 21,030 | 490 | 490 | 23.3 | Plan DAG, parallel executor, merge queue, safety |
| roko-runtime | 1 | 26 | 19,172 | 561 | 228 | 29.3 | ProcessSupervisor, event bus, workflow engine, effect driver |
| roko-acp | 4 | 18 | 17,098 | 181 | 128 | 10.6 | Agent-Client-Protocol editor surface / bridge |
| roko-neuro | 2 | 10 | 16,553 | 252 | 166 | 15.2 | Durable knowledge store, distillation, tier progression |
| roko-dreams | 2 | 26 | 13,741 | 334 | 92 | 24.3 | Offline consolidation (hypnagogia, imagination, cycle) |
| roko-conductor | 3 | 25 | 10,268 | 180 | 300 | 17.5 | 10 watchers, circuit breaker, diagnosis |
| roko-std | 2 | 37 | 8,442 | 108 | 224 | 12.8 | Defaults, 19 builtin tools, mock dispatcher |
| roko-daimon | 2 | 8 | 7,450 | 205 | 89 | 27.5 | Affect engine, somatic markers, dispatch modulation |
| roko-demo | 2 | 21 | 5,860 | 95 | 6 | 16.2 | Chain-demo orchestrator (a bin app living in `crates/`) |
| roko-fs | 2 | 14 | 5,518 | 130 | 129 | 23.6 | FileSubstrate (JSONL), GC, layout |
| roko-graph | 2 | 20 | 4,902 | 107 | 116 | 21.8 | Sequential cell/DAG engine (only consumer: roko-cli) |
| roko-primitives | 0 | 12 | 4,835 | 143 | 133 | 29.6 | HDC vectors, tier routing (the L0 leaf-of-nothing) |
| roko-index | 2 | 7 | 4,575 | 90 | 60 | 19.7 | Parser + graph + HDC indexing |
| roko-agent-server | 4 | 15 | 4,055 | 91 | 25 | 22.4 | Per-agent HTTP sidecar (13 routes) |
| roko-mcp-github | 2 | 1 | 3,195 | 0 | 18 | 0.0 | GitHub MCP bin (0 pub API — single `main.rs`) |
| roko-chain-watcher (app) | — | 7 | 2,932 | 26 | 31 | 8.9 | Chain event watcher daemon |
| agent-relay (app) | — | 7 | 2,184 | 54 | 11 | 24.7 | Relay lib **wrongly placed in `apps/`** (see §5 / doc 11) |
| roko-mcp-code | 2 | 2 | 1,935 | 5 | 13 | 2.6 | Code-intelligence MCP server |
| roko-plugin | 2 | 3 | 1,783 | 31 | 22 | 17.4 | Plugin host / manifest loader |
| roko-lang-rust | 2 | 2 | 1,390 | 18 | 46 | 12.9 | Rust language support for index |
| roko-mcp-slack | 2 | 1 | 1,114 | 0 | 2 | 0.0 | Slack MCP bin |
| roko-lang-typescript | 2 | 1 | 938 | 4 | 33 | 4.3 | TypeScript language support |
| roko-mcp-scripts | 2 | 1 | 765 | 0 | 7 | 0.0 | Scripts MCP bin |
| roko-tests (tests/) | — | 5 | 747 | 6 | 20 | — | Cross-crate integration tests (dev-deps only) |
| roko-lang-go | 2 | 1 | 673 | 2 | 25 | 3.0 | Go language support |
| roko-mcp-stdio | 2 | 1 | 251 | 8 | 2 | 31.9 | Shared stdio MCP transport (root of the mcp mini-cluster) |
| **TOTAL** | | **1,272** | **727,276** | **10,671** | **9,968** | | |

**Density reads.** `roko-core` (24.5 API/kLOC) and `roko-mcp-stdio` (31.9) are thin, API-dense as a kernel/transport should be. `roko-cli` (7.8) and `roko-serve` (6.9) are procedural/glue — lots of code, little exported surface. The four MCP `main.rs` bins export **0** public API (they are pure binaries). `roko-demo` has **6** test attrs across 5,860 LOC — effectively untested for its size.

---

## 3. Biggest files (top 20 by LOC) — the debt signal

Large single files are the clearest structural-debt marker. `orchestrate.rs` alone is **3.3 %** of the entire workspace and **12.6 %** of roko-cli.

| # | File | LOC | Debt read |
|--:|---|--:|---|
| 1 | `crates/roko-cli/src/orchestrate.rs` | **23,676** | God-file; the "wired core loop" per CLAUDE.md but gated behind opt-in `legacy-orchestrate` (see doc 11 §feature-rot). Not in default builds. |
| 2 | `crates/roko-cli/src/runner/event_loop.rs` | **6,681** | Runner v2 live loop — the actual default execution path; the new large coordination file. |
| 3 | `crates/roko-cli/src/tui/dashboard.rs` | **6,373** | Single-file ratatui dashboard render + state. |
| 4 | `apps/mirage-rs/src/rpc.rs` | 6,117 | Monolithic JSON-RPC surface for the EVM sim. |
| 5 | `crates/roko-cli/src/chat_inline.rs` | 5,699 | Inline chat REPL. |
| 6 | `crates/roko-acp/src/bridge_events.rs` | 5,598 | ACP event bridge — largest non-cli/non-mirage file. |
| 7 | `crates/roko-learn/src/runtime_feedback.rs` | 5,445 | Feedback ingestion / replan logic. |
| 8 | `crates/roko-cli/src/main.rs` | 5,085 | CLI arg wiring + dispatch (clap tree). |
| 9 | `crates/roko-cli/src/tui/state.rs` | 4,965 | TUI state model. |
| 10 | `crates/roko-neuro/src/knowledge_store.rs` | 4,751 | Durable KV + tiering; 29 of its `hdc` cfg-sites compile out (dead feature). |
| 11 | `crates/roko-cli/src/tui/app.rs` | 4,491 | TUI app loop. |
| 12 | `crates/roko-cli/src/config.rs` | 4,144 | CLI config model + parsing. |
| 13 | `crates/roko-cli/src/prd.rs` | 3,849 | PRD lifecycle subcommands. |
| 14 | `crates/roko-cli/src/run.rs` | 3,777 | Default (non-legacy) `roko run` path. |
| 15 | `crates/roko-daimon/src/lib.rs` | 3,761 | Entire daimon crate in one `lib.rs` (8 files, 7,450 LOC total). |
| 16 | `crates/roko-dreams/src/cycle.rs` | 3,488 | Dream consolidation cycle. |
| 17 | `crates/roko-compose/src/context_provider.rs` | 3,483 | Context provider assembly. |
| 18 | `crates/roko-mcp-github/src/main.rs` | 3,195 | Entire GitHub MCP in one `main.rs`. |
| 19 | `crates/roko-serve/src/dispatch.rs` | 3,153 | Serve request dispatch. |
| 20 | `crates/roko-serve/src/lib.rs` | 3,141 | Serve bootstrap / router assembly. |

**Top-3 verdict:** files 1–3 (all in `roko-cli`, 36.7 K LOC combined) are the workspace's three worst monoliths. `orchestrate.rs` is ~2× the next file and sits behind an opt-in feature, so its size is *latent* debt (mostly uncompiled by default) — the *active* debt is `event_loop.rs` (default path) and `tui/dashboard.rs`.

**Files >3 K LOC: 20.** **Files >5 K LOC: 9.** Eight of the top nine are in `roko-cli` or `mirage-rs`.

---

## 4. Generated `.roko/` artifacts (runtime state footprint)

Sizes from `du -sh .roko/*` (worktrees excluded). This is *generated* state, not source — useful for spotting runaway logs.

| Artifact | Size | Lines | Note |
|---|--:|--:|---|
| `.roko/events.jsonl` | **43 M** | 157,264 | **Unbounded event log — top GC candidate.** Dwarfs every other artifact. |
| `.roko/chain-watcher.log` | 23 M | — | Chain-watcher stdout; no rotation. |
| `.roko/roko.log` | 12 M | — | Main log; no rotation. |
| `.roko/state/run-ledger.jsonl` | 6.1 M | — | Executor run ledger (within `state/`, 6.0 M dir total). |
| `.roko/dreams/` | 1.4 M | — | Dream journal + archive. |
| `.roko/learn/` | 948 K | — | cascade-router.json (79 ln), experiments.json (2 ln), efficiency.jsonl (37 ln), gate-thresholds |
| `.roko/neuro/` | 780 K | — | Knowledge store. |
| `.roko/signals.jsonl` | 80 K | 467 | Signal DAG log. |
| `.roko/episodes.jsonl` | 68 K | **27** | Agent turn recording — only 27 episodes despite 157 K events. |
| `.roko/engrams.jsonl` | 12 K | 10 | |
| `.roko/GAPS.md` | 4 K | 19 | Canonical gap tracker (per CLAUDE.md rule 4). |

**Empty/zero dirs** (allocated but unused): `traces`, `templates`, `task-outputs`, `subscriptions`, `runs`, `plans`, `metrics`, `jobs`, `config`, `cache` — 10 pre-created dirs with 0 content, indicating scaffolded-but-unexercised subsystems.

**Signal:** `events.jsonl` (43 M / 157 K lines) vs `episodes.jsonl` (27 lines) shows the low-level event bus is heavily exercised but the episode-recording layer barely fires — consistent with the default runner path not persisting episodes as densely as the legacy `orchestrate.rs` loop would.

---

## 5. Structural observations

1. **`roko-cli` is a monolith host** — 281 files / 188 K LOC / 20 intra-workspace deps (the max). It contains the TUI, both execution engines (runner v2 + legacy orchestrate), chat, PRD, and config. This is the primary refactor target.
2. **Four `main.rs`-only bins** (`roko-mcp-github/slack/scripts`, and `roko-demo` nearly) export 0–95 pub API — fine for bins, but `roko-demo` (5.9 K LOC, 6 tests) is an *app* mis-filed under `crates/`.
3. **`agent-relay`** is a 2.2 K-LOC library (54 pub decls) living under `apps/` yet depended on by `roko-agent-server` — a `crates → apps` inversion (full analysis in doc 11).
4. **`roko-daimon`** packs 3,761 of its 7,450 LOC into a single `lib.rs` — a small crate that would benefit from module split.
5. **Test coverage is bimodal**: `roko-agent`/`roko-core`/`roko-cli` are heavily tested (1,000+ attrs each); `roko-demo` (6), `roko-mcp-slack` (2), `roko-mcp-stdio` (2), `agent-relay` (11) are near-zero.

---

## 6. Checklist / roadmap

- [ ] **[P1]** Split `orchestrate.rs` (23,676 LOC) or fully retire it — decide legacy-vs-runner-v2 canonicality (blocks doc 11 open-Q #2). Verify: `wc -l crates/roko-cli/src/orchestrate.rs`.
- [ ] **[P1]** GC / rotate `.roko/events.jsonl` (43 M, 157 K lines) — add a size cap or `roko knowledge gc` sweep. Verify: `du -h .roko/events.jsonl`.
- [ ] **[P2]** Rotate `.roko/roko.log` (12 M) and `.roko/chain-watcher.log` (23 M) — no rotation today.
- [ ] **[P2]** Re-home `roko-demo` (app in `crates/`) and `agent-relay` (lib in `apps/`) to correct trees. Verify: `grep -n layer crates/roko-demo/Cargo.toml`.
- [ ] **[P2]** Decompose `roko-cli` top-4 files (event_loop, dashboard, chat_inline, main = 24 K LOC) into submodules.
- [ ] **[P3]** Add smoke tests to `roko-demo` (6 attrs / 5,860 LOC) and `agent-relay` (11 attrs).
- [ ] **[P3]** Investigate 10 empty `.roko/` dirs (traces, task-outputs, runs, plans, metrics, jobs…) — scaffolded but never written; confirm live vs dead.
- [ ] **[P3]** Reconcile episode-recording gap: 157 K events vs 27 episodes — is the default runner persisting episodes? Verify: `wc -l .roko/episodes.jsonl .roko/events.jsonl`.

---

## 7. Reproduce these numbers

```bash
cd /Users/will/dev/nunchi/roko/roko            # HEAD 5852c93c05
# per-package: files / loc / api / tests
for d in crates/* apps/* tests; do
  f=$(find "$d" -name '*.rs' -not -path '*/target/*' | wc -l)
  l=$(find "$d" -name '*.rs' -not -path '*/target/*' -exec cat {} + | wc -l)
  a=$(rg -c --no-filename 'pub fn |pub struct |pub enum |pub trait ' "$d" | awk '{s+=$1}END{print s}')
  t=$(rg -c --no-filename '#\[(tokio::)?test\]|#\[test\]|#\[rstest\]' "$d" | awk '{s+=$1}END{print s}')
  echo "$(basename $d) $f $l $a $t"
done
# biggest files
find crates apps tests -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20
# artifacts
du -sh .roko/* | sort -rh
```

See also: **doc 11** (dependency graph + layering violations), **doc 03** (crate audit), **doc 59** (API route ledger).
