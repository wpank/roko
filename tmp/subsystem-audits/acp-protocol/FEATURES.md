# ACP Protocol: Feature Inventory

> Consolidated from `tmp/acp-features/00-ACP-FEATURES.md`, v2 UX showcase JSX
> (`roko_acp_showcase_v2.jsx` — 9 scenarios, ~4K LOC), and crate exploration.

Legend: `[x]` done, `[~]` partial, `[ ]` not started, `[*]` novel/proposed

---

## 1. Layout & Chrome

The v2 mockup defines a 4-column layout:
1. **ThreadsSidebar** (230px) — projects, threads, agents, worktrees
2. **AgentPanel** (flex) — main message stream + composer
3. **EditorPeek** (360px) — code preview with agent cursor + gate gutter
4. **RightRail** (320px) — Cost, Router, Knowledge, MCP, EpisodeScrubber, PermissionScope

Plus horizontal strips:
- **TitleBar** — brand toggle (roko/neutral), project/worktree picker, session ID
- **PhaseStrip** — horizontal FSM (8 phases: Pending→Strategizing→Implementing→Gating→Auto-fix→Reviewing→Committing→Complete)
- **ConfigBar** — 11 status-bar controls (Model, Effort, Temp, Routing, Workflow, Review, Compile, Test, Clippy, Max iter, Mode)
- **StatusBar** — connection status, version, phase, gates pass/total, role legend, MCP/LSP counts
- **GateRow** — live gate strip below messages (compile/test/clippy with duration+status)

---

## 2. Nine Scenarios (from v2 JSX)

### 2a. Pipeline Live (`pipeline`)
Full FSM workflow: Express / Standard / Full.
- `[x]` FSM phase strip with active/done/future states
- `[x]` Multi-role sequential dispatch (STR → IMP → FIX → REV → AUD)
- `[ ]` **Plan card** with 8 tasks, role badges (STR/IMP/REV/ARC/AUD), priority (high/low), status (completed/in_progress/pending)
- `[ ]` **Knowledge injection card** — neuro store hits with scores, sources (playbook/episode/neuro), injected into SystemPromptBuilder L7 of 9
- `[ ]` **Shell command execution** in terminal block (openssl genrsa, pnpm test) with streaming + exit codes
- `[ ]` **Inline diff blocks** (old/new lines, file path, hunk count)
- `[ ]` **Permission request card** with scope tags (new dir keys/, writes 2 files, no network) and options (Allowed allow once)
- `[ ]` **Multi-role reviewer round** — REV does code review, ARC does security review, AUD does compliance review
- `[ ]` **Agent-to-agent chat** — ARC→reviewer, AUD→reviewer structured messages with role badges
- `[ ]` **Autofix iteration tracking** — "phase autofixing → gating · iter 2/3"
- `[ ]` **Commit checkpoint** with hash, file count, iteration count, Restore button
- `[ ]` **Episode logging at completion** — "Episode logged with strategy + retry counts → cascade router"
- `[ ]` **Adaptive gate threshold update** on completion

### 2b. Parallel Tournament (`tournament`)
3 approaches in parallel worktrees → benchmarks → merge.
- `[ ]` **SwarmGrid** — N parallel agents as cards (agent name, branch, approach name, progress bar, gate dots)
- `[ ]` **Per-agent worktree isolation** (perf/redis, perf/lru, perf/idx)
- `[ ]` **Per-agent gate tracking** (3 dots: compile/test/clippy per agent)
- `[ ]` **Winner crown indicator** on best approach
- `[ ]` **Synthesis divider** — "SYNTHESIS · REVIEWER READS ALL THREE DIFFS"
- `[ ]` **Comparison table** (approach / p99 / complexity / risk) with quantitative metrics
- `[ ]` **Reviewer + Auditor synthesis** — cross-approach review messages
- `[ ]` **Merge prompt** — "Merge perf/lru + perf/idx into perf/users-fast?" with options: Yes merge, Keep separate, Show 3-way diff
- `[ ]` **Merge scope tags**: 3-way diff, combined test suite, auto-detect conflicts

