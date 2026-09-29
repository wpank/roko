"""Offline tests of the budget lines, their caps and reconciliation (S09 E2): synthetic ledger rows and exports, the
stub provider, no spend.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_ledger.py -q
"""

from __future__ import annotations

import csv
import datetime as dt
import json
import re
from pathlib import Path

import pytest

import layout
import ledger
import validate
import vb
from common import hmac_seed
from stub_provider import StubServer, bash

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
SNAPSHOT = ledger.load_snapshot()


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def run_vb(places: dict[str, Path], url: str, *extra: str) -> int:
    return vb.main(["run", "--experiment", "TEST-BUDGET", "--run-id", "run-2", "--stream", TOY_STREAM,
                    "--arm", "cheap_direct", "--model", "gpt-oss-120b", "--seeds", "1", "--provider-url", url,
                    "--results", str(places["results"]), "--work", str(places["work"]),
                    "--secret-file", str(places["secret"]), *extra])


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def write_row(root: Path, *, line: str = "BL0", experiment: str = "PILOT-A", run: str = "earlier", n: int = 1,
              day: str = "2026-10-12", provider: str = "cerebras", model: str = "gpt-oss-120b",
              tokens: tuple[int, int] = (40_000, 1_200), usd: float | None = None, known: bool = True,
              billed: bool = True) -> dict:
    """Append a schema-valid ledger row for an earlier attempt: priced from the snapshot, unless `usd` is given."""
    usage = ledger.vb_usage(*tokens) if known else None
    cost = (ledger.price(usage, SNAPSHOT.row(model)).api_equiv_usd if usd is None else usd) if known else None
    row = {"ts": f"{day}T12:00:00Z", "line": line, "experiment_id": experiment, "run_id": run,
           "attempt_key": f"{run}/t{n}.s1:1", "provider": provider, "model_reported": model, "usage": usage,
           "api_equiv_usd": cost, "billed_usd": cost if billed else 0.0, "reserved_usd": 0.13,
           "price_snapshot_id": SNAPSHOT.id,
           "source": "unknown" if not known else "provider_usage" if billed else "cli_usage"}
    assert validate.validate("ledger", row) == []
    path = root / experiment / run / "ledger.jsonl"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row) + "\n")
    return row


def open_ledger(root: Path, *, experiment: str = "PILOT-A", run: str = "now", line: str = "BL0") -> ledger.Ledger:
    """The ledger of a run in progress, laid out as `vb run` lays it out."""
    run_dir = root / experiment / run
    run_dir.mkdir(parents=True, exist_ok=True)
    return ledger.Ledger(run_dir / "ledger.jsonl", line=line, experiment_id=experiment, run_id=run,
                         price_snapshot_id=SNAPSHOT.id)


def write_csv(path: Path, header: list[str], rows: list[list]) -> Path:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows(rows)
    return path


def test_budget_is_s09_v1_2():
    budget = ledger.load_budget()
    caps = {line.id: line.cap_usd for line in budget.lines.values()}
    planned = {line.id: line.planned_usd for line in budget.lines.values()}
    assert caps == {"BL0": 10, "BL1": 160, "BL2": 0, "BL3": 30, "BL4": 40, "BL5": 26, "BL6": 44, "BL7": 26, "BL8": 8,
                    "BL9": 20, "BL10": 2, "BL11": 18, "BL13": 6}
    assert planned == {"BL0": 6, "BL1": 152, "BL2": 0, "BL3": 27, "BL4": 36, "BL5": 23, "BL6": 42, "BL7": 24,
                       "BL8": 6, "BL9": 15, "BL10": 1.2, "BL11": 16.2, "BL13": 5}
    assert budget.caps_usd == 390 and budget.planned_usd == 353.4  # S09 §4.6's totals
    assert budget.stop_usd == 400 and budget.total_usd - budget.caps_usd == 110 >= budget.floor_usd  # S09 SC3
    assert "D43" in budget.reserved["BL12"]
    # v1.2's rebalance is a default the author confirms at the lock, and it leaves the caps' sum unchanged.
    lowered = [budget.lines[line_id] for line_id in ("BL1", "BL6", "BL7")]
    assert {line.id for line in budget.lines.values() if line.confirm == "lock"} == {"BL1", "BL6", "BL7", "BL13"}
    assert sum(line.was_cap_usd for line in lowered) == sum(line.cap_usd for line in lowered) + 6
    # W10 rec 7's raise of BL0 is recorded, not in force; with it the caps would still fit the stop.
    assert budget.lines["BL0"].proposed_cap_usd == 14 and budget.caps_usd + 4 <= budget.stop_usd
    [pilot] = budget.experiments
    assert (pilot.id, pilot.experiment_ids, pilot.lines, pilot.cap_usd) == ("pilot", ("PILOT-A", "PILOT-B"),
                                                                            ("BL0", "BL8"), 15)


