# Gates, safety, and supervisor wiring

**Status:** Not started
**Priority:** High
**Crates touched:** `roko-gate`, `roko-agent`, `roko-runtime`, `roko-compose`, `roko-cli`

---

## Scope

Five gaps where existing code is built but not connected to the runtime:

- **Gap A** — Gate rungs 3-6 stub-pass when inputs missing (rungs 5-6 always; rungs 3-4 when source_roots/test artifacts absent)
- **Gap B** — `SafetyLayer` is not enforced for Claude CLI (the default backend)
- **Gap C** — `ProcessSupervisor.spawn()` is never called during plan execution
- **Gap D** — `vcg_allocate` is dead code; CLAUDE.md falsely claims it is wired
- **Gap E** — `LearningBidder` posteriors are never registered with `PromptComposer`

Completing all five makes the verification ladder honest, the safety layer
operational for the default backend, process tracking accurate, the auction
claim true, and prompt section selection data-driven.

---

## Implementation checklist

### Gap A: wire gate rung 3-6 oracles (stub-pass scope wider than originally documented)

**Why it matters:** `stub_verdict()` in `crates/roko-gate/src/rung_dispatch.rs:132`
returns `Verdict::pass()`. Any task output that reaches rung 5 or 6 passes the
gate unconditionally. **Additionally, rungs 3 and 4 also stub-pass** when their
required inputs are missing:

- **Rung 3 `SymbolGate`:** stubs at `rung_dispatch.rs:146,149` when
  `symbol_signal` or `source_roots` are `None`. The gate cannot resolve symbols
  without knowing where the source lives, so it returns `stub_verdict("symbol
  gate — no source roots")` instead of performing real symbol resolution.
- **Rung 4 `GeneratedTestGate`:** stubs at `rung_dispatch.rs:173` when
  `generated_test_artifacts` is not wired. The gate cannot verify test coverage
  without knowing which test files were generated, so it returns
  `stub_verdict("generated test gate — no artifacts")`.
- **Rung 4 `VerifyChainGate`:** stubs at `rung_dispatch.rs:186` when no
  `verify_script` tag is present in the task metadata. Without a script to run,
  the gate cannot verify chain witnesses.

This makes the verification pipeline a false-positive machine across **four
rungs (3-6)**, not just two. Every task that does not explicitly configure
source roots, test artifacts, and verify scripts passes rungs 3-6
unconditionally. In practice, almost no tasks configure these inputs, so the
gate pipeline is effectively a 3-rung pipeline (0-2) with a rubber stamp for
the rest.

#### A-1: rename `stub_verdict` to return `Verdict::skip()` (or equivalent)

- File: `crates/roko-gate/src/rung_dispatch.rs`
- Lines 132-138: `stub_verdict` builds a passing verdict. Change it to emit
  a skipped/advisory verdict that does not count as a pass.
- Verdict needs a `skip` state. Check `roko_core::Verdict` — if no `skip`
  variant exists, add one (or use a `status: VerdictStatus` enum). The key
  constraint: a skipped verdict must NOT count as `passed = true`.
- Every call site in `rung_dispatch.rs` (symbol: 145, 148; generated_test: 172;
  verify_chain: 185; fact_check: 200, 203; llm_judge: 219, 222; integration: 236)
  passes a label string to `stub_verdict`. Those strings are fine; keep them.
- Anti-pattern: do not replace stub verdicts with `Verdict::fail()`. A missing
  oracle is a configuration gap, not a quality failure. Fail is reserved for
  actual gate evaluation that finds a problem.

#### A-2: add `FactCheckOracle` implementation backed by Perplexity backend

- File: `crates/roko-gate/src/fact_check.rs`
- The `SearchOracle` trait is already defined at line 57. No existing production
  implementation exists in the codebase (confirmed: no `PerplexitySearchClient`
  in any crate).
- Create `crates/roko-gate/src/perplexity_oracle.rs` (new file):
  - Struct `PerplexitySearchOracle` holds a base URL and API key.
  - Implement `SearchOracle::search` via `reqwest::Client` POST to
    `https://api.perplexity.ai/chat/completions` with `model: "sonar"`.
  - Parse the response; extract the first assistant message content as a
    single `SearchHit`.
  - If `PERPLEXITY_API_KEY` env var is unset, return `Err("no Perplexity API key")`.
  - Gate `PerplexitySearchOracle` behind a `perplexity` Cargo feature so it
    does not force `reqwest` into builds that do not need it.
