#!/usr/bin/env python3
"""F3: the envelope by difficulty: VS by level, R_m and C_m against X and Y (§6.2; H1). Spec: FIGURES-TABLES.md, F3.

    fig_f3_envelope.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads, in the P1-core experiments (`LOG1`, `E-P1-live`):
- panel a: `vs_rate` per arm at each level (`ladder` m, no family) with its CI, drawn as a line with a 95% band. The
  levels up to E* (`envelope_level`) are shaded as claimed, and S08 §4.4's design bands (cheap direct and frontier
  direct) are light boxes per level: design targets, never results;
- panel b: the cumulative `envelope_ratio_r` R_m of each arm against fd_claude (`arms` {arm, fd_claude}, `ladder` m)
  with its CI and a reference line at X = 0.90;
- panel c: the cumulative `envelope_ratio_c` C_m (log y) with its CI and a reference line at Y = 0.30. A marker at
  the lowest level that S09 §4.1's rule does not claim names the failing bound (R below X, C above Y, or both);
- panel d (robustness, appendix): `vs_rate_by_irt_quintile` per arm against the 2PL IRT difficulty quintile.
A panel with no records is listed as not drawn. figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F3", "f3-envelope", "The envelope by difficulty", "fig_f3_envelope.py")
READS = {
    "envelope_ratio_r": "R_m = VS rate(arm) / VS rate(fd_claude) over the tasks with level <= m (cumulative; S09 "
                        "§4.1); arms {arm, fd_claude}, ladder m; pooled over levels when it has no ladder",
    "envelope_ratio_c": "C_m, the same ratio of $/VS (api_equiv_usd); cost_basis api_equiv_usd",
    "envelope_level": "E*, the last level claimed (0 when level 1 fails); arms {roko_full, fd_claude}, no ladder",
    "vs_rate_by_irt_quintile": "VS rate over the tasks in 2PL IRT difficulty quintile q (clause irt.quintile == q); "
                               "S09 E3's irt.py (task 3337)",
}
LEVELS = (1, 2, 3, 4, 5)
LEVEL_NAMES = tuple(f"ℓ{level}" for level in LEVELS)
# S08 §4.4's design bands per level: design targets the pilot checks (SC3), never results.
DESIGN_BANDS = {
    ("cheap direct", figlib.ARMS["cheap_direct"].colour): ((0.8, 1.0), (0.6, 0.8), (0.4, 0.6), (0.2, 0.4), (0.0, 0.2)),
    ("frontier direct", figlib.ARMS["fd_claude"].colour): ((0.9, 1.0), (0.85, 1.0), (0.7, 0.9), (0.5, 0.8),
                                                           (0.4, 0.7)),
}


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1120, 820)
    boxes = fig.grid(2, 2, right=150, hgap=190)
    core = figlib.P1_CORE
    levels_axis = figlib.category_axis(LEVEL_NAMES, "difficulty level ℓ (S08 §4.4)")

    # a: VS rate by level per arm, the claimed levels and the design bands.
    series = inputs.series_of("vs_rate", experiment=core, level=ANY)
    lines = {name: [rec for level in LEVELS for rec in inputs.where("vs_rate", experiment=core, level=level)
                    if rec.series == name] for name in series}
    lines = {name: recs for name, recs in lines.items() if recs}
    if lines:
        panel = fig.panel("a", "VS rate by level per arm", boxes[0], levels_axis, figlib.rate_axis("VS rate"))
        star = inputs.one("envelope_level", experiment=core, arm="roko_full", against=(figlib.REFERENCE_ARM,))
        claimed = fig.take(star, panel="a", role="E*: the last level claimed") if star is not None else None
        if claimed:
            panel.shade(0.5, claimed + 0.5, 0.0, 1.0, "#0072B2", opacity=0.06)
            panel.text(0.55, 0.97, f"claimed: ℓ1–ℓ{claimed:g} (E* = {claimed:g})", colour="#0072B2", size=9.5)
        elif claimed == 0:
            panel.text(0.55, 0.97, "no level claimed (E* = 0)", colour="#d03b3b", size=9.5)
        elif star is None:
            fig.skip("a", "no envelope_level record, so no level is shaded as claimed")
        for index, ((band, colour), bounds) in enumerate(DESIGN_BANDS.items()):
            for level, (low, high) in zip(LEVELS, bounds):
                offset = -0.2 if index == 0 else 0.2
                panel.shade(level + offset - 0.12, level + offset + 0.12, low, high, colour, opacity=0.1,
                            stroke=colour)
                fig.design("a", f"S08 §4.4 design band, {band}, ℓ{level}", [low, high], "S08 §4.4")
        ends = []
        for name, recs in lines.items():
            style = figlib.arm_style(recs[0].arm)
            points = [(rec.level, fig.take(rec, panel="a", role=f"VS rate at ℓ{rec.level}", need_ci=True), rec)
                      for rec in recs]
            points = [(level, value, rec) for level, value, rec in points if value is not None]
            panel.band([(level, *rec.ci) for level, _, rec in points], style.colour)
            panel.line([(level, value) for level, value, _ in points], style.colour)
            for level, value, rec in points:
                panel.point(level, value, style, tip=f"{name} ℓ{level}: {value:.3f}")
            if points:
                ends.append((points[-1][0], points[-1][1], name, style.colour))
        panel.end_labels(ends)
    else:
        fig.skip("a", "no per-level vs_rate records in the P1-core experiments")

    # b and c: the cumulative ratios against fd_claude, and the lowest unclaimed level.
    ratios = {}
    for letter, metric, box in (("b", "envelope_ratio_r", boxes[1]), ("c", "envelope_ratio_c", boxes[2])):
        recs = [rec for rec in inputs.where(metric, experiment=core, level=ANY)
                if rec.level and figlib.REFERENCE_ARM in rec.arms and len(rec.arms) == 2]
        if not recs:
            fig.skip(letter, f"no per-level {metric} records against {figlib.REFERENCE_ARM}")
            continue
        if letter == "b":
            y_axis = figlib.linear_axis([bound for rec in recs for bound in rec.ci or ()], "R_m (cumulative)",
                                        minimum=1.2)
        else:
            y_axis = figlib.log_axis([bound for rec in recs for bound in rec.ci or ()] + [figlib.Y_BAR],
                                     "C_m (cumulative, log)", money=False)
        title = (f"R_m = VS(arm) / VS({figlib.REFERENCE_ARM}) against X = 0.90" if letter == "b" else
                 f"C_m = $/VS(arm) / $/VS({figlib.REFERENCE_ARM}) against Y = 0.30")
        panel = fig.panel(letter, title, box, levels_axis, y_axis)
        bar = figlib.X_BAR if letter == "b" else figlib.Y_BAR
        fig.design(letter, "H1's bar " + ("X on R" if letter == "b" else "Y on C"), bar, "D3; S09 §4.1")
        panel.ref(y=bar, label=f"{'X' if letter == 'b' else 'Y'} = {bar:.2f}")
        for rec in sorted(recs, key=lambda rec: (rec.arms, rec.level)):
            arm = next(arm for arm in rec.arms if arm != figlib.REFERENCE_ARM)
            style = figlib.arm_style(arm)
            value = fig.take(rec, panel=letter, role=f"{metric} at ℓ{rec.level}", series=arm, need_ci=True)
            if value is None:
                continue
            ratios[(letter, arm, rec.level)] = rec
            panel.whisker_y(rec.level, *rec.ci, style.colour)
            panel.point(rec.level, value, style, tip=f"{arm} ℓ{rec.level}: {value:.3f}")
        _failing_marker(fig, panel, letter, ratios)

    # d: VS rate against IRT difficulty quintiles (appendix).
    quintiles = inputs.where("vs_rate_by_irt_quintile", experiment=core, clauses={"irt.quintile": ANY})
    if quintiles:
        panel = fig.panel("d", "VS rate by IRT difficulty quintile (appendix)", boxes[3],
                          figlib.category_axis([f"Q{q}" for q in range(1, 6)], "2PL IRT difficulty quintile"),
                          figlib.rate_axis("VS rate"))
        ends = []
        for name in sorted({rec.series for rec in quintiles}):
            recs = sorted((rec for rec in quintiles if rec.series == name), key=lambda rec: rec.eq("irt.quintile"))
            style = figlib.arm_style(recs[0].arm)
            points = [(rec.eq("irt.quintile"), fig.take(rec, panel="d", role=f"VS rate, quintile "
                                                         f"{rec.eq('irt.quintile')}", need_ci=True), rec)
                      for rec in recs]
            points = [(q, value, rec) for q, value, rec in points if value is not None]
            panel.line([(q, value) for q, value, _ in points], style.colour, width=1.2)
            for q, value, rec in points:
                panel.whisker_y(q, *rec.ci, style.colour)
                panel.point(q, value, style)
            if points:
                ends.append((points[-1][0], points[-1][1], name, style.colour))
        panel.end_labels(ends)
    else:
        fig.skip("d", "no vs_rate_by_irt_quintile records (irt.py, task 3337)")
    return fig


def _failing_marker(fig: figlib.Figure, panel: figlib.Panel, letter: str, ratios: dict) -> None:
    """On panel c: the lowest level S09 §4.1 does not claim for roko_full, and which bound fails there."""
    if letter != "c":
        return
    for level in LEVELS:
        r, c = ratios.get(("b", "roko_full", level)), ratios.get(("c", "roko_full", level))
        if r is None or c is None:
            fig.skip("c", f"roko_full has no R and C pair at ℓ{level}, so the failing level is not marked")
            return
        checks = (("R below X", r.ci[0] < figlib.X_BAR), ("C above Y", c.ci[1] > figlib.Y_BAR))
        failing = [name for name, fails in checks if fails]
        if failing:
            text = f"ℓ{level}: {' and '.join(failing)}"
            fig.derive("c", "the lowest unclaimed level for roko_full and its failing bound", text, [r, c])
            panel.ref(x=level, label=text, colour="#d03b3b")
            return
    fig.derive("c", "the lowest unclaimed level for roko_full", "none: every level is claimed",
               [ratios[(name, "roko_full", level)] for name in ("b", "c") for level in LEVELS])


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
