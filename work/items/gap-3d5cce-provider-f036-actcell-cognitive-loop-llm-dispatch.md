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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
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
