#!/usr/bin/env python3
"""T10: the audit battery's validity per check (§7.1; data in appendix C; H5 secondary). Spec: FIGURES-TABLES, T10.

    tab_t10_battery.py INPUT [INPUT ...] --out DIR [--dry-run]

One row per S05 battery check (clause `check`: A1 tamper diff, A2 clean re-run, B1 hidden tests, B2 diff-scoped
mutation, B3 cross-family review), the gold tasks (G) and the honeypot rule. The MetricRecords are what the audit
results (`roko.audit/1` `checks[]` and `labels`) give against the truth suite and the planted labels
(`planted_gaming`, `is_honeypot`; S08's `hidden.py` gaming flags): `check_runs`, `sensitivity` and `specificity`
with their CIs, `usd_per_check` (api_equiv_usd, the LLM cost per run of the check) and `dismissed_battery_fp` (S05's
incident records). figlib has the refusal rules and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("T10", "t10-battery", "Audit battery validity per check", "tab_t10_battery.py", kind="table",
                   p1=False)
READS = {
    "check_runs": "how many times the check ran (clause check)",
    "sensitivity": "the check's sensitivity against the truth suite and planted labels, with its CI",
    "specificity": "its specificity, with its CI",
    "usd_per_check": "the LLM cost per run of the check (cost_basis api_equiv_usd)",
    "dismissed_battery_fp": "battery false positives that S05's incident review dismissed",
}
CHECKS = {"A1": "A1 tamper diff", "A2": "A2 clean re-run", "B1": "B1 hidden tests", "B2": "B2 diff-scoped mutation",
          "B3": "B3 cross-family review", "G": "G gold tasks", "honeypot": "the honeypot rule"}
COLUMNS = ("check", "runs", "sensitivity [CI]", "specificity [CI]", "LLM $ per check", "dismissed false positives")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    rows = table.part("", COLUMNS)
    present = {rec.eq("check") for metric in ("check_runs", "sensitivity", "specificity")
               for rec in inputs.where(metric, arm=ANY, experiment=ANY, clauses={"check": ANY})}
    for check in sorted(present, key=lambda check: (list(CHECKS).index(check) if check in CHECKS else 99, check)):
        name = CHECKS.get(check, check)

        def cell(metric: str, kind: str = "rate", with_ci: bool = False, **more: object) -> str:
            return table.cell(inputs.one(metric, arm=ANY, experiment=ANY, clauses={"check": check}, **more),
                              part="T10", row=name, column=metric, kind=kind, with_ci=with_ci)

        rows.append([name, cell("check_runs", "count"), cell("sensitivity", with_ci=True),
                     cell("specificity", with_ci=True), cell("usd_per_check", "usd", cost_basis="api_equiv_usd"),
                     cell("dismissed_battery_fp", "count")])
    if not rows:
        table.skip("T10", "no battery records per check")
    table.footer.append("Sensitivity and specificity are measured against the truth suite and the planted labels; a "
                        "check that did not run prints \"–\".")
    return table


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
