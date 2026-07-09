# E16 — PRD & Self-Hosting Pipeline (generative front-half)

**Owner surface:** `idea → draft → research → plan` (the generative front-half of self-hosting).
**Source docs:** `tmp/status-quo/91-PRD-RESEARCH`, `tmp/status-quo/98-TRACE-SELF-HOSTING-LOOP`.
**Verified against:** HEAD `5852c93c05` (2026-07-09). All line refs below re-checked live.

## TL;DR

The three pre-existing plans (**P08-search-command-fix**, **P09-tool-alias-fix**,
**P23-prd-pipeline-fix** = 13 tasks) already cover ~90% of this epic and **still match the
current code byte-for-byte** — every bug they target is unfixed and every line reference is
accurate. E16 adds only **2 gap tasks** for things those plans miss:
1. the Perplexity *integration* test file (P08 only fixes the unit tests in `search.rs`);
2. an end-to-end smoke test of the front-half loop once all three plans land.

Strong cross-epic dependencies: **E01** (execute half — `prd plan` writes `tasks.toml`, then
`plan run` defaults to a graph dry-run and never executes) and **E14** (tool-alias, == P09).

## Findings (all re-verified against current code)

| # | Finding | Evidence (current code) | Status |
|---|---|---|---|
| F1 | `research search` sends batch `{"queries":[...]}` → Perplexity 422 (no batch endpoint) | `search.rs:150` `json!({ "queries": wire_queries })`; `search_batch:141-188` | Broken, unfixed |
| F2 | Response parsed as nested per-query, not flat `{"results":[...]}` | `search.rs:176-186` iterates `results` as `SearchResponse` items | Broken, unfixed |
| F3 | Reads `content`; API may return `snippet` — silently empty | `types.rs:26` `pub content: String` (no `serde(alias)`) | Broken, unfixed |
| F4 | Date sent ISO `%Y-%m-%d`; Perplexity wants recency filter / MM-DD-YYYY | `research.rs:733-746` `now.format("%Y-%m-%d")` | Broken, unfixed |
| F5 | ~20 unit tests false-green on fabricated batch shape | `search.rs:265-627` `canned_batch`, `MAX_BATCH_SIZE`, `TooManyQueries` | Unfixed |
| F5b | **Integration** tests also encode batch shape (P08 does NOT touch them) | `tests/perplexity_integration.rs:385-489` asserts `body["queries"]`, 1 req / 5 queries, nested results | **GAP** |
| F6 | `prd status` per-PRD Plans/Tasks/Done columns hardcoded `"—"` | `prd.rs:799` `entry.slug, entry.status, "—", "—", "—"` | Broken, unfixed |
| F7 | `read_prd_entry` returns `"unknown"` when frontmatter missing | `prd.rs:639-646` | Unfixed |
| F8 | Tool-alias bug: `parse_allowed_tools_csv` returns raw `HashSet<&str>`; no `canonical_of_claude` → 0 tools on non-Claude | `openai_compat.rs:252` `-> Option<HashSet<&str>>` | Broken, unfixed |
| F9 | draft-new agent has `allowed_tools: Some("none")` → PRD from thin air | `commands/prd.rs:459-470` | Unfixed |
| F10 | `prd plan <slug>` writes `tasks.toml`; `plan run` defaults to graph dry-run — never executes | ties to **E01** | Cross-epic |
| F11 | `auto_plan` trigger on PRD publish **is wired** | `roko-serve/src/lib.rs:336,566` `start_prd_publish_subscriber` | OK (verified) |

## Reconciliation: finding → existing plan → still-open?

| Finding | Covered by | Task(s) | Still matches code? |
|---|---|---|---|
| F1, F2 | **P08** | T1 (single-query rewrite + flat parse) | Yes — `search.rs` unchanged |
| F3 | **P08** | T4 (`serde(alias="snippet")` on `content`) | Yes — `types.rs:26` bare |
| F4 | **P08** | T2 (`recency_filter` in `research.rs`) | Yes — `research.rs:733-746` intact |
| F5 | **P08** | T3 (rewrite unit tests in `search.rs`) | Yes |
| **F5b** | **none** | — | **GAP → E16-T1** |
| F6 | **P23** | T4 (link plans by slug in `cmd_status`) | Yes — `prd.rs:799` dashes intact |
| F7 | **P23** | T5 (infer status from dir) + T6 (`source_prd`) | Yes |
| F8 | **P09** (== E14) | T1/T2/T3 (canonical alias resolution) | Yes — `openai_compat.rs:252` returns `&str` |
| F9 | **P23** | T1/T2 (give draft agent Read/Grep/Glob) | Yes — `commands/prd.rs` `Some("none")` intact |
| F10 | **E01** | (execute half) | Cross-epic dependency |
| F11 | — | already wired | No action |

**Coverage:** P08 (4) + P09 (3) + P23 (6) = **13 tasks cover F1–F9**. E16 adds **2** for the
gaps (F5b + a front-half e2e smoke). Total E16 surface = 15 tasks, 13 pre-existing + 2 new.

## Remaining gaps (new tasks)

### Gap A — F5b: integration tests still encode the dead batch shape
`crates/roko-agent/tests/perplexity_integration.rs` is a separate file from `search.rs`.
P08-T3 rewrites only the in-module unit tests. The integration tests
(`perplexity_search_single_query_uses_search_api:385`,
`perplexity_search_batch_five_queries_are_sent_together:424`) assert `body["queries"]` arrays,
a single HTTP request for five queries, and nested per-query result objects. After P08-T1
rewrites `search_batch` to loop single queries, these will **fail** (or, worse, keep asserting
the wrong wire shape). Must be reconciled in lockstep with P08.

