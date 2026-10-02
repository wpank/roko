#!/usr/bin/env python3
"""F6: reliability diagram by label regime, and rolling ECE (§7.4 X1; used by §6.5 H4). Spec: FIGURES-TABLES.md, F6.

    fig_f6_reliability.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads the R-M3 replay's MetricRecords (`experiment_id == "R-M3"`), one set per label regime (clause `label_regime`:
G gate-trained, A audit-corrected at rho = 0.15 (clause `rho`), C the census reference):
- panel a: per equal-mass bin (clause `bin`), `calibration_bin_mean` (x, the mean predicted P(VS)) and
  `calibration_bin_freq` (y, the observed VS frequency, with its Wilson 95% CI, and its n as the dot's area), beside
  the diagonal; an inset lists `brier`, `bss`, `ece` [CI] and `auroc` per regime;
- panel b: `ece_rolling` (W = 100) per regime against stream position (clause `stream.position`), with its bootstrap
  band and S04's bound 0.08 as a reference line.
The records are S04's `self_model.forecast` / `self_model.outcome` (or `roko.prediction/1` joined to `roko.verdict/1`
and the census label) as the replay scores them. figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import math
import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F6", "f6-reliability", "Reliability diagram by label regime; rolling ECE", "fig_f6_reliability.py",
                   p1=False)
READS = {
    "calibration_bin_mean": "the mean predicted P(VS) in equal-mass bin b of a regime (clauses label_regime, bin)",
    "calibration_bin_freq": "the observed VS frequency in that bin, with its Wilson 95% CI; n is the bin's count",
    "brier": "Brier score per regime", "bss": "Brier skill score against the base rate, per regime",
    "ece": "expected calibration error per regime, with its CI", "auroc": "AUROC per regime",
    "ece_rolling": "ECE over the last 100 outcomes at stream position p (clauses label_regime, stream.position)",
}
EXPERIMENT = "R-M3"
REGIMES = {"G": ("gate-trained", figlib.Style("#E69F00")), "A": ("audit-corrected, ρ = 0.15", figlib.Style("#0072B2")),
           "C": ("census reference", figlib.Style("#000000", filled=True))}
A_RHO = 0.15
ECE_BOUND = 0.08  # S04 §4.7's bound on rolling ECE


def regime_clauses(regime: str, more: dict | None = None) -> dict:
    """The clauses of one regime's cut: A is read at rho = 0.15, G and C have no rho."""
    return {"label_regime": regime, "rho": A_RHO if regime == "A" else None, **(more or {})}


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1000, 500)
    boxes = fig.grid(1, 2)
    panel = None
    inset = []
    for regime, (name, style) in REGIMES.items():
        bins = {rec.eq("bin"): rec for rec in inputs.where("calibration_bin_freq", experiment=EXPERIMENT, arm=ANY,
                                                            clauses=regime_clauses(regime, {"bin": ANY}))}
        if not bins:
            fig.skip("a", f"no calibration bins for regime {regime}")
            continue
        if panel is None:
            panel = fig.panel("a", "Reliability by label regime", boxes[0], figlib.rate_axis("predicted P(VS)"),
                              figlib.rate_axis("observed VS frequency"))
            panel.line([(0, 0), (1, 1)], "#bdbdbd", width=1, dash="4 3")
        largest = max(rec.n for rec in bins.values()) or 1
        points = []
        for number, freq in sorted(bins.items()):
            mean = inputs.one("calibration_bin_mean", experiment=EXPERIMENT, arm=ANY,
                              clauses=regime_clauses(regime, {"bin": number}))
            if mean is None:
                fig.skip("a", f"regime {regime} bin {number} has no calibration_bin_mean")
                continue
            x = fig.take(mean, panel="a", role=f"bin {number} mean prediction", series=regime)
            y = fig.take(freq, panel="a", role=f"bin {number} observed frequency", series=regime, need_ci=True)
            if x is None or y is None:
                continue
            points.append((x, y))
            panel.whisker_y(x, *freq.ci, style.colour)
            panel.point(x, y, style, scale=0.6 + 0.9 * math.sqrt(freq.n / largest), tip=f"{regime} bin {number}: "
                        f"predicted {x:.2f}, observed {y:.2f}, n {freq.n}")
        panel.line(points, style.colour, width=1.1, opacity=0.7)
        scores = []
        for metric in ("brier", "bss", "ece", "auroc"):
            rec = inputs.one(metric, experiment=EXPERIMENT, arm=ANY, clauses=regime_clauses(regime))
            value = fig.take(rec, panel="a", role=f"{metric} (inset)", series=regime) if rec else None
            if value is not None:
                scores.append(f"{metric.upper()} {value:.3f}" + (f" [{rec.ci[0]:.3f}, {rec.ci[1]:.3f}]"
                                                                 if metric == "ece" and rec.ci else ""))
        inset.append((f"{regime} ({name}): " + (", ".join(scores) or "no scores"), style.colour))
    if panel is not None:
        for row, (text, colour) in enumerate(inset):
            panel.text(0.02, 0.97 - 0.05 * row, text, colour=figlib.ink(colour), size=9.5)
        panel.note("Dot area ∝ the bin's n; whiskers are Wilson 95% intervals; the dashed line is perfect "
                   "calibration.")

    curves = {regime: inputs.where("ece_rolling", experiment=EXPERIMENT, arm=ANY,
                                   clauses=regime_clauses(regime, {"stream.position": ANY})) for regime in REGIMES}
    curves = {regime: recs for regime, recs in curves.items() if recs}
    if not curves:
        fig.skip("b", "no ece_rolling records")
        return fig
    positions = [rec.eq("stream.position") for recs in curves.values() for rec in recs]
    tops = [bound for recs in curves.values() for rec in recs for bound in rec.ci or (rec.value or 0,)]
    panel = fig.panel("b", "Rolling ECE over the stream (W = 100)", boxes[1],
                      figlib.linear_axis(positions, "stream position (outcomes)"),
                      figlib.linear_axis(tops + [ECE_BOUND * 1.5], "ECE (rolling)"))
    fig.design("b", "S04's bound on rolling ECE", ECE_BOUND, "S04 §4.7")
    panel.ref(y=ECE_BOUND, label=f"S04 bound {ECE_BOUND}")
    ends = []
    for regime, recs in sorted(curves.items()):
        style = REGIMES[regime][1]
        recs.sort(key=lambda rec: rec.eq("stream.position"))
        points = [(rec.eq("stream.position"), fig.take(rec, panel="b", role=f"ECE at {rec.eq('stream.position')}",
                                                       series=regime, need_ci=True), rec) for rec in recs]
        points = [(position, value, rec) for position, value, rec in points if value is not None]
        panel.band([(position, *rec.ci) for position, _, rec in points], style.colour, opacity=0.14)
        panel.line([(position, value) for position, value, _ in points], style.colour, width=1.4)
        if points:
            ends.append((points[-1][0], points[-1][1], regime, style.colour))
    panel.end_labels(ends)
    return fig


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
