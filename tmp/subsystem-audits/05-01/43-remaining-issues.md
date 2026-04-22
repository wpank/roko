# 43 — Remaining Issues Tracker

Generated: 2026-05-01 morning. Continuation of `41-consolidated-backlog.md`
covering everything not yet committed to `wp-arch2`. **All items are
self-contained**: read your assigned section once and execute.

**Status key**: `[ ]` = open, `[~]` = in progress, `[x]` = done. Each
runner MUST tick its own checkboxes (in this file) when it commits an
item, then include the updated tracker in its branch as part of the
final commit (or a dedicated `chore: tick tracker` commit, your call —
but always update before reporting back).

**Branch convention**: each runner works on a worktree branched off
`wp-arch2`. Commit each item independently. Use the message prefixes
shown per item.

**Pre-commit (mandatory before every commit)**:
```bash
cargo +nightly fmt --all       # if it errors on orchestrate.rs (pre-existing
                               # rustfmt parser issue), use:
                               # rustup run nightly rustfmt --edition 2024 \
                               #   --check <files-you-edited>
cargo clippy -p <crate> --no-deps -- -D warnings
cargo test -p <crate> --lib
```

**Hard rules** (apply to every item below):
1. One backlog item per commit. No "while we're here" cleanup.
2. No drive-by formatting, no `clippy --fix`, no auto-rewrites.
3. No new dispatch paths. Always reuse `ModelCallService` / `DispatchResolver`.
4. Skeletons ≠ migrations. A type existing does not mean the product path is migrated.
5. Unknown ≠ zero. Missing usage/cost/context must stay `None`, never become `0`.
6. Missing config → restricted. Never grant permissions on load failure.
7. No string-interpolated payloads. Use `serde` / `toml` serializers.
8. No `unwrap()` / `panic!()` in changed code. Return typed errors.
9. If an item is blocked or would expand scope, mark `[~]` with the reason
   in this file and STOP that item. Do not commit a partial fix. Continue
   with the next item.
10. After committing an item, update the corresponding `[ ]` to `[x]` in this
    file and add a short result note + commit hash on the same line.

---

## Snapshot of `wp-arch2` at tracker creation

- HEAD: `8d1e5da7 fix(roko-cli): post-T4 merge fixups in chat_inline`
- 44 commits ahead of `origin/wp-arch2`.
- `cargo check --workspace` passes clean.
- Pre-existing test failures (workspace-wide):
  - `roko-cli`: `chat_session::tests::slash_model_sets_new`,
    `model_selection::tests::cascade_router_is_consulted_when_no_explicit_selection_exists`,
    `chat_inline::tests::apply_model_switch_session_failure_leaves_selection_untouched` (currently `#[ignore]`),
    two `prd::plan_validate::tests::validate_*greenfield*`,
    `run::tests::test_v2_share_produces_real_transcript`.
  - `roko-core`: `config::schema::tests::effective_models_backwards_compat`,
    `tool::trace::tests::scrubbed_redacts_model_field`,
    `tool::trace::tests::scrubbed_redacts_custom_event_payload`.
  - `roko-serve`: 9 failures — `routes::middleware::tests::local_origin_accepts_ipv6_loopback`,
    `routes::status::tests::{gate_history_returns_500_for_invalid_jsonl, gate_summary_includes_rung_breakdown_under_api_grouping, gates_history_collection_is_mounted_under_api_grouping}`,
    `routes::learning::tests::{cfactor_trend_returns_empty_array_when_missing, cfactor_trend_returns_hourly_buckets_for_default_window, learn_alias_routes_expose_cascade_router_cost_tiers_and_gate_thresholds}`,
    `tests::{build_app_state_loads_persisted_learning_state_and_falls_back_cleanly, shutdown_persists_cascade_router_state}`.
- 5 unstaged WIP files in main worktree (NOT touched, leave alone unless
  asked): `Dockerfile`, `crates/roko-acp/src/{bridge_events.rs,config.rs,types.rs}`,
  `crates/roko-core/src/config/mod.rs`.

---

# Group A — Tier 5 large architectural extractions (sequential, high-risk)

## [~] T5-35: Extract `dispatch_agent_with` into composable units

