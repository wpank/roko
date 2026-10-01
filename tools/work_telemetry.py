#!/usr/bin/env python3
"""Daily rollup of the development record, and its committed manifest (gap-ccb87e).

Not for workers: `work.py render`, NOW.md and the skills never read what this writes.

`rollup` joins, per item: the `claim`, `release`, `merged`, `post-verify`, `escape` and `intervention` events of
work/telemetry/events/*.jsonl (tools/work.py), the item's [closed] block (through tools/work.py's `load`), the
trailers of merge commits (`Work-Item:`, `Executor:`, `Conflicts:`, `Post-Merge-Verify:`) and of `Fixes:` commits,
and the harvested calls of work/telemetry/harvest/*.jsonl (tools/work_harvest.py). It computes each metric exactly as
work/telemetry/DEFINITIONS.md defines it, and writes work/telemetry/ROLLUP.md (headed by that file's sha256) and
work/telemetry/rollup.json. Every figure states its coverage. Rows with source "backfill" are tabled apart and never
pooled with live ones. Costs are tokens × the prices-2026-09-28 rates (config/prices/2026-09-28.toml), labelled
"API-equivalent (subscription)"; orchestration overhead (calls joined to no item) is its own row.

`manifest` freezes the day's inputs: the sha256 and row count of every events and harvest file, of DEFINITIONS.md, of
the price snapshot and of the items' [closed] blocks, into work/telemetry/manifests/<date>.json. The work skills
commit it with their bookkeeping; git is the freeze.

Percentiles are by nearest rank. All times are UTC.

Usage:
  work_telemetry.py rollup [--today YYYY-MM-DD]
  work_telemetry.py manifest [--date YYYY-MM-DD]
"""
from __future__ import annotations

import argparse, datetime as dt, hashlib, json, math, re, statistics, sys, tomllib
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import work  # noqa: E402

PRICE_SNAPSHOT = "prices-2026-09-28"
PRICE_FILE = "config/prices/2026-09-28.toml"
ESCAPE_DAYS = 14
TOKEN_RATES = (("input_tokens", "input"), ("output_tokens", "output"), ("cache_read_tokens", "cache_read"),
               ("cache_write_5m_tokens", "cache_write_5m"), ("cache_write_1h_tokens", "cache_write_1h"))
COST_LABEL = "API-equivalent (subscription)"
MANIFEST_SCHEMA = "roko.work_manifest/1"
ROLLUP_SCHEMA = "roko.work_rollup/1"


def telemetry() -> Path:
    return work.REPO / "work" / "telemetry"


def parse_ts(s) -> dt.datetime | None:
    """An ISO 8601 timestamp (`Z` or an offset, fractions allowed) as an aware UTC datetime; None if it is not one."""
    if not isinstance(s, str):
        return None
    try:
        t = dt.datetime.fromisoformat(s.replace("Z", "+00:00"))
    except ValueError:
        return None
    return (t if t.tzinfo else t.replace(tzinfo=dt.timezone.utc)).astimezone(dt.timezone.utc)


def jsonl(path: Path) -> list[dict]:
    rows = []
    for ln in path.read_text().splitlines():
        if ln.strip():
            try:
                rows.append(json.loads(ln))
            except json.JSONDecodeError:
                rows.append({"_unreadable": ln[:80]})
    return rows


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def nearest_rank(values, q: float):
    """The q-quantile of `values` by nearest rank; None when there are none."""
    vals = sorted(values)
    return vals[max(0, math.ceil(q * len(vals)) - 1)] if vals else None


def spread(values) -> dict:
    vals = [v for v in values if v is not None]
    return {"n": len(vals), "median": statistics.median(vals) if vals else None, "p90": nearest_rank(vals, 0.9)}


def rate(num: int, den: int) -> dict:
    return {"num": num, "den": den, "share": round(num / den, 4) if den else None}


# ---------------------------------------------------------------- inputs

