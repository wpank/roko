# Interfaces Summary — 12

## Shipping now

- substantial top-level CLI command tree in `main.rs`
- TUI with 7 tabs, modal stack, widgets, PostFX, config and approval surfaces
- `roko-serve` HTTP control plane and route stack
- per-agent sidecar with `/message` and `/stream`
- MCP code-intelligence server via `roko-mcp-code`

## Shipping, but easy to misdescribe

- `roko-serve` is more real than `Scaffold`, but some endpoint details still need route-level verification
- TUI is much more real than the docs imply, but not literally the same thing as the 29-screen spec
- Rosedust ships as a narrow theme/palette surface, not the full design language the docs describe
- CLI command docs overclaim `roko new` and standalone `roko explain`
- serve defaults are inconsistent across code and READMEs (`9090` vs `6677`)

## Mostly future

- Spectre visualization
- portal frontend
- A2UI
- sonification
- voice/gesture/multimodal UX
- ACP runtime and VS Code extension work
