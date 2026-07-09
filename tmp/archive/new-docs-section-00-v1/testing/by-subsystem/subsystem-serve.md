# roko-serve — Test Coverage

> Tests for the HTTP control plane: 200+ routes, SSE streams, and WebSocket connections.

**Status**: Shipping (HTTP server ships; test count not reported in 2026-04-17 audit)
**Crate**: `roko-serve`
**Section**: 12 — Interfaces
**Last reviewed**: 2026-04-19

---

## Test Count

Not reported in the 2026-04-17 audit. `roko-serve` is listed as Shipping but its test count was not captured.

---

## Key Test Focus Areas

### Route Coverage (200+ routes)

- Every route returns the correct HTTP status code for valid requests.
- Every route returns 400 (Bad Request) for malformed request bodies.
- Every route returns 401 (Unauthorized) for missing/invalid authentication.
- Every route returns 404 (Not Found) for unknown resource IDs.

### SSE Streams

- An SSE stream emits events as they occur.
- Disconnecting a client does not crash the server.
- An SSE stream that has no events still sends heartbeat events.

### WebSocket

- WebSocket handshake completes for valid requests.
- WebSocket messages are received and processed in order.
- A closed WebSocket client connection is cleaned up without resource leaks.

### Control Plane Operations

- `POST /plan/run` starts a plan and returns a plan ID.
- `GET /plan/{id}/status` returns the current plan status.
- `POST /plan/{id}/cancel` cancels a running plan.
- `GET /engrams/{id}` returns an Engram by content hash.
- `GET /health` returns 200 OK when the system is healthy.

---

## Known Gaps

- `roko-serve` test count is unknown; this is a coverage audit gap.
- No load tests for 200+ concurrent SSE clients.
- No authentication integration tests with real JWT tokens.

## See also

- [subsystem-cli.md](subsystem-cli.md) — CLI commands that call the HTTP API
- [../gaps-and-roadmap.md](../gaps-and-roadmap.md) — serve test count is a gap
