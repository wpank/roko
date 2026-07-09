# Repo Map — Agent Context

Quick reference for topic `00` parity work.

## Parity Write Scope

All edits in this batch stay under:

`/Users/will/dev/nunchi/roko/roko/tmp/docs-parity/00/`

Treat everything else as read-only verification input unless scope is widened.

## Source Precedence

When wording conflicts, trust sources in this order:

1. current code for existence and naming claims,
2. audit files for corrected factual posture,
3. architecture docs for the original claim being rewritten,
4. older parity wording last.

## Canonical Source Inputs

| What | Path | Why |
|------|------|-----|
| Architecture docs | `docs/00-architecture/` | source material being described and narrowed |
| Audit summary | `tmp/refinements-audit/00-MASTER-SUMMARY.md` | overall verdicts and corrected facts |
| Foundation audit | `tmp/refinements-audit/01-foundation-audit.md` | REF01-09 narrowing guidance |
| Architecture parity pack | `tmp/docs-parity/00/` | files being refreshed for consistency |

## Source Paths Worth Spot-Checking

Use code only to verify claims, not to expand the batch into implementation work:

| What | Path |
|------|------|
| Engram and core traits | `crates/roko-core/src/` |
| Event bus | `crates/roko-runtime/src/event_bus.rs` |
| Routing / active inference | `crates/roko-learn/src/` |
| Prompt composition | `crates/roko-compose/src/` |
| `roko-serve` | `crates/roko-serve/` |
| CLI + TUI | `crates/roko-cli/` |

## Useful Checks

```bash
rg -n "36 workspace members|322,088 Rust LOC|200\\+ routes|58K LOC|two live RokoEvent variants" \
  tmp/docs-parity/00
rg -n "planned|deferred|target-state|planning artifact|dependency ordering" tmp/docs-parity/00
! rg -n "HTTP API not wired|Text-mode dashboard only" tmp/docs-parity/00
bash -n tmp/docs-parity/00/run-docs-parity.sh
```

## Working Rule

If a rewrite starts depending on unimplemented code, stop and label the concept `planned` or
`deferred`. The parity pack should describe reality more clearly, not argue missing code into
existence.

Keep the repository baseline explicit while doing that work: 36 workspace members, 322,088 Rust
LOC, `roko-serve` wired with 200+ routes, and the TUI wired at ~58K LOC.