def load_events():
    """(live rows, backfill rows, invalid rows) from every events file; `reconciled` rows count as live."""
    live, backfill, bad = [], [], []
    for f in sorted((telemetry() / "events").glob("*.jsonl")) if (telemetry() / "events").exists() else []:
        for r in jsonl(f):
            if "_unreadable" in r or work.check_event(r):
                bad.append({"file": f.name, "row": r})
            else:
                (backfill if r["source"] == "backfill" else live).append({**r, "_ts": parse_ts(r["ts"])})
    return live, backfill, bad


def load_harvest() -> list[dict]:
    rows = []
    for f in sorted((telemetry() / "harvest").glob("*.jsonl")) if (telemetry() / "harvest").exists() else []:
        rows += [r for r in jsonl(f) if r.get("row") == "calls"]
    return rows


def load_prices() -> dict:
    p = work.REPO / PRICE_FILE
    if not p.exists():
        return {}
    data = tomllib.loads(p.read_text())
    return {m["slug"]: m for m in data.get("model", [])}


def base_model(model: str | None, prices: dict) -> str | None:
    """The snapshot slug a model id is priced as: itself, or its base id without a date or context suffix."""
    if not model:
        return None
    if model in prices:
        return model
    base = re.sub(r"(?:-\d{8}|\[[^\]]*\])+$", "", model)
    return base if base in prices else None


def call_cost(row: dict, prices: dict) -> float | None:
    """USD for one harvest calls row at the snapshot's rates (per 1M tokens); None when its model is not listed."""
    slug = base_model(row.get("model"), prices)
    if slug is None:
        return None
    return sum((row.get(k) or 0) * prices[slug][rate_key] for k, rate_key in TOKEN_RATES) / 1e6


def tokens(row: dict) -> int:
    return sum(row.get(k) or 0 for k, _ in TOKEN_RATES)


def commit_trailers(merges: bool) -> list[dict]:
    """Commits (merges only, or all) with their trailers: sha, ts, parents and every trailer value by key."""
    fmt = "%H%x1f%P%x1f%ct%x1f%(trailers:unfold,separator=%x1d)%x1e"
    out = []
    for rec in work.git("log", *(["--merges"] if merges else []), f"--format={fmt}").split("\x1e"):
        parts = rec.strip("\n").split("\x1f")
        if len(parts) < 4 or not parts[0]:
            continue
        tr = defaultdict(list)
        for t in parts[3].split("\x1d"):
            k, _, v = t.partition(":")
            if v.strip():
                tr[k.strip().lower()].append(v.strip())
        out.append({"sha": parts[0], "parents": parts[1].split(), "ts": dt.datetime.fromtimestamp(int(parts[2]), dt.timezone.utc),
                    "trailers": tr})
    return out


def branch_commits(merge_sha: str, iid: str) -> list[dict] | None:
    """The non-merge commits a merge brought in that are the item's: those naming it in `Work-Item:`, or naming no
    item. None when `merge_sha` is not a merge in this repository."""
    parents = work.git("rev-list", "--parents", "-n", "1", merge_sha).split()
    if len(parents) < 3:
        return None
    fmt = "%H%x1f%(trailers:key=Executor,valueonly,separator=%x1d)%x1f%(trailers:key=Work-Item,valueonly,separator=%x1d)%x1e"
    out = []
    for rec in work.git("log", "--no-merges", f"--format={fmt}", f"{parents[1]}..{parents[2]}").split("\x1e"):
        parts = rec.strip("\n").split("\x1f")
        if len(parts) < 3 or not parts[0]:
            continue
        items = {i.strip() for i in parts[2].split("\x1d") if i.strip()}
        if items and iid not in items:
            continue
        execs = [e.strip() for e in parts[1].split("\x1d") if e.strip()]
        out.append({"sha": parts[0][:9], "executor": execs[0] if execs else None})
    return out


# ---------------------------------------------------------------- per item

def first(rows, key="_ts"):
    rows = [r for r in rows if r.get(key)]
    return min(rows, key=lambda r: r[key]) if rows else None


