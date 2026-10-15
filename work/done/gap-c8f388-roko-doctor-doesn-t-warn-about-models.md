+++
id = "gap-c8f388"
kind = "gap"
title = "roko doctor doesn't warn about models roko has no price for, whose calls a budget cap cannot count"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/doctor"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "094f9a15c"
source = "roko-init round v5, 2026-10-15 (team lead: plugin wrote unpriced [models.*], caps did not hold)"
anchors = ["crates/roko-cli/src/doctor.rs::check_model_prices", "crates/roko-cli/src/dispatch/model_routing.rs::builtin_rung_model", "crates/roko-cli/src/commands/init.rs::MODELS"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b", "bug-1bc222", "gap-ceffb3", "gap-2bec05", "gap-8548e3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn doctor_warns_about_unpriced_models' crates/roko-cli/ && cargo test -p roko-cli --lib doctor_warns_about_unpriced_models"

[[verify]]
command = "grep -rqw 'fn every_builtin_model_has_a_price' crates/roko-cli/ && cargo test -p roko-cli --lib every_builtin_model_has_a_price"

[[verify]]
command = "grep -rqw 'fn every_model_init_writes_has_a_price' crates/roko-cli/ && cargo test -p roko-cli --lib every_model_init_writes_has_a_price"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T21:06:21Z"
commit = "094f9a15c"
by = "roko-init"
executor = "claude-agent"
forced = false
evidence = "094f9a15c on main 4505cd54d. roko doctor's new model_prices check names the models a task can be routed to that have no price ([models.*] entries, ladder rung models, the default model), judged as a run judges them (dispatch_v2::model_has_price: price snapshot, entry rates, built-in table). builtin_rung_model refuses an unpriced built-in. Tests at 094f9a15c: doctor_warns_about_unpriced_models, every_builtin_model_has_a_price and every_model_init_writes_has_a_price pass; all roko-cli lib/bin and roko-core tests plus cli_fallback, doctor and agent_lifecycle pass; clippy -D warnings and fmt clean. Live: roko doctor in a fresh init workspace on Will's machine warns about gemma4, glm-5v-turbo and llama32 from his global config, and about nothing init wrote. Up-front refusal of capped runs is filed as gap-8548e3. Evidence: tmp/hermes-roko-v5/findings/roko-init.md"
+++

## Problem

roko records a call on a model it has no price for at an unknown cost, counted as $0. The run's price snapshot,
the `[models.*]` entry's `cost_input_per_m`/`cost_output_per_m` and the built-in pricing table all lack it
(`dispatch_v2::model_has_price`). A budget cap counts only priced spend. So under a cap (a plan's `[budget]
max_plan_usd`, or the `max_usd` of a run a chat host starts) such a run stops only after its first unpriced call
has already run: "plan budget cannot be enforced: 1 unpriced call(s) …" (decision 2110, backlog 2111).

Nothing says so before a run:
- `roko doctor` has no check on prices.
- Nothing guards that the models `roko init` writes, and the built-in models a ladder rung may name (bug-4e336b),
  stay priced.

Seen 2026-10-15: the Hermes plugin's `hermes roko init` first wrote `[models.*]` tables without prices. roko
recorded those runs at $0.00, and their `max_usd` caps did not hold.

## Why it matters

A spend cap that cannot count a model's calls is not a cap. People set caps for chat-started runs precisely
because they don't watch them.

## Where

- `crates/roko-cli/src/doctor.rs`: no pricing check.
- `crates/roko-cli/src/dispatch_v2.rs::model_has_price`: what a run counts as a price.
- `crates/roko-cli/src/commands/init.rs::MODELS`: the models init writes; the Claude ones carry no rates of
  their own and rely on the built-in table.
- `crates/roko-cli/src/dispatch/model_routing.rs::builtin_rung_model`: binds a built-in model to a ladder rung.

## Plan

1. `roko doctor` warns about the models a task can be routed to that have no price: every `[models.*]` entry,
   each ladder rung's model and the default model, priced the way a run prices them. It names each model and says
   what to set.
2. A ladder rung never binds a built-in model roko cannot price.
3. Tests that every built-in model and every model init writes has a price.
4. Refusing a capped run up front is the separate item filed with this one.

## Done when

`doctor_warns_about_unpriced_models`, `every_builtin_model_has_a_price` and `every_model_init_writes_has_a_price`
pass (the `[[verify]]` commands).
