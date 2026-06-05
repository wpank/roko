# Performance Optimization Checklist

**Created:** 2026-05-01
**Branch:** wp-arch2
**Last updated:** 2026-05-01

### Benchmark results

Before any fixes: `roko run "hello" --model glm51` → **77 seconds**
After reviewer fix: → **2.4s** (32x speedup)
After P0 batch: → **2.4–5.6s** (all LLM latency, <100ms framework overhead)

---

## P0 — High Impact (each saves 50ms–seconds per call)

- [ ] **1. Wire the warm agent pool**
  - `MultiAgentPool` in `crates/roko-agent/src/multi_pool.rs` is fully implemented but has zero call sites in production
  - `pre_spawn_warm`, `promote_warm`, `recycle_terminal_to_warm` — all unused
  - Every dispatch goes through `create_agent_for_model` cold
  - For Claude CLI backends: fresh subprocess spawn per call
  - For API backends: HTTP client is shared but agent object rebuilt from scratch
  - **Fix:** Wire `MultiAgentPool` into `EffectDriver` and `roko-serve/dispatch.rs`. Pre-spawn warm agents for configured models on startup. Promote from pool instead of cold-creating.
  - **Files:** `crates/roko-agent/src/multi_pool.rs`, `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-serve/src/dispatch.rs`

- [x] **2. Cache `ProviderSemaphores` — stop rebuilding per call** ✅
  - `provider/mod.rs:280` — `ProviderSemaphores::new()` allocates a fresh `HashMap<String, Arc<Semaphore>>` on every `create_agent_for_model` call
  - `ModelCallService` never pre-supplies them via `AgentOptions.provider_semaphores`
  - **Fix:** Create `ProviderSemaphores` once in `ModelCallService::with_config()` or `ServiceFactory::build`, store as `Arc`, pass into every `AgentOptions`.
  - **Files:** `crates/roko-agent/src/provider/mod.rs:280`, `crates/roko-agent/src/model_call_service.rs`

- [x] **3. Eliminate per-call `RokoConfig` clone** ✅
  - `model_call_service.rs:321` — `config_for_model()` does `self.config.clone()` unconditionally
  - Clones all `HashMap<String, ProviderConfig>` and `HashMap<String, ModelProfile>`
  - In the common case (model already configured), only a read is needed
  - **Fix:** Use `Cow<'_, RokoConfig>` or `Arc<RokoConfig>` — only clone when mutation is actually needed (dynamic model insertion).
  - **Files:** `crates/roko-agent/src/model_call_service.rs:321`