def test_budget_file_is_checked(tmp_path):
    text = ledger.BUDGET_FILE.read_text(encoding="utf-8")
    for (old, new), problem in [
        (("cap_usd = 160", "cap_usd = 180"), "must fit the $400 stop"),
        (("stop_usd = 400", "stop_usd = 420"), "never allocated (S09 SC3)"),
        (("planned_usd = 6\n", "planned_usd = 11\n"), "BL0: its planned amount must fit its cap"),
        (("cap_usd = 26\nwas", "cap_usd = -26\nwas"), "cap_usd must be a number of at least 0, not -26"),
        (('id = "BL3"', 'id = "BL2"'), "each appears once"),
        (('162\nconfirm = "lock"', '162\nconfirm = "later"'), "BL1: confirm must be"),
        (('gate = "G6"', 'gate = "G6"\ncolour = "red"'), "unknown key colour"),
        (("[reserved]\n", '[reserved]\nBL0 = "taken"\n'), "BL0: a held-back id"),
        (('lines = ["BL0", "BL8"]', 'lines = ["BL0", "BL12"]'), "experiment cap pilot: its lines must be funded"),
    ]:
        path = tmp_path / "budget.toml"
        path.write_text(text.replace(old, new, 1), encoding="utf-8")
        with pytest.raises(ledger.BudgetError, match=re.escape(problem)):
            ledger.load_budget(path)


def test_dispatch_over_line_cap_is_refused(tmp_path):
    root = tmp_path / "results"
    write_row(root, usd=9.80)  # an earlier pilot run spent $9.80 of BL0's $10
    book = open_ledger(root)
    with pytest.raises(ledger.BudgetError, match=re.escape("line BL0: $9.8000 spent + $0.0000 reserved + $0.2500")):
        book.reserve("now/t1.s1:1", 0.25)
    assert not (root / "PILOT-A" / "now" / ledger.RESERVATIONS).exists()  # a refused dispatch reserves nothing
    book.reserve("now/t1.s1:1", 0.15)

    # Another run on the line counts the open reservation, in its refusal and in its own reservation.
    other = open_ledger(root, run="elsewhere")
    assert "$0.1500 reserved" in other.refusal(0.06)
    with pytest.raises(ledger.BudgetError):
        other.reserve("elsewhere/t1.s1:1", 0.06)

    # The attempt's row releases its reservation: only the $0.01 it cost still counts.
    book.append(attempt_key="now/t1.s1:1", provider="cerebras", model_reported="gpt-oss-120b",
                usage=ledger.vb_usage(20_000, 500), cost=ledger.Cost(0.01, 0.01, "provider_usage"), billed=True,
                reserved_usd=0.15)
    assert ledger.read_books(root).reservations == [] and other.refusal(0.06) is None

    # An attempt that made no call is released; one whose run died mid-attempt stays counted.
    other.reserve("elsewhere/t1.s1:1", 0.06)
    other.release("elsewhere/t1.s1:1")
    book.reserve("now/t2.s1:1", 0.12)
    [held] = ledger.read_books(root).reservations
    assert (held["attempt_key"], held["reserved_usd"], held["line"]) == ("now/t2.s1:1", 0.12, "BL0")
    later = open_ledger(root, run="later")
    assert later.refusal(0.10).startswith("line BL0: $9.8100 spent + $0.1200 reserved")
    assert later.refusal(0.0) is None  # a subscription dispatch bills $0 and still fits


