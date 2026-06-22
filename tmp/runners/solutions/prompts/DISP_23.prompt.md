# DISP_23: Wire Per-Provider Rate Limits from Config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-23`](../ISSUE-TRACKER.md#disp-23)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.23
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`shared_rate_limiter()` in `openai_compat_backend.rs` uses a `OnceLock` for a single global `ProviderRateLimiter` with 60 RPM default. All `OpenAiCompatLlmBackend` instances share this limiter unless `with_rate_limiter()` is called.

Different providers have different rate limits (Anthropic: 1000 RPM, Cerebras: 30 RPM, OpenRouter: varies by plan). The global 60 RPM unnecessarily throttles high-limit providers and may exceed low-limit ones.

## Exact Changes

1. Add `rate_limit_rpm: Option<u32>` field to `ProviderConfig` in the schema (`crates/roko-core/src/config/schema.rs`)
2. In the OpenAI-compat adapter (`provider/openai_compat.rs`), when creating an `OpenAiCompatLlmBackend`, check `provider_config.rate_limit_rpm`:
   ```rust
   let backend = if let Some(rpm) = provider_config.rate_limit_rpm {
       backend.with_rate_limiter(ProviderRateLimiter::new(rpm))
   } else {
       backend  // uses shared default
   };
   ```
3. Create per-provider `ProviderRateLimiter` instances keyed by provider name (use a `HashMap<String, Arc<ProviderRateLimiter>>` singleton or construct per backend)
4. Document the `rate_limit_rpm` config field in `roko.toml` comments

## Design Guidance

Keep the `shared_rate_limiter()` as a fallback for providers without explicit config. Add per-provider overrides only when config specifies them. This is backward compatible -- existing setups get the same 60 RPM default, users who need different limits can configure them.

## Write Scope

- `crates/roko-agent/src/openai_compat_backend.rs`
- `crates/roko-core/src/config/schema.rs`

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

- [ ] `ProviderConfig` has `rate_limit_rpm` field
- [ ] A provider with `rate_limit_rpm = 1000` gets a 1000 RPM limiter
- [ ] A provider without `rate_limit_rpm` gets the default 60 RPM

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `ProviderConfig` has `rate_limit_rpm` field
- A provider with `rate_limit_rpm = 1000` gets a 1000 RPM limiter
- A provider without `rate_limit_rpm` gets the default 60 RPM
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
