# 26 — Terminal & Demo Truth (T5-41 expanded)

The demo automation in `demo/demo-app/src/lib/scenario-runners/` uses
regex matching on PTY output to detect command success ("if the prompt
is back, the command finished"). This is fragile and silently fails
when prompt formats change.

The terminal subsystem now emits **typed `CommandEvent` lifecycle
events** (R9 in the agent-packet ledger). The scenario runners must
subscribe to these instead of scraping output.

Source: doc 36 § Terminal security and lifecycle, doc 41 T5-41.

---

## Today's State (verified 2026-05-01)

- `CommandEvent` DTOs exist in `crates/roko-serve/src/command_events.rs`:
  `Started`, `Output`, `Exited`, `SpawnFailed`, `Cancelled`.
- One terminal lifecycle path emits the typed events (R9).
- All terminal/WebSocket consumers and demo UI are not migrated.
- `demo/demo-app/src/lib/scenario-runners/*.ts` use regex prompt matching.
- A WebSocket `/api/terminal/sessions/{id}/events` endpoint emits raw
  PTY bytes (not typed events).

---

## Anti-Patterns

1. **No regex prompt scraping.** Period.
2. **No "wait for output to look idle" heuristics.** Use `Exited` event.
3. **No leaked PTY processes / `ZDOTDIR` temp dirs on cancel.** Lifecycle
   events guarantee `Cancelled` fires; clean up there.
4. **No "reconnect resets terminal state" hack.** Use a typed lifecycle
   state and refuse to attach an old generation to a new socket.
5. **No polling on terminal state.** Subscribe to events.
6. **No frontend logic that infers exit status.** Use `Exited.code`.

---

## Plan

### Phase 1: Add a typed event WebSocket endpoint

**File**: `crates/roko-serve/src/terminal/events_ws.rs` (new)

Add a separate WebSocket endpoint **distinct from the IO endpoint**:

```rust
GET /api/terminal/sessions/{id}/events
```

The IO endpoint (`/io`) carries raw PTY bytes (for the user to see in a
terminal renderer). The events endpoint (`/events`) carries typed
`CommandEvent`s for automation.

#### Implementation

```rust
async fn terminal_events_handler(
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.max_message_size(64 * 1024)
        .on_upgrade(move |socket| handle_events(socket, state, session_id))
}

async fn handle_events(socket: WebSocket, state: Arc<AppState>, session_id: String) {
    let (mut tx, _rx) = socket.split();
    let session = match state.terminal_sessions.get(&session_id).await {
        Some(s) => s,
        None => {
            let _ = tx.send(Message::Text(serde_json::to_string(&CommandEvent::SpawnFailed {
                reason: "session not found".into(),
            }).unwrap())).await;
            return;
        }
    };

    let mut events = session.subscribe_events();
    while let Some(event) = events.recv().await {
        let json = serde_json::to_string(&event).unwrap();
        if tx.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}
```

#### Wire into `routes/mod.rs`

```rust
.route("/api/terminal/sessions/:id/events", get(terminal_events_handler))
```

#### Authentication

Same auth as the IO WebSocket. Apply existing `require_api_key`
middleware.

**Estimated effort**: 4-6 hours.

---

### Phase 2: Migrate one demo scenario runner

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`
(or whichever scenario is the smallest).

#### Step 1: Add TypeScript types

In `demo/demo-app/src/lib/terminal-events.ts` (new):

```typescript
export type CommandEvent =
  | { type: 'started'; session_id: string; pid: number; command: string }
  | { type: 'output'; session_id: string; bytes: string }
  | { type: 'exited'; session_id: string; code: number | null; duration_ms: number }
  | { type: 'spawn_failed'; reason: string }
  | { type: 'cancelled'; session_id: string };
```

(Match exactly the field names emitted by Rust's `serde` for
`CommandEvent`.)

#### Step 2: Replace polling with subscription

Today (regex):

```typescript
// scenario-runners/prd-pipeline.ts
async function runStep(prompt: string) {
    await terminal.sendText(prompt);
    while (true) {
        const output = await terminal.readUntilTimeout(500);
        if (output.match(/\$ $/)) break;  // PROMPT-SCRAPING ANTI-PATTERN
    }
}
```

Replace with:

```typescript
async function runStep(sessionId: string, command: string): Promise<{ code: number; output: string }> {
    return new Promise((resolve, reject) => {
        const ws = new WebSocket(`${apiBase}/api/terminal/sessions/${sessionId}/events`);
        let buffer = '';

        ws.onmessage = (msg) => {
            const event: CommandEvent = JSON.parse(msg.data);
            switch (event.type) {
                case 'started':
                    // already happened; we sent the command before opening
                    break;
                case 'output':
                    buffer += event.bytes;
                    break;
                case 'exited':
                    ws.close();
                    resolve({ code: event.code ?? 0, output: buffer });
                    break;
                case 'spawn_failed':
                case 'cancelled':
                    ws.close();
                    reject(new Error(event.type === 'spawn_failed' ? event.reason : 'cancelled'));
                    break;
            }
        };

        ws.onerror = (e) => reject(e);
        sendCommand(sessionId, command);
    });
}
```

#### Step 3: Use exit code, not regex

```typescript
const result = await runStep(sessionId, 'roko prd plan auth-redesign');
if (result.code === 0) {
    setStatus('plan generated');
} else {
    setStatus(`plan failed (exit ${result.code})`);
    console.error(result.output);
}
```

#### Step 4: Tests

If the demo app has a Playwright suite, add a regression test:

```typescript
test('scenario succeeds when command exits 0', async ({ page }) => {
    // Mock the events WebSocket to emit started → output → exited(0)
    // ...
    await page.click('[data-test="run-prd"]');
    await expect(page.locator('[data-test="status"]')).toHaveText('plan generated');
});
```

**Estimated effort**: 4-6 hours per scenario (4 scenarios → 16-24 hours).

---

### Phase 3: Migrate remaining scenarios

Repeat Phase 2 for:

- `scenario-runners/knowledge-transfer.ts`
- Any other scenarios under `scenario-runners/`

Each scenario is one commit.

---

### Phase 4: Frontend hooks and components

**File**: `demo/demo-app/src/hooks/useTerminal.ts`

Today the hook reads raw text. After migration:

```typescript
export function useTerminalEvents(sessionId: string | null) {
    const [events, setEvents] = useState<CommandEvent[]>([]);
    const [exitCode, setExitCode] = useState<number | null>(null);
    const [running, setRunning] = useState(false);

    useEffect(() => {
        if (!sessionId) return;
        const ws = new WebSocket(`/api/terminal/sessions/${sessionId}/events`);
        ws.onmessage = (msg) => {
            const event: CommandEvent = JSON.parse(msg.data);
            setEvents(prev => [...prev, event]);
            if (event.type === 'started') setRunning(true);
            if (event.type === 'exited') {
                setRunning(false);
                setExitCode(event.code);
            }
            if (event.type === 'spawn_failed' || event.type === 'cancelled') {
                setRunning(false);
            }
        };
        return () => ws.close();
    }, [sessionId]);

    return { events, exitCode, running };
}
```

Components consume this hook instead of polling.

**Estimated effort**: 2-3 hours.

---

### Phase 5: Terminal lifecycle hardening

These are mentioned in doc 36 § terminal lifecycle and doc 35 § terminal:

#### A. Spawn failure must close WebSocket with typed error

If the PTY spawn fails, emit `SpawnFailed { reason }` and close. Today
some paths leave a half-open connection.

**File**: `crates/roko-serve/src/terminal/...`

#### B. Cleanup on cancel

The temporary `ZDOTDIR` directory is deleted on normal exit; ensure it's
also deleted on `Cancelled` (Ctrl-C) and `SpawnFailed`.

#### C. Generation counter for reconnect safety

Each terminal session has a generation counter. Reconnecting to an old
generation is rejected — the client must request a new session if the
generation changed.

```rust
pub struct TerminalSession {
    pub id: String,
    pub generation: u64,
    // ...
}

// On reconnect
if reconnect_request.generation != session.generation {
    return Err(TerminalError::StaleGeneration);
}
```

#### D. Per-session auth

Today, anyone with a valid API key can attach to any session ID. Add
**owner** field on the session and enforce in the WebSocket handler.

#### E. Rate limit session create / resize / send

5 sessions/min/client (T3-23 covers this).

**Estimated effort**: 6-10 hours total.

---

## Combined Verification

```bash
# Backend
cargo test -p roko-serve terminal --lib
cargo test -p roko-serve command_events --lib
rg 'CommandEvent::(Started|Output|Exited|SpawnFailed|Cancelled)' crates/roko-serve/

# Frontend
cd demo/demo-app
yarn typecheck
yarn lint
yarn test
rg '\.match\(/\$' src/lib/scenario-runners/   # 0 matches (no prompt scraping)
rg 'CommandEvent' src/lib/   # used in scenario runners and hooks

# Manual test
yarn dev
# Run a scenario; confirm status updates come from typed events, not output text
```

---

## Status

- [ ] Phase 1 — Typed event WebSocket endpoint
- [ ] Phase 2 — Migrate first scenario runner
- [ ] Phase 3 — Migrate remaining scenarios
- [ ] Phase 4 — Frontend hooks
- [ ] Phase 5 — Terminal lifecycle hardening (5 sub-tasks)

**Estimated total effort**: 30-50 hours, of which most is the per-scenario
migration in Phase 2-3.
