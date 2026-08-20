# Mori TUI Screenshots — Detailed Descriptions

> Captured 2026-08-19 from `./mori.sh --paused` running against the bardo repo.
> Original screenshots were in macOS temp dirs and are no longer on disk.
> These descriptions serve as the permanent reference.

## Global Status Bar (all tabs)

Top line across every tab:
```
● mori  Wave 1/7  Queue: Audit Remediation  [████████░░]  390/467  83%  1►
  ETA:Xs  Ys  MCP:0  C:5  F1:dash F2:plans F3:agents F4:git F5:logs F6:cfg F7:inspect F8:queue
```

Key elements:
- **Colored dot** — green = running, yellow = paused
- **Wave N/M** — current wave / total waves
- **Queue name** — active queue (e.g. "Audit Remediation")
- **Progress bar** — visual percentage
- **N/M count** — completed/total plans in queue
- **Percentage** — completion %
- **N►** — number of active agents
- **ETA** — estimated time remaining
- **MCP:N** — MCP call count
- **C:N** — context metric
- **F1-F8 tabs** — highlighted active tab

Bottom status line:
```
codex/batch/current  4cfc69f  135d ago  ● PAUSED  36/48  5► lag
  ↑↑:scroll  End:auto  `:agent  Alt+1..7:jump  a/o/d/e/g/m/P:detail  v:verify  i:inject  p:pause
```

---

## Screenshot 1: F1:dash — Dashboard (Agents sub-tab)

**Left panel: Plans list**
- Header: `Plans (36/48 5►)`
- Shows `Queue: Audit Remediation  Audit Remediation 36/48  Runnable`
- Columns: `plan | prog | bar | delta | vfy | ag...`
- Tree view: `Wave 0 (29/35)` expanded showing ~25 plans
- Each plan row: `✓ 01-wor…caffold  11/11  [████]  i3`
- Plans show completion fractions, colored progress bars, iteration count (i2, i3)
- Wave indicator with colored block visualization

**Right panel: Agent sub-tabs**
- Tab bar: `a:Agents  o:Output  d:Diff  e:Errors  g:Git  m:MCP  P:Procs 12%  v:impl`
- Header: `Agents (1 active)`
- Shows: `► cond  global · conductor  0k/200k ———— ● LIVE`
- Agent detail area: `[impl]` section showing selected plan info
- Plan detail: `01-workspace-scaffold > (no agents)`
  - `Status: Completed (11/11 tasks, 0 active, phase complete)`
  - `No agents assigned yet.`
  - `Use ↑/↓ to browse plans, Enter for detail.`

**Bottom-left: Phase panel**
- `Phase · 01-workspace-scaffold`
- Green progress bar, `● complete  ETA ~15m00s`

**Bottom-left: Tasks panel**
- `Tasks · 01-workspace-scaffold (11/11) · complete [1-4 of 11]`
- `clear  all tasks clear · phase complete`
- `✓ T1  Workspace Root & Toolchain`
- `✓ T2  Library Crate Shells (21 crates)`
- `✓ T3  App Binary Shells (5 apps)`
- `✓ T4  Build Tooling & Dev Setup`

**Bottom-right: System panel**
- `System` box with:
  - `CPU  34.8%  [sparkline]`
  - `MEM  36.0G  [bar]`
  - `NET  ↑8.2M  [sparkline]`
  - `DSK  R80.0K`
  - `FPS  62.5`
  - `GW   127.0.0.1:4000`
  - `— top procs ————`

**Background**: Matrix-style falling characters effect across the right panel

---

## Screenshot 2: F1:dash — Dashboard (Output sub-tab)

Same layout as Screenshot 1 but with `o:Output` tab selected.
Shows plan output text for the selected plan. The output area shows
`01-workspace-scaffold > (no agents)` with plan status info.

---

## Screenshot 3: F1:dash — Dashboard (Errors sub-tab)

Same layout but with `e:Errors` tab selected.

**Errors panel content:**
```
— Preflight: 1 warnings —
  └─ No fast linker (mold/lld) configured in .cargo/config.toml

