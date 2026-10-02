"""Dry run of the P2 figure and table scripts (F6-F8, F10, F11; T4, T5, T8, T10) on synthetic records (task 9514).

The MetricRecords follow the cuts each script's READS names, and T4's `loop.health` rows follow S03 §5's
`loop-audit/1`; every record and row says `simulated: true`, and the numbers are seeded random draws, never results.
Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_figures_p2.py -q
"""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest

import fig_f6_reliability
import fig_f7_saso
import fig_f8_audit
import fig_f10_loop_detection
import fig_f11_closures
import figlib
import tab_t4_loop_ledger
import tab_t5_contributions
import tab_t8_saso
import tab_t10_battery
from test_figures_p1 import Fixture

SCRIPTS = {
    fig_f6_reliability: ("a", "b"),
    fig_f7_saso: ("a", "b", "c", "d", "e", "f"),
    fig_f8_audit: ("a", "b", "c"),
    fig_f10_loop_detection: ("a", "b"),
    fig_f11_closures: ("a", "b", "c", "d"),
    tab_t4_loop_ledger: ("T4",),
    tab_t5_contributions: ("T5",),
    tab_t8_saso: ("T8",),
    tab_t10_battery: ("T10",),
}
LIVE_KINDS = ("provider_fault", "budget_cut", "convention_flip")
LOOPS = ("L-route", "L-know", "L-play", "L-sect", "L-M1", "L-placebo")


