# DISP_20: Replace Boolean Flags with ProviderQuirks Struct

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-20`](../ISSUE-TRACKER.md#disp-20)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.20
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`OpenAiCompatLlmBackend` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` has three boolean fields for provider-specific workarounds:
- `skip_session_fields: bool` (line 54) -- for Cerebras (rejects unknown fields)
- `disable_parallel_tool_calls: bool` (line 58) -- for small models
- `normalize_tool_call_content: bool` (line 62) -- for providers that reject empty content with tool_calls

These flags multiply with each new strict provider. The Kimi K2.5 documentation in the module header lists 7 additional constraints not yet encoded.

## Exact Changes

1. Define a `ProviderQuirks` struct in a new file `crates/roko-agent/src/provider_quirks.rs`:
   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct ProviderQuirks {
       pub skip_session_fields: bool,
       pub disable_parallel_tool_calls: bool,
       pub normalize_tool_call_content: bool,
       pub max_tools: Option<usize>,
       pub strict_schemas: bool,
       pub few_shot_tool_examples: bool,
       pub thinking_locks_temperature: bool,
       pub reasoning_in_history: bool,
   }

   impl ProviderQuirks {
       pub fn cerebras() -> Self { Self { skip_session_fields: true, disable_parallel_tool_calls: true, normalize_tool_call_content: true, strict_schemas: true, few_shot_tool_examples: true, ..Self::default() } }
       pub fn kimi() -> Self { Self { thinking_locks_temperature: true, reasoning_in_history: true, ..Self::default() } }
   }
   ```
2. Replace the 3 boolean fields in `OpenAiCompatLlmBackend` with `quirks: ProviderQuirks`
3. Replace the 3 `with_*` builder methods with `with_quirks(quirks: ProviderQuirks)`
4. Update the Cerebras adapter to use `ProviderQuirks::cerebras()`
5. Update the OpenAI-compat adapter to use `ProviderQuirks::default()` (no quirks)
6. Update all callers of the removed builders

## Design Guidance

`ProviderQuirks` is a value type -- `Clone`, `Debug`, `Default`. Named constructors (`cerebras()`, `kimi()`) make it easy to add new provider profiles. Adding a new quirk is one field addition + updating the relevant constructor. No new boolean flags needed on the backend.

The quirks struct can later be loaded from `ModelProfile` or `ProviderConfig` in roko.toml, allowing users to configure provider compatibility without code changes.

## Write Scope

- `crates/roko-agent/src/openai_compat_backend.rs`
- `crates/roko-agent/src/provider/cerebras.rs`
- `crates/roko-agent/src/provider/openai_compat.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -n 'skip_session_fields\|disable_parallel_tool_calls\|normalize_tool_call_content' crates/roko-agent/src/openai_compat_backend.rs` shows only the `ProviderQuirks` struct definition, not loose boolean fields
- [ ] Cerebras adapter uses `ProviderQuirks::cerebras()`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'skip_session_fields\|disable_parallel_tool_calls\|normalize_tool_call_content' crates/roko-agent/src/openai_compat_backend.rs` shows only the `ProviderQuirks` struct definition, not loose boolean fields
- Cerebras adapter uses `ProviderQuirks::cerebras()`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
