# Demo App + roko-serve Audit & Redesign Plan

## Part 1: Issues Found

### A. roko-serve: Why It Hangs (16 issues)

#### Blocking Startup (server won't start if conditions aren't right)

| # | Issue | File | Severity |
|---|-------|------|----------|
| S1 | `AlloyChainClient::http(rpc_url)` called synchronously — blocks forever if RPC unreachable | `state.rs:580-620` | CRITICAL |
| S2 | Chain-watcher subprocess awaits `status()` — hangs if binary missing | `lib.rs:308-346` | HIGH |
| S3 | `block_in_place()` for CascadeRouter loading — blocks tokio runtime | `lib.rs:814-816` | MEDIUM |
| S4 | `StateHub::bootstrap_from_workdir()` does sync I/O — hangs with large `.roko/` logs | `lib.rs:830` | HIGH |
| S5 | JWKS prime has 10s timeout per HTTP request — adds startup latency if Privy is down | `lib.rs:274-280` | MEDIUM |

#### Runtime Resource Leaks (server degrades over time)

| # | Issue | File | Severity |
|---|-------|------|----------|
| S6 | Terminal PTY uses `thread::spawn()` per WebSocket — unbounded OS thread creation | `terminal.rs:539-551` | CRITICAL |
| S7 | `SessionManager` holds `parking_lot::Mutex` during PTY `write_all()`+`flush()` — all terminal ops block each other | `terminal.rs:341-349` | HIGH |
| S8 | `ephemeral_workspaces` HashMap — no size limit, GC only every 5min for entries >1hr | `state.rs:434` | HIGH |
| S9 | `aggregator_cache` — no size limit or LRU eviction | `state.rs:400` | MEDIUM |
| S10 | `discovered_agents` — entries never removed, grows unbounded | `state.rs:398` | MEDIUM |
| S11 | `heartbeats` VecDeque — no `.pop_front()`, grows unbounded | `state.rs:402` | HIGH |
| S12 | `active_plans`, `active_runs` — never cleaned up if task crashes | `state.rs:381-383` | HIGH |
| S13 | `gateway_model_counters` — new entry per unique model slug, unbounded | `state.rs:422` | MEDIUM |
| S14 | `batch_progress` — entries never removed | `state.rs:424` | MEDIUM |

#### Performance Bottlenecks

| # | Issue | File | Severity |
|---|-------|------|----------|
| S15 | Health endpoint acquires 3 RwLocks sequentially, iterates all maps | `health.rs:15-64` | HIGH |
| S16 | Auth middleware iterates ALL discovered agents per request (O(n)) | `middleware.rs:219-246` | CRITICAL |

#### Additional Structural Issues

| # | Issue | Details |
|---|-------|---------|
| S17 | 34 fields in AppState, 13 RwLocks — no contention strategy | State is a god object |
| S18 | 3 parallel event systems (ServerEvent, StateHub, RuntimeEvent) — hard to reason about ordering | Event fragmentation |
| S19 | Gateway inference has NO timeout — can hang indefinitely | `gateway.rs` |
| S20 | Agent sidecar proxy has no timeout or circuit breaker | `agents.rs` |
| S21 | Workspace prefix has no validation — `../` path traversal possible | `workspaces.rs:74` |
| S22 | No global request timeout middleware | Missing tower layer |

---

### B. Demo App: Terminal Layer (14 issues)

| # | Issue | File | Impact |
|---|-------|------|--------|
| T1 | `execCmd()` sends `${cmd}; echo __RKxxx__\r` — markers visible in terminal | `useTerminal.ts:223-228` | User sees `__RK1mom3s62j__` jargon |
| T2 | `outBuf` cleared at start of every `execCmd()`/`showCmd()` — drops concurrent WebSocket data | `useTerminal.ts:225`, `terminal-session.ts:169` | Race condition loses output |
| T3 | Shell "connected" ≠ shell ready — WS opens before PTY has printed first prompt | `terminal-session.ts:86-100` | Commands sent to unready shell |
| T4 | Prompt detection: arbitrary 80ms delay + 120ms stability check + broad regex | `useTerminal.ts:194-212` | False positives/negatives |
| T5 | Prompt regex matches `%`, `#`, `>`, `$`, `→`, `❯` etc — any command output containing these triggers false match | `useTerminal.ts:12` | Commands appear to complete prematurely |
| T6 | `resolveRoko()` cache is global, `resetRokoResolution()` never called between scenarios | `terminal-session.ts:39-75` | Wrong binary in second scenario |
| T7 | Gate detection regex too broad — `test ok` anywhere matches, not just gate output | `terminal-session.ts:235-250` | False positive gates |
| T8 | Workspace `ready` field returned by API but never checked | `ScenarioSlot.tsx:725` | Commands run in non-existent dir |
| T9 | Abort signal not consistently propagated — `typeChars()` ignores abort entirely | `ScenarioSlot.tsx:661` | Reset doesn't stop scenarios |
| T10 | When served from `:6677`, `SERVE_URL=''` and `WS_BASE='ws'` — WebSocket URLs become relative `ws/ws/terminal/...` | `serve-url.ts:2-14` | WS connections fail |
| T11 | Three redundant health check systems with no coordination | `useServerHealth`, `bootstrapTransport`, `useLiveApi` | Stale OFFLINE indicator |
| T12 | 5-second health poll interval with 2s timeout — no manual re-check trigger | `useServerHealth.ts:13-31` | Up to 5s stale after serve starts |
| T13 | `showCmd()` types char-by-char (6-9ms/char) — 80-char command takes ~600ms to type | `terminal-session.ts:122-135` | Slow demo pacing |
| T14 | No explicit terminal cleanup on scenario abort — timers, intervals, async tasks left hanging | Multiple files | Resource leaks |

