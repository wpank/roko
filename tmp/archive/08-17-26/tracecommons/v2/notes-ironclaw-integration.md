# IronClaw ↔ TraceCommons Integration: Current State

Notes on where the integration stands as of August 2026. Written for anyone
catching up — covers what shipped, how traces flow, open issues, and a few
ideas for what could come next.

---

## 1. What's Done

The integration is substantially wired and merged. Three core IronClaw PRs
landed roughly 20K lines across 160 files between June and July 2026, plus a
server-side PR on TraceCommons.

### IronClaw PRs

**PR #4559** (Jun 2026, merged)
Agent-driven onboarding via invite link. This is the foundational PR — it
established the full contribution pipeline:

- Capture → redact → queue → submit
- 6 model-visible capabilities under `builtin.trace_commons.*`
- Ed25519 device-key authentication
- Standing consent policy (opt-in/opt-out persisted per user)
- Deterministic redaction pipeline integrated into the turn runner hook

**PR #5280** (Jun 2026, merged)
Instance-wide enrollment, per-user profiles, trace inspection.

- Trace-credential resolver: routes between personal-invite and instance-wide
  enrollment flows depending on how the user onboarded
- Community profile management (display name, avatar, public stats)
- Credit tracking and display
- Trace preview before submission

**PR #5858** (Jul 2026, merged)
Instance enrollment CLI and hosted-user account login links.

- `traces enroll-instance` command for operators to enroll an entire
  deployment
- Login link generation for hosted users who don't have local device keys
- Streamlined the gap between self-hosted and managed IronClaw deployments

### TraceCommons Server PR

**TC server PR #152** (Jun 2026, merged)
Server-side changes to support multi-tenant device-key auth.

- Per-user subject field on device-key upload claims
- Namespaced `principal_ref` derivation — device keys are scoped to the
  originating IronClaw instance, preventing cross-instance key confusion
- Backwards-compatible with single-user invite-based onboarding

### IronClaw Crate Layout

The `ironclaw_trace_commons` crate exposes 6 public modules:

| Module | Purpose |
|---|---|
| `capture` | Turn-level trace extraction from the agent event stream |
| `client` | HTTP client for TC ingest and profile endpoints |
| `contribution` | Envelope construction, queuing, and submission |
| `conversation_message` | Message-type mapping between IronClaw's internal format and TC wire types |
| `onboarding` | Invite link handling, instance enrollment, consent persistence |
| `redaction` | Deterministic secret/PII removal before upload |

This crate is consumed by 5 other IronClaw workspace crates (turn runner,
CLI commands, settings UI, instance management, and the capability registry).

### CLI Commands

| Command | What it does |
|---|---|
| `traces opt-in` | Enable trace contribution for the current user |
| `traces opt-out` | Disable trace contribution |
| `traces enroll-instance` | Enroll an IronClaw instance for instance-wide contribution |
| `traces preview` | Show what a trace submission would contain before sending |
| `traces submit` | Manually submit a trace (normally automatic via hook) |
| `traces contributor` | Show contributor profile and credit balance |

### WebUI

Settings tab includes TC enrollment toggle and an "Open Trace Commons
account" button that generates a login link. Internationalized for 11
languages.

---

## 2. How Traces Flow

```
Agent Turn
  │
  ▼
ironclaw_turn_runner (post-turn hook)
  │
  ▼
ironclaw_trace_commons::capture
  │  Extract tool calls, LLM messages, timing, token counts
  │
  ▼
Standing policy check
  │  Is the user opted in? Is the instance enrolled?
  │  If no → drop silently, no error
  │
  ▼
Envelope construction
  │  Map IronClaw message types → TC wire format
  │  Attach session metadata, agent config hash, provider tag
  │
  ▼
Deterministic redaction
  │  Strip secrets, PII, file paths, API keys
  │  Compute redaction_hash over redacted content
  │
  ▼
Trace-credential resolver
  │  Route: personal-invite path OR instance-wide path
  │  Resolve the correct signing key and principal_ref
  │
  ▼
Upload claim minting
  │  EdDSA JWT signed with device key
  │  Claims include: principal_ref, envelope hash, timestamp
  │
  ▼
TC Ingest Server
  │  Validate JWT, verify device key, dedup check
  │
  ▼
Quality gate (enclave)
  │  Chunking → embedding → perplexity scoring → vector dedup
  │  Score attestation (EdDSA JWT from enclave)
  │
  ▼
Credit + NEAR settlement
   Contributor credits allocated based on quality score
   Settlement batched to NEAR (when above dust threshold)
```

Key design decisions in the flow:

- **Opt-in only.** The standing policy check is the first gate. No traces
  leave the machine without explicit consent.
- **Redaction before network.** The redaction step runs locally, before any
  data hits the wire. The `redaction_hash` provides an audit trail.
- **Credential flexibility.** The resolver handles both individual users
  (personal invite link) and managed deployments (instance-wide enrollment)
  without the user needing to know which path they're on.

---

## 3. Known Open Issues

### IronClaw Side

**#7144 — Consolidation review defects (29 items)**
A sweep of the merged code found 29 pre-existing defects. Three are serious:

1. **Credential HTTP client builders never check URL scheme.** A bearer token
   could be sent over plaintext HTTP if the configured endpoint URL lacks
   `https://`. No TLS enforcement at the client level.
2. **Privacy gate keyed on prose substring.** The quarantine check looks for
   the string "quarantined" in a status field rather than matching a typed
   enum variant. Fragile if the server changes wording.
3. **`redaction_hash` hashes empty bytes on serialization failure.** If the
   envelope fails to serialize before hashing, the code silently hashes an
   empty byte slice instead of propagating the error. The resulting hash is
   valid but meaningless — it doesn't correspond to the actual content.