- Add `pub mod perplexity_oracle;` and re-export in `crates/roko-gate/src/lib.rs`.

#### A-3: add `LlmJudgeOracle` implementation backed by the haiku model

- File: `crates/roko-gate/src/llm_judge_gate.rs`
- `JudgeOracle` trait is at line 53. No production implementation exists.
- Create `crates/roko-gate/src/haiku_judge_oracle.rs` (new file):
  - Struct `HaikuJudgeOracle` holds an Anthropic API key and model name
    (default: `claude-haiku-4-5`).
  - Implement `JudgeOracle::judge` via a direct Anthropic messages POST.
  - Extract a float from the response using a simple parse: find the first
    float-parseable token in the response text. Clamp to `[0.0, 1.0]`.
  - If `ANTHROPIC_API_KEY` is unset, return `Err("no Anthropic API key")`.
  - Gate behind the same `perplexity` feature or a separate `llm-judge` feature.
- Add `pub mod haiku_judge_oracle;` and re-export in `lib.rs`.

#### A-4: wire both oracles in `enrich_rung_config()`

- File: `crates/roko-cli/src/orchestrate.rs`
- Function `enrich_rung_config` at line 15655.
- Current implementation sets `generated_test_artifacts` (rung 4) and
  `integration_test_pattern` (rung 6). It never touches `fact_check_oracle`
  or `llm_judge_oracle`.
- Add after the integration block (line 15683):

```rust
// Rung 5: wire FactCheckOracle when Perplexity key is available.
if (rung == 5 || rung > 6) && config.fact_check_oracle.is_none() {
    if let Ok(key) = std::env::var("PERPLEXITY_API_KEY") {
        config.fact_check_oracle =
            Some(Arc::new(PerplexitySearchOracle::new(key)));
    }
}

// Rung 6: wire LlmJudgeOracle when Anthropic key is available.
if (rung == 6 || rung > 6) && config.llm_judge_oracle.is_none() {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        config.llm_judge_oracle =
            Some(Arc::new(HaikuJudgeOracle::new(key)));
    }
}
```

- Add the necessary imports at the top of `orchestrate.rs` (or within the
  function, under `#[cfg(feature = "perplexity")]` guards if needed).
- Anti-pattern: do not panic or return an error when the key is absent. The
  oracle is best-effort; the stub path handles absence correctly after A-1.

#### A-5: add integration test for rung 5 and rung 6 with mock oracles

- File: `crates/roko-gate/tests/rung_5_6_integration.rs` (new file)
- Test `rung_5_passes_with_mock_oracle`:
  - Construct `RungExecutionConfig` with a mock `SearchOracle` that always
    returns one hit.
  - Call `run_rung(signal, ctx, 5, &inputs, &config)`.
  - Assert all verdicts have `passed = true` and none have `gate == "stub"`.
- Test `rung_5_skips_without_oracle`:
  - Construct `RungExecutionConfig` with no `fact_check_oracle`.
  - Call `run_rung(signal, ctx, 5, &inputs, &config)`.
  - Assert no verdict has `passed = true` (they are skipped, not passed).
- Same pattern for rung 6 using a mock `JudgeOracle`.

---

### Gap B: safety layer enforcement for Claude CLI

**Why it matters:** `authorize_call_with_taint()` at
`crates/roko-agent/src/safety/mod.rs:430` is called only in tests (lines
1232-1291). Claude CLI is the default backend (`crates/roko-agent/src/claude_cli_agent.rs` and `crates/roko-agent/src/provider/claude_cli.rs`).
Claude runs its own internal tool loop — the `ToolDispatcher` safety checks
only fire in the Ollama/OpenAI-compat tool-loop path. The safety layer is
structurally bypassed for the dominant backend.

#### B-1: compute the allowed tool list and pass it via `--allowedTools`

- File: `crates/roko-agent/src/claude_cli_agent.rs` (the main Claude CLI
  agent implementation) and `crates/roko-agent/src/provider/claude_cli.rs`
  (the provider-level wrapper).
  **NOTE:** The file is NOT at `crates/roko-agent/src/backends/claude_cli.rs`
  as previously stated. That path does not exist.
- Look for where the `claude` subprocess is constructed (the `Command::new`
  call that builds the Claude CLI invocation).