def item_trails(items, live, merges, fixes, harvest, prices):
    """({item id: trail} for every merged item (DEFINITIONS, "Merged item"), [items closed done without a merge])."""
    by_item = defaultdict(list)
    for r in live:
        by_item[r["item"]].append(r)
    merge_tr = defaultdict(list)
    for m in merges:
        for iid in m["trailers"].get("work-item", []):
            merge_tr[iid].append(m)
    fixes_by = defaultdict(list)
    for c in fixes:
        for v in c["trailers"].get("fixes", []):
            for iid in work.ID_ANY.findall(v):
                fixes_by[iid].append(c["ts"])
    harvest_by = defaultdict(list)
    for r in harvest:
        if r.get("item"):
            harvest_by[r["item"]].append(r)
    trails, done_unmerged = {}, []
    for it in items:
        iid, closed = it["id"], it.get("closed") or {}
        if it.get("status") != "done" or closed.get("executor") == "verification-only":
            continue
        ev = sorted(by_item.get(iid, []), key=lambda r: r["_ts"])
        merged_ev = first([r for r in ev if r["event"] == "merged"])
        merge_commit = min(merge_tr.get(iid, []), key=lambda m: m["ts"], default=None)
        if merged_ev:
            merged_at, merge_sha = merged_ev["_ts"], merged_ev.get("merge_sha")
        elif merge_commit:
            merged_at, merge_sha = merge_commit["ts"], merge_commit["sha"][:9]
        else:
            done_unmerged.append(iid)
            continue
        upto = [r for r in ev if r["_ts"] <= merged_at]
        claims = [r for r in upto if r["event"] == "claim" and not r.get("renew")]
        releases = [r for r in upto if r["event"] == "release"]
        claim0 = claims[0] if claims else None
        executor = (claim0 or {}).get("executor") or closed.get("executor")
        size = (claim0 or {}).get("size") or closed.get("size") or it.get("size")
        trailer = merge_commit["trailers"] if merge_commit else {}
        conflicts = merged_ev.get("conflicts") if merged_ev else None
        if conflicts is None and trailer.get("conflicts"):
            conflicts = int(trailer["conflicts"][0]) if trailer["conflicts"][0].isdigit() else None
        post = [r for r in ev if r["event"] == "post-verify" and r["_ts"] >= merged_at]
        if post:
            post_ok = post[0].get("rc") == 0
        elif trailer.get("post-merge-verify"):
            post_ok = trailer["post-merge-verify"][0].lower() == "pass"
        else:
            post_ok = None
        forced = closed.get("forced")
        if forced is None and str(closed.get("by", "")).startswith("plan:"):
            forced = False  # a sync close from a passed plan task never is forced
        commits = branch_commits(merge_sha, iid) if merge_sha else None
        calls = [r for r in harvest_by.get(iid, []) if (parse_ts(r.get("first_ts")) or merged_at) <= merged_at]
        costs = [call_cost(r, prices) for r in calls]
        trails[iid] = {
            "item": iid, "kind": it.get("kind"), "size": size, "lane": it.get("lane"), "executor": executor,
            "merged_at": merged_at, "merge_sha": merge_sha, "claims": claims, "releases": releases,
            "first_claim": claim0["_ts"] if claim0 else None, "post_ok": post_ok, "forced": forced,
            "assist": closed.get("assist"), "conflicts": conflicts, "fixups": (merged_ev or {}).get("fixups"),
            "commits": commits,
            "interventions_logged": [r for r in ev if r["event"] == "intervention" and r["_ts"] <= merged_at],
            "escapes": [r["_ts"] for r in ev if r["event"] == "escape"] + fixes_by.get(iid, []),
            "calls": calls, "cost": sum(c for c in costs if c is not None) if calls else None,
            "unpriced_tokens": sum(tokens(r) for r, c in zip(calls, costs) if c is None),
            "tokens": sum(tokens(r) for r in calls), "tokens_by_join": dict(_sum_by(calls, "join")),
            "via": (claim0 or {}).get("via") or closed.get("via"),
        }
    return trails, done_unmerged


