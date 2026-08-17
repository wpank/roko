# Runner 5: `telemetry-learning` — Granular Batch Specification

Date: 2026-04-28

Parent: [FULL-WORK-PLAN.md](./FULL-WORK-PLAN.md) Runner 5 section.

---

## Runner Goal (one sentence)

Make cost, usage, episodes, learning, and cascade router feedback truthful enough that
dashboards show real data and self-improvement actually works.

## Context Pack Files

```text
tmp/runners/telemetry-learning/
  README.md
  batches.toml
  context/
    00-RULES.md                     — universal + runner-specific anti-patterns
    ARCHITECTURE-CONTRACT.md        — single-owner map for this runner
    ANTI-PATTERNS.md                — forbidden patterns with repo examples
    ACCEPTANCE.md                   — proof commands including negative proofs
    FILE-OWNERSHIP.md               — batch → write path map
    ISSUE-MAP.md                    — batch → issue id map
    TELEMETRY-FLOW-AUDIT.md        — current usage/cost/episode data flow (Group 0 output)
    USAGE-CONTRACT.md              — UsageObservation spec (Group 0 output)
```

---

## Anti-Pattern Rules (00-RULES.md)

Include the universal rules from FULL-WORK-PLAN.md plus:

```markdown
# Telemetry-Learning Anti-Patterns

TL-1. **Unknown ≠ zero.** If token count or cost is unavailable, store `None`/`null`.
      NEVER store `0` for unknown usage. Zero means "free and tokenless." Null means "we
      don't know."

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-agent/src/claude_cli_agent.rs` returns `AgentResult` with usage
        containing only `wall_ms`. Token/cost fields default to 0.
      - `.roko/learn/efficiency.jsonl` has 22 entries all showing
        `total_prompt_tokens: 0, total_completion_tokens: 0, cost_usd: 0.0` despite
        real Claude usage.
      - Dashboards show `$0.00` for runs that cost real money.

TL-2. **One cost event per attempt.** An agent attempt produces exactly ONE cost/usage
      observation. Gate failure is a separate event — it does NOT duplicate the attempt cost.

      EXISTING ANTI-PATTERN (do not repeat):
      - `costs.jsonl` logs the same attempt cost once as "success" and again as "gate_failure"
        when the gate fails afterward. This 2x inflates cost tracking.

TL-3. **Model is known before logging.** Never log `model: "unknown-model"` as a string
      that looks like a real model name. If the model is truly unknown, use `None`. If you're
      about to log and model is None, that's a bug in the dispatch flow (model should always
      be resolved before dispatch).

      EXISTING ANTI-PATTERN (do not repeat):
      - Some efficiency events contain `model: "unknown-model"` because the logger runs
        before model resolution completes.

TL-4. **Skipped gates are not passes.** When computing pass rate or feeding learning:
      skip gates (stub, not wired) are excluded from the denominator. They are neither pass
      nor fail — they didn't run.

TL-5. **Learning reads what execution writes.** If you change where events are written,
      update all readers. `learn all`, `/api/learn/efficiency`, and the TUI must read the
      same paths.

      EXISTING ANTI-PATTERN (do not repeat):
      - Execution writes to `.roko/learn/efficiency.jsonl`. `roko learn all` reads from
        a different expected path and says "empty."