class P2Fixture(Fixture):
    """P1's record builder, with the P2 cuts and the loop-audit/1 rows T4 reads."""

    def __init__(self, seed: int = 9514):
        super().__init__(seed)
        self.rows: list[dict] = []

    def put(self, metric: str, value: float | None, ci: list[float] | None = None, *, experiment: str,
            arms: tuple[str, ...] = ("roko_full",), n: int = 120, **clauses: object) -> dict:
        extra = tuple((field.replace("__", "."), want) for field, want in clauses.items())
        return self.add(metric, value, ci, arms=arms, experiment=experiment, extra=extra, n=n)

    def interval(self, value: float, spread: float = 0.15) -> list[float]:
        return [round(value * (1 - spread), 4), round(value * (1 + spread), 4)]

    def build(self) -> list[dict]:
        self._calibration()
        self._controllers()
        self._audits()
        self._loops()
        self._closures()
        for number in range(1, 10):  # row 10 stays "not run"
            delta = round(self.rng.uniform(-0.05, 0.2), 4)
            self.put("delta_vs", delta, [round(delta - 0.04, 4), round(delta + 0.04, 4)], experiment="T5",
                     contrast=str(number))
            ratio = round(self.rng.uniform(0.5, 1.2), 4)
            self.put("usd_per_vs_ratio", ratio, self.interval(ratio), experiment="T5", contrast=str(number))
            self.put("mechanism_spend_share", round(self.rng.uniform(0, 0.15), 4), experiment="T5",
                     contrast=str(number))
        for check in ("A1", "A2", "B1", "B2", "B3", "G", "honeypot"):
            self.put("check_runs", self.rng.randint(40, 120), experiment="E-H5-live", check=check)
            for metric in ("sensitivity", "specificity"):
                self.put(metric, *self.rate(), experiment="E-H5-live", check=check)
            self.add("usd_per_check", round(self.rng.uniform(0, 0.3), 4), None, arms=("roko_full",),
                     experiment="E-H5-live", cost_basis="api_equiv_usd", extra=(("check", check),))
            self.put("dismissed_battery_fp", self.rng.randint(0, 3), experiment="E-H5-live", check=check)
        return self.records

    def _calibration(self) -> None:
        for regime, rho in (("G", None), ("A", 0.15), ("C", None)):
            cut = {"label_regime": regime, **({"rho": rho} if rho else {})}
            for number in range(1, 11):
                mean = round((number - 0.5) / 10 + self.rng.uniform(-0.03, 0.03), 4)
                self.put("calibration_bin_mean", mean, experiment="R-M3", bin=number, **cut)
                freq = round(min(1.0, max(0.0, mean + self.rng.uniform(-0.1, 0.1))), 4)
                self.put("calibration_bin_freq", freq, [max(0.0, round(freq - 0.08, 4)), min(1.0, round(freq + 0.08,
                                                                                                          4))],
                         experiment="R-M3", n=self.rng.randint(20, 60), bin=number, **cut)
            for metric in ("brier", "bss", "auroc"):
                self.put(metric, round(self.rng.uniform(0.1, 0.8), 4), experiment="R-M3", **cut)
            for position in range(100, 601, 50):
                ece = round(self.rng.uniform(0.02, 0.1), 4)
                self.put("ece_rolling", ece, self.interval(ece, 0.3), experiment="R-M3", stream__position=position,
                         **cut)
        for regime, rho in (("G", None), ("A", 0.10), ("A", 0.15), ("A", 0.30), ("C", None)):
            ece = round(self.rng.uniform(0.02, 0.09), 4)
            self.put("ece", ece, self.interval(ece, 0.3), experiment="R-M3", label_regime=regime,
                     **({"rho": rho} if rho else {}))

    def _controllers(self) -> None:
        for kind in fig_f7_saso.KINDS:
            for controller in tab_t8_saso.CONTROLLERS:
                cut = {"disturbance__type": kind, "controller": controller}
                iae = round(self.rng.uniform(5, 40), 3)
                self.put("iae", iae, self.interval(iae), experiment="R-H6", stage="replay", **cut)
                for metric, low, high in (("detect_delay", 1, 6), ("settling_time", 5, 30), ("settling_usd", 0.1, 3),
                                          ("settling_s", 60, 900), ("overshoot", 0, 0.5), ("ss_error", 0, 0.1),
                                          ("collateral_max", 0, 0.3), ("adaptation_cost_usd", 0, 1)):
                    self.put(metric, round(self.rng.uniform(low, high), 3), experiment="R-H6", **cut)
                for metric in ("param_changes", "rollbacks", "hold_count"):
                    self.put(metric, self.rng.randint(0, 6), experiment="R-H6", **cut)
                if controller in fig_f7_saso.CONTROLLERS:
                    drive = 0.0
                    for offset in range(-10, 41, 5):
                        drive = 0.0 if offset < 0 else max(0.0, (1.5 if offset == 0 else drive * 0.75)
                                                           + self.rng.uniform(-0.1, 0.1))
                        drive = round(drive, 3)
                        self.put("drive_median", drive, [round(max(0.0, drive - 0.2), 3), round(drive + 0.2, 3)],
                                 experiment="R-H6", offset=offset, **cut)
                if controller == "A3":
                    for offset in range(0, 21, 5):
                        self.put("param_change_count", self.rng.randint(0, 2), experiment="R-H6", offset=offset,
                                 **cut)
                    if kind in LIVE_KINDS:
                        self.put("iae_pi90", iae, self.interval(iae, 0.4), experiment="R-H6", **cut)
                        for replicate in (1, 2, 3):
                            self.put("iae", round(iae * self.rng.uniform(0.5, 1.5), 3), experiment="E-H6-live",
                                     stage="live", replicate=replicate, **cut)
                if controller in fig_f11_closures.CONTROLLERS:
                    diff = round(self.rng.uniform(-8, 4), 3)
                    self.put("iae_diff", diff, [round(diff - 3, 3), round(diff + 3, 3)], experiment="R-H6", **cut)

    def _audits(self) -> None:
        for mode in fig_f8_audit.MODES:
            for rho in fig_f8_audit.RHOS:
                coverage = round(self.rng.uniform(0.86, 0.98), 4)
                self.put("coverage", coverage, [round(coverage - 0.03, 4), min(1.0, round(coverage + 0.02, 4))],
                         experiment="R-H5", n=1000, rho=rho, mode=mode)
        for arm in ("H5-A1", "H5-A3"):
            for position in range(20, 201, 20):
                theta = round(self.rng.uniform(0.02, 0.2), 4)
                self.put("fgr_hajek", theta, [max(0.0, round(theta - 0.08, 4)), round(theta + 0.08, 4)],
                         experiment="E-H5-live", arms=(arm,), stream__position=position)
                self.put("theta_true", round(self.rng.uniform(0.02, 0.2), 4), experiment="E-H5-live", arms=(arm,),
                         stream__position=position)
        for arm in fig_f8_audit.H5_ARMS:
            self.put("theta_true", *self.rate(), experiment="E-H5-live", arms=(arm,))

    def _loops(self) -> None:
        for kind in fig_f10_loop_detection.KINDS:
            for index in range(1, 7):
                missed = index == 6 and kind in ("MASK", "STALE")
                record = self.put("ttd", None if missed else round(self.rng.uniform(1, 60)), experiment="R-H7",
                                  fault__kind=kind, fault__id=f"{kind}-{index:02d}")
                if missed:
                    record["estimator"] += "; null: not detected"
            self.put("detection_rate", *self.rate(), experiment="R-H7", fault__kind=kind)
        for loop, transition in (("L-harmful", (300, -1)), ("L-oracle", (400, 1)), ("L-placebo", None)):
            beta = 0.0
            for n_opp in range(50, 501, 50):
                beta = round(beta + self.rng.uniform(-0.03, 0.04) * (-1 if "harmful" in loop else 1), 4)
                width = round(0.4 / (n_opp / 50) ** 0.5, 4)
                self.put("beta_cs", beta, [round(beta - width, 4), round(beta + width, 4)], experiment="E-H7-live",
                         loop_id=loop, n_opp=n_opp)
            if transition:
                self.put("loop_transition", transition[1], experiment="E-H7-live", loop_id=loop, n_opp=transition[0])
        for loop in LOOPS:
            for phase, experiment, state in (("E0", "E0-census", "flagged"), ("live", "E-H7-live", "live")):
                eps = round(self.rng.uniform(0.3, 1.0), 4)
                record = self.put("epsilon_L", eps, [round(eps * 0.8, 4), min(1.0, round(eps * 1.1, 4))],
                                  experiment=experiment, loop_id=loop, phase=phase)
                self.put("iota_L", *self.rate(), experiment=experiment, loop_id=loop, phase=phase)
                beta = round(self.rng.uniform(-0.05, 0.1), 4)
                self.put("beta_L", beta, [round(beta - 0.05, 4), round(beta + 0.05, 4)], experiment=experiment,
                         loop_id=loop, phase=phase, outcome="U")
                self.rows.append({
                    "schema": "loop-audit/1", "kind": "loop.health", "loop_id": loop, "state": state,
                    "reason": "dormant:unlogged" if state == "flagged" else "live:benefit", "h": 0.2,
                    "n_opp": self.rng.randint(50, 600), "eps": {"est": eps, "ucb": record["ci"][1], "read": 1.0,
                                                                "reach": round(self.rng.uniform(0.5, 1), 3),
                                                                "honest": 1.0, "receipt": 1.0},
                    "iota": {"act": 0.9, "aa": round(self.rng.uniform(0, 0.05), 3), "net": 0.85, "lcb": 0.8},
                    "beta": {"est": beta, "reason": None}, "evidence": "log",
                    "ts": "2026-10-02" if phase == "E0" else "2026-10-03",
                    "harness_sha": "dry-run", "config_hash": "dry-run", "audit_epoch": "2026-10-02",
                    "run_id": record["run_ids"][0], "simulated": True})
            self.put("ttd_median", self.rng.randint(5, 40), experiment="R-H7", loop_id=loop)

    def _closures(self) -> None:
        for mode in ("fixed", "adaptive"):
            delay = round(self.rng.uniform(5, 40), 2)
            self.put("fg_breach_delay", delay, self.interval(delay), experiment="R-H5", mode=mode)
            self.put("audit_spend_share", round(self.rng.uniform(0.03, 0.12), 4), experiment="R-H5", mode=mode)
        for policy in fig_f11_closures.POLICIES:
            self.put("vs_rate", *self.rate(), experiment="LOG1", spec_policy=policy)
            self.add("usd_per_vs", *self.usd(), arms=("roko_full",), experiment="LOG1", cost_basis="api_equiv_usd",
                     extra=(("spec_policy", policy),))
        self.put("delta_brier_spec", -0.012, [-0.02, -0.004], experiment="LOG1")


