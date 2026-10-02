#!/usr/bin/env python3
"""T3: the H1/H2 headline numbers (§6.2, §6.3). Spec: paper/FIGURES-TABLES.md, T3.

    tab_t3_headline.py INPUT [INPUT ...] --out DIR [--dry-run]

One row per arm cell in the P1-core experiments (`LOG1`, `E-P1-live`), in the order roko_full, fd_claude,
cheap_direct, then fr_claude, fd_claude_lite, fd_codex and fd_api where run; every number is the pooled cell's (no
level, no family):
- n: `vs_rate`'s n (runs, tasks x seeds) and its seeds;
- VS rate [CI] (`vs_rate`), the unknown = 1 bound (`vs_rate_unknown_as_1`) and the visible-pass rate beside it
  (`visible_pass_rate`), never instead of it;
- $/VS [CI] (`usd_per_vs`, api_equiv_usd) and, apart, the billed spend (`spend_usd`, billed_usd);
- the cost source: "measured", or how many runs were estimated (`estimated_cost_runs`) or unknown
  (`unknown_cost_runs`), derived;
- the pooled R and C against fd_claude (`envelope_ratio_r`, `envelope_ratio_c` with arms {arm, fd_claude});
- pass^1, pass^3 [CI] (`pass_hat_1`, `pass_hat_3`), pass^5 on the subset (`pass_hat_5`, stream `p1_pass5`), CC_3
  (`cc_3`) and the outcome SD (`outcome_sd`);
- the censoring share: `cap_censored_runs` / its n, derived.
The footer holds E* (`envelope_level`) and the Holm decisions for H1 and H2 (`holm_reject`, clause `hypothesis`).
A missing record prints "–", a null value "n/a". figlib has the refusal rules and the sidecar.
"""

from __future__ import annotations

import sys

import figlib

