# Consolidated Roadmap

> **TL;DR**: The 30 previous refinement docs propose dozens of
> individual workstreams. Landing them in the wrong order produces
> wasted effort (doing HDC before demurrage before heuristics
> means redoing all three). This doc sequences the work into a
> six-to-twelve-month roadmap with explicit dependencies, effort
> estimates, and milestones. It is the single answer to "what do
> we build next?" across every other doc in this folder.

> **For first-time readers**: This doc is the sequencing layer. Read
> the individual refinement docs for the *why* and *how*; read this
> for the *when* and *in what order*. Every item cites its home doc
> so you can drill in. Nothing here is net-new design; it's all
> assembled from 02–34.

## 1. Design principles of the sequencing

- **Dependency order first.** A refinement whose substrate assumes
  another refinement's primitive lands after it, not before.
- **Risk budget at each phase.** No phase has more than one "high-
  risk" workstream (kernel rewrite, demurrage rate-tuning,
  multi-tenancy split). Risk stacks poorly.
- **Ship a user-visible win per quarter.** Long silent refactors
  kill morale. Each quarter closes with a demo-able result.
- **Parallelize the independent.** Foundation, learning, UX, and
  ecosystem tracks run in parallel when crate deps allow.
- **Non-blocking for ux-followup.** The refinements don't block or
  require the `tmp/ux-followup/` P0/P1 items; both advance on
  separate timelines.

## 2. The dependency graph

Simplified — nodes are refinement numbers; arrows mean "must land
after." Dashed arrows mean "benefits from but doesn't require."

```
01 critique ──▶ 02 Pulse ──▶ 03 Bus ──▶ 04 operators ──▶ 05 loop
                  │              │
                  ▼              ▼
                06 plan ────▶ 07 naming ──▶ 08 code
                              │
                              ▼
                            20 modularity
                              │
                              ▼
                 ┌──────────┐ │ ┌────────────┐ ┌────────────┐
                 │11 HDC    │◀┼─┤12 demurr.  │ │14 heurist. │
                 └──────────┘ │ └────────────┘ └────────────┘
                      │       │       │              │
                      ▼       ▼       ▼              ▼
                    10 self-learning ◀────────── 16 research
                              │
                              ▼
                    13 c-factor
                              │
                              ▼
                    15 scaling ◀─── 17 plugins ◀── 25 domains
                              │
                              ▼
                    18 moat, 19 catalog, 21 rewrites
                              │
                              ▼
                 ┌──────────┐ ┌──────────┐ ┌──────────┐
                 │26 State  │ │27 realtime│ │22 dev UX │
                 │  Hub     │ │  surface │ │          │
                 └──────────┘ └──────────┘ └──────────┘
                      │           │            │
                      ▼           ▼            ▼
                    23 user UX, 28 CLI, 29 web UI, 30 primitives
                      │           │            │
                      ▼           ▼            ▼
                    24 deployment
                      │
                      ▼
                    32 safety, 33 observability
                      │
                      ▼
                    09 phase-2, chain+mesh

                  31 synergy ── 34 glossary ── (orthogonal)
```

Refinements 01, 02, 03, 04, 05, 06, 07, 08 form the critical path:
nothing else ships without them. Refinements 20 (modularity) and 22
(dev UX) gate most of the ecosystem work.

## 3. Six-to-twelve-month roadmap by quarter

Each quarter names its headline deliverable — the user-visible win —
and the supporting tracks.

### Quarter 1 — Foundation

**Headline**: The two-medium kernel ships. Existing subsystems migrate
off ad-hoc event enums. Two P0 self-hosting closures land.

Tracks:

- **Foundation / kernel** (docs 01–09): Phase A + B + C of
  `06-refactoring-plan.md`. Pulse, Bus trait, Datum, operator
  generalization, seven-step loop, conductor migration,
  self-hosting PlanRevisionPolicy + PrdPublishPolicy.
- **Modularity** (doc 20): Extract `roko-bus` crate; scaffold
  `roko-spi`; CI enforcement of dep graph rules.
- **Naming & glossary** (docs 07, 34): Doc-level rename pass; the
  glossary lands alongside.
- **Observability** (doc 33 §17.1–§17.2): Roko-specific metrics
  wired; default Grafana dashboards shipped.

Risk: kernel refactor (high). Mitigations: feature-flag in 06 §6.1;
test parity before/after.

Demo: `roko plan run` on a PRD that's auto-generated from a published
PRD idea. No human touches the plan step.

### Quarter 2 — Learning substrate

**Headline**: HDC fingerprints everywhere, demurrage shipping,
heuristics becoming a real library, c-factor visible in dashboards.

Tracks:

