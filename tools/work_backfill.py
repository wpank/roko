#!/usr/bin/env python3
"""Backfill the development record for the items closed before the event log existed (gap-dc6775).

One-shot. Writes work/telemetry/events/backfill.jsonl in the roko.work_event/1 schema (tools/work.py), every row with
source "backfill" and session "backfill": one `closed` row per closed item, and one `lane-start` row per lane branch
whose creation the reflog still holds. It never edits an item file, and a rerun writes the file again from the same
inputs. The rollup keeps these rows apart from live ones (work/telemetry/DEFINITIONS.md).

Executor of a closed item, from its [closed].by, by the first rule that matches:
  plan:<plan>#<task>                                   roko-plan (via roko-plan)
  work sweep / triage check / work enrichment / reconcile …   verification-only: it checked the item, it did not do it
  Will …                                               human, with assist "claude" when the text names Claude
  session <name>, or a session name such as roko-b6    claude-session
  wk-<name> …, or (claude-agent)                       claude-agent: a worker the coordinator spawned
  commit trailer                                       unknown (reason commit-trailer)
  coordinator …                                        unknown (reason coordinator-close)
  no by                                                unknown (reason no-by)
  anything else                                        unknown (reason unmapped-by)
An unknown executor is then looked up in the merge that brought the item's branch in (the `Merged in <sha>` of its
evidence, else the oldest merge naming `work/<id>`): the `Executor:` trailers of the merged commits whose `Work-Item:`
names the item, else of all of them. The commonest is the executor, a tie going to the one who committed first; a
second one is `assist`. `executor_from` says which source decided ("by" or "merge-trailers").

The row's `ts` is the commit that set the item's closing status (`ts_from` "commit"), else midnight UTC of
[closed].at (`ts_from` "date").

Lane starts: the creation entry ("branch: Created from …") of each branch's reflog, and for a worktree whose branch
has none, the first entry of the worktree's HEAD log (`ts_from` "worktree"). `main`, `master` and the main checkout's
branch are not lanes. A branch `work/<id>` names its item.

Usage:
  work_backfill.py [--out FILE] [--dry-run]
"""
from __future__ import annotations

import argparse, datetime as dt, json, re, subprocess, sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import work  # noqa: E402

SESSION = "backfill"
VERIFICATION = re.compile(r"^(?:work sweep|triage check|work enrichment|reconcile)\b", re.I)
SESSION_NAME = re.compile(r"\broko-[a-z0-9]+\b")
NULL_SHA = "0" * 40


def git(*args: str) -> str:
    return work.git(*args)


def executor_from_by(by) -> tuple[str, str | None, str | None]:
    """(executor, assist, reason) for a [closed].by text; the reason says why an executor is unknown."""
    if not by or not str(by).strip():
        return "unknown", None, "no-by"
    text = str(by).strip()
    if text.startswith("plan:"):
        return "roko-plan", None, None
    if VERIFICATION.match(text):
        return "verification-only", None, None
    if re.match(r"^will\b", text, re.I):
        return "human", ("claude" if re.search(r"\bclaude\b", text, re.I) else None), None
    if text.startswith("wk-") or "(claude-agent)" in text:
        return "claude-agent", None, None
    if re.match(r"^session\s", text, re.I) or SESSION_NAME.search(text):
        return "claude-session", None, None
    if text == "commit trailer":
        return "unknown", None, "commit-trailer"
    if text.startswith("coordinator"):
        return "unknown", None, "coordinator-close"
    return "unknown", None, "unmapped-by"


