# 105 — HTTP API Response Envelope and Pagination Inconsistencies

**Priority**: P2 — API quality; inconsistent envelopes break client code that reads list endpoints
**Size**: M (2-3 days)
**Crates**: `roko-serve` (`/Users/will/dev/nunchi/roko/roko/crates/roko-serve/`)
**Depends on**: None

---

## Background

The roko HTTP control plane (`roko-serve`) exposes roughly 317 routes registered in
`crates/roko-serve/src/routes/`. These routes were built incrementally across many epics
without a consistent API design standard. The result is that list endpoints return data in at
least five different JSON shapes, some endpoints return no pagination metadata at all, one
endpoint returns HTTP 200 with an empty array when the resource does not exist (masking
errors), and the rate limit middleware never sets `X-RateLimit-*` headers on successful
responses even though the limits are enforced.

These inconsistencies make the API hard for external clients (dashboards, scripts, integrations)
to consume reliably. A client reading `/api/managed-agents` gets a bare JSON array, while a
client reading `/api/arenas` gets a nested object with a `"source"` key. A client that wants to
paginate `/api/managed-agents` has no query parameters to do so. A client hitting the rate limit
has no way to know what the limit is before being rejected with a 429.

The four problems are independent — each can be fixed without touching the others — but they all
live in the same `crates/roko-serve/src/routes/` directory.

## Current State

1. **Five different list response shapes**, verified by reading source:
   - `routes/agents.rs` line 186: `Json(Value::Array(items))` — bare JSON array, no wrapper
   - `routes/aggregator.rs` line 211: `json!(PaginatedResponse::new(items, total, 0, total))` — `PaginatedResponse` with hardcoded `offset=0` and `limit=total` (meaningless pagination metadata)
   - `routes/auth.rs` line 1029: `GET /api/api-keys` returns keys wrapped in `{"keys": [...]}` (found at auth.rs list function)
   - `routes/feeds.rs` line 161: `Json(FeedListResponse { feeds, total })` — custom struct with two fields
   - `routes/arenas.rs` lines 431-437: `json!({"source": "local_durable", "arenas": arenas, "total": total, "offset": offset, "limit": limit})` — ad-hoc inline object

2. **`PaginatedResponse` already exists** in `routes/aggregator.rs` at lines 66-80:
   ```rust
   struct PaginatedResponse<T: Serialize> {
       data: Vec<T>,
       total: usize,
       offset: usize,
       limit: usize,
       has_more: bool,
   }
   ```
   This type is already correct — it just needs to be extracted into `routes/mod.rs` and used
   universally.

3. **Hardcoded pagination** at `routes/aggregator.rs` lines 211 and 388 and 422:
   ```rust
   let total = items.len();
   let body = json!(PaginatedResponse::new(items, total, 0, total));
   ```
   `offset` is always 0 and `limit` is always `total`, so these endpoints return all items
   regardless of what the client requests. No `limit`/`offset` query params are parsed.

4. **Silent 200 for missing episodes file** at `routes/agents.rs` line 1150:
   ```rust
   Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Json(json!([]))),
   ```
   The `agent_episodes` function (not `proxy_agent_logs` as previously stated) returns an empty
   200 array when `episodes.jsonl` is missing, making it impossible for clients to distinguish
   "agent has no episodes" from "file does not exist."

5. **No `X-RateLimit-*` headers** in `routes/mod.rs` at lines 189-228. The middleware enforces
   100 req/s global (`DEFAULT_GLOBAL_RATE_PER_SEC = 100`) and 30 req/s per-key
   (`DEFAULT_PER_KEY_RATE_PER_SEC = 30`) using governor, but neither `rate_limit_middleware` nor
   `keyed_rate_limit_middleware` sets any headers on successful responses. Clients cannot
   proactively throttle.

## Implementation Plan

### Step 1: Extract `PaginatedResponse` into a shared module

In `crates/roko-serve/src/routes/mod.rs`, add a public `ListResponse` type that all list
endpoints will use. The existing `PaginatedResponse` in `aggregator.rs` is the right shape; just
make it public and canonical.

```rust
// Add to crates/roko-serve/src/routes/mod.rs (before the route builder function):
#[derive(Debug, Serialize)]
pub struct ListResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

impl<T: Serialize> ListResponse<T> {
    pub fn new(items: Vec<T>, total: usize, offset: usize, limit: usize) -> Self {
        let has_more = offset + items.len() < total;
        Self { data: items, total, offset, limit, has_more }
    }

    pub fn all(items: Vec<T>) -> Self {
        let n = items.len();
        Self::new(items, n, 0, n)
    }
}
```

### Step 2: Add a shared `PaginationQuery` extractor

Also in `crates/roko-serve/src/routes/mod.rs`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct PaginationQuery {
    #[serde(default)]
    pub offset: usize,
    pub limit: Option<usize>,
}

impl PaginationQuery {
    pub fn effective_limit(&self) -> usize {
        self.limit.unwrap_or(50).min(200)
    }
}
```

### Step 3: Migrate `GET /api/managed-agents` in `routes/agents.rs`

Change the function signature at line 111 and the return at line 186:

```rust
// Before (line 111):
async fn list_managed_agents(State(state): State<Arc<AppState>>) -> Json<Value> {
    // ...
    Json(Value::Array(items))  // line 186
}