### 2c. Incident Triage (`incident`)
P1 spike → re-plan → hotfix.
- `[ ]` **Priority badge** — "P1 incident" + "@checkout-svc" pills on user message
- `[ ]` **Strategist auto-selection** — "P1 triage. Express workflow → fetch logs → bisect → minimal fix → ship"
- `[ ]` **MCP tool calls with server badge** — datadog (fetch logs), github (git bisect) shown with `mcpServer` pill
- `[ ]` **Knowledge injection** — past P1 playbook + past similar episode
- `[ ]` **Dynamic re-plan** — PLAN card with "RE-PLANNED" badge, tasks strikethrough + new tasks, 2/6 counter
- `[ ]` **Permission with auto-revert** — "Allowed · single edit · revert on test fail" with scope: single edit, auto-revert if tests fail, tagged for fast review
- `[ ]` **Terminal block** — test output with FAIL (red) + PASS (green) + streaming indicator
- `[ ]` **Autofix loop** — "Two more tests fail. Re-planning with auto-fixer."
- `[ ]` **Thinking indicator** — "... thinking..." with animated dots

### 2d. Architect → Code (`architect`)
Read-only review then exit-plan.
- `[ ]` **Mode switch** — "mode → Architect · code edits disabled"
- `[ ]` **Embedded resource** — "@rfc/0042-rate-limit.md (4.2 kB)" as context pill
- `[ ]` **Architect structured review** — Strong points (green), Concerns (amber), Blocker (red) sections
- `[ ]` **Agent-to-agent review** — AUD→architect "Concur on fail-open blocker. SOC6.1 requires fail-closed."
- `[ ]` **Mode switch prompt** — "Switch Architect → Code and prototype fail-closed limiter?" with 3 options: Yes auto-accept actions, Yes confirm each, No stay Architect
- `[ ]` **Handoff scope tags**: new branch: experiment/limiter-failsafe, scoped to gateway/middleware/, auto-commit per step

### 2e. Agent Following (`follow`)
Live cursor + call graph.
- `[ ]` **Mode change** — "mode → Research · read-only · no writes"
- `[ ]` **EditorPeek cursor tracking** — "following · L11" pill with crosshair icon, highlighted line in editor
- `[ ]` **AGENT NAVIGATES dividers** — "→ SRC/AUTH/API.TS" between file reads
- `[ ]` **Sequential file trace** — Read login.tsx → Read api.ts → Read auth.ts → Read jwt.ts
- `[ ]` **Per-file analysis** — numbered steps "1 — 'handleSubmit' calls 'login(email, pwd)'. Following the import."
- `[ ]` **CallGraph card** — tree visualization: LoginForm.handleSubmit → api.login → POST /api/auth/login → users.findByEmail → bcrypt.compare → signToken → Set-Cookie → navigate('/dashboard')
- `[ ]` **File references** with source paths (login.tsx:7, api.ts:48, auth.ts:88, etc.)
- `[ ]` **Token usage display** — "Five files, no edits. Token usage ~6.2k."

### 2f. Cascade Learning (`router`)
Router confidence + override.
- `[ ]` **RouterTrace card** — cascade router decision with policy (auto · cost-optimized), candidates with scores + reasons (haiku 94% trivial, sonnet 62% overkill, opus 21% wasteful)
- `[ ]` **Estimated cost pill** — "Routed to haiku · estimated cost $0.001"
- `[ ]` **Gate results inline** — "GATES · compile 0.4s · test 8/8 0.6s · clippy 0.2s"
- `[ ]` **Cascade escalation** — haiku fails gate → router shows escalation: haiku 32% failed gate, sonnet 91% escalated, opus 40% reserve
- `[ ]` **Override recording** — "Override recorded" card in router panel, explains learned pattern
- `[ ]` **Cumulative savings** — "Cumulative session savings vs always-opus: 87%"
- `[ ]` **Router panel: recent decisions** — task → chosen model → success/fail → escalation, per-task history
- `[ ]` **Learning indicator** — "→ LEARNING" badge on cascade router when actively learning

