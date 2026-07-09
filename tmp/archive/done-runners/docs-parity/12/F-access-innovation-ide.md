# F — Status, Accessibility, Innovation, and IDE Material

Refresh target for docs 16, 17, 18, and 20: make the shipping core obvious,
scope accessibility to what exists now, and push the innovation halo into
explicit deferred language.

Generated: 2026-04-18

---

## Headline

- The topic-level status story should be "shipping core with deferred halo,"
  not "scaffold."
- Accessibility claims should track the shipped CLI, TUI, and HTTP surfaces,
  not absent portal/Spectre/A2UI implementations.
- Sonification, rich UX primitives, ACP, and full IDE integration remain
  deferred.
- `roko-mcp-code` is real, but it is not the same thing as a shipped IDE
  product surface.
- Some ACP/Cursor-side plumbing exists in backend crates, but there is still no
  shipped editor-facing runtime or CLI surface.

## Rewrite Guidance

### Shipping Core To Emphasize

- CLI
- ratatui TUI
- `roko-serve` HTTP + SSE/WS
- per-agent sidecar

### Ship Soon To Carry Forward

- REF28 CLI parity / muscle memory
- REF26 StateHub hardening
- cleanup of the `9090` vs `6677` default split

### Deferred

- sonification
- rich UX primitives
- multimodal UX ideas
- ACP runtime
- VS Code participant / extension / fork work

## Accessibility Scope Rule

Keep accessibility language tied to:

- keyboard-driven CLI and TUI use,
- terminal-compatible status/readout surfaces,
- realistic current theme and contrast concerns.

Do not write accessibility sections as though the portal, Spectre canvas, or
A2UI components already exist.

## IDE Rule

Credit the shipping MCP server path where appropriate, and note any backend ACP
plumbing as partial groundwork only. Keep "IDE integration" in proposal
language until there is an actual editor-side runtime.