@pytest.fixture(scope="module")
def fixture() -> P2Fixture:
    built = P2Fixture()
    built.build()
    return built


def write_inputs(directory: Path, records: list[dict], rows: list[dict], *, simulated: bool | None = True) -> list[str]:
    """The records as a vb.metrics/1 document and the loop-audit/1 rows as JSONL, as the scripts' inputs."""
    directory.mkdir(parents=True, exist_ok=True)
    doc = {"schema_version": figlib.METRICS_VERSION, "experiment_id": "DRY-RUN-P2", "records": records}
    if simulated is not None:
        doc["simulated"] = simulated
    (directory / "metrics.json").write_text(json.dumps(doc), encoding="utf-8")
    (directory / "loop-audit.jsonl").write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
    return [str(directory / "metrics.json"), str(directory / "loop-audit.jsonl")]


def outputs(out: Path, module) -> tuple[Path, dict]:
    stem = module.SPEC.stem
    body = out / f"{stem}{'.svg' if module.SPEC.kind == 'figure' else '.md'}"
    return body, json.loads((out / f"{stem}.data.json").read_text(encoding="utf-8"))


def test_p2_figures_dry_run(tmp_path, fixture):
    """All nine render from the synthetic records; every sidecar value and row comes from a record that names it."""
    inputs = write_inputs(tmp_path / "in", fixture.records, fixture.rows)
    by_id = {figlib.canonical_id(record): record for record in fixture.records}
    for module, panels in SCRIPTS.items():
        out = tmp_path / module.SPEC.stem
        assert module.main([*inputs, "--out", str(out), "--dry-run"]) == 0, module.SPEC.script
        body, sidecar = outputs(out, module)
        assert sidecar["dry_run"] is True and sidecar["caption"].startswith(figlib.DRY_RUN_STAMP)
        for value in sidecar["values"]:
            record = by_id[value["record_id"]]
            assert (value["metric"], value["value"], value["ci"]) == (record["metric"], record["value"],
                                                                      record.get("ci"))
        used = {value["record_id"] for value in sidecar["values"]}
        assert all(set(derived["from"]) <= used for derived in sidecar["derived"])
        for row in sidecar["rows"]:
            assert row["named_by"] and set(row["named_by"]) <= used
            assert all(row["run_id"] in by_id[record_id]["run_ids"] for record_id in row["named_by"])
        drawn = {value["panel"] for value in sidecar["values"] + sidecar["rows"]}
        assert set(panels) <= drawn, (module.SPEC.script, sorted(drawn))
        text = body.read_text(encoding="utf-8")
        assert figlib.DRY_RUN_STAMP in text
        if module.SPEC.kind == "figure":
            root = ET.fromstring(text)
            assert {g.get("data-panel") for g in root.iter("{http://www.w3.org/2000/svg}g")} >= set(panels)
    ledger = (tmp_path / "t4-loop-ledger" / "t4-loop-ledger.md").read_text(encoding="utf-8")
    assert "flagged → live" in ledger and "not run" in (tmp_path / "t5-contributions" / "t5-contributions.md"
                                                        ).read_text(encoding="utf-8")
    f10 = json.loads((tmp_path / "f10-loop-detection" / "f10-loop-detection.data.json").read_text())
    assert any(value["value"] is None for value in f10["values"])  # undetected injections are drawn, not dropped