- [x] T5-35a: `select_dispatch_model` extracted (R1 branch `wp-arch2-t5-35-extract`).

**File**: `crates/roko-cli/src/orchestrate.rs`. The function `dispatch_agent_with`
starts at line **14575** (signature: `async fn dispatch_agent_with(...)`). It is
~2,000 lines long.

NOTE: `orchestrate.rs` is gated behind `#[cfg(feature = "legacy-orchestrate")]`.
The file currently has a pre-existing compile error (`AgentConfig::default_model`
missing) when that feature is on. Run `cargo check -p roko-cli --features
legacy-orchestrate --lib` BEFORE editing to capture the baseline error
count. Your edits must NOT introduce new errors. Default-feature builds
the file as dead code only.

**Change** (4 commits — strict order, **pure mechanical move**, no logic
changes):

1. **T5-35a**: Extract `select_model()` — corresponds to "steps 4-6"
   in `dispatch_agent_with` (~335 lines). Pull out the model selection /
   cascade-router / fallback resolution block into a new free function
   (or inherent method on `PlanRunner`) and call it from `dispatch_agent_with`.
   Name it `select_dispatch_model`. Keep all parameters explicit; do not
   group them into a new struct yet.
2. **T5-35b**: Extract `build_prompt()` — steps 14-19 (~350 lines). Pull
   the system + user prompt assembly section into `build_dispatch_prompt`.
3. **T5-35c**: Extract `launch_agent()` — step 24 (~330 lines). Pull the
   per-provider launch / dispatch arm into `launch_dispatched_agent`.
4. **T5-35d**: Extract `record_outcome()` — steps 25-31 (~295 lines). Pull
   the cost / usage / episode / feedback recording tail into
   `record_dispatch_outcome`.

**Rule**: Each extraction MUST be a pure move. No renaming variables, no
"improving" intermediate types, no "cleanup". After each commit:

```bash
cargo check -p roko-cli --lib
cargo check -p roko-cli --features legacy-orchestrate --lib
# Baseline error count must be unchanged.
```

If extracting a step requires touching shared mutable state in a way that
won't compile (e.g. lifetime conflict with `&mut self`), STOP that
sub-item, mark `[~]` with the reason, continue with the next sub-item.

**Commit prefix**: `T5-35a:`, `T5-35b:`, `T5-35c:`, `T5-35d:`.

**Do not**: Change behavior. Do not collapse into "context structs". Do
not delete intermediate variables. Do not move tests.

---

## [x] T5-36: Migrate remaining serve dispatch to `ModelCallService` — no LLM dispatch in routes/ — already migrated (R2 audit-only commit `88bfb26f`)

**Files**: `crates/roko-serve/src/routes/`. The `roko-serve` crate still
constructs `reqwest::Client` directly in some routes for LLM calls.
Audit:

```bash
rg -l 'reqwest::Client' crates/roko-serve/src/routes/
# Currently shows: routes/secrets.rs, routes/deployments.rs (plus non-route
# infrastructure files outside routes/ — those are out of scope).
```

The `secrets.rs` and `deployments.rs` `reqwest::Client` instances are NOT
LLM dispatch — they call out to Railway / vault APIs. Verify by reading
each, and if they aren't LLM calls, leave them alone and mark this item
`[x] no LLM dispatch in routes/ — already migrated`.

If you find route handlers that DO construct LLM HTTP requests (search
for `api.openai.com`, `api.anthropic.com`, `/v1/chat`, `/messages`,
`/messages/stream` inside `crates/roko-serve/src/routes/`), migrate them
**one endpoint per commit** to `ModelCallService::call` /
`ModelCallService::stream` (constructed via `with_config` / `with_cost_table`
fluent builders — see `crates/roko-agent/src/model_call_service.rs:107`).

**Verify per endpoint**:
- Same response body/status (compare via existing route tests).
- `rg reqwest::Client crates/roko-serve/src/routes/` count strictly decreases.

**Commit prefix**: `T5-36-<endpoint>:`.

**Do not**: Migrate non-LLM HTTP calls (Railway, vault, GitHub). Do not
add new dependencies. Do not change response shapes.

---

## [ ] T5-38: Collapse config into validated model

**File**: `crates/roko-core/src/config/`.

