# DB — Dashboard URL migration (manual)

> Source plan: `tmp/ux/implementation-plans/03-dashboard-url-migration.md`.
> Sibling repo: `/Users/will/dev/nunchi/nunchi-dashboard/`.
> Tracker rows: `ISSUE-TRACKER.md` Wave DB.

This work happens in TypeScript / Vite outside the Rust workspace.
The runner can't drive it; tick the boxes by hand.

---

## Pre-flight

- [ ] **Wave AG green**. Verify by running roko-serve locally and
  hitting every aggregator route the dashboard uses. The expected
  output is HTTP 200 + JSON-shaped body for each:

  ```bash
  cargo run -p roko-cli -- serve &
  sleep 3
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
  Every line must be `200`. If `/api/pheromones*` or `/api/knowledge*`
  is `404`, **stop** — Wave AG isn't ready.

- [ ] Open the sibling repo: `cd /Users/will/dev/nunchi/nunchi-dashboard`.

- [ ] On a fresh branch: `git checkout -b db-aggregator-migration`.

---

## DB01 — Verify roko-serve answers every dashboard route

Already done in Pre-flight. Tick when complete.

- [ ] DB01 — smoke probe of every aggregator route returns 200.

---

## DB02 — Split env vars

- [ ] Update `.env.example` (create if missing):

  ```
  VITE_CHAIN_RPC_URL=http://127.0.0.1:8545
  VITE_API_URL=http://127.0.0.1:6677
  # VITE_CHAIN_URL=http://127.0.0.1:8545   # deprecated alias; will be removed
  ```

- [ ] Edit `src/services/constants.ts`:

  ```ts
  export const MIRAGE_RPC_BASE = normalizeBaseUrl(
    import.meta.env.VITE_CHAIN_RPC_URL ?? import.meta.env.VITE_CHAIN_URL,
    "http://127.0.0.1:8545",
  );

  export const API_BASE = normalizeBaseUrl(
    import.meta.env.VITE_API_URL,
    "http://127.0.0.1:6677",
  );

  export const RELAY_BASE = `${API_BASE}/relay`;

  /** @deprecated MIRAGE_BASE → use MIRAGE_RPC_BASE (eth_*) or API_BASE (REST) */
  export const MIRAGE_BASE = MIRAGE_RPC_BASE;
  ```

- [ ] Tick: `[ ] DB02`.

---

## DB03 — Reroute every `MIRAGE_BASE` reference

```bash
cd /Users/will/dev/nunchi/nunchi-dashboard
rg -n 'MIRAGE_BASE|VITE_CHAIN_URL' src/ scripts/ vite.config.* > /tmp/mirage-uses.txt
```

For each line in the audit:

- [ ] Hits an `eth_*` JSON-RPC method or WS subscription → keep on
  `MIRAGE_RPC_BASE`. (e.g. `mirage-chain.ts`, `mirage-contracts.ts`,
  `mirageWsInvalidation.ts`, `perpsMarket-chain.ts`, `bountyMarket-queries.ts`.)
- [ ] Hits `/api/*` paths → switch to `API_BASE`. (e.g. `mirage-api.ts`,
  `mirage-knowledge.ts`.)
- [ ] Hits `/relay/*` → switch to `${API_BASE}/relay/*`.

Commit each file's rewire as its own commit so rollback is granular.

- [ ] Tick: `[ ] DB03`.

---

## DB04 — Two-pill connectivity probe

- [ ] In `src/stores/connectivityStore.ts`, replace the single probe
  with two probes:

  ```ts
  const apiUp = await fetch(`${API_BASE}/health`).then(r => r.ok).catch(() => false);

  const rpcUp = await fetch(MIRAGE_RPC_BASE, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", method: "eth_chainId", params: [], id: 1 }),
  }).then(r => r.ok).catch(() => false);
  ```

- [ ] Render two pills in the topbar: "API" (green=up) and "RPC" (green=up).

- [ ] Tick: `[ ] DB04`.

---

## DB05 — Smoke script + CI

- [ ] Create `scripts/smoke-aggregator.sh`:

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
    if [ "$code" != "200" ]; then echo "FAIL ${path} → ${code}"; exit 1; fi
    echo "ok   ${path}"
  done
  echo "smoke passed"
  ```

  `chmod +x scripts/smoke-aggregator.sh`.

- [ ] Add a `smoke:aggregator` script to `package.json`:

  ```json
  { "scripts": { "smoke:aggregator": "bash scripts/smoke-aggregator.sh" } }
  ```

- [ ] Add a CI workflow that runs this on PRs touching `src/services/`:

  ```yaml
  # .github/workflows/aggregator-smoke.yml
  name: aggregator-smoke
  on:
    pull_request:
      paths: ["src/services/**", "scripts/smoke-aggregator.sh"]
  jobs:
    smoke:
      runs-on: ubuntu-latest
      services:
        roko-serve:
          image: ghcr.io/<org>/roko-serve:latest
          ports: ["6677:6677"]
      steps:
        - uses: actions/checkout@v4
        - run: yarn install
        - run: yarn smoke:aggregator
  ```

  (Skip the workflow if there's no roko-serve image yet — note the
  TODO in `nunchi-dashboard/MIGRATION.md`.)

- [ ] Tick: `[ ] DB05`.

---

## DB06 — README + post-rollout cleanup

- [ ] Update `nunchi-dashboard/README.md` with a "Backend wiring"
  section explaining the two URLs.

- [ ] Stage rollout:
  - `VITE_API_USE_AGGREGATOR=false` in production env (one-off
    feature flag during dual-write window).
  - Deploy the migration PR.
  - After 24 h of green logs: flip to `VITE_API_USE_AGGREGATOR=true`.
  - After 7 stable days: remove the flag and the deprecated
    `MIRAGE_BASE` alias.

- [ ] Tick: `[ ] DB06`.

- [ ] Tick: ISSUE-TRACKER.md Wave DB rows DB01-DB06 to `[x]`.

---

## Done when

- [ ] `rg -n 'MIRAGE_BASE' nunchi-dashboard/src/` returns 0 lines (or
  only the deprecated alias).
- [ ] Smoke script exits 0 against a roko-serve + mirage-rs running locally.
- [ ] All dashboard tabs render against the aggregator with no console errors.
- [ ] Vercel production deploy uses `VITE_API_URL` set to the aggregator.
- [ ] `tmp/ux/04-dashboard-migration.md` carries a "Closed YYYY-MM-DD" header
  pointing at the migration PR.

After DB closes: ping the owner of Wave M (`tmp/runners/ux-impl/prompts/M01.prompt.md`)
to start the mirage extraction.
