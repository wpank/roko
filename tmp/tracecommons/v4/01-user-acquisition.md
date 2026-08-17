# User Acquisition & Growth

**Date**: August 2026

TraceCommons (TC) is an open-source, privacy-preserving AI trace registry. ~235K LOC Rust, TEE-scored quality/novelty, NEAR credit settlement. 3 contributors, ~352 submissions, 6 GitHub stars. IronClaw integration shipped (3 PRs, 20K+ lines).

The product is technically strong. The problem is getting it in front of developers in a form they can use in under 2 minutes.

---

## The Core Problem

TC asks for contribution first and promises value later. Every successful developer tool inverts this: give something useful immediately, make contribution a byproduct.

**Target metric**: Time to first insight < 90 seconds.

Currently: effectively infinite (build from source, DM for invite code, submit before seeing anything back).

---

## 1. Three Must-Ship Changes (Weeks, Not Months)

### 1.1 Prebuilt Binaries + One-Line Install

Every developer without Rust installed is currently blocked. This is the single highest-leverage change.

- GitHub Actions matrix build: linux-x64, linux-arm64, macos-x64, macos-arm64, windows-x64
- Shell installer: `curl -fsSL https://tracecommons.org/install.sh | sh`
- Homebrew formula
- Boilerplate CI from ripgrep/fd/bat (well-established patterns)

**Effort**: 2-3 days.

### 1.2 Self-Service Registration

Drop the DM-for-invite-code flow. GitHub OAuth → agree to terms → authenticated immediately. Keep invite codes for elevated trust levels only.

Every minute between "I want to try this" and "I'm using this" is a cliff. The invite-code flow adds minutes (if Zaki is online) or days (if not).

**Effort**: 1-2 days.

### 1.3 `tc scan` With Immediate Local Insights

The "time to first insight" moment. When a user runs `tc scan`, show quality scores, cost estimates, efficiency patterns, tool usage breakdown -- not just a session list.

```text
$ tc scan
Found 47 sessions from the last 7 days.

  Session                  Quality   Novelty   Cost     Duration
  ---------------------------------------------------------------
  fix-auth-middleware       92/100    High      $0.84    12 min
  refactor-db-queries       84/100    Medium    $2.10    34 min
  debug-ci-pipeline         88/100    High      $1.22    18 min
  ... 43 more

Weekly summary:
  47 sessions | $42.80 total cost | avg 14 min/session
  Your most efficient pattern: sessions with upfront context score 40% higher
  Your biggest time sink: 6 sessions spent >50% of tokens on retries

12 sessions scored above contribution threshold (70+).
Contribute to the commons? [Y/n/always/never]
```

Install → see something useful about YOUR work → THEN get asked to share.

**Effort**: 1-2 weeks (scoring heuristics and trace parsing exist; the work is presentation).

---

## 2. Single-Player Value (What You Get Before Sharing)

These features must work fully on local data. Contribution unlocks additional value but is never required.

### 2.1 Session Backup & Search

AI sessions are ephemeral across every agent. A tool that indexes and searches all sessions across all agents solves: "I fixed this exact issue three weeks ago, let me find that session."

Low-effort, high-retention, natural lock-in.

### 2.2 Personal Analytics Dashboard

Weekly digest: cost breakdowns by agent, efficiency patterns, session quality trends. Auto-generated, local-only.

Key addition: cross-agent comparisons. "Your Claude Code sessions average $1.13/14 min. Your Codex sessions average $1.40/22 min." This feeds the "which AI tool is best" discourse, and every screenshot has a TC watermark.

### 2.3 Local Quality Scoring

Frame scores as self-improvement feedback, not contribution gatekeeping. "Your average session quality improved 12% this month" is motivating even if you never share a trace.

### 2.4 Trajectory Replay (Phase 2)

Local web UI: browse recent AI sessions, replay step-by-step. Interactive timeline showing what the agent was asked, what tools it called, where it got stuck, cost accumulation over time. Start with a basic timeline of tool calls with cost annotations -- already more useful than anything available.

AgentGUI (ETH Zurich) showed 38% faster trace comprehension with visual replay.

---

## 3. Distribution Channels

### 3.1 OTel as the Integration On-Ramp

OTel GenAI conventions (v1.42.0) are the lingua franca. Any team already emitting OTel traces can pipe them to TC with a config change, not a code change. This flips TC from a destination into a receiver you add to your existing observability stack.

