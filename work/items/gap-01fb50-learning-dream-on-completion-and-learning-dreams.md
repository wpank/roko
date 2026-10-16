+++
id = "gap-01fb50"
kind = "gap"
title = "[learning] dream_on_completion and [learning.dreams] trigger_on_plan_complete gate nothing since gap-a960a1 removed the plan-completion dream"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-a960a1 (2026-10-16)"
discovered_from = "agent:gap-a960a1 worker (work/gap-a960a1)"
anchors = ["crates/roko-core/src/config/learning.rs::DreamsConfig", "crates/roko-core/src/config/learning.rs::LearningConfig"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-a960a1", "q-6b7cca", "bug-470de8", "dec-e70592", "spec-9a3131", "spec-7a71d6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dream_trigger_keys_load_but_do_nothing' crates/roko-core/ && cargo test -p roko-core dream_trigger_keys_load_but_do_nothing"
+++

## Problem

Since gap-a960a1 (q-6b7cca option 2) no plan run starts a dream. `DreamConsolidationSink`, the only code that read
`[learning] dream_on_completion` and `[learning.dreams] trigger_on_plan_complete`, is deleted. Both keys still parse.
They still appear in `roko.toml` (`dream_on_completion = false`, `trigger_on_plan_complete = true`), in the config
display (`schema.rs`, `dream_on_completion = …`) and in the presets. Their docs (`config/learning.rs`) still say they
trigger a dream after a plan completes. Setting either one to `true` now does nothing, and nothing says so.

## Why it matters

The config misleads: an operator who opts in expects a dream after each plan, gets none, and sees no warning.
`DreamsConfig::max_concurrent`'s doc also says "The plan-completion trigger keeps its own limit of one", which is now
false. Related: q-6b7cca, gap-a960a1, bug-470de8, dec-e70592. spec-9a3131's verify command greps `roko.toml` for
`^dream_on_completion = false`.

## Where

- `crates/roko-core/src/config/learning.rs::DreamsConfig`: `trigger_on_plan_complete`, and the `max_concurrent` doc.
- `crates/roko-core/src/config/learning.rs::LearningConfig`: `dream_on_completion`.
- `crates/roko-core/src/config/schema.rs`: the config display prints `dream_on_completion`.
- `crates/roko-core/src/config/presets.rs`: two presets set the key, and one test checks it.
- `crates/roko-core/src/config/loader.rs`: a test loads `dream_on_completion = true`.
- `crates/roko-cli/src/config.rs`: a `roko config set learning.dreams.trigger_on_plan_complete` test.
- `roko.toml`: `[learning]` and `[learning.dreams]`.

## Current state

Both keys parse. `DreamsConfig` is `deny_unknown_fields`, so removing the field would stop every config that still sets
it from loading. After gap-a960a1, `grep -rn 'dream_on_completion\|trigger_on_plan_complete' crates/ --include='*.rs'`
finds only the config types, the display, the presets and tests. Dreams run on demand, through
`roko knowledge dream run`.

## Plan

Options:

1. Keep both keys parsing, but make them visibly inert:
   - say so in their docs;
   - drop them from `roko.toml` and the presets;
   - when either is `true`, warn once at load: no plan run starts a dream; run `roko knowledge dream run`;
   - update spec-9a3131's verify command, which greps `roko.toml` for the line.
2. Remove both fields, with a `roko config migrate` step that strips them so old configs keep loading.

Recommended: option 1. It can be undone, and it breaks no existing config.

## Done when

- The named test passes: a config that sets `dream_on_completion = true` loads, and its load reports that the key does
  nothing.
- The keys' docs no longer promise a dream after a plan.

## Notes

- Leave `max_concurrent`, `trigger_on_acp_episodes` and `acp_episode_threshold` alone: ACP still reads them.
- Filed while doing gap-a960a1 (2026-10-16), which kept to roko-cli and left the config surface to this item.
