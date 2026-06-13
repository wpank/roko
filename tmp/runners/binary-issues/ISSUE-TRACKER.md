# Binary Issues — Issue Tracker

**Source**: `tmp/binary-issues/MASTER-INDEX.md` (audit dated 2026-04-28).
**Verified**: 2026-05-01 against current `wp-arch2` HEAD.
**Scope**: Only items still OPEN or PARTIAL in the default build (`cargo build -p roko-cli` with `default = []`). Items only present inside `#[cfg(feature = "legacy-orchestrate")]` modules are excluded — they no longer affect the shipping binary.

This tracker is the single source of truth for the `binary-issues` runner. Every batch in `batches.toml` corresponds to exactly one row here. Tick a row when its batch lands and verifies green.

Status legend:
- `[ ]` — open, batch defined
- `[~]` — partial / multi-batch fix in progress
- `[x]` — verified fixed (close the row, leave it for history)
- ~~strikethrough~~ — verified obsolete (no batch needed)

---

## BI_SEC — Security (P0, do not deploy without)

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_01 | S5.5 | Default `share` to `--secret` gist with scrubbed payload | `[ ]` |
| BI_02 | S5.6 | Add command allowlist to `/api/terminal/sessions` | `[ ]` |
| BI_03 | S5.7 | Cap PTY session count and apply idle TTL | `[ ]` |
| BI_04 | S5.8 | Default `dangerously_skip_permissions` to `false` at all hardcoded sites | `[ ]` |
| BI_05 | S5.9 | Promote secret-leak / forbidden-write violations to `Block` severity | `[ ]` |
| BI_06 | S5.10 | `#[cfg(test)]`-gate `AgentContract::permissive` | `[ ]` |
| BI_07 | S5.11 | Document and audit implementer Python sandbox bypass risk | `[ ]` |

Verified `[x]` (no batch needed): S5.1 auth-default, S5.3 PORT bind safety. PARTIAL but acceptable: S5.2 PTY auth on non-loopback, S5.4 CORS localhost predicate.

---

## BI_PHN — Phantom features (built but not wired)

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_08 | S8.1 | Auto-trigger `EpisodeLogger::compact` on session start / N episodes | `[ ]` |
| BI_09 | S8.2 | Wire `DreamTriggerSink::with_runner` and consume `dream_triggers.jsonl` | `[ ]` |
| BI_10 | S8.3 | Persist LinUCB arm matrices via `CascadeSnapshot` | `[ ]` |
| BI_11 | S8.4 | Activate VCG strategy when budget pressure crosses threshold | `[ ]` |
| BI_12 | S8.5 | Enforce cumulative `MaxCostPerTurn` across tool calls | `[ ]` |
| BI_13 | S8.7 | Share single `SharedStateHub` across TUI and serve | `[ ]` |
| BI_14 | S8.8 | Harden `create_share` empty-transcript edge cases | `[ ]` |
| BI_15 | S8.9 | Fix `Share.tsx` endpoint mismatch (`/api/share/` → `/api/shared/`) | `[ ]` |
| BI_16 | S1.7 | Query knowledge store and populate `knowledge_ids` in chat path | `[ ]` |

---

## BI_CMD — Slash commands that lie (no-op handlers)

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_17 | S3.3 | `/gate <name> on|off` actually toggles runtime config | `[ ]` |
| BI_18 | S3.4 | `/config set <key> <value>` writes to runtime overlay (and optionally roko.toml) | `[ ]` |
| BI_19 | S3.5a | `/run <prompt>` executes inline via WorkflowEngine | `[ ]` |
| BI_20 | S3.5b | `/plan run <dir>` executes inline | `[ ]` |
| BI_21 | S3.5c | `/prd idea <text>` writes idea + opens flow | `[ ]` |
| BI_22 | S3.5d | `/research <query>` runs research backend inline | `[ ]` |
| BI_23 | S3.6 | `roko learn tune gates` actually applies threshold updates | `[ ]` |

---

## BI_STR — Streaming for run / plan execution

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_24 | S6.5 | Forward `WorkflowEngine` lifecycle events to terminal during plan run | `[ ]` |
| BI_25 | S6.6 | Stream incremental output for `roko run` v2 | `[ ]` |

---