- **CRITICAL:** The `--disallow-tools` flag does NOT exist in Claude CLI. The
  real mechanism is `--allowedTools` which is an allowlist-only approach. To
  enforce tool restrictions, compute the effective allowed tool set as:
  `(all available tools) - (denied tools from SafetyLayer)` and pass the
  result via `--allowedTools tool1,tool2,...`.
- The `SafetyLayer` restricts tools via `role_tools` which is a whitelist,
  not a deny list. There is no separate "deny list" field. Extract the
  `role_tools` allowlist from the safety layer and pass it directly as
  `--allowedTools`. If `role_tools` is empty (all tools allowed), omit the
  flag entirely.
- The `current_safety_layer()` function is at
  `crates/roko-agent/src/provider/mod.rs:276`, NOT `agent_spawn.rs:100`.
  Pull the role_tools allowlist from the active layer before spawning the
  subprocess.
- This is not a complete solution but it is the correct first layer: Claude
  only sees the allowed tools in its internal loop.

#### B-2: wire `authorize_call_with_taint()` in the ToolDispatcher (all non-Claude backends)

- File: `crates/roko-agent/src/dispatcher/mod.rs`
- Line 331: `safety.check_pre_execution()` is called per tool call.
  **Note:** `ToolDispatcher::dispatch()` is used for ALL non-Claude backends
  (Ollama, OpenAI-compat, Gemini, Codex, etc.), not just Ollama. Any fix here
  applies to every backend that routes through the dispatcher's tool loop.
- The dispatcher path already has a taint label available from the signal context.
  Replace the direct `check_pre_execution` call with
  `authorize_call_with_taint(call, ctx, taint.as_ref())` and act on the
  returned `AuthzDecision`:
  - `Allow` → proceed
  - `AllowWithConfirm` → log a warning and proceed (operator confirmation is
    not interactive in headless mode; the audit log serves as the record)
  - `Deny` → return `ToolError::PermissionDenied`
- The `ToolDispatcher` at line 108 already has `safety: None` defaulting. No
  structural change is needed; the wire is already present. The gap is that
  `check_pre_execution` is called directly instead of going through the full
  `authorize_call_with_taint` path.

#### B-3: add a contract-enforcement integration test

- File: `crates/roko-agent/tests/safety_contract_integration.rs` (new file)
- Test `unlisted_tool_blocked_by_dispatcher`:
  - Create a `SafetyLayer` with a `role_tools` whitelist that does NOT
    include `"web_fetch"`. (**Note:** SafetyLayer has no "deny list" field;
    tool restriction is via the `role_tools` whitelist. A tool not in the
    whitelist is implicitly denied.)
  - Attach it to a `ToolDispatcher`.
  - Dispatch a `ToolCall` for `"web_fetch"`.
  - Assert the result is `ToolError::PermissionDenied`.
- Test `taint_escalates_to_allow_with_confirm_in_dispatcher`:
  - Create a `SafetyLayer` with a `role_tools` whitelist that includes the
    tool, then call `authorize_call_with_taint` with `Taint::ExternalFetch`.
  - Assert `AuthzDecision::AllowWithConfirm` is returned.
  - Confirm this is logged (check `tracing` subscriber or mock).

---

### Gap C: wire `ProcessSupervisor.spawn()` during plan execution

**Why it matters:** `ProcessSupervisor` exists in `PlanRunner` at
`crates/roko-cli/src/orchestrate.rs:3060`. `shutdown_all()` is called at
line 5571. `count()` is called at lines 6113 and 6449. But `spawn()` at
`crates/roko-runtime/src/process.rs:411` is never called from orchestrate.rs
or agent_exec.rs. The supervisor count is always 0.

#### C-1: locate where Claude CLI subprocesses are actually spawned

- File: `crates/roko-cli/src/agent_spawn.rs`
- Functions `spawn_agent_scoped` (line 92) and `spawn_agent_with_layer`
  (line 110) call `create_agent_for_model`, which eventually calls into
  `crates/roko-agent/src/claude_cli_agent.rs` (or `crates/roko-agent/src/provider/claude_cli.rs`).
- The `claude` subprocess is spawned inside the Claude CLI backend when
  `Agent::run` is called. The `tokio::process::Child` handle is held inside
  the backend struct, not exposed to the supervisor.

#### C-2: expose the OS process ID from the Claude CLI backend

- File: `crates/roko-agent/src/claude_cli_agent.rs`
- After spawning the `tokio::process::Child`, store its OS PID
  (`child.id()`) in an `Arc<AtomicU32>` or expose it via a method on the
  backend.