- **HDC on every Engram** (11): field added; default encoder
  registered; `query_similar` on FileSubstrate.
- **Demurrage** (12): balance/reinforcement; cold tier; dashboard
  tile.
- **Heuristics** (14): type + Calibrator + CLI surface.
- **Self-learning** (10 §9): prediction/outcome topics;
  CalibrationPolicy; TUI F4 tab updates.
- **c-factor measurement** (13 §10 steps 1–2): metrics, dashboard
  tile. No Policy-level actuation yet.
- **Research-to-runtime** (16 §12 steps 1–3): Paper + Claim
  Engrams; starter kit of 20 papers.

Risk: demurrage rate-tuning (high). Mitigations: per-deployment
overrides; sliding-window CI to detect cold-tier blow-up;
opt-out in roko.toml.

Demo: `roko heuristic list` shows calibrated starter library; the
web UI Beliefs page renders it; c-factor gauge moves in real time
on a two-agent plan.

### Quarter 3 — Ecosystem and UX

**Headline**: Plugins are installable. StateHub is kernel-tier.
Realtime wire surface speaks a stable protocol. Web UI first
release. CLI parity with Claude Code shipping.

Tracks:

- **Plugin SPI** (17): Stage A + B + C — tier-3 tool manifests,
  tier-1/2 prompt/profile plugins, tier-4 ABI bridge.
- **StateHub rearchitecture** (26): kernel crate; canonical
  projections; in-process API; tests.
- **Realtime event surface** (27): WebSocket + SSE; wire protocol
  v1 frozen; TypeScript + Python + Rust clients.
- **Developer UX** (22): one-liner + builder API; four-layer SDK
  docs; `examples/` directory.
- **User UX** (23): interactive `roko init`; unified verb set; TUI
  goes interactive.
- **CLI parity** (28): slash commands; diff-first output; transcripts
  + resumption; budget visibility.
- **Web UI** (29): Home + Chat pages shipping; Plans + Beliefs in
  beta.
- **Rich UX primitives** (30): token streams; tool banners; gate
  badges; heuristic footnotes.
- **Deployment UX** (24): state export/import; Docker + Compose;
  single-server profile.

Risk: web UI scope creep (medium). Mitigations: strict five-page
cap; shadcn + Tailwind for speed; no custom design system.

Demo: an external developer ships a tier-3 tool plugin in one
afternoon and sees it running in the TUI, CLI, and web UI
simultaneously.

### Quarter 4 — Scale, safety, domains

**Headline**: Six domain profiles shipping. Safety spine visible
across every surface. Multi-tenant deployment model. Helm chart.

Tracks:

- **Domain profiles** (25): coding, research, blockchain, data, ops,
  writing. Each ships TypedContext schema + starter heuristics
  + gates + profile manifest.
- **Safety spine** (32): custody records for destructive actions;
  tier-5 WASM host; audit CLI.
- **Replication ledger** (16 §12 steps 4–7): ledger + export;
  watchdogs; provenance injection in prompts.
- **Deployment maturation** (24): multi-tenancy; OIDC; Helm chart;
  state portability at scale.
- **c-factor actuation** (13 §10 steps 3–4): Policy acts on c when
  process variables drop; devil's advocate; outsider injection.
- **Scaling instrumentation** (15): KPI dashboards; anti-metric
  alerts; kill-switch CLIs.
- **Commons** (14 §10): cross-deployment heuristic import/export;
  signature-based trust gradient.

Risk: multi-tenancy scope (high). Mitigations: start with
tenant-scoped Substrate namespace + manual auth; OIDC in a second
step.

Demo: a blockchain team installs the blockchain profile, runs a
simulated chain op with custody records and audit trail; an
observer in the web UI sees c-factor, cost, safety events in real
time.

### Quarter 5–6 (optional / Phase 2)

**Headline**: The chain, mesh, dreams layers come online. The
replication ledger crosses deployments. Roko starts feeding back its
own meta-research.

Tracks:

- **Phase 2 Bus/Substrate backends** (09): ChainBus, NatsBus,
  MultiBus.
- **Dreams cycle** (09 §2): Delta-speed consolidation with
  HDC-cluster-driven promotion.
- **Chain witnesses** (32 §8): attestation to on-chain; replication
  ledger cross-deployment trust.
- **Composer rewrite** (21 §2.5): query-driven templates; HDC-picked
  prompt parts.
- **Plugin registry** (17 Stage E): published plugin catalog with
  signed manifests and replication-ledger reputation.
- **Prediction markets** (15 §5.1): intra-system stake tokens for
  heuristic outcome betting.

These are explicitly Q5–Q6 items; Q1–Q4 stand on their own.

## 4. Parallelism and team shape

