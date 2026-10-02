#!/usr/bin/env python3
"""F11: the cross-loop closure tests X1-X4 (§7.4; pre-registered exploratory). Spec: paper/FIGURES-TABLES.md, F11.

    fig_f11_closures.py INPUT [INPUT ...] --out DIR [--dry-run]

S09 §4.4 registers X1-X4 as exploratory, outside the Holm family, and every panel says so. Four dot-and-whisker
panels with 95% CIs, each reading the MetricRecords its replay computes (any experiment; the metric and its clauses
pick the cut):
- a (X1): `ece` per label regime (clause `label_regime`: G; A at rho 0.10, 0.15 and 0.30, clause `rho`; C);
- b (X2): a forest plot of `iae_diff`, IAE(controller) - IAE(A4), per disturbance kind and controller (clauses
  `disturbance.type`, `controller`: A3, A3-gated, A3-mis), with a zero line;
- c (X3): `fg_breach_delay`, the delay to detect the false-green breach, with the fixed and the adaptive audit rate
  (clause `mode`) as paired dots, and each mode's `audit_spend_share` as a text label (no dual axis);
- d (X4): `usd_per_vs` (x, log) against `vs_rate` (y) per spec-handling policy (clause `spec_policy`: gate, no_gate,
  always_refine, always_escalate), with `delta_brier_spec`, the Brier change from adding S07's spec features to M3,
  as a text label.
figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F11", "f11-closures", "Cross-loop closure tests X1–X4 (exploratory)", "fig_f11_closures.py",
                   p1=False)
READS = {
    "ece": "ECE per label regime (clauses label_regime, rho for A), with its CI (X1)",
    "iae_diff": "IAE(controller) - IAE(A4) per kind (clauses disturbance.type, controller), with its CI (X2)",
    "fg_breach_delay": "resolutions to detect the false-green breach (clause mode: fixed or adaptive), with its CI "
                       "(X3)",
    "audit_spend_share": "the audit spend share of run spend (clause mode) (X3)",
    "vs_rate, usd_per_vs (per spec policy)": "report.py's names at clause spec_policy (X4)",
    "delta_brier_spec": "the Brier change from adding S07's spec features to M3 (X4)",
}
REGIMES = (("G", None), ("A", 0.10), ("A", 0.15), ("A", 0.30), ("C", None))
CONTROLLERS = ("A3", "A3-gated", "A3-mis")
POLICIES = ("gate", "no_gate", "always_refine", "always_escalate")
DOT = figlib.Style("#0072B2", filled=True)


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1100, 860)
    boxes = fig.grid(2, 2, left=150, hgap=170, bottom=70)

    regimes = [(regime, rho, inputs.one("ece", arm=ANY, clauses={"label_regime": regime, "rho": rho}))
               for regime, rho in REGIMES]
    regimes = [(regime, rho, rec) for regime, rho, rec in regimes if rec is not None]
    if regimes:
        names = [regime + (f" ρ={rho:g}" if rho else "") for regime, rho, _ in regimes]
        panel = fig.panel("a", "X1: ECE per label regime (exploratory)", boxes[0],
                          figlib.category_axis(names, "label regime"),
                          figlib.linear_axis([bound for *_, rec in regimes for bound in rec.ci or ()], "ECE"))
        for index, (name, (_, _, rec)) in enumerate(zip(names, regimes), 1):
            value = fig.take(rec, panel="a", role="ECE", series=name, need_ci=True)
            if value is not None:
                panel.whisker_y(index, *rec.ci, DOT.colour)
                panel.point(index, value, DOT, tip=f"{name}: ECE {value:.3f}")
    else:
        fig.skip("a", "no ece records per label regime")

    diffs = [rec for rec in inputs.where("iae_diff", arm=ANY, clauses={"disturbance.type": ANY, "controller": ANY})
             if rec.eq("controller") in CONTROLLERS]
    if diffs:
        diffs.sort(key=lambda rec: (rec.eq("disturbance.type"), CONTROLLERS.index(rec.eq("controller"))))
        rows = [f"{rec.eq('disturbance.type')} {rec.eq('controller')}" for rec in diffs]
        bounds = [bound for rec in diffs for bound in rec.ci or (rec.value or 0,)]
        x_axis = figlib.symmetric_axis(bounds, "IAE(controller) − IAE(A4)")
        panel = fig.panel("b", "X2: IAE against A4 (exploratory)", boxes[1], x_axis,
                          figlib.category_axis(list(reversed(rows)), ""))
        panel.ref(x=0.0, label="0")
        colours = {"A3": "#0072B2", "A3-gated": "#009E73", "A3-mis": "#D55E00"}
        for row, rec in enumerate(reversed(diffs), 1):
            value = fig.take(rec, panel="b", role="IAE difference", series=rows[len(rows) - row], need_ci=True)
            if value is not None:
                style = figlib.Style(colours[rec.eq("controller")], filled=True)
                panel.whisker_x(row, *rec.ci, style.colour)
                panel.point(value, row, style)
    else:
        fig.skip("b", "no iae_diff records")

    delays = [(mode, inputs.one("fg_breach_delay", arm=ANY, clauses={"mode": mode})) for mode in ("fixed", "adaptive")]
    delays = [(mode, rec) for mode, rec in delays if rec is not None]
    if delays:
        panel = fig.panel("c", "X3: delay to the false-green breach (exploratory)", boxes[2],
                          figlib.category_axis([mode for mode, _ in delays], "audit rate"),
                          figlib.linear_axis([bound for _, rec in delays for bound in rec.ci or ()],
                                             "resolutions to detection"))
        points = []
        for index, (mode, rec) in enumerate(delays, 1):
            value = fig.take(rec, panel="c", role="breach-detection delay", series=mode, need_ci=True)
            if value is None:
                continue
            points.append((index, value))
            panel.whisker_y(index, *rec.ci, DOT.colour)
            panel.point(index, value, DOT)
            share = inputs.one("audit_spend_share", arm=ANY, clauses={"mode": mode})
            spent = fig.take(share, panel="c", role="audit spend share", series=mode) if share else None
            if spent is not None:
                panel.text(index, value, f"audit spend {spent:.1%}", dx=10, dy=4, size=9.5)
        panel.line(points, DOT.colour, width=1, dash="3 2")
    else:
        fig.skip("c", "no fg_breach_delay records")

    cells = []
    for policy in POLICIES:
        vs = inputs.one("vs_rate", arm=ANY, clauses={"spec_policy": policy})
        cost = inputs.one("usd_per_vs", arm=ANY, cost_basis="api_equiv_usd", clauses={"spec_policy": policy})
        if vs is not None and cost is not None and None not in (vs.value, cost.value):
            cells.append((policy, vs, cost))
    if cells:
        panel = fig.panel("d", "X4: spec-handling policies (exploratory)", boxes[3],
                          figlib.log_axis([bound for *_, cost in cells for bound in cost.ci or (cost.value,)],
                                          "$/VS, API-equivalent USD (log)"), figlib.rate_axis("VS rate"))
        for policy, vs, cost in cells:
            y = fig.take(vs, panel="d", role="y: VS rate", series=policy, need_ci=True)
            x = fig.take(cost, panel="d", role="x: $/VS", series=policy, need_ci=True)
            style = figlib.Style("#0072B2" if policy == "gate" else figlib.GREY, "diamond", True)
            panel.whisker_x(y, *cost.ci, style.colour)
            panel.whisker_y(x, *vs.ci, style.colour)
            panel.point(x, y, style, label=policy)
        brier = inputs.one("delta_brier_spec", arm=ANY)
        change = fig.take(brier, panel="d", role="ΔBrier from spec features", need_ci=True) if brier else None
        if change is not None:
            panel.text(panel.x.lo, 0.05, f"ΔBrier from spec features: {change:+.3f} [{brier.ci[0]:+.3f}, "
                       f"{brier.ci[1]:+.3f}]", dx=6, size=9.5)
    else:
        fig.skip("d", "no vs_rate and usd_per_vs records per spec policy")
    return fig


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