def merges_by_item(ids: set[str]) -> dict[str, str]:
    """{item id: the oldest merge commit whose message names its branch `work/<id>` or starts `merge: <id>`}."""
    out = {}
    for rec in git("log", "--merges", "--format=%H%x1f%s%x1f%b%x1e").split("\x1e"):
        parts = rec.strip("\n").split("\x1f")
        if len(parts) < 3:
            continue
        sha, msg = parts[0].strip(), parts[1] + "\n" + parts[2]
        named = set(re.findall(rf"work/({work.ID_ANY.pattern})\b", msg))
        named |= set(re.findall(rf"^merge: ({work.ID_ANY.pattern})\b", parts[1]))
        for iid in named & ids:
            out[iid] = sha  # git log is newest first, so the last one kept is the oldest merge
    return out


def trailer_executor(merge: str, iid: str) -> tuple[str | None, str | None]:
    """(executor, assist) from the Executor: trailers of the commits `merge` brought in, preferring those whose
    Work-Item: names `iid`; (None, None) when it is not a merge or no trailer says."""
    parents = git("rev-list", "--parents", "-n", "1", merge).split()
    if len(parents) < 3:
        return None, None
    rows = []
    fmt = "%(trailers:key=Executor,valueonly,separator=%x1d)%x1f%(trailers:key=Work-Item,valueonly,separator=%x1d)%x1e"
    for rec in git("log", "--reverse", f"--format={fmt}", f"{parents[1]}..{parents[2]}").split("\x1e"):
        if "\x1f" not in rec:
            continue
        execs, items = rec.split("\x1f", 1)
        rows.append(({e.strip() for e in execs.split("\x1d") if e.strip()}, {i.strip() for i in items.split("\x1d") if i.strip()}))
    named = [r for r in rows if iid in r[1]]
    # Oldest commit first, so a tie goes to whoever started the work (most_common keeps first-seen order).
    counts = Counter(e for execs, _ in (named or rows) for e in execs if e in work.CLOSED_EXECUTORS)
    if not counts:
        return None, None
    ranked = [e for e, _ in counts.most_common()]
    return ranked[0], (ranked[1] if len(ranked) > 1 else None)


def close_ts(path: Path, at) -> tuple[str, str]:
    """(ts, ts_from): when the commit that set the item's closing status was made, else midnight UTC of `at`."""
    rel = str(path.relative_to(work.REPO))
    ct = git("log", "-1", "--format=%ct", "-G", r'^status = "(done|wontfix|superseded)"', "--", rel).strip()
    if ct:
        return work.utc_ts(dt.datetime.fromtimestamp(int(ct), dt.timezone.utc)), "commit"
    return f"{at}T00:00:00Z", "date"


def closed_rows(items) -> list[dict]:
    closed = [i for i in items if i.get("status") not in work.OPEN and i.get("status") != "parked"]
    merges = merges_by_item({i["id"] for i in closed})
    branches = set(git("for-each-ref", "--format=%(refname:short)", "refs/heads/work/").split())
    rows = []
    for it in closed:
        c = it.get("closed") or {}
        executor, assist, reason = executor_from_by(c.get("by"))
        source, merge = "by", None
        if executor == "unknown":
            m = re.search(r"\bMerged in ([0-9a-f]{7,40})\b", str(c.get("evidence") or ""))
            for cand in ([m.group(1)] if m else []) + [merges.get(it["id"])]:
                if not cand:
                    continue
                found, helper = trailer_executor(cand, it["id"])
                if found:
                    executor, assist, source, merge = found, helper, "merge-trailers", git("rev-parse", "--short=9", cand).strip()
                    break
        ts, ts_from = close_ts(it["_path"], c.get("at") or it.get("updated") or it.get("created"))
        branch = f"work/{it['id']}"
        rows.append(work.event_row(
            "closed", it["id"], session=SESSION, ts=ts, executor=executor, via="roko-plan" if executor == "roko-plan" else None,
            branch=branch if branch in branches else None, source="backfill", status=it.get("status"),
            by=c.get("by"), reason=reason if source == "by" else None, assist=assist, executor_from=source,
            commit=c.get("commit"), merge_sha=merge, closed_at=str(c["at"]) if c.get("at") else None, ts_from=ts_from))
    return rows