- The `Agent` trait in `roko-core` currently has no `os_pid()` method. Add
  an optional method:

```rust
// in roko-core Agent trait
fn os_pid(&self) -> Option<u32> { None }
```

- Implement `os_pid()` in the Claude CLI backend to return the held PID.

#### C-3: register the subprocess with `ProcessSupervisor` after spawn

- File: `crates/roko-cli/src/orchestrate.rs`
- After `dispatch_agent_with(...)` (the function that calls into agent_exec
  and returns), get the agent's OS PID and register it:

```rust
// After spawn, if the agent exposes an OS pid:
if let Some(pid) = agent.os_pid() {
    let config = SpawnConfig {
        program: "claude".into(),
        label: format!("agent-{plan_id}-{task}"),
        ..SpawnConfig::default()
    };
    // Register via the supervisor's tracking map, not re-spawn.
    self.supervisor.register_external(pid, config).await;
}
```

- `ProcessSupervisor::spawn()` creates and spawns a new process. That is not
  what is needed here — the process is already running. **Note:**
  `register_external()` does NOT currently exist on `ProcessSupervisor` -- it
  must be added as a new method. Add
  `register_external(os_pid: u32, config: SpawnConfig)` to `ProcessSupervisor`
  in `crates/roko-runtime/src/process.rs` that inserts an externally spawned
  process into the tracking map without calling `Command::spawn`. This requires:
  1. A new `TrackedProcess` variant or field that holds just an OS PID without
     owning a `tokio::process::Child` handle.
  2. The tracking map key type must accommodate external PIDs.
  3. `shutdown_all()` must handle external PIDs by sending `SIGTERM` directly
     rather than calling `child.kill()`.
- Alternatively: restructure the Claude CLI backend so the supervisor is
  responsible for spawning it. Pass `Arc<ProcessSupervisor>` into the backend
  factory and call `supervisor.spawn(config).await?` instead of calling
  `Command::spawn` directly. This is the cleaner path.

#### C-4: verify `count()` returns non-zero after agent dispatch

- File: `crates/roko-cli/src/orchestrate.rs` line 6113
- The `conductor_system_snapshot()` at line 6317 reads `supervisor.count()`.
  After this fix, the snapshot's `active_agents` field should reflect real
  running processes.
- Add an assertion in the integration test suite:
  - Dispatch one agent task in a test plan.
  - Poll `supervisor.count()` within 500ms.
  - Assert count >= 1.

---

### Gap D: resolve the VCG auction dead code

**Why it matters:** CLAUDE.md line says "VCG auction in composition — Wired."
This is false. `vcg_allocate` at `crates/roko-compose/src/auction.rs:293` is
called only from three unit tests. The production path uses
`select_optional_candidates` at `crates/roko-compose/src/prompt.rs:666`.

There are two valid resolutions. Pick one:

#### Option D-1 (preferred): remove the false claim, document reality

- File: `/Users/will/dev/nunchi/roko/roko/CLAUDE.md`
- Change "VCG auction in composition — Wired" to "Greedy knapsack in
  composition — Wired (`select_optional_candidates` + VCG payment summary)".
- File: `crates/roko-compose/src/auction.rs`
- Add a doc comment on `vcg_allocate` that is explicit: "This function is not
  called from the production prompt composition path. The production path uses
  `select_optional_candidates` in `prompt.rs`. This implementation is retained
  as a reference for future A/B testing."
- No code changes needed beyond documentation.

#### Option D-2: replace the greedy path with `vcg_allocate`

- File: `crates/roko-compose/src/prompt.rs`, line 491
- Replace the call to `select_optional_candidates` with a call to `vcg_allocate`:
  - Convert `optional: Vec<AuctionCandidate>` to `Vec<VcgBid>` (one `VcgBid`
    per candidate: `name` = section name, `tokens` = estimated tokens,
    `adjusted_bid` = bid density * estimated tokens).
  - Call `vcg_allocate(bids, remaining_tokens, &affect.as_ref().map_or_else(AffectModulation::default, |a| a.into()))`.
  - Map `VcgAllocation::winners` back to `SelectedCandidate` indices.
- This is a behavioral change. Run the full prompt composition tests before
  committing. The `vcg_payment_summary` call at line 498 can be removed since
  `vcg_allocate` already computes payments.