**#4940 — No behavioral tests for `ContributionHttpSink`**
The egress path (the part that actually sends envelopes to TC) has no
integration or behavioral test coverage. Unit tests mock the HTTP layer
entirely. A real request has never been validated in CI.

**#6714 (PR, open) — Test isolation**
TC parity tests read real `$HOME` state (config files, device keys). This
causes flaky failures on shared CI runners and makes local test runs
non-hermetic.

### TraceCommons Server Side

**#131 — `message_text_included` envelopes quarantined by default**
Envelopes that include raw message text (as opposed to embeddings-only) are
auto-quarantined. There is no policy mechanism to auto-accept them, even when
the contributor has explicitly opted in to full-text contribution. Every
text-included trace requires manual operator review.

**#219 — Redaction penalizes trace scores**
The quality gate treats successful redaction as evidence against a trace. If
the redaction pipeline removed content, the resulting trace scores lower than
an un-redacted one. This creates a perverse incentive: contributors who
properly redact sensitive data get worse credit outcomes than those who
contribute raw text.

**#137, #136, #140, #141**
Wire type updates, device-key auth branch work, and a pilot deployment
runbook. Tracked but not formally closed. These appear to be cleanup items
from the PR #152 review cycle.

---

## 4. Ecosystem Context

**IronClaw** (NEAR AI)
- 12,597 stars, 1,485 forks (as of writing)
- 62 workspace crates
- 26 LLM provider integrations
- Runs across CLI, Telegram, Slack, Discord, Signal
- WASM-sandboxed tool execution with fuel metering
- TEE deployment support

**TraceCommons**
- 6 stars, 2 forks
- 6 crates (protocol, contributor, gate-api, gate-enclave, operator-client, server)
- Pilot stage — small number of active contributors
- Quality gate runs in SGX enclave for score attestation

**Shared infrastructure:**
Both projects are in the NEAR ecosystem. Identity model is NEAR accounts.
Credit settlement uses NEAR for on-chain finality. The `principal_ref` in
TC maps to a NEAR account ID (or a derived namespace under one).

---

## 5. Opportunities Worth Exploring

These are ideas, not proposals. Listed because the integration surface makes
them possible, not because anyone has committed to building them.

### WASM fuel as quality signal

IronClaw's WASM sandboxing provides fuel consumption per tool invocation.
This is a direct, manipulation-resistant measure of computational work. Fuel
data could feed into TC's quality scoring — an agent that accomplishes a task
in fewer fuel units is arguably producing a higher-quality trace. The data is
already captured; it would need to be included in the envelope and respected
by the gate.

### Cross-provider comparison

IronClaw supports 26 LLM providers. With enough trace volume, the corpus
could reveal which providers produce better agent outcomes for specific task
types. This is valuable data that no single provider can produce on their
own — it requires a neutral collection point with standardized quality
scoring. TC is positioned to be that point.

### Channel-specific interaction patterns

IronClaw runs the same agent across CLI, Telegram, Slack, Discord, and
Signal. Same agent logic, different interaction modalities. Do users interact
differently? Do agents perform differently? Trace data tagged with channel
metadata could answer these questions. The channel tag is available in
IronClaw's session metadata but not currently included in the TC envelope.

### TEE attestation chaining

IronClaw can prove an agent ran inside a TEE. TC's quality gate already runs
in an SGX enclave and produces EdDSA attestations. Chaining these together —
agent execution attestation → quality scoring attestation → credit settlement
— would give end-to-end cryptographic proof of the entire pipeline. This is
the highest trust tier TC could offer and it would be unique among trace
collection systems.

### `TraceSource` for IronClaw sessions

TC's contributor CLI defines a `TraceSource` trait with three existing
implementations: ClaudeCode, Codex, and Trajectory (generic trajectory
files). IronClaw could be the fourth implementation, enabling TC's standalone
CLI to discover and upload IronClaw sessions directly — without requiring
the IronClaw-side integration at all. This would be a lighter-weight
alternative for users who want to contribute traces but don't want to enable
the built-in hook.

---

## References

### IronClaw PRs
- [PR #4559](https://github.com/nearai/ironclaw/pull/4559) — Agent-driven onboarding, contribution pipeline, device-key auth
- [PR #5280](https://github.com/nearai/ironclaw/pull/5280) — Instance enrollment, profiles, trace inspection
- [PR #5858](https://github.com/nearai/ironclaw/pull/5858) — Instance enrollment CLI, hosted-user login links

### IronClaw Issues
- [#7144](https://github.com/nearai/ironclaw/issues/7144) — Consolidation review (29 defects)
- [#4940](https://github.com/nearai/ironclaw/issues/4940) — Missing behavioral tests for egress path
- [#6714](https://github.com/nearai/ironclaw/pull/6714) — Test isolation fix (open PR)

### TraceCommons PRs and Issues
- [PR #152](https://github.com/tracecommons/trace-commons-server/pull/152) — Per-user device-key subjects, namespaced principal_ref
- [#131](https://github.com/tracecommons/trace-commons-server/issues/131) — message_text_included quarantine policy
- [#219](https://github.com/tracecommons/trace-commons-server/issues/219) — Redaction penalizing scores
- [#136](https://github.com/tracecommons/trace-commons-server/issues/136), [#137](https://github.com/tracecommons/trace-commons-server/issues/137), [#140](https://github.com/tracecommons/trace-commons-server/issues/140), [#141](https://github.com/tracecommons/trace-commons-server/issues/141) — Wire types, auth branch, pilot runbook
