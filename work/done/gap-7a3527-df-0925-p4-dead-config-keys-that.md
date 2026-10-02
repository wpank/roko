+++
id = "gap-7a3527"
kind = "gap"
title = "Dead config keys that give false confidence"
status = "done"
triage = "verified"
severity = "p2"
size = "M"
goal = "tooling"
subsystem = ["roko-core/config"]
created = 2026-09-25
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "6a08f9e2c"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["crates/roko-core/src/config/gates.rs::GatesConfig", "crates/roko-core/src/config/learning.rs::LearningConfig", "crates/roko-core/src/config/agent.rs::AgentConfig", "crates/roko-gate/src/adaptive_threshold.rs::AdaptiveThresholds::from_gates_config", "crates/roko-cli/src/runner/persist.rs::GateThresholds::observe", "crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings", "crates/roko-cli/src/config.rs::LearningLayer"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqE '\"(gates\\.domain_gates|learning\\.replan_max_per_plan|learning\\.replan_gate_attempts)\"' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_task_dispatch/ && ! grep -qE 'pub (domain_gates|replan_max_per_plan|replan_gate_attempts):' crates/roko-core/src/config/gates.rs crates/roko-core/src/config/learning.rs && { ! grep -q 'fn from_gates_config' crates/roko-gate/src/adaptive_threshold.rs || grep -rn --include='*.rs' 'from_gates_config' crates/ | grep -v 'crates/roko-gate/src/adaptive_threshold.rs' | grep -q .; } && grep -rqw 'fn dead_config_keys_are_removed_and_old_files_still_load' crates/roko-core/src/ && cargo test -p roko-core --lib dead_config_keys_are_removed_and_old_files_still_load"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T07:43:02Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T16:13:54Z"
forced = false
evidence = "dead keys removed with a loader test that old files still load, gates.ema_alpha drives the Graph EMA (wk-cfg 48acf7975); gate 6h2 passed at 285282248 (cargo check, clippy -D warnings, 11,366 lib tests in roko-agent/cli/core/fs/gate/graph/learn/serve, canaries C1-C8 plus integration tests, 446 roko-cli bin tests, run_evidence py, portal tsc and 809 vitest); merged in 6a08f9e2c"
+++

## Problem

Several `roko.toml` keys parse, appear in `roko config show`, presets and docs, but no production
code reads them. An operator who sets them expects a behaviour change and gets none. Still dead at
HEAD:

- `gates.domain_gates` (`GatesConfig::domain_gates`, `crates/roko-core/src/config/gates.rs:165`).
  `docs/v2/19-CONFIG.md:651` and `docs/v2/INTEGRATION-GUIDE.md:680-690, :1870` present it as
  per-domain gate commands. Only a parse test (`crates/roko-cli/tests/e2e_domain.rs:116`) and the
  inert-settings warning touch it. Example of the false confidence: `[gates.domain_gates]
  docs = ["shell:markdownlint ."]` lints nothing.
- `learning.replan_max_per_plan` and `learning.replan_gate_attempts`
  (`crates/roko-core/src/config/learning.rs:81`, `:84`). This repo's own `roko.toml:369-370` sets
  them, presets set them, and `roko config set` accepts them. Nothing uses the values: the Graph
  engine never revises a plan on gate failure.
- `agent.data_llm` (`AgentConfig::data_llm`, `crates/roko-core/src/config/agent.rs:81`). Its own doc
  comment says "no production dispatch path currently consults this field". Setting it suggests that
  untrusted content goes to a separate, isolated LLM. It does not.
- The adaptive gate keys `gates.ema_alpha`, `adaptive_min_retries`, `adaptive_max_retries`,
  `skip_streak_threshold` and `convergence_min_observations` (set in `roko.toml:220-224`). Their only
  reader is `AdaptiveThresholds::from_gates_config` (`crates/roko-gate/src/adaptive_threshold.rs:322`).
  Its doc comment calls it "the preferred constructor in production", but only its own tests call it
  (`:1278` onward). The Graph path learns thresholds with a different type,
  `runner::persist::GateThresholds`, whose `observe` hard-codes alpha `0.1`
  (`crates/roko-cli/src/runner/persist.rs:344`).

Expected: each key either changes what `roko plan run` does, or is gone from the schema.

## Why it matters

Goal `tooling` (Tooling, CLI polish and code hygiene). Config that silently does nothing misleads
operators and anyone reading the docs. `agent.data_llm` looks like a safety control, so it is the
worst of the set. Related: `find-4b4344` (gate and threshold closures that were wired only into the
deleted Runner-v2). That item decides whether `AdaptiveThresholds` ever runs on the Graph path,
which decides what happens to the adaptive keys here. These keys come from section P4 ("dead config
and dead code") of the 2026-09-25 portal-programme dogfood report. `skip_enrichment` from that list
is already fixed.

## Where

- The key declarations are in `crates/roko-core/src/config/`, in `GatesConfig`, `LearningConfig`
  (defaults at :192-213) and `AgentConfig`. Keep the `DataLlmConfig` type (`agent.rs:289`), because
  `roko_agent::safety::data_llm::DataLlmRouter` uses it (`crates/roko-agent/src/safety/data_llm.rs:27`).
- The replan keys are also carried by:
  - `crates/roko-core/src/config/presets.rs:109-110, :180-181`;
  - `schema.rs:1394-1400` (rendering);
  - `crates/roko-cli/src/config.rs::LearningLayer` (:1158-1192);
  - the `config set` allowlist (`crates/roko-cli/src/config.rs:1583-1584`).
  `loader.rs:1506` sets `data_llm` only on the schema-sentinel config.
- `crates/roko-cli/src/runner/persist.rs::GateThresholds::observe` (:337) is the EMA that the Graph
  path really updates. The update runs in `crates/roko-cli/src/graph_task_dispatch.rs` at about
  :2203-2260. `GraphFeedbackContext` (:1012) is built in
  `crates/roko-cli/src/graph_execution/plan_runner.rs` at about :1052-1062.
- `graph_task_dispatch.rs::graph_engine_inert_settings` (:792) is the warning list. Its test
  `inert_settings_list_only_changed_keys_the_graph_engine_ignores` (:7303) uses `gates.domain_gates`
  as its example key.
- Entry points:
  - `roko plan run <dir>` warns once, when the dispatcher is built (:985-988);
  - `roko config doctor` lists the keys (`crates/roko-cli/src/config_cmd.rs:254`).

## Current state

- `skip_enrichment` is fixed. `plan_skips_enrichment` (`graph_task_dispatch.rs:1373`) has read it
  since 725f21e05, and it has been part of the plan fingerprint since 3d0637232.
- 725f21e05 added `graph_engine_inert_settings`, which only warns. It wires nothing.
- `learning.replan_on_gate_failure` is read on Graph (`plan_runner.rs:1059`), but only to log a
  message and to start a post-gate LLM reflection (`graph_task_dispatch.rs:2411`, `:2579`). There
  is no Graph replan loop, so `replan_max_per_plan` and `replan_gate_attempts` have nothing to
  limit. `crates/roko-graph/src/plan_mutation.rs` has split and merge helpers, but no gate-failure
  trigger.
- `TaskDef::domain` exists (`crates/roko-cli/src/task_parser.rs:107`), so the Graph verify step
  could look up `domain_gates` if you choose to wire it.
- Config loading will not break when fields are removed. `ConfigMigrator` (`loader.rs:80`) runs
  first. Then unknown keys are diagnosed and removed by `strip_unknown_fields` (`loader.rs:2043`,
  called at :607) before deserialising, so `#[serde(deny_unknown_fields)]` on these structs does
  not make an old key fatal. `validate_known_config_paths` (`loader.rs:1417`) reports unknown keys.
  `LEGACY_REMOVED_SECTIONS` (`loader.rs:1394`) matches full dotted paths (`:1621-1623`), so an entry
  such as `"learning.replan_max_per_plan"` gives a targeted "removed" message in place of a generic
  unknown-key one. Unknown: whether `roko config validate` counts that diagnostic as an error.
- Side issue, out of scope except to keep the list accurate: the `LEGACY_GATES` reason string in
  `graph_engine_inert_settings` still says "(--engine legacy)", and that flag is now rejected.
  Some of those keys are read elsewhere, for example `gates.skip_tests` in
  `crates/roko-acp/src/session.rs:302`.

## Plan

Recommended: wire the one key that is a one-line change, and delete the keys that have no meaning
on the Graph path.

1. `gates.ema_alpha`: wire it. Add `ema_alpha: f64` to `GraphFeedbackContext` (default `0.1`). Fill
   it in `plan_runner.rs` from `roko_config.gates.ema_alpha`, clamped to the open interval (0, 1).
   Make `GateThresholds::observe` take the alpha, or add `observe_with_alpha`. Remove
   `gates.ema_alpha` from the inert list. Add a unit test showing that a non-default alpha changes
   `ema_pass_rate`.
2. `adaptive_min_retries`, `adaptive_max_retries`, `skip_streak_threshold`,
   `convergence_min_observations` and `from_gates_config`. There are two options:
   - (a) Delete the four keys and `from_gates_config`, together with its tests
     (`adaptive_threshold.rs:1278-1420`), the `roko.toml:221-224` lines and the docs. This is cheap
     and honest.
   - (b) Wire them: build `AdaptiveThresholds::from_gates_config` on the Graph path and let
     `suggested_max_retries` drive retry limits. This is the "P3-15 retry alignment" closure in
     `find-4b4344`, and it is L-sized.
   Recommend (a), unless `find-4b4344` has already put `AdaptiveThresholds` on the Graph path. In
   that case, construct it there with `from_gates_config`.
3. `learning.replan_max_per_plan` and `learning.replan_gate_attempts`: delete them. Remove the fields
   and their defaults, the preset entries, the `LearningLayer` fields, the `config set` allowlist
   arms, the `schema.rs` rendering and `roko.toml:369-370`. For each key you delete in steps 2-5,
   add a `LEGACY_REMOVED_SECTIONS` entry that says why it was removed. If `roko config validate`
   treats that diagnostic as an error, also add a `ConfigMigrator` step that drops the key.
4. `gates.domain_gates` has two options:
   - (a) Delete the field, the two docs and the parse test.
   - (b) Wire it: in the Graph verify step, when `task.domain` matches a key, also run those commands
     (after removing the `shell:` prefix).
   Recommend (a). Plan tasks already declare their own `verify` commands, and (b) needs new rules
   (append or replace, and how domains are named).
5. `agent.data_llm`: remove the field from `AgentConfig` and the sentinel line at `loader.rs:1506`.
   Keep `DataLlmConfig` and `DataLlmRouter` for the future CaMeL work. The unknown-key diagnostic
   then names the key if a user still sets it.
6. Update `graph_engine_inert_settings`. Drop the keys you wired or deleted, fix the stale
   `LEGACY_GATES` wording, and change the test at :7303 to use a key that is still inert (for
   example `runner.warm_pool_size`).
7. Update `docs/v2/19-CONFIG.md` and `docs/v2/INTEGRATION-GUIDE.md` to match.

## Done when

- `gates.domain_gates`, `learning.replan_max_per_plan`, `learning.replan_gate_attempts` and
  `agent.data_llm` are each deleted from the config structs, or have a production reader on the
  `plan run` path.
- `AdaptiveThresholds::from_gates_config` is deleted, or is called from Graph code.
- A unit test shows that `gates.ema_alpha` changes the Graph EMA, or the key is deleted.
- `roko config doctor` on the repo's `roko.toml` reports none of these keys as inert, and
  `roko config validate` still passes on it.
- Removing a key from `graph_engine_inert_settings` without wiring or deleting it does not count.
- The `[[verify]]` command passes:
  `! grep -qE '"(gates\.domain_gates|learning\.replan_max_per_plan|learning\.replan_gate_attempts|agent\.data_llm)"' crates/roko-cli/src/graph_task_dispatch.rs && { ! grep -q 'fn from_gates_config' crates/roko-gate/src/adaptive_threshold.rs || grep -rn --include='*.rs' 'from_gates_config' crates/ | grep -v 'crates/roko-gate/src/adaptive_threshold.rs' | grep -q .; }`

## Notes

- This changes the config schema (`roko-core`). Presets, `roko.toml`, `config set`, the TUI
  config view (`crates/roko-cli/src/tui/config_meta.rs`) and `roko-serve` config routes can all
  mention keys, so grep each key across `crates/`, `docs/` and `roko.toml` before deleting it.
- Keep `DataLlmRouter`. The safety rule in `CLAUDE.md` is to fail closed, so do not make
  `agent.data_llm` look supported.
- Coordinate with `find-4b4344`, which also touches `runner/persist.rs::GateThresholds` and the
  Graph threshold update in `graph_task_dispatch.rs`. Do not run the two in parallel. Parallel work
  on other items is safe.
- Size is M: several small deletions, one wiring change, and the docs and test updates.

- 2026-10-01 (wk-cfg): PARTIAL, implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Landed plan steps 3, 4(a), 5, 6 and 7 for the four keys of the first Done-when bullet. `gates.domain_gates`,
  `learning.replan_max_per_plan`, `learning.replan_gate_attempts` and `agent.data_llm` are gone from `GatesConfig`,
  `LearningConfig` and `AgentConfig`, and from their defaults, the presets, the example renderer, the schema
  sentinels (and `DYNAMIC_MAP_SECTIONS`), the CLI `LearningLayer`, the `config set` arms, `roko.toml` and the
  inert-settings list (whose test now uses `gates.max_rung`, and whose stale "--engine legacy" wording is fixed).
  `DataLlmConfig` and `DataLlmRouter` stay. Each key has a `REMOVED_CONFIG_KEYS` entry in `loader.rs` (added for
  gap-6bc156): validation reports it as removed, with the reason; loading strips it with a warning; and
  `RokoConfig::from_toml` drops it with a warning, so old files still load and still parse. Test
  `dead_config_keys_are_removed_and_old_files_still_load`. Docs: `docs/v2/19-CONFIG.md`, `docs/v2/INTEGRATION-GUIDE.md`
  and a new "Removed keys" table in `docs/v3/depth/21-config/01-schema-sections.md`. The ignored e2e test
  `config_with_domain_gates_parses` is deleted.
- The verify is re-pointed: its first clause grepped `graph_task_dispatch.rs`, but the inert list moved to
  `graph_task_dispatch/inert_settings.rs`, so that clause passed at BASE without any change. The new verify also checks
  that the four fields are gone and runs the new test. It still fails on `from_gates_config`, which is the work left.
- Left (plan steps 1 and 2): wire `gates.ema_alpha` into `GateThresholds::observe` on the Graph path, and delete
  `AdaptiveThresholds::from_gates_config` (a duplicate of `new()` + `apply_gates_config`, called only by its own tests),
  then decide `skip_streak_threshold` and `convergence_min_observations`, which only `should_skip_rung` and the
  convergence check read and the Graph path never calls. Not done this round: `adaptive_threshold.rs` is under
  wk-tuiv's bug-d74b6b (`apply_gates_config`, next to `from_gates_config`), `runner/persist.rs` and
  `graph_task_dispatch/verification.rs` are under wk-honestbench's bug-e0f472 and reg-c7ecf6, and `plan_runner.rs` is
  under wk-planrun's gap-7c9e48, and this item asks to coordinate with find-4b4344 (open, unclaimed). Next step: once
  those land, do steps 1-2 in one change.

- 2026-10-02 (wk-cfg): the remainder (plan steps 1 and 2) is implemented on work/bug-ccfa0d; cargo verification is
  deferred to the batch check. Who reads what at `f8906b3c0`:
  - Graph plan runs learn thresholds through `runner::persist::GateThresholds`, whose EMA used a fixed alpha of 0.1.
    Their `AdaptiveThresholds` (retry budgets) applies `[gates]` but reads only the retry bounds.
  - ACP loads `AdaptiveThresholds` from `gate-thresholds.json` and never applies `[gates]`, so it uses the alpha and
    skip streak the file holds.
  - The TUI, `roko status` and serve apply `[gates]` only to display.
- Chose wiring. `GateThresholds::observe_with_alpha` takes the alpha, `observe_verify_steps` passes it, and Graph
  verify runs give it `GatesConfig::effective_ema_alpha()` (a value outside (0, 1) counts as the default 0.1), so
  `gates.ema_alpha` now drives the Graph EMA. The test-only `observe` keeps the default. `gates.ema_alpha` left the
  inert list. `AdaptiveThresholds::from_gates_config` is deleted, being `new()` plus `apply_gates_config` with no
  production caller; its tests use that pair. Test `gate_threshold_ema_uses_the_configured_alpha` runs a dispatcher
  with `ema_alpha = 0.5` and shows a failure moving the EMA halfway. The `ema_alpha` field doc said smaller values
  weight recent outcomes more; it is the reverse, and the doc now says so.
- Still listed as inert, with an accurate reason: `gates.skip_streak_threshold` (plan runs never skip a verify step;
  ACP reads the streak saved in the file, not the config) and `gates.convergence_min_observations` (its only reader,
  `AdaptiveThresholds::promote_converged`, has no production caller). The repo's roko.toml sets both at their
  defaults, so `config doctor` reports neither. Suggested follow-up: delete `convergence_min_observations`, and either
  make ACP apply `[gates]` (its `PipelineConfig` would need the values) or delete `skip_streak_threshold`.

## Original notes

No read sites for skip_enrichment, gates.domain_gates, learning.replan_max_per_plan/replan_gate_attempts, agent.data_llm; adaptive gate keys only via uncalled AdaptiveThresholds::from_gates_config; most [routing] and [gates] keys are display-only or Runner-v2 only.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`

How to verify: grep each key for read sites; wire or delete.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed in 725f21e05: [meta] skip_enrichment is now read by Graph dispatch (plan_skips_enrichment, crates/roko-cli/src/graph_task_dispatch.rs:1311-1330, used at :3061-3074). An uncommitted change also adds it to the plan fingerprint (crates/roko-graph/src/fingerprint.rs:24). Still dead: gates.domain_gates, learning.replan_max_per_plan, learning.replan_gate_attempts and agent.data_llm have no production reader. The same commit only lists them, together with the legacy-only [gates] keys and display-only [routing] keys, in graph_engine_inert_settings (graph_task_dispatch.rs:764-900), which warns once at dispatch (:957, :1161) and in `config doctor` (config_cmd.rs:254). AdaptiveThresholds::from_gates_config (crates/roko-gate/src/adaptive_threshold.rs:322) is still called only from its own tests module (:946 onward). Remaining work: wire or delete these keys; so far they are only warned about.

Re-checked 2026-09-29 at d9e79e9d8: the plan-fingerprint change for skip_enrichment (crates/roko-graph/src/fingerprint.rs:24) is now committed (3d0637232), so skip_enrichment is fully wired. Still remaining: gates.domain_gates (config/gates.rs:165), learning.replan_max_per_plan and learning.replan_gate_attempts (config/learning.rs:81, :84) and agent.data_llm (config/agent.rs:81) have no production reader and are only warned about through graph_engine_inert_settings. AdaptiveThresholds::from_gates_config (roko-gate/src/adaptive_threshold.rs:322) still has no caller outside its tests. Each of these keys still has to be wired or deleted.

Checked 2026-09-29: Partly fixed: adaptive_min_retries/adaptive_max_retries are now live, and the adaptive retry floor defaults to 3 (41c7ffbd6). Still inert: gates.domain_gates, learning.replan_max_per_plan, learning.replan_gate_attempts and agent.data_llm.
- 2026-10-01 (coordinator): agent.data_llm left this item's verify: gap-b0d514 re-adds [agent.data_llm] as the CaMeL boundary's config (wk-childenv), so it is no longer a dead key.
