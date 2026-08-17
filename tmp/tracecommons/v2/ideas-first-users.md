# Ideas for Getting TraceCommons' First 100+ Contributors

*Brainstorming doc — not a plan, just a pile of ideas worth arguing about.*

---

## 1. The Cold Start Challenge

TraceCommons has a cold start problem, but it's a weird one. Most two-sided
marketplaces need to solve the "chicken and egg" — get buyers before sellers
show up, or vice versa. NFX catalogued 19 tactics for this and most of them
assume you're dealing with two distinct populations who want different things.

TC's situation is different: buyers ARE sellers. Every developer who contributes
traces also wants to learn from other developers' traces. A frontend engineer
submitting their Claude Code sessions also wants to see how experienced Rust
developers prompt their agents, what tool-use patterns lead to higher success
rates, what the difference looks like between a 40-minute flailing session and
a 6-minute clean solve.

This is actually a huge structural advantage. You don't need to recruit two
separate populations and hold one's attention while you build the other. You
need one group of people who are already doing the activity (coding with AI
agents) and who would benefit from seeing aggregate patterns in that activity.

The challenge is more mundane: right now the onboarding flow requires installing
Rust, building from source, getting a DM'd invite code, and manually running
`tc-contributor submit` after each session. That's not a cold start problem,
that's a friction problem. The demand probably exists — you just can't reach it
through a wall of `cargo build --release`.

The real cold start question is: what's the minimum corpus size where the
commons becomes genuinely useful? If there are 50 traces in the system, can
you learn anything from them? Probably not much. At 1,000? Maybe. At 10,000
with good metadata? Now you're starting to see patterns — which agent
configurations work for which kinds of tasks, where tool-use diverges between
experts and novices, what the actual cost distribution looks like across
real-world sessions.

So the goal isn't just "get 100 contributors" — it's "get to a corpus size
where contributors start getting value back," and do it fast enough that early
contributors don't churn before the network effects kick in.


## 2. Single-Player Value First

This is probably the most important idea in this whole document.

Right now, contributing to TraceCommons is an altruistic act. You scrub your
traces, submit them, and... wait. Your traces go into a pool. Eventually the
commons might be useful to someone. You get some NEAR credits, but if you
weren't already holding NEAR, that's not immediately compelling.

The insight from every successful developer tool that started with a
contribution model — Sentry, PostHog, even npm — is that you need to give
people value BEFORE they share anything. Sentry doesn't start with "contribute
your crash reports to the community." It starts with "see your own crashes,
organized and searchable, for free." The sharing comes later, almost as a
side effect.

What would single-player value look like for TraceCommons?

**Personal trace analytics.** Even before you contribute anything, the
`tc-contributor` tool could analyze your local sessions and show you:
- Which tools your agent uses most (and which it never touches)
- Success vs. failure rates per session type
- Average cost per session, broken down by task category
- How your agent's behavior has changed over time
- Sessions where the agent spun its wheels (high token count, low progress)

This is data developers already want. People paste their Claude Code session
costs into Twitter threads. They argue about whether Cursor or Claude Code is
more efficient. They have no systematic way to answer these questions for their
own usage, let alone across a population.

**Local quality scoring.** The TEE scoring pipeline is one of TC's best
technical assets. One approach that could work well: run a local version of the
scoring heuristics (not the full TEE pipeline, just the novelty and quality
signals) and show developers their scores before they decide to contribute.

Something like:

```
$ tc-contributor scan
Found 47 sessions from the last 7 days.

  Session                Score   Novelty   Size
  ─────────────────────────────────────────────
  fix-auth-bug           92/100  High      2.1K tokens
  refactor-router        84/100  Medium    4.7K tokens
  update-readme          31/100  Low       890 tokens
  debug-ci-pipeline      88/100  High      3.3K tokens
  ... 43 more sessions

12 sessions scored above quality threshold (70+).
Contribute these? [Y/n]
```

