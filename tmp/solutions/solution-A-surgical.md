# Solution A — Surgical Batches (Minimum Viable Fix)

**Philosophy**: Fix the most user-visible problems with the least code churn. No architectural
redesigns. Patch what exists, wire what's disconnected, harden what's exposed.

**Total estimate**: ~35-40 hours
**Risk**: Low — each batch is independent, can be committed and tested separately.
**Downside**: Doesn't address S1/S10 (the two fundamental architecture gaps). Chat remains
a thin pipe. Two execution engines still coexist. Technical debt accumulates.

---

## Batch 1: Security Hardening (3h)

**What it solves**: S5 (security-off-by-default) — the only P0 issue set.
**Sources**: binary-issues/19, binary-issues/MASTER-INDEX S5, mori-diffs/GAP-11

All changes are in `crates/roko-serve/`:

| Change | File | LOC |
|---|---|---|
| Auth enabled by default | `src/lib.rs` (config defaults) | ~10 |
| Auto-enable auth when PORT is set | `src/lib.rs:225-233` | ~5 |
| Terminal routes inside auth middleware | `src/routes/mod.rs:137-138` | ~15 |
| CORS restricted to localhost by default | `src/routes/middleware.rs:426-437` | ~5 |
| Terminal command allowlist | `src/terminal.rs:115-125` | ~30 |
| PTY session count limit (16) | `src/terminal.rs:76` | ~10 |
| Private gists by default | `roko-cli/src/share.rs:84` | ~1 |
| LogScrubber on share payloads | `roko-cli/src/share.rs:80-90` | ~15 |

Also fix:
- `dangerously_skip_permissions: true` → configurable (not hardcoded) in `commands/plan.rs:290`
- Post-dispatch secret leak severity → Block in `safety/mod.rs:696`
- `#[cfg(test)]`-gate `AgentContract::permissive()` in `contract.rs:78`

**Test**: Run `roko serve` with `PORT=8080`, verify auth required. Try creating PTY without token.

---

## Batch 2: Connection Reuse (2-3h)

**What it solves**: S2 (throwaway HTTP clients) — directly halves latency for every API call.
**Sources**: binary-issues/12, binary-issues/MASTER-INDEX S2, subsystem-audits/provider-dispatch

| Change | File | LOC |
|---|---|---|
| Create one `reqwest::Client` at session start | `dispatch_direct.rs` (top of module) | ~20 |
| Thread `Arc<Client>` through all dispatch fns | `dispatch_direct.rs:290,372` | ~30 |
| Streaming path uses shared client | `openai_compat_backend.rs:318` | ~5 |
| Apply timeout config from provider | `dispatch_direct.rs:371-389` | ~15 |
| Config loaded once, cached in session | `dispatch_direct.rs:74-107` | ~20 |

**Test**: `roko` → send 3 messages back-to-back. Verify 2nd/3rd are faster (TLS reused).

---

## Batch 3: Wire Slash Commands (3-4h)

**What it solves**: S3 (confirmation theater) — commands that lie.
**Sources**: binary-issues/16, binary-issues/MASTER-INDEX S3

| Change | File | LOC |
|---|---|---|
| `/system` → set in dispatch messages | `chat_inline.rs:2134`, `dispatch_direct.rs` | ~15 |
| `/effort` → map to API `max_tokens` or thinking budget | `chat_inline.rs:2245` | ~20 |
| `/gate` → mutate runtime config | `chat_inline.rs:2304` | ~20 |
| `/config set` → write to roko.toml | `chat_inline.rs:2449` | ~30 |
| `tune gates --dry-run` → remove misleading flag or wire write | `learn.rs:84` | ~10 |
| Fix Share.tsx endpoint → `/api/shared/` | `demo/demo-app/src/pages/Share.tsx:28` | ~1 |

**Hard items** (deferred to Solution B/C): `/run` inline execution, `/plan run` inline.

**Test**: `/system You are a pirate` → next response should reflect the persona.

---

## Batch 4: Hardcoded Values Extraction (2-3h)

**What it solves**: S7 (hardcoded values scattered everywhere).
**Sources**: binary-issues/MASTER-INDEX S7, subsystem-audits/config-consolidation

