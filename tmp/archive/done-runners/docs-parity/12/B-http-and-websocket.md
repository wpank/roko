# B — HTTP, SSE, WebSocket, and Sidecar Truth Pass

Refresh target for docs 05 and 06: present the server layer as a shipping
control plane, flag the default-port drift directly, and keep future transport
work deferred.

Generated: 2026-04-18

---

## Headline

- `roko-serve` is not a scaffold. Use the audit-corrected headline:
  200+ routes and roughly 30K LOC.
- The direct source pass also confirms a broad live route surface under
  `crates/roko-serve/src/routes/`, plus shipping SSE, WebSocket, and sidecar
  messaging.
- The main unresolved docs problem is the `9090` vs `6677` split.

## Verified Anchors

| Surface | Status | Notes |
|---|---|---|
| `roko-serve` control plane | Shipping | audit headline is 200+ routes / 30K LOC; direct pass confirms 19 route files under `src/routes/` |
| Main API composition | Shipping | `crates/roko-serve/src/routes/mod.rs:55-83` merges the route groups and nests `/api` |
| Routing explanation endpoint | Shipping | `crates/roko-serve/src/routes/providers.rs:36-45` exposes `/routing/explain` |
| SSE | Shipping | `routes/sse.rs` is live; treat `/api/events` as current transport |
| Top-level WebSocket | Shipping | `routes/ws.rs` is live; keep realtime wording concrete |
| Per-agent `/message` + `/stream` | Shipping | `crates/roko-agent-server/src/features/messaging.rs:29-33` wires both routes |
| Port defaults | Drift | `main.rs` defaults `serve` and daemon to `9090`; chat and READMEs still use `6677` |

## Rewrite Guidance

### Keep

- `roko-serve` as the current HTTP control plane.
- SSE and WebSocket as live realtime transports.
- Per-agent sidecar messaging, registration, and bearer-auth framing.
- Route families such as status, plans, PRDs, research, providers, templates,
  subscriptions, and webhooks.

### Narrow

- Stop repeating the stale "~85 routes on :6677" headline.
- Do not imply that every endpoint table cell in the interface docs has been
  verified behavior-by-behavior; the route stack is real, but not every prose
  contract was re-audited in this batch.
- Keep the port inconsistency explicit until runtime defaults are unified.

### Defer

- OpenAPI generation
- gRPC
- browser UI claims that depend on a first-party frontend

## Port Drift Note

Use this exact framing until code and docs are unified:

- `crates/roko-cli/src/main.rs:274-275` defaults chat to
  `http://localhost:6677`
- `crates/roko-cli/src/main.rs:337-340` and `:355-372` default serve and
  daemon paths to `9090`
- `crates/roko-serve/README.md` still describes `6677` as the default

That is a concrete follow-up item, not a reason to downgrade the shipping
server surface.