### 2g. Debug / Teach (`debug`)
Step through every decision.
- `[ ]` **Mode indicator** — "mode → Teach · step-through reasoning"
- `[ ]` **StepCard** — numbered steps (step 1/7, step 2/7, ...) with title + thought text
- `[ ]` **Git blame tool call** — "git blame · who wrote line 12?" with IMP role badge
- `[ ]` **MCP tool call** — "linear · ticket BIL-218" with Linear MCP server badge
- `[ ]` **Narrative reasoning** — step-by-step explanation of each decision
- `[ ]` **Reconciliation step** — "The 'bug' is documentation drift, not a code bug."
- `[ ]` **Downstream analysis** — "grep for 'applyDiscount' usage" before changing code
- `[ ]` **Minimal intervention** — comment-only fix when appropriate
- `[ ]` **Insight at completion** — "I almost made a 10x pricing change because the comment was outdated."

### 2h. Pair Convergence (`pair`)
Writer ↔ reviewer alternating.
- `[ ]` **Pair mode pill** — "⚡ pair mode" on user message
- `[ ]` **Writer+Reviewer alternation** — IMP drafts v1, REV reviews ("Two issues"), IMP revises to v2, REV reviews again, IMP revises to v3
- `[ ]` **Versioned edits** — v1 → v2 → v3 with inline diffs per version
- `[ ]` **Agent-to-agent review** — REV→implementer "processReq happens unconditionally", "span not in scope", "name 'runChain' suggests void"
- `[ ]` **Convergence checkpoint** — "Pair convergence · 3 rounds, 2 must-fix, 1 nit · v3 final" with Restore button
- `[ ]` **Meta-metric** — "Pair sessions log convergence rate as meta-metric — this one was fast (3 rounds) because both agents had access to the same trace span knowledge."

### 2i. Episode Replay (`replay`)
Scrub a finished session.
- `[ ]` **Replay header** — "Replaying episode #4291 · auth migration · 11.5 min · workflow=full · 2 retry iterations"
- `[ ]` **Session dividers with timestamps** — "SESSION START · 14:18", "SCRUB TO GATE FAILURE · 5:42", "SCRUB TO ARCHITECT BLOCK · 8:14", "SCRUB TO COMMIT · 11:32"
- `[ ]` **Archived terminal output** — "[archived] pnpm test" with preserved FAIL/PASS coloring
- `[ ]` **EpisodeScrubber panel** — timeline with event markers (user/phase/tool/knowledge/perm/gate/done), scrub head, play/skip controls
- `[ ]` **Scrub position detail** — shows current event type, phase, tool at scrub position
- `[ ]` **Branch from here button** — fork from any scrub position to restore that state
- `[ ]` **Learnings extraction** — "(1) refresh.spec missed → playbook checklist (2) PRIVATE_KEY hoisting → architect lint (3) cascade router → skip iter 1"
- `[ ]` **Post-replay router update** — cascade router shows: playbook 95% amended · checklist, router 88% amended · retry pattern, lint 72% amended · key hoisting

---

## 3. Right Rail Panels

### 3a. Cost · Budget Panel
- `[ ]` This-turn cost ($0.280) with bar vs turn budget ($0.50)
- `[ ]` Session cost ($0.94) vs session budget ($3.00) with bar
- `[ ]` Token breakdown: input / output / cached (green) / thought (purple)
- `[ ]` Cost-per-turn sparkline (last 12 turns) with trend indicator (-34%)

### 3b. Cascade Router Panel
- `[ ]` Tier confidence bars: haiku 74%, sonnet 18%, opus 8% with cost-per-tier
- `[ ]` Recent decisions list: task → chosen model → success/fail → escalation
- `[ ]` Override recorded card (when learning active)
- `[ ]` "→ LEARNING" indicator badge

