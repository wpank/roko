# 01 — Finish the mirage-rs Extraction

> **Source plans**: `tmp/ux/02-mirage-extraction.md`, `tmp/ux/05-build-phases.md`
> Phase 3.
>
> **Status as of 2026-05-01**: Phase 1 + Phase 2 mostly shipped. The `chain`
> and `dashboard-api` Cargo features exist; pure-EVM mirage builds. The
> `legacy-api` flag named in the original plan was actually shipped as
> `dashboard-api`, and `chain` enables it transitively. **Phase 3 cleanup has
> not started.**
>
> **Effort**: 3-5 days.
>
> **Risk**: Medium. Removing routes is irreversible mid-sprint; deprecation
> windows must be honoured.

---

## What this plan accomplishes

Drive `apps/mirage-rs` from "EVM substrate **plus** dashboard backend" to
"EVM substrate **only**". After this track:

- `apps/mirage-rs/src/http_api/` is gone (all 9 files).
- `apps/mirage-rs/src/chain/` (HDC index, InsightEntry state machine,
  pheromone field, agent registry shadow) is gone.
- `cargo.toml` features collapse to `binary`, `library`, `sim-gas`. The
  `chain`, `dashboard-api`, and `roko` features are deleted.
- The bin entry point only exposes JSON-RPC, scenarios, and replay.
- Workspace LOC drops by ~4,500 (matches the original Phase 3 estimate).

## Why now

Phase 1 + Phase 2 are real: the aggregator (`crates/roko-serve/src/routes/aggregator.rs`)
and the per-agent server (`crates/roko-agent-server/`) are both in production
paths. The dashboard URL swap (track `03`) closes the last consumer of mirage's
REST surface. After that, `apps/mirage-rs/src/http_api/` is dead code and a
maintenance burden.

## Sequencing precondition

**Do not start this track until track `03` is complete and validated.**

After `03` lands:
- The Kauri/nunchi-dashboard at sibling repo `nunchi-dashboard` reads its
  data from `roko-serve` aggregator.
- `demo/demo-app/` already does this (verified in `serve-url.ts` —
  `SERVE_URL` is `:6677`).
- No production consumer remains on mirage's `/api/*`.

`02` (knowledge + pheromone backends) **also** must be functionally complete,
otherwise removing the mirage equivalents loses data the aggregator was
relying on.

---

## Required reading

```
apps/mirage-rs/Cargo.toml                         (features section)
apps/mirage-rs/src/lib.rs                         (chain feature gate)
apps/mirage-rs/src/main.rs                        (CLI flags --enable-hdc etc)
apps/mirage-rs/src/http_api/mod.rs                (route map)
apps/mirage-rs/src/http_api/{agent,knowledge,pheromone,task,topology,prediction,skills,isfr,ws}.rs
apps/mirage-rs/src/chain/                         (ChainContext + state machines)
apps/mirage-rs/src/roko_bridge/                   (Gate/Substrate impls behind `roko` feature)
crates/roko-serve/src/routes/aggregator.rs        (the new home of /api/*)
crates/roko-serve/src/state.rs                    (DiscoveredAgent, AppState)
tmp/ux/02-mirage-extraction.md                    (the original plan)
```

Read `git log --oneline -20 -- apps/mirage-rs/src/http_api/` to see whether
any route is touched in flight; if so, coordinate with that author before
deleting.

---

## Deliverables

1. **`Cargo.toml` reshape**

   ```toml
   [features]
   # Phase 3 final. Pure EVM substrate.
   default = ["binary"]
   binary = []
   library = []
   sim-gas = []
   ```

   Remove `chain`, `dashboard-api`, `roko`. Remove the optional
   `roko-primitives`, `roko-core`, `async-trait` deps.

2. **Delete**:

   ```
   apps/mirage-rs/src/http_api/                     (whole directory)
   apps/mirage-rs/src/chain/                        (whole directory)
   apps/mirage-rs/src/roko_bridge/                  (whole directory)
   apps/mirage-rs/src/integration.rs                (only if it still imports chain)
   ```

3. **Surgically prune**:

   - `apps/mirage-rs/src/lib.rs`: drop `pub mod chain;` and `pub mod http_api;`.
   - `apps/mirage-rs/src/main.rs`: remove `--enable-hdc`, `--enable-knowledge`,
     `--enable-stigmergy` CLI flags and any `--with-rest` flag. The bin only
     drives JSON-RPC, scenario replay, and the EVM fork.
   - `apps/mirage-rs/src/rpc.rs`: keep `mirage_*` extension methods (these are
     simulation helpers, not application state). Remove any `chain_*` JSON-RPC
     methods that depend on the deleted `ChainContext` (grep for
     `chain_post_insight`, `chain_query_insights`, `chain_pheromone_*` etc).
   - `apps/mirage-rs/src/main.rs` startup banner: drop the line claiming
     "HDC enabled" / "Knowledge enabled" — there is no such state anymore.

