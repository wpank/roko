#!/usr/bin/env python3
"""F8: the audit false-green estimate against census truth (§7.1, §7.4; H5, X3). Spec: FIGURES-TABLES.md, F8.

    fig_f8_audit.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads the MetricRecords that the R-H5 replay and the live H5 stream (`E-H5-live`) compute from `roko.audit/1`
(selection and result rows) joined to the census's `vs.label`:
- panel a (R-H5): `coverage` of the theta-hat interval per cell, clauses `rho` (0.10, 0.15, 0.30) and `mode`
  (uniform, tilted, theta_ai; adaptive for X3), on the labelled zoomed axis [0.80, 1.00] with Wilson whiskers and
  reference lines at 0.93 (the criterion) and 0.95 (nominal);
- panel b (E-H5-live): theta-hat (`fgr_hajek`, Hajek, with its always-valid band) for H5-A1 and H5-A3 against
  evaluation-task position (clause `stream.position`), and the census `theta_true` at each position as a step line;
- panel c: `theta_true` per H5 arm (H5-A0, H5-A1, H5-A3) with its CI, and a reference at 0.5 x theta_true(H5-A0).
The H5 arms are not P1 arms, so they have their own fixed colours (`H5_ARMS`). figlib has the refusal rules, the
encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F8", "f8-audit", "Audit false-green estimate against census truth", "fig_f8_audit.py", p1=False)
READS = {
    "coverage": "the share of replay lotteries whose theta-hat interval covers theta_true (clauses rho, mode), with "
                "its Wilson CI",
    "fgr_hajek": "the Hajek false-green estimate at an evaluation-task position (clause stream.position), with its "
                 "always-valid band",
    "theta_true": "the census false-green rate among green units, at a position (clause stream.position) or pooled",
}
REPLAY, LIVE = "R-H5", "E-H5-live"
MODES = ("uniform", "tilted", "theta_ai", "adaptive")
RHOS = {0.10: -0.22, 0.15: 0.0, 0.30: 0.22}  # x offsets within a mode's group
H5_ARMS = {"H5-A0": figlib.Style(figlib.GREY, filled=True), "H5-A1": figlib.Style("#E69F00", filled=True),
           "H5-A3": figlib.Style("#0072B2", "diamond", True)}
CRITERION, NOMINAL = 0.93, 0.95


def h5_style(arm: str) -> figlib.Style:
    if arm not in H5_ARMS:
        raise figlib.FigureError(f"H5 arm {arm!r} has no fixed colour in fig_f8_audit.H5_ARMS; add it there")
    return H5_ARMS[arm]


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1180, 470)
    boxes = fig.grid(1, 3, hgap=90, bottom=70)
    cells = inputs.where("coverage", experiment=REPLAY, arm=ANY, clauses={"rho": ANY, "mode": ANY})
    if cells:
        modes = [mode for mode in MODES if any(rec.eq("mode") == mode for rec in cells)]
        panel = fig.panel("a", "Coverage of θ̂'s interval (replay)", boxes[0],
                          figlib.category_axis(modes, "mode (ρ = 0.10, 0.15, 0.30 left to right)"),
                          figlib.rate_axis("coverage", 0.80, 1.00, 0.05))
        fig.design("a", "the coverage criterion and the nominal level", [CRITERION, NOMINAL], "S09 §4.1 (H5)")
        panel.ref(y=CRITERION, label=f"criterion {CRITERION}")
        panel.ref(y=NOMINAL, label=f"nominal {NOMINAL}")
        for rec in cells:
            mode, rho = rec.eq("mode"), rec.eq("rho")
            if mode not in modes or rho not in RHOS:
                fig.skip("a", f"coverage cell mode {mode!r}, rho {rho!r} is outside the figure's grid")
                continue
            value = fig.take(rec, panel="a", role=f"coverage, mode {mode}, rho {rho}", series=f"{mode}/{rho}",
                             need_ci=True)
            if value is not None:
                x = modes.index(mode) + 1 + RHOS[rho]
                style = figlib.Style("#0072B2" if mode != "adaptive" else "#CC79A7", filled=rho == 0.15)
                panel.whisker_y(x, *rec.ci, style.colour)
                panel.point(x, value, style, tip=f"{mode}, ρ = {rho}: {value:.3f}")
    else:
        fig.skip("a", "no coverage records")

    estimates = {}
    for metric in ("fgr_hajek", "theta_true"):
        for rec in inputs.where(metric, experiment=LIVE, arm=ANY, clauses={"stream.position": ANY}):
            if rec.arm is not None:
                estimates.setdefault((metric, rec.arm), []).append(rec)
    if estimates:
        positions = [rec.eq("stream.position") for recs in estimates.values() for rec in recs]
        panel = fig.panel("b", "θ̂ and the census θ over the live stream", boxes[1],
                          figlib.linear_axis(positions, "evaluation-task position"),
                          figlib.rate_axis("false-green rate"))
        ends = []
        for (metric, arm), recs in sorted(estimates.items()):
            style = h5_style(arm)
            recs.sort(key=lambda rec: rec.eq("stream.position"))
            points = [(rec.eq("stream.position"), fig.take(rec, panel="b", role=f"{metric} at "
                                                           f"{rec.eq('stream.position')}", need_ci=metric ==
                                                           "fgr_hajek"), rec) for rec in recs]
            points = [(position, value, rec) for position, value, rec in points if value is not None]
            if metric == "fgr_hajek":
                panel.band([(position, *rec.ci) for position, _, rec in points], style.colour, opacity=0.14)
                panel.line([(position, value) for position, value, _ in points], style.colour, width=1.5)
                label = f"θ̂ {arm}"
            else:
                steps = []
                for position, value, _ in points:
                    steps += [(position, steps[-1][1]), (position, value)] if steps else [(position, value)]
                panel.line(steps, style.colour, width=1.1, dash="4 2")
                label = f"θ {arm} (census)"
            if points:
                ends.append((points[-1][0], points[-1][1], label, style.colour))
        panel.end_labels(ends)
    else:
        fig.skip("b", "no live fgr_hajek or theta_true records")

    pooled = [rec for rec in inputs.where("theta_true", experiment=LIVE, arm=ANY) if rec.arm in H5_ARMS]
    if pooled:
        pooled.sort(key=lambda rec: list(H5_ARMS).index(rec.arm))
        panel = fig.panel("c", "θ (census) per H5 arm", boxes[2],
                          figlib.category_axis([rec.arm for rec in pooled], "H5 arm"),
                          figlib.rate_axis("census false-green rate θ"))
        for index, rec in enumerate(pooled, 1):
            value = fig.take(rec, panel="c", role="theta_true", need_ci=True)
            if value is not None:
                panel.whisker_y(index, *rec.ci, h5_style(rec.arm).colour)
                panel.point(index, value, h5_style(rec.arm), tip=f"{rec.arm}: θ {value:.3f}")
                if rec.arm == "H5-A0":
                    half = fig.derive("c", "0.5 x theta_true of H5-A0 (H5's target)", 0.5 * value, [rec])
                    panel.ref(y=half, label="0.5 × θ(H5-A0)")
    else:
        fig.skip("c", "no pooled theta_true records per H5 arm")
    return fig


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