### 3c. Knowledge Panel
- `[ ]` Neuro hits with scores (0-100), source paths, text snippets
- `[ ]` "injected via SystemPromptBuilder · L7 of 9" footer
- `[ ]` "no neuro hits this turn" empty state

### 3d. MCP Servers Panel
- `[ ]` Server list: datadog (8t), github (12t), linear (6t), postgres (4t), filesystem (9t)
- `[ ]` Per-server status: active (green pulse), idle (dim)
- `[ ]` Per-server call count during session

### 3e. Episode Replay Panel
- `[ ]` Live state: "live session · scrubber idle"
- `[ ]` Replay state: timeline with event markers, scrub slider, play/skip/back controls
- `[ ]` "Branch from here" button
- `[ ]` At-scrub-position event display

### 3f. Permission Scope Panel
- `[ ]` 11 scope rows with auto/ask/deny tri-state toggles
- `[ ]` Scopes: File reads, Searches, Network fetches, Edits in src/, Edits in services/, Deletions, Shell·safe, Shell·network, Shell·write disk, git commit, git push
- `[ ]` Per-scope call count

---

## 4. Message Primitives (16 types)

| Type | Component | Data Requirements |
|---|---|---|
| `user` | UserMessage | text, context pills (icon, label, color) |
| `agent_text` | AgentText | text, role, brand |
| `thinking` | Thinking | animated dots |
| `tool` | ToolCall | kind (10 types), title, status, role, location, cost, tokens, mcpServer, neuro |
| `plan` | PlanList | entries (content, status, role, priority), replan flag |
| `permission` | PermissionRequest | title, description, options (name, kind), scope tags |
| `terminal` | TerminalBlock | command, lines (text, color), running flag, exit code |
| `knowledge` | KnowledgeCard | items (score, text, source) |
| `router_trace` | RouterTrace | decision (policy, candidates: model, score, reason, chosen) |
| `swarm` | SwarmGrid | items (agent, branch, name, status, metric, progress, gates, winner) |
| `callgraph` | CallGraph | nodes (fn, file, depth) |
| `agent_chat` | AgentChat | from, to, role, text |
| `mode_change` | ModeChange | from, to, note |
| `phase_change` | PhaseChange | from, to, note |
| `checkpoint` | Checkpoint | label, hash |
| `step` | StepCard | n, total, title, thought |
| `gate_row` | GateRow | gates (name, status, detail, duration) |
| `divider` | Divider | label, icon |

---

## 5. Config Controls (Status Bar — 11 controls)

| Control | Type | Values | Per-Scenario Variation |
|---|---|---|---|
| Model | Select | claude-opus-4.7, haiku→sonnet (router) | Router shows "haiku → sonnet" |
| Effort | Select | low/medium/high/max | All show "high" |
| Temp | Select | conservative/balanced/aggressive/exploratory | Tournament = "exploratory" |
| Routing | Select | auto/auto·learning/manual | Router = "auto · learning" (teal) |
| Workflow | Select | none/express/standard/full | Color-coded: full=purple, standard=blue, express=green |
| Review | Select | off/quick/standard/thorough | Matches workflow level |
| Compile | Toggle | on/off | Green when on |
| Test | Toggle | on/off | Green when on |
| Clippy | Toggle | on/off | Green when on |
| Max iter | Select | 1/2/3 | All show 3 |
| Mode | Select | code/plan/research | Color: plan=purple, research=blue, code=green |

---

## 6. Slash Command Palette

- **47 builtin** across 10 groups (Status, Research, PRD, Planning, Execution, Verify, Knowledge, Code, Feedback, Workflow)
- **4 user-defined** from roko.toml: /ship-it, /post-mortem, /budget-review, /onboard (dashed border, brand-colored)
- **12 from workflows** (noted in footer)
- Search/filter with keyboard navigation (↵ run · ⇥ complete · ↑↓ navigate)

---

## 7. Design Tokens

