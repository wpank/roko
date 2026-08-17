# M018 — Parallelize batch requests in gateway.rs

## Objective
The batch submission endpoint in the inference gateway already uses `BATCH_CONCURRENCY` (set to 8) and `StreamExt::buffer_unordered`, but verify that the implementation is genuinely parallel and not accidentally sequential. If the current implementation is already correct, add an integration test proving parallelism. If it's sequential, fix it.

## Scope
- Crates: `roko-serve`
- Files:
  - `crates/roko-serve/src/routes/gateway.rs` (lines ~475-605, `batch_submit` function)
- Phase ref: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.2
- Audit ref: `tmp/roko-trustworthy/AUDIT.md` §B2

## Steps
1. Read the batch_submit implementation:
   ```bash
   grep -n 'batch_submit\|BATCH_CONCURRENCY\|buffer_unordered\|JoinSet\|join_all\|stream.*buffer' crates/roko-serve/src/routes/gateway.rs
   ```

2. Trace the actual dispatch path within the batch loop. Check if:
   - Each item is spawned as an independent future (correct)
   - Items are awaited sequentially in a for loop (incorrect)
   - `buffer_unordered` or `JoinSet` is used for concurrency (correct)

3. If the implementation is already parallel (uses `buffer_unordered` or `JoinSet`):
   - Verify `BATCH_CONCURRENCY` is respected
   - Add an integration test that submits 5 batch items, each with a mock delay, and asserts total wall time < 2x single-item time

4. If the implementation is sequential:
   - Refactor to use `tokio::task::JoinSet` or `futures::stream::iter(...).buffer_unordered(BATCH_CONCURRENCY)`:
     ```rust
     let results: Vec<_> = stream::iter(body.requests)
         .map(|req| {
             let state = Arc::clone(&state);
             async move { dispatch_single(&state, req).await }
         })
         .buffer_unordered(BATCH_CONCURRENCY)
         .collect()
         .await;
     ```

5. Ensure the `BatchProgress` atomic counter is incremented after each individual item completes (not after all items), so the status endpoint can report incremental progress.

6. Add or update the test:
   ```rust
   #[tokio::test]
   async fn batch_executes_in_parallel() {
       // Submit 5 items, assert wall time shows parallelism
   }
   ```

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- batch
# Verify concurrency constant exists and is reasonable:
grep -n 'BATCH_CONCURRENCY' crates/roko-serve/src/routes/gateway.rs
```

## What NOT to do
- Do NOT remove the concurrency limit — unbounded parallelism can overwhelm LLM providers
- Do NOT change the batch API contract (request/response shapes)
- Do NOT remove the BatchProgress counter — the status endpoint depends on it
- Do NOT add retry logic here — that's a separate concern
