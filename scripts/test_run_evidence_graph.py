#!/usr/bin/env python3
"""End-to-end checks that evidence bundles from Graph plan runs validate.

Each end-to-end test runs a prebuilt roko (ROKO_BIN, default target/debug/roko)
on a fixture plan in a throwaway workspace whose claude_cli provider is the
deterministic plans/portal-programme/_harness/fake-claude, so a run takes
seconds and costs nothing. Nothing here builds roko; gap-09e478's verify builds
it first. The collector-level tests need no binary.

    python3 scripts/test_run_evidence_graph.py [-k PATTERN]
"""

from __future__ import annotations

import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

REPO = pathlib.Path(__file__).resolve().parents[1]
COLLECTOR = REPO / "scripts" / "run_evidence.py"
FAKE_CLAUDE = REPO / "plans" / "portal-programme" / "_harness" / "fake-claude"
ROKO_BIN = pathlib.Path(os.environ.get("ROKO_BIN") or REPO / "target" / "debug" / "roko")
AGENT_TEXT = "Wrote the requested artifacts."

_spec = importlib.util.spec_from_file_location("run_evidence", COLLECTOR)
assert _spec is not None and _spec.loader is not None
run_evidence = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(run_evidence)

ROKO_TOML = """config_version = 2
schema_version = 2

[agent]
default_model = "claude-sonnet-4-6"

[providers.claude_cli]
kind = "claude_cli"
command = "{fake}"

[models.claude-sonnet-4-6]
provider = "claude_cli"
slug = "claude-sonnet-4-6"
context_window = 200000

[serve.auth]
enabled = false

[gates]
max_review_cycles = 0
cargo_fix_enabled = false
"""

PLAN_TOML = """[meta]
plan = "{plan}"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T01"
title = "Fixture task"
description = "{description} ARTIFACT out/{plan}.txt"
status = "ready"
role = "implementer"
tier = "focused"
model_hint = "claude-sonnet-4-6"
max_retries = 0
timeout_secs = 120
files = ["out/{plan}.txt"]
depends_on = []

[task.context]
read_files = [{{ path = "README.md", why = "fixture" }}]

[[task.verify]]
phase = "structural"
command = "{verify}"
fail_msg = "verify failed"
timeout_ms = {timeout_ms}
"""

GIT_IDENTITY = {
    "GIT_AUTHOR_NAME": "evidence-test",
    "GIT_AUTHOR_EMAIL": "evidence-test@roko",
    "GIT_COMMITTER_NAME": "evidence-test",
    "GIT_COMMITTER_EMAIL": "evidence-test@roko",
}


