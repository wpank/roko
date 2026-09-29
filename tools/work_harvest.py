#!/usr/bin/env python3
"""Transcript harvester: tokens, model and time per work item from Claude Code transcripts (gap-263de5).

Reads every Claude Code project directory of the main checkout and of the directories whose path extends it, such as
worktrees: `~/.claude/projects/<main checkout path, each non-alphanumeric as ->*`, both the session files and
`<session>/subagents/*.jsonl`.
It keeps derived fields only (D27). Message text, prompts, thinking, tool input and tool output are never copied: a
transcript's first prompt and a worker's report are scanned for an item id, and only the id is kept.

Rows (schema roko.work_harvest/1), one file per UTC date: work/telemetry/harvest/<date>.jsonl
  "calls"  one row per (date, session, agent, branch, item, join, model, speed, entrypoint, sidechain): the call count,
           tokens by class, web tool requests, and the first and last timestamp. item null (join "none") is
           overhead: calls that no item claims, reported for their branch. Item rows name their subagent; overhead
           rows sum the session's subagents (sidechain true) apart from its main thread (sidechain false).
  "agent"  one row per finished subagent, from the Agent tool's result: duration, tool uses and tool stats.

Joins: gitBranch work/<id> ("branch"); else the transcript's first prompt, when it names one work/<id> branch or
exactly one item id ("prompt"); else, for a subagent, the id on its last `ITEM: <id>` or `READY <id>` line
("item-line"). A worker given several items on one branch is joined to the item its branch is named after.
Counting: each API call once, by message.id. Streaming writes a line per content block, and a forked subagent's file
repeats its parent's calls, so files are read oldest first. Input and cache tokens are taken once and output is the
largest value seen. The parent session's agent_progress lines repeat the subagent file and are not read.
Origin: entrypoint sdk-cli is Roko's own ClaudeCli child (`claude --print`); cli, claude-desktop and claude-vscode are
operator sessions; Claude Code before 2.1.28x wrote no entrypoint ("unknown"). A subagent inherits its session's.

State lives outside work/, in .roko/work-harvest/state.json: byte offsets, the ids already counted, and the aggregates
of dates not yet frozen. A run reads only new complete lines and rewrites each unfrozen date file from the state, so
a repeated run writes the same bytes. A date more than FREEZE_DAYS old is frozen once written: its state is dropped,
and calls that turn up for it later are counted as late and skipped. A date file is never rewritten with fewer calls
than it holds.

Usage:
  work_harvest.py [--dry-run] [--json] [--projects DIR…] [--out DIR] [--state FILE] [--today YYYY-MM-DD]
"""
from __future__ import annotations

import argparse, datetime as dt, glob, json, os, re, sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import work  # noqa: E402

SCHEMA = "roko.work_harvest/1"
STATE_SCHEMA = "roko.work_harvest_state/1"
FREEZE_DAYS = 35
ITEM = work.ID_ANY.pattern
BRANCH_ITEM = re.compile(rf"work/({ITEM})")
NAMED_BRANCH = re.compile(rf"(?<![\w/.-])work/({ITEM})")
ITEM_LINE = re.compile(rf"(?m)^[\s>*`_-]*(?:ITEM[*`_]*\s*:|READY)[*`_\s]*({ITEM})")
REMINDER = re.compile(r"<system-reminder>.*?</system-reminder>", re.S)
TOKEN = re.compile(r"\S{1,200}")
TS = re.compile(r"\d{4}-\d{2}-\d{2}T[0-9:.]+Z?")
ORIGIN = {"sdk-cli": "roko", "cli": "operator", "claude-desktop": "operator", "claude-vscode": "operator"}
SUMS = ("calls", "input_tokens", "output_tokens", "cache_read_tokens", "cache_write_5m_tokens", "cache_write_1h_tokens",
        "web_search_requests", "web_fetch_requests")
TOOL_STATS = {"readCount": "read_count", "searchCount": "search_count", "bashCount": "bash_count",
              "editFileCount": "edit_file_count", "linesAdded": "lines_added", "linesRemoved": "lines_removed"}
CALL_ROW = ("schema", "row", "date", "session", "agent", "branch", "item", "join", "model", "speed", "entrypoint",
            "origin", "sidechain", *SUMS, "first_ts", "last_ts")