Three consequences:
- Any OTel-instrumented harness is automatically compatible
- Teams add TC as another OTel exporter alongside Datadog/Grafana/Langfuse
- TC becomes composable rather than competitive with existing observability

### 3.2 IronClaw Onboarding-Time Opt-In

Surface TC contribution during IronClaw's agent setup flow, not buried in settings. Standing consent model (PR #4559) already supports this. Lowest-effort, highest-impact change for IronClaw users.

### 3.3 Content Marketing Through Corpus Analysis

Highest-leverage short-term channel. Publish analysis from TC's corpus: "What we learned from N AI coding sessions," "The 5 most common failure patterns," "The real cost of AI coding." Each links back to TC. Present limitations honestly ("352 submissions, mostly Rust developers").

### 3.4 Claude Code Post-Session Hook

A TC hook that runs after each session: `tc scan --last --quiet`. Silently scores, stores locally, auto-submits if opted in. Must run in <2 seconds and be a silent no-op if TC isn't configured.

### 3.5 Background Daemon (PR #244)

Already in development. Silent by default (weekly digest), configurable quality thresholds for auto-submission. The daemon is the retention mechanism -- once configured, contribution is automatic.

---

## 4. The Error Hub: Where Developers Gather

Developers gather where debugging happens. TC could become the Stack Overflow of agent failures.

When an agent fails, TC attributes root cause, bundles the failure context (what went wrong, why, what fixed it), scores for novelty, and publishes to commons. Developers search "my agent keeps failing on this kind of task, has anyone else seen this?" and get back structured failure bundles.

```text
$ tc debug
Found 3 failed sessions in the last 7 days:

  1. fix-auth-middleware (failed: test assertion)
     Root cause: Agent modified wrong config file
     Similar failures in commons: 47 matches
     Most common fix: Provide explicit file path in prompt

  2. refactor-api-routes (failed: user interrupted)
     Root cause: Agent entered retry loop on import resolution
     Similar failures in commons: 123 matches
     Most common fix: Pin dependency version before starting
```

**Why failures work for growth:**
- Failure traces are psychologically easier to share (less concern about leaking proprietary patterns)
- Developer hits failure → searches TC → finds fix → contributes own failures → now in the ecosystem
- Failure commons is top-of-funnel, personal analytics is retention, general trace contribution is steady state

---

## 5. Skill Publishing as Viral Distribution

TC mines its corpus for recurring high-quality patterns and publishes them as SKILL.md files per the Agent Skills spec (~40 compatible products: Claude Code, Codex, Copilot, Cursor, Gemini CLI).

When a Cursor user loads a TC-published skill and it helps them, they see "Source: TraceCommons." Some percentage click through and install TC. Acquisition cost: zero.

**Security angle**: ToxicSkills research found 36.82% of scanned skills have security flaws. TC-published skills with quality scores and provenance chains are meaningfully more trustworthy.

**Start small**: Manually curate 10-20 skills from high-scoring traces. Test whether the viral distribution theory holds before building automated extraction.

---

## 6. What Not to Do

- **Don't lead with crypto.** Lead with the privacy architecture and developer utility. Blockchain details belong in the docs, not the README.
- **Don't over-gamify.** Leaderboards (opt-in) are fine. Streak mechanics and engagement hooks are poison.
- **Don't over-promise on privacy.** TC's privacy is strong but not perfect (side channels, pseudonymous not anonymous). Use the right word.
- **Don't require contribution for value.** If any single-player feature requires contributing traces, the acquisition funnel is broken.
- **Don't build integrations nobody asked for.** Ship OTel ingest first, then build dedicated integrations in the order demand reveals.

---

## 7. Growth Milestones

Not a timeline. Signals that the machine is turning.

| Signal | Target | Current |
|---|---|---|
| Install-to-insight conversion | >30% same-day | ~0% |
| Organic signups (no outreach) | >50% of weekly signups | N/A |
| Day-7 repeat contribution | >30% without prompting | Unknown |
| Unique contributors/week | >10 | ~2 |
| Unprompted mention (blog, tweet, HN) | Any | None |
| Failure commons cited in debugging | Any | None |

---

## Priority Order

1. Prebuilt binaries + install script (2-3 days)
2. Self-service registration (1-2 days)
3. `tc scan` with immediate insights (1-2 weeks)
4. Immediate feedback after contribution (1 week)
5. First analysis post (1 week)
6. OTel ingest prototype (2-4 weeks)
7. Error Hub MVP (6-8 weeks)
8. Trajectory replay (8-10 weeks)

Everything else waits until install-to-insight conversion is above 30%.
