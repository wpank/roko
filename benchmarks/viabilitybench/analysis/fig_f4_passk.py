#!/usr/bin/env python3
"""F4: pass^k curves, CC_k and cost dispersion (§6.3; H2). Spec: paper/FIGURES-TABLES.md, F4.

    fig_f4_passk.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads, in the P1-core experiments (`LOG1`, `E-P1-live`), pooled over levels and families:
- panel a: `pass_hat_<k>` for k = 1-3 per arm on P1-core (no `stream.id` clause, or `stream.id == "p1_core"`), as
  lines with CI whiskers;
- panel b: `pass_hat_<k>` for k = 1-5 on the 30-task subset (`stream.id == "p1_pass5"`), drawn apart, so the two task
  sets never share a line;
- panel c: `cc_3`, CC_3 = pass^3 / pass@3, per arm with its CI;
- panel d: per-task cost CV across seeds (`cost_cv` with a `task.instance_id` clause) as a strip per arm on a log x
  axis, with each arm's median (derived) marked. A per-task value is a description of spread, so it has no CI.
figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import statistics
import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F4", "f4-passk", "pass^k curves, CC_k and cost dispersion", "fig_f4_passk.py")
READS = {
    "cc_<k>": "CC_k = pass^k / pass@k per arm (pooled cell), with its CI",
    "cost_cv": "one task's coefficient of variation of costs.api_equiv_usd across its seeds (clause "
               "task.instance_id == <id>), per arm",
}
CORE_STREAM = (None, "p1_core")  # report.py's records carry no stream clause; 3328's stream id is p1_core


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1200, 820)
    boxes = fig.grid(2, 2, left=180, right=150, hgap=200)
    core = figlib.P1_CORE
    for letter, title, ks, stream, box in (("a", "pass^k on P1-core (k = 1–3)", (1, 2, 3), CORE_STREAM, boxes[0]),
                                           ("b", "pass^k on the 30-task subset (k = 1–5)", (1, 2, 3, 4, 5),
                                            figlib.PASS5_STREAM, boxes[1])):
        curves = {}
        for k in ks:
            for rec in inputs.where(f"pass_hat_{k}", experiment=core, clauses={"stream.id": stream}):
                if rec.arm is not None:
                    curves.setdefault((rec.experiment, rec.series), []).append((k, rec))
        if not curves:
            fig.skip(letter, f"no pass_hat_<k> records for stream {stream}")
            continue
        panel = fig.panel(letter, title, box, figlib.category_axis([str(k) for k in ks], "k (independent seeds)"),
                          figlib.rate_axis("pass^k"))
        ends = []
        for (_, name), recs in sorted(curves.items()):
            style = figlib.arm_style(recs[0][1].arm)
            points = [(k, fig.take(rec, panel=letter, role=f"pass^{k}", need_ci=True), rec) for k, rec in recs]
            points = [(k, value, rec) for k, value, rec in points if value is not None]
            panel.line([(k, value) for k, value, _ in points], style.colour, width=1.3)
            for k, value, rec in points:
                panel.whisker_y(k, *rec.ci, style.colour)
                panel.point(k, value, style, tip=f"{name} pass^{k}: {value:.3f}")
            if points:
                ends.append((points[-1][0], points[-1][1], name, style.colour))
        panel.end_labels(ends)

    # c: CC_3 per arm, one row each.
    cc = [rec for rec in inputs.where("cc_3", experiment=core, clauses={"stream.id": CORE_STREAM})
          if rec.arm is not None]
    if cc:
        cc.sort(key=lambda rec: (figlib.arm_order(rec.arm), rec.series))
        panel = fig.panel("c", "CC_3 = pass^3 / pass@3 per arm", boxes[2], figlib.rate_axis("CC_3"),
                          figlib.category_axis([rec.series for rec in reversed(cc)], ""))
        for row, rec in enumerate(reversed(cc), 1):
            style = figlib.arm_style(rec.arm)
            value = fig.take(rec, panel="c", role="CC_3", need_ci=True)
            if value is not None:
                panel.whisker_x(row, *rec.ci, style.colour)
                panel.point(value, row, style, tip=f"{rec.series} CC_3: {value:.3f}")
    else:
        fig.skip("c", "no cc_3 records")

    # d: per-task cost CV across seeds, one strip per arm.
    spread = [rec for rec in inputs.where("cost_cv", experiment=core, clauses={"task.instance_id": ANY})
              if rec.arm is not None]
    names = sorted({rec.series: rec.arm for rec in spread}.items(), key=lambda item: (figlib.arm_order(item[1]),
                                                                                          item[0]))
    names = [name for name, _ in names]
    if spread:
        panel = fig.panel("d", "Per-task cost CV across seeds", boxes[3],
                          figlib.log_axis([rec.value for rec in spread], "coefficient of variation of api_equiv_usd "
                                          "across seeds (log)", money=False),
                          figlib.category_axis(list(reversed(names)), ""))
        for row, name in enumerate(reversed(names), 1):
            recs = sorted((rec for rec in spread if rec.series == name), key=lambda rec: rec.eq("task.instance_id"))
            style = figlib.arm_style(recs[0].arm)
            drawn = []
            for index, rec in enumerate(recs):
                value = fig.take(rec, panel="d", role=f"cost CV, task {rec.eq('task.instance_id')}")
                if value is None:
                    continue
                drawn.append(rec)
                panel.point(value, row + ((index % 7) - 3) * 0.045, style, tip=f"{name} "
                            f"{rec.eq('task.instance_id')}: {value:.3f}")
            if not drawn:
                continue
            median = fig.derive("d", f"median per-task cost CV of {name}",
                                statistics.median(rec.value for rec in drawn), drawn)
            panel.line([(median, row - 0.3), (median, row + 0.3)], "#0b0b0b", width=2)
    else:
        fig.skip("d", "no per-task cost_cv records")
    return fig


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