- Update CLAUDE.md to match.

**Recommendation:** Do D-1 first (immediate, no behavioral risk). File D-2 as
a separate task if VCG is wanted for its payment guarantees.

---

### Gap E: register learned bidder posteriors with `PromptComposer`

**Why it matters:** `PromptComposer` has `learning_bidders: HashMap<AttentionBidder, LearningBidder>`
at `crates/roko-compose/src/prompt.rs:295`. `register_bidder()` (line 344) and
`with_learning_bidders()` (line 350) are never called from `roko-cli`. At
`PromptComposer::new()` in `system_prompt_builder.rs:773`, the map is empty. The
multiplication at lines 458-462 always evaluates `unwrap_or(1.0)` — a no-op.
The `PromptComposer::new()` call in `orchestrate.rs:13980` also creates an
empty composer.

#### E-1: define a loader for bidder posteriors from efficiency events

- File: `crates/roko-cli/src/orchestrate.rs` — new helper function
  `load_learning_bidders(workdir: &Path) -> HashMap<AttentionBidder, LearningBidder>`

Logic:
1. Read `.roko/learn/efficiency.jsonl` (each line is an `EfficiencyEvent`).
2. For each event, extract: which prompt sections were included, whether the
   subsequent gate passed, and the `AttentionBidder` tag.
3. Call `LearningBidder::update(section_name, was_included, gate_passed)` for
   each observation.
4. Return the resulting map.

The `EfficiencyEvent` type lives in `crates/roko-learn/`. Check the struct
fields to confirm which fields carry section inclusion data. If the event
does not currently record included sections, add that field to the event
type (this is a separate task, but note the dependency here).

#### E-2: wire loaded bidders into the `PromptComposer` before composition

- File: `crates/roko-cli/src/orchestrate.rs`, line 13980
- Replace:

```rust
let composer = PromptComposer::new();
```

with:

```rust
let learned_bidders = load_learning_bidders(&self.workdir);
let composer = if learned_bidders.is_empty() {
    PromptComposer::new()
} else {
    PromptComposer::new().with_learning_bidders(learned_bidders)
};
```

- The same change applies wherever `PromptComposer::new()` is called in
  `crates/roko-compose/src/system_prompt_builder.rs:773`.

#### E-3: update bidders after gate outcomes

- File: `crates/roko-cli/src/orchestrate.rs`
- After each gate verdict is recorded (after `run_gate_rung`), update the
  learning bidders with the outcome and persist them:

```rust
// After verdicts are collected, use the existing update_bidders() method
// at PromptComposer line 362 rather than manual iteration:
composer.update_bidders(&verdicts, &last_prompt_sections_by_bidder);
// Persist back to efficiency.jsonl or a dedicated learning file.
```

**Note:** `PromptComposer::update_bidders()` already exists at line 362 of
`crates/roko-compose/src/prompt.rs`. Use it instead of manually iterating
over bidders. The method accepts verdict data and updates the internal
`learning_bidders` map. Less new code is needed than the manual iteration
above implies.

- Consider a dedicated file `.roko/learn/bidder-posteriors.json` serialized
  as `HashMap<AttentionBidder, LearningBidder>` to avoid re-deriving from
  the full event log on every task.

#### E-4: add a test for posterior propagation

- File: `crates/roko-compose/tests/learning_bidder_integration.rs` (new file)
- Test `bidder_posterior_affects_section_selection`:
  - Create a `LearningBidder` with `section_betas` showing one section
    historically passes 9 out of 10 times and another 1 out of 10.
  - Register both via `composer.register_bidder(...)`.
  - Compose a prompt with both sections competing for the same token budget.
  - Assert the high-pass section is selected.
- Test `empty_bidder_map_is_no_op`:
  - Create a `PromptComposer::new()` with no registered bidders.
  - Confirm composition succeeds and the multiplier does not change any bids
    (all multiply by `1.0`).

---

## Concrete file touchpoints