**Background**: `load_config()` currently returns `Result<RokoConfig,
LoadConfigError>` after T1-12 added strict-validation. Provenance metadata
is computed alongside but not bundled with the config. Goal: return a
`ValidatedConfig` wrapper that carries the parsed `RokoConfig`, the
provenance trace, and any soft-warning diagnostics.

**Pre-check**:
```bash
rg -n 'pub struct ValidatedConfig|pub struct ConfigProvenance' crates/roko-core/src/config/
# If ValidatedConfig already exists (in provenance.rs from your WIP),
# this item is partially started — extend rather than rewrite.
```

**What to change**:
1. Update `pub fn load_config(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError>`
   to return the wrapper. Keep an inherent `RokoConfig::default()` /
   `ValidatedConfig::into_config()` escape hatch for callers that don't
   need provenance.
2. Run all semantic checks (the strict validator from T1-12 plus any
   `validation.rs` rules) inside `load_config` and surface them via
   `ValidatedConfig::diagnostics()`.
3. Update direct callers of `load_config` (search with
   `rg 'load_config\(' crates/`) to either accept `ValidatedConfig` or
   call `.into_config()`. Where a call site only needs the inner
   `RokoConfig`, prefer `.into_config()` to keep blast radius small.

**Verify**:
- `cargo check --workspace` clean.
- `cargo test -p roko-core --lib config` passes (extend the existing
  `load_config_tests` mod — do NOT replace).
- `rg 'load_config\(' crates/` shows every call site adapted.

**Commit prefix**: `T5-38:` (or `T5-38a/b/c` if you split the call-site
adaptation per crate to keep diffs small).

**Do not**: Break `RokoConfig::default()`. Do not require provenance for
callers that don't care. Do not rewrite the validator itself.

---

## [ ] T5-40: Replace event-replay reports with RunLedger

**Files**: `crates/roko-runtime/src/`, `crates/roko-cli/src/orchestrate.rs`.

**Background**: `crates/roko-runtime/src/run_ledger.rs` exists and provides
a typed durable record of run events. Several call sites in `orchestrate.rs`
still derive their reports by replaying JSONL event files — slow and
fragile. Goal: each replay source migrates to `RunLedger`.

NOTE: `orchestrate.rs` is `legacy-orchestrate`-gated and has a pre-existing
compile error in that feature. Same baseline-error rule as T5-35.

**Change** (one source per commit, in this order):

1. **T5-40a**: gates → ledger. Replace gate-verdict replay with
   `RunLedger::append_gate(...)` writes + `RunLedger::iter_gates()` reads.
2. **T5-40b**: artifacts → ledger. Migrate artifact-list replay similarly.
3. **T5-40c**: events → ledger. Migrate generic event replay.
4. **T5-40d**: resume → ledger. Migrate resume-state reconstruction to
   read from the ledger.

**Verify per commit**:
- `cargo check -p roko-runtime --lib` clean.
- `cargo check -p roko-cli --lib` clean (default features).
- `cargo check -p roko-cli --features legacy-orchestrate --lib` — error
  count unchanged from baseline.
- Add or extend a unit test in `roko-runtime` that exercises the ledger
  path for the migrated source.

**Commit prefix**: `T5-40a:`, `T5-40b:`, `T5-40c:`, `T5-40d:`.

**Do not**: Delete the JSONL files yet (downstream tooling may still read
them). Do not change `RunLedger`'s schema. Do not collapse the four
sources into one commit.

---

## [~] T5-42: Provider-native structured history for all adapters — blocked: typed `Message` representation does not exist (preflight grep `pub enum Message|pub struct Message` in `crates/roko-agent/`, `crates/roko-core/`, `crates/roko-primitives/` returns only `MessageRole` + `MessageContent`; canonical `ChatMessage` exists in `roko-core::chat_types` but the user packet explicitly forbids inventing or substituting a `Message` enum). All six sub-items (T5-42a Anthropic, T5-42b Ollama, T5-42c Gemini, T5-42d OpenAI/compat, T5-42e Cursor/ACP, T5-42f Claude CLI) are blocked on that prerequisite packet.

**Files**: `crates/roko-agent/src/` (one adapter per commit).