AGENT_ROW = ("schema", "row", "date", "ts", "session", "agent", "item", "join", "origin", "agent_type", "model", "status",
             "duration_ms", "tool_uses", *TOOL_STATS.values())


def num(v) -> int | None:
    return v if isinstance(v, int) and not isinstance(v, bool) else None


def clean(v) -> str | None:
    """A short whitespace-free token (an id, branch, model or enum value), else None: rows never carry free text."""
    return v if isinstance(v, str) and TOKEN.fullmatch(v) else None


def prompt_text(content) -> str | None:
    """The text of a user message that is a prompt rather than a tool result."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        texts = [b["text"] for b in content if isinstance(b, dict) and b.get("type") == "text" and isinstance(b.get("text"), str)]
        return "\n".join(texts) if texts else None
    return None


def prompt_item(text: str) -> str | None:
    """The item a prompt is about: the id of the one work/<id> branch it names, else its only item id."""
    text = REMINDER.sub("", text)
    ids = set(NAMED_BRANCH.findall(text)) or set(work.ID_ANY.findall(text))
    return next(iter(ids)) if len(ids) == 1 else None


def claude_projects() -> Path:
    return Path(os.environ.get("CLAUDE_CONFIG_DIR") or Path.home() / ".claude") / "projects"


def project_dirs(base: Path, main: Path) -> list[Path]:
    """The project directories of the main checkout and of every directory whose path extends it."""
    prefix = re.sub(r"[^A-Za-z0-9]", "-", str(main))
    return sorted(d for d in base.glob(glob.escape(prefix) + "*") if d.is_dir())


def transcripts(dirs) -> list[Path]:
    """Session and subagent files, oldest first, so a fork's copy of its parent's calls stays with the parent."""
    found = []
    for d in dirs:
        for f in (*d.glob("*.jsonl"), *d.glob("*/subagents/*.jsonl")):
            try:
                s = f.stat()
            except OSError:
                continue
            found.append((getattr(s, "st_birthtime", s.st_mtime), str(f), f))
    return [f for _, _, f in sorted(found)]


def new_state() -> dict:
    return {"schema": STATE_SCHEMA, "frozen_before": None, "next_agg": 0, "late": 0,
            "files": {}, "scopes": {}, "aggs": {}, "seen": {}, "results": {}}


def load_state(path: Path) -> dict:
    if not path.exists():
        return new_state()
    st = json.loads(path.read_text())
    if st.get("schema") != STATE_SCHEMA:
        sys.exit(f"{path}: state schema {st.get('schema')!r} is not {STATE_SCHEMA}")
    return st


def save_state(path: Path, st: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(st, separators=(",", ":")))
    os.replace(tmp, path)


class Harvest:
    """Folds transcript lines into per-scope aggregates; rows() joins them to items."""

    def __init__(self, state: dict, today: dt.date):
        self.st, self.today = state, today
        self.index = {(a["date"], a["scope"], a["branch"], a["model"], a["speed"], a["sidechain"]): n
                      for n, a in state["aggs"].items()}
        self.run = Counter()

    def scope(self, key: str) -> dict:
        return self.st["scopes"].setdefault(key, {"entrypoint": None, "prompt_seen": False, "prompt": None, "item_line": None})

    def frozen(self, date: str) -> bool:
        """True for a date whose file is final; the caller skips the record, and it is counted as late."""
        late = bool(self.st["frozen_before"]) and date < self.st["frozen_before"]
        if late:
            self.run["late"] += 1
            self.st["late"] += 1
        return late

    def read(self, path: Path) -> None:
        """Fold in the complete lines appended to `path` since the last run."""
        try:
            stat = path.stat()
        except OSError:
            return
        self.run["files"] += 1
        rec = self.st["files"].get(str(path))
        fresh = not rec or rec.get("ino") != stat.st_ino or rec.get("offset", 0) > stat.st_size
        offset = 0 if fresh else rec["offset"]
        if offset == stat.st_size:
            return
        self.run["files_read"] += 1
        sub = path.parent.name == "subagents"
        session = path.parent.parent.name if sub else path.stem
        agent = path.stem.removeprefix("agent-") if sub else ""
        with path.open("rb") as fh:
            fh.seek(offset)
            for raw in fh:
                if not raw.endswith(b"\n"):
                    break  # still being written: read it next time
                offset += len(raw)
                # An assistant or user line always holds its type as a quoted value; this skips bash and hook progress.
                if b'"assistant"' in raw or b'"user"' in raw:
                    self.line(raw, session, agent)
        self.st["files"][str(path)] = {"offset": offset, "ino": stat.st_ino}

    def line(self, raw: bytes, session: str, agent: str) -> None:
        try:
            o = json.loads(raw)
        except ValueError:
            self.run["bad_lines"] += 1
            return
        if not isinstance(o, dict) or o.get("type") not in ("assistant", "user"):
            return
        session = clean(o.get("sessionId")) or session
        agent = clean(o.get("agentId")) or agent
        key = f"{session}/{agent}"
        sc = self.scope(key)
        if not sc["entrypoint"]:
            sc["entrypoint"] = clean(o.get("entrypoint"))
        m = o["message"] if isinstance(o.get("message"), dict) else {}
        ts = o.get("timestamp") if isinstance(o.get("timestamp"), str) and TS.fullmatch(o["timestamp"]) else None
        if o["type"] == "user":
            self.user(o, m, sc, session, ts)
        elif ts:
            self.assistant(o, m, sc, key, ts, bool(agent))
        else:
            self.run["bad_lines"] += 1

    def user(self, o: dict, m: dict, sc: dict, session: str, ts: str | None) -> None:
        if not sc["prompt_seen"] and not o.get("isMeta"):
            text = prompt_text(m.get("content"))
            if text is not None:
                sc["prompt_seen"], sc["prompt"] = True, prompt_item(text)
        r = o.get("toolUseResult")
        if not (isinstance(r, dict) and clean(r.get("agentId")) and num(r.get("totalDurationMs")) is not None and ts):
            return
        if self.frozen(ts[:10]):
            return
        child = f"{session}/{r['agentId']}"
        stats = r["toolStats"] if isinstance(r.get("toolStats"), dict) else {}
        self.st["results"][f"{child}@{ts}"] = {
            "scope": child, "ts": ts, "agent_type": clean(r.get("agentType")), "model": clean(r.get("resolvedModel")),
            "status": clean(r.get("status")), "duration_ms": r["totalDurationMs"], "tool_uses": num(r.get("totalToolUseCount")),
            **{name: num(stats.get(k)) for k, name in TOOL_STATS.items()}}

    def assistant(self, o: dict, m: dict, sc: dict, key: str, ts: str, is_agent: bool) -> None:
        usage, model = m.get("usage"), clean(m.get("model"))
        if not isinstance(usage, dict) or not model or model == "<synthetic>":
            return
        if is_agent:
            for b in m.get("content") if isinstance(m.get("content"), list) else []:
                if isinstance(b, dict) and b.get("type") == "text" and isinstance(b.get("text"), str):
                    for found in ITEM_LINE.findall(b["text"]):
                        sc["item_line"] = found
        mid = clean(m.get("id")) or clean(o.get("requestId")) or clean(o.get("uuid"))
        if not mid:
            self.run["bad_lines"] += 1
            return
        out = num(usage.get("output_tokens")) or 0
        seen = self.st["seen"].get(mid)
        if seen:  # another line of a call already counted: only its output can have grown
            agg = self.st["aggs"].get(seen[0])
            if agg is not None and out > seen[1]:
                agg["output_tokens"] += out - seen[1]
                agg["last_ts"] = max(agg["last_ts"], ts)
                seen[1] = out
            return
        date = ts[:10]
        if self.frozen(date):
            return
        k = (date, key, clean(o.get("gitBranch")) or "", model, clean(usage.get("speed")) or "standard", bool(o.get("isSidechain")))
        n = self.index.get(k)
        if n is None:
            n = self.index[k] = str(self.st["next_agg"])
            self.st["next_agg"] += 1
            self.st["aggs"][n] = {"date": k[0], "scope": k[1], "branch": k[2], "model": k[3], "speed": k[4], "sidechain": k[5],
                                  **dict.fromkeys(SUMS, 0), "first_ts": ts, "last_ts": ts}
        agg = self.st["aggs"][n]
        write = num(usage.get("cache_creation_input_tokens")) or 0
        split = usage["cache_creation"] if isinstance(usage.get("cache_creation"), dict) else {}
        write_1h = min(write, num(split.get("ephemeral_1h_input_tokens")) or 0)
        web = usage["server_tool_use"] if isinstance(usage.get("server_tool_use"), dict) else {}
        for f, v in (("calls", 1), ("input_tokens", num(usage.get("input_tokens")) or 0), ("output_tokens", out),
                     ("cache_read_tokens", num(usage.get("cache_read_input_tokens")) or 0),
                     ("cache_write_5m_tokens", write - write_1h), ("cache_write_1h_tokens", write_1h),
                     ("web_search_requests", num(web.get("web_search_requests")) or 0),
                     ("web_fetch_requests", num(web.get("web_fetch_requests")) or 0)):
            agg[f] += v
        agg["first_ts"], agg["last_ts"] = min(agg["first_ts"], ts), max(agg["last_ts"], ts)
        self.st["seen"][mid] = [n, out]
        self.run["calls"] += 1

    def join(self, scope: str, branch: str) -> tuple[str | None, str]:
        m = BRANCH_ITEM.fullmatch(branch)
        if m:
            return m.group(1), "branch"
        sc = self.st["scopes"].get(scope) or {}
        if sc.get("prompt"):
            return sc["prompt"], "prompt"
        if sc.get("item_line") and not scope.endswith("/"):
            return sc["item_line"], "item-line"
        return None, "none"

    def rows(self) -> dict[str, list[dict]]:
        """{date: rows} for every date the state holds, each list sorted."""
        own, session_ep = {}, {}
        for key, sc in self.st["scopes"].items():
            session, agent = key.split("/", 1)
            if sc.get("entrypoint") and (not agent or session not in session_ep):
                session_ep[session] = sc["entrypoint"]
            own[key] = sc.get("entrypoint")
        entrypoint = lambda key: own.get(key) or session_ep.get(key.split("/", 1)[0]) or "unknown"  # noqa: E731
        origin = lambda ep: ORIGIN.get(ep, "unknown" if ep == "unknown" else "other")  # noqa: E731
        out, calls, latest = defaultdict(list), {}, {}
        for a in self.st["aggs"].values():
            session, agent = a["scope"].split("/", 1)
            item, how = self.join(a["scope"], a["branch"])
            ep = entrypoint(a["scope"])
            # Item rows keep their agent; overhead rows sum a session's subagents, split only by the sidechain flag.
            row = {"schema": SCHEMA, "row": "calls", "date": a["date"], "session": session,
                   "agent": (agent or None) if item else None, "branch": a["branch"], "item": item, "join": how,
                   "model": a["model"], "speed": a["speed"], "entrypoint": ep, "origin": origin(ep),
                   "sidechain": a["sidechain"], **dict.fromkeys(SUMS, 0), "first_ts": a["first_ts"], "last_ts": a["last_ts"]}
            row = calls.setdefault(tuple(row[k] for k in CALL_ROW[:13]), row)
            for s in SUMS:
                row[s] += a[s]
            row["first_ts"], row["last_ts"] = min(row["first_ts"], a["first_ts"]), max(row["last_ts"], a["last_ts"])
            if a["last_ts"] >= latest.get(a["scope"], ("",))[0]:
                latest[a["scope"]] = (a["last_ts"], a["branch"])
        for row in calls.values():
            out[row["date"]].append(row)
        for r in self.st["results"].values():
            session, agent = r["scope"].split("/", 1)
            item, how = self.join(r["scope"], latest.get(r["scope"], ("", ""))[1])
            out[r["ts"][:10]].append({
                "schema": SCHEMA, "row": "agent", "date": r["ts"][:10], "ts": r["ts"], "session": session, "agent": agent,
                "item": item, "join": how, "origin": origin(entrypoint(r["scope"])),
                **{k: r[k] for k in AGENT_ROW[AGENT_ROW.index("agent_type"):]}})
        keys = ("row", "session", "agent", "branch", "item", "join", "model", "speed", "entrypoint", "sidechain", "ts")
        order = lambda r: tuple("" if r.get(k) is None else str(r[k]) for k in keys)  # noqa: E731
        return {d: sorted(rows, key=order) for d, rows in out.items()}

    def freeze(self) -> None:
        """Drop the state of dates older than FREEZE_DAYS: their files are final."""
        cut = (self.today - dt.timedelta(days=FREEZE_DAYS)).isoformat()
        self.st["frozen_before"] = max(self.st["frozen_before"] or cut, cut)
        drop = {n for n, a in self.st["aggs"].items() if a["date"] < cut}
        self.st["aggs"] = {n: a for n, a in self.st["aggs"].items() if n not in drop}
        self.st["seen"] = {mid: v for mid, v in self.st["seen"].items() if v[0] not in drop}
        self.st["results"] = {k: r for k, r in self.st["results"].items() if r["ts"][:10] >= cut}
        self.st["files"] = {p: rec for p, rec in self.st["files"].items() if Path(p).exists()}


def calls_in(text: str) -> int:
    return sum(r.get("calls", 0) for r in map(json.loads, text.splitlines()) if r.get("row") == "calls")


def write_dates(rows_by_date: dict[str, list[dict]], out_dir: Path) -> list[str]:
    """Write each date's rows; returns the dates whose file changed."""
    changed = []
    for date, rows in sorted(rows_by_date.items()):
        path = out_dir / f"{date}.jsonl"
        text = "".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows)
        if path.exists():
            old = path.read_text()
            if old == text:
                continue
            if calls_in(old) > calls_in(text):
                print(f"warning: {path} holds more calls than the state has for {date}; left unchanged", file=sys.stderr)
                continue
        out_dir.mkdir(parents=True, exist_ok=True)
        tmp = path.with_name(path.name + ".tmp")
        tmp.write_text(text)
        os.replace(tmp, path)
        changed.append(date)
    return changed


