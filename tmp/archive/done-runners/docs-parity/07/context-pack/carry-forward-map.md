# Carry-Forward Map - Batch 07 Docs Refresh

Use this when a valid finding shows up during the refresh but does not
belong in the owned docs-only scope.

| Item | Better Home | Keep In 07 As | Why |
|------|-------------|---------------|-----|
| `HealthMonitor`, `StuckDetector`, or `MetaCognitionHook` runtime activation | later implementation pass | status note | this refresh should document current posture, not wire orchestrator code |
| circuit-breaker snapshot persistence / restart safety | later implementation pass | scope note | restart behavior is a code contract, not a docs-only edit |
| `ProcessSupervisor` vs `roko-agent` registry ownership | later implementation pass | caveat note | docs should name the split honestly without choosing code ownership here |
| `PhaseTransition`, `adaptive_timeout_ms`, or attempt-tracking rewiring | later implementation pass | bounded caveat | current surfaces can be described without changing runtime behavior |
| Yerkes-Dodson pressure dial / flow detection | later learning pass | deferred design note | keep visible, but do not treat as active parity-refresh work |
| Good Regulator self-model metrics | later self-model pass | deferred design note | outside the core docs-refresh brief |
| typed `CognitiveSignal` / unified algedonic channel | later signal-channel redesign | deferred design note | the refresh should not reopen signal taxonomy design |
| conductor federation / self-healing / triple-loop learning | later governance or meta-learning pass | roadmap note | explicitly deferred from batch `07` refresh posture |
| Linux cgroup / deployment hardening details | later deployment-hardening pass | roadmap note | useful later, not part of this docs package rewrite |

When deferring, record:

1. the exact doc claim or file,
2. the source-backed current status,
3. the future owner of the work,
4. the minimal note this refresh still needs to leave behind.
