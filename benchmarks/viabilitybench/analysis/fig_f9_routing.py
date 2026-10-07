#!/usr/bin/env python3
"""F9: routing policies: cost–VS, regret and escalation flow (§6.5; H4). Spec: paper/FIGURES-TABLES.md, F9.

    fig_f9_routing.py INPUT [INPUT ...] --out DIR [--dry-run]

Reads the R-H4 replay's MetricRecords (`experiment_id == "R-H4"`), whose `arms` hold one S04 policy id: `H4-B0:cheapest`
and `H4-B0:best` (the static baselines), `H4-B1` (the uncalibrated cascade), `H4-B2` (the current CascadeRouter),
`H4-B3` (the oracle), `lcb_aci@<pi*>` (policy (a), one per point of its pi* sweep), `cascade` (policy (b)) and
`hybrid`. Policies are not arms, so they have their own fixed colours (`POLICIES`).
- panel a: `vs_rate` (y) and `usd_per_vs` (x, log) per policy with CI whiskers; the pi* sweep of (a) as a connected
  line; the SC2 target region (VS >= best static - 2 pp and $/VS <= 0.85 x best static, from `H4-B0:best`) shaded;
  the live `roko_full` (experiment `E-P1-live`) as a filled diamond with its own CI. When the live record's filter
  names its policy (`policy == <id>`), that policy's replay interval is drawn behind it as a translucent bar.
- panel b: `regret_cum` per policy against stream position (clause `stream.position == p`): the median line and
  its band (`ci`, the 90% band over orderings, as `ci_method` says).
- panel c: the escalation flow per policy: `escalation_share` per rung (clause `rung == r`) stacked with
  `unresolved_share` (hatched).
figlib has the refusal rules, the encoding and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("F9", "f9-routing", "Routing policies: cost–VS, regret and escalation flow", "fig_f9_routing.py")
READS = {
    "vs_rate, usd_per_vs (R-H4)": "report.py's names, per policy: experiment R-H4, arms [<S04 policy id>] (econ.py, "
                                  "task 6123)",
    "regret_cum": "cumulative regret against the oracle after p tasks (clause stream.position == p); ci is the 90% "
                  "band over the 50 orderings",
    "escalation_share": "the share of tasks a policy resolved at rung r (clause rung == r)",
    "unresolved_share": "the share of tasks a policy left unresolved",
}
EXPERIMENT = "R-H4"
LIVE = "E-P1-live"
SWEEP = "lcb_aci"
BEST_STATIC = "H4-B0:best"
# Policy -> (style, label). Policies (a) and (b) route inside Roko, so they are diamonds in roko_full's and
# roko_fixed's blues; the baselines are grey circles and the oracle a black one. All are filled.
POLICIES = {
    "H4-B0": (figlib.Style(figlib.GREY, "circle", True), "static"),
    "H4-B1": (figlib.Style(figlib.GREY, "circle", True), "uncalibrated cascade"),
    "H4-B2": (figlib.Style(figlib.GREY, "circle", True), "CascadeRouter"),
    "H4-B3": (figlib.Style("#000000", "circle", True), "oracle"),
    SWEEP: (figlib.Style("#0072B2", "diamond", True), "(a)"),
    "cascade": (figlib.Style("#56B4E9", "diamond", True), "(b)"),
    "hybrid": (figlib.Style(figlib.GREY, "circle", True), "hybrid"),
}
RUNGS = ("#c6dbef", "#6baed6", "#2171b5", "#08306b")


def policy_style(policy: str) -> tuple[figlib.Style, str]:
    base = policy.split("@")[0].split(":")[0]
    if base not in POLICIES:
        raise figlib.FigureError(f"policy {policy!r} has no fixed colour in fig_f9_routing.POLICIES; add it there")
    return POLICIES[base]


def build(inputs: figlib.Inputs) -> figlib.Figure:
    fig = figlib.Figure(SPEC, inputs.dry_run, 1120, 840)
    boxes = fig.grid(2, 2, left=110, right=170, hgap=170, bottom=92)
    _pareto(inputs, fig, boxes[0])
    _regret(inputs, fig, boxes[1])
    _flow(inputs, fig, boxes[2])
    return fig


def _pareto(inputs: figlib.Inputs, fig: figlib.Figure, box: tuple) -> None:
    pairs = []
    for vs in inputs.where("vs_rate", experiment=EXPERIMENT):
        cost = inputs.one("usd_per_vs", experiment=EXPERIMENT, arm=vs.arms[0], cost_basis="api_equiv_usd")
        if len(vs.arms) == 1 and cost is not None and None not in (vs.value, cost.value):
            pairs.append((vs.arms[0], vs, cost))
        else:
            fig.skip("a", f"policy {'/'.join(vs.arms)}: no usd_per_vs record, or a null value")
    live = [(vs, inputs.one("usd_per_vs", experiment=LIVE, arm="roko_full", model=vs.model,
                            cost_basis="api_equiv_usd")) for vs in inputs.where("vs_rate", experiment=LIVE,
                                                                                arm="roko_full")]
    live = [(vs, cost) for vs, cost in live if cost is not None and None not in (vs.value, cost.value)]
    if not pairs:
        fig.skip("a", f"no vs_rate and usd_per_vs pair in {EXPERIMENT}")
        return
    bounds = [bound for _, _, cost in pairs for bound in cost.ci or (cost.value,)]
    bounds += [bound for _, cost in live for bound in cost.ci or (cost.value,)]
    panel = fig.panel("a", "Policies on the cost–VS plane (replay)", box,
                      figlib.log_axis(bounds, "$/VS, API-equivalent USD (log)"), figlib.rate_axis("VS rate"))
    taken = {}
    for policy, vs, cost in sorted(pairs, key=lambda item: item[0]):
        y = fig.take(vs, panel="a", role="y: VS rate", series=policy, need_ci=True)
        x = fig.take(cost, panel="a", role="x: $/VS", series=policy, need_ci=True)
        taken[policy] = (x, y, vs, cost)
    best = taken.get(BEST_STATIC)
    if best is not None:
        floor = fig.derive("a", "SC2: VS rate of the best static policy - 0.02", best[1] - 0.02, [best[2]])
        ceiling = fig.derive("a", "SC2: 0.85 x $/VS of the best static policy", 0.85 * best[0], [best[3]])
        fig.design("a", "SC2's margins: 2 pp on VS, 0.85 on $/VS", [0.02, 0.85], "S04 SC2; S09 §4.4 (H4)")
        panel.shade(panel.x.lo, ceiling, floor, 1.0, "#009E73", opacity=0.08)
        panel.text(panel.x.lo, 0.97, "SC2 target region", colour="#009E73", dx=4, size=9.5)
    else:
        fig.skip("a", f"no {BEST_STATIC} policy, so the SC2 target region is not drawn")
    sweep = sorted(((float(policy.split("@")[1]), policy) for policy in taken if policy.startswith(f"{SWEEP}@")))
    panel.line([taken[policy][:2] for _, policy in sweep], POLICIES[SWEEP][0].colour, width=1.2)
    for policy, (x, y, vs, cost) in taken.items():
        style, label = policy_style(policy)
        panel.whisker_x(y, *cost.ci, style.colour)
        panel.whisker_y(x, *vs.ci, style.colour)
        text = f"(a) π* = {policy.split('@')[1]}" if policy.startswith(f"{SWEEP}@") else (
            f"{policy} {label}" if label.startswith("(") else policy)
        panel.point(x, y, style, label=text if not policy.startswith(f"{SWEEP}@") or policy == sweep[-1][1]
                    or policy == sweep[0][1] else None, tip=f"{policy}: VS {y:.3f}, $/VS {x:.4f}")
    for vs, cost in live:
        y = fig.take(vs, panel="a", role="y: VS rate (live)", series="roko_full (live)", need_ci=True)
        x = fig.take(cost, panel="a", role="x: $/VS (live)", series="roko_full (live)", need_ci=True)
        policy = vs.eq("policy")
        if policy in taken:
            low, high = taken[policy][2].ci
            panel.shade(x / 1.04, x * 1.04, low, high, figlib.ARMS["roko_full"].colour, opacity=0.18)
            panel.note(f"The bar behind roko_full (live) is the replay interval of the policy it ran, {policy}.")
        panel.whisker_x(y, *cost.ci, figlib.ARMS["roko_full"].colour)
        panel.whisker_y(x, *vs.ci, figlib.ARMS["roko_full"].colour)
        panel.point(x, y, figlib.ARMS["roko_full"], filled=True, label="roko_full (live)",
                    tip=f"roko_full live: VS {y:.3f}, $/VS {x:.4f}")


def _regret(inputs: figlib.Inputs, fig: figlib.Figure, box: tuple) -> None:
    curves = {}
    for rec in inputs.where("regret_cum", experiment=EXPERIMENT, clauses={"stream.position": ANY}):
        if len(rec.arms) == 1:
            curves.setdefault(rec.arms[0], []).append(rec)
    if not curves:
        fig.skip("b", "no regret_cum records")
        return
    positions = [rec.eq("stream.position") for recs in curves.values() for rec in recs]
    tops = [bound for recs in curves.values() for rec in recs for bound in rec.ci or (rec.value or 0,)]
    panel = fig.panel("b", "Cumulative regret against the oracle", box,
                      figlib.linear_axis(positions, "stream position (tasks resolved)"),
                      figlib.linear_axis(tops, "cumulative regret (median, 90% band)"))
    ends = []
    for policy, recs in sorted(curves.items()):
        style, label = policy_style(policy)
        colour = style.colour
        recs.sort(key=lambda rec: rec.eq("stream.position"))
        points = [(rec.eq("stream.position"), fig.take(rec, panel="b", role=f"regret at position "
                                                       f"{rec.eq('stream.position')}", series=policy, need_ci=True),
                   rec) for rec in recs]
        points = [(position, value, rec) for position, value, rec in points if value is not None]
        panel.band([(position, *rec.ci) for position, _, rec in points], colour, opacity=0.12)
        panel.line([(position, value) for position, value, _ in points], colour, width=1.4)
        if points:
            ends.append((points[-1][0], points[-1][1], policy, colour))
    panel.end_labels(ends)


def _flow(inputs: figlib.Inputs, fig: figlib.Figure, box: tuple) -> None:
    flows = {}
    for rec in inputs.where("escalation_share", experiment=EXPERIMENT, clauses={"rung": ANY}):
        if len(rec.arms) == 1:
            flows.setdefault(rec.arms[0], {})[rec.eq("rung")] = rec
    if not flows:
        fig.skip("c", "no escalation_share records")
        return
    policies = sorted(flows)
    panel = fig.panel("c", "Escalation flow per policy", box, figlib.rate_axis("share of tasks"),
                      figlib.category_axis(list(reversed(policies)), ""))
    for row, policy in enumerate(reversed(policies), 1):
        start, parts = 0.0, []
        for rung, rec in sorted(flows[policy].items()):
            share = fig.take(rec, panel="c", role=f"resolved at rung {rung}", series=policy)
            if share is None:
                continue
            panel.hbar(row, start, start + share, RUNGS[min(rung, len(RUNGS)) - 1], label=f"r{rung}")
            start, parts = start + share, parts + [rec]
        unresolved = inputs.one("unresolved_share", experiment=EXPERIMENT, arm=policy)
        if unresolved is not None and fig.take(unresolved, panel="c", role="unresolved", series=policy) is not None:
            panel.hbar(row, start, start + unresolved.value, "#52514e", hatch=True, label="unresolved")
            start, parts = start + unresolved.value, parts + [unresolved]
        total = fig.derive("c", f"{policy}: the shares' sum", round(start, 9), parts)
        if abs(total - 1) > 0.01:
            fig.skip("c", f"{policy}: the shares sum to {total:.3f}, not 1")
    panel.note("Rungs r1–r4: resolved at that rung of the ladder; hatched: unresolved.")
    panel.note("Policies: H4-B0 static (cheapest, best single), H4-B1 uncalibrated cascade, H4-B2 CascadeRouter, H4-B3 "
               "oracle, lcb_aci@π* policy (a), cascade policy (b).")


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
