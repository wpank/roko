# Dashboard Rewrite Checklist

## Scope

Use this file for the `nunchi-dashboard` repo only.

## Workspace root

- `/Users/will/dev/nunchi/nunchi-dashboard`

## Implementation checklist

- [ ] Start from the existing router and page tree, not from a blank React scaffold.
- [ ] Remove or isolate hardcoded mock data.
  - every fallback must be visibly tagged;
  - live API paths must be preferred;
  - stale mocks must not silently masquerade as truth.
- [ ] Align the API client with real Roko endpoints first.
  - `src/services/rokoApi.ts`
  - websocket client/store
  - query keys and invalidation behavior
- [ ] Standardize data contracts across pages.
  - observatory pages;
  - marketplace/job pages;
  - studio pages;
  - atelier pages;
  - settings/auth flows.
- [ ] Preserve or improve the existing design system instead of creating parallel UI primitives.
- [ ] Make every page degrade cleanly.
  - loading state;
  - error state;
  - empty state;
  - tagged mock fallback only when backend work is still intentionally pending.

## Relevant current files

- `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoWs.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/data/`

## Verification checklist

- [ ] Page routing works for all primary sections.
- [ ] Each page can render from live data, tagged fallback data, loading, and error states.
- [ ] Websocket-driven invalidation works for jobs and agent heartbeats where expected.
- [ ] No page depends on hidden mock globals.

## Acceptance criteria

- The dashboard renders a coherent operator surface backed primarily by live Roko APIs.
- Mock data is explicit and quarantined.
- Shared contracts with Roko are documented enough for parallel backend/frontend work.