- [x] **4. Fix blocking filesystem walk in prompt assembly** ✅
  - `prompt_assembly_service.rs:676` — `collect_source_context_from()` does synchronous recursive `std::fs::read_dir` + `read_to_string` on the tokio async executor thread
  - Reads up to 12 source files from `<workdir>/src/` on every `assemble()` call
  - No `spawn_blocking`, no caching
  - Also: `read_to_string_if_exists` for `Cargo.toml` is blocking
  - **Fix:** Cache conventions per workdir (they don't change between calls in the same run). Compute once in `ServiceFactory::build` or on first call, then reuse. If caching isn't possible, wrap in `spawn_blocking`.
  - **Files:** `crates/roko-compose/src/prompt_assembly_service.rs:676-719`

- [x] **5. Fix `EpisodeLogger::read_all()` — stop reading entire file** ✅
  - `prompt_assembly_service.rs:343` — reads and deserializes the entire growing `.roko/episodes.jsonl`, then takes last 5
  - No reverse-read, no tail-seek, no caching
  - Gets worse over time as episodes accumulate
  - **Fix:** Read the file in reverse (seek to end, read backwards until 5 newlines found) or cache the last N episodes in memory and update incrementally.
  - **Files:** `crates/roko-compose/src/prompt_assembly_service.rs:343-349`, `crates/roko-learn/src/episode_logger.rs`

- [x] **6. Remove `count_changed_files` subprocess** ✅
  - `effect_driver.rs:246` — spawns `git diff --name-only HEAD` after every successful agent completion
  - Result stored in `PipelineStateV2.files_changed` but nothing ever branches on it
  - Pure latency (50-200ms subprocess) with zero behavioral effect
  - **Fix:** Remove the call entirely. If the field is needed later, make it lazy (compute only when accessed).
  - **Files:** `crates/roko-runtime/src/effect_driver.rs:246`

- [ ] **7. Use cached `CascadeRouter` instead of reloading from disk**
  - `roko-serve/src/dispatch.rs` — `record_cascade_router_outcome_with_layout` calls `CascadeRouter::load_or_new` + `save` on every dispatch outcome
  - Completely bypasses the already-loaded `AppState.cascade_router: RwLock<Option<CascadeRouter>>`
  - Also: `routes/providers.rs:173` loads a fresh one per HTTP request
  - **Fix:** Use `AppState.cascade_router` everywhere. Update in-memory, persist asynchronously (debounced write-back).
  - **Files:** `crates/roko-serve/src/dispatch.rs`, `crates/roko-serve/src/routes/providers.rs:173`

- [ ] **8. Fix PTY fork/exec blocking the tokio runtime**
  - `roko-serve/src/terminal.rs` — `create_session_inner` calls blocking `spawn_command` (fork + execve) on the tokio thread pool without `spawn_blocking`
  - Each session also spawns a raw OS thread for PTY reads
  - **Fix:** Wrap PTY creation in `tokio::task::spawn_blocking`. Consider pre-spawning a small pool of PTY sessions.
  - **Files:** `crates/roko-serve/src/terminal.rs`

---

## P1 — Medium Impact (20–200ms per occurrence)

- [x] **9. Cache services across workflow invocations** ✅
  - `run.rs` `build_workflow_effect_services` loads `cascade-router.json`, `section-effects.json`, `daimon.json` (including KD-tree rebuild) from disk on every call
  - For `plan run` with N tasks, this is N full disk loads
  - **Fix:** Build services once per plan run, pass through. Only reload if config changes.
  - **Files:** `crates/roko-cli/src/run.rs` (ServiceFactory::build callers)

- [ ] **10. Deduplicate `config::load_config` calls**
  - `run.rs` has 4+ independent call sites each doing `fs::read_to_string` + two TOML parses + two mutation passes
  - **Fix:** Load config once at dispatch entry point, thread through as `Arc<RokoConfig>`.
  - **Files:** `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/commands/util.rs`

- [ ] **11. Move convergence detection off the hot path**
  - `model_call_service.rs:1562` — O(n²) Levenshtein edit distance runs synchronously on tokio thread after every model response
  - **Fix:** Move to `spawn_blocking` or make async. Consider sampling (only check every Nth response).
  - **Files:** `crates/roko-agent/src/model_call_service.rs:1562`

- [ ] **12. Parallelize knowledge store queries**
  - `prompt_assembly_service.rs:422` — `query_techniques()` and `query_anti_patterns()` run sequentially with separate full-file scans
  - **Fix:** Run both queries concurrently via `tokio::join!` or batch into a single store scan.
  - **Files:** `crates/roko-compose/src/prompt_assembly_service.rs:422-423`

- [ ] **13. Deduplicate CascadeRouter load at startup**
  - `ServiceFactory::build` loads `cascade-router.json`, then `build_app_state` loads the same file again into `AppState.cascade_router`
  - **Fix:** Load once, share the instance.
  - **Files:** `crates/roko-serve/src/lib.rs`

---

## P2 — Frontend / Perceived Responsiveness

- [ ] **14. Reduce synthetic dead time per command**
  - `useTerminal.ts:229` — `waitForPrompt` has mandatory 50ms pre-sleep + 60ms stability sleep = 110ms per command
  - Over 10 commands = 1.1s of pure nothing
  - **Fix:** Use event-driven prompt detection (fire on `appendOutput` match) instead of polling. Remove pre-sleep.
  - **Files:** `demo/demo-app/src/hooks/useTerminal.ts:229`

- [ ] **15. Shorten pre-run animation**
  - `ScenarioSlot.tsx` — 550ms intro + 2400ms countdown (3×800ms) + 600ms blackout = 3.55s before terminals connect
  - **Fix:** Reduce countdown to 2×400ms = 800ms. Overlap intro dismiss with blackout. Target <1s total.
  - **Files:** `demo/demo-app/src/components/ScenarioSlot.tsx`

- [ ] **16. Fix `elapsedMs` re-render cascade**
  - `ScenarioSlot.tsx:357` — 250ms interval updates `elapsedMs` in state, which is in `onStateChange` dep array
  - Causes full `Demo` top-level re-render 4×/sec including all tabs, controls, status bar
  - **Fix:** Move elapsed time to a ref, only update state on meaningful changes. Remove `elapsedMs` from `onStateChange` deps — use a separate, less frequent update path for the timer display.
  - **Files:** `demo/demo-app/src/components/ScenarioSlot.tsx:357,411`

- [ ] **17. Make `rawSleep` calls respect speed multiplier**
  - 8+ call sites in `chat.ts`, `dream-consolidation.ts`, `gate-retry.ts` use `rawSleep` instead of `adjustedSleep`
  - Speed multiplier button has zero effect on these scenarios
  - **Fix:** Replace `rawSleep` with `adjustedSleep` in all scenario pacing delays. Keep `rawSleep` only for true wall-clock needs (WS connect timeouts).
  - **Files:** `demo/demo-app/src/lib/scenario-runners/chat.ts`, `dream-consolidation.ts`, `gate-retry.ts`

- [ ] **18. Parallelize all `enterWorkspace` calls**
  - `explore.ts:34`, `providers.ts:29` — first terminal entered sequentially (up to 18s), then rest in parallel
  - **Fix:** `Promise.all(entries.map(...))` for all terminals. No sequential first entry.
  - **Files:** `demo/demo-app/src/lib/scenario-runners/explore.ts`, `providers.ts`, `provider-race.ts`

- [ ] **19. Reuse `TextDecoder` instance**
  - `useTerminal.ts:379` — `new TextDecoder()` on every binary WS message
  - During streaming output, fires many times per second
  - **Fix:** Allocate once outside the handler, reuse.
  - **Files:** `demo/demo-app/src/hooks/useTerminal.ts:379`

- [ ] **20. Reduce `trackMetrics` interval**
  - `terminal-session.ts:344` — defaults to 500ms, sidebar cost/tokens lag
  - **Fix:** Default to 150ms or make event-driven (fire on `appendOutput` when cost pattern detected).
  - **Files:** `demo/demo-app/src/lib/terminal-session.ts:344`

- [ ] **21. Narrow `MutationObserver` scope on xterm**
  - `TerminalPaneWithHandle.tsx:85` — `characterData: true, subtree: true` on xterm viewport
  - Fires on every DOM mutation during terminal output
  - **Fix:** Use xterm's `onData` or `onRender` event instead of a broad MutationObserver.
  - **Files:** `demo/demo-app/src/components/TerminalPaneWithHandle.tsx:85`

---

## P3 — Minor / Long-term

- [ ] **22. Remove `SharedHttpPoster` double vtable indirection** — `backends/mod.rs`
- [ ] **23. Replace two-mutex LRU cache** — `model_call_service.rs:869`, O(n) `retain` per store
- [ ] **24. Batch `to_lowercase()` in `build_knowledge_advice`** — per entry×slug heap alloc
- [ ] **25. Optimize `scrub_secrets` middleware** — runs on every `/api/*` response body
- [ ] **26. Index `try_agent_token` by hash** — O(N) agent scan per Bearer auth request
- [ ] **27. Fix `logCommandComplete` O(n) scan** — array spread per command completion
- [ ] **28. Bind server port before background task init** — "bind first, init second" pattern
- [ ] **29. Replace `mirage.ts` shell `sleep 1`** — 3s wall-clock unreachable by speed control
