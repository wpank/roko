# User UX: Running Agents

> **TL;DR**: Non-developer users (and developers acting as users)
> interact with Roko through four surfaces: CLI, TUI, Chat, and
> Web. Today CLI is dominant; TUI is wired; Chat exists; Web is
> via the HTTP control plane. This doc proposes a unified
> interaction model where all four surfaces expose the *same*
> underlying verbs over the *same* event stream, so muscle memory
> transfers freely. It also proposes "familiar-first" defaults
> modeled on Claude Code, where users get productive without
> reading docs. The goal: a user who types `roko` for the first
> time produces useful output in under 30 seconds.

### For first-time readers

Key terms used below:

- **Bus** — ephemeral message stream inside Roko (tool banners,
  token streams, gate results). See `03-bus-as-first-class.md`.
- **Engram** — durable record: an episode, plan, heuristic, PRD.
  See `02-engram-vs-pulse.md`.
- **Surface** — one of CLI / TUI / Chat / Web. Each one renders the
  same verbs over the same Bus/Engram data.
- **Heuristic** — a named, calibrated belief the agent applies. See
  `14-worldview-validation.md`.

## 1. The four surfaces

| Surface | Today | Gaps |
|---|---|---|
| **CLI** | `roko run`, `plan`, `prd`, etc. | Inconsistent flag names, some subcommands missing status output |
| **TUI** | `roko dashboard`, F1–F7 tabs | Read-mostly; needs more interactive actions |
| **Chat** | `roko chat --agent <id>` | Only routes to a single agent; no multi-agent or streaming polish |
| **Web** | `roko serve` exposes ~85 routes | No first-party UI — users must build their own |

The surfaces overlap conceptually but diverge in flag names and
defaults. A user who knows `roko run "..."` has no idea what the
equivalent is in the TUI.

## 2. One verb set, four renderings

Propose a canonical verb set exposed by every surface:

| Verb | Meaning |
|---|---|
| **ask** | Run a single-turn query |
| **plan** | Propose a plan without executing |
| **do** | Execute a plan or a single task |
| **watch** | Stream progress of active work |
| **inspect** | Drill into an episode, engram, or heuristic |
| **replay** | Re-run a prior episode, optionally with changes |
| **learn** | Browse / curate heuristics, playbooks, experiments |
| **tune** | Change configuration, thresholds, routing |
| **connect** | Add a plugin, MCP, credential |

Every surface has a rendering of each verb. The user learns the
verbs once; the interface is wherever they happen to be.

## 3. First-run experience

The current first-run experience:

```bash
$ cargo install roko-cli      # 5 minutes compiling
$ roko init                   # creates .roko/
$ roko run "..."              # works if you configured a model
```

Gaps: users who haven't set up Claude/OpenAI API keys get cryptic
errors. No guided onboarding. No "plugin check." No heuristic-commons
opt-in dialog.

Proposed replacement: `roko init` becomes interactive.

```
$ roko init
Welcome to Roko. Let's get you set up.

Which models would you like to use?
  [x] Claude (requires ANTHROPIC_API_KEY)
  [ ] OpenAI (requires OPENAI_API_KEY)
  [ ] Local Ollama (detected at http://localhost:11434)
  [ ] Codex / Cursor / Gemini / Perplexity

Where should your agent memory live?
  > ./.roko  (recommended; git-ignored automatically)

Would you like to start with a heuristic library?
  [x] Import the starter kit (20 calibrated heuristics)
  [ ] Start from scratch

Should Roko look for existing MCP servers?
  [x] Yes, auto-discover
  [ ] No, manual config only

Done. Try:
  roko ask "what's my first task?"
  roko dashboard
  roko plugin list

Docs: https://roko.dev/docs/getting-started
```

This is a 30-second interaction that sets a user up for success.
Current init is non-interactive and assumes knowledge the user
may not have.

### 3.1 Error recovery inside `roko init`

The init flow has to degrade gracefully. Expected failure modes and
recoveries:

```
$ roko init
...
Which models would you like to use?
  [x] Claude (requires ANTHROPIC_API_KEY)

Checking ANTHROPIC_API_KEY... not found in env.
  1) Paste a key now (stored in OS keychain)
  2) Open https://console.anthropic.com/account/keys
  3) Skip Claude, try another provider
  4) Configure later with `roko secret set anthropic.api_key`
  >
```

