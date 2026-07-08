# Batch AR06: In-repo mirage demo UI and quickstart

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/implementation-pack/context-pack/03-VERIFICATION-MATRIX.md`
- `tmp/agent-registry/04-deployment-and-dev.md`

Also inspect:

- `apps/mirage-rs/static/quickstart.sh`
- `apps/mirage-rs/static/index.html`
- `apps/mirage-rs/static/js/api.js`
- `apps/mirage-rs/static/js/polling.js`
- `apps/mirage-rs/static/js/state.js`
- `apps/mirage-rs/static/js/main.js`
- `apps/mirage-rs/src/http_api/mod.rs`
- `tmp/agent-registry/remote-demo-runbook.md` if it already exists

## Task

Upgrade the in-repo mirage demo so it is a real proof surface for the new
discovery model.

The target behavior is:

- no dependency on the deprecated mirage `/api/agents` registry path for
  discovery
- merged view of 8004 identities and relay-connected agents
- ability to send messages through direct or relay transport
- ability to point the demo UI at a **remote** mirage URL

Expected implementation shape:

- `api.js` owns remote-base normalization and the HTTP/RPC reads needed for
  chain + relay discovery
- `state.js` owns merged-agent state rather than sprinkling merge logic across
  UI handlers
- `main.js` focuses on rendering and user actions, not transport policy
- `quickstart.sh` still provides a working local proof path after these changes

Concrete outputs expected from this batch:

- a merged discovery path that does not depend on mirage's old `/api/agents`
- remote-base configuration surfaced in the static UI
- at least one small smoke module/script that can validate the data/transport
  assumptions without a browser

## Suggested subagent split

- explorer: inspect current static UI polling and identify exactly which
  mirage-local endpoints it still assumes
- worker A: API/state layer for RPC + relay reads and remote base URL support
- worker B: agent discovery/messaging UI path
- worker C: quickstart updates for local end-to-end startup

## Write scope

- `apps/mirage-rs/static/quickstart.sh`
- `apps/mirage-rs/static/index.html`
- `apps/mirage-rs/static/js/api.js`
- `apps/mirage-rs/static/js/polling.js`
- `apps/mirage-rs/static/js/state.js`
- `apps/mirage-rs/static/js/main.js`
- any new static JS modules needed

## Constraints

1. The demo must remain usable locally.
2. The demo must be able to target a remote mirage URL.
3. Discovery must come from 8004 + relay, not mirage's old registry model.
4. Keep the static JS organized enough that AR07 can document exact user flows
   without reverse-engineering ad hoc behavior.

## Acceptance criteria

- quickstart still works locally and starts the needed services/agents
- static UI can list merged agents from chain + relay
- static UI can send a test message to at least one relay-backed agent
- static UI supports a configurable remote base URL for the final remote demo
- repo includes a dedicated smoke script at
  `apps/mirage-rs/static/js/agent_registry_smoke.mjs`
- message-path selection is explicit enough to support both a direct/local path
  and a relay-backed path

## Verification

At minimum:

```bash
bash -n apps/mirage-rs/static/quickstart.sh
node --experimental-default-type=module --check apps/mirage-rs/static/js/api.js
node --experimental-default-type=module --check apps/mirage-rs/static/js/polling.js
node --experimental-default-type=module --check apps/mirage-rs/static/js/state.js
node --experimental-default-type=module --check apps/mirage-rs/static/js/main.js
node apps/mirage-rs/static/js/agent_registry_smoke.mjs --check
```
