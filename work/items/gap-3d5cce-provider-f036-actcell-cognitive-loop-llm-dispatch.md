+++
id = "gap-3d5cce"
kind = "gap"
title = "ActCell (cognitive loop LLM dispatch point) is a stub pass-through"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-graph/cells"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F036"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F036"
anchors = ["crates/roko-graph/src/cells/cognitive.rs::ActCell", "crates/roko-graph/src/engine.rs:3008"]
links = { depends_on = [], blocks = [], related = ["gap-83cee0", "gap-57bc59"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! awk '/impl Cell for ActCell/,/^}/' crates/roko-graph/src/cells/cognitive.rs | grep -q 'Ok(input)' && cargo test -p roko-graph act_cell_dispatches_through_provider"
+++
The cognitive loop graph's `ActCell` — designated as the LLM dispatch point with `Protocol: Connect` — implements `execute()` as `Ok(input)`. All six other cognitive cells (Sense, Assess, Compose, Verify, Persist, React) are similarly stubs except `SenseCell`. The cognitive graph cannot perform a...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F036`
- `tmp/archive/provider-audit/20-graph-integration.md`

How to verify: Roadmap AR-7 (wire cognitive cells) deferred; CLAUDE.md claims seven cognitive Cells wired in roko-graph. Check ActCell dispatch. Confirm in crates/roko-graph/src/cells/cognitive.rs whether still true: `ActCell` (cognitive loop LLM dispatch point) is a stub pass-through

Verified 2026-09-28: ActCell::execute only traces and returns Ok(input) (crates/roko-graph/src/cells/cognitive.rs:414-416). The other six cognitive cells also return their input (Ok(input) at cognitive.rs:174, 286, 354, 483, 543, 602), so CLAUDE.md's claim of seven wired cognitive Cells is overstated. gap-83cee0 describes the same pass-through cells and should be merged into this item. Severity p2: Graph plan execution dispatches through task_executor cells, not the cognitive loop.

Rechecked 2026-09-29 at d9e79e9d8: unchanged. ActCell::execute still returns Ok(input) (cognitive.rs:414-416) and the registry still builds it for the act node (engine.rs:2999-3008). The parked gap-83cee0 (created 2026-08-31) reports the same pass-through cognitive cells; it is older, so if it is ever revived it should absorb this item; otherwise record it in this item's links.supersedes.

## Notes

- 2026-10-01 (wk-filer4): blocked on a design call, so no code changed. The premise holds at ebdc0f5d5: `ActCell::execute` still returns `Ok(input)` (crates/roko-graph/src/cells/cognitive.rs:414-416), and `default_registry` builds it for both `act` and the `claude-agent` alias (engine.rs:3341-3352, 3432-3443). roko-graph has no prompt-level provider port: TaskExecutorCell dispatches whole plan tasks through `TaskDispatcher`, and `CellResources` carries only gates and workspaces. Nothing in production runs the cognitive loop, and the cells around ActCell (Compose, Verify, Persist) pass their input through too, so a dispatching ActCell alone would not make the loop work.
- 2026-10-01 (wk-filer4): Next step: choose the port. One option is a prompt-dispatch service in `CellResources`, implemented in roko-cli over the agent dispatcher and provided by `roko graph run`; the other is ActCell reusing `TaskDispatcher` with a synthetic task. Then add `act_cell_dispatches_through_provider` with a fake port. Until then, an interim honest step: mark the pass-through `act` and `claude-agent` descriptors as stubs. Today a production `roko graph run` accepts graphs whose `claude-agent` nodes pass their input through without dispatching an agent, for example examples/graphs/task-execution.toml: neither descriptor is `is_stub`, so `validate_for_start` lets them run.
- 2026-10-01 (wk-filer4): The interim step landed through bug-91a34e (48e8f5d7b, on work/gap-cd51b7): the `act` and `claude-agent` descriptors are stubs, ActCell reports `is_stub`, and production starts refuse them. Wiring ActCell to a provider is what remains here; once it dispatches, drop the two `.with_stub(true)` calls in `default_registry` and the `is_stub` override.
