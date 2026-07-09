# E12 — Dead-Code & Legacy Cleanup

**Status:** GAP — no existing plan. Census complete, deletions not yet sequenced.
**Source docs:** `104-DEAD-CODE-AND-FACADE-CENSUS`, `06-WIRING-STATUS`, `03-CRATE-AUDIT`,
`54-PER-CRATE-MIGRATION-CHECKLIST`, `11-DEPENDENCY-GRAPH`
**Depends on:** E05 (gate adaptivity port), E06 (compose unify), E08 (conductor wiring),
plus E01/E04 for the orchestrator island. Deletion tasks are **gated** behind the epics
that first port each module's live value out of the dead island.
**Owning subsystem:** whole-workspace hygiene (`roko-core`, `roko-runtime`, `roko-orchestrator`,
`roko-conductor`, `roko-plugin`, `roko-cli/src/orchestrate.rs`, `roko-index`)
**Task count:** 9 (E12-T01 … E12-T09)

---

## Why this epic matters

Roughly **52K LOC** compiles into the workspace but is dead-by-default or reachable only
through the legacy `orchestrate.rs` path that runner-v2 (`crates/roko-cli/src/runner/`) already
replaced. Dead code inflates build times, hides real call graphs, and lets duplicate
implementations (two safety layers, two HDC impls) drift. This epic **removes** the island —
but only *after* the epics that mine its still-valuable behaviour (E05 gate adaptivity, E06
compose enrichment, E08 conductor supervision) have ported that value into the live engine.
Deleting first would destroy the reference implementations those epics read from.

The epic front-loads the **safe-now, mechanical** cleanups (orphan files, a layering
violation, a test-only feature façade) that carry zero inter-epic risk, then queues the
**gated** big-island removals.

## Verified findings (2026-07-09)

- **Orphan files** not in any `mod` tree — dead the moment they were written:
  `crates/roko-core/src/pulse_bus.rs` (6,286 B) and `crates/roko-core/src/state_hub.rs`
  (18,172 B). Verified: `rg 'pulse_bus|state_hub' crates/roko-core/src/lib.rs` → 0 matches.
- **Layering violation (worst in the graph):** `roko-runtime` (`layer = 1`) depends on
  `roko-gate` (`layer = 3`) — `crates/roko-runtime/Cargo.toml:27`. The *only* non-test src use
  is `crates/roko-runtime/src/effect_driver.rs:21` (`use roko_gate::GateRegistry`);
  `workflow_engine.rs:1327` (`roko_gate::GateService::new()`) is inside the `#[cfg(test)]`
  module. Crucially the **gate contract already lives one layer down**: `GateRunner`,
  `GateConfig`, `GateReport` are imported from `roko_core::foundation`
  (`workflow_engine.rs:14,1141`). So the runtime already programs against the trait — it only
  pulls in the concrete crate for one registry handle.
- **`legacy-runner-v2` feature façade:** `default = ["legacy-runner-v2"]`
  (`crates/roko-cli/Cargo.toml:15,20`) but **zero `src` cfg sites** — every reference is in
  `tests/` (`cost_dedup.rs`, `smoke.rs`, `phase0_wiring.rs`, `common/mod.rs`). The feature
  gates only tests; the runner-v2 code compiles unconditionally. The name is misleading
  (runner-v2 is the *current default working engine*, per P11).
- **`legacy-orchestrate` feature** — the *real* legacy island. `orchestrate.rs` is 23,676 LOC;
  `run.rs` has ~10 `#[cfg(feature = "legacy-orchestrate")]` src sites; the legacy binary is
  `required-features = ["legacy-orchestrate"]` (`Cargo.toml:108`). This is the dead path E05/
  E06/E08 mine from.