def read_json(path: pathlib.Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def write_plan(ws: pathlib.Path, plan: str, description: str, verify: str, timeout_ms: int = 60_000) -> None:
    directory = ws / "plans" / plan
    directory.mkdir(parents=True)
    (directory / "tasks.toml").write_text(
        PLAN_TOML.format(plan=plan, description=description, verify=verify, timeout_ms=timeout_ms),
        encoding="utf-8",
    )
    (directory / "plan.md").write_text(f"# {plan}\n\nEvidence fixture.\n", encoding="utf-8")


class GraphRunCase(unittest.TestCase):
    """A throwaway workspace per test, with the fixture plans committed."""

    @classmethod
    def setUpClass(cls) -> None:
        if not (ROKO_BIN.is_file() and os.access(ROKO_BIN, os.X_OK)):
            raise AssertionError(f"missing {ROKO_BIN}: run `cargo build -p roko-cli` or set ROKO_BIN")

    def setUp(self) -> None:
        self.ws = pathlib.Path(tempfile.mkdtemp(prefix="roko-evidence-")).resolve()
        self.addCleanup(shutil.rmtree, self.ws, True)
        shutil.copy(FAKE_CLAUDE, self.ws / "fake-claude")
        (self.ws / "fake-claude").chmod(0o755)
        (self.ws / ".roko").mkdir()
        (self.ws / "README.md").write_text("# evidence fixture\n", encoding="utf-8")
        (self.ws / ".gitignore").write_text(".roko/\nout/\n", encoding="utf-8")
        (self.ws / "roko.toml").write_text(ROKO_TOML.format(fake=self.ws / "fake-claude"), encoding="utf-8")
        write_plan(self.ws, "one", "Write the artifact.", "test -f out/one.txt")
        write_plan(self.ws, "early", "Crash before any output. EXIT_EARLY 3", "test -f out/early.txt")
        write_plan(self.ws, "slowgate", "Write the artifact.", "sleep 30", timeout_ms=1500)
        self.env = {**os.environ, **GIT_IDENTITY}
        for name in ("ROKO_EVIDENCE_RUN_ID", "ROKO_EVIDENCE_BUNDLE", "ROKO_CONFIG"):
            self.env.pop(name, None)
        for argv in (["git", "init", "-q"], ["git", "add", "-A"], ["git", "commit", "-qm", "fixture"]):
            subprocess.run(argv, cwd=self.ws, env=self.env, check=True, capture_output=True)

    def collect(self, plan: str, *native: str) -> tuple[subprocess.CompletedProcess[str], pathlib.Path]:
        """Run `roko plan run plans/<plan>` under the collector, as FAST does."""
        result = subprocess.run(
            [
                sys.executable, str(COLLECTOR), "--require-events", "--deadline", "120", "--label", plan,
                "--", str(ROKO_BIN), "plan", "run", f"plans/{plan}", "--no-tui", "--max-retries", "0",
                "--log-file", "{bundle}/events.jsonl", *native,
            ],
            cwd=self.ws,
            env=self.env,
            capture_output=True,
            text=True,
            timeout=300,
        )
        return result, self.bundle_from(result)

    def bundle_from(self, result: subprocess.CompletedProcess[str]) -> pathlib.Path:
        for line in result.stderr.splitlines():
            if line.startswith("[evidence] bundle="):
                return pathlib.Path(line.split("=", 1)[1])
        self.fail(f"no bundle announced; stderr:\n{result.stderr[-3000:]}")

    def assert_validates(self, bundle: pathlib.Path) -> dict:
        """The check `./dev.sh evidence-validate` runs, plus the stored result."""
        cli = subprocess.run(
            [sys.executable, str(COLLECTOR), "validate", str(bundle), "--require-events"],
            capture_output=True,
            text=True,
        )
        validation = json.loads(cli.stdout)
        self.assertEqual(cli.returncode, 0, validation["errors"])
        self.assertTrue(read_json(bundle / "validation.json")["valid"])
        return validation

    def graph_plan(self, bundle: pathlib.Path, plan_id: str) -> dict:
        plans = [plan for plan in read_json(bundle / "graph" / "index.json")["plans"] if plan["plan_id"] == plan_id]
        self.assertEqual(len(plans), 1, f"one Graph record for {plan_id}")
        return plans[0]

    def assert_one_lifecycle(self, bundle: pathlib.Path, outcome: str) -> None:
        facts = read_json(bundle / "events-validation.json")
        self.assertEqual((facts["run_start_count"], facts["run_terminal_count"]), (1, 1))
        self.assertEqual(facts["run_ids"], [read_json(bundle / "manifest.json")["run_id"]])
        terminals = [row for row in read_jsonl(bundle / "events.jsonl") if row["type"] == "run.completed"]
        self.assertEqual([row["outcome"] for row in terminals], [outcome])

    def assert_derived_only(self, bundle: pathlib.Path) -> None:
        """Graph evidence and the event log hold no agent output, and the metadata no home path."""
        home = str(pathlib.Path.home())
        for path in [bundle / "events.jsonl", *sorted((bundle / "graph").rglob("*"))]:
            if path.is_file():
                self.assertNotIn(AGENT_TEXT, path.read_text(encoding="utf-8"), path.name)
        for name in ("manifest.json", "command.txt", "summary.json", "commands.jsonl", "graph/index.json"):
            if len(home) > 1:
                self.assertNotIn(home + "/", (bundle / name).read_text(encoding="utf-8"), name)


class GraphBundleScenarios(GraphRunCase):
    def test_one_task_success_bundle_validates_with_graph_state(self) -> None:
        result, bundle = self.collect("one")
        self.assertEqual(result.returncode, 0, result.stderr[-3000:])
        self.assertEqual(read_json(bundle / "summary.json")["state"], "succeeded")
        self.assert_validates(bundle)
        self.assert_one_lifecycle(bundle, "succeeded")

        plan = self.graph_plan(bundle, "one")
        self.assertEqual((plan["status"], plan["mode"]), ("succeeded", "new"))
        self.assertEqual(plan["verdicts"], {"passed": 1})
        rows = read_jsonl(bundle / plan["activities"]["artifact"])
        self.assertGreaterEqual(len(rows), 1)
        self.assertEqual({row["run_id"] for row in rows}, {plan["run_id"]})
        self.assertEqual({row["node_id"] for row in rows}, {"T01"})
        self.assertEqual(read_json(bundle / plan["checkpoint"]["artifact"])["run_id"], plan["run_id"])
        self.assertEqual(read_json(bundle / plan["costs"]["artifact"])["run_id"], plan["run_id"])
        self.assertEqual(plan["diagnose"]["command_exit_code"], 0)
        self.assertEqual(read_json(bundle / plan["diagnose"]["artifact"])["plan_id"], "one")
        self.assertEqual(read_json(bundle / "metrics.json")["verification"]["gates_passed"], 1)
        self.assert_derived_only(bundle)

    def test_agent_exit_before_first_event_fails_with_a_diagnosis(self) -> None:
        result, bundle = self.collect("early")
        self.assertNotEqual(result.returncode, 0)
        summary = read_json(bundle / "summary.json")
        self.assertEqual(summary["state"], "failed")
        self.assert_validates(bundle)
        self.assert_one_lifecycle(bundle, "failed")
        self.assertFalse(read_json(bundle / "score.json")["targets"]["terminal_succeeded"])
        self.assertIn("exit-early 3", (self.ws / ".roko" / "fake-claude.log").read_text(encoding="utf-8"))

        failure = read_json(bundle / "metrics.json")["first_failure"]
        self.assertEqual((failure["failure_kind"], failure["task_id"]), ("failed_before_gate", "T01"))
        plan = self.graph_plan(bundle, "early")
        self.assertEqual(plan["status"], "failed")
        self.assertEqual(plan["activities"]["lines_selected"], 0)
        self.assertEqual(plan["failed_task"]["task_id"], "T01")
        self.assertIn("exit 3", plan["failed_task"]["last_error"])
        diagnosis = read_json(bundle / plan["diagnose"]["artifact"])
        self.assertEqual(diagnosis["status"], "failed")
        self.assert_derived_only(bundle)

    def test_gate_timeout_timing_and_kind_agree_and_the_ledger_is_checked(self) -> None:
        result, bundle = self.collect("slowgate")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(read_json(bundle / "summary.json")["state"], "failed")
        validation = self.assert_validates(bundle)
        self.assert_one_lifecycle(bundle, "failed")

        metrics = read_json(bundle / "metrics.json")
        failure = metrics["first_failure"]
        self.assertEqual((failure["failure_kind"], failure["timeout_ms"]), ("timeout", 1500))
        self.assertEqual((metrics["provider"]["timeouts"], metrics["verification"]["gates_failed"]), (1, 1))
        gates = metrics["events"]["graph_gates"]
        self.assertEqual(len(gates), 1)
        # The gate ran for its timeout and was cut well before the 30 s sleep.
        self.assertGreaterEqual(gates[0]["duration_ms"], 1500)
        self.assertLess(gates[0]["duration_ms"], 30_000)
        self.assertLess(metrics["latency_ms"]["command"], 30_000)

        index = read_json(bundle / "graph" / "index.json")
        ledger_rows = read_jsonl(bundle / "graph" / "ledgers" / "gate-failures.jsonl")
        task_rows = [row for row in ledger_rows if (row["plan_id"], row["task_id"]) == ("slowgate", "T01")]
        self.assertGreaterEqual(len(task_rows), 1, "the gate failure reaches the ledger")
        [check] = index["gate_timeouts"]
        kinds = sorted({row["failure_kind"] for row in task_rows})
        self.assertEqual(check["event_timeout_ms"], 1500)
        self.assertEqual(check["ledger_failure_kinds"], kinds)
        self.assertEqual(check["agrees"], kinds == ["timeout"])
        # A disagreement is never hidden: the validator names it.
        named = any("slowgate/T01 timed out after 1500 ms" in warning for warning in validation["warnings"])
        self.assertEqual(named, not check["agrees"])
        self.assert_derived_only(bundle)

    def test_bundles_hold_only_their_own_run(self) -> None:
        roko = self.ws / ".roko"
        (roko / "state").mkdir()
        (roko / "state" / "status.json").write_text(json.dumps({"run_id": "old-run", "phase": "running"}))
        with (roko / "events.jsonl").open("w", encoding="utf-8") as stream:
            for kind in ("run.started", "run.completed"):
                stream.write(json.dumps({"type": kind, "run_id": "old-run", "timestamp_ms": 1}) + "\n")

        first_result, first = self.collect("one")
        resumed_result, resumed = self.collect("one")
        fresh_result, fresh = self.collect("one", "--fresh")
        for result in (first_result, resumed_result, fresh_result):
            self.assertEqual(result.returncode, 0, result.stderr[-3000:])
        first_plan, resumed_plan, fresh_plan = (self.graph_plan(bundle, "one") for bundle in (first, resumed, fresh))

        # A resume keeps the checkpoint's run and slices only what it appended:
        # none of the records the first run wrote.
        self.assertEqual(resumed_plan["mode"], "resumed")
        self.assertEqual(resumed_plan["run_id"], first_plan["run_id"])
        self.assertGreater(resumed_plan["activities"]["offset"], 0)

        def signal_ids(bundle: pathlib.Path, plan: dict) -> set[str]:
            rows = read_jsonl(bundle / plan["activities"]["artifact"])
            return {item["id"] for row in rows for item in row["signals"]}

        self.assertTrue(signal_ids(first, first_plan))
        self.assertFalse(signal_ids(first, first_plan) & signal_ids(resumed, resumed_plan))
        # --fresh truncates the Activity log under a new run ID; the slice
        # starts over and holds that run only, even at the same byte size.
        self.assertEqual(fresh_plan["mode"], "fresh")
        self.assertNotEqual(fresh_plan["run_id"], first_plan["run_id"])
        fresh_rows = read_jsonl(fresh / fresh_plan["activities"]["artifact"])
        self.assertGreaterEqual(len(fresh_rows), 1)
        self.assertEqual({row["run_id"] for row in fresh_rows}, {fresh_plan["run_id"]})

        for bundle in (first, resumed, fresh):
            self.assert_validates(bundle)
            self.assert_one_lifecycle(bundle, "succeeded")
            # The run's own status.json revisions are sampled (gap-568056),
            # never the stale file of the old run.
            run_id = read_json(bundle / "manifest.json")["run_id"]
            samples = read_jsonl(bundle / "status-samples.jsonl")
            self.assertTrue(samples, "the Graph run wrote no status.json revision")
            self.assertEqual({sample["source_run_id"] for sample in samples}, {run_id})
            self.assertEqual(samples[-1]["status"]["phase"], "completed")
            sources = read_json(bundle / "filtered-logs" / "index.json")["sources"]
            events_log = [source for source in sources if source["source"].endswith(".roko/events.jsonl")]
            self.assertEqual([source["lines_selected"] for source in events_log], [0])
            sampling = read_json(bundle / "summary.json")["collection"]["status_sampling"]
            self.assertEqual(sampling["state"], "sampled")


class FastWrapper(GraphRunCase):
    def test_dev_fast_bundle_validates(self) -> None:
        # dev.sh runs from its own directory, so linking it and the collector
        # into the workspace makes `./dev.sh fast` run the workspace's plan.
        (self.ws / "scripts").mkdir()
        (self.ws / "scripts" / "run_evidence.py").symlink_to(COLLECTOR)
        (self.ws / "dev.sh").symlink_to(REPO / "dev.sh")
        result = subprocess.run(
            ["./dev.sh", "fast", "--min-free-gib", "0", "--min-free-percent", "0", "plans/one"],
            cwd=self.ws,
            env={**self.env, "ROKO_FAST_BIN": str(ROKO_BIN)},
            capture_output=True,
            text=True,
            timeout=300,
        )
        self.assertEqual(result.returncode, 0, result.stderr[-3000:])
        bundle = self.bundle_from(result)
        self.assertTrue(bundle.name.split("-", 1)[1].startswith("roko-fast"))
        manifest = read_json(bundle / "manifest.json")
        self.assertTrue(manifest["requirements"]["events"])
        self.assertEqual(manifest["environment"]["safe_values"]["ROKO_FAST_MODE"], "1")
        self.assert_validates(bundle)
        self.assert_one_lifecycle(bundle, "succeeded")
        self.assertEqual(self.graph_plan(bundle, "one")["status"], "succeeded")
        self.assert_derived_only(bundle)


class CollectorUnits(unittest.TestCase):
    def test_unhome_rewrites_whole_home_components_only(self) -> None:
        with mock.patch.dict(os.environ, {"HOME": "/Users/ada"}):
            self.assertEqual(
                run_evidence.unhome('{"cwd": "/Users/ada/w", "other": "/Users/adam/w", "home": "/Users/ada"}'),
                '{"cwd": "~/w", "other": "/Users/adam/w", "home": "~"}',
            )

    def test_activity_rows_keep_derived_facts_only(self) -> None:
        row = run_evidence.graph_activity_row(
            {
                "graph_id": "p",
                "run_id": "graph-p-1",
                "node_id": "T01",
                "tick": 0,
                "signals": [
                    {
                        "id": "s1",
                        "kind": "agent_output",
                        "body": {"format": "text", "data": "agent transcript text"},
                        "tags": {"model": "m", "roko.gate.verdict": "passed", "nested": {"x": 1}},
                    }
                ],
            }
        )
        self.assertNotIn("agent transcript text", json.dumps(row))
        [signal_row] = row["signals"]
        self.assertEqual(signal_row["tags"], {"model": "m", "roko.gate.verdict": "passed"})
        self.assertEqual((signal_row["body_format"], signal_row["body_bytes"]), ("text", 23))

    def test_gate_timeouts_are_read_from_the_logged_excerpt(self) -> None:
        """The --log-file keeps a gate's redacted output tail, not its output (bug-4c4eea)."""

        def gate_result(task_id: str, **output: object) -> dict:
            event = {"type": "gate_result", "plan_id": "p", "task_id": task_id, "gate": "verify[0]", "passed": False}
            return {"type": "dashboard.gate_result", "run_id": "r", "seq": 1, "ts_millis": 1, "event": {**event, **output}}

        rows = [
            gate_result(
                "T01",
                output_text_bytes=900,
                output_text_lines=40,
                output_text_sha256="0" * 64,
                output_text_excerpt="\u2026line 40\n\u2717 timed out after 1500 ms",
            ),
            # A log written before the fix holds the whole output.
            gate_result("T02", output_text="$ sleep 9\n\u2717 timed out after 2500 ms"),
        ]
        with tempfile.TemporaryDirectory() as raw:
            events = pathlib.Path(raw) / "events.jsonl"
            events.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            metrics = run_evidence.event_metrics(events)
        self.assertEqual([gate["timeout_ms"] for gate in metrics["graph_gates"]], [1500, 2500])
        self.assertEqual((metrics["gate_failed"], metrics["timeouts"]), (2, 2))

    def test_fresh_run_of_equal_size_is_sliced_from_the_start(self) -> None:
        """A byte offset alone would skip a fresh log that regrew to the old size."""
        with tempfile.TemporaryDirectory() as raw:
            cwd = pathlib.Path(raw)
            plan_dir = cwd / run_evidence.GRAPH_STATE_DIR / "p"
            plan_dir.mkdir(parents=True)

            def record(run_id: str) -> None:
                (plan_dir / "checkpoint.json").write_text(json.dumps({"plan_id": "p", "run_id": run_id, "status": "succeeded"}))
                (plan_dir / "activities.jsonl").write_text(
                    json.dumps({"graph_id": "p", "run_id": run_id, "node_id": "T01", "tick": 0, "signals": []}) + "\n"
                )

            record("graph-p-aaaa")
            baselines = run_evidence.graph_state_baselines(cwd)
            record("graph-p-bbbb")
            bundle = cwd / "bundle"
            bundle.mkdir()
            index = run_evidence.collect_graph_state(
                bundle, cwd, baselines, run_evidence.ledger_baselines(cwd), {}, None, {}, 1.0
            )
            [plan] = index["plans"]
            self.assertEqual((plan["mode"], plan["activities"]["offset"]), ("fresh", 0))
            self.assertEqual([row["run_id"] for row in read_jsonl(bundle / plan["activities"]["artifact"])], ["graph-p-bbbb"])
            self.assertEqual(plan["diagnose"], {"skipped_reason": "the command is not a `roko plan run`"})


if __name__ == "__main__":
    unittest.main()
