# IMPL-10-DEMO Rewrite: Demo Sprint Overview

This folder replaces `../IMPL-10-DEMO.md`.

## Objective

Turn the large demo sprint plan into assignment-friendly files that can be executed in parallel across:

- dashboard repo work;
- Roko backend/jobs work;
- TUI completion and rehearsal work.

## Workspace roots

- Roko: `/Users/will/dev/nunchi/roko/roko`
- Dashboard: `/Users/will/dev/nunchi/nunchi-dashboard`

## Current codebase reality

- The dashboard repo already contains most of the pages/components named in the original demo doc.
- The Roko repo already has TUI F8/F9 scaffolding, websocket routes, plans routes, and state/projection infrastructure.
- The original demo plan proposes job models/routes that are not yet first-class in the repo and therefore need explicit backend work before the UI can honestly claim live bounty flows.

## Deliverable split

- `01-dashboard-stream-checklist.md`
- `02-backend-and-tui-stream-checklist.md`
- `03-rehearsal-and-demo-acceptance.md`

## Fresh-agent rules

- Demo-visible mock data is allowed only when tagged and isolated.
- Backend tasks must land before UI tasks that claim “live” flows.
- Rehearsal must use the actual routes/events built in the sprint, not imagined future endpoints.