| Change | File | LOC |
|---|---|---|
| Model defaults → constants module | New `roko-core/src/model_defaults.rs` | ~40 |
| API URLs/versions → provider config | `dispatch_direct.rs:300,302,374` | ~20 |
| max_tokens → config field | `dispatch_direct.rs:295,379` | ~15 |
| Opus pricing → config or cost_table | `chat_inline.rs:3516` | ~10 |
| CostTable → load from config, hardcoded fallback | `cost_table.rs:99-122` | ~30 |

**Test**: `roko config show` → shows effective model, URL, max_tokens values.

---

## Batch 5: Subprocess Safety (3-4h)

**What it solves**: S9 (subprocess management gaps).
**Sources**: binary-issues/16, binary-issues/MASTER-INDEX S9, binary-issues/04

| Change | File | LOC |
|---|---|---|
| Auth detection timeout (3s) | `auth_detect.rs:99-104` | ~5 |
| MCP stderr → log file | `mcp/client.rs:187` | ~10 |
| Claude CLI dispatch timeout (120s) | `dispatch_direct.rs:246` | ~10 |
| CancellationToken in chat dispatch | `chat_inline.rs:1299-1413` | ~40 |
| Chain-watcher handle stored + killed | `lib.rs:300-334` | ~15 |
| Non-blocking background serve | `unified.rs:48` | ~10 |
| Replace bare eprintln! with tracing | ~12 call sites in `claude_cli_agent.rs` | ~12 |

**Test**: Start `roko`, Ctrl+C during long response → should cancel cleanly.

---

## Batch 6: Mutex & Error Handling (3-4h)

**What it solves**: S11 (mutex/unwrap risks) + S4 partial (worst .ok() calls).
**Sources**: binary-issues/MASTER-INDEX S11, S4

| Change | File | LOC |
|---|---|---|
| Add `parking_lot` dep, swap Mutex | `roko-agent/Cargo.toml`, 4 call sites | ~20 |
| TOCTOU fix → `if let Some` | `orchestrate.rs:15268` | ~5 |
| `.expect("just registered")` → `.ok_or()` | `routes/feeds.rs:127` | ~5 |
| Remove crate-level lint suppression | `roko-agent/src/lib.rs:22` | ~1 |
| Audit top-10 worst `.ok()` in orchestrate.rs | `orchestrate.rs` (10 sites) | ~30 |
| Fail-fast on empty env var interpolation | `config.rs:2197` | ~10 |

**Test**: `cargo clippy --workspace` → no new warnings from removed suppression.

---

## Batch 7: Phantom Feature Wiring (3-4h)

**What it solves**: S8 (built but never wired).
**Sources**: binary-issues/20, binary-issues/MASTER-INDEX S8, subsystem-audits/learning

| Change | File | LOC |
|---|---|---|
| Auto-trigger compaction (on session start) | `chat_inline.rs` or `unified.rs` | ~10 |
| Persist LinUCB bandit weights in save() | `cascade_router.rs:1551` | ~30 |
| Wire MaxCostPerTurn to cumulative spend | `contract.rs:442`, tool dispatch | ~20 |
| Populate create_share with actual run data | `shared_runs.rs:89-104` | ~20 |
| Dream trigger → inline at plan completion | `orchestrate.rs` | ~15 |
| Add `roko episodes compact` CLI command | `roko-cli/src/commands/` | ~20 |

**Test**: Restart roko after 200+ observations → cascade router should be in UCB1 stage with
retained weights.

---

## What This Approach Does NOT Fix

| Gap | Why skipped | Impact |
|---|---|---|
| **S1 (dispatch = thin pipe)** | Requires session-scoped agent architecture | Chat has no tools, no history, no workspace context |
| **S6 (no streaming)** | Requires wiring StreamingState to dispatch | User still sees spinner then full response |
| **S10 (two engines)** | 21K LOC merge, high regression risk | Maintenance burden continues |
| **mori-diffs GAP-01-12** | Most require S1/S6/S10 first | Mori parity blocked |
| **converge-runner 55 issues** | Most are subsystem-level wiring | Built-not-wired pattern continues |

**Bottom line**: This gets you a secure, faster, more honest binary. But chat is still
fundamentally broken — no tools, no context, no streaming. The "Claude Code experience"
requires Solution B or C.