def test_dispatch_over_line_cap_is_refused_before_any_provider_call(places):
    write_row(places["results"], experiment="TEST-BUDGET", usd=9.80)  # less than one task's $0.29 worst case left
    with StubServer(lambda body: bash("echo VB_SUBMIT")) as stub:
        assert run_vb(places, stub.url) == 1
        assert stub.requests == []
    run_dir = places["results"] / "TEST-BUDGET" / "run-2"
    [error] = read_jsonl(run_dir / "errors.jsonl")
    assert error["stage"] == "budget" and "line BL0: $9.8000 spent" in error["error"]
    assert not (run_dir / "records.jsonl").exists() and not (run_dir / "ledger.jsonl").exists()


def test_attempt_over_line_cap_is_refused_by_the_runner(places, monkeypatch):
    """The runner's own reservation binds when the room went after the task's check (say, to a concurrent run)."""
    write_row(places["results"], experiment="TEST-BUDGET", usd=9.90)  # less than one attempt's $0.13 worst case left
    monkeypatch.setattr(ledger.Ledger, "refusal", lambda self, worst_usd: None)
    with StubServer(lambda body: bash("echo VB_SUBMIT")) as stub:
        assert run_vb(places, stub.url, "--limit", "1") == 0
        assert stub.requests == []
    run_dir = places["results"] / "TEST-BUDGET" / "run-2"
    [record] = read_jsonl(run_dir / "records.jsonl")
    assert record["execution"]["status"] == "aborted_cap" and record["execution"]["reason"] == "budget"
    assert record["execution"]["attempts"] == [] and record["vs"]["label"] == 0
    assert not (run_dir / "ledger.jsonl").exists() and ledger.read_books(places["results"]).reservations == []


def test_experiment_cap_programme_stop_and_unfunded_lines(tmp_path, monkeypatch):
    root = tmp_path / "results"
    write_row(root, usd=9.00)  # Pilot A on BL0
    write_row(root, line="BL8", provider="openai", model="gpt-5.4", usd=5.90, n=2)  # and its fd_api runs on BL8
    pilot_b = open_ledger(root, experiment="PILOT-B", run="b-1")
    assert pilot_b.refusal(0.13).startswith("experiment cap pilot: $14.9000 spent")  # BL0 alone has $1.00 left
    assert pilot_b.refusal(0.05) is None
    assert "may book only to BL0, BL8" in open_ledger(root, experiment="PILOT-B", run="b-2", line="BL1").refusal(0.05)
    assert "held back (D43" in open_ledger(root, experiment="LOG1", run="l-1", line="BL12").refusal(0.0)
    assert "BL99 is not a budget line" in open_ledger(root, experiment="LOG1", run="l-2", line="BL99").refusal(0.0)

    # Spend on a line the budget does not fund still counts toward the programme stop.
    write_row(root, line="BL99", experiment="OLD", run="old", usd=385.0)
    log1 = open_ledger(root, experiment="LOG1", run="l-3", line="BL1")
    assert log1.refusal(0.13).startswith("the programme stop: $399.9000 spent")
    assert log1.refusal(0.10) is None

    # A row still being written is skipped; an unreadable one refuses every dispatch.
    ledger_file = root / "OLD" / "old" / "ledger.jsonl"
    with ledger_file.open("a", encoding="utf-8") as handle:
        handle.write('{"ts": "2026-10-12T12:00:00Z", "line": "BL1"')
    assert log1.refusal(0.10) is None
    with ledger_file.open("a", encoding="utf-8") as handle:
        handle.write("\n")
    assert "an unreadable ledger row" in log1.refusal(0.0)

    # So does a budget file that does not load.
    broken = tmp_path / "broken.toml"
    broken.write_text('schema_version = "vb.budget/0"\n', encoding="utf-8")
    monkeypatch.setattr(ledger, "BUDGET_FILE", broken)
    fresh = open_ledger(tmp_path / "other-results")
    assert str(broken) in fresh.refusal(0.0)
    with pytest.raises(ledger.BudgetError):
        fresh.reserve("now/t1.s1:1", 0.01)