def item_totals(rows) -> dict[str, dict]:
    """{item: its calls rows summed, with the number of sessions and agents that worked on it}. Overhead is left out."""
    out = {}
    for r in rows:
        if r["row"] != "calls" or not r["item"]:
            continue
        t = out.setdefault(r["item"], {**dict.fromkeys(SUMS, 0), "sessions": set(), "agents": set(), "joins": set(),
                                        "first_ts": r["first_ts"], "last_ts": r["last_ts"]})
        for s in SUMS:
            t[s] += r[s]
        t["sessions"].add(r["session"])
        if r["agent"]:
            t["agents"].add((r["session"], r["agent"]))
        t["joins"].add(r["join"])
        t["first_ts"], t["last_ts"] = min(t["first_ts"], r["first_ts"]), max(t["last_ts"], r["last_ts"])
    for t in out.values():
        t["sessions"], t["agents"], t["joins"] = len(t["sessions"]), len(t["agents"]), sorted(t["joins"])
    return out


def add(acc: dict, r: dict) -> None:
    for s in SUMS:
        acc[s] = acc.get(s, 0) + r[s]


def summarize(rows_by_date: dict[str, list[dict]], run: Counter, dirs: list[Path]) -> dict:
    rows = [r for d in sorted(rows_by_date) for r in rows_by_date[d]]
    calls = [r for r in rows if r["row"] == "calls"]
    totals, by_origin, by_join, by_model, overhead = {}, defaultdict(dict), Counter(), defaultdict(dict), defaultdict(dict)
    for r in calls:
        add(totals, r)
        add(by_origin[r["origin"]].setdefault("item" if r["item"] else "overhead", {}), r)
        by_join[r["join"]] += r["calls"]
        add(by_model[r["model"]], r)
        if not r["item"]:
            add(overhead[r["branch"] or "(none)"], r)
    items = item_totals(rows)
    top = sorted(items.items(), key=lambda kv: -kv[1]["output_tokens"])[:10]
    agents = [r for r in rows if r["row"] == "agent"]
    return {
        "project_dirs": len(dirs), "files": run["files"], "files_read": run["files_read"], "new_calls": run["calls"],
        "late_calls": run["late"], "bad_lines": run["bad_lines"],
        "dates": [min(rows_by_date), max(rows_by_date)] if rows_by_date else [], "date_files": len(rows_by_date),
        "rows": len(rows), "bytes": sum(len(json.dumps(r, separators=(",", ":"))) + 1 for r in rows),
        "totals": totals, "by_origin": by_origin, "by_join": dict(by_join),
        "by_model": dict(sorted(by_model.items(), key=lambda kv: -kv[1]["calls"])),
        "overhead_top_branches": dict(sorted(overhead.items(), key=lambda kv: -kv[1]["output_tokens"])[:5]),
        "items": {"count": len(items), "top": {i: t for i, t in top}},
        "agents": {"results": len(agents), "with_tool_stats": sum(r["lines_added"] is not None for r in agents)},
    }


