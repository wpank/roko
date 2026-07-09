# Gaps Summary — 11 Safety

## Primary gaps for this pass

### 1. Core parity docs still understate the shipped safety system — HIGH

- `00-INDEX.md`, `SOURCE-INDEX.md`, and files `A` through `C` need to lead with the shipped two-crate, `7,183`-LOC framing.
- The problem is mostly status drift, not missing safety primitives.

### 2. Threat/risk and Doc 16 need a status refresh, not a redesign — HIGH

- `D-threat-risk-adaptive.md` and `F-kernel-forensics-gap.md` should describe current coverage honestly.
- "Coverage status" is the right frame; "generic critical gap" is too loose for this pass.

### 3. Chain-safety material needs explicit defer language — MEDIUM

- `E-chain-safety.md` should be treated as a defer/containment pass.
- Tier-6 chain, formal methods, and frontier verification work are not blockers for parity batch `11`.

### 4. The context pack and runner must stay aligned on the narrowed scope — MEDIUM

- Context notes and `run-docs-parity.sh` should stay aligned on the current `M1`-`M5` plan.
- Verification should stay centered on `tmp/docs-parity/11`.

## Working rule

If the task stops being a `tmp/docs-parity/11` doc/status refresh and starts implying new runtime, compliance, chain, or kernel implementation, defer it.
