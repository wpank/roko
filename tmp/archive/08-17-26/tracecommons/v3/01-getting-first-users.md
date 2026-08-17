# Getting First Users: It's a DevEx Problem

*Brainstorming doc -- ideas, not specs.*

**Context:** TraceCommons (TC) is an open-source, Rust-based, privacy-preserving register
of AI coding agent session traces. Contributors submit scrubbed traces of what their AI
agents did (Claude Code, Codex, IronClaw, etc.); quality and novelty are scored inside TEEs
(Trusted Execution Environments); contributors earn NEAR blockchain credits. Built by Zaki
Manian. 6 Rust crates, ~235K LOC, pilot on GCP. ~352 submissions, ~13/week, 3 contributors,
6 GitHub stars.

---

## The Core Insight

TC's engineering is genuinely good -- honest differential privacy, TEE attestation,
multi-gate scoring. None of that matters yet because the user acquisition problem is a
DevEx problem, not a product problem.

Developers adopt tools that give them something useful *immediately* and contribute to
commons where contribution feels like a byproduct of getting that useful thing. TC's
current flow inverts this -- it asks for contribution first and promises value later.
Every idea in this doc is about flipping that order.

---

## Four New Levers

### 1. OTel GenAI Conventions

The OTel `gen_ai.*` semantic conventions (v1.42.0, June 2026) are pre-stable but adopted
by Langfuse, Datadog, Phoenix/Arize, and MLflow. They're the de facto standard for agent
trace interchange.

**What this means:** Instead of asking developers to learn TC's envelope format, TC can
accept OTel spans over OTLP. Any team already emitting OTel traces can pipe them to TC
with a config change, not a code change. Integration cost drops from "learn our SDK" to
"add an exporter endpoint."

This is the single highest-leverage technical investment for user acquisition. It turns TC
from a destination into a receiver you add to your existing observability stack.

### 2. AgentGUI-Style Trajectory Replay

AgentGUI (ETH Zurich, arXiv:2607.26300) is an open-source GUI for replaying agent
trajectories with interactive scrubbing and visual structure. Their user study showed 38%
faster trace comprehension.

TC already parses cross-harness traces. Adding a replay viewer turns "browse your trace
history" from a metadata table into an interactive experience where you scrub through a
session and see exactly where the agent went wrong. Developers currently have no good way
to replay their AI coding sessions across agents. A tool that does this is worth
installing even if you never share a trace.

### 3. Error Hub (Failure-Trace Commons)

AgentDebugX (arXiv:2607.18754) ships an opt-in Error Hub for sharing scrubbed
failure-diagnosis-repair bundles. Their DeepDebug core repaired 13 of 73 failed GAIA
tasks in a single rerun (vs. 4-6 for baselines).

Developers gather where debugging happens. If TC becomes the place to search "my agent
keeps failing on this kind of task, has anyone else seen this?" and get back structured
failure bundles from the commons, that's a reason to install AND contribute.

Failure traces are also psychologically easier to share. There's less concern about
leaking proprietary patterns when the trace is "my agent spun in circles for 20 minutes
on an import error."

### 4. SKILL.md Distribution

The Agent Skills spec (~40 compatible products: Codex, Copilot, Cursor, Gemini CLI) uses
progressive disclosure -- a harness only loads a skill's name until it matches.

TC could mine its corpus for recurring high-quality patterns and publish them as SKILL.md
files. Every published skill credits TC and links back to the commons. Developers discover
TC not by hearing about it, but by using a skill extracted from it.

**Security angle:** Snyk's ToxicSkills research found 36.82% of scanned skills had at
least one security flaw. TC could offer skill security scoring as a service, extending its
scoring moat into a space with documented unmet need.

---

## Time to First Insight: The Only Growth Metric That Matters

Sentry grew by obsessing over "time to first event." PostHog had "time to insight." Both
got it under 5 minutes.

For TC, the equivalent is **time to first insight**: how long from `curl install.sh` to
seeing something useful about your own coding sessions.

Right now that number is effectively infinite -- building from source, DM'd invite code,
manual trace submission before you see anything back.

Target: under 90 seconds.