```
Detecting local models... Ollama not reachable at :11434.
  1) Start Ollama and retry
  2) Skip — continue without local fallback
  3) Install Ollama (opens docs)
  >
```

```
Auto-discovering MCP servers... found 2, 1 returned errors.
  - ok: code-intel (roko-mcp-code)
  - err: github-mcp (connection refused)

  r) Retry
  s) Skip errored servers
  d) Show diagnostic for github-mcp
  >
```

Rules for every prompt:

- Always offer a `skip / configure later` option; `roko init`
  should never be a dead end.
- Every failure carries a literal next command the user can run
  (`roko secret set ...`, `roko plugin doctor <id>`).
- Partial success is a first-class outcome: `roko.toml` is
  committed incrementally as each step succeeds, so a user who
  Ctrl-C's mid-flow resumes from where they stopped.
- Tenth-percentile networks: every remote check has a 5s timeout
  and a retry prompt; no step blocks for longer than that without
  a visible cancel.

See also `24-deployment-ux.md` §3 for how the same keys migrate
into server-shape secret stores later, and `28-cli-parity-familiar-workflows.md`
§3 on slash-command parity for these prompts inside chat/TUI.

## 4. CLI: consistency, conventions, colors

### 4.1 Flag conventions

Adopt a house style. Every subcommand:

- `--format {human|json|yaml}` with `human` default.
- `--quiet` and `--verbose` with predictable volume.
- `--no-color` respects `NO_COLOR` env.
- `--dry-run` wherever side effects happen.
- `--plan-file <path>` never positional; positional slots are
  reserved for subjects (the plan ID, the hash, the prompt).

### 4.2 Output shape

- Success paths: single-line summary → details behind `--verbose`.
- Errors: include a *"try:"* line with a remediation hint.
- Long-running: progress via carriage-return overwrite in TTY,
  one-line-per-event in pipes.
- Colors are semantic (green=success, red=error, yellow=warning),
  not decorative.

### 4.3 Help that teaches

```bash
$ roko help ask
USAGE: roko ask <prompt> [options]

Ask the agent to respond to a single prompt.

OPTIONS:
  --role <role>          Which role to use (researcher, implementer, ...)
  --model <model>        Override the model routing
  --stream               Stream the response as it's produced
  --save                 Persist to an episode (default: ephemeral)
  --context <path>       Include a file or directory as context

EXAMPLES:
  roko ask "what does this codebase do?"
  roko ask "fix the failing test" --role implementer --stream
  roko ask "summarize" --context README.md

RELATED:
  roko plan     Propose a plan
  roko do       Execute a plan
  roko watch    Watch an in-flight conversation
```

Every help page teaches the *next step*. RELATED lines are
load-bearing for discovery.

## 5. TUI: interactive, not just displays

Today's TUI is mostly read-only. The seven F-tabs display
episodes, plans, gates, etc. A modern TUI would let the user *act*.

Proposed interactions added to F-tabs:

- **Episodes tab**: `r` to replay a selected episode; `i` to inspect
  its heuristic citations; `/` to search.
- **Plans tab**: `x` to execute the selected plan; `e` to edit the
  plan's markdown; `p` to pause/resume.
- **Gates tab**: `t` to adjust thresholds interactively; `v` to
  view recent failures.
- **Heuristics tab** (new): `c` to challenge a heuristic (force
  recalibration); `r` to retire; `e` to edit.
- **C-factor tab** (new): visualizations with keyboard zoom.
- **Chat tab**: full-duplex chat with any running agent.

The TUI becomes a control surface, not just a dashboard. Every
action the CLI can do should have a TUI binding.

## 6. Chat: streaming, multi-agent, inline artifacts

Chat today is one-agent-at-a-time. Upgrades:

- **Multi-agent chat**: `@researcher` and `@implementer` as address
  prefixes route to different roles; you can see both responding.
- **Streaming with cursor**: response tokens appear live.
- **Inline artifacts**: when an agent produces a code block, the
  chat renders it as a fenced block with *apply*, *copy*, and
  *diff* affordances, not just text.
- **Slash commands**: `/plan`, `/run`, `/explain`, `/heuristics`,
  `/replay` available from within chat so you don't have to leave
  to take actions.
- **Attachments**: drop a file path onto the chat; it becomes
  context for the next turn.

Feels like Claude Desktop or Claude Code but native to Roko's
runtime and with access to Bus, Substrate, Heuristics.

## 7. Web: first-party UI, eventually

