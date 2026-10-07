#!/usr/bin/env python3
"""F5: the spec x model interaction (§6.4; H3). Spec: paper/FIGURES-TABLES.md, F5.

    fig_f5_spec_interaction.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads the H3 stream's records (clause `stream.id == "p1_h3"`), pooled over levels and families:
- panel a: `vs_rate` per tier and spec variant (clause `task.spec_variant`): the cheap tier is `roko_fixed` on
  gpt-oss-120b (its arm cell, or its `model == "gpt-oss-120b"` cell when the arm ran two models), the frontier tier is
  `fr_claude`. Two lines, vague to precise, with CI whiskers; the refined variant is offset beside precise for the
  cheap tier only; any other `roko_fixed` model (the GLM-4.7 robustness arm) is a grey point at vague. Faint lines
  join each task's vague and precise `vs_rate` (clauses `task.instance_id` and `task.spec_variant`), so the pairing
  shows. The interaction `interaction_rd` (arms {roko_fixed, fr_claude}) is printed with its CI.
- panel b: `usd_per_vs` (api_equiv_usd) per tier and variant on a log axis, with CI whiskers.
figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F5", "f5-spec-interaction", "Spec × model interaction", "fig_f5_spec_interaction.py")
READS = {
    "interaction_rd": "H3's interaction, (VS precise - VS vague) of the cheap tier minus that of the frontier tier, "
                      "a risk difference with its task-cluster bootstrap CI; arms {roko_fixed, fr_claude}, clause "
                      "stream.id == p1_h3",
    "vs_rate (per task)": "report.py's vs_rate at one task and variant (clauses task.instance_id, task.spec_variant): "
                          "the paired per-task slopes",
}
VARIANTS = ("vague", "precise")
X = {"vague": 1.0, "precise": 2.0, "refined": 2.28}
CHEAP_MODEL = "gpt-oss-120b"


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1000, 500)
    boxes = fig.grid(1, 2)
    h3 = {"stream.id": figlib.H3_STREAM, "task.instance_id": None}
    tiers = _tiers(inputs, h3)
    if not tiers:
        fig.skip("a", f"no vs_rate records of roko_fixed or fr_claude in stream {figlib.H3_STREAM}")
        return fig
    axis = figlib.category_axis(VARIANTS, "spec variant (refined: cheap tier only, beside precise)")
    panel = fig.panel("a", "VS rate by spec variant and tier", boxes[0], axis, figlib.rate_axis("VS rate"))
    for tier, (arm, model) in tiers.items():
        style = figlib.arm_style(arm)
        points = []
        for variant in (*VARIANTS, "refined") if tier == "cheap" else VARIANTS:
            rec = inputs.one("vs_rate", arm=arm, model=model, clauses={**h3, "task.spec_variant": variant})
            value = fig.take(rec, panel="a", role=f"VS rate, {variant}", need_ci=True) if rec else None
            if value is not None:
                points.append((variant, value, rec))
        panel.line([(X[variant], value) for variant, value, _ in points if variant in VARIANTS], style.colour)
        for variant, value, rec in points:
            panel.whisker_y(X[variant], *rec.ci, style.colour)
            panel.point(X[variant], value, style, tip=f"{tier} ({arm}) {variant}: {value:.3f}",
                        label="refined" if variant == "refined" else None)
        if points:
            panel.text(X[points[0][0]], points[0][1], f"{tier}: {figlib.metrics.cell_name(arm, model)}",
                       colour=figlib.ink(style.colour), anchor="end", dx=-10, dy=4, size=9.5)
        _slopes(inputs, fig, panel, arm, model, style)
    for rec in inputs.where("vs_rate", arm="roko_fixed", model=ANY,
                            clauses={**h3, "task.spec_variant": "vague"}):
        if rec.model not in (None, CHEAP_MODEL):  # the robustness model, a grey point at vague
            value = fig.take(rec, panel="a", role="VS rate, vague (robustness model)", need_ci=True)
            if value is not None:
                panel.whisker_y(X["vague"] - 0.18, *rec.ci, figlib.GREY)
                panel.point(X["vague"] - 0.18, value, figlib.Style(figlib.GREY, "diamond"), label=rec.model)
    interaction = inputs.one("interaction_rd", arm="roko_fixed", against=("fr_claude",),
                             clauses={"stream.id": figlib.H3_STREAM})
    if interaction is not None and fig.take(interaction, panel="a", role="interaction (risk difference)",
                                            need_ci=True) is not None:
        low, high = interaction.ci
        panel.text(1.0, 0.04, f"interaction Δ = {interaction.value:+.3f} [{low:+.3f}, {high:+.3f}]", size=10.5,
                   dx=-40)
    elif interaction is None:
        fig.skip("a", "no interaction_rd record, so Δ is not printed")

    cells = []
    for tier, (arm, model) in tiers.items():
        for variant in (*VARIANTS, "refined") if tier == "cheap" else VARIANTS:
            rec = inputs.one("usd_per_vs", arm=arm, model=model, cost_basis="api_equiv_usd",
                             clauses={**h3, "task.spec_variant": variant})
            if rec is not None and rec.value is not None:
                cells.append((tier, arm, variant, rec))
            elif rec is not None:
                fig.take(rec, panel="b", role=f"$/VS, {variant}")  # listed as not drawn
    if not cells:
        fig.skip("b", "no usd_per_vs records in the H3 stream")
        return fig
    panel = fig.panel("b", "$/VS by spec variant and tier", boxes[1], axis,
                      figlib.log_axis([bound for *_, rec in cells for bound in rec.ci or (rec.value,)],
                                      "$/VS, API-equivalent USD (log)"))
    for tier, arm, variant, rec in cells:
        style = figlib.arm_style(arm)
        value = fig.take(rec, panel="b", role=f"$/VS, {variant}", need_ci=True)
        panel.whisker_y(X[variant], *rec.ci, style.colour)
        panel.point(X[variant], value, style, tip=f"{tier} {variant}: ${value:.4f}",
                    label=tier if variant == "vague" else ("refined" if variant == "refined" else None))
    for tier, (arm, _) in tiers.items():
        pts = [(X[variant], rec.value) for t, _, variant, rec in cells if t == tier and variant in VARIANTS]
        panel.line(pts, figlib.arm_style(arm).colour, width=1.2)
    return fig


def _tiers(inputs: figlib.Inputs, h3: dict) -> dict[str, tuple[str, str | None]]:
    """{"cheap": (arm, model), "frontier": (arm, model)} for the cells the H3 stream has."""
    found = {}
    for tier, arm, models in (("cheap", "roko_fixed", (None, CHEAP_MODEL)), ("frontier", "fr_claude", (None,))):
        recs = [rec for rec in inputs.where("vs_rate", arm=arm, model=ANY, clauses={**h3, "task.spec_variant": ANY})
                if rec.model in models]
        if recs:
            found[tier] = (arm, recs[0].model)
    return found


def _slopes(inputs: figlib.Inputs, fig: figlib.Figure, panel: figlib.Panel, arm: str, model: str | None,
            style: figlib.Style) -> None:
    """Faint vague -> precise lines per task, from the per-task vs_rate records."""
    per_task = {}
    for rec in inputs.where("vs_rate", arm=arm, model=model, clauses={"stream.id": figlib.H3_STREAM,
                                                                      "task.instance_id": ANY,
                                                                      "task.spec_variant": ANY}):
        if rec.eq("task.spec_variant") in VARIANTS:
            per_task.setdefault(rec.eq("task.instance_id"), {})[rec.eq("task.spec_variant")] = rec
    for task, recs in sorted(per_task.items()):
        if set(recs) != set(VARIANTS):
            fig.skip("a", f"task {task} of {arm} lacks a {' or '.join(set(VARIANTS) - set(recs))} record; no slope")
            continue
        values = [fig.take(recs[variant], panel="a", role=f"task {task}, {variant}") for variant in VARIANTS]
        if None not in values:
            panel.line([(X[variant], value) for variant, value in zip(VARIANTS, values)], style.colour, width=0.8,
                       opacity=0.25)


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
