# F — Cognitive Kernel, Forensics, Integration Gap (Docs 14, 15, 16)

Parity of the three closure chapters: cognitive kernel safety
(Namespaces with ACL, Cognitive Signals as typed interrupts,
Cognitive Scheduling, Engram Syscalls), forensic AI regulatory
pre-compliance (EU AI Act, SEC/CFTC, HIPAA, SOX, GDPR), and the
critical integration gap (ToolDispatcher wiring).

**The integration gap doc (Doc 16) has actually been resolved for
most HTTP provider paths** — its own top paragraph now reads
"SafetyLayer is fully built and wired to the ToolDispatcher for the
routed provider-backed paths". The residual gap is **subprocess
paths** (Claude CLI) which still bypass the dispatcher.

Generated: 2026-04-16.

---

## F.01 — Namespaces with ACL (Doc 14 §"Namespaces with ACL")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 14 describes Cognitive Kernel Primitives: namespaces providing ACL-gated access to Engrams / signals / capabilities.
**Reality**: `Grep 'CognitiveNamespace\|namespace_acl\|KernelNamespace' crates/ --include=*.rs` returns zero matches. Frontier. The closest shipping analogue is `AgentWarrant` at `roko-agent/src/safety/capabilities.rs` (A.05) which gates tool execution — but that is tool-level, not namespace-level.

---

## F.02 — Cognitive Signals (typed interrupts) (Doc 14 §"Cognitive Signals")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 14 describes typed interrupts (algedonic signals) that bypass normal flow for safety-critical events.
**Reality**: The shipping `SignalEmit` capability kind at `roko-orchestrator/src/safety/capability_tokens.rs:79-80` (A.04) gates signal emission. Conductor emits `Custom("conductor.decision")` + `Custom("conductor:alert:<watcher>")` signals (batch 07 A.11). This is signal emission + watcher-driven signaling, but not the full typed-interrupt "algedonic channel" Doc 14 describes. Partial via shipping primitives, frontier for the full kernel-level redesign.

---

## F.03 — Cognitive Scheduling with priority + deadline + cooperative yield (Doc 14 §"Cognitive Scheduling")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Priority + deadline + cooperative-yield scheduler for cognitive tasks.
**Reality**: The shipping orchestrator uses the plan DAG + priority ordering via `crates/roko-orchestrator/src/executor/` but is not the cognitive-kernel-style priority + deadline scheduler Doc 14 describes. Frontier.

---

## F.04 — Engram Syscalls (universal enforcement) (Doc 14 §"Engram Syscalls")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: All Engram operations go through "syscalls" that enforce capability + ACL + audit + taint checks uniformly.
**Reality**: `Grep 'EngramSyscall\|engram_syscall' crates/ --include=*.rs` returns zero matches. The shipping `ToolDispatcher` 7-stage pipeline (Doc 16) covers the tool-call axis uniformly; the "every Engram mutation" axis is implicitly guarded by ContentHash immutability + append-only AuditChain, but there is no explicit `EngramSyscall` abstraction.

---

## F.05 — Content-addressed causal replay (Doc 15 §"Content-Addressed Causal Replay")

**Status**: DONE (via Engram lineage + `roko replay`)
**Severity**: —
**Doc claim**: Any Engram can be replayed with full causal history.
**Reality**: CLAUDE.md CLI commands table: `roko replay | Walk signal DAG by hash`. Shipping. Combined with content-addressing + Engram lineage, causal replay is a natural byproduct per Doc 02 / Doc 15.

---

## F.06 — Forensic AI regulatory pre-compliance (Doc 15 §"EU AI Act", §"SEC/CFTC", §"HIPAA", §"SOX", §"GDPR")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 positions Roko's forensic architecture as regulatory pre-compliant (EU AI Act, SEC/CFTC reporting, HIPAA audit, SOX controls, GDPR).
**Reality**: No compliance-specific types or reports ship. `Grep 'EU AI Act\|HIPAA\|SOX\|GDPR\|SEC/CFTC' crates/ --include=*.rs` returns zero matches. Doc 15 is a **positioning document** — the technical foundation (content-addressing + audit chain + Engram lineage) IS there (F.05), but compliance-specific packaging (certified report generators, regulator-facing export formats) is frontier.
**Fix sketch**: Doc 15 should mark itself `Implementation: Positioning — technical foundation ships (see F.05); compliance-specific exports frontier`.

---

## F.07 — Pre-certified agent templates (Doc 15 §"Pre-Certified Agent Templates")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 offers "pre-certified agent templates" for specific regulatory regimes.
**Reality**: No template library. Frontier.

---

## F.08 — SafetyLayer → ToolDispatcher wiring (Doc 16 §"The Gap in Detail")