```text
$ curl -fsSL https://tracecommons.org/install.sh | sh     # 10 seconds
$ tc auth                                                   # 15 seconds (OAuth)
$ tc scan                                                   # 5 seconds
Found 47 sessions from the last 7 days.

  Session                  Quality   Novelty   Cost     Duration
  ---------------------------------------------------------------
  fix-auth-middleware       92/100    High      $0.84    12 min
  refactor-db-queries       84/100    Medium    $2.10    34 min
  debug-ci-pipeline         88/100    High      $1.22    18 min
  update-readme             31/100    Low       $0.12     3 min
  ... 43 more

Weekly summary:
  47 sessions | $42.80 total cost | avg 14 min/session
  Your most efficient pattern: sessions with upfront context score 40% higher
  Your biggest time sink: 6 sessions spent >50% of tokens on retries

12 sessions scored above contribution threshold (70+).
Contribute to the commons? [Y/n/always/never]
```

Install, see something useful about YOUR work, THEN get asked to share.

### Funnel Instrumentation

Track five steps: install started, install completed, auth completed, first scan, first
insight acknowledged (scrolled/clicked/exported/contributed). Measure elapsed time from
step 1 to step 5 and drop-off at each step. This is standard growth engineering, but it
requires treating the CLI as a product with a funnel.

---

## Single-Player Hooks (What You Get Before Sharing)

### Hook 1: Trajectory Replay

A local web UI where you browse recent AI sessions and replay them step-by-step. Not
metadata in a table -- an interactive timeline showing:

- What the agent was asked to do
- What tools it called, in what order
- Where it got stuck (high token consumption, low progress)
- Where it recovered
- Cost accumulation over time (running $/minute counter)

