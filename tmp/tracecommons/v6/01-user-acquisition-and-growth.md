# User Acquisition & Growth

*Brainstorming doc -- ideas with depth, not specs.*

**Date**: August 2026 (v6)

**What is TraceCommons?** TraceCommons (TC) is an open-source, Rust-based, privacy-preserving
registry of AI coding agent session traces. Contributors submit scrubbed traces of what their
AI agents did (Claude Code, Codex, IronClaw, etc.); quality and novelty are scored inside TEEs
(Trusted Execution Environments); contributors earn NEAR blockchain credits. Built by Zaki
Manian (Cosmos SDK, IBC). ~235K LOC Rust, 6 crates. Pilot on GCP. ~352 submissions, ~13/week,
3 contributors (brapse joined Aug 10), 6 GitHub stars.

**Key integration:** IronClaw is NEAR AI's agent runtime (12.6K GitHub stars) and TC's primary
integration partner (3 PRs merged, 20K+ lines).

**How scoring works:** The gate pipeline processes traces through: redaction, chunking,
embedding, similarity search, perplexity scoring, gate evaluation (accept/reject). Credit
formula: `q = f * g * a` where f = freshness, g = gate score, a = attestation weight.

**Project-blocking bugs:** Issue #210 (scoring logic inversion -- 0/99 sessions accepted) and
Issue #219 (redaction penalizes quality scores) must be fixed before any growth effort.

