# C — Runtime Guards and Coverage

Parity review for Docs 05, 06, and 07.

Generated: 2026-04-18

---

## Core Read

The runtime-guard section is mostly a documentation problem, not an implementation problem.

The shipped baseline already includes:

- loop protection in both the agent/conductor path and orchestrator path
- sandbox and path enforcement
- runtime pre-checks and output scrubbing through the dispatcher path

The highest-value correction is to call the shipping guards by name and keep prompt-security frontier material clearly labeled as future work.

---

## Shipping Now

### C.01 — `LoopGuard` ships

`LoopGuard` exists in `roko-orchestrator` at **364 LOC**. Docs 05 and related status materials should cite it directly instead of treating loop protection as only conceptual.

### C.02 — `SandboxEnforcer` ships

`SandboxEnforcer` exists in `roko-orchestrator` at **651 LOC**. Docs that still imply sandboxing is only a future container-sandbox concept are stale.

### C.03 — the agent safety layer still matters

The parity docs should keep the split clear:

- `roko-agent/src/safety/` is the live runtime guard layer
- `roko-orchestrator/src/safety/` adds the higher-level loop, permit, taint, audit, capability, and sandbox pieces

### C.04 — the current integration issue is coverage, not absence

Doc 16's stale framing leaks into this section. The real remaining problem is not "there is no safety runtime." It is that some subprocess and specialty paths still do not route through the shared safety pipeline.

---

## Narrow, Don’t Inflate

### C.05 — prompt-security research patterns are not this batch's implementation gap

Keep CaMeL, Ventriloquist, and similar patterns explicitly deferred.

### C.06 — MCP should not be framed as something Roko avoids

The parity materials should reflect that the live system uses MCP-related surfaces and that the real safety question is dispatcher coverage and guard application, not blanket MCP avoidance.

---

## Recommended Doc Posture

For Docs 05, 06, and 07:

1. cite `LoopGuard` and `SandboxEnforcer` as shipping
2. preserve the live agent-layer guards as the base safety runtime
3. describe the remaining issue as partial coverage
4. move academic prompt-security additions into explicit future-work language