You can scrub forward/backward, compare sessions side-by-side ("why did yesterday's debug
take 4x longer?"), and annotate the timeline.

**Why it works for acquisition:** Developers constantly debate Claude Code vs. Cursor vs.
Codex but it's all anecdotal. Nobody has systematic data because nobody has a tool for
cross-harness session analysis. TC could be that tool.

Start with a basic timeline of tool calls with cost annotations. That's already more
useful than anything available for cross-harness analysis.

### Hook 2: Personal Analytics Dashboard

Weekly digest: cost breakdowns by agent, efficiency patterns, session quality trends.
Auto-generated, local-only. Becomes organic marketing when developers screenshot
dashboards and post "my AI coding cost me $X this week."

Key addition: cross-agent comparisons. "Your Claude Code sessions average $1.13/14 min.
Your Codex sessions average $1.40/22 min." This feeds the "which AI tool is best"
discourse, and every screenshot has a TC watermark.

### Hook 3: Local Quality Scoring

Frame scores as self-improvement feedback, not contribution gatekeeping. "Your average
session quality improved 12% this month" is motivating even if you never share a trace.
The scoring pipeline is one of TC's best technical assets -- let it work for the
individual.

### Hook 4: Session Backup and Search

AI sessions are ephemeral across every agent. A tool that indexes and searches all your
sessions across all agents solves: "I fixed this exact issue three weeks ago, let me find
that session." Low-effort, high-retention, natural lock-in.

---

## OTel as the Integration On-Ramp

Before OTel, every integration was bespoke -- a Claude Code adapter, a Codex adapter,
a Cursor adapter. Each is a maintenance burden.

With OTel, the pattern is: source emits `gen_ai.*` spans (many already do), TC accepts
them over OTLP, TC maps spans to envelopes via a version-pinned mapping layer.

Three consequences:

- **Any OTel-instrumented harness is automatically compatible.** No per-agent adapters.
- **Teams add TC as another OTel exporter.** Config change alongside Datadog/Grafana/Langfuse.
- **TC becomes composable** rather than competitive with existing observability.

### What TC Needs to Ship

1. **OTLP receiver** -- gRPC + HTTP/protobuf. Rust crates exist (opentelemetry-otlp, tonic).
2. **Mapping layer** -- `gen_ai.chat.completions` spans to TC envelope fields, pinned to
   OTel GenAI v1.42.0, abstracted for convention changes.
3. **OTel exporter in `tc` CLI** -- local analytics use the same data model; if a developer
   later adds Langfuse, their TC data is already compatible.
4. **One-page integration doc** -- "Send your existing OTel traces to TC in 5 minutes."
   Config snippets for Collector, Langfuse, direct SDK.

OTel doesn't solve privacy/scrubbing (raw code in spans), consent, or quality scoring. But
format interop alone is massive friction reduction.

---

## The Error Hub: Where Developers Gather

Developers gather where debugging happens. Stack Overflow isn't popular because it's a nice
website -- it's where you go when you're stuck.

TC could become the Stack Overflow of agent failures.

### How It Works

TC already has trace collection, privacy scrubbing, quality/novelty scoring, TEE attestation,
and credit compensation. Adding failure attribution means:

1. Detect that a session ended in failure (agent gave up, user interrupted, tests failed)
2. Attribute the root cause to a specific step or pattern
3. Bundle the failure context: what went wrong, why, and what fixed it
4. Score the bundle for novelty/usefulness and publish to commons (with scrubbing + consent)

### Example Flow

```text
$ tc debug
Analyzing your recent failed sessions...

Found 3 failed sessions in the last 7 days:

  1. fix-auth-middleware (failed: test assertion)
     Root cause: Agent modified wrong config file
     Similar failures in commons: 47 matches
     Most common fix: Provide explicit file path in prompt

  2. refactor-api-routes (failed: user interrupted)
     Root cause: Agent entered retry loop on import resolution
     Similar failures in commons: 123 matches
     Most common fix: Pin dependency version before starting

  3. deploy-script (failed: build error)
     Root cause: Agent used deprecated API
     Similar failures in commons: 12 matches
     Most common fix: Include changelog/migration guide in context

Contribute these failure bundles to help others? [Y/n]
```

You learn about your own failures AND see that others hit the same problems. The
contribution ask feels natural.

### The Flywheel

Developer hits failure, searches TC's failure commons, finds a fix from someone else's
bundle, contributes their own failures, now they're in the ecosystem -- personal analytics,
replay, eventually success traces too. Failure commons is top-of-funnel. Personal analytics
is retention. General trace contribution is steady state.

---

## Skill Publishing as Viral Distribution

1. TC's corpus accumulates cross-developer, cross-codebase traces
2. Extraction pipeline identifies recurring high-quality patterns (e.g., "when debugging a
   flaky test, experienced developers check the dependency graph before modifying the test")
3. Patterns get published as SKILL.md files per the Agent Skills spec
4. Any of the ~40 compatible products can discover and use these skills
5. Each SKILL.md credits TraceCommons and links to the corpus

When a Cursor user loads a TC-published skill and it helps them, they see "Source:
TraceCommons." Some percentage click through and install TC. Acquisition cost: zero.

**Starting small:** Manually curate 10-20 skills from high-scoring traces. Publish on a TC
GitHub repo. Test whether the viral distribution theory holds before building automated
extraction.

---

## Distribution Channels

### Claude Code Post-Session Hook

A TC hook that runs after each session: `tc scan --last --quiet`. Silently scores, stores
locally, auto-submits if opted in. Must run in <2 seconds and be a silent no-op if TC isn't
configured.

### OTel Collector Integration

For teams already running OTel: add TC as an exporter alongside Datadog/Grafana. Config
change, not code change.

### Background Daemon (PR #244)

Already in development. Silent by default (weekly digest), configurable quality thresholds
for auto-submission, easy pause/resume. The daemon is the retention mechanism -- once
configured, contribution is automatic.

### Content Marketing Through Corpus Analysis

Possibly the highest-leverage short-term channel. Publish analysis from TC's corpus:
"What we learned from N AI coding sessions," "The 5 most common failure patterns," "The
real cost of AI coding." Each links back to TC. Must be genuine analysis, not marketing.
Present limitations honestly ("352 submissions, mostly Rust developers").

### VS Code Extension (Lower Priority)

Save for when CLI + daemon have traction. Sidebar with session scores, inline annotations,
one-click replay.

---

## Quick Wins (2-4 Weeks Each)

Ordered by expected impact per effort.

### 1. Prebuilt Binaries and One-Line Install

GitHub Actions matrix build (linux-x64, linux-arm64, macos-x64, macos-arm64, windows-x64),
shell installer, Homebrew formula. Every developer without Rust installed is currently
blocked. This is the highest-leverage single change. Boilerplate CI from ripgrep/fd/bat.

### 2. Self-Service Registration

Drop the DM-for-invite-code flow. GitHub OAuth, agree to terms, authenticated immediately.
Keep invite codes for elevated trust levels. Every minute between "I want to try this" and
"I'm using this" is a cliff. The invite-code flow adds minutes (if Zaki is online) or days
(if not).

### 3. `tc scan` With Immediate Local Insights

When a user runs `tc scan`, show quality scores, cost estimates, efficiency patterns, tool
usage breakdown. Don't just list sessions -- analyze them. This is the "time to first
insight" moment. Scoring heuristics and trace parsing exist; the work is in presentation.

### 4. Immediate Feedback After Contribution

After submitting: show scores, comparison to corpus average, what made traces interesting.
Not "envelope received" -- "your trace scored 92/100 because it demonstrated a novel
approach to [pattern]." Contribution without feedback feels like shouting into a void.

### 5. The First Analysis Post

Publish "What we learned from analyzing N AI coding sessions." Average costs, tool-use
patterns, failure rates, cross-harness differences. TC has data nobody else has. If it gets
>100 HN points, that's a strong acquisition signal.

### 6. `tc doctor`

Diagnostic command: verifies installation, checks for detected agent sessions, validates
privacy settings, confirms auth status. A few hours of work. Should resolve >50% of setup
issues without human intervention.

### 7. OTLP Ingest Prototype

Basic OTLP receiver handling the common path (chat completions with tool calls from a
standard OTel-instrumented harness). 2-4 weeks. Validates that TC is interoperable with
the observability ecosystem. Success metric: one team pipes their existing OTel agent
traces to TC with just a config change.

---

## What Not to Do

**Don't lead with crypto.** Don't mention NEAR, tokens, or blockchain in the README's first
paragraph or the HN launch post. The pitch is: "Understand your AI coding sessions.
Optionally contribute and get compensated." For Show HN, lead with the privacy architecture
(TEE, honest DP, contributor-controlled consent). HN loves novel architecture, not token
economics.

**Don't over-gamify.** Leaderboards and quality scores (opt-in, reflecting real analysis)
are fine. Streak mechanics, artificial scarcity, and engagement hooks are poison.

**Don't over-promise on privacy.** TC's privacy is strong, but scrubbing can't guarantee no
identifying info remains in trace structure or timing patterns. TEEs have known side-channel
vulnerabilities. If TC can correlate traces to contributors (needed for credits), traces are
pseudonymous, not anonymous. Use the right word.

**Don't build integrations nobody asked for.** Ship OTel ingest first -- broadest coverage
from a single implementation. Then build dedicated integrations for agents your *actual
users* are connecting, in the order demand reveals.

**Don't require contribution for value.** If any single-player feature (replay, dashboard,
scoring, search) requires contributing traces, the acquisition funnel is broken. The
baseline must work fully on local data. Contribution unlocks additional value (commons
insights, comparisons, credits) but is never required.

---

## How You'll Know It's Working

Not a timeline. Milestones that indicate the machine is turning.

**Install-to-insight conversion > 30%.** Of people who run the install script, >30% should
complete `tc scan` and see results the same day. Currently ~0% (no install script, no
immediate insight).

**Organic signups without outreach.** When >50% of weekly signups come from people who
weren't directly asked -- blog posts, SKILL.md credits, HN comments, word of mouth -- the
growth engine is starting.

**Repeat contribution without prompting.** Day-7 retention: what percentage of new
contributors submit again within 7 days without being asked? Below 10% = value proposition
isn't landing. Above 30% = something is working. Track separately from daemon auto-submissions.

**Contributors > 10 unique per week.** Currently ~2 regular. 10/week means ~40-50
submissions/week, reaching 1,000 scored traces in ~5 months -- the rough threshold where
aggregate patterns become statistically meaningful.

**Someone writes about TC unprompted.** Blog post, tweet, HN comment where someone mentions
TC without being asked. Strongest signal of product-market fit. Can't manufacture it; can
only create conditions by making the product genuinely useful.

**Failure commons gets cited in debugging conversations.** Lagging indicator, but the one
that matters most. Tools that help when you're stuck are tools you keep.

---

TC's challenge is not "build a better product." The product is technically impressive. The
challenge is "get it in front of developers in a form they can use in under 2 minutes."

The hard part isn't knowing what to build. It's choosing which 2-3 things to build first
and shipping them fast enough that momentum builds. Prebuilt binaries, self-service
registration, and `tc scan` with immediate insights are the obvious first three. Everything
else waits until those are working and install-to-insight conversion is above 30%.

*August 2026. Brainstorming, opinionated, not a spec. Build the thing that makes developers
say "oh, that's useful" within 90 seconds of installing it.*