def test_p2_figures_dry_run_is_refused_in_paper_mode(tmp_path, fixture, capsys):
    inputs = write_inputs(tmp_path / "in", fixture.records, fixture.rows)
    for module in SCRIPTS:
        out = tmp_path / "paper" / module.SPEC.stem
        assert module.main([*inputs, "--out", str(out)]) == 1, module.SPEC.script
        assert not out.exists()
    assert "simulated is true" in capsys.readouterr().err


def test_p2_figures_dry_run_reads_only_rows_its_records_name(tmp_path, fixture):
    """A loop.health row no record names is never shown, and an unmarked row stops a dry run."""
    rows = [dict(row, run_id="dry-run-elsewhere") if row["loop_id"] == "L-know" else row for row in fixture.rows]
    inputs = write_inputs(tmp_path / "in", fixture.records, rows)
    assert tab_t4_loop_ledger.main([*inputs, "--out", str(tmp_path / "out"), "--dry-run"]) == 0
    sidecar = outputs(tmp_path / "out", tab_t4_loop_ledger)[1]
    assert not any(row["series"] == "knowledge" for row in sidecar["rows"])
    assert all(row["run_id"] != "dry-run-elsewhere" for row in sidecar["rows"])
    unmarked = [dict(fixture.rows[0])] + fixture.rows[1:]
    del unmarked[0]["simulated"]
    inputs = write_inputs(tmp_path / "in2", fixture.records, unmarked)
    assert tab_t4_loop_ledger.main([*inputs, "--out", str(tmp_path / "out2"), "--dry-run"]) == 1


def test_p2_figures_dry_run_paper_mode_draws_real_records(tmp_path, fixture):
    """In paper mode the same scripts draw unmarked records, P2 labels need not be vs_census, and nothing is
    stamped."""
    records = [{key: value for key, value in record.items() if key != "simulated"} for record in fixture.records]
    records[0]["label_source"] = "vs_estimated"
    rows = [{key: value for key, value in row.items() if key != "simulated"} for row in fixture.rows]
    inputs = write_inputs(tmp_path / "in", records, rows, simulated=None)
    for module in SCRIPTS:
        out = tmp_path / "paper" / module.SPEC.stem
        assert module.main([*inputs, "--out", str(out)]) == 0, module.SPEC.script
        body, sidecar = outputs(out, module)
        assert sidecar["dry_run"] is False and figlib.DRY_RUN_STAMP not in body.read_text(encoding="utf-8")
