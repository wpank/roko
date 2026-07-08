# Batch AR08: Optional Kauri dashboard migration

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/04-deployment-and-dev.md`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`

Also inspect these external-repo files:

- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/ai-studio/AskPanel.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/constants.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/.env.example`
- `/Users/will/dev/nunchi/nunchi-dashboard/package.json`

## Task

Prepare or implement the external Kauri dashboard migration from:

- `localStorage` endpoint cache
- `VITE_ROKO_URL` split

to:

- 8004 identity reads
- relay presence reads
- direct-vs-relay transport selection

Expected implementation shape:

- constants/config should derive the relay base from the main mirage/chain base
  by default rather than depending on a second cached URL source
- service-layer code should own discovery/transport selection, not the UI alone
- UI changes should be limited to selecting the intended agent and invoking the
  right service path

Concrete outputs expected from this batch:

- no main-path dependence on `localStorage` endpoint cache
- merged-agent discovery coming from chain identity + relay presence
- transport selection that can explain “direct” vs “relay” in code instead of
  hidden conditional behavior

This batch is optional for repo completion because the repo-local success
criterion is the in-repo mirage demo UI, but it is useful for parity.

## Suggested subagent split

- explorer: inspect the current dashboard transport and config seams
- worker A: constants + API client changes
- worker B: AskPanel and merged-agent hook changes
- worker C: external repo verification notes

## Write scope

- only the external repo files listed above, unless a clearly-related new hook
  or service file is needed

## Acceptance criteria

- dashboard no longer depends on localStorage endpoint cache for the main path
- dashboard can derive relay URL from chain URL by default
- dashboard can route through direct or relay transport as appropriate
- if new dashboard service/helpers are added, they stay adjacent to the
  existing `mirage-api` / constants seam instead of scattering logic through
  multiple components

## Verification

At minimum:

```bash
npm --prefix /Users/will/dev/nunchi/nunchi-dashboard ci
npm --prefix /Users/will/dev/nunchi/nunchi-dashboard run typecheck
npm --prefix /Users/will/dev/nunchi/nunchi-dashboard run build
```