| Gap | File | Lines | Change |
|-----|------|-------|--------|
| A-1 | `crates/roko-gate/src/rung_dispatch.rs` | 132-138 | `stub_verdict` returns skip, not pass |
| A-1 | `roko-core` (Verdict) | — | Add `skip` state or `VerdictStatus::Skipped` |
| A-2 | `crates/roko-gate/src/perplexity_oracle.rs` | new | `PerplexitySearchOracle` impl |
| A-2 | `crates/roko-gate/src/lib.rs` | — | Add module + re-export |
| A-3 | `crates/roko-gate/src/haiku_judge_oracle.rs` | new | `HaikuJudgeOracle` impl |
| A-3 | `crates/roko-gate/src/lib.rs` | — | Add module + re-export |
| A-4 | `crates/roko-cli/src/orchestrate.rs` | 15683 | Wire oracles in `enrich_rung_config` |
| A-5 | `crates/roko-gate/tests/rung_5_6_integration.rs` | new | Integration tests |
| B-1 | `crates/roko-agent/src/claude_cli_agent.rs` + `crates/roko-agent/src/provider/claude_cli.rs` | — | Compute allowed tool list and pass `--allowedTools` to subprocess |
| B-2 | `crates/roko-agent/src/dispatcher/mod.rs` | ~331 | Call `authorize_call_with_taint` |
| B-3 | `crates/roko-agent/tests/safety_contract_integration.rs` | new | Contract tests |
| C-1 | `crates/roko-agent/src/claude_cli_agent.rs` | — | Expose `os_pid()` |
| C-2 | `roko-core` Agent trait | — | Add optional `os_pid()` method |
| C-3 | `crates/roko-runtime/src/process.rs` | — | Add `register_external()` to supervisor |
| C-3 | `crates/roko-cli/src/orchestrate.rs` | post-dispatch | Call `supervisor.register_external()` |
| D-1 | `CLAUDE.md` | — | Correct the VCG claim |
| D-1 | `crates/roko-compose/src/auction.rs` | 285 | Add accurate doc comment on `vcg_allocate` |
| E-1 | `crates/roko-cli/src/orchestrate.rs` | new fn | `load_learning_bidders` |
| E-2 | `crates/roko-cli/src/orchestrate.rs` | 13980 | Wire bidders into `PromptComposer` |
| E-2 | `crates/roko-compose/src/system_prompt_builder.rs` | 773 | Wire bidders into `PromptComposer::new()` |
| E-3 | `crates/roko-cli/src/orchestrate.rs` | post-gate | Update and persist bidder posteriors |
| E-4 | `crates/roko-compose/tests/learning_bidder_integration.rs` | new | Posterior propagation tests |

---

## Verification checklist

Run each command after the corresponding gap is closed.

### After Gap A

```bash
# Confirm stub_verdict no longer returns passed = true.
cargo test -p roko-gate stub_verdict -- --nocapture

# Confirm rung 5 skips (not passes) without an oracle.
cargo test -p roko-gate rung_5_skips_without_oracle -- --nocapture

# Confirm rung 5 passes with a mock oracle.
cargo test -p roko-gate rung_5_passes_with_mock_oracle -- --nocapture

# Confirm rung 6 skips without an oracle.
cargo test -p roko-gate rung_6_skips_without_oracle -- --nocapture

# Confirm full pipeline does not regress on rungs 0-4.
cargo test -p roko-gate -- --nocapture
```

### After Gap B

```bash
# Safety dispatcher test.
cargo test -p roko-agent deny_listed_tool_blocked_by_dispatcher -- --nocapture

# Taint escalation test.
cargo test -p roko-agent taint_escalates_to_allow_with_confirm -- --nocapture

# Existing safety tests still pass.
cargo test -p roko-agent safety -- --nocapture
```

### After Gap C

```bash
# Confirm supervisor count is non-zero after agent dispatch.
cargo test -p roko-cli supervisor_count_nonzero_after_dispatch -- --nocapture

# Confirm shutdown_all shuts down the registered agents.
cargo test -p roko-runtime spawn_and_reap -- --nocapture
```

### After Gap D

```bash
# No test required for the doc-only path.
# If D-2 (replace algorithm) was chosen:
cargo test -p roko-compose prompt_composition -- --nocapture
cargo clippy -p roko-compose -- -D warnings
```

### After Gap E

```bash
# Bidder posterior affects selection.
cargo test -p roko-compose bidder_posterior_affects_section_selection -- --nocapture

# Confirm empty bidder map is a no-op.
cargo test -p roko-compose empty_bidder_map_is_no_op -- --nocapture

# Confirm the full system still compiles and tests pass.
cargo test --workspace -- --nocapture
```

### Full pre-commit gate

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Acceptance criteria

**Gap A is closed when:**

1. `cargo test -p roko-gate stub_verdict` passes AND the test confirms the
   returned verdict has `passed = false`.