def test_ledger_report_shows_spent_reserved_and_cap(tmp_path, capsys):
    root = tmp_path / "results"
    write_row(root, usd=2.50)  # Pilot A on BL0
    write_row(root, line="BL8", provider="openai", model="gpt-5.4", usd=1.25, n=2)  # Pilot A on BL8
    write_row(root, line="BL1", experiment="LOG1", run="log1-a", known=False)  # unknown cost: its $0.13 reservation
    open_ledger(root).reserve("now/t1.s1:1", 0.126)  # an attempt in flight
    assert vb.main(["ledger", "report", "--results", str(root), "--json"]) == 0
    data = json.loads(capsys.readouterr().out)
    lines = {line["id"]: line for line in data["lines"]}
    assert list(lines) == [f"BL{n}" for n in (*range(12), 13)]
    assert (lines["BL0"]["spent_usd"], lines["BL0"]["reserved_usd"], lines["BL0"]["cap_usd"]) == (2.5, 0.126, 10)
    assert lines["BL0"]["left_usd"] == pytest.approx(7.374) and lines["BL0"]["rows"] == 1
    assert lines["BL1"]["spent_usd"] == 0.13 and lines["BL8"]["spent_usd"] == 1.25
    [pilot] = data["experiments"]
    assert (pilot["spent_usd"], pilot["reserved_usd"], pilot["cap_usd"]) == (3.75, 0.126, 15)
    programme = data["programme"]
    assert (programme["caps_usd"], programme["planned_usd"], programme["stop_usd"]) == (390, 353.4, 400)
    assert (programme["unallocated_usd"], programme["caps_with_proposals_usd"]) == (110, 394)
    assert programme["spent_usd"] == pytest.approx(3.88) and data["ok"] and data["over"] == []

    write_row(root, line="BL2", experiment="LOG1", run="log1-a", n=2, usd=0.01)  # replays have a $0 cap
    assert vb.main(["ledger", "report", "--results", str(root)]) == 1
    text = capsys.readouterr().out
    assert "OVER CAP: BL2" in text and "BL12  held back, funds nothing: D43" in text
    assert "proposed cap $14.00 awaits the author" in text
    assert "v1.2 default (was $162.00), to be confirmed at the lock" in text and "v1.2 default (new)" in text


def test_reconcile_flags_drift_over_five_percent(tmp_path, capsys):
    root = tmp_path / "results"
    days = {"2026-10-12": [write_row(root, n=n) for n in range(3)],
            "2026-10-13": [write_row(root, n=10 + n, day="2026-10-13", tokens=(60_000, 2_000)) for n in range(2)]}
    write_row(root, n=20, day="2026-10-13", line="BL8", provider="openai", model="gpt-5.4")  # on OpenAI's bill
    write_row(root, n=21, day="2026-10-13", billed=False)  # a subscription row is on no bill
    sums = {day: [sum(row["usage"]["tokens_in"] for row in rows), sum(row["usage"]["tokens_out"] for row in rows),
                  sum(row["api_equiv_usd"] for row in rows)] for day, rows in days.items()}
    header = ["Date", "Model", "Input Tokens", "Output Tokens", "Cost (USD)"]
    export = tmp_path / "cerebras-usage.csv"
    # Cerebras bills day one 2% above the ledger, within the tolerance, and day two 7% above it.
    write_csv(export, header, [[day, "gpt-oss-120b", tokens_in, tokens_out, cost * factor]
                               for (day, (tokens_in, tokens_out, cost)), factor in zip(sums.items(), (1.02, 1.07))])
    command = ["ledger", "reconcile", "--provider", "cerebras", "--csv", str(export), "--results", str(root)]
    assert vb.main([*command, "--json"]) == 1
    result = json.loads(capsys.readouterr().out)
    assert result["ledger_rows"] == 5 and result["window"] == ["2026-10-12", "2026-10-13"] and not result["ok"]
    assert [(e["day"], e["measure"]) for e in result["comparisons"] if e["flagged"]] == [("2026-10-13", "cost_usd")]
    cost = {entry["day"]: entry for entry in [*result["comparisons"], *result["totals"]]
            if entry["measure"] == "cost_usd"}
    assert cost["2026-10-12"]["drift"] == pytest.approx(1 / 1.02 - 1, abs=1e-6) and not cost["2026-10-12"]["flagged"]
    assert cost["2026-10-13"]["drift"] == pytest.approx(1 / 1.07 - 1, abs=1e-6) and cost["2026-10-13"]["flagged"]
    assert not cost["total"]["flagged"]  # −4.3% over both days

    write_csv(export, header, [[day, "gpt-oss-120b", *values] for day, values in sums.items()])
    assert vb.main(command) == 0
    assert "OK: every comparison is within ±5%" in capsys.readouterr().out
    write_csv(export, header, [[day, "gpt-oss-120b", *values] for day, values in sums.items()]
              + [["2026-10-14", "gpt-oss-120b", 1000, 100, 0.0005]])  # usage the ledger has no row for
    assert vb.main(command) == 1
    assert "2026-10-14" in capsys.readouterr().out
    assert vb.main([*command, "--since", "2026-10-20"]) == 2  # a window with no export days is no agreement
    assert "outside --since and --until" in capsys.readouterr().err