---

### C. Scenario Runners (13 scenarios audited)

#### Per-Scenario Issues

| Scenario | Panes | Critical Issue |
|----------|-------|----------------|
| **prd-pipeline** | 1 | Depends on serve workflow SSE/WS — fails if serve is unhealthy. `base64 -D` (macOS) vs `-d` (Linux) platform mismatch. |
| **prd-research-loop** | 1 | 180s timeout for research agent — silently times out. Cost/token metrics hardcoded, not real. |
| **gate-retry** | 2 | Gate pattern ambiguity — "compile failed on test foo" matches both compile and test gates. |
| **knowledge-accumulation** | 2 | Metrics hardcoded (`$0.02 / 1.2k → $0.08 / 3.8k`), not extracted from output. |
| **knowledge-transfer** | 2 | Cross-workspace `cp -r .roko/neuro` may fail if dirs don't exist. No wait for knowledge distillation. |
| **dream-consolidation** | 2 | Hardcoded phase timings (1500ms hypnagogia, 1200ms NREM/REM) — gates set to "pass" before actual completion. |
| **race** | 2 | If both runs fail, no winner detection. Timeout asymmetry between panes. |
| **provider-race** | 4 | `finishOrder[0] ?? 0` crashes if all providers fail. Missing API keys fail silently. |
| **providers** | 4 | No gate detection at all. Missing API keys produce silent failure with "pending" indefinitely. |
| **explore** | 4 | Signal abort is per-pane, not per-command — 1-2 commands run after abort. |
| **chat** | 1 | Chat prompt regex `/❯\|roko>\|\/help\|model\|chat/i` too loose — false positives on log output. |
| **chain-intelligence** | 2 | Requires mirage-rs at `:8545` + `cast` CLI. Hardcoded wallet addresses. Knowledge graph timing issues. |
| **mirage** | 1 | Does nothing — just waits for WS + prompt. No actual demo content. |

#### Cross-Cutting Issues

| # | Issue | Affected Scenarios |
|---|-------|--------------------|
| C1 | Only prd-pipeline uses serve APIs (SSE/WS). All others rely on CLI output regex for gate/cost/token detection. | All except prd-pipeline |
| C2 | No scenario validates that required CLI subcommands exist before running. | All |
| C3 | Hardcoded metrics in several scenarios — demo shows fake data. | knowledge-accumulation, dream-consolidation |
| C4 | No scenario checks for required external services (mirage-rs, API keys, providers). | chain-intelligence, providers, provider-race |
| C5 | `trackMetrics()` interval must be manually cleared — forgetting causes memory leak. | race, gate-retry, knowledge-transfer |
| C6 | Multi-pane scenarios have no coordination between panes — timing is "best effort". | knowledge-transfer, dream-consolidation, race |

---

## Part 2: Full Redesign Plan

### 2.1 roko-serve Redesign

#### Startup: Async + Timeout Everything

```
Current:  ChainClient::http(rpc) [BLOCKS] → JWKS::prime() [10s] → StateHub::bootstrap() [BLOCKS] → bind()
Proposed: bind() FIRST → spawn { ChainClient with 3s timeout } → spawn { JWKS with 5s timeout } → spawn { StateHub async load }
```

- Start listening IMMEDIATELY on `:6677` — return health as `{ status: "starting" }` until warm
- All external I/O (chain, JWKS, state loading) runs in background tasks with timeouts
- Health endpoint uses atomic counters, never acquires locks

#### State: Break Up the God Object

