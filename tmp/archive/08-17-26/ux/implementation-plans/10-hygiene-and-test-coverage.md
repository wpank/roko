# 10 — Hygiene and Test Coverage

> **Source plans**: `tmp/ux/ux-followup/03-non-batch-followups.md` item 19,
> `tmp/ux/ux-followup/09-hygiene-and-test-coverage.md` items 56, 58,
> `tmp/ux/ux-followup/14-observability-gaps.md` item 87, plus the smoke-test
> qualifier deferred from `09-stale-docs-and-drift.md` item 45.
>
> **Status as of 2026-05-01**:
> - Item 19 (SystemPromptBuilder snapshot tests): the crate compiles, but
>   no per-role golden snapshot exists. Reviewers can't tell when prompt
>   composition regresses silently.
> - Item 56 (clippy `missing_errors_doc` / `missing_panics_doc`): several
>   crates still carry crate-level `#[allow(...)]` masking docs debt.
> - Item 58 (flaky tests): timeout-based assertions like
>   `with_timeout_ms(100)` in `crates/roko-agent/src/exec.rs:504`.
> - Item 87 (per-gate timeline): verdicts substrate is now read by
>   `verdicts.rs` (item 83 closed), but the dashboard Gate tab shows a
>   single rolling EMA per rung, not per-gate sparklines.
> - Item 45 carry-over: CLAUDE.md "What to work on" items 1-9 lack
>   integration smoke tests.
>
> **Effort**: 4-6 days.
>
> **Risk**: Low. Tests-only and docstrings-only; no runtime code paths
> change.

---

## What this plan accomplishes

Close the four hygiene items + the deferred CLAUDE.md smoke tests.
After this plan:

- Each agent role has a captured system-prompt snapshot under
  `crates/roko-compose/tests/snapshots/<role>.snap`.
- The crate-level `#[allow(clippy::missing_errors_doc)]` /
  `missing_panics_doc` lists are gone; missing docs are filled.
- Flaky timeout-based tests use mocked time (or scaled timeouts under
  `CI=true`).
- The Gate tab in the TUI shows per-gate (compile / test / clippy /
  symbol / generated_test / fact_check / property_test / verify_chain /
  llm_judge / integration) rolling pass-rate sparklines.
- CLAUDE.md "What to work on" items 1-9 each have a smoke test under
  `crates/roko-cli/tests/smoke_<concern>.rs`.

## Why this matters

Snapshot tests catch the silent prompt regressions that destroy
agent behaviour without any error. Per-gate timelines unblock the
"why is rung-1 EMA dragging?" question. Smoke tests on items 1-9 turn
CLAUDE.md status from "we believe X works" into "CI proves X works".

---

## Required reading

```
crates/roko-compose/src/system_prompt_builder.rs
crates/roko-compose/src/                          (related modules)
crates/roko-compose/tests/                        (existing tests, if any)
crates/roko-agent/src/exec.rs                     (line 504; flaky timeout)
crates/roko-cli/src/tui/dashboard.rs              (gate tab plumbing)
crates/roko-cli/src/tui/views/dashboard_view.rs   (where verdict trends render)
crates/roko-cli/src/tui/verdicts.rs               (the substrate reader)
crates/roko-learn/src/aggregate.rs                (gate trend bucketing)
.cargo/config.toml or `Cargo.toml` workspace lints
CLAUDE.md "What to work on"
crates/roko-cli/tests/                            (existing smoke tests)
tmp/ux/ux-followup/{03,09,14}-*.md                (item context)
```

---

## Deliverables

### Item 19 — SystemPromptBuilder snapshot tests

1. Take the canonical role list from `roko.toml` schema or
   `crates/roko-core/src/config/schema.rs::RoleOverride`. Likely:
   `implementer`, `reviewer`, `planner`, `researcher`, `tester`,
   `documenter` (verify against the canonical list before writing
   tests).

2. For each role:

   ```rust
   #[test]
   fn role_implementer_prompt_snapshot() {
       let cfg = test_config_for_role("implementer");
       let prompt = SystemPromptBuilder::new(&cfg)
           .with_skills(default_skills())
           .with_tools(default_tools())
           .render();
       insta::assert_snapshot!(prompt);
   }
   ```

3. Snapshots live under `crates/roko-compose/tests/snapshots/`.
   Reviewer runs `cargo insta review` to bless changes.

4. CI gate: `cargo insta test --check` (or `cargo insta accept --no-touch`
   plus a diff guard).

5. The 6-layer assertion in item 19 says we should verify all 6 layers
   compose. Add a single test:

   ```rust
   #[test]
   fn six_layers_present_in_every_role() {
       for role in ALL_ROLES {
           let p = build_for(role);
           assert!(p.contains("# Role"),     "role layer missing for {role}");
           assert!(p.contains("# Skills"),   "skills layer missing for {role}");
           assert!(p.contains("# Tools"),    "tools layer missing for {role}");
           assert!(p.contains("# Context"),  "context layer missing for {role}");
           assert!(p.contains("# Memory"),   "memory layer missing for {role}");
           assert!(p.contains("# Policy"),   "policy layer missing for {role}");
       }
   }
   ```

   (Layer names match whatever `system_prompt_builder.rs` emits;
   verify before locking in.)

