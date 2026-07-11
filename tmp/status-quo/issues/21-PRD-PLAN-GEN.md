# PRD and Plan Generation Issues

## Critical

### Perplexity `/search` endpoint doesn't exist — 100% broken
- `perplexity/search.rs:103-106`: POSTs to `{base_url}/search`. Perplexity API only exposes `/chat/completions`. Every request → HTTP 422.
- `commands/research.rs:761-765`: `roko research search` always fails.

### No cycle detection in runner path
- `runner/plan_loader.rs:67-80`: Only calls `validate_against_schema()`. Does NOT include cycle detection.
- `validate_structure()` (with `detect_cycle_nodes`) only called from legacy `orchestrate.rs`.
- A plan with dependency cycles will deadlock silently until plan timeout.

## High

### Generated plan validation doesn't detect cycles
- `prd.rs:1276-1283`: `validate_and_fix_generated_plan` validates syntax/fields only.
- Post-write `plan_validate::validate_plans_dir_with_workdir` at `prd.rs:1532-1545` calls `detect_cycle_nodes` — but only as a WARNING, doesn't block write or execution.

### HTTP publish race condition
- `routes/prds.rs:549-567`: `tokio::fs::rename` without workspace lock. Two concurrent HTTP publish requests can race.

### Auto-plan double-trigger
- `routes/prds.rs:256-262`: Spawns both `spawn_prd_publish_subscriber` (live bus) and `follow_prd_published_audit` (audit log poller). Same slug queued twice → duplicate plan-generation agents.

## Medium

### `prd plan` accepts drafts
- `commands/prd.rs:745-782`: `find_prd` searches both `published/` and `drafts/`. Incomplete draft can enter plan-execute loop.

### `regenerate_old_format_plan` uses unrestricted tools
- `prd.rs:312-321`: `allowed_tools: None` (full access). New generation correctly sandboxes to `Read,Grep,Glob`.

### Plan loader skips invalid plans silently in batch mode
- `runner/plan_loader.rs:127-133`: `tracing::warn` only. If invalid plan contains tasks that others `depends_on_plan`, cross-plan ordering breaks silently.

### File paths in generated plans never verified to exist
- `prd.rs:2181`: `validate_and_fix_generated_plan` doesn't check that listed paths are real. Agents reference nonexistent files.