Minimum team to land Q1–Q4 in 12 months:

- **Kernel engineer (1)**: owns Q1; continues as steward through Q4.
- **Learning engineer (1–2)**: owns Q2 tracks 10, 11, 12, 14, 16.
  Continues into Q3–Q4 for polish.
- **UX engineer (2)**: owns Q3 tracks 22, 23, 28, 29, 30. Q4 polish
  + mobile.
- **Platform / deployment engineer (1)**: owns 17, 24, 27, 32, 33.
  Continues through Q4.
- **Domain lead (1, rotating)**: owns Q4 domain profiles. Domain-
  expert partnerships helpful.
- **Research lead (0.5)**: owns 16 starter kit curation; active
  oversight in Q2.

Total: 5–7 engineers for 12 months to land Q1–Q4 comfortably. With
fewer, drop domain profiles to 3 (coding, research, ops) and extend
by a quarter.

## 5. Risk register

Top risks across the whole roadmap, with mitigations:

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Kernel refactor breaks subsystems | Medium | High | Feature-flag; test parity; phase-B bake period |
| Demurrage rates over/under-tuned | High | Medium | Dashboard tiles; auto-tuning policy; kill-switch |
| HDC encoder drift across deploys | Medium | Medium | Versioned encoder; refuse cross-version mixing |
| Plugin ABI churn | Medium | High | Frozen ABI at each release; semver-strict |
| Web UI scope creep | High | Medium | Strict 5-page cap; no custom design system |
| Multi-tenancy auth complexity | Medium | High | Namespace-only in Q1; OIDC in Q2 |
| c-factor gets reward-hacked | Medium | Medium | c as covariate, not objective (13 §13) |
| Commons pollution | Medium | High | Curation; reputation; signature-based trust |
| Cross-doc consistency drift | High | Low | Glossary 34 is source of truth; CI checks |
| User confusion from new vocabulary | Medium | Medium | "For first-time readers" blocks in each doc; 34 |

Review the register quarterly; delete resolved, add new.

## 6. Decision checkpoints

Some refinements are "go/no-go" decisions, not continuous work.
Schedule explicit checkpoints:

- **After Q1 week 4**: Does the kernel refactor feel safe to continue?
  If not, revert to incremental patching; rescope.
- **After Q2**: Is demurrage reinforcement producing observable
  compounding (via the KPIs in 15 §10)? If not, tune or disable.
- **After Q3**: Is plugin ecosystem actually attracting plugins? If
  <5 external plugins in 6 weeks, audit onboarding.
- **After Q4**: Is any domain profile's replication ledger producing
  surprising findings? If yes, publish; if no, extend observation.

Each checkpoint has an owner and a written go/no-go. "Momentum"
isn't a checkpoint; evidence is.

## 7. Mapping to the existing catalog

Cross-reference to the current `tmp/ux-followup/` gap catalog:

| ux-followup section | Refinement equivalent | When it closes |
|---|---|---|
| 02 high-impact-quick-wins | Mostly polish; parallelized across Q1–Q3 | Q3 |
| 07 spec-code-drift (P0s) | 05 loop rewrite, 07 naming, doc rewrites | Q1 Phase A |
| 12 tui-event-parity (P0s) | Subsystem migration in 06 Phase C | Q1 end |
| 15 safety-and-learning-closure (P0s) | PlanRevisionPolicy + PrdPublishPolicy | Q1 end |
| 04 t9-t19-residuals | Policy migration to Bus subscriptions | Q1 Phase C |
| 05 partially-wired | Graduate to full wiring during Phase C | Q1 |
| 13 session-state-mgmt | State export/import in 24 | Q3 |
| 14 observability-gaps | 33 observability doc | Q2–Q3 |

Several ux-followup items become trivial once the refinements land
(the P0s in 12 are "subscribe to Bus topic"). Others (09
hygiene-and-test-coverage) are parallel hygiene work that doesn't
block either track.

## 8. Mapping to MASTER-PLAN tiers

The MASTER-PLAN.md tiers vs. refinements:

| Tier | MASTER-PLAN scope | Refinement coverage |
|---|---|---|
| 1 Mori parity | ~129 items | Closes gradually across Q1–Q3 |
| 2 Agent platform | ~81 items | Q3 (plugins, realtime) + Q4 (domains) |
| 3 Templates & events | ~28 items | Q2 (heuristics) + Q3 (realtime) |
| 4 Daemon & multi-repo | ~40 items | Q4 (deployment) |
| 5 Cognitive layer | ~92 items | Q2 (self-learning), Q5–Q6 (dreams) |
| 6 Chain layer | ~68 items | Q5–Q6 (Phase 2) |

