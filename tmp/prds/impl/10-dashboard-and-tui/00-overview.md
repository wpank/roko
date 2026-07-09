# IMPL-10 Rewrite: Dashboard And TUI Overview

This folder replaces `../IMPL-10-DASHBOARD-AND-TUI.md`.

## Objective

Stabilize the Roko backend surfaces, add the Nexus relay path, complete the dashboard/TUI parity layer, and make the operator surfaces credible across both repos.

## Workspace roots

- Roko backend/TUI repo: `/Users/will/dev/nunchi/roko/roko`
- Dashboard repo: `/Users/will/dev/nunchi/nunchi-dashboard`

## Current codebase reality

- TUI tabs F8 Marketplace and F9 Atelier already exist in `crates/roko-cli/src/tui/`.
- `ProviderHealth`, `ModelComparison`, `EngramDag`, `EpisodeReplay`, and `KnowledgeBrowse` are already represented in the TUI page/subview enums.
- `roko-serve` already exposes routes for plans, agents, providers, websocket streaming, projections, and status.
- The dashboard repo already has routing, design-system components, pages, stores, mock data, websocket helpers, and docs, but still mixes mock/live behavior.
- The original IMPL assumes more Nexus and jobs infrastructure than currently exists in Roko.

## Relevant code and docs

- Roko: `crates/roko-cli/src/tui/`, `crates/roko-serve/src/`
- Dashboard: `/Users/will/dev/nunchi/nunchi-dashboard/src/`
- Dashboard docs: `/Users/will/dev/nunchi/nunchi-dashboard/docs/`
- PRD: `../PRD-10-DASHBOARD-AND-TUI.md`
- Original plan: `../IMPL-10-DASHBOARD-AND-TUI.md`

## Deliverable split

- `01-stabilization-and-nexus-checklist.md`
- `02-dashboard-rewrite-checklist.md`
- `03-tui-polish-and-cross-surface-verification.md`
- `04-page-catalog-widgets-data-contracts-and-network-intelligence.md`

## PRD coverage map

- PRD-10 sections 3-6 map to terminology, topology, Nexus protocol, auth, and identity.
- PRD-10 section 7 maps to the full page catalog.
- PRD-10 sections 8-11 map to widgets, data contracts, network intelligence displays, and jobs-system integration.
- PRD-10 sections 12-15 map to TUI enhancements, dashboard enhancements, stabilization requirements, and demo requirements.

## Fresh-agent rules

- Backend truth lives in Roko, not in dashboard mocks.
- TUI and dashboard should converge on shared data contracts even if rendering differs.
- Any missing backend endpoint must be called out explicitly in the dashboard plan instead of hidden behind mock-only behavior.