`roko serve` exposes the API. A first-party web UI shipped with it
(or in a sibling repo) would give non-CLI users a way in. Initial
scope:

- **Home**: current state of the agent runtime (what's running,
  how long, c-factor).
- **Ask**: a chat interface rendering in real-time.
- **Plans**: tree view of plans and tasks with DAG visualization.
- **Episodes**: searchable list with replay.
- **Heuristics**: the externalized-beliefs browser.
- **Settings**: visual configuration.

Must be a small number of pages — not a kitchen sink. Web UI
targeting completeness is a maintenance trap; targeting
discoverability is sustainable.

## 7.5 Power-user shortcuts

Once a user has done the basics three times, they're a power user
and every extra keystroke is a papercut. First-class shortcuts:

- **Cmd-K / Ctrl-K command palette** — the surface-agnostic index
  of every verb and subcommand. Available in TUI, Chat, and Web.
  Fuzzy-matches against command, synonyms, recent args, and
  heuristic names. Arrow-keys navigate; Enter runs; Shift-Enter
  copies the equivalent CLI invocation to the clipboard.
- **Named sessions** — `roko session use research-q2` attaches to a
  persistent session; the TUI title-bar shows the name; chat and
  Web auto-scope to it. `roko session list` shows recent sessions
  ordered by last-touched. Sessions are Engrams too (see
  `02-engram-vs-pulse.md` §4), so they replay cleanly.
- **Recent-prompt history** — `Ctrl+R` in CLI/Chat opens fuzzy
  history (scoped to the current session by default; `Ctrl+Ctrl+R`
  widens to all sessions). History is indexed by both prompt text
  and the heuristics that fired.
- **Bookmarks** — `#` on any episode/engram hash pins it; `@#slug`
  resolves anywhere a prompt is accepted. Bookmarks sync to the
  heuristic-commons client, so the same label works across devices.
- **Keybinding profiles** — `--keymap {default,vim,emacs}` flips
  the TUI and Chat keyset. Profiles are config-editable and
  documented in one place.
- **Batch mode** — `roko ask --batch prompts.txt` fans prompts over
  a pool; output as JSONL. Pairs with `27-realtime-event-surface.md`
  cursors so a long batch can be monitored from the Web UI.

These aren't free; every shortcut has a cost in discoverability
debt. But they're what turns a user from "productive" into "fast."

Users come with expectations from Claude Code, OpenClaw, Cursor,
Aider, and chat interfaces. Adopt rather than innovate where it
costs nothing:

- **`:` to open command palette** (like VS Code).
- **`/` to slash-command** (like Claude Code / Discord).
- **`?` for context help** (like vim).
- **`q` to quit** (like less / top / vim).
- **`j/k` for navigation** (TUI-wide convention).
- **`Ctrl+R` for fuzzy history search** (bash).
- **`#` for anchoring a thought/task** (like GitHub issues).

Familiar keystrokes produce goodwill; novel keystrokes produce
frustration until they compound into muscle memory. Spend novelty
carefully.

## 9. Live progress, not polling

When an agent is working:

- **Token streaming** from the LLM.
- **Tool call banners**: "→ reading src/main.rs (line 1–200)" as
  the tool fires.
- **Gate feedback**: "✗ Unit tests failed (3 of 47)" as gates fail.
- **Episode events**: "+ heuristic 'flaky-test-log-first' applied
  (confidence 0.82)" as beliefs feed the decision.

The user feels the agent *thinking* rather than staring at a
stopped spinner. This is why `26-statehub-rearchitecture.md` and
`27-realtime-event-surface.md` matter — the Bus already produces
all this data; the surfaces need to render it.

## 10. Human-in-the-loop checkpoints

Some decisions should invoke the user. Three kinds:

### 10.1 Permission checkpoints

Before dangerous actions (deleting files, network calls to unseen
endpoints, hitting rate-limited APIs), ask. Default: ask once per
session for each class of action, remember within a session.

### 10.2 Ambiguity checkpoints

When the agent has low confidence between two paths, surface a
choice:

```
The agent is unsure how to proceed:

  1. Add a new module to src/net/ (confidence 0.51)
  2. Extend the existing client.rs (confidence 0.49)

  a) Always prefer 1
  b) Always prefer 2
  c) Ask me every time for this kind of decision
  d) Let the agent decide

  >
```

The choice itself becomes a heuristic ingredient — "this user
prefers extending over adding new modules." Over time the user is
asked less and less.

