# IronClaw Integration: Status & Opportunities

> **Date**: 2026-08-10

TraceCommons (TC) is an open-source AI trace registry. IronClaw (NEAR AI) is its primary integration partner -- 12.6K GitHub stars, 26 LLM providers, WASM sandboxing. The integration is substantially done: 3 merged PRs on IronClaw (~20K lines across 160 files), 1 merged PR on TC server, and 6 modules in the `ironclaw_trace_commons` crate (capture, client, contribution, conversation_message, onboarding, redaction) consumed by 5 other IronClaw workspace crates.

The full pipeline works end-to-end:

```
agent turn -> capture -> policy check -> envelope -> redact ->
credential resolve -> JWT -> ingest -> gate -> credit
```

---

## 1. Shipped PRs

| PR | Summary | When |
|----|---------|------|
| IronClaw [#4559](https://github.com/nearai/ironclaw/pull/4559) | Agent-driven onboarding, 6 `builtin.trace_commons.*` capabilities, Ed25519 auth, standing consent, deterministic redaction | Jun 2026 |
| IronClaw [#5280](https://github.com/nearai/ironclaw/pull/5280) | Instance enrollment, community profiles, trace inspection, credit tracking | Jun 2026 |
| IronClaw [#5858](https://github.com/nearai/ironclaw/pull/5858) | Instance enrollment CLI (`traces enroll-instance`), hosted-user login links | Jul 2026 |
| TC server [#152](https://github.com/tracecommons/trace-commons-server/pull/152) | Per-user device-key subjects, namespaced `principal_ref` | Jun 2026 |

---

## 2. Open Issues

Five tracked issues across both repos. Two are correctness problems in the trust path.

| Issue | Severity | Summary |
|-------|----------|---------|
| IronClaw [#7144](https://github.com/nearai/ironclaw/issues/7144) | 3 serious / 26 minor | Consolidation review: TLS enforcement, quarantine check, redaction_hash |
| IronClaw [#4940](https://github.com/nearai/ironclaw/issues/4940) | Medium | No behavioral tests for `ContributionHttpSink` egress path |
| IronClaw [#6714](https://github.com/nearai/ironclaw/pull/6714) | Low (PR open) | Test isolation: reads real `$HOME`, flaky on shared CI |
| TC [#131](https://github.com/tracecommons/trace-commons-server/issues/131) | Medium | `message_text_included` envelopes auto-quarantined, no policy override |
| TC [#219](https://github.com/tracecommons/trace-commons-server/issues/219) | High | Redaction penalizes quality scores (perverse incentive) |

---

## 3. Critical Fixes

The three serious defects from #7144 and the test gap from #4940 are the most important open items. They sit in the trust path -- the part of the system that determines whether traces are authentic, properly redacted, and securely transmitted. Fixing these should come before any new feature work on the integration.

### 3.1 No TLS enforcement on credential HTTP clients

The HTTP client builders in `ironclaw_trace_commons::client` do not validate that the configured endpoint uses HTTPS. If an operator sets `trace_commons_url = "http://..."`, bearer tokens and signed JWTs are sent over plaintext. In a coffee-shop or corporate-proxy scenario, the device key's JWT is exposed in transit; an attacker with the JWT can replay uploads or correlate contributor identity.

The device-key authentication model assumes the JWT never leaves a TLS tunnel. If it does, the authentication guarantee breaks. TC's ingest server cannot distinguish a legitimate upload from a replayed one if the JWT was captured in transit.

**Fix:** Add a scheme check at client construction time. If the URL scheme is not `https`, either reject it outright or accept it only when an explicit `allow_insecure = true` flag is set (useful for local development against `localhost`). This is a ~10-line change in the client builder, but it needs to be done carefully to avoid breaking existing local-dev workflows. The check should happen at construction, not at request time, so misconfigurations fail fast.

### 3.2 Quarantine check keyed on prose substring

The privacy gate checks quarantine status by looking for the substring `"quarantined"` in a server-returned status string. This is a stringly-typed check against what should be a typed enum. If TC changes the wording (e.g., `"quarantined_for_review"` to `"held_for_review"`), the check silently stops matching and quarantined traces are treated as accepted.

This is a privacy boundary. The quarantine status exists to prevent traces with flagged content from being treated as clean. A missed match means a trace that TC flagged for review gets treated as successfully submitted on the IronClaw side, which could mislead the contributor about what data left their machine.

**Fix:** Define a `TraceStatus` enum on the wire protocol, or at minimum match against an exhaustive set of known status strings and treat unknown values as quarantined-by-default. The defensive default matters -- if you don't recognize the status, assume the worst.

### 3.3 Empty-bytes `redaction_hash` on serialization failure

The redaction pipeline computes a `redaction_hash` over the serialized envelope after redaction. If serialization fails, the error is swallowed and the hash is computed over an empty byte slice. The resulting hash (`sha256(b"")` = `e3b0c44298fc1c149afbf4c8996fb924...`) is valid but corresponds to nothing.

The `redaction_hash` is TC's audit trail for what was redacted. If TC later needs to verify that a contributor's local redaction matched the received content, the empty-bytes hash will always fail verification. Worse, every serialization failure produces the same hash, so TC cannot distinguish "serialization failed for trace A" from "serialization failed for trace B" -- they all look identical.

**Fix:** Propagate the serialization error. If the envelope cannot be serialized, the trace submission should fail rather than proceeding with a meaningless hash. This is a correctness-over-availability tradeoff: better to reject one trace than to submit it with a hash that undermines the entire audit mechanism.

### 3.4 No behavioral tests for `ContributionHttpSink` (#4940)

The `ContributionHttpSink` is the component that actually sends envelopes over the wire to TC's ingest endpoint. It has unit tests, but they mock the HTTP layer entirely. No test in IronClaw's CI has ever made a real HTTP request to a TC-compatible endpoint. The egress path is where serialization format, header construction, authentication headers, content-type negotiation, and error handling all converge. Any mismatch between the mocked behavior and real server behavior will only be caught in production. Given that the three defects above were also in this path, the lack of integration tests is a compounding factor.

**Fix:** A `wiremock`-based test that stands up a fake ingest server, exercises the full `ContributionHttpSink::submit()` path including JWT construction and envelope serialization, and asserts on the received request structure. The `trace-commons-protocol` crate's wire types can be used directly for deserialization assertions. This does not need to validate scoring -- it just needs to prove that what leaves the client is what the server expects to receive.

The test isolation issue (#6714) is related -- fixing it makes the behavioral tests reliable by ensuring they do not read real `$HOME` state.

---

## 4. Next Opportunities

Five concrete ideas enabled by the existing integration surface. Each one builds on code that already exists in one or both projects. Ordered roughly by effort and impact.

### 4.1 OTel-native trace emission

IronClaw's `capture` module already extracts the fields that map onto OTel GenAI semantic conventions (`gen_ai.system`, `gen_ai.request.model`, `gen_ai.usage.input_tokens`, etc.) -- it just serializes them into TC's custom envelope. Emitting traces as OTLP spans would let TC accept contributions from any OTel-instrumented agent framework, not just IronClaw. IronClaw becomes the reference implementation, but the protocol is standard. Contributors who already run OTel collectors for observability could route a copy of their agent spans to TC with a config change, not a code change.

**Effort:** Medium. The OTel GenAI semantic conventions are stabilizing. The mapping from IronClaw's internal event types to OTel spans is straightforward. The harder part is on TC's side: adding an OTLP ingest endpoint alongside the existing envelope-based one, and mapping OTel span attributes back to the internal quality gate inputs. The custom envelope carries TC-specific fields (consent metadata, redaction_hash, device-key claims) without OTel equivalents, so coexistence (OTel for trace data, custom wrapper for TC metadata) is the likely path.

### 4.2 WASM fuel as a quality signal

IronClaw sandboxes tool execution in WASM with fuel metering. Fuel consumption is a direct, manipulation-resistant measure of computational work -- unlike token counts (volume) or wall-clock time (latency). An agent that accomplishes a task with fewer fuel units per tool call is producing a more efficient trace, and you cannot inflate fuel consumption without actually running more WASM instructions.

Including per-tool-call fuel in the TC envelope adds a process efficiency dimension to scoring: not just "was the output good?" but "was the process efficient?" This is the kind of signal that gets more valuable as the corpus grows, because it enables cross-trace comparisons of how different agents (or different configurations) approach the same kind of task.

**Effort:** Low-medium. What needs to happen: (1) include per-tool-call fuel consumption in the TC envelope (new field on the tool invocation event type), (2) surface it in the gate as an optional scoring input, (3) decide whether fuel efficiency contributes positively to quality scores or is tracked as a separate metric. The conservative choice is to track it separately at first and only incorporate it into scoring once there is enough data to validate the correlation.

### 4.3 Cross-provider comparison analytics

IronClaw supports 26 providers; TC has standardized quality scoring. The combination produces a unique dataset: the same quality metrics applied across dozens of providers on real-world agent tasks, not vendor-chosen benchmarks. No single LLM provider can produce this data -- it is closer to Consumer Reports than a vendor benchmark.

Provider tag is already in the envelope metadata (from IronClaw's session context). TC already stores it. The missing piece is analytics queries that aggregate quality scores by provider, broken down by task type, model, and time period. Examples:

- "Traces using Provider X for code generation scored 12% higher than Provider Y in July 2026"
- "Provider Z had the most consistent scores (lowest variance) across all task types"
- "For multi-turn tool-use tasks, Provider W outperformed all others by a significant margin"

**Effort:** Low (analytics queries over existing data).

The sensitivity question: publishing provider-level quality rankings is politically charged. Providers will not like it if they rank poorly. But the data is enormously valuable for the developer community, and it is the kind of neutral, empirical comparison that does not exist today. TC could start by making this data available only to contributors (as a private insight -- related to PR #241) rather than publishing it publicly.

### 4.4 TEE attestation chaining

IronClaw supports TEE deployment; TC's quality gate runs in an SGX enclave with EdDSA attestations. Chaining these creates end-to-end cryptographic proof:

```
IronClaw TEE attestation (agent execution was genuine)
         |
         v
TC enclave attestation (quality scoring was unbiased)
         |
         v
NEAR settlement (credit allocation is final)
```

The full chain proves: (1) the agent actually ran the code it claims to have run, (2) the quality score was computed by untampered gate logic, and (3) the credit allocation was recorded on-chain. No participant in the pipeline could have manipulated the outcome without breaking the attestation chain.

Enterprise and regulated environments need this. If you are a financial services firm running AI agents, you need audit trails. If you are contributing traces from those agents, you need proof that the scoring pipeline did not discriminate against your submissions. TEE attestation chaining provides that proof.

**Effort:** High. The individual pieces exist (IronClaw TEE, TC enclave, NEAR settlement), but chaining them requires a verification protocol: each stage needs to include the previous stage's attestation in its own input, and the final verifier needs to walk the chain. This is a non-trivial protocol design exercise, but the cryptographic primitives are standard (EdDSA signatures, attestation reports).

The practical path is two-link chains (IronClaw TEE -> TC enclave) first, since NEAR settlement is already trustworthy by virtue of being on-chain.

### 4.5 Channel-specific interaction patterns

IronClaw runs the same agent logic across CLI, Telegram, Slack, Discord, and Signal. Same model, same tools, different interaction modality. Traces tagged with channel metadata would reveal whether interaction patterns differ by channel -- and whether those differences affect trace quality.

Hypotheses worth testing: Do CLI users produce longer, more structured traces? Do Telegram users produce more conversational, multi-turn traces? Does the same agent configuration perform differently when the human interaction style changes? These are empirical questions that the IronClaw + TC combination is uniquely positioned to answer.

**Effort:** Low. The channel tag is available in IronClaw's session metadata but is not currently included in the TC envelope. Adding it is a small change (one field on the envelope, one field threaded through capture). The analytics value comes later, once there is enough volume across channels to make comparisons meaningful. Channel metadata is low-sensitivity ("this trace came from Telegram," not "this user's Telegram handle") but should be in the redaction pipeline's scope so contributors can opt out of channel tagging.

---

## 5. User Acquisition: Making the Integration Useful for Growth

IronClaw is TC's primary distribution channel -- 12.6K stars, active community, agents running in production across five platforms. The integration code is merged and working. The question is how to make contribution visible and rewarding enough that IronClaw users actually opt in.

### 5.1 The current contributor experience is invisible

Today, an IronClaw user who opts in to TC contribution gets nothing visible. Their traces flow silently through the pipeline. Credits accumulate somewhere. There is no feedback loop.

The user has to run `traces contributor` to see their credit balance, and even then the number is abstract -- there is no context for whether 47 credits is a lot or a little, no comparison to other contributors, no trend over time.

This is the single biggest gap in the integration. The pipeline works technically, but it provides no psychological reward for contributing. The items below address this gap at different points in the user experience.

### 5.2 Immediate scoring feedback

When a trace is scored by TC's quality gate, the score should flow back to IronClaw and be surfaced to the contributor. Not in a separate dashboard they have to navigate to, but inline -- in the same place where the agent interaction happened.

Concrete example: after an agent session completes and the trace is submitted, the user sees a brief notification:

```
Trace contributed to TraceCommons
Quality score: 92/100 (top 15% this week)
Credits earned: 3.2 TC
```

This requires three changes:

1. TC's ingest endpoint returning the quality score in the submission response (it currently returns a 202 with minimal body)
2. IronClaw's `ContributionHttpSink` parsing and surfacing that score
3. A display path in IronClaw's CLI and WebUI output

The "top 15% this week" context is important -- it turns an abstract number into a relative position, which is more motivating than a raw score. TC already has the data to compute percentiles; it just needs to include them in the response.

### 5.3 Contribution stats in IronClaw dashboards

IronClaw already has usage dashboards (CLI stats, agent performance, token consumption). TC contribution stats should live alongside these existing metrics, not in a separate TC dashboard that requires a different login.

What to surface:

- Total traces contributed (lifetime)
- Average quality score (rolling 30-day)
- Credits earned (with NEAR equivalent)
- Contribution streak (consecutive days with at least one trace)
- Provider comparison (if the user uses multiple providers: "Your Claude traces score 8% higher on average than your GPT traces")

The provider comparison is particularly useful because it gives the contributor actionable information. It is the kind of insight that makes contribution feel personally valuable, not just altruistic.

### 5.4 Opt-in contributor leaderboard

For users who want public recognition: an opt-in leaderboard showing top contributors by volume, quality score, or streak. This is a well-understood gamification pattern. The key design decision is making it opt-in by default (privacy-first) and allowing pseudonymous participation (display name, not real identity).

IronClaw's community profile system (PR #5280) already supports display names and public stats -- the leaderboard is a natural extension of that infrastructure.

### 5.5 Founding contributor status

TC is in pilot stage. There will never be an easier time to be a top-1% contributor than right now when the corpus is small. Creating a "Founding Contributor" designation for users who contribute during the pilot phase creates urgency and exclusivity.

This status should be permanent -- once the corpus grows to 100K traces, founding contributors who were there at 1K traces should still be recognized.

Surface via IronClaw's `traces contributor` command (badge display) and TC's profile API (for integration partners to display however they choose).

### 5.6 Onboarding-time opt-in

The highest-leverage moment for contributor acquisition is during IronClaw's agent setup flow. When a user configures their first agent, a single screen explaining TC contribution (what it is, what's shared, what's redacted) with one-tap opt-in captures contributors at the moment of highest engagement.

The standing consent model (PR #4559) already supports this -- it just needs to be surfaced earlier in the user journey, not buried in settings.

This is the lowest-effort, highest-impact change on this list. Everything else requires new infrastructure or API changes. This one is a UX reshuffle of capabilities that already exist.

---

## 6. Third Contributor Signal

[PR #250](https://github.com/tracecommons/trace-commons-server/pull/250) from brapse is TC's first contribution outside the core team. TC has had two active contributors (core team). A third person showing up -- reading the codebase, understanding the contribution model, and submitting a PR -- is meaningful signal for a 6-star project.

What it indicates:

- The codebase is readable enough for an outsider to contribute
- The contribution model is interesting enough to attract attention beyond the core team
- The project is past the "founders only" phase

What it does not tell you: whether brapse found TC through IronClaw, through NEAR, through academic interest, or through something else entirely. The acquisition channel matters for understanding how to get contributors 4 through 100. If brapse came through the IronClaw integration, that is direct evidence the distribution channel works. If they came through some other path, that is evidence of organic interest independent of the integration -- arguably more valuable because it is harder to manufacture.

Either way, the right response is to make brapse's contribution experience excellent -- fast review, clear feedback, recognition. The third contributor's experience sets the template for every contributor after them. If PR #250 sits in review for two weeks with no response, that is a signal too, and not a good one.

---

## 7. Open Questions

A few things that don't have clear answers yet but are worth tracking:

1. **OTel vs. custom envelope: coexistence or migration?** If TC adds OTLP ingest (section 4.1), does the custom envelope format stay as a parallel path or eventually deprecate? The custom format carries TC-specific fields (consent metadata, redaction_hash, device-key claims) that don't have OTel equivalents. Likely answer: OTel for the trace data, custom wrapper for TC-specific metadata, but the details matter and will determine how much of the existing IronClaw integration code needs to change.

2. **A2A delegation traces.** IronClaw supports multi-agent workflows where one agent delegates to another. These delegation events are a new data type for TC -- they are not single-agent traces, they are inter-agent coordination records. How should TC model them? As linked traces? As a new event type? As a parent-child span relationship (which maps naturally to OTel)? This connects directly to the OTel question above.

3. **TC #219 interaction with IronClaw.** IronClaw's redaction pipeline is thorough -- it strips secrets, PII, file paths, API keys. If TC's gate penalizes redacted traces, IronClaw contributors are systematically disadvantaged compared to contributors from less privacy-conscious integrations. This is the most urgent TC-side fix for the integration because it directly undermines the incentive to contribute through a well-engineered pipeline.

4. **VET composed proofs.** The Verifiable Execution Transcript concept could chain an IronClaw execution proof with a TC scoring proof into a single verifiable artifact. This is the formal version of the TEE attestation chaining idea in section 4.4, but using general-purpose proof composition rather than TEE-specific attestation. Worth tracking as the VET specification matures.

5. **Credit settlement economics.** NEAR settlement is batched above a dust threshold. For a pilot-stage project with low volume, most contributors may never hit the threshold. Is there a way to make credits feel real before settlement? Pre-settlement balance display, in-app credit utility, or lowering the threshold during the pilot phase are all options worth exploring.

---

## References

- IronClaw PRs: [#4559](https://github.com/nearai/ironclaw/pull/4559), [#5280](https://github.com/nearai/ironclaw/pull/5280), [#5858](https://github.com/nearai/ironclaw/pull/5858)
- IronClaw issues: [#7144](https://github.com/nearai/ironclaw/issues/7144), [#4940](https://github.com/nearai/ironclaw/issues/4940), [#6714](https://github.com/nearai/ironclaw/pull/6714)
- TC PRs/issues: [#152](https://github.com/tracecommons/trace-commons-server/pull/152), [#250](https://github.com/tracecommons/trace-commons-server/pull/250), [#131](https://github.com/tracecommons/trace-commons-server/issues/131), [#219](https://github.com/tracecommons/trace-commons-server/issues/219)
- v1: [IronClaw Integration](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a) (1,605 lines, design doc)
- v2: [Integration Notes](https://gist.github.com/wpank/c1a372b003538f31a77d8471db301a3d) (293 lines, status report)