### Item 56 — Fill missing `# Errors` / `# Panics`

1. Grep the workspace for the offenders:

   ```bash
   rg -t rust '#\[allow\(clippy::missing_(errors|panics)_doc\)\]'
   ```

2. For each occurrence:
   - Locate the public functions in scope.
   - Add a `# Errors` section to functions returning `Result`.
   - Add a `# Panics` section to functions that can panic (e.g. via
     `unwrap`, `expect`, indexing, `assert!`).
   - Remove the `#[allow(...)]`.

3. Some files have the `#[allow]` because adding 100 doc sections is a
   week of work. **Not all of them must close in this plan**; pick the
   N highest-leverage public crates first (`roko-core`, `roko-runtime`,
   `roko-gate`, `roko-compose`). Leave the rest as a tracked TODO with
   a follow-on item.

### Item 58 — Flaky timeout-based tests

1. Audit:

   ```bash
   rg -t rust 'with_timeout_ms\(\s*[0-9]+' crates/ apps/
   ```

2. For each hit, two strategies:

   **Strategy A** (preferred, deterministic): `tokio::time::pause()`
   and manually advance time:

   ```rust
   #[tokio::test(start_paused = true)]
   async fn fast_timeout() {
       let job = my_thing_with_timeout(Duration::from_millis(100));
       tokio::time::advance(Duration::from_millis(101)).await;
       assert!(job.timed_out());
   }
   ```

   **Strategy B** (fallback): scale timeouts under CI:

   ```rust
   fn timeout() -> Duration {
       if std::env::var("CI").is_ok() {
           Duration::from_secs(2)  // 20× normal
       } else {
           Duration::from_millis(100)
       }
   }
   ```

   Document the choice per test.

3. Add a `tests-flaky` allowlist file
   (`crates/<crate>/tests/.flaky-allow.txt`) for tests that
   intentionally race; reviewers must justify additions.

### Item 87 — Per-gate timeline

The verdicts aggregator (`crates/roko-cli/src/tui/verdicts.rs`) already
produces `GateStats` with rolling 24×1h pass/fail buckets per gate
(see `tmp/ux/ux-followup/14-observability-gaps.md` item 83). Item 87 is
to *render* the per-gate breakdown, not just per-rung.

1. In `verdicts.rs`, ensure `GateStats` is keyed by `gate_name` (the
   string the gate emits, e.g. `"clippy"`, `"compile"`,
   `"property_test"`, `"llm_judge"`). Confirm; if it's keyed by rung,
   refactor to per-gate.

2. In `views/dashboard_view.rs`, find `render_gate_trend_grid` (line
   ~1564 per the audit). Render one row per gate, each with a
   24-bucket sparkline. Use `ratatui::widgets::Sparkline` if available
   in the version of ratatui in use; otherwise hand-render with block
   characters (`▁▂▃▄▅▆▇█`).

3. Color: green if pass-rate ≥ 0.9, yellow ≥ 0.7, red below.

4. Add a tooltip / status line: "Hover info disabled in TUI; the gate
   names and last-bucket pass-rate render in the legend."

5. Tests: in `views/dashboard_view.rs::tests`, add a test that builds
   `GateStats` for three gates with diverging pass rates and asserts
   the rendered output contains all three gate names and three
   distinct color codes.

### Item 45 carry-over — CLAUDE.md smoke tests

CLAUDE.md "What to work on" items 1-9:

| # | Concern | Existing test surface |
|---|---------|----------------------|
| 1 | Rust toolchain ready | trivial — `cargo build --workspace` passes in CI |
| 2 | SystemPromptBuilder | covered by item 19 above |
| 3 | EpisodeLogger | `crates/roko-learn/src/episode_logger.rs::tests` |
| 4 | ProcessSupervisor | `crates/roko-runtime/src/process.rs::tests` |
| 5 | MCP wired | covered by plan `06` |
| 6 | Learning & feedback | partial; cascade router tested in plan `08` |
| 7 | TUI | partial; plan `05` covers event parity |
| 8 | Sidecar | `roko-agent-server` integration tests exist |
| 9 | HTTP | `roko-serve/tests/api_integration.rs` exists |

Items 6, 7, 9 are the ones that need explicit *end-to-end* smoke tests
beyond unit coverage.