### 10.3 Review checkpoints

Before creating a PR or committing, show the diff. User can
approve, edit, or cancel. Default: always review; never blind
commit.

### 10.4 Permission model at a glance

| Action | Default | Remember within | Can auto-approve? |
|---|---|---|---|
| Read file inside project root | auto | — | n/a |
| Read file outside project root | ask | session | yes (per-path glob) |
| Write file inside project root | ask once | session | yes (per-directory) |
| Delete file (any path) | ask every time | never | no |
| Run local command (whitelisted) | auto | — | n/a |
| Run local command (unrecognized) | ask | session, per-command | yes |
| Network request to allowlisted host | auto | — | n/a |
| Network request to new host | ask | session, per-host | yes |
| Spend > `$budget` on a single turn | ask | never | no |
| `git commit` / open PR | ask | never | no |
| Install/upgrade a plugin | ask | never | no |
| Execute a tool flagged `role_allow=` miss | deny | — | no (role change required) |
| Override a failing gate | ask | per-rung, per-session | yes for non-critical rungs |
| Send a chat-reply or email on user's behalf | ask every time | never | no |

Rules that bind the table:

- "Auto" actions still emit a Pulse so the TUI banner / `watch`
  stream shows what happened — silent success is still visible
  success.
- Session-scoped remember is cleared by `roko session forget` and
  by `roko init --reset-permissions`.
- Every denial carries the literal override command (e.g.
  `roko permit network:example.com --duration 1h`).
- Prohibited actions (see `24-deployment-ux.md` §3 on secret
  handling) are never promotable to auto — they remain `ask every
  time` regardless of config.

## 11. Making undo first-class

Users need to feel safe. Three levels of undo:

- **Ephemeral**: a recently-sent prompt can be edited (chat).
- **Short-term**: `roko undo last` reverts file changes from the
  last task.
- **Long-term**: every episode has a snapshot; `roko replay
  <episode>` can replay from there or `roko revert <episode>` can
  undo its diffs.

Undo being real makes users more adventurous. More adventurous
usage → more reinforcement signal → more learning.

## 12. Shareable, replayable sessions

Every session should be exportable:

```bash
roko session export > my-session.jsonl
roko session share --expires 24h     # uploads to registry, returns URL
```

Another user (or the same user later) can `roko session replay`.
This powers bug reports ("here's my session, reproduce"), blog
posts, and tutorials. It's also the unit of empirical training
data across deployments for the heuristic commons.

## 13. Accessibility

Not optional. Non-negotiables:

- TUI colors have configurable high-contrast mode.
- Screen-reader friendly markup in Web UI.
- Every critical action also available as a CLI command (for users
  who can't / won't use graphical surfaces).
- Keyboard-only navigation in TUI and Web.
- Internationalizable strings — English by default, but no
  hardcoded strings in user-facing paths.

## 14. The moments that matter

If we get seven moments right, most users forgive everything else:

1. First successful `roko ask` (under 30 seconds).
2. First `roko do` producing a real change (under 5 minutes).
3. First time the user sees a heuristic they didn't know they
   "taught" get applied.
4. First time the user watches c-factor go up after they made a
   choice to rotate pairs / diversify / introduce a challenger.
5. First time the user recovers from a mistake via `roko undo` or
   replay.
6. First time the user switches surface mid-task (CLI → TUI,
   Chat → Web) and the session follows them without state loss.
   This is the payoff of `26-statehub-rearchitecture.md` and
   `27-realtime-event-surface.md` on the user side.
7. First time a shared session (`12` above) reproduces someone
   else's bug on the user's machine, and the commons heuristic
   the original author had kicks in locally as it did for them.

Each moment is a product-design commitment, not just a feature.
The surfaces in this doc exist to produce them reliably.

### Related refinements

- `26-statehub-rearchitecture.md` — the projections backing the
  live progress, permission banners, and multi-surface continuity.
- `27-realtime-event-surface.md` — WebSocket/SSE cursors that make
  `watch` and the Web UI feel real-time.
- `28-cli-parity-familiar-workflows.md` — the slash-command and
  palette conventions this doc leans on.
- `30-rich-ux-primitives.md` — tool banners, uncertainty bars, and
  heuristic footnotes that render the Bus stream.
- `22-developer-ux-rust.md` §2 — the Rust SDK layer a power user
  drops into when they outgrow the shipped surfaces.