Now the contributor understands what's valuable and why. The act of contributing
feels informed rather than opaque.

**Personal efficiency tracking.** Over time, show trends:
- "Your average session quality has improved 12% this month"
- "You're using the file search tool 3x more than last month — your sessions
  are getting shorter"
- "Sessions where you provide context upfront score 40% higher"

This is the kind of thing people would install the tool *just* for, even if they
never contribute a single trace. And once they have it installed, the distance
from "see my own stats" to "opt into sharing" shrinks dramatically.

PR #241 (private contributor insight via TEE) is heading in this direction.
It might be worth making this the flagship feature rather than a follow-on — the
thing you lead with in the README and the launch post.

**Backup and organization.** A more prosaic but genuinely useful angle: AI
coding sessions are ephemeral by default. Claude Code sessions disappear,
Codex sessions are hard to find later. Even acting as a structured backup and
search tool for your own sessions has value. "I solved this exact problem three
weeks ago, let me find that session" — that's a real use case.


## 3. The 60-Second Onboarding

The current onboarding flow is roughly:

1. Have Rust installed (or install it — 5 minutes if you're lucky)
2. `git clone` and `cargo build --release` (2-10 minutes depending on machine)
3. Get an invite code from an operator via DM
4. `tc-contributor login --invite '<link>'`
5. Answer 5 consent prompts
6. `tc-contributor list` to see what's available
7. `tc-contributor submit --since 7d`

That's maybe 15-20 minutes for someone who already has Rust, and potentially
30+ minutes for someone who doesn't. Every minute of onboarding is a cliff
where you lose people.

The ideal flow looks more like:

```
$ curl -fsSL https://tracecommons.org/install.sh | sh
# or: brew install tracecommons

$ tc-contributor auth
# Opens browser → OAuth with GitHub/Google → done

$ tc-contributor scan
Found 47 sessions. 12 scored above quality threshold.
Contribute? [Y/n]

12 traces contributed. You earned 156 credits.
Best trace scored 94/100 — nice.
```

Under 60 seconds from "I heard about this" to "I contributed and got feedback."

The technical blockers for this are well-understood:

**Prebuilt binaries.** This is the single highest-leverage technical investment
for contributor growth. Every successful Rust CLI tool (ripgrep, fd, bat,
starship, delta) ships prebuilt binaries for major platforms. The pattern is
well-established: GitHub Actions matrix build for linux-x64, linux-arm64,
macos-x64, macos-arm64, windows-x64, then a shell installer that detects
platform and downloads the right one. It's a weekend of CI work, not a month.

Sentry's growth playbook is instructive here. They obsessed over "time to
first event" — the elapsed time from landing on the docs page to seeing your
first crash report in the dashboard. They got it under 5 minutes and that
became their primary growth metric. PostHog had a similar focus on "time to
insight." For TC, the equivalent metric would be "time to first contribution
with feedback."

**Self-service registration.** The invite-code flow makes sense for a private
beta, but it's a hard wall for organic growth. If someone reads a blog post
and wants to try TC, they currently have to... find Zaki and ask for a code?
That's fine for 20 people, not for 100.

One approach: keep the invite system but make codes self-service. A web page
where you sign in with GitHub, agree to terms, and get a code instantly. Or
just drop invite codes entirely for the initial registration and use them only
for elevated trust levels.

**Immediate feedback.** After submitting, the contributor should see something
right away. Not "your envelope has been received" but "your trace scored 84/100
for novelty, here's why, and here's how it compares to the corpus average."
The leaderboard recomputation being a manual operator action is a bottleneck
here — even a rough local estimate would be better than silence.

**Setup verification.** A `tc-contributor doctor` command would help a lot:

```
$ tc-contributor doctor
  ✓ tc-contributor v0.4.2
  ✓ Authenticated as @willcrichton
  ✓ Claude Code detected (47 sessions found)
  ✗ Codex not detected
  ✓ Privacy filter: PII scrubbing enabled
  ✓ Consent: metadata + scrubbed traces
  ✓ NEAR wallet linked
  ─────────────────────────────
  Ready to contribute. Run `tc-contributor scan` to see eligible sessions.
```


## 4. Progressive Trust Escalation

The current consent flow asks for 5 scopes upfront: debugging, benchmark,
ranking training, model training, and public attribution. That's a lot of
decisions to make before you've gotten any value from the tool.

It might be worth trying a progressive model where contributors start with
minimal sharing and escalate as they build trust:

**Level 0 — Anonymous analytics only.**
You install the tool, it analyzes your sessions locally, and sends only
aggregate anonymous usage stats (session count, average length, agent type).
No trace content leaves your machine. You get the personal dashboard.

**Level 1 — Metadata.**
Tool names, token counts, timing, error rates. Still no code content. You
start appearing in aggregate statistics. Small credit accrual.

**Level 2 — Scrubbed traces.**
Structure and flow of sessions without code content. Tool calls with arguments
redacted. Enough to analyze agent behavior patterns. Moderate credits.

**Level 3 — Selective sharing.**
You review and pick which sessions to contribute. Full scrubbed content for
selected sessions. Higher credits, access to similar sessions from others.

**Level 4 — Auto-contribution.**
Background daemon submits sessions automatically after quality threshold.
Highest credit rate. Full access to commons insights.

The key is that each level unlocks more value — both credits and access to
insights from the commons. You can only see "how your tool usage compares to
the community" at Level 1+. You can only see "anonymized session patterns from
senior engineers" at Level 2+. And so on.

This mirrors how Cursor and Copilot handle data sharing. They start with
anonymous telemetry that's on by default, and let users opt into more detailed
sharing. The difference is that TC can make the value exchange explicit and
bidirectional — you share more, you see more.

One thing to be careful about: don't make lower levels feel punitive. If someone
stays at Level 1 forever, that's fine. They're still contributing useful
metadata. The escalation should feel like unlocking features, not like being
nagged to hand over more data.

The quarantine behavior in the current system (envelopes with message text get
quarantined by default) is a version of this idea, but it's invisible to the
contributor. They submit, their trace gets quarantined, and from their
perspective it just... disappeared. Making the trust levels explicit and
visible would help a lot.


## 5. Seeding the Commons

There's a bootstrapping problem: the commons isn't useful until it has enough
traces to show patterns, but people won't contribute until the commons is
useful. Classic cold start.

One approach that's worked for similar projects: seed the corpus with existing
public data so that early contributors are joining something that already has
content, not contributing into a void.

**Existing public datasets.** There are already agent trace datasets on
HuggingFace — Exgentic/agent-llm-traces is one, TraceLab published a dataset of
4,265 coding agent sessions. These aren't in TC's envelope format, but
converting them is straightforward. They wouldn't have the same trust properties
as contributor-submitted traces (no TEE attestation, no consent chain), but they
could be clearly labeled as "seed corpus" and used for the analytics and pattern
features.

**Founding traces.** The TC team's own development traces are an obvious seed
source. Building TraceCommons itself generates hundreds of agent sessions. These
are traces from experienced developers working on a real, complex Rust codebase —
exactly the kind of high-quality traces the commons wants. Contributing them as
a "genesis corpus" with the team's consent demonstrates skin in the game and
gives the scoring pipeline something to calibrate against.

NFX calls this "making supply look bigger" (Tactic 4 in their 19 Tactics
framework). The idea isn't to fake it — the seed data should be clearly labeled
as imported or founding — but to ensure that when the first external contributor
looks at the commons, they see something alive rather than an empty room.

**Synthetic benchmarks.** Another angle: create a set of benchmark tasks
(implement a REST API, fix a concurrency bug, refactor a module) and run them
across multiple agents, then publish the traces as a reference corpus. This has
the side benefit of creating a reproducible evaluation set that the community
can extend.

The goal is to get to ~1,000 scored traces before the first public launch. At
that point you can credibly say "here are real patterns we've found in 1,000
AI coding sessions" and that becomes both the marketing and the proof of value.


## 6. The Founding Contributor Program

Before going public, recruit 10-20 teams from the personal network. Zaki's
network in the crypto and infrastructure world is deep — there are plenty of
teams using Claude Code or Codex daily who'd try a tool if asked directly.

The framing matters: this isn't "be our beta testers" (which sounds like free
QA work). It's "join as a founding contributor with permanent status."

