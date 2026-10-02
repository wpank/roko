#!/usr/bin/env python3
"""T5: the contribution ladder (§8; ablations). Spec: paper/FIGURES-TABLES.md, T5.

    tab_t5_contributions.py INPUT [INPUT ...] --out DIR [--dry-run]

Ten fixed rows (`ROWS`, from the spec), each a contrast read from its source experiment's MetricRecords at the clause
`contrast == "<row number>"`: `delta_vs` (the VS difference with its CI), `usd_per_vs_ratio` (the $/VS ratio with its
CI) and `mechanism_spend_share` (the mechanism's own spend: audit, refinement or predictor calls); n is
`delta_vs`'s. A row with no record prints "not run", as when BL11's cap binds. The spec's caveat is printed with the
table: rows 1-7 assemble contrasts from different designs, task sets and seeds, so they are neither additive nor a
decomposition, and the leave-one-out rows 8-10 share tasks and seed but do not sum to row 7 either. figlib has the
refusal rules and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("T5", "t5-contributions", "Contribution ladder (ablations)", "tab_t5_contributions.py",
                   kind="table", p1=False)
READS = {
    "delta_vs": "the contrast's VS difference with its CI (clause contrast == <row>)",
    "usd_per_vs_ratio": "the contrast's $/VS ratio with its CI (clause contrast)",
    "mechanism_spend_share": "the mechanism's own spend over the arm's spend (clause contrast)",
}
# (contrast, design and data), FIGURES-TABLES T5.
ROWS = (
    ("Harness core (gates, ≤ 2 retries with feedback, loops frozen)", "roko_fixed − cheap_direct on gpt-oss-120b, "
     "P1-core (LOG1 blocks A and B)"),
    ("Routing with escalation (M3)", "policy (a) or (b) vs the best static cheap arm (H4 replay)"),
    ("Spec gate and refiner (S07)", "ρ_R and $/VS of R vs V, cheap tier (H3, block C)"),
    ("Audit feedback (M4)", "θ_true and $/VS, H5-A3 vs H5-A1 and H5-A0 (H5 live)"),
    ("Regulation (M1)", "IAE and e₂ ($/VS) under budget_cut (H6)"),
    ("Loop auditing (M2)", "the VS forgone on the h = 0.1 holdout chains (a cost of regulation)"),
    ("Full stack", "roko_full − roko_fixed (best cheap model), P1-core"),
    ("Leave-one-out, − S04 routing", "roko_full vs roko_full without S04 routing (60 P1-core tasks × seed 1)"),
    ("Leave-one-out, − S07 gate", "roko_full vs roko_full without the spec gate (same tasks and seed)"),
    ("Leave-one-out, − S05 feedback", "roko_full vs roko_full without audit feedback (same tasks and seed)"),
)
COLUMNS = ("#", "contrast", "design and data", "n", "ΔVS [CI]", "$/VS ratio [CI]", "mechanism's spend share",
           "status")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    rows = table.part("", COLUMNS)
    for number, (contrast, design) in enumerate(ROWS, 1):
        cut = {"arm": ANY, "experiment": ANY, "clauses": {"contrast": str(number)}}
        delta, ratio, share = (inputs.one(metric, **cut) for metric in ("delta_vs", "usd_per_vs_ratio",
                                                                         "mechanism_spend_share"))
        if delta is None and ratio is None and share is None:
            rows.append([str(number), contrast, design, "–", "–", "–", "–", "not run"])
            continue
        rows.append([str(number), contrast, design, str(delta.n) if delta else "–",
                     table.cell(delta, part="T5", row=f"row {number}", column="ΔVS"),
                     table.cell(ratio, part="T5", row=f"row {number}", column="$/VS ratio"),
                     table.cell(share, part="T5", row=f"row {number}", column="spend share", kind="pct",
                                with_ci=False),
                     "pre-registered secondary (BL11)" if number >= 8 else "measured"])
    table.footer += [
        "Rows 1–7 assemble contrasts from different designs, task sets and seeds, so they are neither additive nor a "
        "decomposition.",
        "Rows 8–10 come from S09 v1.1's funded leave-one-mechanism-out ablation (BL11, a pre-registered secondary; "
        "S09 §4.4). They share tasks and seed, but mechanisms interact, so they do not sum to row 7 either.",
    ]
    return table


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