// After:
async fn list_managed_agents(
    State(state): State<Arc<AppState>>,
    Query(page): Query<PaginationQuery>,
) -> Json<ListResponse<Value>> {
    // ... build items vec as before ...
    let total = items.len();
    let offset = page.offset;
    let limit = page.effective_limit();
    let paged: Vec<Value> = items.into_iter().skip(offset).take(limit).collect();
    Json(ListResponse::new(paged, total, offset, limit))
}
```

### Step 4: Migrate `GET /api/api-keys` in `routes/auth.rs`

Find the `list_api_keys` handler (registered at line 879 as `get(list_api_keys)`). Change the
response to use `ListResponse<Value>` instead of the current custom `{"keys": [...]}` wrapper.

### Step 5: Migrate `routes/aggregator.rs` pagination stubs

At lines 211, 388, and 422, replace:
```rust
let total = items.len();
let body = json!(PaginatedResponse::new(items, total, 0, total));
```
With actual pagination using the query params. Add `Query(page): Query<PaginationQuery>` to the
handler signature of each affected function, then apply `skip(page.offset).take(limit)`.

### Step 6: Migrate `routes/feeds.rs`

Change `list_feeds` (line 127) to return `ListResponse<FeedView>` instead of `FeedListResponse`.
Delete the private `FeedListResponse` struct if nothing else uses it (check with `grep -n
FeedListResponse crates/roko-serve/src/`). Update the existing tests in `feeds.rs` (lines 357,
378, 694, 745) to deserialize `ListResponse<FeedView>` instead of `FeedListResponse`.

### Step 7: Fix `agent_episodes` 404 in `routes/agents.rs`

At line 1150, change:
```rust
// Before:
Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Json(json!([]))),

// After:
Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
    // Return 404 only when the agent itself doesn't exist; return empty array
    // when the episodes file simply hasn't been created yet (no episodes yet).
    // The file is created lazily on first episode write, so NotFound means
    // no episodes have been recorded — return empty array is correct here.
    // If you need strict agent-exists check, verify against supervisor list first.
    return Ok(Json(json!([])));
}
```
**Note:** After re-reading the code, the `NotFound` at line 1150 is for `episodes.jsonl` itself,
not for the agent. The agent check happens earlier via `state.discovered_agent(&id)` at line
1188. The current 200/empty behavior is actually correct semantics for "no episodes yet." The
real gap is the `proxy_agent_logs` function (line 1182) which correctly returns a 404 when the
agent is not discovered. No change needed here — remove this item from scope, or add a
regression test confirming the existing behavior is intentional.

### Step 8: Add `X-RateLimit-Limit` headers in `routes/mod.rs`

In `rate_limit_middleware` (line 189), pass through the response and insert the header:

```rust
pub(crate) async fn rate_limit_middleware(
    State(limiter): State<Arc<GlobalRateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if limiter.check().is_err() {
        return Err(ApiError { /* existing 429 */ });
    }
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_static("100"),
    );
    Ok(response)
}
```

Do the same for `keyed_rate_limit_middleware` with the value `"30"`. Add
`use axum::http::HeaderValue;` if not already imported.

### Step 9: Update or add tests

The existing route tests in `feeds.rs` (line 357 onward) already deserialize response bodies. Update the `FeedListResponse` deserialization to `ListResponse`. Add one new test per migrated endpoint that:
- Calls with `?offset=1&limit=2`
- Asserts `data.len() <= 2` and `offset == 1` in the response

## Acceptance Criteria

1. All five list endpoints (`managed-agents`, `api-keys`, aggregator predictions/sessions, aggregator predictions/claims, feeds) return JSON in the shape `{"data": [...], "total": N, "offset": N, "limit": N, "has_more": bool}`.
2. All five endpoints accept `?offset=N&limit=N` query parameters and respect them.
3. `PaginatedResponse::new(items, total, 0, total)` with hardcoded offset/limit does not exist anywhere in the codebase.
4. `X-RateLimit-Limit: 100` header is present on all successful responses passing through the global rate limiter.
5. All existing route handler tests pass after the migration.
6. One new test per migrated endpoint verifies `offset` and `limit` query params are respected.

## Verification Checklist

- [ ] `grep -n 'Value::Array(items)' crates/roko-serve/src/routes/agents.rs` returns no results
- [ ] `grep -n 'PaginatedResponse::new.*0,.*total' crates/roko-serve/src/routes/aggregator.rs` returns no results
- [ ] `curl -s http://localhost:6677/api/managed-agents | jq 'keys'` returns `["data","has_more","limit","offset","total"]`
- [ ] `curl -s 'http://localhost:6677/api/managed-agents?offset=0&limit=2' | jq '.limit'` returns `2`
- [ ] `cargo test -p roko-serve` passes
- [ ] `curl -sI http://localhost:6677/api/managed-agents | grep X-RateLimit` shows `X-RateLimit-Limit: 100`

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-serve/src/routes/mod.rs` | Add `ListResponse<T>` and `PaginationQuery`; add `X-RateLimit-Limit` header to both rate limit middlewares |
| `crates/roko-serve/src/routes/agents.rs` | Change `list_managed_agents` (line 111) to use `ListResponse`, add pagination query params |
| `crates/roko-serve/src/routes/aggregator.rs` | Replace hardcoded `PaginatedResponse::new(items, total, 0, total)` at lines 211, 388, 422 with real pagination; add `PaginationQuery` to handler signatures |
| `crates/roko-serve/src/routes/auth.rs` | Change `list_api_keys` to return `ListResponse<Value>` |
| `crates/roko-serve/src/routes/feeds.rs` | Change `list_feeds` to return `ListResponse<FeedView>`; delete `FeedListResponse`; update tests at lines 357-745 |
