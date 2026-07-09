# PAA_03: Wire GET /api/share/{token} and GET /api/shared/{token} receipt endpoints

## Task
Implement the run receipt sharing endpoints.

## Runner Context
Runner PAA, batch 3 of 3. No dependencies.

## Problem
`Share.tsx:28` calls `GET /api/share/{token}` — public receipt view.
`ShareView.tsx:120` calls `GET /api/shared/{token}` — dashboard share view (uses `useApi` with NO fallback).

Expected shapes:
```ts
// /api/share/{token}
{ prompt?: string; model?: string; cost_usd?: number; gate_results?: ...; created_at?: string; }

// /api/shared/{token}
{ id: string; agent: string; role: string; prompt: string; success: boolean; gates: [string, boolean][]; output?: string; cost_usd?: number; ... }
```

## Exact Changes

### Step 1: Check if share.rs exists

Search `crates/roko-serve/src/routes/` for `share.rs`.

### Step 2: Implement receipt lookup

```rust
async fn get_shared_receipt(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Look up receipt by token in .roko/shared/ or state.shared_receipts
    let receipt = state.shared_receipts.get(&token)
        .ok_or_else(|| ApiError::not_found("receipt not found"))?;
    Ok(Json(receipt))
}
```

### Step 3: Register both paths

```rust
.route("/share/{token}", get(get_shared_receipt))
.route("/shared/{token}", get(get_shared_receipt))  // same handler
```

### Step 4: Implement receipt creation (if not exists)

The `--share` flag in `roko run` should create a receipt and return a share token. Check if this exists and wire it.

## Write Scope
- `crates/roko-serve/src/routes/share.rs`


## Verify
```bash
cargo build -p roko-serve 2>&1 | head -30
cargo test -p roko-serve 2>&1 | tail -20
```
## Acceptance Criteria
- `GET /api/share/{token}` returns receipt data
- `GET /api/shared/{token}` returns the same data
- Missing token returns 404 with "receipt not found"
- Response shapes match what Share.tsx and ShareView.tsx expect

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