def _sum_by(rows, key) -> Counter:
    c = Counter()
    for r in rows:
        c[r.get(key)] += tokens(r)
    return c


def interventions(t: dict) -> tuple[list[str], bool, list[str]]:
    """(signals, complete trail, branch commits without an Executor: trailer) for one merged item."""
    signals = []
    unlabelled = [c["sha"] for c in t["commits"] or [] if not c["executor"]]
    if t["executor"] and any(c["executor"] and c["executor"] != t["executor"] for c in t["commits"] or []):
        signals.append("other-executor-commit")
    if (t["fixups"] or 0) >= 1:
        signals.append("fixups")
    if t["forced"] is True:
        signals.append("forced-close")
    if any(r.get("reason") == "decision-needed" for r in t["releases"]):
        signals.append("decision-needed")
    if t["interventions_logged"]:
        signals.append("intervention-event")
    if t["assist"]:
        signals.append("assist")
    complete = bool(t["claims"]) and t["forced"] is not None and t["commits"] is not None and not unlabelled
    return signals, complete, unlabelled


# ---------------------------------------------------------------- metrics

def metrics(trails: list[dict], today: dt.date, releases: list[dict] | None = None) -> dict:
    """Every metric of DEFINITIONS.md over `trails` (the merged items of one table cell), with coverage. Merges
    abandoned over a conflict are counted from `releases`, else from the trails' own releases."""
    n = len(trails)
    cov = lambda k: {"k": k, "n": n, "share": round(k / n, 4) if n else None}  # noqa: E731
    attempts = [len(t["claims"]) for t in trails if t["claims"]]
    ft_in = [t for t in trails if t["claims"] and t["forced"] is not None and t["post_ok"] is not None]
    ft = [t for t in ft_in if len(t["claims"]) == 1 and not t["releases"] and t["forced"] is False and t["post_ok"]]
    hours = [(t["merged_at"] - t["first_claim"]).total_seconds() / 3600 for t in trails if t["first_claim"]]
    with_conflicts = [t for t in trails if t["conflicts"] is not None]
    post = [t for t in trails if t["post_ok"] is not None]
    window_over = [t for t in trails if t["merged_at"].date() + dt.timedelta(days=ESCAPE_DAYS) <= today]
    escaped = [t for t in window_over
               if any(t["merged_at"] <= e <= t["merged_at"] + dt.timedelta(days=ESCAPE_DAYS) for e in t["escapes"])]
    iv = [(t, *interventions(t)) for t in trails]
    complete = [x for x in iv if x[2]]
    costed = [t for t in trails if t["calls"]]
    return {
        "merged_items": n,
        "attempts": {"distribution": {"1": attempts.count(1), "2": attempts.count(2), "3+": sum(a >= 3 for a in attempts)},
                     "mean": round(statistics.mean(attempts), 3) if attempts else None, "coverage": cov(len(attempts))},
        "first_try_merge": {**rate(len(ft), len(ft_in)), "coverage": cov(len(ft_in))},
        "claim_to_merge_hours": {**spread(hours), "coverage": cov(len(hours))},
        "conflict_rate": {**rate(sum(t["conflicts"] >= 1 for t in with_conflicts), len(with_conflicts)),
                          "abandoned_over_conflict": sum(r.get("reason") == "conflict" for r in
                                                         (releases if releases is not None else
                                                          [x for t in trails for x in t["releases"]])),
                          "coverage": cov(len(with_conflicts))},
        "post_merge_verify_failure": {**rate(sum(not t["post_ok"] for t in post), len(post)), "coverage": cov(len(post))},
        "escape": {**rate(len(escaped), len(window_over)), "pending": n - len(window_over), "coverage": cov(len(window_over))},
        "interventions": {"per_item": round(statistics.mean(len(s) for _, s, c, _ in complete), 3) if complete else None,
                          "items_with_one": rate(sum(bool(s) for _, s, c, _ in complete), len(complete)),
                          "coverage": cov(len(complete))},
        "unassisted_merge_share": {**rate(sum(not s for _, s, c, _ in complete), len(complete)), "coverage": cov(len(complete))},
        "cost_per_merged_item_usd": {**spread([t["cost"] for t in costed if t["cost"] is not None]),
                                     "total": round(sum(t["cost"] or 0 for t in costed), 4),
                                     "unpriced_token_share": (round(sum(t["unpriced_tokens"] for t in costed)
                                                                    / max(1, sum(t["tokens"] for t in costed)), 4)
                                                              if costed else None),
                                     "label": COST_LABEL, "coverage": cov(len(costed))},
    }