**Background**: Most provider adapters today accept a flat string history
and re-parse it. They should accept `Vec<Message>` and convert to the
provider's native message format on serialization.

**Change** (one adapter per commit, in this order):

1. **T5-42a**: Anthropic — `crates/roko-agent/src/translate/anthropic.rs`
   (or wherever the message builder lives — `rg -l 'anthropic' crates/roko-agent/src/`).
2. **T5-42b**: Ollama.
3. **T5-42c**: Gemini.
4. **T5-42d**: OpenAI / OpenAI-compat (in `crates/roko-agent/src/translate/openai.rs`).
5. **T5-42e**: Cursor / ACP (`crates/roko-agent/src/cursor_agent.rs`).
6. **T5-42f**: Claude CLI (`crates/roko-agent/src/claude_agent.rs` /
   `provider/claude_cli.rs`).

**Per adapter**:
- Accept a `Vec<Message>` parameter where today's API takes `&str` or
  `Vec<String>`.
- Convert internally to the provider-native representation (Anthropic's
  `messages` array, Gemini's `contents`, etc.).
- Keep a thin shim that accepts the old flat format and converts via
  `Message::from_legacy_string` so callers can migrate incrementally.

**Verify per adapter**:
- Tests distinguish multi-turn vs single-turn correctly.
- `cargo test -p roko-agent --lib <adapter_name>` passes.

**Commit prefix**: `T5-42a:` through `T5-42f:`.

**Do not**: Change the public `AgentRuntime` trait surface in this packet.
Do not migrate consumers to the new typed API yet (that's a future packet).
Do not invent a `Message` enum if one doesn't exist — search with
`rg 'pub enum Message|pub struct Message' crates/roko-agent/`. If none
exists, mark `[~]` and STOP — typed message representation is itself a
prerequisite item.

---

# Group B — Smaller follow-ups from the prior session

## [ ] T2-17 follow-up: Remove the 6 internally-consumed learn modules

**Background**: Agent A's first pass blocked these because each is
referenced by another module that was kept. The internal consumers
themselves have **zero external callers** (verified at the time), so a
cascading removal is feasible.

| Module | Internal consumer to remove first |
| --- | --- |
| `calibration_policy` | `event_subscriber` | done (`e43dc272`, R3) |
| `regression` | `runtime_feedback` | done (`eac3407d`, R3; trimmed, runtime_feedback kept) |
| `local_reward` | `runtime_feedback` |
| `section_outcome` | `contextual_bandit` |
| `post_gate_reflection` | `playbook_rules` + `runtime_feedback` |
| `verdict_scorer` | `event_subscriber` | done (`ea1fc724`, R3) |

**Procedure (one module per commit)**:
1. Pre-check that the named consumer is itself not referenced from
   outside `roko-learn`:
   ```bash
   rg 'roko_learn::<consumer>' crates/ -g '*.rs' --glob '!crates/roko-learn/'
   ```
   If non-zero, mark `[~]` and STOP for that pair.
2. Delete the consumer module's references to the orphan, OR delete the
   consumer entirely if removing the references gives an empty module.
3. Delete the orphan module.
4. Run `cargo test -p roko-learn --lib`.

**Commit prefix**: `T2-17b-<module>:`.

**Do not**: Delete a consumer that has external callers. Do not "fix"
or "refactor" the consumer to avoid using the orphan — just remove it.

---

## [x] T3-23 follow-up: Per-endpoint rate limit overrides — R2 commit `ea763106` on branch `r2/t5-36-t3-23b-serve`

**File**: `crates/roko-serve/src/routes/mod.rs`.

**Background**: T3-23 landed only the global 100 req/sec layer. The three
per-endpoint overrides need their own middleware.

**Endpoints + caps**:
- `POST /api/terminal/sessions` → 5/min
- `POST /api/inference/complete` → 30/min
- `POST /api/agents/register` → 10/min

**Approach**: extend the existing `governor` setup (added in commit
`18da6890 T3-23`) with per-endpoint sub-routers. Use `axum::middleware::from_fn_with_state`
and a per-endpoint `governor::Quota`. Three sub-routers merged into the
main router will likely be the smallest diff.

**Verify**:
- A test per endpoint that exceeds the cap and asserts 429.
- Global 100/sec test from T3-23 still passes.

**Commit prefix**: `T3-23b:` (one commit covering all three overrides is
fine since they share the same wiring pattern).

---

## [x] T4-31d: Migrate shared OpenAI-compat parser to `UsageObservation` (c807088b — perplexity chat seam migrated; shared parser already thin)

**File**: `crates/roko-agent/src/translate/openai.rs` (around line 297
where `parse_usage` lives — `parse_usage_observation` already exists at
line 267 from the T4-31 series).

**Background**: T4-31a/b/c/e migrated provider-specific parsers. Cerebras
+ GLM + generic OpenAI all delegate to this shared parser. The migration
is the same pattern: have callers consume `UsageObservation` (with
`Option<u64>` fields) and convert to `Usage` only when a legacy seam
demands it.

**Change**:
1. Make `parse_usage` thin: build a `UsageObservation` and call
   `Usage::from(observation)` for legacy callers.
2. Update each call site that wants the unknown / zero distinction (search
   with `rg 'parse_usage(' crates/roko-agent/`) to consume
   `parse_usage_observation` directly.
3. Add or extend tests that distinguish absent-`usage` (None) vs
   explicit-zero-tokens (Some(0)) in the OpenAI-compat wire format.

**Verify**:
- `cargo test -p roko-agent --lib openai` passes.
- `cargo test -p roko-agent --lib cerebras` passes (the test name may
  differ; search what exists).

**Commit prefix**: `T4-31d:` (single commit covering the shared parser;
this migrates Cerebras + GLM + generic OpenAI together because they share
the parser).

---

## [ ] T4-32 audit doc tick

**File**: `tmp/subsystem-audits/05-01/41-consolidated-backlog.md`.

**Change**: T4-32 was already wired by `511cdb45 ux-followup(UX24)`. Flip
the `[ ]` next to `T4-32:` to `[x] (already wired by 511cdb45)`. Also
update the progress summary at the bottom to reflect T4 = 6 / 6 done.

**Commit prefix**: `chore(audit): tick T4-32`.

---

## [x] T5-37 / T5-39: already merged this session

Already in `wp-arch2`. Listed here only for completeness.

---

## [ ] T5-41: Migrate demo automation off prompt scraping

**File**: `demo/demo-app/src/lib/scenario-runners/`.

**Now unblocked**: `crates/roko-serve/src/command_events.rs` was committed
as part of `098778d6 build: restore missing roko-serve infrastructure for
T3 work` and `terminal.rs` already imports from it. The serve route
exposes typed `CommandEvent`s.

**Change**:
1. Audit current regex usage:
   ```bash
   rg -n 'test result: ok|passing|/test|/passing' demo/demo-app/src/lib/scenario-runners/
   # The known hits are in prd-pipeline.ts:337
   ```
2. Replace each regex hit with a `CommandEvent` lifecycle subscription.
   Determine success by `exit_code === 0`, not regex match.
3. `knowledge-transfer.ts` already relies on `showCmd` / exit-code
   semantics — confirm and leave alone.

**Verify** (use **`yarn`** not `npm`):
```bash
cd demo/demo-app
yarn lint    # or whatever lint script exists
yarn tsc --noEmit
```

**Commit prefix**: `T5-41:` (one commit covering both runners is fine).

If `CommandEvent` types aren't exported to TS yet (search the demo's
`shared/` or `types/` directory), STOP and mark `[~]` — the typed-export
plumbing is its own packet.

---

# Group C — Pre-existing test failures (diagnostic + small fixes only)

These were already failing on `wp-arch2` BEFORE this session. Each
warrants a short investigation and either a focused fix or a
`#[ignore]` + filed-as-followup commit. **Do not** invest more than ~1
hour per failure — if the root cause turns out to need a real refactor,
write up findings and STOP.

## [ ] PreFail-1: `roko-core::config::schema::tests::effective_models_backwards_compat`

Look at the test, compare against current `RokoConfig` shape, decide
whether the assertion is now stale (because earlier commits changed
defaults) or whether there's a real regression. Either fix the test or
file as `tracker:43 PreFail-1` issue with findings.

## [ ] PreFail-2: `roko-core::tool::trace::tests::scrubbed_redacts_model_field`

Likely missing field in the redaction allowlist. Check what new field
was added since the test was written.

## [ ] PreFail-3: `roko-core::tool::trace::tests::scrubbed_redacts_custom_event_payload`

Similar to PreFail-2 — likely a redaction policy gap.

## [ ] PreFail-4: `roko-cli::chat_session::tests::slash_model_sets_new`

The test panics with a `ClaudeCliAgent { … }` debug print. Likely a
fixture mismatch or a now-strict assertion. Inspect the panic at
`crates/roko-cli/src/chat_session.rs:2246` and either fix the assertion
or the fixture.

## [ ] PreFail-5: `roko-cli::model_selection::tests::cascade_router_is_consulted_when_no_explicit_selection_exists`

Resolver returns `Ok(UnknownModel { … })` when it should return `Err`.
Same root cause as the currently-`#[ignore]`d `apply_model_switch_session_failure_leaves_selection_untouched`.
Decide: tighten the resolver to reject unknown slugs (small change), or
loosen the test (and document the new behavior).

If you tighten the resolver, also un-`#[ignore]` the
`apply_model_switch_session_failure_leaves_selection_untouched` test in
`crates/roko-cli/src/chat_inline.rs` and verify it now passes.

## [ ] PreFail-6: `roko-cli::prd::plan_validate::tests::validate_no_greenfield_duplicates_flags_existing_crate_and_phrases`

Off-by-one in the assertion (`left: 2, right: 1`). Likely the validator
flags one more thing now than the test expected.

## [ ] PreFail-7: `roko-cli::prd::plan_validate::tests::validate_plans_dir_with_workdir_rejects_greenfield_duplicates`

Off-by-one (`left: 4, right: 3`). Similar fix to PreFail-6.

## [ ] PreFail-8: `roko-cli::run::tests::test_v2_share_produces_real_transcript`

Asserts `gate_results == [("compile", true)]`, but actually got `[]`.
Either the test setup needs to wait for gate completion, or share
serialization changed. Inspect `crates/roko-cli/src/run.rs:3266`.

## [ ] PreFail-9..17: `roko-serve` 9 failures

The 9 failures listed in the snapshot above. Two of them
(`build_app_state_loads_persisted_learning_state_and_falls_back_cleanly`,
`shutdown_persists_cascade_router_state`) panic with "can call blocking
only when running on the multi-threaded runtime" — a `#[tokio::test]`
flavor bug that's a one-line fix (`flavor = "multi_thread"`). The others
need short investigation each.

If a roko-serve test failure is genuinely unrelated to feature changes
(e.g. test infrastructure rot), prefer fixing it. If it points at a
real regression, document and STOP.

**Commit prefix**: `fix(<crate>): pre-existing test PreFail-N`. One
commit per failure (or per cluster if they share root cause).

---

# Group D — Lingering working-tree state (informational only)

5 unstaged files in the main `wp-arch2` worktree at session start:
- `Dockerfile`
- `crates/roko-acp/src/{bridge_events.rs, config.rs, types.rs}`
- `crates/roko-core/src/config/mod.rs`

Runners MUST NOT touch these in their own worktrees (their worktree is
branched from `HEAD`, so these edits won't be present anyway). Listed
here so the user knows they're outstanding; not assigned to any runner.

---

# Sub-agent assignments

| Runner | Group / Items | Branch suffix | Approx commits |
| --- | --- | --- | --- |
| **R1** | T5-35 (a/b/c/d) | `t5-35-extract` | 4 |
| **R2** | T5-36 + T3-23 follow-up | `t5-36-t3-23b-serve` | 1-4 |
| **R3** | T5-38 + T2-17 follow-up | `t5-38-t2-17b-config-learn` | 2-7 |
| **R4** | T5-40 (a/b/c/d) + T4-32 audit tick | `t5-40-runledger` | 4-5 |
| **R5** | T5-42 (up to 6 adapters) + T4-31d shared parser | `t5-42-t4-31d-agent` | 2-7 |
| **R6** | T5-41 demo + Group C pre-existing test failures | `t5-41-prefails` | 2-10 |

Each runner reads only its row's items in this file plus the global rules
at the top.