def test_reconcile_covers_zai_and_moonshot_exports(tmp_path, capsys):
    root = tmp_path / "results"
    zai = [write_row(root, line="BL1", experiment="LOG1", run="log1-z", n=n, provider="zai", model="glm-4.7",
                     tokens=(50_000, 3_000)) for n in range(4)]
    kimi = [write_row(root, line="BL1", experiment="LOG1", run="log1-k", n=n, provider="moonshot",
                      model="kimi-k2.6", tokens=(50_000, 3_000)) for n in range(4)]

    def reconcile(provider: str, export: Path, *extra: str) -> tuple[int, dict | str]:
        code = vb.main(["ledger", "reconcile", "--provider", provider, "--csv", str(export), "--results", str(root),
                        "--json", *extra])
        captured = capsys.readouterr()
        return code, captured.err if code == 2 else json.loads(captured.out)

    # zai: other header names, an offset timestamp, dollar signs and a currency column; its bill matches the ledger.
    cost = sum(row["api_equiv_usd"] for row in zai)
    export = write_csv(tmp_path / "zai.csv", ["Usage Date", "Model Name", "Prompt Tokens", "Completion Tokens",
                                              "Amount", "Currency"],
                       [["2026-10-12T20:00:00+08:00", "GLM-4.7", 200_000, 12_000, f"${cost:.6f}", "USD"]])
    code, result = reconcile("zai", export, "--by-model")
    assert code == 0 and result["ledger_rows"] == 4 and {e["model"] for e in result["comparisons"]} == {"glm-4.7"}
    [note] = result["notes"]
    assert "infers reasoning_in_output = true for glm-4.7, and the export agrees" in note

    # Moonshot bills hidden reasoning as output the ledger never saw: the inferred flag is called into question.
    tokens_out = sum(row["usage"]["tokens_out"] for row in kimi)
    hidden = tokens_out // 2
    billed = sum(row["api_equiv_usd"] for row in kimi) + hidden * SNAPSHOT.row("kimi-k2.6")["output"] / 1e6
    noon = int(dt.datetime(2026, 10, 12, 12, tzinfo=dt.UTC).timestamp())
    export = write_csv(tmp_path / "moonshot.csv", ["time", "model", "prompt_tokens", "Tokens (output)", "cost"],
                       [[noon, "kimi-k2.6", 200_000, tokens_out + hidden, billed]])
    code, result = reconcile("moonshot", export, "--column", "output_tokens=Tokens (output)")
    assert code == 1 and {e["measure"] for e in result["totals"] if e["flagged"]} == {"output_tokens", "cost_usd"}
    [note] = result["notes"]
    assert "infers reasoning_in_output = true for kimi-k2.6, which may be wrong" in note

    # A bill in another currency, and a provider the snapshot does not price, are refused.
    export = write_csv(tmp_path / "moonshot-cny.csv", ["date", "model", "cost", "currency"],
                       [["2026-10-12", "kimi-k2.6", "1.70", "CNY"]])
    code, error = reconcile("moonshot", export)
    assert code == 2 and "amounts in CNY" in error
    code, error = reconcile("deepseek", export)
    assert code == 2 and "--provider must be one of" in error
