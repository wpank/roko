# Demo Stream A: Dashboard Checklist

## Scope

Use this file for the dashboard repo stream only.

## Workspace root

- `/Users/will/dev/nunchi/nunchi-dashboard`

## Checklist

- [ ] Keep the router/layout/design-system path already present and clean it up instead of rebuilding again.
- [ ] Remove untagged mocks from `src/data/` usage.
- [ ] Align service clients and websocket invalidation with the real Roko backend routes that exist today.
- [ ] Land the high-value demo pages first.
  - landing
  - observatory/live agents
  - marketplace/job board/create/detail
  - atelier
  - one agent-studio path
- [ ] Make every demo page visually and behaviorally honest.
  - loading state;
  - error state;
  - tagged fallback state.
- [ ] Verify navigation and deep links.
  - `/`
  - `/app`
  - `/app/marketplace`
  - `/app/marketplace/jobs/:id`
  - `/app/atelier`

## Verification checklist

- [ ] Dashboard builds and runs locally.
- [ ] Core demo routes render without blank/error screens.
- [ ] Live-data pages tolerate backend unavailability with explicit tagged fallbacks.

## Acceptance criteria

- A fresh demo operator can navigate the dashboard without hitting dead buttons or silent mocks.
- Live pages prefer real API data.
- Mock-only behavior, where still needed, is visibly tagged in code and UI.
