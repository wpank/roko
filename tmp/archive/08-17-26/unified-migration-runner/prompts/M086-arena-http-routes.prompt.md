# M086 — Arena and Bounty HTTP Routes

**[BLOCKED:depth]** -- This item depends on M082-M085 (arena types, flywheel, eval, bounty) being implemented first, and `tmp/unified-depth/19-arenas/` depth docs.

## Objective
Implement HTTP routes for arenas and bounties in roko-serve: full CRUD, submission, scoring, leaderboard, and bounty lifecycle endpoints. Routes live under `/api/arenas/` and `/api/bounties/`.

## Scope
- Crates: `roko-serve`
- Files: `crates/roko-serve/src/routes/arenas.rs` (new), `crates/roko-serve/src/routes/bounties.rs` (new), `crates/roko-serve/src/routes/mod.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.5
- Depth docs: `tmp/unified-depth/19-arenas/` (pending)

## Steps
1. Read existing route patterns:
   ```bash
   grep -rn 'Router\|.route(' crates/roko-serve/src/routes/mod.rs | head -15
   ```

2. Implement arena routes:
   ```rust
   // GET  /api/arenas                    -> list all arenas
   // POST /api/arenas                    -> create arena
   // GET  /api/arenas/{id}               -> arena detail
   // POST /api/arenas/{id}/submit        -> submit attempt
   // GET  /api/arenas/{id}/leaderboard   -> leaderboard
   // GET  /api/arenas/{id}/flywheel      -> flywheel state
   ```

3. Implement bounty routes:
   ```rust
   // GET  /api/bounties                  -> list bounties
   // POST /api/bounties                  -> post bounty
   // POST /api/bounties/{id}/claim       -> claim bounty
   // POST /api/bounties/{id}/submit      -> submit result
   // POST /api/bounties/{id}/dispute     -> file dispute
   // GET  /api/bounties/{id}/status      -> bounty status
   ```

4. Write tests: HTTP API supports full arena and bounty lifecycle.

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- arenas
cargo test -p roko-serve -- bounties
```

## What NOT to do
- Do NOT implement routes before the underlying arena/bounty types exist
- Do NOT proceed without depth docs
- Do NOT add authentication -- that is a cross-cutting concern
- Do NOT duplicate arena logic -- HTTP handlers call the same core functions