SPEC = figlib.Spec("T3", "t3-headline", "H1/H2 headline numbers", "tab_t3_headline.py", kind="table")
READS = {
    "visible_pass_rate": "the share of runs whose visible checks passed on the census's clean rerun",
    "envelope_ratio_r": "pooled R = VS rate(arm) / VS rate(fd_claude), arms {arm, fd_claude}, no ladder",
    "envelope_ratio_c": "pooled C, the same ratio of $/VS",
    "envelope_level": "E*, arms {roko_full, fd_claude}",
    "cc_<k>": "CC_k = pass^k / pass@k",
    "outcome_sd": "the SD of the per-seed VS rate across seeds",
    "holm_reject": "1 when Holm's procedure rejects the hypothesis's null, else 0 (clause hypothesis == H#); S09 E9",
}
ORDER = ("roko_full", "fd_claude", "cheap_direct", "fr_claude", "fd_claude_lite", "fd_codex", "fd_api")
CORE_STREAM = (None, "p1_core")
COLUMNS = ("arm", "experiment", "n (runs)", "VS rate [CI]", "VS, unknown = 1", "visible pass", "$/VS [CI]",
           "billed $", "cost source", "R vs fd_claude", "C vs fd_claude", "pass^1", "pass^3 [CI]", "pass^5 (subset)",
           "CC₃", "outcome SD", "censored")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    rows = table.part("", COLUMNS)
    cells = sorted({(rec.arm, rec.series, rec.experiment, rec.model) for rec in inputs.where(
        "vs_rate", experiment=figlib.P1_CORE) if rec.arm in ORDER}, key=lambda cell: (ORDER.index(cell[0]), cell[1:3]))
    for arm, series, experiment, model in cells:
        cut = {"experiment": experiment, "arm": arm, "model": model}

        def cell(metric: str, column: str, kind: str = "rate", with_ci: bool = False, **more: object) -> str:
            return table.cell(inputs.one(metric, **cut, **more), part="T3", row=series, column=column, kind=kind,
                              with_ci=with_ci)

        vs = inputs.one("vs_rate", **cut)
        n = f"{vs.n} ({len(vs.doc['seeds'])} seeds)"
        rows.append([
            series, experiment, n,
            table.cell(vs, part="T3", row=series, column="VS rate"),
            cell("vs_rate_unknown_as_1", "VS, unknown = 1"),
            cell("visible_pass_rate", "visible pass"),
            cell("usd_per_vs", "$/VS", "usd", True, cost_basis="api_equiv_usd"),
            cell("spend_usd", "billed $", "usd", cost_basis="billed_usd"),
            _cost_source(inputs, table, cut, series),
            _ratio(inputs, table, "envelope_ratio_r", experiment, arm, series),
            _ratio(inputs, table, "envelope_ratio_c", experiment, arm, series),
            cell("pass_hat_1", "pass^1", clauses={"stream.id": CORE_STREAM}),
            cell("pass_hat_3", "pass^3", with_ci=True, clauses={"stream.id": CORE_STREAM}),
            cell("pass_hat_5", "pass^5 (subset)", clauses={"stream.id": figlib.PASS5_STREAM}),
            cell("cc_3", "CC₃", clauses={"stream.id": CORE_STREAM}),
            cell("outcome_sd", "outcome SD"),
            _censored(inputs, table, cut, series),
        ])
    if not rows:
        table.skip("T3", "no vs_rate record of a T3 arm in the P1-core experiments")
    star = inputs.one("envelope_level", experiment=figlib.P1_CORE, arm="roko_full", against=(figlib.REFERENCE_ARM,))
    value = table.take(star, panel="footer", role="E*") if star is not None else None
    table.footer.append(f"E* = ℓ{value:g} (the last level claimed)" if value else "E* = 0: no level claimed"
                        if value == 0 else "E*: no envelope_level record in the inputs.")
    for hypothesis in ("H1", "H2"):
        rec = inputs.one("holm_reject", experiment=figlib.P1_CORE, arm=figlib.ANY, clauses={"hypothesis": hypothesis})
        decision = table.take(rec, panel="footer", role=f"Holm decision {hypothesis}") if rec is not None else None
        table.footer.append(f"{hypothesis}: " + ("no holm_reject record in the inputs (T6 holds the decisions)."
                                                 if decision is None else "Holm rejects the null." if decision == 1
                                                 else "Holm does not reject the null."))
    table.footer.append("$/VS uses api_equiv_usd (U′); billed_usd is reported apart and never enters $/VS.")
    return table


def _cost_source(inputs: figlib.Inputs, table: figlib.Table, cut: dict, series: str) -> str:
    estimated, unknown = (inputs.one(metric, **cut) for metric in ("estimated_cost_runs", "unknown_cost_runs"))
    if estimated is None or unknown is None:
        return "–"
    counts = [table.take(rec, panel="T3", series=series, role=rec.metric) for rec in (estimated, unknown)]
    if None in counts:
        return "n/a"
    text = "measured" if counts == [0, 0] else f"{counts[0]:g} estimated, {counts[1]:g} unknown"
    return str(table.derive("T3", f"cost source of {series}", text, [estimated, unknown]))


def _ratio(inputs: figlib.Inputs, table: figlib.Table, metric: str, experiment: str, arm: str, series: str) -> str:
    if arm == figlib.REFERENCE_ARM:
        return "1 (reference)"
    rec = inputs.one(metric, experiment=experiment, arm=arm, against=(figlib.REFERENCE_ARM,))
    return table.cell(rec, part="T3", row=series, column=metric, kind="rate")


def _censored(inputs: figlib.Inputs, table: figlib.Table, cut: dict, series: str) -> str:
    rec = inputs.one("cap_censored_runs", **cut)
    if rec is None:
        return "–"
    count = table.take(rec, panel="T3", series=series, role="cap-censored runs")
    if count is None or not rec.n:
        return "n/a"
    share = table.derive("T3", f"censoring share of {series}: cap_censored_runs / n", count / rec.n, [rec])
    return f"{figlib.fmt(share, 'pct')} ({count:g} of {rec.n})"


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