## BI_SUB — Subprocess & process management

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_26 | S9.1 | 3 s timeout on `claude --version` probe in `auth_detect.rs` | `[ ]` |
| BI_27 | S9.2 | Capture MCP server stderr to log file (no `Stdio::inherit`) | `[ ]` |
| BI_28 | S9.4 + S9.5 | Thread `CancellationToken` into chat dispatch + Ctrl+C cancels | `[ ]` |
| BI_29 | S9.6 | Store chain-watcher join handle and kill on shutdown | `[ ]` |
| BI_30 | S9.7 | Non-blocking background-serve startup (don't block chat boot) | `[ ]` |
| BI_31 | S9.8 + S9.9 | Replace bare `eprintln!` in `claude_cli_agent.rs` and guard `main.rs` | `[ ]` |

---

## BI_ERR — Silent error swallowing

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_32 | S4.2 | Log JSONL write/flush errors at `warn` instead of `let _ =` | `[ ]` |
| BI_33 | S4.3 | Log `AffectPolicy::persist` failures (warn + counter) | `[ ]` |
| BI_34 | S4.4 | Surface background-serve failure to user (not just `tracing::warn`) | `[ ]` |
| BI_35 | S4.7 | Return PTY `send_input` errors to WS client | `[ ]` |
| BI_36 | S4.8 | Store `fswatcher` join handle, log spawn failure | `[ ]` |
| BI_37 | S4.10 | Fix double REST event delivery FIXME on `EventBus` | `[ ]` |

---

## BI_HRD — Hardcoded values → config

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_38 | S7.3 | Replace ad-hoc `claude-opus-4-6` literals with one constant / preset key | `[ ]` |
| BI_39 | S7.4 | Anthropic base URL through provider config (no `DEFAULT_BASE_URL` hard-ref in dispatch) | `[ ]` |
| BI_40 | S7.5 | Anthropic API version through provider config | `[ ]` |
| BI_41 | S7.6 | Consolidate `8192` `max_tokens` literal into per-role / per-provider config | `[ ]` |
| BI_42 | S7.7 | Replace `naive_opus_cost` `$15/$75` with `CostTable::lookup` | `[ ]` |
| BI_43 | S7.8 | `CostTable` loaded from config with hardcoded fallbacks (not vice versa) | `[ ]` |
| BI_44 | S7.9 | Perplexity URL/model through `web_search` config | `[ ]` |
| BI_45 | S7.10 | PID file path uses `.roko` discovery, not `current_dir()` | `[ ]` |

---

## BI_COD — Code health & duplication

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_46 | S10.2 | Extract shared chat event-loop body (kill HTTP/Session duplication) | `[ ]` |
| BI_47 | S10.4 | Extract `render_session_summary()` helper, call from both sites | `[ ]` |
| BI_48 | S10.5 | Remove or merge legacy `chat.rs` into `chat_inline.rs` | `[ ]` |
| BI_49 | S10.6 | Unify `roko init` and `roko config init` into one entry point | `[ ]` |

---

## BI_MTX — Mutex / unwrap risks

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_50 | S11.1 | Switch dispatcher audit-signals lock to `parking_lot::Mutex` | `[ ]` |
| BI_51 | S11.2 | LRU/cache mutexes in `model_call_service` use `parking_lot` | `[ ]` |
| BI_52 | S11.4 | Replace `expect("just registered")` with `ok_or` in `routes/feeds.rs` | `[ ]` |
| BI_53 | S11.5 | Remove `roko-agent` crate-level lint suppressions and fix call sites | `[ ]` |

---

## BI_PRT — Complete partial fixes

| Batch | MASTER ID | Title | Status |
|-------|-----------|-------|--------|
| BI_54 | S1.3 | Add typed `tools` field to `ModelCallRequest` (no more Claude-CLI-only) | `[ ]` |
| BI_55 | S2.4 | Cache and reuse provider agent in `ProviderCallCell::execute` | `[ ]` |
| BI_56 | S6.1 | Stream HTTP `DispatchMode::Http` deltas (parity with Session mode) | `[ ]` |

---

## How this tracker maps to batches

- Every `[ ]` row above has a matching `[[batch]]` block in `batches.toml` whose `id` is the row's batch column.
- Every batch has a prompt at `prompts/<id>.prompt.md` whose first heading is the same `BI_NN: Title`.
- When a batch's PR lands and verification is green, flip `[ ]` → `[x]` here. Do **not** remove the row — historical context is part of the artifact.
- If a batch becomes obsolete mid-run (e.g. fixed by another runner), strike through with `~~...~~` and add a one-line reason.

## Wave summary

| Wave | Groups | Batches | Why this wave |
|------|--------|---------|---------------|
| 1 | BI_SEC, BI_ERR, BI_MTX, BI_HRD (most) | ~24 | Independent, mostly single-file mechanical fixes; no shared dependencies |
| 2 | BI_CMD, BI_SUB, BI_COD, BI_PHN (most), BI_PRT | ~24 | Touches chat / dispatch surfaces; depends on Wave 1 stability |
| 3 | BI_STR, BI_PHN compaction/dreams, BI_CMD inline executors | ~8 | Need wave-2 streaming + dispatch in place |

Detailed per-batch dependencies are encoded in `batches.toml`.