6 role colors: Strategist (#7F77DD purple), Implementer (#5DCAA5 teal), AutoFixer (#EF9F27 amber), Reviewer (#378ADD blue), Architect (#ED93B1 pink), Auditor (#E24B4A red)

10 tool kind icons: read (FileText), edit (Edit3), delete (Trash2), move (Move), search (Search), execute (Play), think (Brain), fetch (Globe), mcp (Database), other (Wrench)

Brand toggle: roko (warm orange #FF6A3D) vs neutral/zed (purple #7F77DD)

---

## 8. Feature → Subsystem Routing

| Feature | Primary Subsystem | Secondary | Data Feed Required |
|---|---|---|---|
| SwarmGrid (parallel agents) | orchestration | acp-protocol | Per-agent: status, progress, gates, branch, metric |
| Synthesis table | gate-pipeline | learning-feedback | Per-approach: p99, complexity, risk scores |
| KnowledgeCard injection | cognitive-layer | prompt-assembly | Neuro hits: score, source, text, injected layer |
| RouterTrace decisions | inference-dispatch | learning-feedback | Per-decision: model, score, reason, chosen, escalation |
| EpisodeScrubber | learning-feedback | http-persistence | Episode timeline: events with types + timestamps |
| PermissionScope panel | safety-agent | acp-protocol | 11 scope rows: id, label, allowed, count |
| CostPanel | inference-dispatch | acp-protocol | Turn/session cost, budget, tokens, sparkline |
| MCPPanel | config-tools-events | acp-protocol | Per-server: name, tools, status, call count |
| Plan card (re-plan) | orchestration | acp-protocol | Entries: content, status, role, priority, replan flag |
| CallGraph | code-intelligence | cli-chat-tui | Nodes: fn, file, depth (synthesized from traversal) |
| Agent cursor tracking | cli-chat-tui | acp-protocol | Line number, file path, following indicator |
| StepCard (debug/teach) | acp-protocol | prompt-assembly | Step number, total, title, thought text |
| AgentChat (pair/review) | orchestration | acp-protocol | From, to, role, text (cross-agent messaging) |
| Checkpoint (commit) | gate-pipeline | http-persistence | Label, hash, Restore action |
| Dynamic slash commands | config-tools-events | acp-protocol | User-defined + workflow-installed commands from TOML |
| GateRow (live strip) | gate-pipeline | acp-protocol | Per-gate: name, status, detail, duration |
| Terminal streaming | acp-protocol | — | Command, lines with colors, running flag, exit code |
| DiffBlock | acp-protocol | — | Path, old/new text, hunk count |
| Mode switch | acp-protocol | safety-agent | From/to mode, note, tool restrictions |
| Brand toggle | cli-chat-tui | — | roko vs neutral theme tokens |

---

## Sources

- `crates/roko-acp/src/types.rs` — `SessionUpdate` variants (AgentMessageChunk, AgentThoughtChunk, ToolCall, ToolCallUpdate, Plan, AvailableCommandsUpdate, ConfigOptionUpdate, UsageUpdate, SessionInfoUpdate), `ConfigOption`, `SlashCommand`, `PlanEntry`, `ToolCallKind`, `ToolCallStatus`
- `crates/roko-acp/src/session.rs` — `build_slash_commands` (49 commands), `build_config_options` (9 options), `SessionConfigState` fields, `session/set_mode` handling
- `crates/roko-acp/src/pipeline.rs` — `PipelinePhase` (10 states: Pending, Strategizing, Implementing, AutoFixing, Gating, Reviewing, Committing, Complete, Halted, Cancelled), `WorkflowTemplate` (Express/Standard/Full), auto-select logic
- `crates/roko-acp/src/runner.rs` — multi-role dispatch (Strategist, Implementer, AutoFixer, single-reviewer, multi-role Architect+Auditor), `run_claude_cli` subprocess, gate execution
- `crates/roko-acp/src/bridge_events.rs` — `CognitiveEvent` (TokenChunk, ThinkingChunk, ToolCallStart, ToolCallComplete, PlanUpdate, Complete, MaxTokens), provider routing (ClaudeCli vs API), slash command dispatch

