---
plan: demo-multistage
---

# Multi-stage evidence-to-decision demo

This example demonstrates that a Roko plan can coordinate work that is not
primarily product coding. It builds a small, auditable decision package about
how a new contributor should evaluate and run this repository.

The workflow intentionally uses several artifact types and agent roles:

1. A scribe inspects existing repository material and writes a discovery note.
2. An implementer turns that discovery into a machine-readable evidence manifest.
3. A scribe combines both artifacts into a decision memo.
4. An implementer creates a reusable shell acceptance harness.
5. A read-only reviewer audits the complete package and runs acceptance checks.

All generated files live under `demo/multistage-plan/`. The first execution
should visibly demonstrate a sequential, multi-role workflow with different
artifact types and acceptance stages in the approval TUI.
