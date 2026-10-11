+++
id = "gap-3698cd"
kind = "gap"
title = "Apply PK08's, PK78's and PK80's CLAUDE.md rows (the records plan runs write; stale prompt_experiment and watcher count; the parked features)"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["docs"]
created = 2026-10-03
updated = 2026-10-09
last_verified = 2026-10-09
last_verified_rev = "b8b8489cb"
source = "tmp/backlog/2026-10-02-complete-and-wire 2121 and 9213 (held at gates 4c and 5a)"
discovered_from = "gap-a0043b"
anchors = ["CLAUDE.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-a0043b", "gap-2339e2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Canonical signal log' CLAUDE.md && grep -q 'attempts.jsonl' CLAUDE.md"

[[verify]]
command = "! grep -q 'prompt_experiment: None' CLAUDE.md && ! grep -q '12 watchers' CLAUDE.md"

[[verify]]
command = "grep -q 'Parked (off the default build)' CLAUDE.md"

[closed]
at = 2026-10-09
at_ts = "2026-10-09T11:44:38Z"
commit = "b8b8489cb"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "Closed by b8b8489cb: CLAUDE.md: apply PK08, PK78 and PK80 rows and the missing command rows"
+++

## Problem

Two merged packages left their CLAUDE.md rows unapplied. PK08 (gap-a0043b, task 2121) says the signal log is the
kernel substrate's log, which plan runs don't write, and names the records plan runs do write (`attempts.jsonl`,
`manifest.json`, `events.jsonl`). PK78 (gap-2339e2, task 9213) replaces the stale `prompt_experiment: None` example
and the "12 watchers" count. The code they describe is merged (gates 4c and 5a). The rows aren't, because the main
checkout carries an uncommitted CLAUDE.md edit (the Goal line) that isn't the batch's, and a merge that touches
CLAUDE.md would overwrite it.

## Why it matters

CLAUDE.md is what every agent reads first. Today it calls `signals.jsonl` the canonical log, which plan runs never
write, and it names a `prompt_experiment: None` that Graph dispatch no longer passes.

## Where

`CLAUDE.md`: the Components table (Signal log row), the Absolute paths table, the Key crates table (roko-conductor
row) and the long-term priorities list.

## Current state

Both patches are below, against CLAUDE.md at 8765adea9 (the committed text under the Goal-line edit). They don't
touch the Goal line.

## Plan

1. Once the Goal-line edit is committed (or Will says to apply the rows on top of it), apply both patches to
   CLAUDE.md and commit them.
2. Run the two verifies.

## Done when

- [ ] Both `[[verify]]` commands pass.

## Notes

- Moved out of PK08's and PK78's package items on 2026-10-03 (coordinator, after gate 6b) so that the packages that
  wait on them (PK30, PK80) can start.
- 2026-10-03 (wave-6 follow-up, PK35/gap-943046): when this edit finally lands, also add three more missing
  CLI-table/docs entries found in the same pass: `roko config` has no row for `[experiments]` (the model A/B
  config section, already in `commands/config_cmd.rs`'s own table elsewhere in CLAUDE.md as "Model A/B
  experiments" but the `[experiments]` TOML section itself isn't documented in the config-schema docs either);
  `roko plan run --no-holdout` (it exists since PK35, decision 4115: it turns on `[experiments] maximize` for the
  run; gap-29fe0a covers only the spec gate's holdout, which the flag doesn't reach yet); and `roko learn patterns` has no CLI-table
  row despite existing as a real subcommand (`crates/roko-cli/src/commands/learn.rs`).
- 2026-10-03 (wave-7 follow-up, PK76/gap-99c9ae): also add `roko effects` to the "### Utilities" CLI table (no
  row today). It's a real, shipped command: `crates/roko-cli/src/commands/effects.rs::EffectsCmd` (`list`,
  `show`, `approve`, `reject`), wired at `main.rs:753-759` — lists the tool calls a `stage` policy held for
  approval and lets an operator approve or reject one. Suggested row, after `roko new`:
  `| roko effects list/show/approve/reject | List, inspect and approve or reject a staged (held) tool call |`.
- 2026-10-03 (wave-8 follow-up, PK49/gap-7ec3ef): also add two Learning-table rows, both real, shipped
  subcommands with no CLI-table row today: `roko learn self-model fit` and `roko learn self-model replay`
  (`crates/roko-cli/src/commands/learn_self_model.rs::SelfModelCmd::Fit`/`::Replay` — the M3 offline fit over S01
  run logs with a data audit and prequential scores; policy replays over a run-record matrix, S09 R-H4), and
  `roko learn econ prices` (`::EconCmd::Prices` — the price snapshot's rows with their source URLs).

PK08 (task 2121):

```diff
diff --git a/CLAUDE.md b/CLAUDE.md
index 8765adea9..6551758bc 100644
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -42,7 +42,8 @@ What each subsystem is and where it lives. The table makes no maturity or status
 | GitHub integration | `roko github status`; `GitHubOps` trait with no-op and live adapters; `[github]` config | `crates/roko-cli/src/commands/github.rs`, `crates/roko-cli/src/github_ops.rs`, `crates/roko-cli/src/github_ops_impl.rs`, `roko.toml` |
 | Plugins | Plugin manifests, declarative tools, capability policy, dependency resolution | `crates/roko-plugin/` |
 | Chain primitives | Optional chain client plus local registry, marketplace, arena and DeFi state machines | `crates/roko-chain/` |
-| Signal log | Canonical signal log (a legacy `engrams.jsonl` is read only as a fallback) | `.roko/signals.jsonl` (path logic in `crates/roko-fs/src/layout.rs`) |
+| Signal log | The kernel substrate's log (`FileSubstrate`; a legacy `engrams.jsonl` is read only as a fallback). Serve and `roko prd plan` write it; plan runs do not. `roko status` and `roko replay` read it | `.roko/signals.jsonl` (path logic in `crates/roko-fs/src/layout.rs`) |
+| Plan run records | What a plan run writes: per run, an attempt-open line and one settled verdict per attempt (`attempts.jsonl`) and the run manifest (`manifest.json`); for the workspace, the run's dashboard events (`events.jsonl`); per plan, the execution state above | `.roko/runs/<run>/` (`attempts.jsonl`, `manifest.json`), `.roko/events.jsonl` |
 
 ## Critical rules
 
@@ -327,7 +328,8 @@ safety, auth, persistence, migration, payment, or other high-risk changes. FAST
 | **Graph checkpoints** | `/Users/will/dev/nunchi/roko/roko/.roko/state/graph/` |
 | **Plans** | `/Users/will/dev/nunchi/roko/roko/plans/` |
 | **Research artifacts** | `/Users/will/dev/nunchi/roko/roko/.roko/research/` |
-| **Signal log** | `/Users/will/dev/nunchi/roko/roko/.roko/signals.jsonl` |
+| **Signal log (kernel substrate; plan runs do not write it)** | `/Users/will/dev/nunchi/roko/roko/.roko/signals.jsonl` |
+| **Plan run records** | `/Users/will/dev/nunchi/roko/roko/.roko/runs/<run>/` (`attempts.jsonl`, `manifest.json`), `/Users/will/dev/nunchi/roko/roko/.roko/events.jsonl` |
 | **Episode log** | `/Users/will/dev/nunchi/roko/roko/.roko/episodes.jsonl` |
 
 ## Reference material (read-only, do not modify)
```

PK78 (task 9213):

```diff
diff --git a/CLAUDE.md b/CLAUDE.md
index 8765adea9..847d203a2 100644
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -287,7 +287,7 @@ safety, auth, persistence, migration, payment, or other high-risk changes. FAST
 | roko-serve | `crates/roko-serve/` | HTTP control plane: REST routes + SSE + WebSocket on :6677 |
 | roko-gate | `crates/roko-gate/` | 19 gates, 7-rung pipeline, adaptive thresholds |
 | roko-compose | `crates/roko-compose/` | Prompt assembly, 11 role templates, enrichment |
-| roko-conductor | `crates/roko-conductor/` | 12 watchers, circuit breaker, diagnosis |
+| roko-conductor | `crates/roko-conductor/` | 13 watchers, circuit breaker, diagnosis |
 | roko-learn | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing, experiments, efficiency |
 | roko-cli | `crates/roko-cli/` | CLI, plan DAG/runner, merge queue, worktree manager, ratatui TUI |
 | roko-fs | `crates/roko-fs/` | FileSubstrate (JSONL), GC, layout |
@@ -387,8 +387,9 @@ Long-term priorities that still hold:
    end. The blockers from the first dogfood run have regression fixes, but no live rerun has been
    recorded.
 2. **Learning loops on the Graph path**: several feedback paths were attached to the deleted
-   Runner-v2 event loop and have not been re-attached to Graph runs. For example, Graph task
-   dispatch passes `prompt_experiment: None` (`crates/roko-cli/src/graph_task_dispatch.rs:1791`).
+   Runner-v2 event loop and have not been re-attached to Graph runs. For example, Graph runs never
+   emit `FeedbackEvent::PlanCompleted`, so the dream, daimon, theta and delta plan-completion sinks
+   never fire (q-6b7cca).
 3. **roko tracks its own work**: the `roko work` CLI (it extends `roko backlog`, per
    `work/README.md`) plus plan-task `closes = [...]` links, so that roko itself maintains the work
    graph.
```

PK80 (task 9227; added at gate 7a, 2026-10-03). Against CLAUDE.md as committed at 9fe7b177c; the worker's patch put
the new section inside the Components table, and this one places it after the table:

```diff
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -28,7 +28,7 @@
 | Runtime services | Shared `RuntimeServices` builder (the service facade for CLI, serve and ACP) | `crates/roko-execution/` |
 | Process supervision | ProcessSupervisor, event bus, cancellation | `crates/roko-runtime/` |
 | Execution state | Per-plan Graph checkpoint, activity log and cost state | `.roko/state/graph/<plan>/` (`checkpoint.json`, `activities.jsonl`, `costs.json`) |
-| Episodes | Per-turn episode records. `hdc_fingerprint` is `HdcVector::from_seed` of the serialized prompt and outcome, i.e. one 64-bit FNV-1a hash expanded into a vector. It identifies exact inputs; it does not measure semantic similarity | `crates/roko-cli/src/runtime_feedback/episodes.rs`, `crates/roko-primitives/src/hdc.rs`, `.roko/episodes.jsonl` |
+| Episodes | Per-turn episode records. `hdc_fingerprint`, written only when `[learning] episode_hdc_fingerprint = true` (off by default), is `HdcVector::from_seed` of the serialized prompt and outcome, i.e. one 64-bit FNV-1a hash expanded into a vector. It identifies exact inputs; it does not measure semantic similarity | `crates/roko-cli/src/runtime_feedback/episodes.rs`, `crates/roko-primitives/src/hdc.rs`, `.roko/episodes.jsonl` |
 | Learning | Model routing (CascadeRouter), bandits, playbooks, prompt experiments, efficiency events | `crates/roko-learn/`; state in `.roko/learn/` (`cascade-router.json`, `gate-thresholds.json`, `efficiency.jsonl`) |
 | Knowledge and dreams | Durable knowledge store, distillation, tiers; offline Dream consolidation | `crates/roko-neuro/`, `crates/roko-dreams/` |
 | Affect | Daimon affect engine and dispatch modulation | `crates/roko-daimon/` |
@@ -41,8 +41,26 @@
 | TUI and chat | ratatui dashboard (`roko dashboard`) with a file watcher; `roko chat` REPL | `crates/roko-cli/src/tui/` (`fs_watch.rs`), `crates/roko-cli/src/chat.rs` |
 | GitHub integration | `roko github status`; `GitHubOps` trait with no-op and live adapters; `[github]` config | `crates/roko-cli/src/commands/github.rs`, `crates/roko-cli/src/github_ops.rs`, `crates/roko-cli/src/github_ops_impl.rs`, `roko.toml` |
 | Plugins | Plugin manifests, declarative tools, capability policy, dependency resolution | `crates/roko-plugin/` |
-| Chain primitives | Optional chain client plus local registry, marketplace, arena and DeFi state machines | `crates/roko-chain/` |
+| Chain primitives | Chain client plus local registry, marketplace, arena and DeFi state machines; parked (see below) | `crates/roko-chain/` |
 | Signal log | Canonical signal log (a legacy `engrams.jsonl` is read only as a fallback) | `.roko/signals.jsonl` (path logic in `crates/roko-fs/src/layout.rs`) |
+
+## Parked (off the default build)
+
+Decision 9201 parked what the plan path never uses behind cargo features that are off by default
+(the conductor stays on). A default `roko` build leaves these out; build with
+`cargo build -p roko-cli --features <feature>` (or `-p <crate>` for a crate feature) to bring one
+back. Line counts are in `benchmarks/park/`.
+
+| Feature | Crates | Brings back |
+|---|---|---|
+| `chain` | roko-cli, roko-serve | roko-chain, the 17 `chain.*` tools, the chain-family routes (501 without it), chain state and feed agents, x402 paid feeds, chain jobs |
+| `alloy-backend` | roko-cli, roko-serve | Real EVM JSON-RPC (implies `chain`) |
+| `groups` | roko-cli, roko-serve | Agent groups and pheromone state, and their routes |
+| `relay` | roko-cli, roko-serve | Relay registration, the subscription and feed relay bridge, the `/relay` proxy |
+| `cognitive-clock` | roko-cli, roko-runtime | The heartbeat clock, `CorticalState`, the theta and delta consumers and sinks, the attention auction, heartbeat probes |
+| `cross-cut-functors` | roko-compose | The cross-cut functors and `CrossCutArbitrator` |
+| `spc` | roko-gate | CUSUM, EWMA and BOCPD detectors, PELT, Hotelling's T-squared |
+| `active-inference` | roko-learn | The expected-free-energy tier selector |
 
 ## Critical rules
 
@@ -291,7 +309,7 @@
 | roko-learn | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing, experiments, efficiency |
 | roko-cli | `crates/roko-cli/` | CLI, plan DAG/runner, merge queue, worktree manager, ratatui TUI |
 | roko-fs | `crates/roko-fs/` | FileSubstrate (JSONL), GC, layout |
-| roko-std | `crates/roko-std/` | 35 definitions by default (16 executable local + 19 GitHub MCP); 52 with typed optional-chain placeholders; HTTP MCP clients/resolvers retained at runtime |
+| roko-std | `crates/roko-std/` | 35 definitions in a default build (16 executable local + 19 GitHub MCP); 52 with `--features chain` (the 17 chain tools); HTTP MCP clients/resolvers retained at runtime |
 | roko-execution | `crates/roko-execution/` | RuntimeServices builder, diagnostic service, execution control, feedback settlement |
 | roko-runtime | `crates/roko-runtime/` | ProcessSupervisor, event bus, cancellation, workflow contract |
 | roko-primitives | `crates/roko-primitives/` | HDC vectors, tier routing |
@@ -306,7 +324,7 @@
 | roko-plugin | `crates/roko-plugin/` | Plugin manifests, executable declarative tools, canonical tier/capability policy, semantic-version/dependency resolution |
 | roko-graph | `crates/roko-graph/` | Graph engine, DAG cells, topology, cost state |
 | roko-demo | `crates/roko-demo/` | Demo/example binary for showcasing features |
-| roko-chain | `crates/roko-chain/` | Optional chain client/runtime primitives plus tested local registry, marketplace, arena, and DeFi state machines. daeji owns node/BFT/precompiles in a separate repo. |
+| roko-chain | `crates/roko-chain/` | Parked (`--features chain`): chain client/runtime primitives plus tested local registry, marketplace, arena, and DeFi state machines. daeji owns node/BFT/precompiles in a separate repo. |
 
 ## Absolute paths
 
```
