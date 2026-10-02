#!/usr/bin/env python3
"""T4: the loop-liveness ledger: epsilon, iota, beta and state per loop (§7.3; H7, closure 4). Spec: FIGURES-TABLES.md,
T4.

    tab_t4_loop_ledger.py INPUT [INPUT ...] --out DIR [--dry-run]

One row per registered loop, displayed by its mechanism name (the data key is its S03 loop id, `NAMES`). The numbers
are MetricRecords with clauses `loop_id` and `phase` (E0, the census at the frozen tag before repair, or live):
`epsilon_L` (the exposure estimate, its CI's upper end the UCB), `iota_L` (net influence, its CI's lower end the LCB),
`beta_L` (the benefit with its confidence sequence, clause `outcome`: U or beta_cost) and `ttd_median` (TTD in the
fault runs). The state, reason, evidence kind, opportunities and epsilon's read/reach/honest components and the A/A
floor come from S03's `loop.health` rows (`loop-audit/1`, given as JSONL inputs): for each loop and phase, the last
row whose `run_id` the phase's `epsilon_L` record names. A row no record names is never shown. figlib has the
refusal rules and the sidecar.
"""

from __future__ import annotations

import sys

import figlib
from figlib import ANY

SPEC = figlib.Spec("T4", "t4-loop-ledger", "Loop-liveness ledger (ε, ι, β, state)", "tab_t4_loop_ledger.py",
                   kind="table", p1=False)
READS = {
    "epsilon_L": "a loop's exposure estimate, ci [low, UCB] (clauses loop_id, phase)",
    "iota_L": "a loop's net influence, ci [LCB, high] (clauses loop_id, phase)",
    "beta_L": "a loop's benefit with its confidence sequence (clauses loop_id, phase, outcome)",
    "ttd_median": "the median TTD of the loop's injected faults in the E1 runs (clause loop_id)",
    "loop-audit/1 loop.health rows": "state, reason, evidence, n_opp, eps.{read, reach, honest}, iota.aa",
}
# S03 loop id -> the mechanism name the table shows; loops the records name beyond these show their id.
NAMES = {
    "L-route": "routing", "L-linucb": "LinUCB", "L-know": "knowledge", "L-play": "playbooks",
    "L-sect": "prompt sections", "L-pexp": "prompt experiments", "L-gate": "gate thresholds", "L-rag": "retrieval",
    "L-holdout": "the holdout", "L-dream": "offline-consolidation bias", "L-M1": "regulator M1",
    "L-M3": "regulator M3", "L-M4": "regulator M4 (sensor exempt)", "L-placebo": "placebo",
}
SCHEMA = "loop-audit/1"
COLUMNS = ("loop", "state E0 → live", "ε̂ at E0", "reason (live)", "evidence", "ε̂ [UCB] (read, reach, honest)",
           "ι_net [LCB] (A/A floor)", "β̂ [CS] (outcome)", "opportunities", "TTD (fault runs)")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    rows = table.part("", COLUMNS)
    loops = {rec.eq("loop_id") for rec in inputs.where("epsilon_L", arm=ANY, clauses={"loop_id": ANY, "phase": ANY})}
    order = list(NAMES)
    for loop in sorted(loops, key=lambda loop: (order.index(loop) if loop in order else len(order), loop)):
        name = NAMES.get(loop, loop)
        health = {}
        for phase in ("E0", "live"):
            rec = inputs.one("epsilon_L", arm=ANY, clauses={"loop_id": loop, "phase": phase})
            if rec is not None:
                health[phase] = (rec, _health_row(inputs, loop, rec))
        if "live" not in health:
            table.skip("T4", f"{loop}: no live epsilon_L record")
            continue
        live, row = health["live"]
        before = health.get("E0", (None, None))[1]
        epsilon = table.cell(live, part="T4", row=name, column="epsilon", with_ci=False)
        if live.ci and live.value is not None:
            epsilon += f" [{figlib.fmt(live.ci[1])}]"
        if row is not None:
            parts = [table.take_row("T4", name, f"eps.{key}", row["eps"][key], row, [live])
                     for key in ("read", "reach", "honest")]
            epsilon += " (" + ", ".join(figlib.fmt(part) for part in parts) + ")"
        iota_rec = inputs.one("iota_L", arm=ANY, clauses={"loop_id": loop, "phase": "live"})
        iota = table.cell(iota_rec, part="T4", row=name, column="iota", with_ci=False)
        if iota_rec is not None and iota_rec.ci and iota_rec.value is not None:
            iota += f" [{figlib.fmt(iota_rec.ci[0])}]"
        if row is not None and iota_rec is not None:
            iota += f" ({figlib.fmt(table.take_row('T4', name, 'iota.aa', row['iota']['aa'], row, [live]))})"
        beta_rec = inputs.one("beta_L", arm=ANY, clauses={"loop_id": loop, "phase": "live", "outcome": ANY})
        beta = table.cell(beta_rec, part="T4", row=name, column="beta")
        if beta_rec is not None:
            beta += f" ({beta_rec.eq('outcome')})"
        before_eps = (table.cell(health["E0"][0], part="T4", row=name, column="epsilon (E0)", with_ci=False)
                      if "E0" in health else "–")
        phases = (("E0", health.get("E0")), ("live", (live, row)))
        states = [table.take_row("T4", name, f"state ({phase})", found[1]["state"], found[1], [found[0]])
                  if found and found[1] else "–" for phase, found in phases]
        rows.append([
            name + (f" (`{loop}`)" if loop in NAMES else ""),
            " → ".join(states),
            before_eps,
            table.take_row("T4", name, "reason", row["reason"], row, [live]) if row else "–",
            table.take_row("T4", name, "evidence", row.get("evidence", "–"), row, [live]) if row else "–",
            epsilon, iota, beta,
            str(table.take_row("T4", name, "n_opp", row["n_opp"], row, [live])) if row else "–",
            table.cell(inputs.one("ttd_median", arm=ANY, clauses={"loop_id": loop}), part="T4", row=name,
                       column="TTD", kind="count", with_ci=False),
        ])
        if before is None and "E0" in health:
            table.skip("T4", f"{loop}: no loop.health row for the E0 phase's runs")
    table.footer.append("ε̂ with its UCB, ι_net with its LCB and β̂ with its confidence sequence come from the "
                        "MetricRecords; states, reasons, evidence and opportunities from the loop.health rows they "
                        "name (S03 §5).")
    return table


def _health_row(inputs: figlib.Inputs, loop: str, rec: figlib.Rec) -> dict | None:
    """The last `loop.health` row of the loop whose run the record names."""
    rows = [row for row in inputs.rows(SCHEMA, kind="loop.health", loop_id=loop) if row["run_id"] in rec.doc["run_ids"]]
    return max(rows, key=lambda row: str(row.get("ts", ""))) if rows else None


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
