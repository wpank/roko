#!/usr/bin/env python3
"""F7: SASO step responses per disturbance and controller (§7.2; H6). Spec: paper/FIGURES-TABLES.md, F7.

    fig_f7_saso.py INPUT [INPUT ...] --out DIR [--dry-run]

Small multiples, one panel per disturbance kind (clause `disturbance.type`; the `flaky_verify` panel is labelled
"unregulable: expected HOLD"). Each reads S06's `controller/1` and `disturbances.jsonl` as the R-H6 replay
(`experiment_id == "R-H6"`) and the live Stage B (`E-H6-live`) summarise them in MetricRecords:
- `drive_median` per controller (clause `controller`: A0 static, A2 reactive, A3 M1) and offset in resolutions from
  onset (clause `offset`, -10 to +40): the median drive D(t) >= 0, with its 90% replay band;
- vertical markers at onset, at A3's median confirmed breach (`detect_delay`) and at its median settle
  (`settling_time`), both in resolutions; rug ticks where A3 applied parameter changes (`param_change_count` > 0 at an
  offset);
- for the live kinds, the 3 live IAEs (`iae` with clauses `stage == "live"` and `replicate`) against the replay's 90%
  predictive interval (`iae_pi90`), counted in the panel and over all kinds (the >= 7 of 9 validity check).
A1, A4 and A5 belong to the appendix version. figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F7", "f7-saso", "SASO step responses per disturbance and controller", "fig_f7_saso.py", p1=False)
READS = {
    "drive_median": "the median drive D(t) at an offset from onset (clauses disturbance.type, controller, offset), "
                    "with its 90% replay band",
    "detect_delay": "the median resolutions from onset to the confirmed breach (clauses disturbance.type, controller)",
    "settling_time": "the median resolutions from onset to settling (clauses disturbance.type, controller)",
    "param_change_count": "parameter changes applied at an offset (clauses disturbance.type, controller, offset)",
    "iae": "integrated absolute error; stage live, one per replicate (clauses disturbance.type, controller, stage, "
           "replicate)",
    "iae_pi90": "the replay's median IAE with its 90% predictive interval for one live run (clauses "
                "disturbance.type, controller)",
}
KINDS = ("provider_fault", "model_swap", "harder_mix", "budget_cut", "convention_flip", "flaky_verify")
CONTROLLERS = {"A0": ("static", figlib.GREY), "A2": ("reactive", "#E69F00"), "A3": ("M1", "#0072B2")}
REPLAY, LIVE = "R-H6", "E-H6-live"


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1150, 820)
    boxes = fig.grid(2, 3, right=40, hgap=74, bottom=78)
    inside_total, live_total = [], 0
    for letter, kind, box in zip("abcdef", KINDS, boxes):
        lines = {}
        for rec in inputs.where("drive_median", experiment=REPLAY, arm=ANY,
                                clauses={"disturbance.type": kind, "controller": ANY, "offset": ANY}):
            if rec.eq("controller") in CONTROLLERS:
                lines.setdefault(rec.eq("controller"), []).append(rec)
        if not lines:
            fig.skip(letter, f"no drive_median records for {kind}")
            continue
        offsets = [rec.eq("offset") for recs in lines.values() for rec in recs]
        tops = [bound for recs in lines.values() for rec in recs for bound in rec.ci or (rec.value or 0,)]
        title = kind + (" (unregulable: expected HOLD)" if kind == "flaky_verify" else "")
        x_axis = figlib.Axis(min(offsets), max(offsets), "resolutions from onset",
                             ticks=tuple((t, str(t)) for t in range(-10, 41, 10) if min(offsets) <= t <= max(offsets)))
        panel = fig.panel(letter, title, box, x_axis, figlib.linear_axis(tops, "drive D(t) (0 = viable)"))
        ends = []
        for controller, recs in sorted(lines.items()):
            label, colour = CONTROLLERS[controller]
            recs.sort(key=lambda rec: rec.eq("offset"))
            points = [(rec.eq("offset"), fig.take(rec, panel=letter, role=f"drive at {rec.eq('offset')}",
                                                  series=f"{kind}/{controller}", need_ci=True), rec) for rec in recs]
            points = [(offset, value, rec) for offset, value, rec in points if value is not None]
            panel.band([(offset, *rec.ci) for offset, _, rec in points], colour, opacity=0.12)
            panel.line([(offset, value) for offset, value, _ in points], colour, width=1.5)
            if points:
                ends.append((points[-1][0], points[-1][1], f"{controller} {label}", colour))
        panel.end_labels(ends)
        panel.ref(x=0, label="onset", colour="#52514e")
        cut = {"disturbance.type": kind, "controller": "A3"}
        for metric, text in (("detect_delay", "breach"), ("settling_time", "settled")):
            rec = inputs.one(metric, experiment=REPLAY, arm=ANY, clauses=cut)
            value = fig.take(rec, panel=letter, role=f"A3 median {text}", series=f"{kind}/A3") if rec else None
            if value is not None:
                panel.ref(x=value, label=text, colour="#0072B2")
        for rec in inputs.where("param_change_count", experiment=REPLAY, arm=ANY, clauses={**cut, "offset": ANY}):
            count = fig.take(rec, panel=letter, role=f"A3 parameter changes at {rec.eq('offset')}",
                             series=f"{kind}/A3")
            if count:
                x = panel.px(rec.eq("offset"))
                bottom = panel.box[1] + panel.box[3]
                panel.marks.append(f'    <path d="M{x:.1f},{bottom:.1f}V{bottom - 7:.1f}" stroke="#0072B2" '
                                   'stroke-width="1.5"/>')
        inside = _live_check(inputs, fig, panel, letter, kind)
        if inside is not None:
            inside_total.append(inside)
            live_total += inside[1]
    if inside_total:
        hits = sum(hit for hit, _ in inside_total)
        fig.derive("all", "live IAEs inside the replay's 90% PI, over the live kinds (validity check: >= 7 of 9)",
                   f"{hits} of {live_total}", [rec for rec in fig.used.values() if rec.metric in ("iae", "iae_pi90")])
    return fig


def _live_check(inputs: figlib.Inputs, fig: figlib.Figure, panel: figlib.Panel, letter: str,
                kind: str) -> tuple[int, int] | None:
    """How many of the kind's live IAEs (A3) fall inside the replay's 90% predictive interval, printed in the panel."""
    cut = {"disturbance.type": kind, "controller": "A3"}
    interval = inputs.one("iae_pi90", experiment=REPLAY, arm=ANY, clauses=cut)
    live = inputs.where("iae", experiment=LIVE, arm=ANY, clauses={**cut, "stage": "live", "replicate": ANY})
    if interval is None or not live:
        return None
    fig.take(interval, panel=letter, role="replay IAE, 90% PI", series=f"{kind}/A3", need_ci=True)
    values = [fig.take(rec, panel=letter, role=f"live IAE, replicate {rec.eq('replicate')}", series=f"{kind}/A3")
              for rec in live]
    inside = sum(value is not None and interval.ci[0] <= value <= interval.ci[1] for value in values)
    text = fig.derive(letter, f"{kind}: live IAEs inside the replay's 90% PI", f"{inside} of {len(values)}",
                      [interval, *live])
    panel.note(f"Live A3 IAEs inside the replay's 90% PI: {text}.")
    return inside, len(values)


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