### Gap B — front-half e2e smoke absent
No plan wires an end-to-end assertion that `idea → draft → research search → prd plan` produces
a valid `tasks.toml` after the fixes land. `e2e_self_host.rs` exists but does not cover the
`research search` real-path or the `prd status` columns. A gated smoke test (structural, no live
API key) closes the loop and guards regressions across P08+P09+P23.

---

## E16 tasks (gaps only)

```toml
[meta]
plan = "E16-prd-self-hosting-gaps"
total = 2
done = 0
status = "ready"
max_parallel = 1

# ═══════════════════════════════════════════════════════════════════════
# E16-T1: Reconcile Perplexity integration tests with the single-query API
# ═══════════════════════════════════════════════════════════════════════
# P08-T3 rewrites the UNIT tests inside search.rs but leaves the separate
# integration test file untouched. After P08-T1 makes search_batch loop over
# single-query POSTs, these integration tests break: they assert body["queries"]
# arrays, one HTTP request for five queries, and nested per-query result objects.
# Rewrite them to the flat single-query shape: N queries => N requests, each with
# a top-level {"query": "..."} body, and a flat {"results": [...]} response.

[[task]]
id = "E16-T1"
title = "Rewrite perplexity_integration.rs for the single-query search API"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-5"
max_loc = 90
files = ["crates/roko-agent/tests/perplexity_integration.rs"]
role = "implementer"
depends_on_plan = ["P08-search-command-fix"]

[task.context]
read_files = [
  { path = "crates/roko-agent/tests/perplexity_integration.rs", lines = "385-489", why = "The two batch-format tests: single-query test asserts body[\"queries\"][0] (416-418); five-query test expects 1 request + nested results (424-489)" },
  { path = "crates/roko-agent/src/perplexity/search.rs", lines = "141-203", why = "Post-P08 search_batch loops self.search over single queries; wire body is flat {\"query\": ...}" },
]
symbols = [
  "PerplexitySearchClient::search_batch — post-P08 loops single-query POSTs, N queries => N requests",
  "spawn_scripted_server — test helper returning one scripted response per request",
]
anti_patterns = [
  "Do NOT assert body[\"queries\"] anywhere — the flat body has a top-level \"query\" field.",
  "Do NOT expect one request for five queries — the loop issues one request per query, so script five responses.",
  "Do NOT return nested {\"results\":[{\"query\":..,\"results\":..}]} — the real API returns flat {\"results\":[{url,title,content}]}.",
]

[[task.verify]]
phase = "structural"
command = "! grep -q 'queries\\[0\\]\\|body\\[\"queries\"\\]' crates/roko-agent/tests/perplexity_integration.rs"
fail_msg = "No integration test may assert a queries array in the request body"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-agent --test perplexity_integration 2>&1"
fail_msg = "Perplexity integration tests must pass against the single-query API"

# ═══════════════════════════════════════════════════════════════════════
# E16-T2: Front-half self-hosting smoke test (idea → draft → plan)
# ═══════════════════════════════════════════════════════════════════════
# No test guards the generative front-half after P08/P09/P23 land. Add a
# structural (no live-API) smoke test that drives: prd idea -> prd draft new
# -> prd plan <slug>, then asserts a parseable tasks.toml and that prd status
# shows real (non-"—") columns for the linked PRD. Gate on offline stubs so
# CI can run it without PERPLEXITY_API_KEY / Claude CLI.

[[task]]
id = "E16-T2"
title = "Add offline front-half self-hosting smoke test"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-5"
max_loc = 120
files = ["crates/roko-cli/tests/e2e_self_host.rs"]
role = "implementer"
depends_on = ["E16-T1"]
depends_on_plan = ["P23-prd-pipeline-fix", "P09-tool-alias-fix"]

[task.context]
read_files = [
  { path = "crates/roko-cli/tests/e2e_self_host.rs", lines = "1-80", why = "Existing e2e harness + stub patterns to extend" },
  { path = "crates/roko-cli/src/prd.rs", lines = "752-819", why = "cmd_status columns — assert non-dash output for the linked PRD (post-P23-T4)" },
  { path = "crates/roko-cli/src/task_parser.rs", lines = "1-60", why = "TaskDef/TaskMeta parse target for the generated tasks.toml" },
]
symbols = [
  "cmd_status — pub fn cmd_status(workdir: &Path, plans_dir: Option<&Path>) -> Result<()>",
  "TaskDef — native task schema parsed from tasks.toml",
]
anti_patterns = [
  "Do NOT call the live Perplexity or Claude APIs — use the mock/stub dispatcher so CI runs offline.",
  "Do NOT duplicate P23's cmd_status unit tests — this asserts the full CLI-driven pipeline end to end.",
  "Do NOT hardcode a plan slug that skips the slug-linking path — the point is to prove prd status links the generated plan.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'tasks.toml' crates/roko-cli/tests/e2e_self_host.rs"
fail_msg = "Smoke test must assert a tasks.toml is produced by prd plan"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli --test e2e_self_host 2>&1"
fail_msg = "Front-half self-hosting smoke test must pass offline"
```

## Dependencies & sequencing

```
P09/E14 (tool-alias) ─┐
                      ├─> prd/research agents work on non-Claude ─┐
P23 (prd pipeline) ───┘                                          │
P08 (search)  ──> E16-T1 (integration tests) ───────────────────┼─> E16-T2 (front-half smoke)
                                                                 │
E01 (execute half) ── needed for `plan run` to actually run ─────┘ (F10, out of E16 scope)
```

- **E16-T1** must land in the same PR/sequence as **P08** (shared wire-format change).
- **E16-T2** depends on **P23** (real `prd status` columns) and **P09** (agents get tools) landing.
- **F10** (dry-run default) is owned by **E01**; E16 stops at generating a valid `tasks.toml`.
