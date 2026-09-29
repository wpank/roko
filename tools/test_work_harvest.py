#!/usr/bin/env python3
"""Tests for tools/work_harvest.py: joining transcript calls to work items, counting each call once, keeping no
message content, and incremental runs. The transcripts are synthetic, in a temp dir.

Run: python3 tools/test_work_harvest.py
"""

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work_harvest as wh  # noqa: E402

TODAY = "2026-09-29"


def call(mid, session, branch, *, out=10, agent=None, entrypoint=None, ts=f"{TODAY}T10:00:00.000Z", text="Working on it.",
         content=None, model="claude-opus-5-5"):
    """One line of API call `mid`: 100 input, 1,000 cache-read and 50 cache-write tokens (10 of them 1-hour)."""
    line = {"parentUuid": None, "isSidechain": agent is not None, "userType": "external", "cwd": "/repo",
            "sessionId": session, "version": "2.1.282", "gitBranch": branch, "type": "assistant", "uuid": f"{mid}-{out}",
            "timestamp": ts, "requestId": f"req-{mid}",
            "message": {"id": mid, "type": "message", "role": "assistant", "model": model,
                        "content": content or [{"type": "text", "text": text}],
                        "usage": {"input_tokens": 100, "cache_creation_input_tokens": 50, "cache_read_input_tokens": 1000,
                                  "cache_creation": {"ephemeral_5m_input_tokens": 40, "ephemeral_1h_input_tokens": 10},
                                  "output_tokens": out, "service_tier": "standard", "speed": "standard"}}}
    if agent:
        line["agentId"] = agent
    if entrypoint:
        line["entrypoint"] = entrypoint
    return line


def user(content, session, branch="main", *, agent=None, entrypoint=None, ts=f"{TODAY}T09:59:00.000Z", **extra):
    line = {"parentUuid": None, "isSidechain": agent is not None, "userType": "external", "cwd": "/repo",
            "sessionId": session, "version": "2.1.282", "gitBranch": branch, "type": "user", "uuid": f"u-{session}-{ts}",
            "timestamp": ts, "message": {"role": "user", "content": content}, **extra}
    if agent:
        line["agentId"] = agent
    if entrypoint:
        line["entrypoint"] = entrypoint
    return line