**Recent progress:** IronClaw integration substantially merged. Background daemon merged
(PR #244). Binary releases on tag merged (PR #240).

---

## The Core Problem (Unchanged)

TC's engineering is genuinely good -- honest differential privacy, TEE attestation, multi-gate
scoring. None of that matters yet because the user acquisition problem is a DevEx problem.
Developers adopt tools that give them something useful *immediately* and contribute to commons
where contribution feels like a byproduct. TC's current flow inverts this -- it asks for
contribution first and promises value later. Every idea in this doc flips that order.

---

## 1. Distribution: cargo-dist + One-Line Install

PR #240 shipped CLI binary releases on git tag but isn't using **cargo-dist** (v0.32.0), the
Rust ecosystem standard. ripgrep, fd, bat, delta, zoxide, starship all use it. cargo-dist
generates shell/PowerShell installers, Homebrew tap, GitHub Release artifacts with checksums,
and an npm wrapper. Install time: 5-30 seconds. The `update-informer` crate adds version
notifications on subsequent runs.

```text
# macOS/Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nicholasgasior/trace-commons/releases/latest/download/tc-installer.sh | sh

# Homebrew
brew install tracecommons/tap/tc

# npm (works everywhere Node does)
npx @tracecommons/tc scan
```

**What to do:** Replace PR #240's custom release CI with `cargo dist init` -> merge -> test on
5 targets. Effort: 1-2 days. cargo-dist handles the entire release pipeline.

### Time to First Insight

Sentry's North Star was "4 minutes 20 seconds to first event." PostHog had "time to insight"
under 5 minutes. TC's equivalent: **90 seconds from `curl install` to seeing insights about
your own AI coding sessions.**

Current: effectively infinite (build from source, DM Zaki for invite code, manual submission).

Target funnel:
1. Install (10 seconds, cargo-dist)
2. Auth (15 seconds, OAuth)
3. First scan (5 seconds, `tc scan`)
4. First insight (60 seconds, local analysis)
5. Contribution prompt (user-initiated, not forced)

Track install started -> install completed -> auth completed -> first scan -> first insight
acknowledged. Measure elapsed time and drop-off at each step.

---

## 2. Claude Code SessionEnd Hook (Hours of Work, Maximum Leverage)

Claude Code exposes **30 lifecycle hook events**. The relevant one:

```jsonc
// ~/.claude/hooks.json
{
  "hooks": {
    "SessionEnd": [{
      "matcher": {},
      "hooks": [{
        "type": "command",
        "command": "tc scan --last --quiet --auto-submit-if-opted-in"
      }]
    }]
  }
}
```

**Technical details:** Default timeout is 1.5s (hardcoded). Since v2.1.74, respects custom
`timeout` field. Network calls require the `nohup` + `disown` pattern to avoid blocking Claude
Code's exit. Third-party examples already exist: `opentelemetry-hooks`, `claude_telemetry`,
Langfuse hook.

**Why maximum leverage:** Zero ongoing friction once configured. No daemon required. Natural
opt-in moment during `tc init`. Composable with other hooks.

```bash
$ tc init
  Created ~/.tc/config.toml
  Detected Claude Code (v2.x)
  Install SessionEnd hook to auto-score sessions? [Y/n]
  Wrote ~/.claude/hooks.json
  Auto-submit to commons? [y/N/configure-threshold]
```

The hook must run in <2 seconds, be a silent no-op if TC isn't configured, never block Claude
Code's exit, and write results to `~/.tc/sessions/` for later review.

Other hookable events worth watching: **PreToolUse/PostToolUse** (real-time tool telemetry,
heavier -- save for v2) and **Stop** (user-interrupt pattern tracking).

---

## 3. Single-Player Hooks (What You Get Before Sharing)

Every feature below must work fully on local data. Contribution unlocks additional value but
is never a prerequisite.

### Hook 1: `tc scan` -- The "Time to First Insight" Moment

```text
$ tc scan
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

Install, see something useful about YOUR work, THEN get asked to share. Scoring heuristics and
trace parsing exist; the work is in presentation and insight generation.

### Hook 2: Session Backup and Search (Highest Retention)

AI sessions are ephemeral. A tool that indexes and searches all sessions across all agents
solves: "I fixed this exact issue three weeks ago, let me find that session."

```text
$ tc search "authentication middleware fix"
Found 3 matching sessions:

  2026-08-03  fix-auth-middleware    Claude Code    Quality: 92
    "Added Bearer token validation to the /api/v1/* routes..."

  2026-07-28  auth-refactor          Codex         Quality: 78
    "Moved auth logic from middleware to a shared utility..."

  2026-07-15  debug-jwt-expiry       Claude Code    Quality: 85
    "Fixed JWT expiry check that was comparing seconds to ms..."
```

### Hook 3: Personal Analytics Dashboard

Weekly digest: cost breakdowns by agent, efficiency patterns, session quality trends.
Cross-agent comparisons ("Claude Code: $1.13/14 min avg vs Codex: $1.40/22 min avg"). Every
screenshot has a TC watermark -- organic marketing.

**Competitive landscape:** TokenShift (PointFive, $60M Series B), Exceeds AI/Ink (code-level
provenance), and UseAI (free/open-source) track cross-agent costs. None combines cost tracking
with quality scoring and failure attribution. TC occupies that intersection.

### Hook 4: Local Quality Scoring

Frame scores as self-improvement feedback. "Your average session quality improved 12% this
month." The scoring pipeline is one of TC's best technical assets -- let it work for the
individual. **Critical dependency:** Issues #210 and #219 must be fixed first.

### Hook 5: Trajectory Replay

AgentGUI (ETH Zurich, arXiv:2607.26300, now confirmed) demonstrated that visual replay helps
users identify key trace elements 38% faster (p=0.023) and raises task completion by up to
34 percentage points for small agents via drift prevention. Start with a terminal viewer:
`tc replay <session-id>` -- step through events, show tool calls with timing and token counts,
highlight failure points. Web UI is v2.

**Contributor dashboard integration:** TC should integrate AgentGUI-style visualization
directly into its contributor dashboard. Contributors debugging their own traces get immediate
value (faster inspection while contributing), and the visualization doubles as a retention
hook -- seeing your trace replayed with scoring overlays is more engaging than a table of
numbers.

---

## 4. The Error Hub: Where Developers Gather

### The Flywheel

Developer hits failure -> searches TC's failure commons -> finds a fix -> contributes own
failures -> enters the ecosystem. **Failure commons is top-of-funnel.** Personal analytics is
retention. General trace contribution is steady state.

### `tc debug`

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

The contribution ask feels natural because you just received value from others' contributions.

### Why Failure Traces Are Easier to Share

Less proprietary risk -- "my agent spun in circles on an import error" reveals nothing
competitive. TraceLab (Zhu et al., University of Washington, 4,265 sessions) characterizes
coding agent workloads at scale -- sessions involve complex multi-step tool use and
frequent retries -- validating the demand for failure pattern intelligence.

### Gap Validated

r/ClaudeAI has 1M+ members, Claude Code gets 291 issues/day, Mozilla cq has 1,200 stars,
Stack Overflow for Agents launched beta June 2026. No session-trace-first failure database
exists. Causal Agent Replay (arXiv:2606.08275) shows correlational attribution only gets ~14%
step-level accuracy -- interventional methods needed.

**AgentDebugX** (arXiv:2607.18754, confirmed): The closest existing system to what TC's Error
Hub should become. AgentDebugX ships an opt-in Error Hub for sharing scrubbed
failure-diagnosis-repair bundles, and its DeepDebug core achieves 28.8% exact agent+step
accuracy on Who&When attribution (vs 21.7% baseline). On GAIA tasks, it repairs 13/73 failed
tasks in a single rerun, improving accuracy from 55.8% to 63.6%. **But AgentDebugX's Error Hub
is limited to GAIA benchmark tasks, not real coding agents.** TC should position itself as the
next-generation Error Hub: broader scope (all scored traces, not just failures), TEE-attested
privacy, NEAR credit incentives, and coding-agent focus. The pitch: "Your agent's failures are
someone else's training data."

**AgenTracer-8B** (arXiv:2509.03312, confirmed; ICLR 2026 status UNVERIFIED): First automated
framework for annotating failed multi-agent trajectories. Outperforms Gemini-2.5-Pro and
Claude-4-Sonnet by up to 18.18% on Who&When failure attribution. TC relevance: AgenTracer-8B
is a candidate scoring model for the failure attribution service inside TC's TEE pipeline, and
its TracerTraj dataset could seed TC's corpus if open-sourced.

**TRAIL** (arXiv:2505.08638, confirmed): Provides a three-domain failure taxonomy (Reasoning
Errors, System Execution Errors, Planning/Coordination Errors) validated across 148 traces,
1,987 OTel spans, and 841 annotated errors, all OTel/OpenInference compatible. TC's corpus at
~352 submissions already exceeds TRAIL's 148 traces. TRAIL's taxonomy is the natural candidate
for TC's canonical failure classification in the Error Hub.

**Interruptible Agents** (arXiv:2604.00892, confirmed): First systematic study of handling
user interruptions during long-horizon tasks. Identifies three interruption types: Addition,
Revision, and Retraction. Current models perform poorly on all three. Interrupted traces are
high-value Error Hub content because they capture a failure mode (inability to handle
mid-stream user corrections) that is extremely common in real coding sessions but
underrepresented in benchmark datasets.

**Human-Agent Collaboration Survey** (arXiv:2505.00753, confirmed): Identifies four
collaboration subtypes -- Delegation & Direct Command, Supervision, Cooperation, and
Coordination -- each producing fundamentally different trace structures. TC should capture
collaboration mode as a single enum field in the envelope schema. A supervised coding session
and a fully delegated one have different failure profiles, and scoring should account for this.

---

## 5. Skill Publishing as Viral Distribution

The Agent Skills ecosystem: 490K+ skills (**⚠️ UNSOURCED** — this figure cannot be traced to a primary source; official agentskills.io lists ~40 adopters, catalog sizes vary by directory), 32+ adopters (Claude Code, Codex, Cursor, Gemini
CLI, Windsurf), SkillsMP indexes 1.5M, skills.sh has 83K skills / 8M installs. ClawHavoc
incident exposed 341 malicious skills; existing scanners are bypassable.

TC's position: the only system that can produce skills with **provenance-verified quality
scores** and **security scanning** backed by a real trace corpus.

```yaml
# SKILL.md frontmatter
source: TraceCommons
quality_score: 92/100
security_scan: passed (2026-08-10)
provenance: tc://corpus/traces/abc123,def456,ghi789
install_tc: curl --proto '=https' --tlsv1.2 -LsSf https://...tc-installer.sh | sh
```

**Starting small:** Manually curate 10-20 skills from high-scoring traces. Publish on TC
GitHub repo. Measure click-through on `install_tc`. Don't build automated extraction until
manual curation proves the distribution theory.

---

## 6. Distribution Channels

**OTel as integration on-ramp.** OTel GenAI conventions are NOT stable (all "Development"
status). TC must pin attribute versions. But the pattern is correct: source emits spans -> TC
accepts over OTLP -> TC maps to envelopes. Any OTel-instrumented harness becomes automatically
compatible. Teams add TC as another exporter alongside Datadog/Grafana/Langfuse.

**Claude Code SessionEnd hook.** See section 2. Hours of work, maximum leverage.

**Background daemon (PR #244 -- merged).** Silent by default (weekly digest), configurable
thresholds, easy pause/resume. The retention mechanism.

**Telemetry opt-in UX.** Go telemetry aimed for 10-20% opt-in with mission-first framing.
GitHub CLI has the best payload transparency (suppress-and-preview). TC should target 5%
contribution rate among engaged users. Key principles: mission-first framing ("help improve AI
coding for everyone"), payload transparency (`tc preview`), granularity over binary (failures
only / high-quality only / all), name the recipient ("TraceCommons, an open-source project").

**IronClaw onboarding-time opt-in.** Surface TC contribution during agent setup flow.
Standing consent model (PR #4559) already supports one-tap opt-in.

**Content marketing through corpus analysis.** "What we learned from N AI coding sessions."
Reference TraceLab (Zhu et al., University of Washington, 4,265 sessions, 357K LLM steps)
as benchmark for workload scale and complexity. Cross-harness comparison is TC's unique angle.

**Vercel deploy button pattern.** "Deploy to Railway/Fly/GCP" for self-hosted TC instances.
One click from zero to running. The button itself circulates in blog posts and READMEs.

**VS Code extension.** Deferred. Save for when CLI + daemon have traction. Sidebar with
session scores, inline annotations, one-click replay. Building this before the CLI workflow is
proven is premature.

---

## 7. Quick Wins (2-4 Weeks Each)

1. **Prebuilt binaries and one-line install.** cargo-dist. Every developer without Rust
   installed is currently blocked. Highest-leverage single change.
2. **Self-service registration.** Drop the DM-for-invite-code flow. GitHub OAuth, agree to
   terms, authenticated immediately. Keep invite codes for elevated trust levels.
3. **`tc scan` with immediate local insights.** The "time to first insight" moment. Scoring
   and parsing exist; the work is in presentation.
4. **Immediate feedback after contribution.** Not "envelope received" -- "your trace scored
   92/100 because it demonstrated a novel approach to [pattern]." Contribution without
   feedback feels like shouting into a void.
5. **The first analysis post.** "What we learned from analyzing N AI coding sessions." If it
   gets >100 HN points, that's a strong acquisition signal.
6. **`tc doctor`.** Diagnostic command: verifies installation, checks for detected agent
   sessions, validates privacy settings, confirms auth status. A few hours of work. Resolves
   >50% of setup issues without human intervention.
7. **OTLP ingest prototype.** Basic OTLP receiver for the common path. 2-4 weeks. Success
   metric: one team pipes existing OTel traces to TC with just a config change.

---

## 8. What Not to Do

**Don't lead with crypto.** Lead with privacy architecture and developer utility.

**Don't ship broken scoring.** Issues #210/#219 first. A developer who sees 0/47 sessions
accepted will never come back.

**Don't claim OTel compatibility prematurely.** Say "supports OTel GenAI draft conventions."

**Don't overstate EU AI Act compliance.** Article 12 for standalone high-risk AI deferred to
Dec 2027. GPAI obligations are live. Use precise language in grant applications.

**Don't build integrations nobody asked for.** Ship OTel ingest first, then follow demand.

**Don't require contribution for value.** Every single-player feature must work on local data.

**Don't over-gamify.** Quality scores are fine. Streak mechanics and engagement hooks are
poison.

**Don't over-promise on privacy.** Scrubbing can't guarantee no identifying info in trace
structure or timing. TEEs have side-channel vulnerabilities. Traces are pseudonymous, not
anonymous. Use the right word.

---

## 9. Cold-Start Playbook

Evidence-backed (*The Cold Start Problem*, Andrew Chen):

- **Solve the hard/supply side first.** Hand-hold the first suppliers. With 3 contributors,
  that means personal onboarding for the next 20.
- **"Come for the tool, stay for the network."** `tc scan` with local insights -> contribution
  as byproduct. Nobody installs a tool to contribute to a commons.
- **Manufacture the first atomic network** in one narrow niche. Claude Code Rust traces until
  density self-sustains, then expand.
- **Bounded subsidy/prizes** to reach critical mass. NEAR credits taper as network effects
  take over. Don't create permanent subsidy dependency.
- **Sentry wizard model**: Auto-detect the user's agent, generate config, verify first scan
  in <90 seconds.
- **PostHog autocapture**: Default to capturing everything locally; let users opt into what
  to share.
- **Artifact-driven distribution**: Every TC output (skills, failure bundles, analytics
  screenshots) carries a branded breadcrumb back to TC.

---

## 10. Growth Milestones

Not a timeline. Milestones that indicate the machine is turning.

**Install-to-insight conversion > 30%.** Currently ~0%. Below 10% = install flow is broken or
first-run UX is confusing. Above 30% = value proposition is landing immediately.

**Organic signups > 50% of weekly total.** Below 20% = you're doing all the pushing. Above
50% = something is pulling people in without outreach.

**Day-7 repeat contribution > 30% (without daemon auto-submit).** Below 10% = value
proposition isn't landing -- people tried it, didn't find it useful enough to come back.
Above 30% = something is working. Track separately from daemon auto-submissions.

**Contributors > 10 unique per week.** Currently ~2 regular. 10/week means ~40-50
submissions/week, reaching 1,000 scored traces in ~5 months -- the threshold where aggregate
patterns become statistically meaningful.

**Someone writes about TC unprompted.** Strongest signal of product-market fit. Can't
manufacture it; can only create conditions by making the product genuinely useful.

**Failure commons gets cited in debugging conversations.** Lagging indicator, but the one that
matters most. Tools that help when you're stuck are tools you keep.

---

Prebuilt binaries, self-service registration, and `tc scan` with immediate insights are the
obvious first three. Everything else waits until those are working and install-to-insight
conversion is above 30%.

*Build the thing that makes developers say "oh, that's useful" within 90 seconds of installing
it.*