4. **Move what stays**: `/api/health` and `/api/stats` (EVM-only metrics)
   are *retained* per the original plan. Move them to a tiny
   `apps/mirage-rs/src/http_health.rs` exposing exactly two routes:

   - `GET /health` — `{ "status": "ok", "uptime_secs": N, "block_number": N }`.
   - `GET /stats` — block height, fork upstream URL, cache stats, **no
     application state**.

   This is ~80 LOC. Mount them in `main.rs` only when `binary` feature is
   on (the library mode shouldn't host HTTP).

5. **Update `apps/mirage-rs/Cargo.toml` description**:

   ```text
   description = "In-process Ethereum fork simulator with lazy upstream reads,
   copy-on-write scenario branching, and JSON-RPC server."
   ```

   (No more "Optional chain/roko extensions for HDC-indexed knowledge,
   stigmergy, and roko-core trait bridges.")

6. **CI knob**: ensure `.github/workflows/*.yml` no longer pass
   `--features chain` or `--features dashboard-api` to mirage-rs jobs.

7. **Update CLAUDE.md** "Status table" row for mirage-rs to read:

   | Concern | Status | Notes |
   |---------|--------|-------|
   | `apps/mirage-rs` | Wired | EVM fork + JSON-RPC + scenario replay only |

---

## Step-by-step

### Step 1 — Sanity-check no live consumer remains (30 min)

Before deleting anything, prove nothing depends on the routes outside the
crates we control.

```bash
# Inside the repo root.
rg -t rust -t toml --no-heading 'mirage_rs::http_api|mirage-rs.*api/|VITE_CHAIN_URL' \
  -- crates/ apps/ demo/ contracts/

# In sibling repos.
rg --no-heading 'http://[^"]*:8545/api|VITE_CHAIN_URL' /Users/will/dev/nunchi/nunchi-dashboard /Users/will/dev/nunchi/eng-command-center /Users/will/dev/nunchi/dashboard-job
```

Expected after track `03`: zero hits in the sibling dashboard, zero hits in
`demo/`. Hits inside `apps/mirage-rs/` are fine (they're being deleted).
Any unexpected hit blocks this track until the consumer is rewired.

Anti-pattern: do not silently force-push the deletion if a consumer surfaces.
Stop, file an issue, escalate to the consumer's owner.

### Step 2 — Land `dashboard-api = []` as a no-op shim (1 hr)

Briefly *before* deletion, make `chain` no longer imply `dashboard-api` so
operators get a clean signal:

```toml
chain = ["dep:roko-primitives"]   # was ["dep:roko-primitives", "dashboard-api"]
dashboard-api = []
```

Push and tag this as `mirage-rs-vX.Y.Z-pre-extract`. Operators who still
relied on the implication can opt back in for one release; the deletion in
step 4 is the hard cutover. Keep this shim for **at most** one week.

### Step 3 — Replace `/health` + `/stats` with the slim module (2 hrs)

Create `apps/mirage-rs/src/http_health.rs`:

```rust
use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

#[derive(Clone)]
pub struct HealthState {
    pub started_at: Instant,
    pub current_block: Arc<dyn Fn() -> u64 + Send + Sync>,
    pub upstream: Option<String>,
}

pub fn router(state: HealthState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/stats", get(stats))
        .with_state(state)
}

async fn health(State(s): State<HealthState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "uptime_secs": s.started_at.elapsed().as_secs(),
        "block_number": (s.current_block)(),
    }))
}

async fn stats(State(s): State<HealthState>) -> Json<serde_json::Value> {
    Json(json!({
        "uptime_secs": s.started_at.elapsed().as_secs(),
        "block_number": (s.current_block)(),
        "upstream": s.upstream,
    }))
}
```

Wire it from `main.rs` in place of the old `http_api::router`. Verify with
`curl localhost:8545/health` returns `{"status":"ok",...}`.

### Step 4 — Delete the bolted-on modules (1 day)

In one branch, do the deletes in this order so each compile passes:

1. Delete `apps/mirage-rs/src/roko_bridge/` (depends only on chain).
2. Delete `apps/mirage-rs/src/http_api/` (consumes ChainContext via
   `ApiState`).
3. Delete `apps/mirage-rs/src/chain/`.
4. Edit `apps/mirage-rs/src/lib.rs` — remove `pub mod chain;`,
   `pub mod http_api;`, `pub mod roko_bridge;`.
5. Edit `apps/mirage-rs/src/main.rs` — drop the CLI flags and the
   `http_api::router(ApiState { ... })` call; mount `http_health::router`
   instead.
6. Edit `apps/mirage-rs/src/rpc.rs` — drop any `chain_*` JSON-RPC methods.
   Keep `mirage_*` extensions.
7. Edit `Cargo.toml` per Deliverable 1.
8. `cargo build -p mirage-rs --all-features` — should now build with the
   shrunken feature set.
9. `cargo build --workspace` — should still pass.
10. `cargo test -p mirage-rs` — adapt or remove tests in
    `apps/mirage-rs/tests/` that targeted the deleted surface.

Don't try to do this in many smaller commits unless each one compiles. The
modules are deeply interdependent; either commit the whole deletion or
revert.

### Step 5 — Rewire scenario runner if it touched ChainContext (3-4 hrs)

`apps/mirage-rs/src/scenario.rs` historically touched `ChainContext` to
pre-seed insights for replay scenarios. If it still does:

- Move that logic to a per-scenario fixture file under
  `apps/mirage-rs/tests/fixtures/<name>.json`. The runner reads the
  fixture and uses *only* JSON-RPC calls to set up state (e.g.
  `setBalance`, `setStorageAt`). EVM scenarios should be EVM scenarios.

If a scenario test asserted "after replay the InsightBoard contains X",
move that assertion to `crates/roko-serve/tests/aggregator_e2e.rs`, which
queries the contract directly via `roko-chain`.

### Step 6 — Update docs (1 hr)

- `apps/mirage-rs/README.md`: rewrite the "Features" section. Drop the HDC
  / knowledge / stigmergy paragraphs. Keep EVM fork + JSON-RPC + scenarios.
- `CLAUDE.md`: update the mirage-rs row in the Status table (see
  Deliverable 7).
- `tmp/ux/02-mirage-extraction.md`: append a "Closed 2026-05-XX" header at
  the top noting the extraction is complete and link to the PR.
- `tmp/ux/ux-followup/00-INDEX.md`: bump the closure tally.

### Step 7 — Smoke test sequence (1-2 hrs)

```bash
# 1. Pure-EVM mirage starts and serves JSON-RPC.
cargo run -p mirage-rs -- --upstream <RPC>

# In another terminal:
curl -X POST http://localhost:8545 \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
# expect a numeric result

curl http://localhost:8545/health
# expect {"status":"ok",...}

# 2. Roko-serve aggregator answers /api/* (verifying mirage is no longer
# in the path).
cargo run -p roko-cli -- serve &
sleep 3
curl http://localhost:6677/api/agents
# expect { "items": [...], "total": ... } from the aggregator.

# 3. Demo app loads with no console errors.
yarn --cwd demo/demo-app dev &
# point browser at http://localhost:5173 and confirm.
```

---

## Anti-patterns to avoid

- **Don't keep "just in case" feature gates.** Once the deletion is in,
  there is no `--features chain` to fall back to — that is the whole point.
- **Don't move the deleted code to a sibling crate "for archiving".**
  The git history is the archive. New crates require ongoing maintenance.
- **Don't preserve `chain_*` JSON-RPC methods**. They are part of the
  bolted-on application state, not the simulator.
- **Don't assume the dashboard already migrated.** The Step 1 grep is the
  guard. Skipping it bricks the dashboard.
- **Don't merge this with track `02` (aggregator backends) or `04` (chain
  discovery) in one PR.** Each track has independent rollback risk.

## What success looks like

```
$ tokei apps/mirage-rs/src
-------------------------------------------------------------
 Language            Files        Lines         Code
-------------------------------------------------------------
 Rust                  ~10        <2,500       <2,000
-------------------------------------------------------------
```

(Down from ~9,000 LOC pre-extraction.) Concretely:

- `apps/mirage-rs/src/{fork.rs, rpc.rs, replay.rs, scenario.rs, http_health.rs, persist.rs, provider.rs, main.rs, lib.rs}` and the `precompiles/` directory remain.
- A `cargo build -p mirage-rs --no-default-features` produces a library
  with no HTTP server.
- A `cargo build -p mirage-rs --features binary` produces a CLI with EVM
  fork, JSON-RPC, scenario replay, and the two-route health module.

## Done when

1. Cargo features list = `["binary", "library", "sim-gas"]`.
2. `apps/mirage-rs/src/{http_api,chain,roko_bridge}/` directories are
   gone from `git ls-files`.
3. `cargo build --workspace` passes.
4. `cargo clippy --workspace --no-deps -- -D warnings` passes.
5. `curl localhost:8545/health` returns the slim shape from Step 3.
6. `curl localhost:8545/api/agents` returns 404 (route doesn't exist).
7. `curl localhost:6677/api/agents` returns the aggregator response.
8. Sam's nunchi-dashboard renders without console errors.
9. CLAUDE.md mirage-rs row reflects the new reality.
10. `tmp/ux/02-mirage-extraction.md` has the "Closed" header.
