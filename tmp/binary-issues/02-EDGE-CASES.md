# Edge Cases & Combinations

## Every workflow combination that currently exists

### Chat workflows (currently broken/confusing)

| What user wants | Commands needed today | What breaks | Fixed version |
|---|---|---|---|
| Chat with default agent | `roko serve` (T1) + `roko agent chat` (T2) | Needs 2 terminals, provider config | `roko` |
| Chat with specific agent | `roko agent serve --agent-id X` (T1) + `roko agent chat --agent X` (T2) | Port discovery, sidecar config | `roko` → `/agent X` |
| Chat without serve | `roko agent serve --agent-id X` (T1) + `roko agent chat --agent X` (T2) | Sidecar must be running | `roko` (in-process) |
| Chat via serve fallback | `roko serve` (T1) + `roko agent chat` (T2) | 502 errors, routing config | `roko` (in-process) |

### Run workflows (mostly working)

| What user wants | Commands needed | Issues | Fixed |
|---|---|---|---|
| One-shot task | `roko run "prompt"` | Works but plain output if not TTY | `roko "prompt"` |
| One-shot with share | `roko run "prompt" --share` | Needs gh CLI, starts serve | Same but auto-detect gh |
| One-shot with web | `roko run "prompt" --serve` | Must know the flag exists | Always available |

### Dashboard workflows

| What user wants | Commands needed | Issues | Fixed |
|---|---|---|---|
| See TUI dashboard | `roko dashboard` | Standalone, no live agent data | Ctrl+T from chat |
| Dashboard with live data | `roko serve --tui` | Different from `roko dashboard`! | Ctrl+T (connected to serve) |
| Dashboard + chat | Not possible in one terminal | Two separate UIs | Ctrl+T toggles |

### Plan workflows (working but disconnected)

| What user wants | Commands needed | Issues | Fixed |
|---|---|---|---|
| Run a plan | `roko plan run plans/` | No live output unless --serve | In-process, events visible |
| Resume a plan | `roko plan run --resume-plan` or `roko resume` | Two ways to do same thing | `roko resume` |
| Generate + run | `roko prd plan X` then `roko plan run` | Two separate steps | `/plan generate+run` in chat |

### Serve workflows (confusing multiplicity)

| What user wants | Commands needed | Issues | Fixed |
|---|---|---|---|
| HTTP API for tools | `roko serve` | Just this, fine | `roko serve` (unchanged) |
| HTTP + TUI | `roko serve --tui` | Different from `roko dashboard` | Ctrl+T in `roko` |
| HTTP + agents | `roko up` | Hidden command, few know it exists | `roko` does this |
| HTTP + chat | `roko serve` (T1) + `roko agent chat` (T2) | Two terminals | `roko` does both |

## Auth edge cases

| Scenario | Current behavior | Fixed behavior |
|---|---|---|
| No auth at all | `error: Missing API key` | Friendly prompt with options |
| claude CLI logged in | Works only with `command = "claude"` in config | Auto-detected, no config needed |
| ANTHROPIC_API_KEY set but no credits | `400 invalid_request_error` | Detect and suggest: "API key has no credits. Use `claude` CLI instead?" |
| ANTHROPIC_API_KEY set + claude logged in | Config determines which, often wrong | Auto-detect: try CLI first (free), fall back to API |
| Key in ~/.roko/.env but commented out | Silent failure | Parse warning: "ANTHROPIC_API_KEY is commented out in ~/.roko/.env" |
| Key in ~/.roko/.env with mangled name | Silent failure (happened: `#NTHROPIC_API_KEY`) | Fuzzy match warning |
| Multiple providers configured | Uses default_backend which may be wrong | Auto-detect working provider |
| Provider kind typo in config | `unknown variant "perplexity"` | Suggest: "did you mean `perplexity_api`?" |
| Missing `command` field for claude_cli | `Missing required config field: providers.*.command` | Auto-fill: default to "claude" |
| Missing `slug` field for models | `missing field slug` | Use model key as slug default |

## Config edge cases

| Scenario | Current behavior | Fixed behavior |
|---|---|---|
| No roko.toml | Most commands fail | Auto-create with defaults |
| roko.toml exists but empty | Parse errors | Treat empty as defaults |
| config_version mismatch | Warning on every command | Silent migration |
| schema_version mismatch | Warning on every command | Silent migration |
| Duplicate provider keys | TOML parse error | Clear error message |
| `[providers]` section empty | Confusing behavior | Works fine (auto-detect) |
| Provider configured but key not set | Runtime error deep in dispatch | Startup warning: "anthropic provider configured but ANTHROPIC_API_KEY not set" |

## Port/network edge cases

| Scenario | Current behavior | Fixed behavior |
|---|---|---|
| Port 6677 already in use | Serve crashes | Auto-pick next available port, print URL |
| Sidecar on random port | Written to agents.json, user can't find it | Not exposed to user (in-process) |
| Firewall blocks localhost | Serve starts but unreachable | Warning if health check fails |
| Multiple roko instances | Port conflict | Lock file or auto-port |
| Serve started but killed | Stale agents.json entries | Cleanup on startup |

## TUI edge cases

| Scenario | Current behavior | Fixed behavior |
|---|---|---|
| Terminal too narrow | Rendering breaks | Graceful degradation, min-width check |
| Terminal too small for TUI | Fullscreen TUI unusable | Stay in inline mode, warn |
| SSH session (no TTY features) | Raw mode may fail | Detect and use plain mode |
| tmux/screen | Usually works | Explicitly test and handle |
| NO_COLOR set | Colors disabled | Symbols still render (Unicode, no ANSI) |
| Non-UTF8 terminal | Symbols garbled | Detect and fall back to ASCII |
| Ctrl+C during streaming | Partial response | Clean interrupt, push partial to scrollback |
| Process crash | Terminal left in raw mode | Panic hook restores terminal |

## Chain/relay noise

| Scenario | Current behavior | Fixed behavior |
|---|---|---|
| mirage-rs not running | Relay reconnect logs every 30s | Only log once, then silent |
| Chain configured but not needed | Pheromone/insight logs flood stdout | Default log level: warn for chain modules |
| No chain config | Relay still tries to connect | Skip relay if no chain.rpc_url |
