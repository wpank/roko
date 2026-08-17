# IMPL-08 Rewrite: Surfaces And UX Overview

This folder replaces `../IMPL-08-SURFACES.md`.

## Objective

Extend the user-facing surfaces around the current runtime: CLI, persistent chat, TUI, API-backed web surfaces, MCP distribution, and packaging flows.

## Current codebase reality

- CLI and TUI already exist in `crates/roko-cli/src/`.
- HTTP and websocket surfaces already exist in `crates/roko-serve/src/`.
- F8 Marketplace and F9 Atelier tabs already have initial TUI implementations.
- The package system and several CLI DX items are still specified or partial.
- Web dashboard work lives mostly in the separate repo `/Users/will/dev/nunchi/nunchi-dashboard`.

## Relevant code and docs

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/chat.rs`
- `crates/roko-cli/src/tui/`
- `crates/roko-serve/src/routes/`
- `docs/12-interfaces/17-accessibility-and-current-status.md`
- `docs/19-deployment/13-current-status-and-port-allocation.md`
- `../PRD-08-DEPLOYMENT-AND-UX.md`

## Deliverable split

- `01-cli-chat-and-tui-checklist.md`
- `02-web-mcp-packaging-and-dx-checklist.md`
- `03-product-surfaces-deployment-onboarding-security-and-observability.md`

## PRD coverage map

- PRD-08 sections 2-6 map to AI Studio, Agent Studio, OpenClaw, CLI design, CLI DX, persistent chat, and TUI.
- PRD-08 sections 7-9 map to deployment, gateway, onboarding, and MCP distribution.
- PRD-08 sections 10-12 map to coordination, security model, and monitoring/observability.

## Fresh-agent rules

- Keep CLI as the canonical fallback surface.
- Prefer extending existing commands and routes over inventing new parallel entrypoints.
- Any new surface feature must define its CLI equivalent or justify why it is surface-specific.