The refinements don't replace MASTER-PLAN; they give it a framing
where each remaining item has a clearer "home doc." MASTER-PLAN
items should reference refinement numbers in their subsequent
updates.

## 9. Not-doing list

Explicit "we considered and rejected for now" items. The list is as
important as the roadmap; it says what we deliberately defer.

- **Custom LLM training on accumulated episodes**. Worth discussing
  later; not in scope now. The data compounds; training on it is a
  separate effort.
- **Graphical plan editor** (beyond the DAG view in web UI). The
  web UI surfaces plans but editing happens in the underlying
  plans/ markdown. A drag-and-drop editor is a Q6+ item.
- **Multi-language SDK** (Python / TS / Go native clients beyond
  what `27 §8` specifies). First-party clients for realtime is
  enough; full SDKs wait for demand.
- **Self-hosted LLM runtime**. Ollama/LM Studio are supported as
  backends (24 §1.1) but Roko doesn't ship its own inference server.
- **Kubernetes operator**. Helm chart (24 §7.1) is sufficient; an
  operator is a Q6+ item if demand materializes.
- **Mobile native app**. Progressive web app (29 §8) is sufficient;
  native mobile is a Q6+ consideration.
- **Voice-only interface**. Voice as an assist in chat (23 §5, 30
  §5) is fine; a standalone voice-driven workflow is out of scope.

Each of these can move onto the roadmap via an explicit proposal.
Until then, they're not.

## 10. One-year demo sequence

If Q1–Q4 ship, the one-year demo is a single 20-minute walkthrough:

1. `roko init` — interactive; user picks a profile.
2. `roko ask "what does this codebase do?"` — researcher role; web
   search turned on; result cites heuristics.
3. User asks to fix a bug. Roko classifies as multi-step, proposes
   plan. User approves.
4. Plan runs. Web UI shows live c-factor as two agents collaborate.
   Tool-call banners + reasoning stream visible. Gate badges green.
5. Heuristic is applied with a footnote; user hovers and sees
   calibration = 0.82 from 41 trials.
6. Plan completes. Diff shown per hunk. User accepts. Undo
   available.
7. User runs `roko custody list --after-plan <id>` — full chain of
   custody for every action.
8. User runs `roko cost report` — $0.42 spent; breakdown by model.
9. User installs a tier-3 tool plugin. Reruns. The plugin is in the
   toolset.
10. User exports session; imports on another machine; resumes.

Every step is a concrete capability from 02–30. If the demo works,
the roadmap worked.

## 11. Twelve-year view (aspirational)

Where 12 years of steady progress on this roadmap could lead:

- A deployed Roko runtime is an agent-research laboratory that
  publishes replication findings automatically.
- The heuristic commons rivals academic textbooks for engineering
  knowledge; chain-witnessed high-calibration heuristics are cited
  in papers.
- A dozen domains have well-developed profiles; mainstream companies
  deploy Roko for their core knowledge work.
- The substrate architecture is studied in graduate compilers
  courses (two-medium kernel, HDC-native content addressing,
  demurrage memory management).
- Plugin ecosystem: 10,000+ plugins; second-order plugins
  (plugin-of-plugins); cross-plugin commons emerge.
- Phase 2 on-chain witnessing creates a decentralized truth substrate
  for empirical engineering knowledge.

None of this is guaranteed. All of it becomes possible when Q1–Q4
actually ships and compounds. The near-term roadmap earns the right
to the long-term vision.

## 12. Cross-references

Every refinement doc is cited. In order:

- Foundation: 01, 02, 03, 04, 05, 06, 07, 08, 09.
- Learning: 10, 11, 12, 13, 14, 15, 16.
- Moat & ecosystem: 17, 18, 19, 20, 21.
- UX: 22, 23, 24, 25.
- Kernel UX plumbing: 26, 27.
- Surface UX: 28, 29, 30.
- Integrators: 31, 32, 33, 34, and this doc (35).

Plus the living planning material this roadmap intersects:
`tmp/MASTER-PLAN.md`, `tmp/ux-followup/00-INDEX.md`,
`docs/00-architecture/23-architectural-analysis-improvements.md`.

## 13. Maintenance

This doc is the single source of truth for sequencing.

- Updated at the end of each quarter with actual-vs-planned status.
- Checkpoint decisions (§6) logged in a changelog block at the
  bottom.
- Risk register (§5) reviewed quarterly.
- Not-doing list (§9) updated when items promote or demote.
- The dependency graph (§2) updates when refinements add or remove
  dependencies on each other.

## Changelog

- **2026-04-16** — Initial roadmap authored alongside refinements.
  Covers Q1–Q4 in earnest; Q5–Q6 as Phase-2 markers.