```

---

## Group 0: Contract Guardrails

### Z01 — Audit telemetry data flow

**Type:** Context-only (no code changes)

**Goal:** Map where usage/cost/episode data is created, written, read, and displayed.

**Write scope:**
- `tmp/runners/telemetry-learning/context/TELEMETRY-FLOW-AUDIT.md`

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (AgentResult, stream-json parsing)
- `crates/roko-cli/src/orchestrate.rs` (efficiency logger, episode logger, cost logger)
- `crates/roko-learn/src/runtime_feedback.rs` (where feedback is consumed)
- `crates/roko-learn/src/cascade_router.rs` (how observations are fed)
- `crates/roko-cli/src/commands/learn.rs` (learn all reader)
- `crates/roko-serve/src/routes/` (API endpoints for learn/efficiency/cascade)
- `.roko/learn/` (actual files: efficiency.jsonl, costs.jsonl, cascade-router.json)
- `.roko/memory/episodes.jsonl` (episode store)

**Required output:**
- For each data type (usage, cost, episodes, router observations):
  - Where is it generated? (exact file:line)
  - What format is it written in? (JSONL fields)
  - Where is it read? (learn command, API route, TUI)
  - What's wrong? (zero instead of null, duplicated, path mismatch, etc.)
- Identify the Claude CLI stream-json `result` event format (for usage extraction)
- Document all `unknown-model` emission sites

**DO NOT:** Change any source code.

---

### Z02 — Define UsageObservation contract

**Type:** Context-only (no code changes)

**Goal:** Write the precise normalized usage/cost observation spec.

**Write scope:**
- `tmp/runners/telemetry-learning/context/USAGE-CONTRACT.md`

**Read:**
- Z01 output
- FULL-WORK-PLAN.md Runner 5 section
- COMPREHENSIVE-ISSUES 3.1-3.4

**Required output:**
- Exact `UsageObservation` struct with field types
- How it maps to existing fields in efficiency.jsonl (migration path)
- Where it should live (crate + file)
- How consumers handle `None` vs `Some(0)` vs `Some(n)`
- Display formatting rules: `None` → "unknown", `Some(0.0)` → "$0.00", `Some(1.42)` → "$1.42"

**DO NOT:** Change any source code.

---

## Group A: Usage Extraction

### A01 — Parse Claude CLI stream-json result usage

**Goal:** Extract token counts and cost from Claude CLI output.

**Write scope:**
- `crates/roko-agent/src/claude_cli_agent.rs`

**Read:**
- `tmp/runners/telemetry-learning/context/TELEMETRY-FLOW-AUDIT.md`
- Claude CLI documentation (stream-json format)

**Required behavior:**
- Parse the `result` event from Claude CLI stream-json output
- Extract fields (when present):
  - `usage.input_tokens` → `Option<u64>`
  - `usage.output_tokens` → `Option<u64>`
  - `usage.cache_creation_input_tokens` → `Option<u64>`
  - `usage.cache_read_input_tokens` → `Option<u64>`
  - `total_cost_usd` (or `cost_usd`) → `Option<f64>`
  - `model` → `Option<String>` (confirms effective model)
- Store in `AgentResult.usage` fields (extend the struct if needed)
- If fields are absent in the stream: leave as None (NOT zero)

**DO NOT:**
- Guess at stream-json format — read what `ClaudeCliAgent` already parses
- Break existing stream parsing (text deltas, tool events)
- Change how other providers report usage (only Claude CLI in this batch)
- Store absent usage as numeric zero

**Verify:** `cargo check -p roko-agent`

**Evidence:** COMPREHENSIVE-ISSUES 3.1, DEMO-RUN-AUDIT F6

---

### A02 — Add `UsageObservation` type

**Goal:** One normalized type for all usage data.

**Write scope:**
- `crates/roko-agent/src/usage.rs` (NEW or EXISTING file)
- OR `crates/roko-core/src/foundation.rs` (if it belongs in core)

**Required behavior:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageObservation {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub source: UsageSource,
    pub model: Option<String>,
    pub wall_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageSource {
    ProviderReported,   // from Claude CLI result event or API response
    Estimated,          // from token counting heuristic
    Unknown,            // provider didn't report, no estimate available
}
```
- `AgentResult` should include `pub usage: UsageObservation`
- Default `UsageObservation` has all `None` fields and `UsageSource::Unknown`
- If Claude CLI reports values: `UsageSource::ProviderReported`

**DO NOT:**
- Replace the existing `AgentResult` — extend it with a `usage` field
- Break existing serialization (new fields have `#[serde(default)]`)
- Add this type to roko-core unless it's already there (check first!)

**Verify:** `cargo check -p roko-agent`

---

### A03 — Thread usage through efficiency logger

**Goal:** Efficiency events include real usage when available.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (efficiency logger section)

**Read:**
- `tmp/runners/telemetry-learning/context/TELEMETRY-FLOW-AUDIT.md` (where efficiency is logged)

**Required behavior:**
- When writing efficiency events: use `UsageObservation` from `AgentResult`
- If `usage.input_tokens` is Some: write the value
- If `usage.input_tokens` is None: write `null` in JSON (not 0)
- If `usage.cost_usd` is Some: write the value
- If `usage.cost_usd` is None: write `null` (not 0.0)
- Existing fields that aren't in the new type: preserve them with `#[serde(default)]`

