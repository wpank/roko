"""Tests for `vb refine` and its live model (refine_live.py, task 3236), offline: a fake chat client, or a stub model
behind the metering proxy. No provider is called."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import ledger
import provider
import refine_live
import secret
import vb
from refine_live import LiveModel, parse_additions
from specops import refine
from stub_provider import ErrorStatus, StubServer

import speclint

ROW = ledger.load_snapshot().row("gpt-oss-120b")
FIXTURE = Path(speclint.__file__).resolve().parent / "fixtures" / "sq09-non-goals" / "tasks.toml"
NON_GOAL = {"field": "non_goals", "item": "Do not change any file outside the task's files.", "source": "files"}


def fixture_task() -> tuple[speclint.PlanContext, dict]:
    pairs, errors = speclint.plan_tasks([FIXTURE], vb.layout.REPO_ROOT)
    assert not errors and pairs
    return pairs[0]


class FakeChat:
    """A chat client that plays `replies` in order: a Completion, or an exception to raise. `seen` holds each call."""

    def __init__(self, *replies: object) -> None:
        self.replies = list(replies)
        self.seen: list[dict] = []

    def complete(self, messages, *, model, max_tokens, timeout_s):
        self.seen.append({"messages": messages, "model": model, "max_tokens": max_tokens})
        reply = self.replies.pop(0) if len(self.replies) > 1 else self.replies[0]
        if isinstance(reply, Exception):
            raise reply
        return reply


def completion(additions: list[dict], *, prompt: int = 1000, out: int = 200, usage: bool = True) -> provider.Completion:
    return provider.Completion(content=json.dumps({"additions": additions}), model_reported="gpt-oss-120b",
                               usage=provider.Usage(prompt, out) if usage else None, finish_reason="stop")


def test_parse_additions_reads_the_reply_and_drops_malformed_items():
    text = "Here you go:\n```json\n" + json.dumps({"additions": [
        NON_GOAL,
        {"field": "acceptance", "item": {"not": "a string"}, "source": "description"},  # dropped
        {"field": "verify", "item": {"phase": "test"}, "source": "description"},  # no command: dropped
        {"field": "context.read_files", "item": "src/app.py", "source": "files"},  # a bare path becomes a table
        {"field": "files", "item": "extra.py", "source": "title"},  # kept: refine rejects the round
        {"field": "assumptions", "item": "  Paths are POSIX.  "},  # no source: kept as "", refine rejects it
        "not an entry",  # dropped
    ]}) + "\n```\nDone."
    additions, dropped, problem = parse_additions(text)
    assert problem is None and dropped == 3
    assert [(a.field, a.item, a.source) for a in additions] == [
        ("non_goals", NON_GOAL["item"], "files"), ("context.read_files", {"path": "src/app.py"}, "files"),
        ("files", "extra.py", "title"), ("assumptions", "Paths are POSIX.", "")]
    assert parse_additions('{"additions": []}') == ([], 0, None)
    assert parse_additions("I would add a non-goal.")[2] == "the reply holds no JSON object with an additions list"
    assert parse_additions('{"changes": []}')[2] is not None


def test_the_live_model_proposes_what_the_reply_holds_and_books_its_usage():
    ctx, task = fixture_task()
    chat = FakeChat(completion([NON_GOAL]), completion([]))
    model = LiveModel(chat, model="gpt-oss-120b", price_row=ROW, budget_usd=0.02, ctx=ctx, sleep=lambda s: None)
    result = refine.refine(task, model, ctx)
    assert len(chat.seen) == model.calls == 2 and result.stopped == "no_proposal" and model.stop is None
    assert len(result.rounds) == 1 and result.spec["non_goals"][-1] == NON_GOAL["item"]
    first = chat.seen[0]["messages"]
    assert first[0]["content"] == refine_live.SYSTEM_PROMPT and "Round 1 of at most 2" in first[1]["content"]
    assert json.loads(first[1]["content"].split("The spec:\n", 1)[1]) == task
    assert model.usage == ledger.add_usage(ledger.vb_usage(1000, 200), ledger.vb_usage(1000, 200))
    assert model.spent_bound_usd == pytest.approx(2 * ledger.price(ledger.vb_usage(1000, 200), ROW).api_equiv_usd)
    assert [proposal["round"] for proposal in model.proposals] == [1, 2] and model.model_reported == "gpt-oss-120b"


def test_the_live_model_never_passes_the_tasks_budget():
    ctx, task = fixture_task()
    probe = LiveModel(FakeChat(completion([])), model="gpt-oss-120b", price_row=ROW, budget_usd=1.0, ctx=ctx)
    messages = probe.messages(task, 1)
    bound = refine_live.PREAMBLE_TOKENS + sum(len(m["content"].encode()) + refine_live.MESSAGE_OVERHEAD_TOKENS
                                              for m in messages)
    worst = ledger.worst_case_usd(ROW, input_tokens=bound, output_tokens=refine_live.MAX_TOKENS)
    # The first call bills its whole worst case, so a second one would pass a budget of 1.5 of it.
    chat = FakeChat(completion([NON_GOAL], prompt=bound, out=refine_live.MAX_TOKENS))
    model = LiveModel(chat, model="gpt-oss-120b", price_row=ROW, budget_usd=1.5 * worst, ctx=ctx)
    result = refine.refine(task, model, ctx)
    assert model.calls == 1 and len(result.rounds) == 1 and model.stop.startswith("budget: ")
    assert model.spent_bound_usd <= 1.5 * worst
    # A budget below one call's worst case makes no call at all.
    starved = LiveModel(FakeChat(completion([NON_GOAL])), model="gpt-oss-120b", price_row=ROW,
                        budget_usd=worst / 2, ctx=ctx)
    assert refine.refine(task, starved, ctx).rounds == [] and starved.calls == 0 and starved.stop.startswith("budget")


def test_a_retryable_error_is_retried_and_every_call_is_counted():
    ctx, task = fixture_task()
    waits: list[float] = []
    busy = provider.ProviderError("HTTP 429 from cerebras: slow down", retryable=True, billed_unknown=False,
                                  retry_after=3.0)
    chat = FakeChat(busy, completion([]))
    model = LiveModel(chat, model="gpt-oss-120b", price_row=ROW, budget_usd=0.02, ctx=ctx, sleep=waits.append)
    assert refine.refine(task, model, ctx).stopped == "no_proposal"
    assert model.calls == 2 and waits == [3.0] and not model.usage_unknown
    assert [entry["event"] for entry in model.transcript] == ["provider_error", "reply"]
    # A fatal error stops at once; one that may have been billed makes the usage unknown, at its worst case.
    lost = provider.ProviderError("timed out", retryable=False, billed_unknown=True)
    model = LiveModel(FakeChat(lost), model="gpt-oss-120b", price_row=ROW, budget_usd=0.02, ctx=ctx)
    assert refine.refine(task, model, ctx).rounds == [] and model.calls == 1
    assert model.reported_usage() is None and model.spent_bound_usd > 0 and model.stop.startswith("provider_error")


def test_a_reply_without_usage_counts_its_worst_case():
    ctx, task = fixture_task()
    model = LiveModel(FakeChat(completion([], usage=False)), model="gpt-oss-120b", price_row=ROW, budget_usd=0.02,
                      ctx=ctx)
    refine.refine(task, model, ctx)
    assert model.usage_unknown and model.reported_usage() is None and model.spent_bound_usd > 0


def test_a_proposal_under_a_list_is_rejected_not_raised():
    ctx, task = fixture_task()
    bad = {"field": "acceptance.extra", "item": "AC9: x", "source": "description"}
    model = LiveModel(FakeChat(completion([bad])), model="gpt-oss-120b", price_row=ROW, budget_usd=0.02, ctx=ctx)
    result = refine.refine({**task, "acceptance": ["AC1: the call returns 0"]}, model, ctx)
    assert result.rounds == [] and "no list there" in result.stopped


def respond(body: dict) -> str:
    """The stub model: a non-goal in round 1, nothing in round 2."""
    round_one = "Round 1 of" in body["messages"][1]["content"]
    return json.dumps({"additions": [NON_GOAL] if round_one else []})


def refine_args(tmp_path: Path, url: str, *extra: str, run_id: str = "t1") -> list[str]:
    return ["refine", "--provider-url", url, "--proxy", "--results", str(tmp_path / "results"), "--tasks", "4",
            "--run-id", run_id, *extra]


def test_vb_refine_runs_the_sample_through_the_proxy_and_the_ledger(tmp_path, capsys):
    with StubServer(respond) as stub:
        assert vb.main(refine_args(tmp_path, stub.url)) == 0
    run_dir = tmp_path / "results" / "REFINE-COST" / "t1"
    rows = [json.loads(line) for line in (run_dir / "refine.jsonl").read_text().splitlines()]
    booked = [json.loads(line) for line in (run_dir / "ledger.jsonl").read_text().splitlines()]
    calls = [json.loads(line) for line in (run_dir / "proxy.jsonl").read_text().splitlines()]
    manifest = json.loads((run_dir / "manifest.json").read_text())
    assert [row["group"] for row in rows] == ["plans", "plans", "fixtures", "fixtures"]
    assert [entry["spec_hash"] for entry in manifest["sample"]] == [row["before"]["spec_hash"] for row in rows]
    assert all(row["before"]["band"] in ("C", "D") for row in rows)
    for row, booked_row in zip(rows, booked):
        assert row["calls"] == 2 and row["stopped"] == "no_proposal" and len(row["rounds"]) == 1
        assert row["after"]["score"] >= row["before"]["score"] and row["proposals"][0]["kept"]
        assert (booked_row["line"], booked_row["experiment_id"], booked_row["billed"]) == ("BL15", "REFINE-COST", True)
        assert booked_row["attempt_key"] == row["attempt_key"] and booked_row["reserved_usd"] == 0.02
        assert booked_row["api_equiv_usd"] == row["api_equiv_usd"] > 0 and row["meter_usd"] == pytest.approx(
            row["api_equiv_usd"])
    assert len(stub.requests) == len(calls) == 8 and {body["model"] for body in stub.requests} == {"gpt-oss-120b"}
    assert {call["task"] for call in calls} == {row["attempt_key"] for row in rows}
    assert all(call["forwarded"] and call["status"] == 200 for call in calls)
    assert all("authorization" not in headers for headers in stub.headers)  # an override URL never gets a key
    summary = (run_dir / "summary.md").read_text()
    assert "within the $0.02 cap: yes" in summary and "2 from fixtures, 2 from plans" in summary
    assert "**Hard fails**" in summary and "| Hard fails |" in summary
    capsys.readouterr()
    assert vb.main(["ledger", "report", "--results", str(tmp_path / "results"), "--json"]) == 0
    report = json.loads(capsys.readouterr().out)
    line = next(entry for entry in report["lines"] if entry["id"] == "BL15")
    assert line["spent_usd"] == pytest.approx(sum(row["api_equiv_usd"] for row in rows), abs=1e-6)
    assert line["reserved_usd"] == 0 and line["rows"] == 4
    bundle = tmp_path / "bundle"
    assert vb.main(["refine", "--summary", str(run_dir), "--bundle", str(bundle)]) == 0
    assert sorted(path.name for path in bundle.iterdir()) == ["ledger.jsonl", "manifest.json", "refine.jsonl",
                                                              "summary.md"]
    assert (bundle / "summary.md").read_text() == summary


def test_refused_calls_book_zero_and_never_a_yes(tmp_path, capsys):
    """The first live run's shape (bug-7b1b7f): every call refused with a 403 billed nothing, so each task's ledger row
    is a known $0, never an unknown cost charged at its $0.02 reservation (the direct loop's bug-2a5566), and a sample
    the model never answered gets no `yes`."""
    with StubServer(lambda body: ErrorStatus(status=403)) as stub:
        assert vb.main(refine_args(tmp_path, stub.url, run_id="r403")) == 0
    run_dir = tmp_path / "results" / "REFINE-COST" / "r403"
    rows = [json.loads(line) for line in (run_dir / "refine.jsonl").read_text().splitlines()]
    booked = [json.loads(line) for line in (run_dir / "ledger.jsonl").read_text().splitlines()]
    assert len(rows) == len(booked) == len(stub.requests) == 4
    assert all(row["calls"] == 1 and row["replies"] == 0 and row["stopped"].startswith("provider_error: HTTP 403")
               for row in rows)
    assert {(entry["api_equiv_usd"], entry["billed_usd"], entry["source"]) for entry in booked} == {
        (0.0, 0.0, "provider_usage")}
    assert "within the $0.02 cap: no" in (run_dir / "summary.md").read_text()
    capsys.readouterr()
    assert vb.main(["ledger", "report", "--results", str(tmp_path / "results"), "--json"]) == 0
    line = next(entry for entry in json.loads(capsys.readouterr().out)["lines"] if entry["id"] == "BL15")
    assert (line["spent_usd"], line["reserved_usd"], line["rows"]) == (0, 0, 4)


def test_only_the_proxy_sends_the_key(tmp_path):
    key = "sk-test-" + "k" * 32
    key_file = secret.create_keys(tmp_path / "config" / "keys", {"CEREBRAS_API_KEY": key})
    with StubServer(respond) as stub:
        arm = tmp_path / "arm.toml"
        text = (vb.layout.ARMS_DIR / "cheap_direct.toml").read_text()
        arm.write_text(text.replace('base_url = "https://api.cerebras.ai/v1"', f'base_url = "{stub.url}"'))
        assert vb.main(["refine", "--arm", str(arm), "--proxy", "--key-file", str(key_file), "--results",
                        str(tmp_path / "results"), "--tasks", "2", "--run-id", "k1"]) == 0
    assert {headers.get("authorization") for headers in stub.headers} == {f"Bearer {key}"}
    run_dir = tmp_path / "results" / "REFINE-COST" / "k1"
    for path in run_dir.iterdir():
        assert key not in path.read_text(), path.name


def test_vb_refine_admits_a_network_provider_only_with_both_flags(tmp_path, capsys):
    results = tmp_path / "results"
    for extra, words in (([], "--allow-network"), (["--allow-network"], "--max-cost-usd"),
                         (["--allow-network", "--max-cost-usd", "0.01"], "more than --max-cost-usd")):
        assert vb.main(["refine", "--results", str(results), "--tasks", "2", *extra]) == 2
        assert words in capsys.readouterr().err
    assert not results.exists()


def test_vb_refine_refuses_a_task_that_could_pass_the_line_cap(tmp_path, capsys):
    results = tmp_path / "results"
    earlier = ledger.Ledger(results / "REFINE-COST" / "old" / "ledger.jsonl", line="BL15", experiment_id="REFINE-COST",
                            run_id="old", price_snapshot_id="prices-2026-09-28")
    earlier.path.parent.mkdir(parents=True)
    earlier.append(attempt_key="old/01:1", provider="cerebras", model_reported="gpt-oss-120b",
                   usage=ledger.vb_usage(1000, 100), cost=ledger.Cost(0.385, 0.385, "provider_usage"), billed=True,
                   reserved_usd=0.02)
    with StubServer(respond) as stub:
        assert vb.main(refine_args(tmp_path, stub.url)) == 2
    assert "line BL15" in capsys.readouterr().err and stub.requests == []
    assert not (results / "REFINE-COST" / "t1").exists()


def test_the_dry_run_draws_half_from_each_group_the_same_way_every_time(capsys):
    def sample(*extra: str) -> list[dict]:
        assert vb.main(["refine", "--dry-run", *extra]) == 0
        return json.loads(capsys.readouterr().out)["sample"]

    first = sample()
    assert len(first) == 20 and [entry["group"] for entry in first] == ["plans"] * 10 + ["fixtures"] * 10
    assert {entry["band"] for entry in first} <= {"C", "D"} and sample() == first
    assert all(not entry["plan_path"].startswith("plans/") or "archive" not in entry["plan_path"] for entry in first)
    assert sample("--seed", "1") != first
    assert vb.main(["refine", "--dry-run", "--tasks", "3"]) == 2
