#!/usr/bin/env python3
"""T7: the envelope by level and family, the mandatory honest envelope report (§6.2; App.). Spec: FIGURES-TABLES, T7.

    tab_t7_envelope.py INPUT [INPUT ...] --out DIR [--dry-run]

Rows: each level ℓ1-ℓ5, pooled and then per family (F1-F5, F7, as the records have them), from the P1-core
experiments (`LOG1`, `E-P1-live`), then the external slice (`E-P1-ext`, pooled). Columns:
- n: roko_full's `vs_rate` n (runs);
- for each cell of roko_full, fd_claude and cheap_direct: VS rate [CI] (`vs_rate`), $/VS [CI] (`usd_per_vs`,
  api_equiv_usd) and pass^3 (`pass_hat_3`) at that level and family. `report.py` emits only the label metrics per
  family x level, so a family row's $/VS prints "–" until a producer adds it;
- R_m and C_m [CI] cumulative (`envelope_ratio_r`, `envelope_ratio_c`, the primary) and per level
  (`envelope_ratio_r_level`, `envelope_ratio_c_level`), each with arms {roko_full, fd_claude};
- the verdict, derived from roko_full's ratios (cumulative where the row has them, else per level) against X = 0.90
  and Y = 0.30 (S09 §4.1): "within target" when R's lower bound >= X and C's upper bound <= Y; "frontier wins" when
  R's upper bound < X; "cost target missed" when R's lower bound >= X but C's upper bound > Y; "VS short" when C's
  upper bound <= Y but R's lower bound < X; otherwise "inconclusive". Task 3338's envelope.py may state its own
  mapping; this one follows the claim rule.
figlib has the refusal rules and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("T7", "t7-envelope", "Envelope by level × family", "tab_t7_envelope.py", kind="table")
READS = {
    "envelope_ratio_r": "cumulative R_m, arms {roko_full, fd_claude}, ladder m (the primary)",
    "envelope_ratio_c": "cumulative C_m, likewise",
    "envelope_ratio_r_level": "R at level m alone, arms {roko_full, fd_claude}, ladder m (and family, per family)",
    "envelope_ratio_c_level": "C at level m alone, likewise",
}
ARMS = ("roko_full", "fd_claude", "cheap_direct")
LEVELS = (1, 2, 3, 4, 5)
CORE_STREAM = (None, "p1_core")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    core = figlib.P1_CORE
    cells = sorted({(rec.arm, rec.model) for rec in inputs.where("vs_rate", experiment=core + figlib.P1_EXT, level=ANY,
                                                                   family=ANY) if rec.arm in ARMS},
                   key=lambda cell: (ARMS.index(cell[0]), cell[1] or ""))
    names = [figlib.metrics.cell_name(arm, model) for arm, model in cells]
    columns = ["level", "family", "n (runs)"]
    for name in names:
        columns += [f"{name}: VS [CI]", f"{name}: $/VS [CI]", f"{name}: pass^3"]
    columns += ["R_m cum. [CI]", "C_m cum. [CI]", "R level [CI]", "C level [CI]", "verdict"]
    rows = table.part("", columns)
    families = sorted({rec.family for rec in inputs.where("vs_rate", experiment=core, level=ANY, family=ANY)
                       if rec.family})
    plan = [(f"ℓ{level}", level, family, core) for level in LEVELS for family in (None, *families)]
    plan.append(("P1-ext", None, None, figlib.P1_EXT))
    for label, level, family, experiments in plan:
        where = {"experiment": experiments, "level": level, "family": family}
        row = f"{label} {family or 'pooled'}"
        first = inputs.one("vs_rate", arm="roko_full", model=None, **where)
        if first is None and not inputs.where("vs_rate", **where):
            continue
        line = [label, family or "pooled", f"{first.n}" if first is not None else "–"]
        for arm, model in cells:
            name = figlib.metrics.cell_name(arm, model)
            cut = {**where, "arm": arm, "model": model}
            line += [table.cell(inputs.one("vs_rate", **cut), part="T7", row=row, column=f"{name}: VS"),
                     table.cell(inputs.one("usd_per_vs", cost_basis="api_equiv_usd", **cut), part="T7", row=row,
                                column=f"{name}: $/VS", kind="usd"),
                     table.cell(inputs.one("pass_hat_3", clauses={"stream.id": CORE_STREAM}, **cut), part="T7",
                                row=row, column=f"{name}: pass^3", with_ci=False)]
        ratios = {}
        for metric in ("envelope_ratio_r", "envelope_ratio_c", "envelope_ratio_r_level", "envelope_ratio_c_level"):
            rec = inputs.one(metric, arm="roko_full", against=(figlib.REFERENCE_ARM,), **where)
            line.append(table.cell(rec, part="T7", row=row, column=metric))
            if rec is not None and rec.value is not None and rec.ci is not None:
                ratios[metric] = rec
        line.append(_verdict(table, row, ratios))
        rows.append(line)
    table.footer += [f"Verdicts compare roko_full's ratios with X = {figlib.X_BAR:.2f} and Y = {figlib.Y_BAR:.2f} "
                     "(D3; S09 §4.1), cumulative where the row has them, else per level.",
                     "Family rows are per level, not cumulative; report.py emits only label metrics per family."]
    table.design("T7", "H1's bars X and Y", [figlib.X_BAR, figlib.Y_BAR], "D3; S09 §4.1")
    return table


def _verdict(table: figlib.Table, row: str, ratios: dict) -> str:
    pairs = (("envelope_ratio_r", "envelope_ratio_c"), ("envelope_ratio_r_level", "envelope_ratio_c_level"))
    for r_name, c_name in pairs:
        r, c = ratios.get(r_name), ratios.get(c_name)
        if r is None or c is None:
            continue
        if r.ci[0] >= figlib.X_BAR and c.ci[1] <= figlib.Y_BAR:
            verdict = "within target"
        elif r.ci[1] < figlib.X_BAR:
            verdict = "frontier wins"
        elif r.ci[0] >= figlib.X_BAR:
            verdict = "cost target missed"
        elif c.ci[1] <= figlib.Y_BAR:
            verdict = "VS short"
        else:
            verdict = "inconclusive"
        return str(table.derive("T7", f"verdict for {row} from {r_name} and {c_name}", verdict, [r, c]))
    return "–"


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