Replace the 34-field AppState with focused sub-states:

```
AppState {
  core: CoreState,          // config, layout, cancel, started_at
  agents: AgentState,       // supervisor, discovered, fleet
  inference: InferenceState, // cascade_router, model_counters, gateway
  execution: ExecState,     // active_plans, active_runs, operations
  knowledge: KnowledgeState, // neuro store, dreams, episodes
  terminal: TerminalState,  // session manager (isolated)
  events: EventState,       // unified event bus + SSE
}
```

Each sub-state owns its own locking strategy. No cross-state locks held simultaneously.

#### Terminal: Fix the Thread Leak

```
Current:  thread::spawn(blocking PTY read) → mpsc::blocking_send → WebSocket
Proposed: tokio::spawn_blocking(PTY read) → tokio::mpsc → WebSocket
          + per-session lock (not global mutex)
          + session TTL with auto-cleanup
          + read timeout on PTY (kill stale sessions)
```

#### Collections: Bounded + Auto-Clean

| Collection | Current | Proposed |
|---|---|---|
| `ephemeral_workspaces` | HashMap, unbounded | LRU cache, max 100, 30min TTL |
| `aggregator_cache` | HashMap, unbounded | LRU cache, max 500, 60s TTL |
| `discovered_agents` | HashMap, unbounded | HashMap with 1hr TTL per entry |
| `heartbeats` | VecDeque, unbounded | VecDeque, max 1000, auto-trim |
| `active_plans/runs` | HashMap, unbounded | HashMap with 24hr TTL, background GC |
| `gateway_model_counters` | HashMap, unbounded | HashMap, max 50 models |
| `batch_progress` | HashMap, unbounded | HashMap with 1hr TTL |

#### Auth: O(1) Lookup

```
Current:  for agent in discovered_agents.values() { if token_hash matches... }  // O(n)
Proposed: token_index: HashMap<String, String>  // token_hash → agent_id, O(1)
```

#### Health: Atomic Counters

```
Current:  health() { plans.read().len() + runs.read().len() + agents.read().len() }  // 3 locks
Proposed: health() { plan_count.load() + run_count.load() + agent_count.load() }  // 0 locks
```

#### Global Timeout

Add `tower::timeout::TimeoutLayer` at the router level:
- Default: 30s for all routes
- Override: 120s for `/api/inference/*`, `/api/plans/*/execute`
- Terminal WS: no timeout (long-lived connection)

---

### 2.2 Demo Terminal Layer Redesign

#### Replace Marker-Based Detection with OSC Sideband

**Current flow:**
```
execCmd("ls") → sendRaw("ls; echo __RK1__\r") → waitForMarker("__RK1__")
                                                   ↓
                                        marker visible in terminal
```

**Proposed flow using OSC (Operating System Command) escape sequences:**
```
execCmd("ls") → sendRaw("ls; printf '\\033]133;C;%s\\033\\\\' $?\r") → waitForOSC()
                                                                         ↓
                                                              invisible to terminal
                                                              (xterm.js swallows OSC)
```

OSC 133 is the shell integration protocol used by iTerm2, VS Code, WezTerm. The terminal emulator processes it as metadata, never displays it.

**Implementation in two parts:**

1. **Backend (`terminal.rs`):** No changes needed — PTY passes all bytes through.

2. **Frontend (`useTerminal.ts`):**
```typescript
// Replace execCmd:
handle.execCmd = async (cmd: string, timeout = 30000): Promise<{ ok: boolean; exitCode: number }> => {
  const seq = ++execSeq;
  const oscMarker = `roko-cmd-${seq}`;

  // Wrap command: run it, capture exit code, emit OSC 133 with exit code
  const wrapped = `${cmd}; __ec=$?; printf '\\033]133;D;%d;${oscMarker}\\033\\\\' $__ec; (exit $__ec)`;

  handle.sendRaw(wrapped + '\r');

  return new Promise((resolve) => {
    const handler = (exitCode: number, marker: string) => {
      if (marker === oscMarker) {
        term.off('osc-133-d', handler);
        resolve({ ok: exitCode === 0, exitCode });
      }
    };
    term.on('osc-133-d', handler);
    setTimeout(() => { term.off('osc-133-d', handler); resolve({ ok: false, exitCode: -1 }); }, timeout);
  });
};
```

3. **xterm.js OSC handler:**
```typescript
// Register custom OSC handler for sequence 133
term.parser.registerOscHandler(133, (data) => {
  // Parse: "D;<exit_code>;<marker>"
  const parts = data.split(';');
  if (parts[0] === 'D' && parts.length >= 3) {
    const exitCode = parseInt(parts[1], 10);
    const marker = parts[2];
    term.emit('osc-133-d', exitCode, marker);
  }
  return true; // swallow — don't display
});
```

