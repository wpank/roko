#!/usr/bin/env python3
"""F2: cost–VS Pareto per arm (§6.2, §6.5; H1, and H4's `hybrid`). Spec: paper/FIGURES-TABLES.md, F2.

    fig_f2_pareto.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads, for each arm (or arm and model) in the pooled cell (no level, no family), two MetricRecords that `report.py`
emits: `vs_rate` (y) and `usd_per_vs` with `cost_basis` api_equiv_usd (x), each with its CI. Panel a holds the
P1-core experiments (`LOG1`, `E-P1-live`); panel b the external slice (`E-P1-ext`), whose y axis is the
official-test resolve rate. One mark per cell with whiskers on both axes, the non-dominated frontier as a step line,
and H1's reference lines at 0.90 x VS(fd_claude) and 0.30 x $/VS(fd_claude), which bound its target quadrant
(shaded). A cell with only one of the two numbers, or a null one, is listed as not drawn. The routing policies'
points are F9's. figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib

SPEC = figlib.Spec("F2", "f2-pareto", "Cost–VS Pareto per arm", "fig_f2_pareto.py")
READS: dict[str, str] = {}  # every name F2 reads is one report.py emits
PANELS = (("a", "P1-core, pooled (F8 honeypots excluded)", figlib.P1_CORE, "VS rate"),
          ("b", "P1-ext (SWE-bench Verified slice)", figlib.P1_EXT, "official-test resolve rate"))


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1000, 500)
    for (letter, title, experiments, y_title), box in zip(PANELS, fig.grid(1, 2)):
        marks = []
        for vs in inputs.where("vs_rate", experiment=experiments):
            if vs.arm is None:
                continue
            cost = inputs.one("usd_per_vs", experiment=vs.experiment, arm=vs.arm, model=vs.model,
                              cost_basis="api_equiv_usd")
            if cost is None or None in (vs.value, cost.value):
                fig.skip(letter, f"{vs.series} ({vs.experiment}): " + ("no usd_per_vs record" if cost is None else
                         "a null value: " + ", ".join(f"{rec.metric} {rec.id}" for rec in (vs, cost)
                                                      if rec.value is None)))
                continue
            marks.append((vs, cost, figlib.arm_style(vs.arm)))
        if not marks:
            fig.skip(letter, f"no vs_rate and usd_per_vs pair in {', '.join(experiments)}")
            continue
        twice = {vs.series for vs, _, _ in marks if sum(other.series == vs.series for other, _, _ in marks) > 1}
        x_axis = figlib.log_axis([bound for _, cost, _ in marks for bound in (cost.ci or (cost.value,))],
                                 "$/VS, API-equivalent USD (log scale)")
        panel = fig.panel(letter, title, box, x_axis, figlib.rate_axis(y_title))
        points = []
        for vs, cost, style in marks:
            name = vs.series + (f" ({vs.experiment})" if vs.series in twice else "")
            y = fig.take(vs, panel=letter, role="y: VS rate", series=name, need_ci=True)
            x = fig.take(cost, panel=letter, role="x: $/VS", series=name, need_ci=True)
            points.append((x, y, name, vs, cost))
        _references(fig, panel, letter, points)
        frontier = []
        for x, y, name, vs, cost in sorted(points, key=lambda point: (point[0], -point[1])):
            if not frontier or y > frontier[-1][1]:
                frontier.append((x, y, name, vs, cost))
        steps = [(frontier[0][0], frontier[0][1])]
        for x, y, *_ in frontier[1:]:
            steps += [(x, steps[-1][1]), (x, y)]
        panel.line(steps, "#52514e", width=1.2, dash="2 2")
        fig.derive(letter, "non-dominated frontier (lower $/VS, higher VS), in order: "
                   + ", ".join(name for _, _, name, _, _ in frontier), [name for _, _, name, _, _ in frontier],
                   [rec for point in frontier for rec in point[3:]])
        for x, y, name, vs, cost in points:
            style = figlib.arm_style(vs.arm)
            panel.whisker_x(y, *cost.ci, style.colour)
            panel.whisker_y(x, *vs.ci, style.colour)
            panel.point(x, y, style, label=name, tip=f"{name}: VS {y:.3f}, $/VS {x:.4f}")
    return fig


def _references(fig: figlib.Figure, panel: figlib.Panel, letter: str, points: list[tuple]) -> None:
    """H1's bars against fd_claude's mark in this panel: 0.90 x its VS rate and 0.30 x its $/VS."""
    reference = [point for point in points if point[3].arm == figlib.REFERENCE_ARM and point[3].model is None]
    if len(reference) != 1:
        fig.skip(letter, f"H1's reference lines need one {figlib.REFERENCE_ARM} mark; the panel has {len(reference)}")
        return
    x, y, _, vs, cost = reference[0]
    vs_bar = fig.derive(letter, f"{figlib.X_BAR:.2f} x VS rate of {figlib.REFERENCE_ARM} (H1's bar on R)",
                        figlib.X_BAR * y, [vs])
    cost_bar = fig.derive(letter, f"{figlib.Y_BAR:.2f} x $/VS of {figlib.REFERENCE_ARM} (H1's bar on C)",
                          figlib.Y_BAR * x, [cost])
    fig.design(letter, "H1's bars X and Y", [figlib.X_BAR, figlib.Y_BAR], "D3; S09 §4.1")
    panel.shade(panel.x.lo, cost_bar, vs_bar, 1.0, "#0072B2", opacity=0.07)
    panel.ref(y=vs_bar, label=f"{figlib.X_BAR:.2f} × VS({figlib.REFERENCE_ARM})")
    panel.ref(x=cost_bar, label=f"{figlib.Y_BAR:.2f} × $/VS({figlib.REFERENCE_ARM})")


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