— Runtime —
  └─ [orch] Found resumable checkpoint from the previous session
  └─ [executor] Started paused (--paused). Press 'p' to begin execution.
  └─ [preflight] No fast linker (mold/lld) configured in .cargo/config.toml
```

Small sparkline/block graphics in lower-right area.

---

## Screenshot 4: F2:plans — Plans Detail View

**Left panel: Waves / Plans tree**
```
Pipeline
  ► W0  ██░  29/35
  ✓ W1      3/3
    W2  █░  1/4
    W3      1/1
  ► W4  █░  2/2
  · W5      0/2
  · W6 ———— 0/1

Plans in W0 (queue groups):
Audit Remediation (35)
  ► ✓  01-workspace-scaffold [Audit Remediation+1] [...]
  ·    04-terminal-scaffold [Audit Remediation+1] [q...]
  ·    04b-testing-bootstrap [Audit Remediation+1] [...]
  ·    10-safety-types [Audit Remediation+1] [q#4] [...]
  ... (many more plans listed)
  ·    94-m1-subsystem-integration [Audit Remediatio...]
  ► ·  95-grimoire-learning-pipeline [Audit Remediati...]
  ... etc
```

**Right panel: Plan Detail**
```
Plan 01-workspace-scaffold  [MERGED]
phase complete  ·  impl 7  ·  verify 31  ·  agents 0  ·  branch codex/plan/01-workspace-scaffold
Iteration 3 of 3 max

Implementation:  [██████████]  7/7          Verification:  ————————— 0/31
  ✓ T1  Workspace Root & Toolchain           · CG1  Workspace Full Compilation
  ✓ T2  Library Crate Shells (21 crates)     · CG2  Core Crate Compilation
  ✓ T3  App Binary Shells (5 apps)           · CG3  Mortality Crate Compilation
  ✓ T4  Build Tooling & Dev Setup            · CG4  Grimoire Crate Compilation
  ✓ T5  mdbook Scaffold                      · CG5  Daimon Crate Compilation
  ✓ T6  TypeScript Sidecar Stub              · CG6  Dreams Crate Compilation
  ✓ T7  Legacy Plan Cleanup Review           · CG7  Terminal Application Compilation
                                              · LS1  Lifecycle: Phase Transition Sequence
                                              · LS2  Lifecycle: Composit...lity Multiplicative
                                              · LS3  Lifecycle: Vitality Range Validation
                                              · LS4  Lifecycle: Death Ca...n Critical Vitality
                                              · LS5  Lifecycle: Senescen...steresis Prevention
                                              · LS6  Lifecycle: Thanatopsis Terminal Phase
                                              · LS7  Lifecycle: No Tick ... with High Vitality
                                              · LS8  Lifecycle: Phase-Sp...ehavior Constraints
                                              · LS9  Lifecycle: Snapshot State Consistency
                                              · LS10 Lifecycle: Event Payload Variants
                                              · LS11 Lifecycle: Repeated Phase Transitions
                                              · LS12 Lifecycle: Vitality...nt Payload Accuracy
                                              · LS13 Lifecycle: Event Subsystem Routing
                                              · LS14 Lifecycle: Critical...ity Boundary at 0.0
                                              · LS15 Lifecycle: Vitality...vation Across Ticks
                                              · LS16 Lifecycle: Terminal...on on Zero Vitality
                                              · SV1  Scaffold: Configuration Files Presence
                                              · SV2  Scaffold: Workspace...tadata (26 members)
                                              · SV3  Scaffold: Rust Toolchain Pin
                                              · SV4  Scaffold: Mold Linker Configuration
                                              · SV5  Scaffold: Documentation Build
                                              · SV6  Scaffold: TypeScript Sidecar Types
                                              · SV7  Scaffold: Test Harness Directories
                                              · SV8  Scaffold: Just Recipe Configuration

Agents on this plan:
  (none)

  Branch:    codex/plan/01-workspace-scaffold
  Worktree:  .worktrees/plan-01-workspace-scaffold ●
  Commit:    unknown
```

**Bottom bar**: `↑↑:nav  +/-:waves  Enter/Esc:tree  PgUp/PgDn:jump  /:filter  s:retry  z:diag  S/R:repair  c:reverify`

---

## Screenshot 5: F3:agents — Agent Detail View

**Left panel: Agent list**
```
Agents (1 active)
  role/model  plan : task       ctx       meter  stat
  ► cond      global · con...tor  0k/200k  ——————  ——
```

**Right panel: Agent detail**
```
·: conductor ——————————————————————————
Plan: global     Task: conductor
Instance: conductor:llm
Model: auto      Provider: unknown
Route: implicit  Ctx: default
Tokens: 0/200k ——————————————————————————————————— 0%
  In: 0  Out: 0

conductor output ——————————————————————
  waiting...
```

**Bottom bar**: `↑↑:nav  Tab:panel  `:cycle  Alt+1..7:jump  End:auto  p:pause  F1:dash  ?:help`

---

## Screenshot 6: F4:git — Git Branches & Worktrees

**Top-left: Branches**
Tree view showing branch structure:
```
main 4089d37b
├─20260320 deb0a933
│ ├─01-workspace-scaffold c9365472
│ ├─04b-testing-bootstrap 29b19f50
│ ├─102-safety-integration 7b185e70
│ ├─104-heartbeat-advanced dbabd40d
│ ├─105-gateway-production-readiness 35a3cda9
│ ├─107-sonification-integration 944582cf
│ ├─109-owner-intervention-system 3b525484
│ ├─11-inference-gateway 9722e9b7
│ ├─110-policycage-constitution 34ff523c /User...tution
│ ├─111-death-testament-succession 7eb51ce2
│ ├─112-dream-daimon-bridge 5ee81a47 /Users...–bridge
│ ... (many more branches)
```

**Top-right: Commit Graph**
Empty in this screenshot (no commit graph loaded).

**Bottom-left: Worktrees**
```
· codex/batch/current  4cfc69fa ok /Users/will...iswap/bardo
·                      5ee81a47 detached /Users/w...in-fresh
· worktree-...t-af9be90d dbbaaff0 ok /Users/will...nt-af9be90d
► codex/pla...nstitution 34ff523c ok /Users/will...onstitution
► codex/pla...mon-bridge 5ee81a47 ok /Users/will...imon-bridge
► codex/pla...chitecture f9ab8e90 ok /Users/will...rchitecture
► codex/pla...de-lineage 3639716a ok /Users/will...ade-lineage
► codex/pla...arketplace 95822d05 ok /Users/will...arketplace
  codex/pla...earth-mind a0d35cfc ok /Users/will...hearth-mind
  codex/pla...te-command 7c857a0a ok /Users/will...ate-command
► codex/pla...1-mvp-gate 8f967538 ok /Users/will...71-mvp-gate
► codex/pla...ntegration cfd6c30b ok /Users/will...integration
► codex/pla...g-pipeline 6a931a73 ok /Users/will...ng-pipeline
► codex/pla...–attention 64903930 ok /Users/will...c-attention
► codex/pla...e-pipeline 3210be60 ok /Users/will...ge-pipeline
► codex/pla...oring-hdc b50e99e6 ok /Users/will...scoring-hdc
```

**Bottom-right: Branch Info**
```
main
commit 4089d37b
root codex/batch/current
main sync diverged
warn repo-root checkout has local changes; reconcile will avoid destructive resets
```

**Bottom bar**: `↑↑:nav  Enter:select  p:pause  F1:dash  ?:help`

---

## Screenshot 7: F5:logs — Runtime Logs

Full-width log viewer:
```
Logs (9)
10:03:56 △ [orch] Found resumable checkpoint from the previous session
10:03:56 · [queue] Queue Audit Remediation active: 48 plan(s) across 8 milestone group(s)
10:03:56 · [executor] Resumed: 36 plans already complete
10:03:56 △ [executor] Started paused (--paused). Press 'p' to begin execution.
10:03:56 · ☉ [executor] Express mode enabled: no strategist, no reviews, auto-fix on gate failure
10:03:56 · [system] Initializing Mori runtime...
10:03:56 △ ☉ [preflight] No fast linker (mold/lld) configured in .cargo/config.toml
10:03:56 · [conductor] Conductor LLM agent spawned [model=claude-sonnet-4-6]
10:03:56 · [executor] Parallel mode: 48 plans, 15 max agents, 5 max parallel plans, 3 initial spawn actions (10 total actions, 7 setup)
```

**Bottom bar**: `↑↑/PgUp/PgDn:scroll  p:pause  F1:dashboard  ?:help`

---

## Screenshot 8: F6:cfg — Configuration View

**Left panel: Configuration**
```
Configuration [j/k:nav h/l:cycle Enter:toggle]
  Source: repo override
  Repo config is active and overrides your global Mori defaults.

—— Backend Defaults ——
► Codex default: < Claude Sonnet 4.6 >
  Cursor default: < Composer 2 Fast >
  Claude default: < Claude Sonnet 4.6 >
  Conductor model: < Claude Sonnet 4.6 > [cl]
  Fallback model: < Claude Sonnet 4.6 > [cl]
  Force one model: [ ]
  Forced model: < Claude Sonnet 4.6 >
  Disabled providers: (none)

—— Per-Role Overrides ——
conductor:  (cl: cl-sonnet-4-6)
strat:      (cl: cl-sonnet-4-6)
impl:       (cl: cl-sonnet-4-6)
arch:       (cd: cl-sonnet-4-6)
audit:      (cd: cl-sonnet-4-6)
scribe:     (cl: cl-sonnet-4-6)
critic:     (cl: cl-sonnet-4-6)
refac:      (cd: cl-sonnet-4-6)
prepl:      (cd: cl-sonnet-4-6)
docvf:      (cd: cl-sonnet-4-6)
itest:      (cd: cl-sonnet-4-6)
merge:      (cd: cl-sonnet-4-6)
tval:       (cd: cl-sonnet-4-6)
glct:       (cd: cl-sonnet-4-6)
sdrf:       (cd: cl-sonnet-4-6)
regd:       (cd: cl-sonnet-4-6)
perf:       (cd: cl-sonnet-4-6)
covr:       (cd: cl-sonnet-4-6)
plcm:       (cd: cl-sonnet-4-6)
xsys:       (cd: cl-sonnet-4-6)
errdx:      (cd: cl-sonnet-4-6)
rsrch:      (cd: cl-sonnet-4-6)
depv:       (cd: cl-sonnet-4-6)
patrn:      (cd: cl-sonnet-4-6)
snapc:      (cd: cl-sonnet-4-6)
afix:       (cl: cl-sonnet-4-6)
qrev:       (cl: cl-sonnet-4-6)
FLV:        (cl: cl-sonnet-4-6)

—— Context & Effort ——
```

**Right panel: Agent Status**
```
Agent Status
· strategist      cl-sonnet-4-6   0/150k  0% t0 high
· implementer     cl-sonnet-4-6   0/150k  0% t0 high
· architect       cl-sonnet-4-6   0/150k  0% t0 high
· auditor         cl-sonnet-4-6   0/150k  0% t0 medium
· scribe          cl-sonnet-4-6   0/150k  0% t0 low
· critic          cl-sonnet-4-6   0/150k  0% t0 medium
· refactorer      cl-sonnet-4-6   0/150k  0% t0 medium
· pre-planner     cl-sonnet-4-6   0/150k  0% t0 medium
· doc-verifier    cl-sonnet-4-6   0/150k  0% t0 medium
· integration-tester cl-sonnet-4-6 0/150k  0% t0 medium
· merge-resolver  cl-sonnet-4-6   0/150k  0% t0 medium
· terminal-validator cl-sonnet-4-6 0/150k  0% t0 medium
· golem-lifecycle-tester cl-sonnet-4-6 0/150k 0% t0 medium
· spec-drift-detector cl-sonnet-4-6 0/150k 0% t0 medium
· regression-detector cl-sonnet-4-6 0/150k 0% t0 medium
· performance-sentinel cl-sonnet-4-6 0/150k 0% t0 medium
· coverage-tracker cl-sonnet-4-6  0/150k  0% t0 medium
· plan-lifecycle-mgr cl-sonnet-4-6 0/150k 0% t0 medium
· cross-system-tester cl-sonnet-4-6 0/150k 0% t0 medium
· error-diagnoser cl-sonnet-4-6   0/150k  0% t0 medium
· researcher      cl-sonnet-4-6   0/150k  0% t0 medium
· dep-validator   cl-sonnet-4-6   0/150k  0% t0 high
· pattern-extractor cl-sonnet-4-6 0/150k  0% t0 medium
· snapshot-comparator cl-sonnet-4-6 0/150k 0% t0 medium
· auto-fixer      cl-sonnet-4-6   0/150k  0% t0 medium
· quick-reviewer  cl-sonnet-4-6   0/150k  0% t0 low
· full-loop-validator cl-sonnet-4-6 0/150k 0% t0 medium

MCP / Context
[MCP]  PARTIAL  stdio/on-demand  refresh 2s ago
codex:on  claude:on  cursor:on
6.1k files  153.6k syms  0 mcp calls
```

**Bottom bar**: `j/k:nav  h/l:cycle  Enter:toggle  p:pause  MCP summary on right  F1:dashboard`

---

## Screenshot 9: F7:inspect — MCP / AST / Learning

**Top: MCP Runtime**
```
MCP Runtime
[MCP]  PARTIAL  stdio/on-demand  selected 01-workspace-scaffold  refresh 2s ago
codex cli:on  claude cli:on  cursor acp:on  ·  root repo
task route T1  band standard
live route idle
learning  6.6k ep  98 rules  routing 92%  playbook 5.3k  knowledge 0
fixtures  none
```

**Left panel: Servers / Roots**
```
claude repo   ok .mori/mcp-config.json
  claude wt   miss
.worktrees/plan-01-workspace-scaffold/.mori/mcp-config
.local.json
  codex repo  ok .codex/config.toml
  codex wt    miss
.worktrees/plan-01-workspace-scaffold/.codex/config.to
ml
cursor repo   ok .cursor/mcp.json
  cursor wt   miss
.worktrees/plan-01-workspace-scaffold/.cursor/mcp.json
  launch miss mori-mcp
root path     ok /Users/will/dev/uniswap/bardo
  index db    ok .mori/index.db
Selected plan is currentl...po-root index and MCP cwd.
```

**Center panel: AST Index**
```
AST Index
  files 6.1k
  symbols 153.6k
  references 634.9k
  resolved 285.3k (45%) [████░░░]
  density 25.3 symbols/file
  routing 92% [██████████░]

AST index gives the agents signatures and symbol
topology without reading full files.
This is why Mori can compress context before the prompt
even starts spending tokens.
```

**Right panel: Tool / Learning**
```
Tool / Learning
  episodes 6.6k ok / 0 fail
  playbook 98 total  98 learned  0 manual
    routing 1.6k / 1.7k (92%)
  rich route 1.6k / 1.7k (91%)
    hints 12 model  12 provider  366 research  5.3k
  playbook
    prompt 35.7kB avg  1.3kB inline avg  8.6kB pack avg
    artifacts 137 research  137 integration  145 dep  145
  fixture
    registries 1.1k deps  612 fixtures  2.9k sidecars
    live fx 0 active  612 planned
    support 0 fresh  29 missing
  knowledge
    0 file-intel  0 warnings  0 wave-ctx  0 err-pat
  history
    98 plans  67 refl plans / 1.5k entries
    model claude-opus-4-6 100% pass · 129s avg · 0.9
      retry · 10811 tok · $1.433/run · 1905 runs
    provider claude 100% pass · 136s avg · 0.8 retry · 13010
      tok · $1.060/run · 3918 runs
    strategy mcp_first 100% pass · 127s avg · 0.5 retry ·
      22386 tok · $0.685/run · 5032 runs
    route global override (composer-2-fast) 100% pass ·
      60s avg · 0.2 retry · 0 tok · $0.000/run · 1277 runs
    fixture mock-http 100% pass · 127s avg · 0.3 retry ·
      22466 tok · $0.587/run · 2853 runs
  refresh q 25
    refresh 2s ago

  tool calls 0
    mcp calls 0 (0%)
    cd 0  cl 0  cx 0

  observed tools: 0
```

**Bottom bar**: `Inspect view  MCP/AST/learning/fixtures  selection stays pinned  p:pause  F1:dashboard  ?:help`

---

## Screenshot 10: F8:queue — Queue Overview

**Left: MCP Runtime (same as F7 top)**

**Center-left: Queue Overview**
```
Queue Overview
Milestones
  Queue: Audit Remediation

Servers / Roots (same as F7 left panel)
```

**Center-right: Queue Order**
```
Queue Order
  [Audit Remediation]  36/48 complete

Pending (queue order)
  40-styx-architecture, 47-clade-lineage, 48-knowledge-marketplace,
  70a-terminal-hearth-mind, 70c-terminal-fate-command, 71-mvp-gate,
  94-m1-subsystem-integration, 95-grimoire-learning-pipeline,
  96-chainscope-dynamic-attention, 97-triage-pipeline, 98-curiosity-scoring-hdc,
  99-prediction-engine-causal-graph
  Next up: 40-styx-architecture

  ✓ 01-workspace-scaffold  [complete]
  ✓ 04-terminal-scaffold   [complete]
  ✓ 04b-testing-bootstrap  [complete]
  ✓ 10-safety-types  [complete]
  ✓ 100-chain-layer-completion  [complete]
  ✓ 101-lifecycle-death-succession  [complete]
  ✓ 102-safety-integration  [complete]
  ✓ 103-grimoire-storage-infrastructure  [complete]
  ✓ 104-heartbeat-advanced  [complete]
  ✓ 105-gateway-production-readiness  [complete]
  ✓ 106-strategy-interpreter  [complete]
  ✓ 107-sonification-integration  [complete]
  ✓ 108-surfaces-communication  [complete]
  ✓ 109-owner-intervention-system  [complete]
  ✓ 11-inference-gateway  [complete]
  ✓ 110-policycage-constitution  [complete]
  ✓ 111-death-testament-succession  [complete]
  ✓ 112-dream-daimon-bridge  [complete]
  ✓ 113-knowledge-validation-pipeline  [complete]
  ✓ 114-context-engineering  [complete]
  ✓ 116-sleepwalker-phenotype  [complete]
  ✓ 117-behavioral-modulation  [complete]
  ✓ 118-advanced-retrieval  [complete]
  ✓ 119-dream-evolution  [complete]
  ✓ 12-grimoire  [complete]
```

**Milestone groups sidebar:**
```
Runnable Golem
  8/13 plans
  mvp · golem ·
  critical-path

Intelligence, Terminal & Styx
  25/41 plans
  t0 · intelligence ·
  terminal · styx ·
  critical-path

Autonomous Golem Loop
  3/26 plans
  golem · ingest · learning
  · trading

TA And Market Intelligence
  0/17 plans
  ta · signals ·
  market-intel · research

Terminal And Creature Experience
  0/26 plans
  terminal · creature ·
  sonification · demo
```

---

## Key Design Patterns to Preserve

1. **Dense information hierarchy** — Status bar → panel layout → detail drilldown
2. **Sub-tabs within tabs** — F1:dash has a:Agents/o:Output/d:Diff/e:Errors/g:Git/m:MCP/P:Procs/v:impl
3. **Keyboard-driven navigation** — Every action has a key binding shown in context
4. **Live system metrics** — CPU/MEM/NET/DSK/FPS always visible
5. **Wave-based pipeline** — Plans organized into dependency waves
6. **Queue/milestone grouping** — Plans belong to named queues with milestone groups
7. **Branch-per-plan** — Each plan gets its own git branch and worktree
8. **30+ specialized agent roles** — Each with model/provider/priority assignment
9. **MCP integration display** — Server status, AST index, tool call counts
10. **Learning metrics** — Episodes, playbook rules, routing accuracy, cost/run stats
11. **Matrix-style visual effects** — Subtle animated characters in background
12. **Color coding** — Green=complete, yellow=warning, red=error, orange=in-progress
