# 03 — Dashboard URL Migration (nunchi-dashboard → roko-serve)

> **Source plan**: `tmp/ux/04-dashboard-migration.md`. Phase 2 plan: "Sam's
> work: change `NEXT_PUBLIC_API_URL` from mirage-rs to roko-serve aggregator."
>
> **Status as of 2026-05-01**:
> - The aggregator on roko-serve exists and serves the agent / predictions /
>   tasks / WS routes already.
> - The Kauri-style sibling dashboard at `/Users/will/dev/nunchi/nunchi-dashboard/`
>   still defaults to `MIRAGE_BASE = http://127.0.0.1:8545` (line 28 of
>   `src/services/constants.ts`). 90 % of REST traffic still flows through
>   mirage's REST surface.
> - `demo/demo-app/` already talks to roko-serve at port 6677.
>
> **Effort**: 1-2 days.
>
> **Risk**: Low. Reversible by reverting the env var. Provided track `02`
> closes first, this is a pure URL swap.

---

## What this plan accomplishes

Move every `/api/*` REST call in `nunchi-dashboard/` from mirage-rs
(`http://127.0.0.1:8545`) to roko-serve (`http://127.0.0.1:6677`). EVM
JSON-RPC calls (`eth_*`) and WebSocket subscriptions (`eth_subscribe`)
stay on mirage-rs since that's its actual job.

After this plan:

- `VITE_CHAIN_URL` (formerly the only knob, used as both EVM RPC and
  REST base) splits into `VITE_CHAIN_RPC_URL` (mirage JSON-RPC) and
  `VITE_API_URL` (roko-serve aggregator). Defaults: `:8545` and `:6677`.
- The `MIRAGE_BASE` constant is renamed to `MIRAGE_RPC_BASE` and used
  only for `eth_*`, `mirage_*`, and the connection probe in `verifyChainId`.
- `RELAY_BASE` is sourced from roko-serve, not mirage.
- Every `fetch(\`${MIRAGE_BASE}/api/...\`)` becomes
  `fetch(\`${API_BASE}/api/...\`)`.
- `nunchi-dashboard` works against an environment where mirage-rs has been
  reduced to JSON-RPC only (validates track `01`).

## Why this matters

Until the Kauri-style dashboard switches, mirage-rs cannot retire its
REST surface. This is the gate keeping `01` (mirage cleanup) blocked.

## Sequencing precondition

**Track `02` (knowledge + pheromones in aggregator) must be merged and
deployed first.** Otherwise the Knowledge and Stigmergy tabs in
nunchi-dashboard will go blank after the swap.

The Step 1 verification below catches this.

---

## Required reading

```
nunchi-dashboard/src/services/constants.ts
nunchi-dashboard/src/services/mirage-api.ts             (1656 lines — API client)
nunchi-dashboard/src/services/mirage-knowledge.ts
nunchi-dashboard/src/services/mirage-chain.ts           (eth_*; stays on mirage)
nunchi-dashboard/src/services/mirage-contracts.ts       (eth_*; stays on mirage)
nunchi-dashboard/src/services/mirageWsInvalidation.ts   (eth_subscribe; stays)
nunchi-dashboard/src/services/perpsMarket-chain.ts
nunchi-dashboard/src/services/bountyMarket-queries.ts
nunchi-dashboard/src/services/deployment.ts
nunchi-dashboard/src/stores/connectivityStore.ts
nunchi-dashboard/.env.example                           (if present, else create)
nunchi-dashboard/README.md
crates/roko-serve/src/routes/aggregator.rs              (the new target)
demo/demo-app/src/lib/serve-url.ts                      (reference for already-migrated pattern)
```

---

## Deliverables

1. **Split env vars** in `nunchi-dashboard`:

   - `VITE_CHAIN_RPC_URL` — mirage-rs JSON-RPC, default `http://127.0.0.1:8545`.
   - `VITE_API_URL` — roko-serve aggregator base, default `http://127.0.0.1:6677`.
   - Keep `VITE_CHAIN_URL` for one release as a deprecated alias mapped to
     `VITE_CHAIN_RPC_URL` to ease ops migration.

2. **Refactored `constants.ts`**:

   ```ts
   // Mirage-RS handles JSON-RPC + WS only. URL accepted in both old and
   // new env shapes.
   export const MIRAGE_RPC_BASE = normalizeBaseUrl(
     import.meta.env.VITE_CHAIN_RPC_URL ?? import.meta.env.VITE_CHAIN_URL,
     "http://127.0.0.1:8545",
   );

   // Roko-serve aggregator handles /api/*. Bearer token from auth flow.
   export const API_BASE = normalizeBaseUrl(
     import.meta.env.VITE_API_URL,
     "http://127.0.0.1:6677",
   );

   export const RELAY_BASE = `${API_BASE}/relay`;
   ```

   Drop `MIRAGE_BASE`. Audit consumers and route them to either
   `MIRAGE_RPC_BASE` or `API_BASE` based on whether they make a JSON-RPC
   call or a REST call.