def human(n: int) -> str:
    for size, unit in ((1e9, "B"), (1e6, "M"), (1e3, "k")):
        if n >= size:
            return f"{n / size:.1f}{unit}"
    return str(n)


def tok(x: dict) -> str:
    return (f"{human(x.get('calls', 0))} calls, out {human(x.get('output_tokens', 0))}, in {human(x.get('input_tokens', 0))}, "
            f"cache read {human(x.get('cache_read_tokens', 0))}, write 5m {human(x.get('cache_write_5m_tokens', 0))} "
            f"/ 1h {human(x.get('cache_write_1h_tokens', 0))}")


def render(s: dict, note: str) -> str:
    t = s["totals"]
    lines = [f"{s['files']} transcripts in {s['project_dirs']} project directories ({s['files_read']} with new lines): "
             f"{s['new_calls']} new API calls; {s['late_calls']} late, {s['bad_lines']} unreadable lines.",
             f"State covers {s['date_files']} dates {'..'.join(s['dates'])}: {s['rows']} rows, {human(s['bytes'])} bytes.",
             f"Total: {tok(t)}." if t else "Total: nothing harvested."]
    for origin, parts in sorted(s["by_origin"].items()):
        for part in ("item", "overhead"):
            if part in parts:
                lines.append(f"  {origin:<8} {part:<8} {tok(parts[part])}")
    lines.append("Joins (calls): " + ", ".join(f"{k} {human(v)}" for k, v in sorted(s["by_join"].items())))
    lines.append("Models (calls): " + ", ".join(f"{m} {human(v['calls'])}" for m, v in s["by_model"].items()))
    lines.append("Overhead, top branches by output: " + "; ".join(
        f"{b} {human(v['output_tokens'])} out" for b, v in s["overhead_top_branches"].items()))
    lines.append(f"Items joined: {s['items']['count']}. Top by output tokens:")
    for item, v in s["items"]["top"].items():
        lines.append(f"  {item}  {tok(v)}; {v['sessions']} sessions, {v['agents']} agents, via {'+'.join(v['joins'])}")
    lines.append(f"Finished subagents: {s['agents']['results']} ({s['agents']['with_tool_stats']} with tool stats).")
    lines.append(note)
    return "\n".join(lines)


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--dry-run", action="store_true", help="read and summarise; write neither rows nor state")
    ap.add_argument("--json", action="store_true", help="print the summary as JSON")
    ap.add_argument("--projects", nargs="+", type=Path, help="project directories (default: all of this checkout's)")
    ap.add_argument("--out", type=Path, default=work.REPO / "work" / "telemetry" / "harvest")
    ap.add_argument("--state", type=Path, default=work.REPO / ".roko" / "work-harvest" / "state.json")
    ap.add_argument("--today", type=dt.date.fromisoformat, default=dt.datetime.now(dt.timezone.utc).date(),
                    help="UTC date that sets the freeze horizon (default: today)")
    a = ap.parse_args(argv)
    dirs = a.projects or project_dirs(claude_projects(), work.main_root())
    h = Harvest(load_state(a.state), a.today)
    for f in transcripts(dirs):
        h.read(f)
    rows = h.rows()
    if a.dry_run:
        note = f"Dry run: nothing written (would write {len(rows)} date files to {a.out}, state to {a.state})."
    else:
        changed = write_dates(rows, a.out)
        h.freeze()
        save_state(a.state, h.st)
        note = f"Wrote {len(changed)} date files to {a.out}; state {a.state}; frozen before {h.st['frozen_before']}."
    s = summarize(rows, h.run, dirs)
    print(json.dumps(s, indent=2, default=list) if a.json else render(s, note))


if __name__ == "__main__":
    main()