def overhead(harvest: list[dict], prices: dict, merged: int) -> dict:
    """Orchestration overhead: harvested calls joined to no item, by origin, and per merged item."""
    rows = [r for r in harvest if not r.get("item")]
    by_origin = defaultdict(float)
    for r in rows:
        by_origin[r.get("origin") or "unknown"] += call_cost(r, prices) or 0
    total = sum(by_origin.values())
    return {"total_usd": round(total, 4), "by_origin_usd": {k: round(v, 4) for k, v in sorted(by_origin.items())},
            "per_merged_item_usd": round(total / merged, 4) if merged else None, "calls_rows": len(rows), "label": COST_LABEL}


def build(today: dt.date) -> dict:
    items = work.load("work")[0]  # local items stay private
    live, backfill, bad = load_events()
    harvest, prices = load_harvest(), load_prices()
    trails, done_unmerged = item_trails(items, live, commit_trailers(True), commit_trailers(False), harvest, prices)
    merged = sorted(trails.values(), key=lambda t: (t["merged_at"], t["item"]))
    groups = {
        "day": lambda t: t["merged_at"].date().isoformat(),
        "executor": lambda t: t["executor"] or "unknown",
        "kind_size": lambda t: f"{t['kind']} × {t['size'] or '?'}",
        "lane": lambda t: t["lane"] or "none",
    }
    tables = {}
    for name, key in groups.items():
        cells = defaultdict(list)
        for t in merged:
            cells[key(t)].append(t)
        tables[name] = {k: metrics(v, today) for k, v in sorted(cells.items())}
    defs = telemetry() / "DEFINITIONS.md"
    return {
        "schema": ROLLUP_SCHEMA, "date": today.isoformat(), "head": work.head_rev(),
        "definitions_sha256": sha256(defs.read_bytes()) if defs.exists() else None,
        "price_snapshot": PRICE_SNAPSHOT, "labels": ["observational", COST_LABEL],
        "inputs": {"live_events": len(live), "backfill_events": len(backfill), "invalid_events": len(bad),
                   "harvest_calls_rows": len(harvest)},
        "all": metrics(merged, today, releases=[r for r in live if r["event"] == "release"]),
        "tables": tables,
        "overhead": overhead(harvest, prices, len(merged)),
        "merged": [{"item": t["item"], "merged_at": work.utc_ts(t["merged_at"]), "executor": t["executor"],
                    "attempts": len(t["claims"]), "cost_usd": None if t["cost"] is None else round(t["cost"], 4),
                    "tokens_by_join": t["tokens_by_join"], "interventions": interventions(t)[0],
                    "unlabelled_commits": interventions(t)[2]} for t in merged],
        "done_without_merge": sorted(done_unmerged),
        "merged_without_claim": [t["item"] for t in merged if not t["claims"]],
        "backfill": {"closed_by_executor": dict(Counter(r["executor"] or "unknown" for r in backfill if r["event"] == "closed")),
                     "lane_starts": sum(r["event"] == "lane-start" for r in backfill)},
        "switch": "not fixed: the tracker item \"Roko executes work items\" is not filed yet",
    }


# ---------------------------------------------------------------- outputs

def fmt_rate(r: dict) -> str:
    return f"{r['num']}/{r['den']}" + (f" ({r['share']:.0%})" if r.get("share") is not None else "")