What founding contributors get:
- **Permanent "Founding Contributor" badge** — visible in the commons, can't be
  earned later
- **Elevated credit multipliers** (2-3x) for the first 6 months
- **Direct feedback channel** — weekly sync, immediate bug fixes, feature
  requests get priority
- **Early access** to insights and analytics features
- **Input on governance** — founding contributors help set norms for the commons

What you get from them:
- Real usage data from diverse codebases and workflows
- Direct feedback on friction points (you'll find 10 things wrong with the
  onboarding flow that you never would have caught internally)
- Case studies and testimonials for the public launch
- A warm body of content in the commons when you open the doors

The Langfuse playbook is relevant here. They launched to their YC batch first —
a captive audience of ~200 companies who were all building with LLMs and all
needed observability. By the time they launched publicly, they had enough usage
data and testimonials to be credible. TC doesn't have YC batch access, but the
NEAR ecosystem and Cosmos/IBC community provide a similar warm network.

One specific idea: recruit founding contributors in pairs or small teams rather
than individuals. A team of 3-4 developers at a company will hold each other
accountable for actually using the tool, and their traces create a
within-team comparison that's immediately interesting ("why do Alice's sessions
score higher than Bob's?").

The private beta should run for 4-6 weeks. Long enough to iterate on real
feedback, short enough to maintain momentum toward public launch. Have a clear
end date and a clear transition to public availability.