3. **Audit table** — every grep hit for `MIRAGE_BASE` across
   `nunchi-dashboard/src/`. Record in this plan or in `nunchi-dashboard/MIGRATION.md`.
   Each row ends in either `→ MIRAGE_RPC_BASE` (eth_*) or `→ API_BASE`
   (REST).

4. **Connectivity probe**: `connectivityStore.ts` currently probes one URL
   to check both REST and JSON-RPC. Split into two probes; render two
   distinct status pills in the topbar (already common UX in dev tools).

5. **WebSocket multiplex**: today nunchi-dashboard opens a WS to
   `ws://localhost:8545` for `eth_subscribe`. The aggregator at
   `:6677/api/ws` is a separate channel for live agent data. Both stay;
   document the topology in `nunchi-dashboard/README.md`.

6. **Per-PR migration** (rollback-friendly): make the swap behind a
   short-lived `VITE_API_USE_AGGREGATOR` boolean during the dual-write
   window so ops can flip back if a regression appears. Remove the flag
   after one stable week.

7. **Smoke fixture**: `nunchi-dashboard/scripts/smoke-aggregator.sh` curls
   `${VITE_API_URL}/api/agents`, `/api/predictions/sessions`,
   `/api/knowledge/entries`, `/api/pheromones`, `/api/tasks`, asserts each
   is HTTP 200 and JSON-shaped.

---

## Step-by-step

### Step 1 — Verify prerequisites (10 min)

Before changing the dashboard, prove roko-serve actually answers all the
routes nunchi-dashboard will hit. Run roko-serve locally:

```bash
cargo run -p roko-cli -- serve &
sleep 3

# REST surfaces (must all return 200; pheromones + knowledge depend on track 02).
for path in \
  "/api/agents" \
  "/api/agents/topology" \
  "/api/predictions/sessions" \
  "/api/predictions/claims" \
  "/api/knowledge/entries" \
  "/api/knowledge/edges" \
  "/api/knowledge/kinds" \
  "/api/pheromones" \
  "/api/pheromones/topology" \
  "/api/tasks" \
  "/api/tasks/stats"; do
    code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:6677${path}")
    echo "${code}  ${path}"
done
```

Every line must be `200`. If `/api/pheromones*` or `/api/knowledge*` is
404, **track `02` is not yet shipped — stop and complete that track first.**

### Step 2 — Split env vars (1 hr)

1. Add to `nunchi-dashboard/.env.example` (create if missing):

   ```
   # JSON-RPC + WS to mirage-rs.
   VITE_CHAIN_RPC_URL=http://127.0.0.1:8545

   # REST aggregator on roko-serve.
   VITE_API_URL=http://127.0.0.1:6677

   # (Deprecated; will be removed.)
   # VITE_CHAIN_URL=http://127.0.0.1:8545
   ```

2. Edit `src/services/constants.ts` per Deliverable 2. Keep `MIRAGE_BASE`
   as a deprecated re-export aliased to `MIRAGE_RPC_BASE` *only* until the
   audit (Step 3) reroutes every consumer.

### Step 3 — Audit + reroute every `MIRAGE_BASE` reference (3-4 hrs)

```bash
cd /Users/will/dev/nunchi/nunchi-dashboard
rg -n 'MIRAGE_BASE|VITE_CHAIN_URL' src/ scripts/ vite.config.* | tee /tmp/mirage-uses.txt
```

For each line, classify:

- Hits an `eth_*` JSON-RPC method or `MIRAGE_RPC_BASE` is needed for
  WS subscription → keep on `MIRAGE_RPC_BASE`.
