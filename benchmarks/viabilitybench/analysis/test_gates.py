"""Tests of the gate pages (`gates.py`, 3309): G0 on a fixture bundle with every kind of evidence, one check failing and
one evidence file missing. Offline; the records come from `test_analysis`'s factories.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_gates.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import gates
import report
from test_analysis import budget_file, run_record, write_run

PILOT = "PILOT-A"


def fixture_runs() -> dict[str, list[dict]]:
    """The pilot's arms on a few tasks. cheap_direct's level-5 tasks pass half the time, which breaks the ladder band
    (VS ≤ 0.3 at level 5): the one failing check."""
    cheap = [run_record(f"F1-l1-000{n}", seed, run_id="cheap") for n in (1, 2, 3, 4) for seed in (1, 2, 3)]
    cheap += [run_record(f"F4-l5-000{n}", seed, run_id="cheap", label=int(n <= 2)) for n in (1, 2, 3, 4)
              for seed in (1, 2, 3)]
    runs = {"cheap": cheap,
            "api": [run_record("F4-l2-0001", run_id="api", arm="fd_api", cost=0.2, model="gpt-5.4"),
                    run_record("F1-l3-0001", run_id="api", arm="fd_api", cost=0.2, label=0, model="gpt-5.4")],
            "claude": [run_record("F1-l5-0001", run_id="claude", arm="fd_claude", cost=0.3, billed=False, vendor=0.28,
                                  model="claude-opus-5-5"),
                       run_record("F4-l5-0002", run_id="claude", arm="fd_claude", cost=0.3, billed=False,
                                  vendor=0.27, label=0, model="claude-opus-5-5")],
            "roko": [run_record("F1-l1-0001", run_id="roko", arm="roko_fixed", cost=0.05, label=0, verdict="passed"),
                     run_record("F4-l3-0002", run_id="roko", arm="roko_fixed", cost=0.05, label=0)]}
    for records in runs.values():
        for record in records:
            record["experiment_id"] = PILOT
    return runs


def reconcile_json(path: Path, provider: str, drift: float) -> Path:
    entry = {"day": "2026-10-05", "model": "*", "measure": "cost_usd", "ledger": 1.0, "export": 1.0 / (1 + drift),
             "drift": drift, "flagged": abs(drift) > 0.05}
    path.write_text(json.dumps({"provider": provider, "window": ["2026-10-05", "2026-10-05"], "tolerance": 0.05,
                                "comparisons": [entry], "totals": [{**entry, "day": "total"}], "flagged": 0,
                                "ok": True}))
    return path


def sc2_file(path: Path, records: list[dict]) -> Path:
    """Twenty hand labels, ten on VS = 1 outputs and ten on VS = 0 ones; the checker disagrees once."""
    ones = [record for record in records if record["vs"]["label"] == 1][:10]
    zeros = [record for record in records if record["vs"]["label"] == 0][:10]
    items = "".join(f'\n[[item]]\nrun_id = "{r["run_id"]}"\ninstance_id = "{r["task"]["instance_id"]}"\n'
                    f'seed = {r["seed"]}\nhuman_label = {r["vs"]["label"] if i else 1 - r["vs"]["label"]}\n'
                    for i, r in enumerate(ones + zeros))
    path.write_text('schema_version = "vb.sc2/1"\nchecked_by = "the author"\ndate = 2026-10-05\n\n[direct_loop]\n'
                    'choice = "mini-loop"\nreason = "mini-swe-agent was not pinned in time"\n' + items)
    return path


def test_g0_page_reports_every_check_with_its_value(tmp_path):
    results, bundle = tmp_path / "results", tmp_path / "reports" / "pilot_a"
    runs = fixture_runs()
    for run_id, records in runs.items():
        path = write_run(results / PILOT, records, line="BL8" if run_id == "api" else "BL0")
        if run_id == "api":  # fd_api bills OpenAI
            rows = [json.loads(line) for line in (path / "ledger.jsonl").read_text().splitlines()]
            (path / "ledger.jsonl").write_text("".join(json.dumps({**row, "provider": "openai"}) + "\n"
                                                       for row in rows))
    assert report.main(["--experiment", PILOT, "--results", str(results), "--bundle", str(bundle)]) == 0
    runaway = run_record("F1-l1-0002", run_id="runaway", label=0, status="aborted_cap")
    runaway["execution"]["reason"] = "model_calls"
    runaway["execution"]["attempts"][0]["calls"] = 30
    write_run(tmp_path / "runaway", [runaway])
    every = [record for records in runs.values() for record in records]
    args = ["G0", "--runs", str(bundle), "--verifier-ci", str(tmp_path / "missing-ci.json"),
            "--reconcile", str(reconcile_json(tmp_path / "cerebras.json", "cerebras", 0.012)),
            "--reconcile", str(reconcile_json(tmp_path / "openai.json", "openai", -0.031)),
            "--sc2", str(sc2_file(tmp_path / "sc2.toml", every)), "--runaway", str(tmp_path / "runaway"),
            "--out", str(bundle)]
    assert gates.main(args) == 1  # NO-GO

    doc = json.loads((bundle / "g0.json").read_text())
    assert doc["schema_version"] == "vb.gate/1" and doc["gate"] == "G0" and doc["verdict"] == "NO-GO"
    assert doc["thresholds_sha256"] == gates.thresholds_sha256("G0") and doc["thresholds"] == gates.THRESHOLDS["G0"]
    checks = {check["id"]: check for check in doc["checks"]}
    assert list(checks) == ["verifier_ci", "ledger_reconcile", "runaway_killed", "canary_hits", "sc2_spot_check",
                            "ladder_cheap_l1", "ladder_cheap_l5", "ladder_frontier_l5", "cost_per_task.cheap_direct",
                            "cost_per_task.fd_api", "cost_per_task.roko_fixed", "u_prime_and_r", "fault_rates",
                            "roko_ingest", "direct_loop_choice"]
    verdicts = {name: check["verdict"] for name, check in checks.items()}
    assert [name for name, verdict in verdicts.items() if verdict == "fail"] == ["ladder_cheap_l5"]
    assert sorted(name for name, verdict in verdicts.items() if verdict == "not evaluated") == [
        "fault_rates", "verifier_ci"]  # the missing CI file, and no provider_fault run in the fixture
    assert any("missing-ci.json" in problem for problem in doc["evidence_problems"])
    assert checks["verifier_ci"]["value"] is None and "no verifier-CI JSON" in checks["verifier_ci"]["detail"]

    # Every evaluated check carries its value, its threshold and the runs behind it.
    assert checks["ladder_cheap_l5"]["value"] == 0.5 and checks["ladder_cheap_l5"]["run_ids"] == ["cheap"]
    assert checks["ladder_cheap_l5"]["threshold"] == "VS ≤ 0.3"
    assert (checks["ladder_cheap_l1"]["value"], checks["ladder_frontier_l5"]["value"]) == (1.0, 0.5)
    assert checks["ledger_reconcile"]["value"] == {"cerebras": 0.012, "openai": 0.031}
    assert checks["ledger_reconcile"]["run_ids"] == ["api", "cheap", "roko"]
    assert checks["runaway_killed"]["value"] == [{"run_id": "runaway", "reason": "model_calls", "calls": 30}]
    assert (checks["canary_hits"]["value"], checks["sc2_spot_check"]["value"]) == (0, "19/20")
    assert checks["cost_per_task.cheap_direct"]["value"] == 0.02 and checks["cost_per_task.fd_api"]["value"] == 0.2
    assert checks["u_prime_and_r"]["value"] == {"captured": "2/2", "u_r_gap": round(abs(0.6 - 0.55) / 0.55, 6)}
    assert checks["roko_ingest"]["value"] == {"recorded": "2/2", "mismatch_outside_infra_error": 0}
    assert checks["direct_loop_choice"]["value"] == "mini-loop"
    for check in checks.values():
        assert check["threshold"] and (check["verdict"] == "not evaluated" or check["value"] is not None)

    page = (bundle / "go-no-go.md").read_text()
    assert page.startswith("# G0 go/no-go: NO-GO")
    for check in checks.values():
        assert f"| {check['title']} |" in page and f"**{check['verdict']}**" in page
    # The page sits in the bundle, which report.py --check still accepts.
    budget = budget_file(tmp_path / "budget.toml", {"BL0": 10, "BL8": 8}, "\n[programme]\nstop_usd = 400\n")
    assert report.check([bundle], budget) == []


def test_g0_checks_fail_closed_and_go_needs_every_check(tmp_path):
    """No evidence is no pass; a leak, a short ingest and a reconcile past 5% each fail; GO needs every check."""
    empty = gates.Evidence(runs=[])
    assert {check.verdict for check in gates.g0(empty)} == {"not evaluated"}
    assert gates.verdict(gates.g0(empty)) == "INCOMPLETE"
    results = tmp_path / "results"
    leaked = run_record("F1-l1-0001", run_id="leak", status="leak_suspected")
    leaked["provenance"].update(canary_hits=1, canary_places=["transcript"])
    leaked["experiment_id"] = PILOT
    write_run(results / PILOT, [leaked])
    short = write_run(results / PILOT, [run_record("F1-l1-0002", run_id="rk", arm="roko_fixed", verdict="passed")])
    manifest = json.loads((short / "manifest.json").read_text())
    (short / "manifest.json").write_text(json.dumps({**manifest, "runs": 2}))  # one planned run has no record
    evidence = gates.Evidence(runs=report.load_runs(results / PILOT),
                              reconciles={"cerebras": json.loads(reconcile_json(tmp_path / "c.json", "cerebras",
                                                                                0.08).read_text())})
    checks = {check.id: check for check in gates.g0(evidence)}
    assert checks["canary_hits"].verdict == "fail" and checks["canary_hits"].run_ids == ["leak"]
    assert checks["roko_ingest"].verdict == "fail" and checks["roko_ingest"].value["recorded"] == "1/2"
    assert checks["ledger_reconcile"].verdict == "fail" and checks["ledger_reconcile"].value == {"cerebras": 0.08}
    assert gates.verdict(list(checks.values())) == "NO-GO"
    assert gates.verdict([gates.Check("a", "A", "pass", 1, "x")]) == "GO"
