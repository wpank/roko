# DISP_13: Migrate web_search.rs to Injected Provider Config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-13`](../ISSUE-TRACKER.md#disp-13)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.13
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The web search builtin tool at `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/web_search.rs:332` reads `PERPLEXITY_API_KEY` directly:
```rust
let api_key = match std::env::var("PERPLEXITY_API_KEY") {
    Ok(k) if !k.is_empty() => k,
    _ => { return ToolResult::Err(...) }
};
```

There is already a TODO comment at line 330: `// TODO(gateway): wire ModelCaller from runtime ToolContext.`

The `ToolContext` passed to tool handlers carries metadata but currently lacks a reference to provider configuration or a pre-configured HTTP client.

## Exact Changes

1. Add an `api_keys: HashMap<String, String>` or `provider_config: Option<Arc<RokoConfig>>` field to the tool execution context (this may be in `ToolContext`, `ToolDispatchContext`, or the equivalent struct in `roko-std`)
2. If adding to `ToolContext` would cause too many downstream changes, add a simpler `perplexity_api_key: Option<String>` field that is populated from provider config at dispatch time
3. In the web_search handler, read the API key from context instead of env:
   ```rust
   let api_key = ctx.api_key("perplexity")
       .ok_or_else(|| ToolError::Other("Perplexity API key not configured".into()))?;
   ```
4. Populate the key from `RokoConfig::effective_providers()["perplexity"].resolve_api_key()` at the point where `ToolContext` is constructed
5. Remove the `std::env::var("PERPLEXITY_API_KEY")` call
6. Remove the `warn_direct_api_key_path_once()` helper since the direct path no longer exists

## Design Guidance

Ideally the web_search tool would use `create_agent_for_model()` to get a Perplexity `Agent` and call it through the provider system. But the Perplexity adapter's `run()` returns agent results, not search results with citations. The current implementation does raw HTTP POST to the Perplexity chat/completions endpoint. A compromise is to pass the API key through `ToolContext` (injected from provider config) rather than reading it from env. Full provider integration is a follow-up.

## Write Scope

- `crates/roko-std/src/tool/builtin/web_search.rs`

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

- [ ] `grep -n 'std::env::var.*PERPLEXITY_API_KEY' crates/roko-std/src/` returns zero results (outside tests)
- [ ] Web search works when API key is configured in roko.toml providers section

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'std::env::var.*PERPLEXITY_API_KEY' crates/roko-std/src/` returns zero results (outside tests)
- Web search works when API key is configured in roko.toml providers section
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