- Hits `/api/*` paths → switch to `API_BASE`.
- Hits `/relay/*` → `${API_BASE}/relay/*` (relay logic moves to roko-serve
  in this migration; verify roko-serve's `relay` mount in `lib.rs`).

Common substitutions:

```ts
// Before
const url = `${MIRAGE_BASE}/api/agents`;
// After
const url = `${API_BASE}/api/agents`;

// Before
const url = `${MIRAGE_BASE}`;  // POST eth_call
// After
const url = `${MIRAGE_RPC_BASE}`;
```

Commit each file change separately — git history makes a rollback trivial
if one file breaks the dashboard in QA.

### Step 4 — Update `connectivityStore.ts` (1 hr)

Replace the single probe with two:

```ts
// Probe roko-serve.
const apiUp = await fetch(`${API_BASE}/health`)
  .then(r => r.ok)
  .catch(() => false);

// Probe mirage-rs JSON-RPC.
const rpcUp = await fetch(MIRAGE_RPC_BASE, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ jsonrpc: "2.0", method: "eth_chainId", params: [], id: 1 }),
}).then(r => r.ok).catch(() => false);
```

Render two pills in the topbar: "API" and "RPC". The latter must turn red
gracefully when mirage-rs is offline; existing dashboard panels that need
on-chain reads should show empty / loading states rather than crashing.

### Step 5 — Yarn install + dev server (15 min)

```bash
cd /Users/will/dev/nunchi/nunchi-dashboard
yarn install
yarn dev
```

(Per the user's repo rules, `yarn`, never `npm`.) Open the dashboard,
walk every tab. Two things to verify:

- The Network tab shows REST hitting `:6677` and JSON-RPC hitting `:8545`.
- No tab is blank; no panel shows a skeleton beyond ~5 s.

### Step 6 — Run the smoke script (15 min)

`nunchi-dashboard/scripts/smoke-aggregator.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
API="${VITE_API_URL:-http://127.0.0.1:6677}"
for path in \
  "/api/agents" "/api/agents/topology" \
  "/api/predictions/sessions" "/api/predictions/claims" \
  "/api/knowledge/entries" "/api/knowledge/kinds" \
  "/api/pheromones" "/api/pheromones/topology" \
  "/api/tasks" "/api/tasks/stats"; do
  code=$(curl -s -o /dev/null -w '%{http_code}' "${API}${path}")
  if [ "$code" != "200" ]; then
    echo "FAIL ${path} → ${code}"
    exit 1
  fi
  echo "ok   ${path}"
done
echo "smoke passed"
```

Add to `package.json` scripts as `smoke:aggregator`. CI (`.github/workflows/`)
runs this whenever PRs touch `src/services/`.

### Step 7 — Update README + .env.example (30 min)

`nunchi-dashboard/README.md` — add a section:

```markdown
## Backend wiring

The dashboard talks to two backends:

- **`VITE_CHAIN_RPC_URL`** (default `:8545`) — mirage-rs JSON-RPC. Used for
  `eth_*` reads, contract calls, and `eth_subscribe` over WS.
- **`VITE_API_URL`** (default `:6677`) — roko-serve REST aggregator. Used
  for agent registry, predictions, knowledge, pheromones, tasks, the WS
  multiplex stream, and orchestration views.

In production both URLs typically resolve to the same host with different
paths. In dev they map to two local processes.
```

### Step 8 — Stage rollout (1 day in production)

For Vercel deployments (`kauri-dashboard-v2.vercel.app`):

1. Set `VITE_API_USE_AGGREGATOR=false` in the production environment.
   The migrated code keeps the old `MIRAGE_BASE` path active while the
   flag is false.
2. Deploy the migration PR.
3. After 24 h of zero-error logs, set `VITE_API_USE_AGGREGATOR=true`. Watch
   the next 1 hr of logs.
4. After 7 stable days, remove the flag and the old code path entirely.

### Step 9 — Post-migration cleanup (1 hr, +7 days later)

- Delete `MIRAGE_BASE` re-export.
- Delete `VITE_API_USE_AGGREGATOR` references.
- Delete the dual-path branches.
- Bump nunchi-dashboard package version (semver minor — public env shape
  changed).

This unblocks track `01` (mirage extraction). Ping its owner.

---

## Anti-patterns to avoid

- **Don't unify the two URLs into a single base.** mirage-rs and
  roko-serve have different ports, different auth models (none vs
  bearer), and different lifecycles. Conflating them creates outages
  when one is down and the other isn't.
- **Don't move JSON-RPC to roko-serve.** roko-serve does not implement
  EVM. Routing eth_* through it would either fail or require a proxy
  layer we don't want.
- **Don't remove the deprecation alias on day one.** Anyone with an
  in-flight branch using `VITE_CHAIN_URL` will see breakage. One release
  cycle of dual support saves morning Slack threads.
- **Don't change the WS endpoint at the same time.** The eth_subscribe WS
  on mirage-rs and the aggregator's `/api/ws` are different protocols.
  Migrating them together couples failure modes; do them in two PRs even
  if the LOC is small.
- **Don't bypass the smoke script in CI.** A typo in a constants file is
  the most likely regression here; the smoke catches it.

## Done when

1. `rg -n 'MIRAGE_BASE' nunchi-dashboard/src/` returns 0 lines (or only a
   single deprecated re-export gated by a comment that names this plan).
2. The smoke script exits 0 against a roko-serve + mirage-rs running
   locally.
3. nunchi-dashboard renders all tabs with no console errors against the
   roko-serve aggregator.
4. Network tab shows REST traffic on `:6677` and JSON-RPC on `:8545`.
5. The Vercel deployment has `VITE_API_URL` set to the aggregator.
6. `tmp/ux/04-dashboard-migration.md` has a "Closed YYYY-MM-DD" note at
   the top linking the dashboard migration PR.