## 7. Distribution Through Integration

The best contributor acquisition is invisible. Instead of asking developers to
adopt a new tool and remember to use it, embed contribution into tools they're
already using.

**IronClaw integration (already built).** This is the most immediate
opportunity. The IronClaw integration is substantially merged — 3 PRs, 20K+
lines. If IronClaw users can contribute traces as a toggle in their existing
workflow, that's distribution without acquisition cost. The question is how
many IronClaw users there are and whether the integration surfaces the value
proposition clearly.

**Claude Code post-session hook.** Claude Code supports custom hooks that run
after each session. A TC hook that auto-scans and prompts for contribution at
the end of each session would be nearly frictionless:

```
Session complete. This trace scored 86/100.
Contribute to TraceCommons? [Y/n/always/never]
```

This is speculative — it depends on Claude Code's hook API being flexible
enough — but it's the kind of integration that makes contribution feel like
a natural part of the workflow rather than a separate step.

**VS Code extension.** A sidebar panel that shows your session history, quality
scores, and a one-click contribute button. This is higher investment than a
CLI tool but reaches a much larger audience. VS Code extensions also benefit
from marketplace discovery — people search for "AI coding analytics" and find TC.

**Langfuse / Braintrust bridge.** Teams already using LLM observability tools
have their traces in structured formats. A bridge that imports from Langfuse or
Braintrust with appropriate scrubbing would tap into an existing population of
teams who are already comfortable with trace analysis and would understand the
value proposition immediately.

**Background daemon.** PR #244 adds a background daemon with IPC. This is
crucial for the "invisible contribution" model. Once a contributor opts in,
their traces flow automatically without any manual `submit` commands. The
daemon watches for new sessions, scores them locally, and submits ones above
threshold. The contributor just... codes normally, and contributions happen.

The general principle: every integration point should reduce the number of
deliberate actions required to contribute from "several" to "one" to "zero."


## 8. Community and Transparency

Developer trust is the product. If developers don't trust TC's privacy pipeline,
scoring algorithm, and credit distribution, they won't contribute no matter how
smooth the onboarding is.

