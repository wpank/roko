# Task 06: Missing HTTP Endpoints for Chat Commands

**Priority**: P1
**Crate**: `roko-serve`
**Files**: `crates/roko-serve/src/routes/` (new files or extend existing)

## Problem

The Atelier chat PRD identifies several CLI commands that should be exposed as HTTP endpoints
to enable new slash commands. These endpoints don't exist in roko-serve yet.

From the PRD's "Incremental path forward" section:

| Endpoint | Slash command | CLI equivalent | Status |
|----------|--------------|----------------|--------|
| `POST /api/research/enhance-prd` | (existing) | `roko research enhance-prd` | **Exists** ✓ |
| `POST /api/research/topic` | `/research` | `roko research topic` | **Exists** ✓ |
| `POST /api/neuro/query` | (new) | `roko knowledge query` | **Missing** |
| `POST /api/prd/consolidate` | (new) | `roko prd consolidate` | **Missing** |
| `POST /api/dream/run` | (new) | `roko knowledge dream run` | **Missing** |
| `POST /api/jobs/{id}/assign` | (existing) | n/a | **Exists** ✓ |
| `POST /api/jobs/{id}/submit` | (existing) | n/a | **Exists** ✓ |
| `POST /api/jobs/{id}/evaluate` | (existing) | n/a | **Exists** ✓ |

Three endpoints need to be added. Each one maps directly to existing CLI functionality
that just needs an HTTP wrapper.

## Implementation

### Endpoint 1: POST /api/neuro/query

**Purpose**: Search the durable knowledge store (roko-neuro).

**CLI equivalent**: `cargo run -p roko-cli -- knowledge query "<topic>"`

**Request:**
```json
{
  "query": "string — the search topic",
  "limit": 10,         // optional, default 10
  "min_tier": null      // optional, minimum knowledge tier
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "entry-id",
      "content": "knowledge content text",
      "kind": "fact" | "procedure" | "concept",
      "tier": "T1",
      "relevance": 0.85,
      "created_at": "2026-04-22T10:00:00Z"
    }
  ],
  "total": 42
}
```

**Implementation steps:**

1. Find the knowledge query logic. It's in `crates/roko-neuro/` — look for a query/search
   function. The CLI at `crates/roko-cli/src/` has a `knowledge query` subcommand that
   calls it.

2. Create a new route file `crates/roko-serve/src/routes/neuro.rs` (or add to an existing
   knowledge-related file if one exists).

3. The handler should:
   - Parse request body
   - Call the neuro store's query function (same as CLI does)
   - Serialize results to JSON
   - Return 200 with results

4. Register the route in the router.

**Note**: The neuro store may need to be added to `AppState` if it's not already there.
Check if `AppState` has a knowledge store field. The agent-server has one
(`state.knowledge_store`) — serve may need the same.

### Endpoint 2: POST /api/prd/consolidate

**Purpose**: Scan PRDs for gaps and duplicates.

**CLI equivalent**: `cargo run -p roko-cli -- prd consolidate`

**Request:**
```json
{}
```
(No parameters needed — it scans all PRDs.)

**Response:**
```json
{
  "id": "operation-id"
}
```

This should be a background operation (like plan generation) since it may take time.
Return an operation ID that can be polled via `GET /api/operations/{id}`.

**Implementation steps:**

1. Find the consolidation logic. It's invoked from the CLI's `prd consolidate` subcommand.
   Likely in `crates/roko-cli/src/` — search for `consolidate`.

2. Add the route to `crates/roko-serve/src/routes/prds.rs` (alongside other PRD routes).

3. The handler should:
   - Spawn the consolidation as a background task
   - Register it as an operation in AppState
   - Return 202 Accepted with the operation ID

4. Register the route.

### Endpoint 3: POST /api/dream/run

**Purpose**: Kick off a dream consolidation cycle (offline knowledge consolidation).

**CLI equivalent**: `cargo run -p roko-cli -- knowledge dream run`

**Request:**
```json
{
  "mode": "full" | "quick",   // optional, default "full"
  "workdir": null              // optional, override workdir
}
```

**Response:**
```json
{
  "id": "operation-id"
}
```

Also a background operation.

**Implementation steps:**

1. Find the dream cycle logic in `crates/roko-dreams/`. The CLI has a `knowledge dream run`
   subcommand that invokes it.

2. Create `crates/roko-serve/src/routes/dream.rs` or add to a knowledge routes file.

3. The handler should:
   - Spawn the dream cycle as a background task
   - Register as operation
   - Return 202 with operation ID

4. Register the route.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/routes/neuro.rs` | New file — knowledge query endpoint |
| `crates/roko-serve/src/routes/prds.rs` | Add consolidate endpoint |
| `crates/roko-serve/src/routes/dream.rs` | New file — dream run endpoint |
| `crates/roko-serve/src/lib.rs` | Register new routes |
| `crates/roko-serve/src/state.rs` | May need to add NeuroStore to AppState |
| `crates/roko-serve/Cargo.toml` | Add roko-neuro, roko-dreams deps if not present |

## Verification

### Automated

```bash
cargo build -p roko-serve
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual — neuro query

```bash
cargo run -p roko-cli -- serve &

# Query knowledge store
RESP=$(curl -s -X POST http://127.0.0.1:6677/api/neuro/query \
  -H 'Content-Type: application/json' \
  -d '{"query": "agent dispatch", "limit": 5}')

echo "$RESP" | jq .
# Should return { results: [...], total: N }
# Results may be empty if knowledge store is empty — that's OK
# But the endpoint must not 404 or 500

echo "$RESP" | jq 'has("results", "total")'
# MUST be true
```

### Manual — PRD consolidate

```bash
RESP=$(curl -s -X POST http://127.0.0.1:6677/api/prd/consolidate \
  -H 'Content-Type: application/json' \
  -d '{}')

echo "$RESP" | jq .
# Should return { id: "operation-id" }

# Poll operation
OP_ID=$(echo "$RESP" | jq -r '.id')
curl -s "http://127.0.0.1:6677/api/operations/${OP_ID}" | jq .
# Should show operation status
```

### Manual — dream run

```bash
RESP=$(curl -s -X POST http://127.0.0.1:6677/api/dream/run \
  -H 'Content-Type: application/json' \
  -d '{}')

echo "$RESP" | jq .
# Should return { id: "operation-id" }
```

## Acceptance criteria

- [ ] `POST /api/neuro/query` accepts `{"query": "..."}` and returns `{results: [], total: N}`
- [ ] `POST /api/prd/consolidate` returns `{id: "op-id"}` and runs in background
- [ ] `POST /api/dream/run` returns `{id: "op-id"}` and runs in background
- [ ] All three endpoints return proper HTTP status codes (200, 202, 4xx for bad input)
- [ ] Endpoints are registered in the router and show in `GET /api/openapi.json` if available
- [ ] All existing tests still pass
- [ ] No new clippy warnings