def first_entry(log: Path) -> tuple[str, str] | None:
    """(ts, message) of a reflog's first line when it records a creation (old sha all zeros), else None."""
    try:
        line = log.read_text(errors="replace").split("\n", 1)[0]
    except OSError:
        return None
    m = re.match(r"^([0-9a-f]{40}) [0-9a-f]{40} .*? (\d+) [+-]\d{4}\t?(.*)$", line)
    if not m or m.group(1) != NULL_SHA:
        return None
    return work.utc_ts(dt.datetime.fromtimestamp(int(m.group(2)), dt.timezone.utc)), m.group(3)


def lane_item(branch: str, ids: set[str]) -> str | None:
    """The item a `work/<id>` branch is for, also when the id carries a suffix (`work/gap-3506f1b`)."""
    if not branch.startswith("work/"):
        return None
    name = branch[len("work/"):]
    if name in ids:
        return name
    return next((i for i in sorted(ids, key=len, reverse=True) if name.startswith(i)), None)


def lane_rows(ids: set[str]) -> list[dict]:
    common = Path(git("rev-parse", "--path-format=absolute", "--git-common-dir").strip())
    trunk = {"main", "master", git("-C", str(work.main_root()), "symbolic-ref", "--short", "-q", "HEAD").strip()}
    rows, seen = [], set()
    heads = common / "logs" / "refs" / "heads"
    for log in sorted(p for p in heads.rglob("*") if p.is_file()) if heads.exists() else []:
        branch = str(log.relative_to(heads))
        entry = first_entry(log)
        if branch in trunk or not entry:
            continue
        ts, msg = entry
        created = re.match(r"^branch: Created from (.+)$", msg)
        rows.append(work.event_row("lane-start", lane_item(branch, ids), session=SESSION, ts=ts, branch=branch,
                                   source="backfill", created_from=created.group(1) if created else None,
                                   ts_from="branch-reflog"))
        seen.add(branch)
    for wt in sorted((common / "worktrees").glob("*")) if (common / "worktrees").exists() else []:
        head = (wt / "HEAD").read_text(errors="replace").strip() if (wt / "HEAD").exists() else ""
        branch = head[len("ref: refs/heads/"):] if head.startswith("ref: refs/heads/") else None
        entry = first_entry(wt / "logs" / "HEAD")
        if not branch or branch in seen or branch in trunk or not entry:
            continue
        rows.append(work.event_row("lane-start", lane_item(branch, ids), session=SESSION, ts=entry[0], branch=branch,
                                   source="backfill", worktree=wt.name, ts_from="worktree"))
        seen.add(branch)
    return rows


def build(items) -> list[dict]:
    rows = closed_rows(items) + lane_rows({i["id"] for i in items})
    return sorted(rows, key=lambda r: (r["ts"], r["event"], r.get("item") or "", r.get("branch") or ""))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", help="default: work/telemetry/events/backfill.jsonl in this checkout, to commit with it")
    ap.add_argument("--dry-run", action="store_true", help="print a summary; write nothing")
    a = ap.parse_args()
    items = [i for i in work.load("work")[0]]
    rows = build(items)
    bad = [(r.get("item") or r.get("branch"), e) for r in rows for e in work.check_event(r)]
    if bad:
        sys.exit(f"invalid rows: {bad[:5]}")
    closed = [r for r in rows if r["event"] == "closed"]
    print(f"{len(closed)} closed rows, {len(rows) - len(closed)} lane-start rows")
    print("executors: " + ", ".join(f"{k} {v}" for k, v in Counter(r["executor"] for r in closed).most_common()))
    print("decided by: " + ", ".join(f"{k} {v}" for k, v in Counter(r["executor_from"] for r in closed).most_common()))
    if a.dry_run:
        return
    out = Path(a.out) if a.out else work.REPO / "work" / "telemetry" / "events" / f"{SESSION}.jsonl"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(json.dumps(r) + "\n" for r in rows))
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
