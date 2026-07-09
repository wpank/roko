# STAB_09: Normalize `[[gate]]` vs `[gates]` config schema

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-09`](../ISSUE-TRACKER.md#stab-09)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.09
- Priority: **P0**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`RokoConfig` in `schema.rs` at line 64 has `pub gates: GatesConfig`. The `from_toml()` at
line 171 calls `toml::from_str()` which deserializes `[gates]` table format. The `[[gate]]`
array format (TOML array of tables) is a different key name and is silently ignored by serde.

`roko init` (in `commands/init.rs` at line 130) writes `[[gate]]` format:
```toml
[[gate]]
kind = "shell"
program = "cargo"
```

This format is never read by the runtime.

## Exact Changes

1. In `schema.rs`, add an `extra_gates` field to `RokoConfig`:
   ```rust
   #[serde(default, rename = "gate")]
   pub extra_gates: Vec<LegacyGateEntry>,
   ```
   where `LegacyGateEntry` captures the `[[gate]]` array format.
2. In `from_toml()`, after parsing, merge `extra_gates` into `gates`:
   ```rust
   pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
       let mut config: Self = toml::from_str(s)?;
       if !config.extra_gates.is_empty() {
           if config.gates.enabled.is_empty() {
               // Convert legacy format to new format
               for gate in &config.extra_gates {
                   config.gates.enabled.push(gate.kind.clone());
                   if gate.kind == "shell" {
                       config.gates.shell_gates.push(ShellGateCommand { ... });
                   }
               }
           } else {
               tracing::warn!("Both [[gate]] and [gates] found; preferring [gates]");
           }
       }
       Ok(config)
   }
   ```
3. Add a unit test: parse a TOML string with `[[gate]]` entries, verify they appear in
   `config.gates.enabled`.
4. Add a unit test: parse a TOML string with both formats, verify `[gates]` is preferred
   and a warning is emitted.
5. Add a unit test: parse with only `[gates]`, verify normal behavior unchanged.

## Design Guidance

Use serde's `rename` attribute to map `[[gate]]` to a separate field, then merge in
`from_toml()`. This avoids complex custom deserializer logic. The merge is a one-time
normalization at load time.

## Write Scope

- `crates/roko-core/src/config/schema.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `RokoConfig::from_toml("[[gate]]\nkind = \"shell\"\nprogram = \"cargo\"\n")` produces a config with `gates.enabled` containing the gate
- [ ] Existing `[gates]` format continues to work
- [ ] When both are present, `[gates]` wins with a warning

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Existing `[gates]` format continues to work
- When both are present, `[gates]` wins with a warning
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
