# M016 — Audit ConnectorRegistry and FeedRegistry

## Objective
The `ConnectorRegistry` in `crates/roko-core/src/connector.rs` and any FeedRegistry in `crates/roko-runtime/src/` are built but effectively empty at runtime. Audit these registries: if the unified Connect protocol (Phase 1, §1.12) supersedes them, mark them deprecated with `#[deprecated]` and add doc comments pointing to the future replacement. If they serve a purpose the Connect trait won't cover, wire them into the runtime.

## Scope
- Crates: `roko-core`, `roko-runtime`
- Files:
  - `crates/roko-core/src/connector.rs` (ConnectorRegistry, ConnectorConfig, etc.)
  - `crates/roko-runtime/src/` (scan for FeedRegistry or feed-related modules)
  - `crates/roko-core/src/feed.rs` (if present)
- Phase ref: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.1
- Audit ref: `tmp/roko-trustworthy/AUDIT.md` §A3
- Future ref: `tmp/unified/12-CONNECTIVITY.md` §1-3 (Connect protocol)

## Steps
1. Inventory existing registry code:
   ```bash
   grep -rn 'ConnectorRegistry\|FeedRegistry\|connector_registry\|feed_registry' crates/ --include='*.rs' | grep -v target/
   grep -rn 'pub struct.*Registry' crates/roko-core/src/ crates/roko-runtime/src/ --include='*.rs'
   ```

2. Check if `ConnectorRegistry` is used anywhere at runtime:
   ```bash
   grep -rn 'ConnectorRegistry' crates/ --include='*.rs' | grep -v target/ | grep -v 'connector.rs'
   ```

3. Read the Connect protocol spec:
   ```bash
   head -60 tmp/unified/12-CONNECTIVITY.md
   ```

4. Decision matrix:
   - If the registry has callers → wire them properly, ensure health checks work
   - If the registry has no callers and Connect trait will supersede → mark `#[deprecated(since = "0.1.0", note = "Use the Connect trait (Phase 1 §1.12) instead")]`
   - If Feed-related types exist → same analysis

5. For deprecated types, add doc comments explaining the migration path:
   ```rust
   /// **Deprecated**: Will be replaced by the `Connect` trait in Phase 1 (§1.12).
   /// See `tmp/unified/12-CONNECTIVITY.md` for the replacement design.
   #[deprecated(since = "0.1.0", note = "Use the Connect trait instead")]
   ```

6. Remove any dead imports referencing these types across the workspace:
   ```bash
   grep -rn 'use.*ConnectorRegistry\|use.*FeedRegistry' crates/ --include='*.rs' | grep -v target/
   ```

7. Ensure clippy is clean after the audit:
   ```bash
   cargo clippy --workspace --no-deps -- -D warnings 2>&1 | grep -i 'connector\|feed\|registry'
   ```

## Verification
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
# Confirm no unused imports:
cargo clippy --workspace --no-deps 2>&1 | grep -c 'unused import'
```

## What NOT to do
- Do NOT delete the types — deprecate them so existing code keeps compiling
- Do NOT implement the Connect trait yet — that's M037
- Do NOT wire empty registries into hot paths just to satisfy the audit — if they have no callers, deprecation is the correct action
- Do NOT modify `tmp/unified/` spec files
