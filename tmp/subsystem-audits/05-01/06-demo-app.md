# 06 — Demo App Integration Issues

## MEDIUM: Workspace paths not portable

**File:** `demo/demo-app/src/lib/workflow-api.ts`

```typescript
GET /api/workflows/latest?root=/tmp/roko-ws-abc123def456
```

Absolute filesystem paths are passed as query parameters. Works locally but breaks when:
- Server runs on a different machine (Railway deployment)
- Docker container has different filesystem layout
- Multiple users share a server

**Fix:** Use workspace IDs instead of paths. Return ID from `POST /api/workspaces` and use that for workflow queries.

---

## MEDIUM: SSE stream has no heartbeat detection

**File:** `demo/demo-app/src/lib/workflow-api.ts`

SSE connection has exponential backoff on errors, but no heartbeat detection. If the server stops sending frames without closing the connection, the client hangs for 30-60s (browser TCP timeout) before detecting the failure.

**Fix:** Server should send periodic heartbeat frames. Client should track last-received timestamp and reconnect if stale > 10s.

---

## MEDIUM: WebSocket subscribe has no ACK

**File:** `demo/demo-app/src/lib/workflow-api.ts:224-228`

```typescript
ws.onopen = () => {
  patchStatus({ ws: 'live' });
  ws?.send(JSON.stringify({ type: 'subscribe', root, projections: [...] }));
};
```

Client marks connection as "live" on `onopen` before subscription is confirmed. If the server rejects the subscribe (e.g., root doesn't exist), client shows "connected" but never receives data.

---

## MEDIUM: Base64 file write validation missing

**File:** `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts:124`

```typescript
await handle.execCmd(
  `echo '${b64}' | base64 -D > '${path}' 2>/dev/null || echo '${b64}' | base64 -d > '${path}'`,
  5000,
);
```

Both `base64 -D` (macOS) and `base64 -d` (Linux) can fail silently because stderr is redirected to `/dev/null`. The `||` fallback masks errors. If both fail, the file is empty or missing, and subsequent pipeline steps fail with cryptic errors.

---

## LOW: Hard-coded port 6677

**File:** `demo/demo-app/src/lib/serve-url.ts`

```typescript
if (port === '6677' && preferSameOrigin) return '';
return `http://${hostname}:6677`;
```

Always assumes roko-serve is on port 6677. Vite proxy config also hardcodes this. Should be configurable via `VITE_SERVE_URL` env var.

---

## LOW: Terminal prompt detection fragile for custom shells

**File:** `demo/demo-app/src/hooks/useTerminal.ts:12`

```typescript
const PROMPT_RE = /(?:^|\n)[^\n]*[❯%#>→➜➤›]\s*$|(?:^|\n)\$\s+$/;
```

Works for common shells but fails on:
- Custom Zsh themes with unusual prompt characters
- Fish shell with non-standard prompts
- Windows Git Bash

When prompt detection fails, `waitForPrompt()` times out and commands appear to hang.

---

## LOW: 15s config poll interval is slow

**File:** `demo/demo-app/src/hooks/useRokoConfig.ts:128`

Config is polled every 15s. If user changes roko.toml via CLI, the UI takes up to 15s to reflect the change. Should use SSE push from the server's `config_reloaded` event.
