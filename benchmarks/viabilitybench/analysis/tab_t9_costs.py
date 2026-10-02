#!/usr/bin/env python3
"""T9: the cost-accounting reconciliation (§6.2; appendix E; H1 robustness). Spec: paper/FIGURES-TABLES.md, T9.

    tab_t9_costs.py INPUT [INPUT ...] --out DIR [--dry-run]

One row per arm cell (arm, or arm and model: the provider/model rows) in the P1 experiments (`LOG1`, `E-P1-live`,
`E-P1-ext`), pooled over levels and families. Columns:
- the cost-source mix: `cost_source_runs` per source (clause `costs.source` in provider_usage, cli_usage, estimated,
  mock, unknown; S01 §4.4);
- |ΣU′ − ΣR|/ΣR (`cost_gap_ur`, report.py) and the per-run |U′ − R|/R median and p90 (`cost_gap_ur_median`,
  `cost_gap_ur_p90`);
- the ledger against the provider export (`ledger_export_gap`, from `vb ledger reconcile`);
- the metering proxy against S01 (`meter_gap`, from `meter_cross_check_usd`);
- the cache-write TTL assumption's 5-minute sensitivity (`cache_ttl_5m_delta`);
- the share of attempts the proxy marks `usage_source = missing` (`usage_missing_share`, S08 §4.11);
- the `cost_source = unknown` count (`unknown_cost_runs`, which must be 0 in H1's cells);
- the billed and API-equivalent totals (`spend_usd` with each `cost_basis`).
The footer names any H1 cell (roko_full, fd_claude) with unknown costs, derived. figlib has the refusal rules and
the sidecar.
"""

from __future__ import annotations

import sys

import figlib

SPEC = figlib.Spec("T9", "t9-costs", "Cost-accounting reconciliation", "tab_t9_costs.py", kind="table")
READS = {
    "cost_source_runs": "runs per cost source (clause costs.source == <source>)",
    "cost_gap_ur_median": "the median over runs that carry a vendor figure of |U′ − R| / R",
    "cost_gap_ur_p90": "the 90th percentile of the same",
    "ledger_export_gap": "|ledger − provider export| / provider export over the cell's days (vb ledger reconcile)",
    "meter_gap": "|proxy meter − S01 api_equiv_usd| / S01 api_equiv_usd (meter_cross_check_usd)",
    "cache_ttl_5m_delta": "the relative change of api_equiv_usd when cache writes are priced at the 5-minute TTL",
    "usage_missing_share": "the share of attempts the metering proxy marks usage_source = missing",
}
SOURCES = ("provider_usage", "cli_usage", "estimated", "mock", "unknown")
H1_ARMS = ("roko_full", figlib.REFERENCE_ARM)
COLUMNS = ("arm (model)", "experiment", "cost-source mix (runs)", "|ΣU′ − ΣR|/ΣR", "|U′ − R|/R median",
           "|U′ − R|/R p90", "ledger vs export", "proxy vs S01", "cache TTL 5-min Δ", "usage missing",
           "unknown-cost runs", "billed $", "API-equivalent $")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    rows = table.part("", COLUMNS)
    experiments = figlib.P1_CORE + figlib.P1_EXT
    cells = sorted({(rec.arm, rec.series, rec.experiment, rec.model) for metric in ("spend_usd", "cost_source_runs")
                    for rec in inputs.where(metric, experiment=experiments, clauses={"costs.source": figlib.ANY}
                                            if metric == "cost_source_runs" else None) if rec.arm is not None},
                   key=lambda cell: (figlib.arm_order(cell[0]), cell[1], cell[2]))
    unknown_in_h1 = []
    for arm, series, experiment, model in cells:
        cut = {"experiment": experiment, "arm": arm, "model": model}

        def cell(metric: str, column: str, kind: str = "pct", **more: object) -> str:
            return table.cell(inputs.one(metric, **cut, **more), part="T9", row=series, column=column, kind=kind,
                              with_ci=False)

        mix = []
        for source in SOURCES:
            rec = inputs.one("cost_source_runs", **cut, clauses={"costs.source": source})
            if rec is not None:
                count = table.cell(rec, part="T9", row=series, column=f"runs, {source}", kind="count",
                                   with_ci=False)
                mix.append(f"{source} {count}")
        unknown = inputs.one("unknown_cost_runs", **cut)
        unknown_text = table.cell(unknown, part="T9", row=series, column="unknown-cost runs", kind="count",
                                  with_ci=False)
        if arm in H1_ARMS and unknown is not None and unknown.value:
            unknown_in_h1.append(table.derive("T9", f"{series} ({experiment}) is an H1 cell with unknown costs",
                                              f"{series} ({experiment}): {unknown.value:g}", [unknown]))
        rows.append([series, experiment, ", ".join(mix) or "–",
                     cell("cost_gap_ur", "|ΣU′ − ΣR|/ΣR"),
                     cell("cost_gap_ur_median", "|U′ − R|/R median"),
                     cell("cost_gap_ur_p90", "|U′ − R|/R p90"),
                     cell("ledger_export_gap", "ledger vs export"),
                     cell("meter_gap", "proxy vs S01"),
                     cell("cache_ttl_5m_delta", "cache TTL 5-min Δ"),
                     cell("usage_missing_share", "usage missing"),
                     unknown_text,
                     cell("spend_usd", "billed $", "usd", cost_basis="billed_usd"),
                     cell("spend_usd", "API-equivalent $", "usd", cost_basis="api_equiv_usd")])
    if not rows:
        table.skip("T9", "no cost record of an arm in the P1 experiments")
    checked = [cell for cell in cells if cell[0] in H1_ARMS
               and inputs.one("unknown_cost_runs", experiment=cell[2], arm=cell[0], model=cell[3]) is not None]
    table.footer.append("cost_source = unknown must be 0 in H1's cells (roko_full, fd_claude); "
                        + (f"it is not in: {'; '.join(unknown_in_h1)}." if unknown_in_h1 else
                           f"it is 0 in all {len(checked)} H1 cells with an unknown_cost_runs record." if checked
                           else "no H1 cell has an unknown_cost_runs record in the inputs."))
    table.footer.append("Ledger and provider exports are reconciled by `vb ledger reconcile`; a switch of the headline "
                        "from U′ to R is recorded as a `vb.deviation/1` row.")
    return table


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
