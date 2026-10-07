#!/usr/bin/env python3
"""F10: loop-audit detection: TTD per fault, and the beta confidence sequences (§7.3; H7). Spec: FIGURES-TABLES, F10.

    fig_f10_loop_detection.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads the MetricRecords that the R-H7 fault replay (E1, `experiment_id == "R-H7"`) and the live E3 streams
(`E-H7-live`) compute from S03's `loop-audit/1` rows and the ground truth in `faults.jsonl`:
- panel a: one strip per fault kind (clause `fault.kind`: CUT, MASK, UNLOGGED, DEGENERATE, LABEL_ONLY, STALE), one
  dot per injection (`ttd` with clause `fault.id`: opportunities from onset to detection, log x). A null `ttd` is an
  undetected injection, drawn in the "not detected" column at the right. Each kind's median (derived) is marked, its
  lock threshold (30, 50 or 1 opportunities; S09's prereg lock) is a tick, and its `detection_rate` with its CI is
  printed beside the row;
- panel b: `beta_cs` (the E3 benefit estimate beta with its confidence sequence) against opportunities (clause
  `n_opp`) for the harmful, oracle and placebo loops (clause `loop_id`), with a zero line; `loop_transition` records
  (value 1 a promotion, -1 a demotion) at their opportunity are marked.
figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import statistics
import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F10", "f10-loop-detection", "Loop-audit detection: TTD per fault; β confidence sequences",
                   "fig_f10_loop_detection.py", p1=False)
READS = {
    "ttd": "opportunities from fault onset to detection for one injection (clauses fault.kind, fault.id); null when "
           "the injection was not detected",
    "detection_rate": "detected injections / injections per fault kind (clause fault.kind), with its CI",
    "beta_cs": "E3's beta estimate after n opportunities (clauses loop_id, n_opp), ci its confidence sequence",
    "loop_transition": "a state transition at n opportunities (clauses loop_id, n_opp): 1 promotion, -1 demotion",
}
REPLAY, LIVE = "R-H7", "E-H7-live"
# S09's prereg lock, e1_faults.ttd_max: the opportunities within which each kind must be detected.
KINDS = {"CUT": 30, "MASK": 30, "UNLOGGED": 30, "DEGENERATE": 50, "LABEL_ONLY": 1, "STALE": 30}
LOOPS = {"harmful": "#D55E00", "oracle": "#0072B2", "placebo": figlib.GREY}


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1100, 480)
    boxes = fig.grid(1, 2, left=110, right=150, hgap=120, bottom=70)
    injections = inputs.where("ttd", experiment=REPLAY, arm=ANY, clauses={"fault.kind": ANY, "fault.id": ANY})
    kinds = [kind for kind in KINDS if any(rec.eq("fault.kind") == kind for rec in injections)]
    if kinds:
        detected = [rec.value for rec in injections if rec.value is not None]
        x_axis = figlib.log_axis(detected + [1, max(KINDS.values())], "opportunities to detection (log)",
                                 money=False)
        panel = fig.panel("a", "Time to detection per injected fault (E1)", boxes[0], x_axis,
                          figlib.category_axis(list(reversed(kinds)), ""))
        for row, kind in enumerate(reversed(kinds), 1):
            recs = sorted((rec for rec in injections if rec.eq("fault.kind") == kind),
                          key=lambda rec: rec.eq("fault.id"))
            found, missed = [], []
            for index, rec in enumerate(recs):
                value = fig.take(rec, panel="a", role=f"TTD, {rec.eq('fault.id')}", series=kind, allow_null=True)
                jitter = ((index % 5) - 2) * 0.07
                if value is None:
                    missed.append(rec)
                    panel.text(x_axis.hi, row + jitter, "×", colour="#d03b3b", anchor="middle", dy=4, size=12)
                else:
                    found.append(rec)
                    panel.point(value, row + jitter, figlib.Style("#0072B2"), tip=f"{kind} {rec.eq('fault.id')}: "
                                f"{value:g}")
            if found:
                median = fig.derive("a", f"median TTD of the detected {kind} injections",
                                    statistics.median(rec.value for rec in found), found)
                panel.line([(median, row - 0.32), (median, row + 0.32)], "#0b0b0b", width=2)
            if missed:
                fig.derive("a", f"{kind}: injections not detected", len(missed), missed)
            limit = fig.design("a", f"{kind}: the lock's TTD limit", KINDS[kind], "S09 §5 prereg lock, e1_faults")
            panel.line([(limit, row - 0.4), (limit, row + 0.4)], "#d03b3b", width=1, dash="3 2")
            rate = inputs.one("detection_rate", experiment=REPLAY, arm=ANY, clauses={"fault.kind": kind})
            share = fig.take(rate, panel="a", role="detection share", series=kind, need_ci=True) if rate else None
            if share is not None:
                panel.text_px(panel.box[0] + panel.box[2] + 14, panel.py(row) + 3.5,
                              f"{share:.2f} [{rate.ci[0]:.2f}, {rate.ci[1]:.2f}]", size=9.5)
        panel.note("× at the right edge: not detected. Black bar: median; red dashed tick: the lock's limit. Right: "
                   "detection share [CI].")
    else:
        fig.skip("a", "no ttd records")

    sequences = {}
    for rec in inputs.where("beta_cs", experiment=LIVE, arm=ANY, clauses={"loop_id": ANY, "n_opp": ANY}):
        sequences.setdefault(rec.eq("loop_id"), []).append(rec)
    if not sequences:
        fig.skip("b", "no beta_cs records")
        return fig
    opportunities = [rec.eq("n_opp") for recs in sequences.values() for rec in recs]
    bounds = [bound for recs in sequences.values() for rec in recs for bound in rec.ci or (rec.value or 0,)]
    panel = fig.panel("b", "β and its confidence sequence (E3)", boxes[1],
                      figlib.linear_axis(opportunities, "opportunities"), figlib.symmetric_axis(bounds, "β (benefit)"))
    panel.ref(y=0.0, label="0")
    ends = []
    for loop, recs in sorted(sequences.items()):
        colour = next((colour for name, colour in LOOPS.items() if name in loop), "#000000")
        recs.sort(key=lambda rec: rec.eq("n_opp"))
        points = [(rec.eq("n_opp"), fig.take(rec, panel="b", role=f"beta at {rec.eq('n_opp')}", series=loop,
                                             need_ci=True), rec) for rec in recs]
        points = [(n, value, rec) for n, value, rec in points if value is not None]
        panel.band([(n, *rec.ci) for n, _, rec in points], colour, opacity=0.14)
        panel.line([(n, value) for n, value, _ in points], colour, width=1.5)
        if points:
            ends.append((points[-1][0], points[-1][1], loop, colour))
        for rec in inputs.where("loop_transition", experiment=LIVE, arm=ANY, clauses={"loop_id": loop, "n_opp": ANY}):
            change = fig.take(rec, panel="b", role=f"transition at {rec.eq('n_opp')}", series=loop)
            if change:
                level = next((value for n, value, _ in points if n >= rec.eq("n_opp")), 0.0)
                panel.text(rec.eq("n_opp"), level, "▲" if change > 0 else "▼", colour=colour, anchor="middle",
                           dy=-8 if change > 0 else 14, size=11)
    panel.end_labels(ends)
    panel.note("▲ promotion, ▼ demotion; bands are the confidence sequences.")
    return fig


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
