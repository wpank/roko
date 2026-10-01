#!/usr/bin/env python3
"""Import the research programme's open checklist rows as unverified work items (gap-25065c).

One-shot. `--dry-run` lists every in-scope row as imported (with the id it will get) or skipped (with the item that
already covers it), and writes nothing; Will reviews that list first. Without `--dry-run` it writes the items with
work.py's `new` checks (gap-2bc1b9), all `triage = "unverified"`, and writes nothing at all if any row has a problem.

A row is covered, and skipped, when a W3a/W3b crosswalk marks it TRACKED or CLOSED, or when an existing gap, bug,
regression or finding (or a workstreams manifest entry) names its id. PARTIAL rows are imported, related to the items
that cover part of them.

Usage:
  python3 tools/work_import_checklist.py [--dry-run] [--checklist PATH] [--manifest PATH] [--crosswalk PATH]...
Run it in the main checkout: the checklist and the crosswalks live in its untracked tmp/cybernetic-harness/.
"""

import argparse
import datetime as dt
import json
import re
import sys
from argparse import Namespace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work  # noqa: E402

HARNESS = "tmp/cybernetic-harness"
CHECKLIST = f"{HARNESS}/execution/checklist.json"
MANIFEST = f"{HARNESS}/workstreams/manifest.json"
CROSSWALKS = [f"{HARNESS}/workstreams/assessment/W3a-crosswalk-core.md",
              f"{HARNESS}/workstreams/assessment/W3b-crosswalk-product.md"]
OPEN_ROWS = {"todo", "partial"}
SCOPE_SPECS = {"S02", "S03", "S04", "S05", "S06", "S07", "S10", "S11"}
# Open rows that no epic took (wk-filer, 2026-09-29), outside the specs above.
EXTRA_ROWS = ({f"S01.P0-{n}" for n in (3, 4, 5, 7, 8, 9, 10)}
              | {f"S08.T{n}" for n in (8, 9, 10, 14, 15, 16)} | {f"S08.T17{x}" for x in "abcdf"}
              | {f"S09.E{n}" for n in range(3, 12)} | {"E2", "E3", "E4", "E6", "E11", "E13"})
# Checklist lanes → work lanes. L5 (deploy) goes to rust-cold, whose paths hold .github/workflows.
LANES = {"L1": "bench", "L2": "rust-cold", "L3": "rust-hot", "L4": "frontend", "L5": "rust-cold", "L6": "bench",
         "L7": "paper", "L8": "paper", "L9": "paper"}
SIZES = {"S": "S", "M": "M", "L": "L", "ML": "L"}
COVERED = ("TRACKED", "CLOSED")
COVERING_KINDS = {"gap", "bug", "regression", "finding"}
ROW_ID = r"S\d\d\.[A-Za-z]+\d+[a-z]?(?:-\d+[a-z]?)?"
SELF = "gap-25065c"


def placement(spec: str) -> dict:
    """Goal, parent epic and hold for a row of `spec` (gap-25065c plan step 3)."""
    if spec in ("S02", "S03", "S04", "S05", "S06"):
        return {"goal": "cybernetic", "parent": "spec-6ac537", "hold": None}
    return {
        "S01": {"goal": "truth", "parent": "spec-b7303f", "hold": None},
        "S07": {"goal": "golden-path", "parent": "spec-e57870", "hold": None},
        "S08": {"goal": "proof", "parent": "spec-567e52", "hold": None},
        "S09": {"goal": "proof", "parent": "spec-567e52", "hold": None},
        "S10": {"goal": "visibility", "parent": None, "hold": None},
        "S11": {"goal": "release", "parent": None, "hold": "after the golden path"},
        "E": {"goal": "whitepaper", "parent": "spec-f8d196", "hold": None},
    }[spec]


def in_scope(row: dict) -> bool:
    return row["status"] in OPEN_ROWS and (row["spec"] in SCOPE_SPECS or row["id"] in EXTRA_ROWS)


def row_ids(cell: str) -> list[str]:
    """Checklist ids at the start of a crosswalk cell: `S01.P0-4/8/9/10 …` → S01.P0-4, S01.P0-8, S01.P0-9, S01.P0-10."""
    m = re.match(rf"\s*({ROW_ID}|[DEPR]\d+[a-z]?)((?:/[\w-]+)*)(?=[\s,(]|$)", cell)
    if not m:
        return []
    first, rest = m.group(1), [x for x in m.group(2).split("/") if x]
    stem = re.sub(r"\w+$", "", first)
    return [first] + [stem + x for x in rest]