def fmt_cov(c: dict) -> str:
    return f"{c['k']} of {c['n']} merged items" + (f" ({c['share']:.0%})" if c.get("share") is not None else "")


def fmt_num(v, unit="") -> str:
    return "–" if v is None else (f"{v:.2f}{unit}" if isinstance(v, float) else f"{v}{unit}")


def render(r: dict) -> str:
    a = r["all"]
    out = ["# Development record: rollup", "",
           "Not for workers: no view, skill prompt or `work.py next` reads this, and no metric here is a target.", "",
           f"DEFINITIONS.md sha256 `{r['definitions_sha256']}` · report date {r['date']} · HEAD `{r['head']}` · "
           f"prices `{r['price_snapshot']}`. Every figure is observational; costs are {COST_LABEL}. Percentiles are "
           "by nearest rank; times are UTC.", "",
           f"Inputs: {r['inputs']['live_events']} live events, {r['inputs']['backfill_events']} backfill events "
           f"(tabled apart), {r['inputs']['invalid_events']} invalid event rows skipped, "
           f"{r['inputs']['harvest_calls_rows']} harvested call rows.", "",
           "## All merged items", "",
           "| Metric | Value | Coverage |", "|---|---|---|",
           f"| Merged items | {a['merged_items']} | |",
           f"| Attempts (1 / 2 / 3+; mean) | {a['attempts']['distribution']['1']} / {a['attempts']['distribution']['2']} / "
           f"{a['attempts']['distribution']['3+']}; {fmt_num(a['attempts']['mean'])} | {fmt_cov(a['attempts']['coverage'])} |",
           f"| First-try merges | {fmt_rate(a['first_try_merge'])} | {fmt_cov(a['first_try_merge']['coverage'])} |",
           f"| Claim-to-merge hours (median / p90) | {fmt_num(a['claim_to_merge_hours']['median'])} / "
           f"{fmt_num(a['claim_to_merge_hours']['p90'])} | {fmt_cov(a['claim_to_merge_hours']['coverage'])} |",
           f"| Conflict rate (abandoned over a conflict: {a['conflict_rate']['abandoned_over_conflict']}) | "
           f"{fmt_rate(a['conflict_rate'])} | {fmt_cov(a['conflict_rate']['coverage'])} |",
           f"| Post-merge verify failures | {fmt_rate(a['post_merge_verify_failure'])} | "
           f"{fmt_cov(a['post_merge_verify_failure']['coverage'])} |",
           f"| Escapes ({a['escape']['pending']} pending) | {fmt_rate(a['escape'])} | {fmt_cov(a['escape']['coverage'])} |",
           f"| Interventions per item (items with one) | {fmt_num(a['interventions']['per_item'])} "
           f"({fmt_rate(a['interventions']['items_with_one'])}) | {fmt_cov(a['interventions']['coverage'])} |",
           f"| Unassisted merge share | {fmt_rate(a['unassisted_merge_share'])} | {fmt_cov(a['unassisted_merge_share']['coverage'])} |",
           f"| Cost per merged item, USD (median / p90; total) | {fmt_num(a['cost_per_merged_item_usd']['median'])} / "
           f"{fmt_num(a['cost_per_merged_item_usd']['p90'])}; {fmt_num(a['cost_per_merged_item_usd']['total'])} | "
           f"{fmt_cov(a['cost_per_merged_item_usd']['coverage'])} |",
           f"| Orchestration overhead, USD (per merged item) | {fmt_num(r['overhead']['total_usd'])} "
           f"({fmt_num(r['overhead']['per_merged_item_usd'])}) | {r['overhead']['calls_rows']} unjoined call rows |", ""]
    titles = {"day": "By day of first merge", "executor": "By executor", "kind_size": "By kind × size", "lane": "By lane"}
    for name, title in titles.items():
        out += [f"## {title}", "", "| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |",
                "|---|---|---|---|---|---|"]
        for g, m in r["tables"][name].items():
            out.append(f"| {g} | {m['merged_items']} | {fmt_rate(m['first_try_merge'])} | "
                       f"{fmt_num(m['claim_to_merge_hours']['median'])} | {fmt_num(m['cost_per_merged_item_usd']['median'])} | "
                       f"{fmt_cov(m['first_try_merge']['coverage'])} |")
        out += ["- none" if not r["tables"][name] else "", ""]
    out += ["## Coverage", "",
            f"Items closed done without a merge (not counted): {len(r['done_without_merge'])}"
            + (f" ({', '.join(r['done_without_merge'][:40])}{', …' if len(r['done_without_merge']) > 40 else ''})"
               if r["done_without_merge"] else "") + ".", "",
            f"Items merged outside the skills (no claim event): {', '.join(r['merged_without_claim']) or 'none'}.", "",
            "Items with branch commits that carry no `Executor:` trailer (trail incomplete): "
            + (", ".join(f"{m['item']} ({len(m['unlabelled_commits'])})" for m in r["merged"] if m["unlabelled_commits"]) or "none")
            + ".", "",
            "## Backfill (labelled, not pooled)", "",
            "Reconstructed once from history (`tools/work_backfill.py`); never mixed into the tables above.", "",
            "| Executor | Closed items |", "|---|---|"]
    out += [f"| {k} | {v} |" for k, v in sorted(r["backfill"]["closed_by_executor"].items(), key=lambda kv: -kv[1])]
    out += [f"", f"Lane starts from the reflog: {r['backfill']['lane_starts']}.", "",
            "## The switch", "", f"The switch point is {r['switch']}.", ""]
    return "\n".join(out)


