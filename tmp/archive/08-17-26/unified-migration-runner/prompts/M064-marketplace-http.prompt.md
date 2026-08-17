# M064 — Marketplace HTTP Routes

## Objective
Implement marketplace HTTP routes in roko-serve: search, publish, install, and fork via REST API. These routes mirror the CLI functionality from M063 but expose it over HTTP for dashboard integration and remote access. Routes live under `/api/marketplace/`.

## Scope
- Crates: `roko-serve`
- Files: `crates/roko-serve/src/routes/marketplace.rs` (new), `crates/roko-serve/src/routes/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.11
- Spec ref: `tmp/unified/15-MARKETPLACE-AND-SHARING.md`

## Steps
1. Read the existing route structure:
   ```bash
   grep -rn 'fn.*router\|Router::new\|.route(' crates/roko-serve/src/routes/mod.rs | head -20
   ls crates/roko-serve/src/routes/
   ```

2. Implement routes in `crates/roko-serve/src/routes/marketplace.rs`:
   ```rust
   // GET  /api/marketplace/search?q=<query>&protocol=<proto>
   //   -> Vec<CellManifest>
   pub async fn search(query: Query<SearchParams>, state: State<AppState>) -> impl IntoResponse;

   // POST /api/marketplace/publish
   //   Body: multipart (manifest.toml + files)
   //   -> ArtifactInfo { name, version, hash }
   pub async fn publish(state: State<AppState>, body: Multipart) -> impl IntoResponse;

   // POST /api/marketplace/install/{id}
   //   -> InstallResult { name, version, path }
   pub async fn install(Path(id): Path<String>, state: State<AppState>) -> impl IntoResponse;

   // POST /api/marketplace/fork/{id}
   //   Body: { new_name, new_author }
   //   -> ForkResult { name, forked_from }
   pub async fn fork(Path(id): Path<String>, body: Json<ForkRequest>, state: State<AppState>) -> impl IntoResponse;

   // GET  /api/marketplace/list
   //   -> Vec<CellManifest>
   pub async fn list(state: State<AppState>) -> impl IntoResponse;

   // GET  /api/marketplace/{id}
   //   -> CellManifest (detailed)
   pub async fn get_cell(Path(id): Path<String>, state: State<AppState>) -> impl IntoResponse;
   ```

3. Wire into the main router in `crates/roko-serve/src/routes/mod.rs`:
   ```rust
   .nest("/api/marketplace", marketplace::router())
   ```

4. Reuse the marketplace logic from M063 (the CLI functions) -- HTTP handlers are thin wrappers that call the same core functions.

5. Add appropriate error responses (404 for not found, 400 for invalid input, 409 for name collision).

6. Write tests:
   - GET /api/marketplace/list returns JSON array
   - GET /api/marketplace/search?q=classify returns matching Cells
   - POST /api/marketplace/install/{id} with valid ID returns success
   - POST /api/marketplace/fork/{id} with new name returns fork info

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- marketplace
```

## What NOT to do
- Do NOT implement authentication/authorization on marketplace routes -- that is a cross-cutting concern
- Do NOT implement remote artifact hosting -- artifacts are local files
- Do NOT add rate limiting -- not needed for local use
- Do NOT duplicate the marketplace logic -- call the same functions used by the CLI