class HarvestTest(unittest.TestCase):
    """A Claude Code project directory in a temp dir, and the tool pointed at it."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.projects = self.root / "projects" / "-Users-x-roko"
        self.projects.mkdir(parents=True)
        self.out, self.state = self.root / "harvest", self.root / "state.json"

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, rel, lines, *, append=False, raw=""):
        p = self.projects / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        with p.open("a" if append else "w") as fh:
            fh.write("".join(json.dumps(x) + "\n" for x in lines) + raw)

    def session(self, sid, lines, **kw):
        self.write(f"{sid}.jsonl", lines, **kw)

    def subagent(self, sid, aid, lines, **kw):
        self.write(f"{sid}/subagents/agent-{aid}.jsonl", lines, **kw)

    def run_tool(self, *flags, out=None, state=None) -> str:
        with contextlib.redirect_stdout(io.StringIO()) as buf:
            wh.main(["--projects", str(self.projects), "--out", str(out or self.out), "--state", str(state or self.state),
                     "--today", TODAY, *flags])
        return buf.getvalue()

    def harvest(self, out=None, state=None):
        """Run the tool; return every row in its output directory."""
        self.run_tool(out=out, state=state)
        return [json.loads(x) for p in sorted((out or self.out).glob("*.jsonl")) for x in p.read_text().splitlines()]

    def calls(self, rows, **match):
        return [r for r in rows if r["row"] == "calls" and all(r[k] == v for k, v in match.items())]


class TestJoins(HarvestTest):
    def test_harvest_joins_calls_to_items_by_branch(self):
        b = "work/gap-aaaaaa"
        self.session("s1", [user("Pick up the next item.", "s1", b, entrypoint="cli"),
                            # streaming writes a line per content block: the call counts once, with its final output
                            call("m1", "s1", b, out=5, entrypoint="cli"), call("m1", "s1", b, out=5, entrypoint="cli"),
                            call("m1", "s1", b, out=20, entrypoint="cli"),
                            call("m2", "s1", "main", out=7, entrypoint="cli")])
        self.subagent("s1", "a1", [user("Check the tests.", "s1", b, agent="a1"),
                                   call("m3", "s1", b, out=11, agent="a1"), call("m3", "s1", b, out=11, agent="a1")])
        self.session("s2", [call("m4", "s2", b, out=13, entrypoint="cli")])
        rows = self.harvest()
        items = wh.item_totals(rows)
        self.assertEqual(list(items), ["gap-aaaaaa"])
        t = items["gap-aaaaaa"]
        self.assertEqual((t["calls"], t["output_tokens"], t["input_tokens"], t["cache_read_tokens"]), (3, 44, 300, 3000))
        self.assertEqual((t["cache_write_5m_tokens"], t["cache_write_1h_tokens"]), (120, 30))
        self.assertEqual((t["sessions"], t["agents"], t["joins"]), (2, 1, ["branch"]))
        # the call on main is overhead for its branch; the subagent inherits its session's entrypoint
        self.assertEqual([(r["branch"], r["calls"], r["output_tokens"], r["join"]) for r in self.calls(rows, item=None)],
                         [("main", 1, 7, "none")])
        self.assertEqual({(r["entrypoint"], r["origin"]) for r in self.calls(rows)}, {("cli", "operator")})

    def test_prompt_and_report_joins(self):
        # a worker in an isolation worktree, told its one item in the prompt
        self.subagent("s1", "a1", [user("Do exactly one task: work item `gap-bbbbbb`.", "s1", "worktree-agent-a1", agent="a1"),
                                   call("m1", "s1", "worktree-agent-a1", agent="a1")])
        # a worker given two items on one branch is joined to the item its branch is named after
        self.subagent("s1", "a2", [user("Do gap-cccccc and gap-dddddd on one branch, `work/gap-cccccc`.", "s1", agent="a2"),
                                   call("m2", "s1", "main", agent="a2")])
        # a worker whose prompt names no item, but whose report does
        self.subagent("s1", "a3", [user("Fix the flaky test.", "s1", agent="a3"),
                                   call("m3", "s1", "main", agent="a3", text="Done.\n\n`ITEM:` gap-eeeeee · `BRANCH:` x")])
        # an orchestrator whose prompt names two items, quoting a worker's report, is overhead
        self.session("s1", [user("Work on gap-ffffff and gap-111111.", "s1", entrypoint="cli"),
                            call("m4", "s1", "main", entrypoint="cli", text="ITEM: gap-222222 is merged.")])
        joins = {(r["agent"], r["item"], r["join"]) for r in self.calls(self.harvest())}
        self.assertEqual(joins, {("a1", "gap-bbbbbb", "prompt"), ("a2", "gap-cccccc", "prompt"),
                                 ("a3", "gap-eeeeee", "item-line"), (None, None, "none")})

    def test_origin_comes_from_the_entrypoint(self):
        self.session("s1", [call("m1", "s1", "plan/x", entrypoint="sdk-cli")])
        self.subagent("s1", "a1", [call("m2", "s1", "plan/x", agent="a1")])
        self.session("s2", [call("m3", "s2", "main")])  # Claude Code before 2.1.28x wrote no entrypoint
        origins = {(r["session"], r["sidechain"], r["entrypoint"], r["origin"]) for r in self.calls(self.harvest())}
        self.assertEqual(origins, {("s1", False, "sdk-cli", "roko"), ("s1", True, "sdk-cli", "roko"),
                                   ("s2", False, "unknown", "unknown")})

    def test_overhead_sums_a_sessions_subagents(self):
        self.subagent("s1", "a1", [user("Survey the crates.", "s1", agent="a1"), call("m1", "s1", "main", agent="a1")])
        self.subagent("s1", "a2", [user("Survey the docs.", "s1", agent="a2"), call("m2", "s1", "main", agent="a2")])
        rows = self.calls(self.harvest())
        self.assertEqual([(r["agent"], r["item"], r["sidechain"], r["calls"]) for r in rows], [(None, None, True, 2)])


class TestContent(HarvestTest):
    def test_harvest_keeps_no_message_content(self):
        """Text marked PLUM sits in every place a transcript holds content; none of it reaches the rows or the state."""
        b, w = "work/gap-bbbbbb", "worktree-agent-a1"
        answer = call("m1", "s1", b, entrypoint="cli", content=[
            {"type": "thinking", "thinking": "PLUM thinking", "signature": "PLUMsig"},
            {"type": "text", "text": "PLUM answer\nITEM: gap-bbbbbb"},
            {"type": "tool_use", "id": "t1", "name": "Bash", "input": {"command": "echo PLUM-tool-input"}}])
        answer.update(cwd="/home/PLUM", slug="PLUM-slug")
        report = {"status": "completed", "prompt": "PLUM agent prompt", "agentId": "a1", "agentType": "general-purpose",
                  "content": [{"type": "text", "text": "PLUM agent report"}], "totalDurationMs": 5000, "totalTokens": 99,
                  "totalToolUseCount": 3, "resolvedModel": "claude-sonnet-5", "usage": {"output_tokens": 1},
                  "toolStats": {"readCount": 1, "searchCount": 0, "bashCount": 2, "editFileCount": 1, "linesAdded": 10,
                                "linesRemoved": 2}}
        progress = {"type": "progress", "sessionId": "s1", "gitBranch": b, "timestamp": f"{TODAY}T10:01:00.000Z",
                    "data": {"type": "agent_progress", "prompt": "PLUM progress", "agentId": "a1",
                             "message": {"type": "assistant", "message": call("m2", "s1", w, text="PLUM streamed")["message"]}}}
        self.session("s1", [
            user("Fix gap-bbbbbb: PLUM prompt", "s1", b, entrypoint="cli"), answer,
            user([{"type": "tool_result", "tool_use_id": "t1", "content": "PLUM tool output"}], "s1", b,
                 toolUseResult={"stdout": "PLUM stdout", "stderr": "", "interrupted": False}),
            progress,
            user([{"type": "tool_result", "tool_use_id": "t2", "content": "PLUM agent report"}], "s1", b,
                 ts=f"{TODAY}T10:02:00.000Z", toolUseResult=report),
            {"type": "last-prompt", "lastPrompt": "PLUM last prompt", "sessionId": "s1"},
            {"type": "queue-operation", "operation": "enqueue", "content": "PLUM queued", "sessionId": "s1"}])
        self.subagent("s1", "a1", [user("PLUM agent prompt for gap-bbbbbb", "s1", w, agent="a1"),
                                   call("m2", "s1", w, agent="a1", text="PLUM report\nITEM: gap-bbbbbb")])
        rows = self.harvest()
        self.assertEqual(len(self.calls(rows)), 2)
        for path in [*self.out.iterdir(), self.state]:
            self.assertNotIn("PLUM", path.read_text(), path.name)
        for r in rows:
            self.assertEqual(tuple(r), wh.CALL_ROW if r["row"] == "calls" else wh.AGENT_ROW)
            self.assertTrue(all(v is None or isinstance(v, (bool, int)) or not any(c.isspace() for c in v) for v in r.values()))
        agents = [(r["agent"], r["item"], r["join"], r["duration_ms"], r["tool_uses"], r["lines_added"], r["model"])
                  for r in rows if r["row"] == "agent"]
        self.assertEqual(agents, [("a1", "gap-bbbbbb", "prompt", 5000, 3, 10, "claude-sonnet-5")])


class TestIncremental(HarvestTest):
    def test_repeated_runs_read_only_new_lines_and_write_the_same_bytes(self):
        b = "work/gap-aaaaaa"
        self.session("s1", [call("m1", "s1", b, out=5)])
        self.harvest()
        first = (self.out / f"{TODAY}.jsonl").read_bytes()
        self.harvest()
        self.assertEqual((self.out / f"{TODAY}.jsonl").read_bytes(), first)
        # the rest of m1's lines, a new call, and a line still being written
        partial = json.dumps(call("m3", "s1", b, out=3))
        self.session("s1", [call("m1", "s1", b, out=9), call("m2", "s1", b, out=7)], append=True, raw=partial[:40])
        self.assertEqual([(r["calls"], r["output_tokens"]) for r in self.calls(self.harvest())], [(2, 16)])
        self.session("s1", [], append=True, raw=partial[40:] + "\n")
        rows = self.harvest()
        self.assertEqual([(r["calls"], r["output_tokens"]) for r in self.calls(rows)], [(3, 19)])
        # a run from scratch agrees with the incremental ones
        self.assertEqual(self.harvest(out=self.root / "again", state=self.root / "again.json"), rows)

    def test_frozen_dates_are_final(self):
        self.session("s1", [call("m1", "s1", "main", ts="2026-08-01T10:00:00.000Z"), call("m2", "s1", "main")])
        self.harvest()
        old = (self.out / "2026-08-01.jsonl").read_bytes()
        # a call that turns up later for a frozen date (a copy, or a rewritten file) is counted as late, not added
        self.session("s1", [call("m9", "s1", "main", ts="2026-08-01T11:00:00.000Z")], append=True)
        self.harvest()
        self.assertEqual((self.out / "2026-08-01.jsonl").read_bytes(), old)
        state = json.loads(self.state.read_text())
        self.assertEqual((state["frozen_before"], state["late"]), ("2026-08-25", 1))

    def test_dry_run_writes_nothing(self):
        self.session("s1", [call("m1", "s1", "main")])
        summary = json.loads(self.run_tool("--dry-run", "--json"))
        self.assertEqual((summary["totals"]["calls"], summary["by_join"]), (1, {"none": 1}))
        self.assertFalse(self.out.exists() or self.state.exists())


if __name__ == "__main__":
    unittest.main(verbosity=1)