**Benefits:**
- Zero visible markers in terminal output
- Exit code captured (not just "did marker appear")
- Standard protocol — compatible with all terminal emulators
- No output buffer clearing needed

#### Proper Shell Readiness Detection

```
Current:  WS.onopen → status='connected' → scenario starts → commands may fail
Proposed: WS.onopen → send PS1 detection prompt → wait for first OSC 133;A → status='ready'
```

Use OSC 133;A (prompt start) and 133;B (command start) to track shell state machine:

```
CONNECTING → WS_OPEN → SHELL_INITIALIZING → PROMPT_READY → EXECUTING → PROMPT_READY → ...
```

#### Single Health Check System

Replace the three redundant systems with one:

```typescript
const serveHealth = createHealthMonitor({
  url: `${ABSOLUTE_SERVE_URL}/health`,
  pollInterval: 3000,
  timeout: 2000,
  onStatusChange: (status) => { /* update all consumers */ },
  // Immediate check on user action (play button)
  checkNow: () => Promise<boolean>,
});
```

---

### 2.3 Scenario Runtime Redesign

#### Formal State Machine

Replace ad-hoc polling with explicit state transitions:

```typescript
interface ScenarioStep {
  id: string;
  label: string;
  command: string | ((ctx: StepContext) => string);
  mode: 'exec' | 'type' | 'interactive';  // exec=hidden, type=visible, interactive=chat
  timeout: number;
  gate?: { name: string; detect: (output: string) => 'pass' | 'fail' | null };
  onComplete?: (result: StepResult) => void;
  requires?: string[];  // step IDs that must complete first
}

// State machine: PENDING → RUNNING → WAITING_PROMPT → DETECTING → COMPLETE | FAILED
type StepState = 'pending' | 'running' | 'waiting' | 'detecting' | 'complete' | 'failed';
```

#### Structured Gate Detection

Instead of regex on raw terminal output, use roko CLI's structured output:

```bash
roko run "..." --output-format json 2>/dev/null
# Outputs: {"gates":{"compile":"pass","test":"pass","clippy":"pass"},"cost":"$0.03","tokens":1847}
```

If structured output isn't available, at minimum use dedicated gate markers:

```bash
roko run "..." 2>&1 | tee /dev/stderr | grep -o 'GATE:[a-z]*:[a-z]*'
# Outputs: GATE:compile:pass\nGATE:test:pass\nGATE:clippy:fail
```

#### Pre-Flight Checks

Before running any scenario, validate requirements:

```typescript
async function preflight(scenario: Scenario): Promise<PreflightResult> {
  return {
    serveHealthy: await checkServeHealth(),
    rokoBinary: await resolveRokoBinary(),
    requiredProviders: await checkProviders(scenario.requiredProviders),
    externalServices: await checkServices(scenario.externalServices), // mirage-rs, etc.
    workspaceCreatable: await testWorkspaceCreation(),
  };
}
```

#### Per-Command Output Isolation

Instead of one global `outBuf` cleared on every command:

```typescript
interface CommandExecution {
  id: string;
  command: string;
  startOffset: number;  // offset in global buffer when command started
  endOffset: number;    // offset when command completed
  output(): string;     // slice of global buffer between start and end
  exitCode: number;
}
```

The global buffer never clears — commands just track their window into it.

---

### 2.4 Priority Order

| Priority | What | Why | Effort |
|----------|------|-----|--------|
| P0 | Fix roko-serve startup to not block on chain/JWKS | Server literally won't start | Small |
| P0 | OSC sideband for terminal markers | User-facing visual garbage | Medium |
| P0 | Fix `SERVE_URL`/`WS_BASE` URL resolution | WS connections fail entirely | Small |
| P1 | Single health check + immediate re-check | SERVE OFFLINE indicator wrong | Small |
| P1 | Shell readiness state machine | Commands sent to unready shell | Medium |
| P1 | Bounded collections + auto-cleanup in serve | Server degrades over time | Medium |
| P2 | Structured gate detection | False positive/negative gates | Medium |
| P2 | Per-command output isolation | Race conditions in detection | Medium |
| P2 | Break up AppState god object | Maintainability, lock contention | Large |
| P2 | Proper abort/cleanup in scenarios | Reset doesn't stop running demos | Medium |
| P3 | Pre-flight checks for scenarios | Silent failures on missing deps | Small |
| P3 | Global request timeout middleware | Unbounded handler hangs | Small |
| P3 | Auth O(1) lookup | Performance under load | Small |