def crosswalk_coverage(paths) -> dict[str, tuple[str, list[str]]]:
    """Row id → (status word, item ids named), from each crosswalk table row `| id … | items | STATUS | …`."""
    cover = {}
    for path in paths:
        for line in Path(path).read_text().splitlines():
            if not line.lstrip().startswith("|"):
                continue
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            status = cells[2].split()[0].upper() if len(cells) > 2 and cells[2] else ""
            if status not in ("NEW", "PARTIAL", "TRACKED", "CLOSED"):
                continue
            for rid in row_ids(cells[0]):
                cover[rid] = (status, work.ID_ANY.findall(cells[1]))
    return cover


def mentions(items, manifest_path) -> dict[str, list[str]]:
    """Row id → ids of the gaps, bugs, regressions and findings (and manifest entries) that name it."""
    texts = [(it["id"], " ".join(str(it.get(k) or "") for k in ("title", "source", "discovered_from")) + it["_body"])
             for it in items if it.get("kind") in COVERING_KINDS and it["id"] != SELF]
    if manifest_path and Path(manifest_path).exists():
        texts += [(e["id"], " ".join(str(e.get(k) or "") for k in ("title", "hint", "source")))
                  for e in json.loads(Path(manifest_path).read_text()).get("items", [])
                  if e.get("id") and e["id"] != SELF and e.get("kind") in COVERING_KINDS]
    found: dict[str, list[str]] = {}
    for iid, text in texts:
        for rid in set(re.findall(rf"(?<![\w.]){ROW_ID}(?![\w-])", text)):
            found.setdefault(rid, []).append(iid)
    return found


def existing_anchors(row: dict) -> list[str]:
    """files_owned as anchors that exist: a glob becomes its directory, a new file its nearest existing parent."""
    out = []
    for f in row.get("files_owned") or []:
        p = re.split(r"[*{]", f)[0].rstrip("/")
        while p and not (work.REPO / p).exists():
            p = str(Path(p).parent) if "/" in p else ""
        if p and p not in out:
            out.append(p)
    if not out:
        spec = next(work.REPO.glob(f"{HARNESS}/specs/{row['spec']}-*.md"), None)
        out = [str(spec.relative_to(work.REPO))] if spec else [CHECKLIST]
    return out


def subsystems(anchors) -> str:
    """`crates/roko-learn/…` → roko-learn; `tmp/cybernetic-harness/specs/…` → cybernetic-harness/specs; else two parts."""
    out = []
    for a in anchors:
        parts = a.split("/")
        sub = (parts[1] if parts[0] == "crates" and len(parts) > 1 else
               "/".join(parts[1:3]) if parts[0] == "tmp" else "/".join(parts[:2]))
        if sub not in out:
            out.append(sub)
    return ",".join(out)


def guarded_verify(row: dict) -> str | None:
    """The row's verify command when it is runnable and guarded (it lints clean); else None."""
    m = re.search(r"`([^`]+)`", row.get("verify") or "")
    cmd = m.group(1).strip() if m else ""
    return cmd if re.match(r"(grep|test|python3|bash|!) ", cmd) and not work.lint_verify(cmd) else None


def body(row: dict, verify_in_text: bool) -> str:
    done = [f"- [ ] {row['acceptance'].strip()}"] if (row.get("acceptance") or "").strip() else []
    if verify_in_text and (row.get("verify") or "").strip():
        done.append(f"- [ ] The checklist's check: {row['verify'].strip()}")
    notes = [f"- Imported from `{CHECKLIST}#{row['id']}` by tools/work_import_checklist.py (gap-25065c). Check the "
             "premise before working on it.",
             f"- Checklist effort {row.get('effort') or '?'} (its S is up to half a day), hot: {row.get('hot') or '?'}, "
             f"lane {row.get('lane')}, milestone {row.get('milestone') or '-'}, deps: {row.get('deps_raw') or '-'}."]
    if (row.get("risk") or "").strip():
        notes.append(f"- Risk: {row['risk'].strip()}")
    parts = ["## Problem", "", f"{row['title'].strip()} (research checklist row {row['id']}, spec {row['spec']}).", ""]
    if (row.get("notes") or "").strip():
        parts += [row["notes"].strip(), ""]
    plan = (row.get("parallel_notes") or "").strip() or f"Follow {row['spec']}'s plan for this row."
    parts += ["## Where", "", (row.get("files") or "See the spec.").strip(), "", "## Plan", "", plan, "",
              "## Done when", "", *(done or ["- [ ] The spec's acceptance for this row holds."]), "", "## Notes", "",
              *notes]
    return "\n".join(parts) + "\n"


