# S-cog-1: Inventory pheromone + daimon callers; produce deletion plan

## Task
Audit document. Inventory every caller of `pheromone::*` and `daimon::*` modules. Output to `logs/S-cog-1-inventory.md`. This audit feeds S-cog-2..5 (FailureTracker replacement, daimon deletion, pheromones deletion).

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/31-cognitive-layer-cleanup.md` § CL-1.

## Why
The audit recommends deleting pheromones (~68K LOC) entirely and replacing daimon (~40K) with a `FailureTracker` (~2K). Before deletion, inventory what currently uses each.

## Exact changes

This batch produces an audit doc, not code changes.

### 1. Locate the crates / modules

```bash
ls crates/roko-cognitive/ crates/roko-daimon/ 2>&1
rg -l 'pheromone|Pheromone' crates/ -g '*.rs'
rg -l 'daimon|Daimon|DaemonPolicy|AffectPolicy' crates/ -g '*.rs'
```

Note where pheromones and daimon live (separate crates? modules within `roko-cognitive`?).

### 2. Map external callers

For each external file:

```bash
rg 'use roko_(cognitive|daimon)|roko_(cognitive|daimon)::' crates/ -g '*.rs' \
  | rg -v 'crates/roko-cognitive/|crates/roko-daimon/'
```

For each call site, note:

- File / line
- The symbol used (`Pheromone`, `DaimonPolicy`, `FatigueDetector`, etc.)
- What the caller does with it (read state, apply policy, ...)

### 3. Categorize daimon callers

For each daimon caller, decide whether the use case maps to:

- **`FailureTracker`** (most common): tracks recent failures, suggests retry strategy.
- **`AffectPolicy`** (D-track delivered some of this): leave as-is or migrate.
- **Genuinely needs daimon**: rare; document.

### 4. Categorize pheromone callers

Each pheromone use case is most likely **deletable** (per audit). For each, note whether the caller needs replacement and what the replacement looks like.

### 5. Output to `tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md`

```markdown
# S-cog-1: Pheromone + daimon caller inventory

Generated: 2026-05-01

## Pheromone callers

| Caller | Symbol | Use case | Replacement |
|---|---|---|---|
| `crates/roko-cli/src/orchestrate.rs:XXXX` | `Pheromone::trail_strength` | Modulating retry decision | Delete; use `FailureTracker::consecutive_count` |
| ... | ... | ... | ... |

## Daimon callers

| Caller | Symbol | Use case | Replacement | New API |
|---|---|---|---|---|
| `crates/roko-cli/src/orchestrate.rs:XXXX` | `DaimonState::record` | Record agent failure | `FailureTracker::record(FailureRecord {...})` | `roko_learn::failure_tracker` |
| `crates/roko-cli/src/runner/event_loop.rs:XXXX` | `FatigueDetector` | Adjust prompt | (kept; per F04 already wired) | n/a |

## Recommended deletion plan

1. **S-cog-2**: implement `FailureTracker` in `roko-learn`.
2. **S-cog-3**: migrate the daimon callers above (one commit per caller).
3. **S-cog-4**: delete `roko-daimon` crate; remove from workspace `Cargo.toml`.
4. **S-cog-5**: delete `roko-cognitive` (or pheromone modules within it); remove from workspace `Cargo.toml`.

Estimated LOC removal:
- Daimon: ~40K (after FailureTracker replacement of ~2K)
- Pheromones: ~68K
- Total: ~106K
```

## Write Scope
- `tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md` (new)

## Read-Only Context
- All `crates/`

## Verify

```bash
ls tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md

wc -l tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md
# Should be > 30 lines (real inventory)
```

## Do NOT

- Do NOT touch any source file.
- Do NOT bundle with other S-cog batches.
- Do NOT skip the daimon "kept" rows — F04's `FatigueDetector` may legitimately stay.