**Publish everything.** The scoring algorithm, the credit formula, the privacy
scrubbing pipeline, the TEE attestation process — all of it should be not just
open source (it already is) but actively documented and explained. Not buried in
code comments, but front-and-center in the docs with worked examples.

Something like:

> "Here's a sample trace. Here's what our scrubber removes. Here's what the
> TEE sees. Here's how the novelty score is computed. Here's how credits are
> allocated. Here's the exact formula."

This level of transparency is rare and it becomes a competitive advantage.
PostHog grew almost entirely through organic channels (they've cited 97%
organic growth) and a big part of that was radical transparency — public
handbook, public roadmap, public financials. Developers trust what they can
inspect.

**Monthly transparency report.** Publish monthly: how many traces in the
commons, how credits were distributed, what the top contributing patterns were,
any changes to scoring or scrubbing algorithms. This creates a heartbeat that
keeps the community engaged and holds the team accountable.

**"Show HN" launch.** When TC is ready for public launch, HN is the right
venue. The pitch should lead with the privacy architecture (TEE-based scoring,
contributor-controlled consent, scrubbing pipeline) rather than the crypto
mechanics. HN is skeptical of crypto projects but enthusiastic about
privacy-respecting developer tools with novel architectures.

A title like "Show HN: TraceCommons — A privacy-first commons for AI coding
traces, scored inside TEEs" would land better than anything mentioning tokens
or blockchain. Let the NEAR integration be a detail in the README, not the
headline.

**Documentation as marketing.** The concepts section of TC's docs — explaining
what trace analysis reveals about AI coding patterns, what novelty scoring
means, how the privacy pipeline works — is marketing content disguised as
documentation. People who read deep technical docs are exactly the audience TC
wants. Invest in this.

It might also be worth publishing analysis posts based on the seed corpus:
"What we learned from analyzing 1,000 AI coding sessions" is the kind of
content that gets shared in developer communities and naturally leads people
to the tool.


## 9. What NOT to Do

Some anti-patterns to avoid, drawn from watching developer tools get this wrong:

**Don't gamify with engagement mechanics.** Daily quests, XP bars, streak
counters, countdown timers, loot boxes — developers see through this instantly
and it poisons the relationship. The value exchange should be straightforward:
you contribute useful traces, you get credits and insights. No manipulation
needed.

This doesn't mean you can't have a leaderboard or contributor rankings — those
can be fine if they're opt-in and based on genuine contribution quality. But
there's a big difference between "you're a top-10 contributor this month"
(recognition of real value) and "log in 7 days in a row to earn a badge"
(engagement hacking).

**Don't auto-escalate sharing permissions.** If someone opts into Level 1
(metadata only), don't silently start collecting Level 2 data after 30 days.
Don't nag them to upgrade. Don't make Level 1 progressively less useful to
force escalation. Any trust violation here is catastrophic and permanent.

**Don't lock features behind contribution counts.** "Contribute 50 traces to
unlock the analytics dashboard" sounds like it incentivizes contribution, but
it actually creates an adversarial dynamic where contributors are grinding
toward a reward rather than sharing because they want to. It also incentivizes
low-quality bulk submissions. If the analytics dashboard is valuable, give it
to everyone and let the quality of contributions speak for itself.

**Don't spend money on ads before single-player value works.** Paid acquisition
for a tool that requires building from source and has no immediate feedback loop
is just burning money. Fix the onboarding, nail the single-player value, get
organic traction from founding contributors, and THEN consider amplification.

**Don't optimize for trace volume over trace quality.** 100 high-quality,
well-scored traces from experienced developers are worth more than 10,000
trivial "update readme" sessions. The scoring pipeline exists for this reason —
lean into it. Make contributors feel good about submitting fewer, higher-quality
traces rather than pressured to submit everything.

**Don't lead with crypto.** The NEAR integration is a feature, not the product.
Many developers have a reflexive negative reaction to anything that smells like
"web3 developer tool" regardless of the actual merits. Position TC as a
developer tool that happens to use blockchain for transparent compensation,
not as a crypto project that happens to involve coding traces.


