# /search Command Broken — Wrong Perplexity API Request Format

## Problem

`/search hdc` in Zed ACP returns:
```
Searching: hdc
error: search error: http error: http 400: {"error":{"message":"query validation: query is required","type":"invalid_request","code":400}}
```

Every `/search` invocation fails with 400. The query is passed correctly through the
ACP bridge → CLI → Perplexity client chain, but the HTTP request body is wrong.

## Root Cause

**`search.rs` sends a batch format that Perplexity's API doesn't accept.**

### What roko sends (line 149-150 of `search.rs`):
```json
{
  "queries": [
    { "query": "hdc" }
  ]
}
```

### What Perplexity's API expects (per [docs](https://docs.perplexity.ai/api-reference/search-post)):
```json
{
  "query": "hdc"
}
```

The `search_batch` method wraps everything in a `{"queries": [...]}` array. Perplexity's
`POST /search` endpoint expects a flat `{"query": "..."}` at the top level. Since there's
no top-level `query` field, the API returns 400: "query is required".

**The entire "multi-query bundling: up to 5 queries per call" feature described in the
module doc is fabricated** — Perplexity has no batch search endpoint. The tests pass because
they mock the HTTP poster and never hit the real API.

## Full Call Chain (working correctly until HTTP body)

```
Zed ACP: /search hdc
  ↓
bridge_events.rs:3407-3410 — builds CLI args ["research", "search", "hdc"]
  ↓
research.rs:718-765 — SearchQuery { query: "hdc", ... }
  ↓
search.rs:141-163 — search_batch([SearchQuery])
  ↓ ← BUG HERE
POST https://api.perplexity.ai/search  body: {"queries": [{"query": "hdc"}]}
  ↓
Perplexity: 400 "query is required"
```

## Additional Issues Found

### A. Date format wrong

Roko sends: `"2024-01-01"` (ISO 8601)
Perplexity expects: `"01/01/2024"` (MM/DD/YYYY)

`research.rs:742-745`:
```rust
after.format("%Y-%m-%d").to_string()  // WRONG format
```

### B. Recency filter not passed

Perplexity has a dedicated `search_recency_filter` field that accepts `"hour"`, `"day"`,
`"week"`, `"month"`, `"year"`. The `--recency` flag from the CLI is converted to a date
range instead of using this native filter. Simpler and more reliable to just pass through.

### C. Response format mismatch

Roko expects: `{"results": [{"query": "...", "results": [...]}]}`
Perplexity returns: `{"results": [{"url": "...", "title": "...", "snippet": "..."}]}`

The response is a flat array of result objects, not a nested per-query grouping.

## Fix

### Fix 1: Rewrite `search_batch` to send correct format (~30 min)

**File:** `crates/roko-agent/src/perplexity/search.rs`

Replace the batch wrapper with single-query calls:

```rust
/// Execute a single search query.
pub async fn search(&self, q: &SearchQuery) -> Result<SearchResponse, SearchError> {
    let mut body = json!({ "query": q.query });

    if let Some(ref domains) = q.domain_filter {
        body["search_domain_filter"] = json!(domains);
    }
    if let Some((ref after, ref before)) = q.date_range {
        body["search_after_date_filter"] = json!(after);
        body["search_before_date_filter"] = json!(before);
    }
    if let Some(ref region) = q.region {
        body["country"] = json!(region);
    }

    let body_bytes = serde_json::to_vec(&body)
        .map_err(|e| SearchError::Serialize(e.to_string()))?;

    let response_text = self.poster
        .post_json(&self.endpoint(), &self.headers(), &body_bytes, self.timeout_ms)
        .await
        .map_err(|e| SearchError::Http(e.to_string()))?;

    let parsed: Value = serde_json::from_str(&response_text)
        .map_err(|e| SearchError::Parse(format!("malformed response json: {e}")))?;

    if let Some(err) = parsed.get("error") {
        let msg = err.get("message").and_then(Value::as_str)
            .unwrap_or("unknown api error");
        return Err(SearchError::Api(msg.to_string()));
    }

    let results = parsed.get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| SearchError::Parse("response missing 'results' array".into()))?;

    let mut search_results = Vec::new();
    for item in results {
        let result: SearchResult = serde_json::from_value(item.clone())
            .map_err(|e| SearchError::Parse(format!("failed to parse result: {e}")))?;
        search_results.push(result);
    }

    Ok(SearchResponse {
        query: q.query.clone(),
        results: search_results,
    })
}

/// Execute multiple queries sequentially (Perplexity has no batch endpoint).
pub async fn search_batch(
    &self,
    queries: &[SearchQuery],
) -> Result<Vec<SearchResponse>, SearchError> {
    let mut responses = Vec::with_capacity(queries.len());
    for q in queries {
        responses.push(self.search(q).await?);
    }
    Ok(responses)
}
```

### Fix 2: Fix date format (~5 min)

**File:** `crates/roko-cli/src/commands/research.rs:742-745`

```rust
// Change from:
after.format("%Y-%m-%d").to_string()  // "2024-01-01"
now.format("%Y-%m-%d").to_string()

// To:
after.format("%m/%d/%Y").to_string()  // "01/01/2024"
now.format("%m/%d/%Y").to_string()
```

Or better, use the native `search_recency_filter` and drop date math entirely:
```rust
// Add to SearchQuery:
pub recency_filter: Option<String>,  // "hour", "day", "week", "month", "year"

// In query_to_wire:
if let Some(ref recency) = q.recency_filter {
    body["search_recency_filter"] = json!(recency);
}
```

### Fix 3: Fix response parsing (~10 min)

**File:** `crates/roko-agent/src/perplexity/search.rs`

The `SearchResult` struct may need field adjustments. Perplexity returns `snippet` not
`content`. Check `types.rs`:

```rust
// Perplexity returns:
// { "url": "...", "title": "...", "snippet": "...", "date": "..." }
//
// Ensure SearchResult maps correctly, or use serde aliases:
#[derive(Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    #[serde(alias = "snippet")]
    pub content: String,
    pub date: Option<String>,
    pub last_updated: Option<String>,
}
```

### Fix 4: Update tests to match real API format (~15 min)

All existing tests use the fabricated batch format. Replace mock responses with the
actual Perplexity response structure.

### Fix 5: Remove `TooManyQueries` error variant

The `MAX_BATCH_SIZE = 5` limit and `TooManyQueries` error are meaningless since there's
no batch endpoint. Remove or repurpose as a sequential rate-limit guard.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-agent/src/perplexity/search.rs` | Rewrite to use correct single-query format |
| `crates/roko-agent/src/perplexity/types.rs` | Add `#[serde(alias = "snippet")]` to SearchResult |
| `crates/roko-cli/src/commands/research.rs:742-745` | Fix date format to MM/DD/YYYY |
| Tests in `search.rs` | Update mock responses to match real API |

## Priority

**P0** — `/search` is 100% broken. Every invocation fails. This is a core research
capability that blocks knowledge gathering. The fix is straightforward: stop wrapping
the query in a `{"queries": [...]}` array.