2. `run_rung(signal, ctx, 5, &inputs_with_no_oracle, &config_with_no_oracle)`
   returns verdicts where every element has `passed = false` (skipped).
3. `run_rung(signal, ctx, 5, &inputs_with_signal, &config_with_mock_oracle)`
   returns verdicts where the fact-check verdict has `passed = true` when the
   mock oracle confirms the claims.
4. All existing rung 0-4 tests continue to pass.
5. In a live plan run with `PERPLEXITY_API_KEY` set, the gate logs show
   `fact_check` verdicts with real search queries, not stub messages.

**Gap B is closed when:**

1. The Claude CLI subprocess invocation includes `--allowedTools <name1>,<name2>,...`
   with only the role-permitted tools (visible via `RUST_LOG=debug roko plan run`).
   Tools not in the `role_tools` allowlist are excluded from the `--allowedTools` list.
2. `ToolDispatcher::dispatch` returns `ToolError::PermissionDenied` for a tool
   not in the `role_tools` allowlist across all non-Claude backends (Ollama,
   OpenAI-compat, Gemini, Codex, etc.).
3. `authorize_call_with_taint` with `Taint::ExternalFetch` returns
   `AuthzDecision::AllowWithConfirm` in production code, not just tests.

**Gap C is closed when:**

1. `supervisor.count()` returns >= 1 during an active plan run (visible in
   `roko status` output or TUI dashboard agent count).
2. `supervisor.shutdown_all()` produces at least one `ProcessOutcome` entry
   (currently produces zero because there is nothing registered).
3. The TUI F5 agents tab shows process metrics for at least one agent PID.

**Gap D is closed when:**

1. CLAUDE.md does not claim VCG is wired unless D-2 was chosen and the call
   path is confirmed.
2. If D-1: `vcg_allocate` has a doc comment that accurately describes its
   status as a non-production function.
3. If D-2: `cargo test -p roko-compose prompt_composition` passes and a
   grep confirms `vcg_allocate` is called from `prompt.rs`.

**Gap E is closed when:**

1. `PromptComposer` created in `orchestrate.rs:13980` has at least one
   registered bidder after the first task completes (log at debug level).
2. The bidder posterior for a section that consistently produces passing gates
   is > 1.0 (alpha > beta) after five task observations.
3. A test with artificially high posterior for one section confirms that
   section is selected when the budget is tight enough to force a choice.

---

## Errata applied

Corrections applied 2026-04-22 based on audit discrepancy report:

1. **Gap A scope widened.** Rungs 3 and 4 ALSO stub-pass when their required
   inputs are missing (`source_roots` for rung 3, `generated_test_artifacts`
   for rung 4). The scope description and title updated from "rungs 5-6" to
   "rungs 3-6".

2. **BLOCKER FIX: `--disallow-tools` does not exist.** Claude CLI has no
   `--disallow-tools` flag. The real mechanism is `--allowedTools` (allowlist
   only). Gap B-1 rewritten to compute the effective allowed tool set from the
   `role_tools` whitelist and pass it via `--allowedTools`.

3. **Wrong file path for Claude CLI corrected.** Changed from
   `crates/roko-agent/src/backends/claude_cli.rs` (does not exist) to the actual
   files: `crates/roko-agent/src/claude_cli_agent.rs` and
   `crates/roko-agent/src/provider/claude_cli.rs`.

4. **`current_safety_layer()` location corrected.** Changed from
   `crates/roko-cli/src/agent_spawn.rs:100` to the actual location:
   `crates/roko-agent/src/provider/mod.rs:276`.

5. **SafetyLayer deny list clarified.** SafetyLayer has no "deny list" field.
   Tool restriction is via the `role_tools` whitelist. Tests updated to reflect
   allowlist-based enforcement.

6. **Gap B-2 scope corrected.** `ToolDispatcher::dispatch()` is used for ALL
   non-Claude backends (Ollama, OpenAI-compat, Gemini, Codex, etc.), not just
   Ollama.

7. **`register_external()` documented as new.** The method does NOT exist on
   `ProcessSupervisor`. Added detailed plan for implementation including the
   `TrackedProcess` variant, tracking map changes, and `shutdown_all()` handling.

8. **`update_bidders()` reuse noted.** `PromptComposer::update_bidders()` already
   exists at line 362. Gap E-3 updated to use the existing method instead of
   manual iteration.