## 10. A Suggested Sequence

Pulling the above ideas into a rough timeline. This isn't a plan — it's a
strawman for discussion about sequencing.

### Phase 1: Foundation (Weeks 1-4)

Focus: make the tool worth installing even if you never contribute.

- Ship prebuilt binaries (GitHub releases + install script + Homebrew)
- Implement local trace analytics (personal dashboard, quality scores)
- Add `tc-contributor doctor` for setup verification
- Self-service registration (drop invite codes for initial signup)
- Immediate feedback after submission (local score + comparison to corpus)
- Seed corpus: import public datasets + contribute team's own traces

The exit criteria: a developer can go from "never heard of TC" to "seeing
insights about their own coding sessions" in under 2 minutes, without
contributing anything.

### Phase 2: Founding Contributors (Weeks 5-8)

Focus: get 10-20 teams actively contributing and iterate on their feedback.

- Recruit founding contributors from personal network (pairs/teams preferred)
- Weekly feedback syncs, fast iteration on pain points
- Progressive trust levels (start everyone at Level 1, let them escalate)
- Background daemon (PR #244) merged and stable
- Quarantine flow made transparent to contributors
- Internal milestone: 500+ scored traces in the commons

The exit criteria: founding contributors are consistently using the tool
without being reminded, and at least some have escalated to Level 3+
voluntarily.

### Phase 3: Public Launch (Weeks 9-12)

Focus: go public with a credible corpus and clear value proposition.

- "Show HN" launch post focused on privacy architecture
- Published analysis: "What we learned from 1,000+ AI coding sessions"
- IronClaw integration highlighted as a distribution channel
- Documentation site with concepts, architecture, and worked examples
- Transparency report covering the founding contributor period
- Claude Code post-session hook (if feasible)
- Internal milestone: 100+ individual contributors, 2,000+ scored traces

The exit criteria: organic contributor signups are happening without direct
outreach from the team.

### Phase 4: Expand (Weeks 13+)

Focus: broaden reach and deepen value.

- VS Code extension for visual trace management
- Broader agent support (Cursor, Copilot, Windsurf, Cline)
- Langfuse/Braintrust bridge for teams with existing observability
- Hackathon sponsorships (contribute traces from the hackathon, win prizes)
- API access for researchers and tool builders
- Community-contributed scoring plugins
- Consider a "TC for Teams" tier with private analytics across a team's traces

This phase is more speculative — the specifics should be shaped by what
founding contributors and early public users actually ask for.


## References

- **NFX, "The NFX 19 Tactics to Solve the Cold Start Problem"** — Framework
  for two-sided marketplace bootstrapping. Tactics 1 (single-player mode),
  4 (make supply look bigger), and 7 (host-managed marketplace) are most
  relevant to TC.

- **Sentry growth playbook** — "Time to first event" as primary growth metric.
  Sub-5-minute onboarding. Value before network effects (you see YOUR crashes
  before you care about anyone else's).

- **PostHog's growth model** — 97% organic growth through radical transparency,
  documentation-led acquisition, and "time to insight" optimization. Public
  handbook, public roadmap.

- **Langfuse launch** — Used YC batch as first users. Small, captive audience
  of LLM-native teams who understood the problem. Iterated on their feedback
  before going public.

- **Cursor/Copilot/Windsurf data practices** — Progressive opt-in models for
  code telemetry. Useful reference for how mainstream tools handle the privacy
  vs. value tradeoff.

- **HuggingFace: Exgentic/agent-llm-traces** — Existing public dataset of
  agent LLM traces. Potential seed corpus source.

- **TraceLab coding agent sessions** — 4,265 coding agent sessions dataset.
  Another seed corpus candidate.

---

*These are ideas, not commitments. The right next step is probably to pick the
2-3 that feel most promising and actually try them, rather than trying to
execute all 10 sections in parallel.*