def cmd_rollup(a):
    today = dt.date.fromisoformat(a.today) if a.today else dt.datetime.now(dt.timezone.utc).date()
    r = build(today)
    d = telemetry()
    d.mkdir(parents=True, exist_ok=True)
    (d / "rollup.json").write_text(json.dumps(r, indent=1, default=str) + "\n")
    (d / "ROLLUP.md").write_text(render(r))
    print(f"rollup {today}: {r['all']['merged_items']} merged items; wrote work/telemetry/ROLLUP.md and rollup.json")


def manifest(date: str) -> dict:
    """The sha256 and row count of every input of the rollup."""
    files = []
    d = telemetry()
    paths = sorted([*(d / "events").glob("*.jsonl"), *(d / "harvest").glob("*.jsonl")]) + [d / "DEFINITIONS.md", work.REPO / PRICE_FILE]
    for p in paths:
        if p.exists():
            data = p.read_bytes()
            files.append({"path": str(p.relative_to(work.REPO)), "sha256": sha256(data),
                          "rows": len([ln for ln in data.decode(errors="replace").splitlines() if ln.strip()])})
    closed = sorted((i["id"], i.get("status"), i.get("closed") or {}) for i in work.load("work")[0] if i.get("closed"))
    files.append({"path": "work/items/*.md [closed]", "sha256": sha256(json.dumps(closed, default=str, sort_keys=True).encode()),
                  "rows": len(closed)})
    return {"schema": MANIFEST_SCHEMA, "date": date, "head": work.head_rev(), "files": files}


def cmd_manifest(a):
    date = a.date or dt.datetime.now(dt.timezone.utc).date().isoformat()
    m = manifest(date)
    out = telemetry() / "manifests" / f"{date}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(m, indent=1) + "\n")
    print(f"wrote {out.relative_to(work.REPO)}: {len(m['files'])} inputs")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sp = ap.add_subparsers(dest="cmd", required=True)
    p = sp.add_parser("rollup"); p.add_argument("--today", help="report date, YYYY-MM-DD (default: today, UTC)")
    p = sp.add_parser("manifest"); p.add_argument("--date", help="YYYY-MM-DD (default: today, UTC)")
    a = ap.parse_args()
    {"rollup": cmd_rollup, "manifest": cmd_manifest}[a.cmd](a)


if __name__ == "__main__":
    main()
