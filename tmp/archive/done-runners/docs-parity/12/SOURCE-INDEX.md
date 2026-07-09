# SOURCE-INDEX — Corrected Anchors for Topic 12

Use these anchors when rewriting the interface docs. They replace the older
parity pack's stale route counts, port assumptions, and command claims.

Generated: 2026-04-18

---

## Audit-Corrected Headline Facts

- `roko-serve`: 200+ routes, roughly 30K LOC
- Topic-level TUI surface: 58K LOC, ratatui wired
- `roko new`: does not ship as a top-level CLI command
- standalone `roko explain`: does not ship as a top-level CLI command
- `9090` vs `6677`: unresolved default-port drift that should be documented

## CLI Anchors

| Anchor | What It Proves |
|---|---|
| `crates/roko-cli/src/main.rs:191-344` | live top-level command tree; use this instead of stale command tables |
| `crates/roko-cli/src/main.rs:274-275` | chat default still points at `http://localhost:6677` |
| `crates/roko-cli/src/main.rs:337-340` | `roko serve` port help/default is `9090` |
| `crates/roko-cli/src/main.rs:355-372` | daemon start/restart also default to `9090` |
| `crates/roko-cli/src/main.rs:503-520` | `PrdDraftCmd::New` exists; do not confuse it with a hypothetical top-level `roko new` |
| `crates/roko-cli/src/main.rs:658-677` | `model route --explain` is the live nearby explain surface |
| `crates/roko-core/src/config/schema.rs:2554` | config schema default for the server port is also `9090` |

## TUI Anchors

| Anchor | What It Proves |
|---|---|
| `crates/roko-cli/src/tui/tabs.rs:8-49` | `F1`-`F7` tabs are real and named |
| `crates/roko-cli/src/tui/` | substantial ratatui implementation exists; rewrite around tabs/views/modals instead of "not wired" |
| `crates/roko-cli/src/tui/postfx.rs` and `postfx_pipeline.rs` | effects pipeline is real enough to mention directly |

## HTTP and Streaming Anchors

| Anchor | What It Proves |
|---|---|
| `crates/roko-serve/src/routes/mod.rs:55-83` | the main `/api` surface is assembled from many live route groups |
| `crates/roko-serve/src/routes/providers.rs:36-45` | `/routing/explain` ships |
| `crates/roko-serve/src/routes/sse.rs` | SSE ships |
| `crates/roko-serve/src/routes/ws.rs` | top-level WebSocket ships |
| `crates/roko-agent-server/src/features/messaging.rs:29-33` | sidecar `/message` and `/stream` ship |

## Port Drift Anchors

| Anchor | Value |
|---|---|
| `crates/roko-cli/src/main.rs:274-275` | chat uses `6677` |
| `crates/roko-cli/src/main.rs:337-340` | serve uses `9090` |
| `crates/roko-cli/src/main.rs:355-372` | daemon uses `9090` |
| `crates/roko-serve/README.md:3-21` | README still presents `6677` as the default |
| `crates/roko-cli/README.md:79-82` | README examples are mixed `6677` / `9090` |

## Explicitly Deferred Surfaces

- Spectre runtime or renderer
- first-party browser frontend
- A2UI runtime/schema
- sonification/audio runtime
- full IDE runtime; backend ACP/Cursor plumbing does not yet make this a
  shipped user-facing surface

## Practical Search Set

```bash
sed -n '191,372p' crates/roko-cli/src/main.rs
sed -n '503,520p' crates/roko-cli/src/main.rs
sed -n '658,677p' crates/roko-cli/src/main.rs
sed -n '55,83p' crates/roko-serve/src/routes/mod.rs
sed -n '36,45p' crates/roko-serve/src/routes/providers.rs
sed -n '29,33p' crates/roko-agent-server/src/features/messaging.rs
rg -n "9090|6677" crates/roko-cli crates/roko-serve docs/12-interfaces tmp/docs-parity/12 --glob '*.rs' --glob '*.md'
```
