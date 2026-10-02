#!/usr/bin/env python3
"""T8: the SASO scorecard (§7.2, §7.4; H6, X2). Spec: paper/FIGURES-TABLES.md, T8.

    tab_t8_saso.py INPUT [INPUT ...] --out DIR [--dry-run]

One row per disturbance kind x controller (A0-A5, then the exploratory A3-gated and A3-mis), each number a
MetricRecord at clauses `disturbance.type` and `controller`, as the R-H6 replay (Stage A) and the live Stage B
summarise S06's `controller/1` rows (`recovery`, `ev.breach`, `param.change`, `param.evaluate`, `controller.hold`):
`iae` at `stage == "replay"` with its CI, `detect_delay`, `settling_time` (resolutions), `settling_usd`,
`settling_s`, `overshoot`, `ss_error`, `collateral_max`, `adaptation_cost_usd`, `param_changes`, `rollbacks` and
`hold_count`; for the Stage B kinds, how many live IAEs (`iae` at `stage == "live"`, one per `replicate`) fall inside
the replay's 90% predictive interval (`iae_pi90`), derived. figlib has the refusal rules and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("T8", "t8-saso", "SASO scorecard", "tab_t8_saso.py", kind="table", p1=False)
READS = {
    "iae": "integrated absolute error (clauses disturbance.type, controller, stage replay with a CI, or stage live "
           "per replicate)",
    "iae_pi90": "the replay's 90% predictive interval for a live run's IAE",
    "detect_delay, settling_time": "resolutions to the confirmed breach and to settling",
    "settling_usd, settling_s": "the settling cost in dollars and seconds",
    "overshoot, ss_error, collateral_max": "SASO's overshoot, steady-state error and largest collateral excursion",
    "adaptation_cost_usd": "what the controller's own actions cost",
    "param_changes, rollbacks, hold_count": "counts of applied changes, rollbacks and HOLDs",
}
KINDS = ("provider_fault", "model_swap", "harder_mix", "budget_cut", "convention_flip", "flaky_verify")
CONTROLLERS = ("A0", "A1", "A2", "A3", "A4", "A5", "A3-gated", "A3-mis")
COLUMNS = ("kind", "controller", "IAE [CI]", "detection delay", "settling (res., $, s)", "overshoot",
           "steady-state error", "collateral", "adaptation $", "param. changes", "rollbacks", "HOLDs",
           "live IAE in replay PI")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    rows = table.part("", COLUMNS)
    found = {(rec.eq("disturbance.type"), rec.eq("controller")) for rec in inputs.where(
        "iae", arm=ANY, experiment=ANY, clauses={"disturbance.type": ANY, "controller": ANY, "stage": "replay"})}
    for kind, controller in sorted(found, key=lambda cell: (KINDS.index(cell[0]) if cell[0] in KINDS else 99,
                                                            CONTROLLERS.index(cell[1]) if cell[1] in CONTROLLERS
                                                            else 99, cell)):
        name = f"{kind}/{controller}"
        cut = {"disturbance.type": kind, "controller": controller}

        def cell(metric: str, kind_: str = "rate", **more: object) -> str:
            return table.cell(inputs.one(metric, arm=ANY, clauses={**cut, **more}), part="T8", row=name,
                              column=metric, kind=kind_, with_ci=False)

        settling = ", ".join((cell("settling_time", "count"), cell("settling_usd", "usd"), cell("settling_s", "count")))
        rows.append([kind, controller,
                     table.cell(inputs.one("iae", arm=ANY, clauses={**cut, "stage": "replay"}), part="T8", row=name,
                                column="IAE"),
                     cell("detect_delay", "count"), settling, cell("overshoot"), cell("ss_error"),
                     cell("collateral_max"), cell("adaptation_cost_usd", "usd"), cell("param_changes", "count"),
                     cell("rollbacks", "count"), cell("hold_count", "count"), _live(inputs, table, cut, name)])
    if not rows:
        table.skip("T8", "no replay iae records per kind and controller")
    table.footer.append("A3-gated and A3-mis are exploratory (X2). Settling is in resolutions, dollars and seconds; "
                        "the live column counts Stage B IAEs inside the replay's 90% predictive interval.")
    return table


def _live(inputs: figlib.Inputs, table: figlib.Table, cut: dict, name: str) -> str:
    interval = inputs.one("iae_pi90", arm=ANY, clauses=cut)
    live = inputs.where("iae", arm=ANY, experiment=ANY, clauses={**cut, "stage": "live", "replicate": ANY})
    if interval is None or not live or interval.ci is None:
        return "–"
    table.take(interval, panel="T8", series=name, role="replay IAE, 90% PI")
    values = [table.take(rec, panel="T8", series=name, role=f"live IAE {rec.eq('replicate')}") for rec in live]
    inside = sum(value is not None and interval.ci[0] <= value <= interval.ci[1] for value in values)
    return str(table.derive("T8", f"{name}: live IAEs inside the replay's 90% PI", f"{inside} of {len(values)}",
                            [interval, *live]))


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