**Status**: PARTIAL (Doc 16 itself acknowledges partial closure)
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"Overview" (updated version) now reads: "the SafetyLayer is fully built and wired to the ToolDispatcher for the routed provider-backed paths... Routed HTTP provider paths now reach the ToolDispatcher for OpenAI-compatible providers, Anthropic API, Gemini compat models, Gemini-native non-grounding tool-capable models, and Perplexity tool-capable chat. Known-protocol subprocess paths and some native or specialty endpoints still bypass it."
**Reality**: This is a substantially more nuanced status than Doc 16's headline "critical integration gap" suggests. The gap is narrower than the doc's title implies:
- **Shipping wired**: OpenAI-compat, Anthropic API, Gemini compat, Gemini-native tool-capable, Perplexity tool-capable — 5 HTTP provider paths.
- **Still bypasses**: Subprocess-based providers (Claude CLI), possibly Codex / Cursor CLI backends, native-tool specialty endpoints.

The `with_safety(SafetyLayer)` connector exists on `ToolDispatcher` at `roko-agent/src/dispatcher/mod.rs`. The 7-stage dispatch pipeline runs as documented. The question is which execution path in `orchestrate.rs` reaches the `ToolDispatcher` vs spawns a subprocess that manages its own tools.
**Fix sketch**: Rename Doc 16 to "SafetyLayer Coverage Status" (or similar) to reflect the partial-closure reality. The headline "Critical Integration Gap" overstates the current status. Add a coverage matrix: provider path × `ToolDispatcher` reached Y/N.

---

## F.09 — The 6-guard SafetyLayer composite (Doc 16 §"What Is Built")

**Status**: DONE (cross-ref A.01)
**Severity**: —
**Doc claim**: 6-guard SafetyLayer with BashPolicy / GitPolicy / NetworkPolicy / PathPolicy / ScrubPolicy / RateLimiter. 50+ tests.
**Reality**: See A.01 — matches exactly.

---

## F.10 — ToolDispatcher 7-stage pipeline (Doc 16 §"The dispatch() method")

**Status**: DONE
**Severity**: —
**Doc claim**: `dispatch()` runs 7 stages: validate → tool_filter → permission → safety pre-exec → handler → truncate → safety scrub. Each stage emits an audit Engram via `emit_audit()`.
**Reality**: Confirmed in Doc 16 §"ToolDispatcher". 7-stage pipeline shipping at `crates/roko-agent/src/dispatcher/mod.rs` (~1,070 LOC per Doc 16).

---

## F.11 — 4-phase resolution path (Doc 16 §"Resolution Path")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 16 tables a 4-phase resolution path for the integration gap.
**Reality**: Phase 1 (connect ToolDispatcher from routed HTTP providers) is DONE for 5 of ~8 provider paths (F.08). Phase 2-4 (subprocess paths + specialty endpoints + comprehensive coverage) remain open.
**Fix sketch**: Update Doc 16 §"Resolution Path" with per-phase status: Phase 1 (5 of N providers wired), Phase 2 (subprocess paths frontier), etc.

---

## F.12 — Architecture mismatch analysis (Doc 16 §"Architecture Mismatch Analysis")

**Status**: DONE (partially)
**Severity**: LOW
**Doc claim**: Doc 16 discusses architectural tensions between subprocess-based (Claude CLI) and HTTP-based (OpenAI / Anthropic API / Gemini) providers.
**Reality**: The 5 shipping integrations (F.08) resolve the HTTP side. Subprocess providers have different tool-dispatch semantics (the subprocess owns its own tool loop), so the architecture mismatch is inherent — the question is whether a subprocess-side adapter shim ships. Frontier.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 3 (F.05 content-addressed causal replay, F.09 6-guard SafetyLayer, F.10 7-stage dispatcher pipeline) |
| PARTIAL | 3 (F.02 cognitive signals via SignalEmit, F.08 SafetyLayer integration with 5-of-N providers wired, F.11 resolution path phase 1) |
| NOT DONE | 6 (F.01 namespaces, F.03 cognitive scheduling, F.04 Engram syscalls, F.06 regulatory pre-compliance, F.07 pre-certified templates, F.12 arch mismatch) |

Section F has the **doc whose framing has shifted fastest** (Doc 16).
Its own headline still reads "Critical Integration Gap" but the body
now acknowledges substantial closure for HTTP providers. The residual
gap is subprocess paths, which is an architecture question more than a
wiring question.

## Agent Execution Notes

### F.08 / F.11 — Rename or reframe Doc 16

Doc 16 should be renamed or rewritten so the headline reflects
partial closure. A provider × ToolDispatcher-reached matrix would
make the remaining gap concrete.

The highest-value fix is alignment, not reinvestigation: the body
already describes partial closure reasonably well. The title, status
banner, and section headlines need to match that body.

### F.01 / F.03 / F.04 — Cognitive kernel frontier

Namespaces, cognitive scheduling, Engram syscalls — all Phase 2+.

### F.06 / F.07 — Compliance packaging frontier

Forensic-AI positioning is real; regulator-specific export templates
are frontier.

Acceptance criteria:

- Doc 16 headline reflects partial closure,
- Doc 16 body, title, and status banner all describe the same bounded
  residual gap,
- Doc 14 cognitive-kernel subsections marked Phase 2+,
- Doc 15 marked positioning/foundation with F.05 cross-link.
