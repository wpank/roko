# Demo Architecture

## Page Structure

Single HTML page served at `/demo/` (replaces index.html, terminal.html, builder.html).

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ◆ roko                              [scenario tabs]         [controls] │
├──────────────────────────────────────────────────┬──────────────────────┤
│                                                  │                      │
│                                                  │  METRICS             │
│                                                  │  ┌────────────────┐  │
│              TERMINAL(S)                         │  │ cost: $0.042   │  │
│              xterm.js panes                      │  │ tokens: 4.2k   │  │
│              (1, 2, or 4 split)                  │  │ model: glm-5.1 │  │
│                                                  │  │ savings: 3.2x  │  │
│                                                  │  └────────────────┘  │
│                                                  │                      │
│                                                  │  GATES               │
│                                                  │  ✔ compile           │
│                                                  │  ✔ test              │
│                                                  │  ⏳ clippy            │
│                                                  │                      │
│                                                  │  FILES               │
│                                                  │  📦 Cargo.toml       │
│                                                  │  🦀 src/main.rs      │
│                                                  │  🦀 src/lib.rs       │
│                                                  │                      │
├──────────────────────────────────────────────────┴──────────────────────┤
│  ❯ [prompt input]                                          [Build]      │
├─────────────────────────────────────────────────────────────────────────┤
│  $0.042 · 4.2k tok · glm-5.1 · 12.3s · ▸ play ⏸ pause ↺ reset       │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Tree

```
DemoPage
├── TopBar
│   ├── Logo ("◆ roko")
│   ├── ScenarioTabs (self-hosting | builder | race | fleet | compounding)
│   └── Controls (play/pause/reset/speed)
├── Main (flex row)
│   ├── TerminalArea (flex: 1)
│   │   ├── TerminalPane[0..N] (xterm.js + WebSocket)
│   │   └── TerminalGrid (1/2/4 column layout)
│   └── SidePanel (280px fixed)
│       ├── MetricsPanel (cost, tokens, model, savings)
│       ├── GatesPanel (compile/test/clippy status)
│       ├── FilesPanel (detected file tree)
│       └── EventLog (scrolling event stream)
├── PromptBar (visible in builder scenario)
│   ├── PromptInput
│   └── BuildButton
└── StatusBar
    ├── CostTicker ($X.XXXX with digit-slide animation)
    ├── TokenCount
    ├── ModelName
    ├── ElapsedTime
    └── PlaybackControls
```

## Data Flow

```
User clicks scenario tab
  → loadScenario(id)
    → destroy existing terminals
    → create N terminal panes per scenario config
    → connect each to WebSocket PTY
    → resolve roko binary path (once, cached)
    → run commands sequentially (wait for prompt between each)
    → watch output for:
        - gate results (compile/test/clippy ✔/✖)
        - file creation (Write tool, .rs files)
        - cost data (token counts, model name)
        - completion markers (done/error)
    → update side panel in real-time
```

## Terminal Management

Each terminal pane wraps:
- `xterm.js` Terminal instance
- `FitAddon` for responsive sizing
- WebSocket connection to `/ws/terminal/{sessionId}`
- Output buffer (2KB ring) for prompt detection
- Command queue (FIFO, sequential execution)

### Prompt Detection

```javascript
// Wait for shell prompt before sending next command
const PROMPT_RE = /[❯\$>%#]\s*$/;

async function waitForPrompt(pane, timeoutMs) {
  pane.outputBuffer = '';
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    await sleep(250);
    if (PROMPT_RE.test(pane.outputBuffer)) return true;
  }
  return false; // timed out
}
```

### Binary Resolution

Run once at page load, cache result:
```javascript
async function resolveRoko(pane) {
  // 1. Check PATH
  // 2. Check ./target/release/roko
  // 3. Check ./target/debug/roko
  // 4. Fall back to 'roko'
}
```

## Scenario System

Each scenario is a config object:

```javascript
const SCENARIOS = {
  'self-hosting': {
    label: 'Self-Hosting Workflow',
    description: 'PRD → plan → execute → validate',
    panes: 1,
    showPromptBar: false,
    showSidePanel: true,
    commands: (R, DIR) => [
      `cd ${DIR} && ${R} init`,
      `cd ${DIR} && ${R} prd idea "Wire SystemPromptBuilder into orchestrate.rs"`,
      `cd ${DIR} && ${R} prd draft new "system-prompt-wiring"`,
      `cd ${DIR} && ${R} status`,
    ],
  },
  'builder': {
    label: 'Builder',
    description: 'Type a request, roko builds it live',
    panes: 1,
    showPromptBar: true,
    showSidePanel: true,
    commands: null, // dynamic from prompt input
  },
  'race': {
    label: 'The Race',
    description: 'Side-by-side: stock LLM vs roko',
    panes: 2,
    showPromptBar: false,
    showSidePanel: true,
    // left pane: stock, right pane: roko
  },
  'multi-command': {
    label: 'Command Showcase',
    description: '4 commands running in parallel',
    panes: 4,
    showPromptBar: false,
    showSidePanel: false,
    commands: (R, DIR) => [
      // Each array = one pane
      [`cd ${DIR} && ${R} init && ${R} status`],
      [`cd ${DIR} && ${R} init && ${R} learn all`],
      [`cd ${DIR} && ${R} init && ${R} agent list`],
      [`cd ${DIR} && ${R} init && ${R} doctor`],
    ],
  },
};
```

## Server Requirements

- `roko serve` must be running on `:6677`
- PTY session endpoints: `POST/GET/DELETE /api/terminal/sessions`, `GET /ws/terminal/{id}`
- Health check: `GET /health`
- Static files served from `demo/demo-web/` at `/demo/`