**DO NOT:**
- Change the efficiency JSONL path (that creates a reader mismatch)
- Convert None to 0 for serialization compatibility
- Remove the `wall_ms` field (it's always known)

**Verify:** `cargo check -p roko-cli`

---

### A04 — Thread usage through episode logger

**Goal:** Episodes include real usage when available.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (episode logger section)

**Required behavior:**
- Episode metadata includes `UsageObservation` (or its fields directly)
- Same None handling as A03
- Model field in episodes uses the resolved model from `EffectiveModelSelection`, not "unknown-model"

**DO NOT:**
- Change the episode JSONL path
- Remove existing episode fields
- Break episode fingerprinting

**Verify:** `cargo check -p roko-cli`

---

### A05 — Display "unknown" instead of "$0.00" for null cost

**Goal:** UI and CLI consumers show the truth.

**Write scope:**
- `crates/roko-cli/src/commands/status.rs` (status display)
- `crates/roko-cli/src/commands/learn.rs` (learn display)
- `crates/roko-serve/src/routes/` (API routes that include cost)

**Required behavior:**
- `None` cost → display as `"unknown"` or `"-"` (not `$0.00`)
- `Some(0.0)` cost → display as `"$0.00"` (legitimately free)
- `Some(1.42)` cost → display as `"$1.42"`
- In JSON API responses: `null` for unknown, `0.0` for free, `1.42` for real
- The negative-zero fix from Runner 1 (E02) is preserved

**DO NOT:**
- Change stored values — only change display/serialization logic
- Make all `$0.00` displays say "unknown" (only when source is `None`/`Unknown`)
- Touch dashboard rendering (that's a frontend concern)

**Verify:** `cargo check -p roko-cli -p roko-serve`

**Evidence:** COMPREHENSIVE-ISSUES 3.1, 3.3

---

## Group B: Cost Event Semantics

### B01 — Identify all cost event emitters

**Type:** Context-only (no code changes)

**Goal:** Document every place that writes cost/attempt events.

**Write scope:**
- Append to `tmp/runners/telemetry-learning/context/TELEMETRY-FLOW-AUDIT.md`

**Read:**
- `crates/roko-cli/src/orchestrate.rs` (search for cost, attempt, efficiency writes)
- `.roko/learn/costs.jsonl` (examine existing entries)

**Required output:**
- List every call site that writes to costs.jsonl
- For each: what triggers it? (dispatch start? dispatch end? gate pass? gate fail?)
- Identify the duplication: where is the same attempt logged twice?
- Propose the fix: which emission stays, which is removed?

**DO NOT:** Change any source code.

---

### B02 — Deduplicate cost events

**Goal:** One attempt = one cost event, regardless of gate outcome.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (cost event emission)

**Required behavior:**
- Agent dispatch completion emits ONE cost event with:
  - model, provider, usage (from A03), wall_ms, task_id
  - `outcome: "dispatch_complete"` (not "success" or "failure" — those are gate results)
- Gate result is logged as a SEPARATE event type (or field on the attempt):
  - `gate_outcome: "passed" | "failed" | "skipped"`
- No cost duplication for gate failure
- The gate_outcome event does NOT carry cost_usd

**DO NOT:**
- Remove gate outcome logging (just separate it from cost)
- Change the attempt event shape broadly (just stop duplicating it)
- Merge cost and gate into one event (they're semantically different)

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 3.2

---

### B03 — Attach gate outcome to attempt record

**Goal:** You can see dispatch + gate result in one view.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs`

**Required behavior:**
- After gate completes: update the attempt record (or append a linked event) with:
  - `gate_outcome`, `gate_details` (pass/fail/skipped for each gate)
- The linked event uses the same `attempt_id` or `task_id` for correlation
- `learn all` and cost dashboards can join attempt + gate by id

**DO NOT:**
- Store gate results in the cost event itself
- Create a new file for gate outcomes (use existing learning files)
- Change the gate service return type

**Verify:** `cargo check -p roko-cli`

---

### B04 — Regression test: one attempt + failed gate = one cost event

**Goal:** Prove deduplication works.

**Write scope:**
- `crates/roko-cli/tests/` (integration test)
- OR test module in orchestrate.rs

**Required behavior:**
- Simulate: one agent dispatch → one gate failure
- Assert: costs.jsonl (or efficiency.jsonl) contains exactly ONE cost entry for this attempt
- Assert: gate outcome is linked but does NOT duplicate cost
- Assert: total cost summed = the single attempt cost (not 2x)

**DO NOT:**
- Require real agent dispatch (mock the result)
- Require real gates (mock the verdict)
- Write a flaky test

**Verify:** `cargo test -p roko-cli -- cost_dedup` OR equivalent

---

## Group C: Learning and Router Feedback

### C01 — Feed dispatch outcomes into cascade router

**Goal:** Real runs update the cascade router's observations.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (dispatch completion handling)
- `crates/roko-learn/src/cascade_router.rs` (observe method)

**Required behavior:**
- After successful dispatch (process exited cleanly AND gates passed):
  - Call `cascade_router.observe(model_key, task_domain, true)` (positive)
- After failed dispatch (process error OR gates failed):
  - Call `cascade_router.observe(model_key, task_domain, false)` (negative)
- After artifact validation failure (Runner 4):
  - Do NOT call observe (neither positive nor negative)
- `task_domain` derived from task role or category when available
- Cascade router persists after each observation (or on batch/interval)

**DO NOT:**
- Feed observations before model resolution (model must be known)
- Feed observations for skipped/not-wired gates
- Change the cascade router API significantly (just call existing observe method)
- Feed observations from non-execution paths (explain, status, etc.)

**Verify:** `cargo check -p roko-cli -p roko-learn`

**Evidence:** COMPREHENSIVE-ISSUES 4.2

---

### C02 — Prune unavailable model slugs from router display

**Goal:** Router UI shows only models that can actually be used.

**Write scope:**
- `crates/roko-learn/src/cascade_router.rs` (display/query methods)
- OR `crates/roko-serve/src/routes/` (API response filtering)

**Required behavior:**
- When returning cascade router state to UI/API: filter or mark models by availability
- Available = model exists in current config `[models]` table OR has been successfully used
- Unavailable models: still stored internally (observations are historical) but:
  - API response includes `available: bool` per model
  - CLI `learn all` marks unavailable models with `(unavailable)` suffix
- Do NOT delete historical observations

**DO NOT:**
- Remove models from the internal state (history is valuable)
- Require provider health probing (just check config)
- Change the recommend() algorithm (just filter display)

**Verify:** `cargo check -p roko-learn`

**Evidence:** COMPREHENSIVE-ISSUES 1.5

---

### C03 — Align `learn all` with efficiency/episode write paths

**Goal:** `learn all` shows data that execution actually wrote.

**Write scope:**
- `crates/roko-cli/src/commands/learn.rs`

**Required behavior:**
- Read efficiency from the EXACT path that orchestrate.rs writes to
- Read episodes from the EXACT path that the episode logger writes to
- Read cascade router from the EXACT path that persist writes to
- If path exists and has entries: print summary (count, latest timestamp, top model)
- If path exists but empty: print `"0 entries at <path>"`
- If path doesn't exist: print `"No data at <path> (no runs recorded yet)"`
- Print the checked paths always (so user can verify)

**DO NOT:**
- Change where execution writes (only change where `learn all` reads)
- Invent new data formats
- Silently skip missing files

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 4.1, E2E-TEST-RESULTS S4

---

### C04 — Exclude failed artifact validation from positive learning

**Goal:** Only real successes improve the router and learning.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (learning feedback section)

**Required behavior:**
- Before feeding positive learning (efficiency success, router positive obs, knowledge seeds):
  - Check `artifact_valid` from Runner 4's C06 (if applicable)
  - Check gate verdicts (exclude skipped gates from pass count)
- Only feed positive when:
  - Process succeeded
  - AND real gates passed (not just stubs)
  - AND artifact validation passed (for artifact-generation tasks)
- Log when learning is withheld: `"Withholding positive learning: <reason>"`

**DO NOT:**
- Block negative learning (gate failures should still feed the router as failures)
- Require all gates to be non-stub (just exclude stubs from the pass calculation)
- Change what "success" means for non-artifact tasks (code tasks with real gates are fine)

**Verify:** `cargo check -p roko-cli`

---

### C05 — Update dashboard projections for truthful data

**Goal:** API routes return honest data that consumers can render.

**Write scope:**
- `crates/roko-serve/src/routes/` (learn/efficiency/cascade API routes)
- `crates/roko-serve/src/projection_contract.rs` (if projection logic lives here)

**Required behavior:**
- `/api/learn/efficiency` reads from the same paths as `learn all`
- `/api/learn/cascade-router` returns model availability status
- `/api/metrics/c_factor` uses real data (not just static seed values)
- All cost fields in API responses: `null` for unknown, number for known
- API response includes `data_quality` summary:
  ```json
  {
    "efficiency": { "entries": 22, "has_real_usage": true/false },
    "cascade_router": { "total_observations": 15, "from_real_runs": 0 }
  }
  ```

**DO NOT:**
- Change API response structure broadly (add fields, don't remove)
- Make API routes depend on Runner 4's artifact validation being complete
- Add demo data to API responses (that's Runner 7)

**Verify:** `cargo check -p roko-serve`

---

## Group D: Model String Cleanup

### D01 — Eliminate "unknown-model" string

**Goal:** No event or record contains the string `"unknown-model"`.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (where "unknown-model" is emitted)

**Required behavior:**
- Find all sites that emit `"unknown-model"` as a model string
- Replace with: use the resolved model from `EffectiveModelSelection` (Runner 2)
- If model is genuinely not resolved at logging time: use `None` (serialize as `null`)
- Never use a string that looks like a model name when the value is actually unknown

**DO NOT:**
- Change the model selection logic (Runner 2 does that)
- Add a new model resolution path — just use what Runner 2 provides
- Replace with another placeholder string

**Verify:** `grep -rn 'unknown-model' crates/ --include='*.rs' | grep -v target` → empty

**Evidence:** COMPREHENSIVE-ISSUES 3.4

---

## Group E: Proof

### E01 — Usage extraction unit test

**Write scope:**
- `crates/roko-agent/src/claude_cli_agent.rs` (test module)

**Required behavior (tests):**
- Stream-json with result event containing usage → `UsageObservation` has real values
- Stream-json with result event WITHOUT usage → `UsageObservation` has None values
- Stream-json with no result event (interrupted) → `UsageObservation::Unknown`
- Verify: no zero values for absent data

---

### E02 — Cost deduplication integration test

(Same as B04 — already specified there)

---

### E03 — Learning path alignment test

**Write scope:**
- `crates/roko-cli/tests/learn_paths.rs`

**Required behavior (tests):**
- Write a fake efficiency event to the correct path
- Write a fake episode to the correct path
- Call `learn all` underlying function
- Assert it finds and reports both entries
- Assert paths printed match paths written

---

### E04 — Full telemetry proof script

**Write scope:**
- `tmp/runners/telemetry-learning/context/PROOF-TELEMETRY.md`

**Required proof steps:**
1. Run a real agent dispatch (via `roko run` or `plan run`)
2. Check `.roko/learn/efficiency.jsonl`: cost_usd is NOT 0.0 (or is `null`)
3. Check `.roko/memory/episodes.jsonl`: model is NOT "unknown-model"
4. Check costs: exactly ONE cost entry per agent attempt
5. Run `roko learn all`: shows non-empty data with real values
6. Check cascade-router.json: `total_observations` increased by 1
7. If gate was stub/skipped: it's NOT counted in pass rate
8. API `/api/learn/efficiency`: returns matching data

---

## Batch Summary

| Group | Batches | Main scope |
|---|---:|---|
| 0: Contracts | 2 | telemetry flow audit, usage contract |
| A: Usage Extraction | 5 | stream-json parse, UsageObservation type, thread through loggers |
| B: Cost Semantics | 4 | deduplication, gate separation, test |
| C: Learning/Router | 5 | feed router, prune models, align paths, exclude bad artifacts, dashboard |
| D: Model Cleanup | 1 | eliminate unknown-model string |
| E: Proof | 4 | unit tests, integration tests, proof script |
| **Total** | **21** | |

## Suggested Execution Waves

Wave 1: Z01, Z02 (context-only, parallel)
Wave 2: A01, A02 (parse usage, define type — parallel, different files)
Wave 3: A03, A04 (thread through efficiency + episode loggers)
Wave 4: A05, D01 (display unknown + eliminate unknown-model — parallel)
Wave 5: B01 (audit cost emitters — context)
Wave 6: B02, B03 (dedup + gate attachment)
Wave 7: B04 (cost dedup test)
Wave 8: C01, C02, C03 (feed router, prune, align paths — can be parallel)
Wave 9: C04, C05 (exclude bad artifacts, dashboard projections)
Wave 10: E01, E03, E04 (tests + proof)

## Acceptance Criteria

This runner is done when:

**Positive proofs:**
- Real agent dispatch → efficiency.jsonl has non-null cost_usd
- `roko learn all` shows data after a run (paths aligned)
- Cascade router `total_observations` increases after real dispatch
- API `/api/learn/efficiency` returns matching data
- Unknown cost displays as "unknown" or "-" (not "$0.00")
- Known cost displays correctly (e.g., "$1.42")
- Gate pass rate excludes skipped gates

**Negative proofs:**
- Absent usage stored as `null`, never as `0` (verify JSONL)
- `grep "unknown-model" .roko/` → no results
- One agent attempt + one gate failure → exactly ONE cost entry in costs.jsonl
- Failed artifact → NO positive cascade router observation
- Stub gates → NOT counted as pass in any rate calculation
- `roko learn all` with no data → says "No data at <path>" (not "empty")