- **Duplicate safety layer:** `crates/roko-orchestrator/src/safety/` = 3,387 LOC across 7 files
  (`audit_chain`, `capability_tokens`, `loop_guard`, `permit`, `sandboxing`,
  `taint_propagation`), duplicating `roko-agent/src/safety/` (E04's live target).
- **`#[allow(dead_code)]`** appears in **37 files** across `crates/` — a proxy for accreted
  dead members to prune once callers are gone.
- **`roko-index` reimplements HDC** with no `roko-primitives` dependency
  (`rg 'roko-primitives' crates/roko-index/Cargo.toml` → 0). Duplicate of the canonical HDC.
- **`roko-conductor`** has no `layer` key in `Cargo.toml` (minor: layer-checker can't place it).

---

## Deletion-candidate table

| Module / file | LOC | Category | Blocking dependency (port live value first) | Delete after |
|---|---:|---|---|---|
| `roko-core/src/pulse_bus.rs` | ~0.2K | orphan (no `mod`) | none | **now** (E12-T01) |
| `roko-core/src/state_hub.rs` | ~0.6K | orphan (no `mod`) | none | **now** (E12-T01) |
| `roko-runtime → roko-gate` dep edge | — | layer violation L1→L3 | none (contract already in roko-core) | **now** (E12-T02) |
| `legacy-runner-v2` feature | — | test-only façade | none | **now** (E12-T03) |
| 37× `#[allow(dead_code)]` members | ~? | dead members | callers removed by owning epics | **now**, incremental (E12-T04) |
| `roko-index` HDC reimpl | ~? | duplicate impl | E03 type consolidation must expose primitives HDC | after **E03** (E12-T05) |
| `roko-orchestrator/src/safety/` | 3.4K | duplicate safety | E04 makes roko-agent safety canonical | after **E04** (E12-T06) |
| `roko-orchestrator` (rest) | ~? | superseded DAG/executor | E01 runner-v2 owns execution | after **E01/E08** (E12-T06) |
| `roko-cli/src/orchestrate.rs` | 23.7K | dead legacy engine | E05 (gate adaptivity), E06 (compose enrich), E08 (conductor) | after **E05/E06/E08** (E12-T07) |
| `legacy-orchestrate` feature + `run.rs` legacy path | ~? | legacy binary gate | orchestrate.rs deleted (E12-T07) | after **E12-T07** (E12-T08) |
| `roko-plugin` (facade crate) | ~? | facade | audit real consumers (roko-serve/cli imports) | after audit (E12-T09) |

**Split:** ~4 tasks are **safe to delete/refactor now** (T01–T04); **5 tasks are gated behind
other epics** (T05←E03, T06←E01/E04, T07←E05/E06/E08, T08←T07, T09←audit).

---

## Tasks

Ordered: mechanical orphan deletes → layering fix → façade resolution → dead-member audit →
gated big-island removals.

### E12-T01 — Delete orphan files (mechanical, safe now)
Delete `pulse_bus.rs` + `state_hub.rs`; both are absent from every `mod` tree.

### E12-T02 — Invert `roko-runtime → roko-gate` layering violation (safe now)
Program `effect_driver.rs` against the `GateRunner` trait from `roko_core::foundation`
(already used by `workflow_engine.rs`); inject the concrete `GateService`/`GateRegistry` from
the composition root (`roko-cli`). Drop `roko-gate` from `[dependencies]` → `[dev-dependencies]`
(needed only by the `workflow_engine` test). Result: no L1→L3 edge; `layer_check` clean.

### E12-T03 — Resolve `legacy-runner-v2` feature façade (safe now)
Remove the feature (zero src cfg sites); make the tests it gated compile unconditionally (or
`#[cfg(test)]`). Remove it from `default`.

### E12-T04 — Prune `#[allow(dead_code)]` members (safe now, incremental)
Walk the 37 files; delete members whose callers are already gone; leave allows that guard
items an owning epic will wire. Acceptance is a net reduction, not zero.

### E12-T05 — De-dup `roko-index` HDC onto `roko-primitives` *(gated: after E03)*
Add the `roko-primitives` dependency and delete the local HDC reimplementation.

### E12-T06 — Delete `roko-orchestrator` incl. duplicate `safety/` *(gated: after E01, E04)*
Once runner-v2 (E01) owns execution and roko-agent safety (E04) is canonical, remove the crate
and its 3.4K-LOC duplicate safety layer; drop workspace + dependent Cargo entries.

### E12-T07 — Delete `orchestrate.rs` (23.7K LOC) *(gated: after E05, E06, E08)*
Only after E05 ports gate adaptivity, E06 ports compose enrichment, and E08 ports conductor
supervision out of it. Remove `pub mod orchestrate` + the `PlanRunner` re-export from `lib.rs`.

### E12-T08 — Remove `legacy-orchestrate` feature + `run.rs` legacy path *(gated: after T07)*
Delete the ~10 cfg sites, the `[[bin]]` `required-features`, and the feature definition.

### E12-T09 — Retire `roko-plugin` facade *(gated: after consumer audit)*
Audit `roko-serve`/`roko-cli` imports; if unused, delete the crate and its Cargo edges.

---

## First three tasks — executable TOML

```toml
[meta]
plan = "E12-DEAD-CODE-CLEANUP"
total = 9
done = 0
status = "ready"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────────────
# E12-T01: Delete orphan files not present in any mod tree
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E12-T01"
title = "Delete orphan roko-core files pulse_bus.rs and state_hub.rs"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 5
files = [
    "crates/roko-core/src/pulse_bus.rs",
    "crates/roko-core/src/state_hub.rs",
]
role = "implementer"
depends_on = []
acceptance = "Both orphan files are deleted; no module or use statement anywhere references pulse_bus or state_hub; the workspace still builds."

[task.context]
read_files = [
    { path = "crates/roko-core/src/lib.rs", lines = "1-160", why = "Confirm no `mod pulse_bus` / `mod state_hub` declaration exists — both files are orphans." },
]
symbols = []
anti_patterns = [
    "Do NOT delete any file that IS declared with `mod` — only these two orphans.",
    "Do NOT add a `mod` declaration to 'fix' them; they are dead and must be removed.",
    "If `rg` finds ANY reference to pulse_bus or state_hub outside these files, STOP and report instead of deleting.",
]

[[task.verify]]
phase = "structural"
command = "test ! -e crates/roko-core/src/pulse_bus.rs && test ! -e crates/roko-core/src/state_hub.rs"
fail_msg = "Both orphan files must be deleted."

[[task.verify]]
phase = "structural"
command = "! rg -q 'pulse_bus|state_hub' crates/roko-core/src"
fail_msg = "No reference to pulse_bus or state_hub may remain in roko-core/src."

[[task.verify]]
phase = "compile"
command = "cargo build -p roko-core"
fail_msg = "roko-core must still compile after deleting the orphan files."

# ─────────────────────────────────────────────────────────────────────────────
# E12-T02: Invert the roko-runtime -> roko-gate layering violation
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E12-T02"
title = "Program runtime against GateRunner trait; drop roko-gate from runtime deps"
status = "ready"
tier = "standard"
model_hint = "claude-sonnet-4-5"
max_loc = 120
files = [
    "crates/roko-runtime/Cargo.toml",
    "crates/roko-runtime/src/effect_driver.rs",
    "crates/roko-runtime/src/workflow_engine.rs",
]
role = "implementer"
depends_on = []
acceptance = "roko-runtime (layer 1) no longer has a normal-dependency edge to roko-gate (layer 3); effect_driver programs against roko_core::foundation::{GateRunner,GateRegistry-equivalent}; the concrete GateService is injected, not constructed inside layer 1; scripts/layer_check.rs reports zero layer violations; workspace builds and roko-runtime tests pass."

[task.context]
read_files = [
    { path = "crates/roko-runtime/Cargo.toml", lines = "20-45", why = "roko-gate is at line 27 in [dependencies]; must move to [dev-dependencies] (workflow_engine test needs GateService)." },
    { path = "crates/roko-runtime/src/effect_driver.rs", lines = "1-60", why = "Line 21 `use roko_gate::GateRegistry` is the only non-test src use — replace with a roko_core contract / injected handle." },
    { path = "crates/roko-runtime/src/workflow_engine.rs", lines = "1-20", why = "Lines 14 and 1141 already import GateRunner/GateConfig/GateReport from roko_core::foundation — the trait to program against." },
    { path = "scripts/layer_check.rs", lines = "1-100", why = "Layer checker: from_layer < to_layer is a violation; runtime=1, gate=3." },
]
symbols = [
    "roko_core::foundation::GateRunner",
    "roko_gate::GateRegistry",
    "roko_gate::GateService",
]
anti_patterns = [
    "Do NOT lower roko-gate's layer or raise roko-runtime's layer to silence the checker — fix the dependency direction.",
    "Do NOT construct a concrete GateService/GateRegistry inside roko-runtime src; inject an Arc<dyn GateRunner> (or equivalent trait handle) from the caller.",
    "Do NOT delete the workflow_engine test that uses GateService — keep roko-gate as a [dev-dependencies] entry so tests still build.",
    "Do NOT introduce a new roko-runtime->roko-gate edge anywhere else in the crate.",
]

[[task.verify]]
phase = "structural"
command = "! rg -q '^roko-gate' crates/roko-runtime/Cargo.toml || rg -q 'dev-dependencies' crates/roko-runtime/Cargo.toml"
fail_msg = "roko-gate must not be a normal dependency of roko-runtime (dev-dependencies only)."

[[task.verify]]
phase = "structural"
command = "! rg -q 'use roko_gate::' crates/roko-runtime/src/effect_driver.rs"
fail_msg = "effect_driver.rs must program against the roko_core gate contract, not roko_gate directly."

[[task.verify]]
phase = "compile"
command = "cargo build -p roko-runtime && cargo test -p roko-runtime --no-run"
fail_msg = "roko-runtime must build and its tests must compile after the dependency inversion."

# ─────────────────────────────────────────────────────────────────────────────
# E12-T03: Resolve the legacy-runner-v2 feature facade (gates only tests)
# ─────────────────────────────────────────────────────────────────────────────
[[task]]
id = "E12-T03"
title = "Remove legacy-runner-v2 feature facade; unconditionally compile the tests it gated"
status = "ready"
tier = "standard"
model_hint = "claude-sonnet-4-5"
max_loc = 60
files = [
    "crates/roko-cli/Cargo.toml",
    "crates/roko-cli/tests/cost_dedup.rs",
    "crates/roko-cli/tests/smoke.rs",
    "crates/roko-cli/tests/phase0_wiring.rs",
    "crates/roko-cli/tests/common/mod.rs",
]
role = "implementer"
depends_on = []
acceptance = "The legacy-runner-v2 feature is removed from Cargo.toml (definition and default list); every `#![cfg(feature = \"legacy-runner-v2\")]` / `#[cfg(feature = \"legacy-runner-v2\")]` guard in tests is removed so those tests compile unconditionally under `cargo test`; no `src` file referenced the feature (verified) so no src change is needed; workspace builds and the previously-gated tests now run."

[task.context]
read_files = [
    { path = "crates/roko-cli/Cargo.toml", lines = "13-25", why = "Line 15 default=[\"legacy-runner-v2\"], line 20 defines it; feature gates only tests." },
    { path = "crates/roko-cli/tests/phase0_wiring.rs", lines = "1-20", why = "File-level `#![cfg(feature = \"legacy-runner-v2\")]` guard to remove." },
    { path = "crates/roko-cli/tests/smoke.rs", lines = "195-315", why = "Two `#[cfg(feature = \"legacy-runner-v2\")]` item guards to remove." },
    { path = "crates/roko-cli/tests/common/mod.rs", lines = "255-265", why = "One `#[cfg(feature = \"legacy-runner-v2\")]` guard to remove." },
]
symbols = []
anti_patterns = [
    "Do NOT touch the legacy-orchestrate feature — that is a separate, still-live gate handled by E12-T08.",
    "Do NOT delete the tests themselves; only remove their feature guards so they always compile.",
    "Do NOT leave the feature in `default` while deleting its definition — remove both.",
    "If `rg 'legacy-runner-v2' crates/roko-cli/src` returns ANY match, STOP and report — the census expects zero src sites.",
]

[[task.verify]]
phase = "structural"
command = "! rg -q 'legacy-runner-v2' crates/roko-cli/Cargo.toml"
fail_msg = "The legacy-runner-v2 feature must be fully removed from Cargo.toml."

[[task.verify]]
phase = "structural"
command = "! rg -q 'legacy-runner-v2' crates/roko-cli/tests"
fail_msg = "No test may still reference the removed legacy-runner-v2 feature."

[[task.verify]]
phase = "compile"
command = "cargo test -p roko-cli --no-run"
fail_msg = "roko-cli tests must compile unconditionally after removing the feature guards."
```

---

## Sequencing note

T01–T04 can land immediately and independently. T05–T09 are **hard-gated**: their `depends_on`
must reference the completing task of the owning epic (E03 for T05; E01+E04 for T06; E05+E06+E08
for T07; E12-T07 for T08). Do **not** schedule a gated deletion until the porting epic has
merged and its "live value extracted" acceptance is green — deleting first destroys the
reference implementation the epic reads from.