1. **Smoke test for item 6**: a single test that runs a tiny
   `roko-agent` cascade through `MockDispatcher`, asserts an Episode
   lands in `.roko/episodes.jsonl`, and asserts `cascade-router.json`
   updates. Place at `crates/roko-cli/tests/smoke_learning_loop.rs`.
   (Overlaps with plan `08`'s cascade integration test; share fixtures.)

2. **Smoke test for item 7**: drive the TUI's app loop with a synthetic
   event source, assert that on a `RokoEvent::TaskCompleted` the
   relevant pane updates within 1 s. Place at
   `crates/roko-cli/tests/smoke_tui.rs`.

3. **Smoke test for item 9**: hit every aggregator route and assert
   200 + JSON shape. Already mostly covered by
   `crates/roko-serve/tests/api_integration.rs`; extend if pheromone /
   knowledge routes (plan `02`) aren't covered.

---

## Step-by-step

### Step 1 — SystemPromptBuilder snapshots (1 day)

Read `crates/roko-compose/src/system_prompt_builder.rs`. Identify:

- The constructor and the public render API.
- The role list (passed in via config or hard-coded?).
- The 6 layer names emitted in the rendered output.

Build the test scaffold:

```rust
// crates/roko-compose/tests/system_prompt_snapshots.rs

use roko_compose::system_prompt_builder::SystemPromptBuilder;

fn fixture_config(role: &str) -> roko_core::config::schema::RokoConfig { /* ... */ }

#[test]
fn implementer_snapshot() {
    let cfg = fixture_config("implementer");
    let prompt = SystemPromptBuilder::new(&cfg).render_default();
    insta::assert_snapshot!("implementer", prompt);
}
// ... one per role
```

Run `cargo test -p roko-compose --test system_prompt_snapshots` once;
inspect the new `.snap.new` files; `cargo insta accept` to bless. Commit
the `.snap` files.

### Step 2 — Clippy doc cleanup (1 day for the four key crates)

In each of `roko-core`, `roko-runtime`, `roko-gate`, `roko-compose`:

```bash
crate=roko-core
rg -t rust -l '#\[allow\(clippy::missing_(errors|panics)_doc\)\]' \
  crates/$crate/src/
```

For each file, open it, locate the public functions, add the missing
sections, then remove the `#[allow(...)]`. After:

```bash
cargo clippy -p $crate --no-deps -- -D warnings
```

Must pass. Iterate.

### Step 3 — Flaky test fix (half day)

For `exec.rs:504` and any siblings, prefer Strategy A
(`tokio::time::pause`). If the call site is hard to mock, use Strategy
B (CI scaling).

Run `cargo test -p roko-agent --release` 10 times locally. If still
flaky, the test is structurally broken and must be re-thought, not
papered over.

### Step 4 — Per-gate timeline (1.5 days)

Section by section per Deliverable item 87 above.

Anti-pattern: don't try to compute the timeline from scratch —
`verdicts.rs::GateStats` already does this; the work is rendering.

### Step 5 — Smoke tests for items 6, 7, 9 (1 day)

Each is a focused integration test with cleanup. Use `tempfile::TempDir`.

For the TUI smoke (item 7), drive `App::tick` directly with a synthetic
hub instance and a scripted event sequence. ratatui's `TestBackend`
captures the rendered frame for assertions.

### Step 6 — CLAUDE.md update (15 min)

Convert the strikethrough qualifiers from plan `09` to fully struck
items. Strike items 6, 7, 9 only after their smoke tests are merged.

### Step 7 — Followup catalogue closure (5 min)

Mark items 19, 56, 58, 87, and the carry-over from 45 as DONE in the
followup catalogue.

---

## Anti-patterns to avoid

- **Don't bless snapshot diffs without reading them.** The whole point
  is to catch silent regressions; rubber-stamping the diff defeats the
  test.
- **Don't add `# Panics` sections that read "Panics if the input is
  invalid"** without saying *what* "invalid" means. The point of the
  doc is to be useful at the call site.
- **Don't fix flakes by retrying the test.** A flaky test with retry is
  a silenced bug, not a fixed one. Use mock time or scaled timeouts.
- **Don't render every gate in a 10-line sparkline.** The dashboard
  pane has limited vertical space. Cap to the top 6 by recent
  activity, with a "+ N more" footer.
- **Don't write a smoke test that asserts only "the binary starts".**
  A smoke test that doesn't drive the actual feature is theatre.
- **Don't merge the snapshot file in a PR that also changes prompt
  rendering.** Two PRs: one to add the snapshot infrastructure, one to
  bless the latest output. Otherwise reviewers can't tell what
  changed.

## Done when

1. `cargo test -p roko-compose --test system_prompt_snapshots` passes
   with snapshots committed for every role.
2. `rg -t rust '#\[allow\(clippy::missing_(errors|panics)_doc\)\]'
   crates/{roko-core,roko-runtime,roko-gate,roko-compose}/` returns 0
   hits.
3. `cargo test -p roko-agent --release` passes 10× in a row locally.
4. The TUI Gate tab shows per-gate sparkline rows with color coding.
5. `crates/roko-cli/tests/smoke_learning_loop.rs`,
   `smoke_tui.rs` exist and pass in CI.
6. `crates/roko-serve/tests/api_integration.rs` covers all
   aggregator routes including pheromone + knowledge from plan `02`.
7. CLAUDE.md "What to work on" items 1-9 fully struck (no "smoke
   test pending" qualifier).
8. `tmp/ux/ux-followup/{03,09,14}-*.md` items 19, 56, 58, 87 marked
   DONE.