def plan_import(rows, cover, named, created) -> tuple[list[dict], list[tuple[str, str]]]:
    """(in-scope rows to import, dependencies first, each with its item id; (row id, why) for every skip)."""
    skips, chosen = [], {}
    for row in rows:
        if not in_scope(row):
            continue
        status, ids = cover.get(row["id"], ("", []))
        if status in COVERED:
            skips.append((row["id"], f"crosswalk {status}: {', '.join(ids) or 'no item named'}"))
        elif named.get(row["id"]):
            skips.append((row["id"], f"named by {', '.join(sorted(set(named[row['id']])))}"))
        else:
            title = f"{row['title'].strip().rstrip('.')} ({row['id']})"
            iid = work.make_id("gap", title, created, f"{CHECKLIST}#{row['id']}")
            chosen[row["id"]] = {"row": row, "id": iid, "title": title, "partial": ids if status == "PARTIAL" else []}
    ordered, seen = [], set()

    def visit(rid, path=()):
        if rid in seen or rid not in chosen or rid in path:
            return
        for dep in chosen[rid]["row"].get("deps") or []:
            visit(dep, path + (rid,))
        seen.add(rid)
        ordered.append(chosen[rid])

    for rid in chosen:
        visit(rid)
    for plan in ordered:
        deps = []
        for dep in plan["row"].get("deps") or []:
            if dep in chosen:
                deps.append(chosen[dep]["id"])
            else:  # covered elsewhere: depend on the covering item
                deps += [i for i in (cover.get(dep, ("", []))[1] or named.get(dep, []))[:1] if i not in deps]
        plan["depends_on"] = deps
    return ordered, skips


def namespace(plan: dict, created: str, milestones) -> Namespace:
    row, place = plan["row"], placement(plan["row"]["spec"])
    verify, anchors = guarded_verify(row), existing_anchors(row)
    return Namespace(
        kind="gap", title=plan["title"], source=f"{CHECKLIST}#{row['id']}", created=created, root="work",
        subsystem=subsystems(anchors), severity="p2", status="open", triage="unverified", goal=place["goal"],
        lane=LANES.get((row.get("lane") or "")[:2]), parent=place["parent"],
        milestone=row.get("milestone") if row.get("milestone") in milestones else None,
        size=SIZES.get(row.get("effort") or ""), rank=None, hold=place["hold"],
        discovered_from=f"{CHECKLIST} (gap-25065c import)", doc=None, anchor=anchors, verify=[verify] if verify else [],
        depends_on=",".join(plan["depends_on"]), related=",".join(plan["partial"]), blocks=None, body_file=None,
        no_verify_yet=not verify, dry_run=False)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--dry-run", action="store_true", help="list what would be imported or skipped; write nothing")
    ap.add_argument("--checklist", default=str(work.REPO / CHECKLIST))
    ap.add_argument("--manifest", default=str(work.REPO / MANIFEST))
    ap.add_argument("--crosswalk", action="append", help="repeatable; default: the W3a and W3b crosswalks")
    a = ap.parse_args(argv)
    created = dt.date.today().isoformat()
    rows = json.loads(Path(a.checklist).read_text())["items"]
    idx = work.index()
    work.index = lambda: idx  # one load for the whole run; planned ids are checked apart below
    crosswalks = [c for c in (a.crosswalk or [str(work.REPO / c) for c in CROSSWALKS]) if Path(c).exists()]
    plans, skips = plan_import(rows, crosswalk_coverage(crosswalks), mentions(list(idx.values()), a.manifest), created)
    planned, milestones = {p["id"] for p in plans}, work.load_lanes().get("milestones") or []
    lines, problems, writes = [], [], []
    for plan in plans:
        ns = namespace(plan, created, milestones)
        known = ",".join(d for d in work.comma_list(ns.depends_on) if d not in planned)
        problems += [f"{plan['row']['id']}: {e}" for e in work.new_problems(Namespace(**{**vars(ns), "depends_on": known}))]
        lines.append(f"import {plan['row']['id']:<11} -> {plan['id']}  {ns.goal}/{ns.parent or '-'}  lane {ns.lane or '-'}"
                     f"  size {ns.size or '-'}" + ("  hold" if ns.hold else "") + ("  verify" if ns.verify else "")
                     + (f"  deps {ns.depends_on}" if ns.depends_on else "") + (f"  related {ns.related}" if ns.related else ""))
        writes.append((work.ROOTS["work"] / work.home_dir("open") / f"{plan['id']}-{work.slug(plan['title'])}.md",
                       work.new_text(ns, plan["id"], created, body(plan["row"], verify_in_text=not ns.verify))))
    lines += [f"skip   {rid:<11} {why}" for rid, why in skips]
    if not a.dry_run and not problems:
        clash = [str(path) for path, _ in writes if path.exists()]
        if clash:
            sys.exit(f"already imported, nothing written: {', '.join(clash)}")
        for path, text in writes:
            path.write_text(text)
    verb = "would import" if a.dry_run or problems else "imported"
    print("\n".join(lines + [f"{verb} {len(plans)}, skipped {len(skips)}, problems {len(problems)}"] + problems))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
