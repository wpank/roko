#!/usr/bin/env python3
"""Status matrix: every Roko mechanism with its status tag at one pinned commit (whitepaper appendix).

The data file holds one row per mechanism: its group, name, status tag and verdict, code anchors (`path::symbol`),
evidence (commits, tests or work items) and the work item that would change its tag. This tool checks every row
against the pinned commit and renders `docs/whitepaper/appendix-status-matrix.md` from it. The matrix is a dated
snapshot; the work graph in work/items stays the live status.

Usage:
  status_matrix.py                # check the rows at the pinned commit and write the appendix
  status_matrix.py --check        # check the rows, regenerate the appendix in memory, fail on any difference
  status_matrix.py --probe-head   # list rows whose anchors no longer exist at HEAD (exit 1 if any)

Options (paths are relative to --repo):
  --repo DIR      repository to read (default: the checkout this script is in)
  --data PATH     default docs/whitepaper/data/mechanisms.toml
  --out PATH      default docs/whitepaper/appendix-status-matrix.md

Row rules:
  - tag is one of TAGS, verdict one of VERDICTS, and every anchor exists at the pinned commit
    (`git cat-file -e` for the path, `git grep -w` for the symbol);
  - a row without anchors is MISSING, DOCS-ONLY or REMOVED and names an item;
  - evidence is non-empty: a commit that is an ancestor of the pin, a work item that exists at the pin,
    or a `path::symbol` test anchor;
  - text never carries a bare status tag, and every backticked span in it exists at the pin: a commit in the
    pin's history, a repository path, or an identifier in crates/.
A row that is not WIRED, has an actionable verdict and names no item is reported as a warning.
"""
from __future__ import annotations

import argparse, difflib, re, subprocess, sys, tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DATA = Path("docs/whitepaper/data/mechanisms.toml")
OUT = Path("docs/whitepaper/appendix-status-matrix.md")

# The tldr/00 status vocabulary, in the order the matrix lists it.
TAGS = {
    "WIRED": "Runs on the production path (plan runs through the Graph engine) and does its job: a sensor "
             "records, a regulator changes behaviour",
    "PARTIAL": "On the path but incomplete, masked, or fed wrong data",
    "BROKEN": "Wired, but does the wrong thing",
    "ORPHANED": "Worked under Runner-v2, the event loop deleted in `6b5da8616`, and was not re-attached",
    "BUILT-UNWIRED": "Code and tests exist, with no production caller",
    "DOCS-ONLY": "The docs describe it as working, but there is no meaningful code",
    "MISSING": "Not built",
    "REMOVED": "Deliberately deleted",
    "UNPROVEN": "Might be true; no measurement exists",
}
VERDICTS = {
    "keep": "It works; keep it",
    "wire": "The code exists; connect it to the production path",
    "fix": "It is connected but wrong or incomplete; correct it",
    "build": "Nothing exists yet; build it",
    "redesign": "Keep the goal, change the mechanism",
    "park": "Move it behind a feature flag, out of the default build, the core docs and the pitch",
    "remove": "Delete it, or the claim made for it",
}
ANCHORLESS_TAGS = {"MISSING", "DOCS-ONLY", "REMOVED"}
ACTIONABLE = {"wire", "fix", "build", "redesign"}
STATUSES = ("stub", "draft", "reviewed")
ITEM_DIRS = ("work/items", "work/parked")

ITEM_RE = re.compile(r"^(?:bug|gap|reg|find|spec|dec|q)-[0-9a-f]{6}$")
SHA_RE = re.compile(r"^[0-9a-f]{7,40}$")
IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
PATH_RE = re.compile(r"^[A-Za-z0-9_.@\-]+(?:/[A-Za-z0-9_.@\-]+)+/?$")
BARE_TAG_RE = re.compile(r"(?<![\w-])(" + "|".join(sorted(map(re.escape, TAGS), key=len, reverse=True)) + r")(?![\w-])(?!@)")
TICK_RE = re.compile(r"`([^`]*)`")


class Git:
    """Read-only queries against one repository, cached."""

    def __init__(self, repo: Path):
        self.repo = repo
        self._cache: dict[tuple, object] = {}

    def _run(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(["git", *args], cwd=self.repo, capture_output=True, text=True)

    def _cached(self, key: tuple, fn):
        if key not in self._cache:
            self._cache[key] = fn()
        return self._cache[key]

    def commit(self, rev: str) -> str | None:
        def resolve():
            p = self._run("rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}")
            return p.stdout.strip() if p.returncode == 0 else None
        return self._cached(("commit", rev), resolve)

    def commit_date(self, rev: str) -> str:
        return self._run("show", "-s", "--format=%cs", rev).stdout.strip()

    def path_exists(self, rev: str, path: str) -> bool:
        return self._cached(("path", rev, path), lambda: self._run("cat-file", "-e", f"{rev}:{path}").returncode == 0)

    def word_in(self, rev: str, word: str, path: str) -> bool:
        key = ("word", rev, word, path)
        return self._cached(key, lambda: self._run("grep", "-q", "-w", "-F", "-e", word, rev, "--", path).returncode == 0)

    def is_ancestor(self, older: str, newer: str) -> bool:
        return self._cached(("anc", older, newer),
                            lambda: self._run("merge-base", "--is-ancestor", older, newer).returncode == 0)

    def item_ids(self, rev: str) -> set[str]:
        def ids():
            out = self._run("ls-tree", "-r", "--name-only", rev, "--", *ITEM_DIRS).stdout.split()
            return {m.group(1) for f in out if (m := re.match(r"^([a-z]+-[0-9a-f]{6})-", Path(f).name))}
        return self._cached(("items", rev), ids)


def split_anchor(anchor: str) -> tuple[str, str | None]:
    """'path::symbol' or 'path' → (path, symbol). The same `path::symbol` form as the work items' anchors, but
    checked against a commit rather than the working tree."""
    path, _, sym = anchor.strip().partition("::")
    return path.rstrip("/"), (sym or None)


def anchor_problem(git: Git, rev: str, anchor: str) -> str | None:
    path, sym = split_anchor(anchor)
    if not PATH_RE.match(path):
        return f"anchor {anchor!r} is not a repository path"
    if sym is not None and not IDENT_RE.match(sym):
        return f"anchor {anchor!r}: the symbol must be one identifier"
    if not git.path_exists(rev, path):
        return f"anchor {anchor!r}: {path} does not exist at {rev}"
    if sym is not None and not git.word_in(rev, sym, path):
        return f"anchor {anchor!r}: {sym} is not in {path} at {rev}"
    return None


def load(data_path: Path) -> dict:
    with open(data_path, "rb") as f:
        return tomllib.load(f)


def text_problems(git: Git, pin: str, where: str, text: str) -> list[str]:
    errs = [f"{where}: bare status tag {m.group(1)}; write it as {m.group(1)}@<commit> or rephrase"
            for m in BARE_TAG_RE.finditer(TICK_RE.sub("", text))]
    for span in TICK_RE.findall(text):
        if SHA_RE.match(span):
            full = git.commit(span)
            if full is None or not git.is_ancestor(full, git.commit(pin)):
                errs.append(f"{where}: commit `{span}` is not in the history of the pin {pin}")
        elif PATH_RE.match(span):
            if not git.path_exists(pin, span.rstrip("/")):
                errs.append(f"{where}: `{span}` does not exist at {pin}")
        elif IDENT_RE.match(span):
            if not git.word_in(pin, span, "crates"):
                errs.append(f"{where}: `{span}` is not in crates/ at {pin}")
        else:
            errs.append(f"{where}: `{span}` is neither one identifier nor a repository path")
    return errs


def validate(git: Git, data: dict) -> tuple[list[str], list[str], str | None]:
    """→ (errors, warnings, full sha of the pin)."""
    errs: list[str] = []
    warns: list[str] = []
    meta = data.get("matrix", {})
    pin = str(meta.get("pinned", ""))
    full = git.commit(pin) if SHA_RE.match(pin) else None
    if full is None:
        return [f"matrix: pinned {pin!r} is not a commit in this repository"], warns, None
    if meta.get("status") not in STATUSES:
        errs.append(f"matrix: status must be one of {', '.join(STATUSES)}")
    if not ITEM_RE.match(str(meta.get("owner", ""))):
        errs.append("matrix: owner must be a work item id")
    if not str(meta.get("budget", "")).strip():
        errs.append("matrix: budget must be set (a word count or 'none')")
    for field in ("title", "intro", "claims_intro"):
        errs += text_problems(git, pin, f"matrix {field}", str(meta.get(field, "")))
    items = git.item_ids(pin)

    def check_item(where: str, iid: str):
        if not ITEM_RE.match(iid):
            errs.append(f"{where}: {iid!r} is not a work item id")
        elif iid not in items:
            errs.append(f"{where}: work item {iid} does not exist at {pin}")

    groups = data.get("group", [])
    gids = [g.get("id", "") for g in groups]
    for g in groups:
        if not g.get("id") or not g.get("name"):
            errs.append(f"group {g!r}: needs an id and a name")
        errs += text_problems(git, pin, f"group {g.get('id')}", f"{g.get('name', '')} {g.get('blurb', '')}")
    if len(set(gids)) != len(gids):
        errs.append("group: ids must be unique")

    rows = data.get("row", [])
    rids = [r.get("id", "") for r in rows]
    if not rows:
        errs.append("row: the matrix has no rows")
    if len(set(rids)) != len(rids):
        errs.append("row: ids must be unique: " + ", ".join(sorted({i for i in rids if rids.count(i) > 1})))
    for r in rows:
        rid = r.get("id") or "?"
        where = f"row {rid}"
        if r.get("group") not in gids:
            errs.append(f"{where}: unknown group {r.get('group')!r}")
        elif not re.fullmatch(re.escape(r["group"]) + r"\d+", rid):
            errs.append(f"{where}: the id must be its group id followed by a number")
        for field in ("name", "note"):
            if not str(r.get(field, "")).strip():
                errs.append(f"{where}: {field} is empty")
            else:
                errs += text_problems(git, pin, f"{where} {field}", r[field])
        tag, verdict = r.get("tag"), r.get("verdict")
        if tag not in TAGS:
            errs.append(f"{where}: unknown tag {tag!r}")
        if verdict not in VERDICTS:
            errs.append(f"{where}: unknown verdict {verdict!r}")
        anchors = r.get("anchors", [])
        for a in anchors:
            if (p := anchor_problem(git, pin, a)) is not None:
                errs.append(f"{where}: {p}")
        item = r.get("item", "")
        if item:
            check_item(where, item)
        if not anchors:
            if tag not in ANCHORLESS_TAGS:
                errs.append(f"{where}: a row without anchors must be tagged {', '.join(sorted(ANCHORLESS_TAGS))}")
            if not item:
                errs.append(f"{where}: a row without anchors must name the item or spec that would build it")
        elif tag != "WIRED" and verdict in ACTIONABLE and not item:
            warns.append(f"{where}: {tag} with verdict {verdict!r} but no work item filed")
        evidence = r.get("evidence", [])
        if not evidence:
            errs.append(f"{where}: evidence is empty")
        for e in evidence:
            if SHA_RE.match(e):
                full_e = git.commit(e)
                if full_e is None:
                    errs.append(f"{where}: evidence commit {e} does not exist")
                elif not git.is_ancestor(full_e, full):
                    errs.append(f"{where}: evidence commit {e} is not an ancestor of the pin {pin}")
            elif ITEM_RE.match(e):
                check_item(where, e)
            elif "/" in e:
                if (p := anchor_problem(git, pin, e)) is not None:
                    errs.append(f"{where}: evidence {p}")
            else:
                errs.append(f"{where}: evidence {e!r} is not a commit, a work item or a path::symbol anchor")

    for c in data.get("claim", []):
        where = f"claim {c.get('id') or '?'}"
        if c.get("tag") not in TAGS:
            errs.append(f"{where}: unknown tag {c.get('tag')!r}")
        for field in ("name", "blocker"):
            if not str(c.get(field, "")).strip():
                errs.append(f"{where}: {field} is empty")
            else:
                errs += text_problems(git, pin, f"{where} {field}", c[field])
        unknown = [x for x in c.get("rows", []) if x not in rids]
        if not c.get("rows") or unknown:
            errs.append(f"{where}: rows must name existing rows" + (f" (unknown: {', '.join(unknown)})" if unknown else ""))
    return errs, warns, full


def cell(text: str) -> str:
    return " ".join(str(text).split()).replace("|", "\\|")


def render_anchors(anchors: list[str]) -> str:
    by_path: dict[str, list[str]] = {}
    for a in anchors:
        path, sym = split_anchor(a)
        by_path.setdefault(path, [])
        if sym:
            by_path[path].append(sym)
    return "; ".join(f"`{p}`" + (": " + ", ".join(f"`{s}`" for s in syms) if syms else "")
                     for p, syms in by_path.items())


def render_evidence(evidence: list[str], pin: str) -> str:
    out = []
    for e in evidence:
        if SHA_RE.match(e):
            out.append(f"code read at `{e}`" if e == pin else f"`{e}`")
        elif ITEM_RE.match(e):
            out.append(e)
        else:
            out.append("test " + render_anchors([e]))
    return ", ".join(out)


def render(data: dict, git: Git, pin: str) -> str:
    meta = data["matrix"]
    tag_at = lambda t: f"{t}@{pin}"  # noqa: E731
    rows, groups = data["row"], data["group"]
    used = [t for t in TAGS if any(r["tag"] == t for r in rows) or any(c["tag"] == t for c in data.get("claim", []))]
    lines = [
        f"Status: {meta['status']} · budget {meta['budget']} · owner {meta['owner']}",
        "",
        f"# {meta.get('title', 'Appendix: status matrix')}",
        "",
        cell(meta.get("intro", "")).replace("{pin}", pin).replace("{date}", git.commit_date(pin)),
        "",
        "Each row gives the mechanism, its status at the pinned commit, what runs today, the code that implements it, "
        "the evidence for the tag and the next step. A code anchor names a file and a symbol in it; "
        "`tools/status_matrix.py` fails if either is missing at the pinned commit. Evidence is a commit, a test, or a "
        "work item whose recorded state supports the tag. The next step is a verdict and, where one is filed, the work "
        "item that would change the tag.",
        "",
        "| Status | Meaning |",
        "|---|---|",
        *[f"| {tag_at(t)} | {cell(TAGS[t])} |" for t in used],
        "",
        "| Verdict | Meaning |",
        "|---|---|",
        *[f"| {v} | {cell(m)} |" for v, m in VERDICTS.items()],
        "",
        "## Summary",
        "",
        "Rows per status and group at " + pin + ".",
        "",
        "| Status | " + " | ".join(g["id"] for g in groups) + " | Total |",
        "|---|" + "---|" * (len(groups) + 1),
    ]
    for t in TAGS:
        counts = [sum(1 for r in rows if r["group"] == g["id"] and r["tag"] == t) for g in groups]
        if sum(counts):
            lines.append(f"| {tag_at(t)} | " + " | ".join(str(n) for n in counts) + f" | {sum(counts)} |")
    lines.append("| Rows | " + " | ".join(str(sum(1 for r in rows if r["group"] == g["id"])) for g in groups)
                 + f" | {len(rows)} |")
    for g in groups:
        lines += ["", f"## {g['id']} · {g['name']}", ""]
        if g.get("blurb"):
            lines += [cell(g["blurb"]), ""]
        lines += ["| Row | Mechanism | Status | Today | Code | Evidence | Next |", "|---|---|---|---|---|---|---|"]
        for r in (r for r in rows if r["group"] == g["id"]):
            code = render_anchors(r.get("anchors", [])) or "none"
            nxt = r["verdict"] + (f" · {r['item']}" if r.get("item") else
                                  " · no item filed" if r["tag"] != "WIRED" and r["verdict"] in ACTIONABLE else "")
            lines.append(f"| {r['id']} | {cell(r['name'])} | {tag_at(r['tag'])} | {cell(r['note'])} | {code} | "
                         f"{render_evidence(r['evidence'], meta['pinned'])} | {nxt} |")
    claims = data.get("claim", [])
    if claims:
        lines += ["", "## Vision claims", ""]
        if meta.get("claims_intro"):
            lines += [cell(meta["claims_intro"]), ""]
        lines += ["| Claim | Status | Rests on | Main blocker |", "|---|---|---|---|"]
        for c in claims:
            lines.append(f"| {c['id']} {cell(c['name'])} | {tag_at(c['tag'])} | {', '.join(c['rows'])} | "
                         f"{cell(c['blocker'])} |")
    return "\n".join(lines) + "\n"


def probe_head(git: Git, data: dict) -> list[tuple[str, str, str]]:
    """Rows whose anchors no longer exist at HEAD → [(row id, anchor, problem)]."""
    head = git.commit("HEAD")
    stale = []
    for r in data.get("row", []):
        for a in r.get("anchors", []):
            if (p := anchor_problem(git, head, a)) is not None:
                stale.append((r.get("id", "?"), a, p))
    return stale


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="fail if a row is bad or the appendix is out of date")
    mode.add_argument("--probe-head", action="store_true", help="list rows whose anchors are gone at HEAD")
    ap.add_argument("--repo", type=Path, default=REPO)
    ap.add_argument("--data", type=Path, default=DATA)
    ap.add_argument("--out", type=Path, default=OUT)
    args = ap.parse_args(argv)
    repo = args.repo.resolve()
    git = Git(repo)
    data_path, out_path = repo / args.data, repo / args.out
    try:
        data = load(data_path)
    except (OSError, tomllib.TOMLDecodeError) as e:
        print(f"status_matrix: cannot read {args.data}: {e}", file=sys.stderr)
        return 1

    if args.probe_head:
        stale = probe_head(git, data)
        for rid, anchor, problem in stale:
            print(f"{rid}\t{anchor}\t{problem}")
        print(f"status_matrix: {len(stale)} anchor(s) gone at HEAD" if stale else "status_matrix: every anchor exists at HEAD",
              file=sys.stderr)
        return 1 if stale else 0

    errs, warns, full = validate(git, data)
    for w in warns:
        print(f"status_matrix: warning: {w}", file=sys.stderr)
    for e in errs:
        print(f"status_matrix: {e}", file=sys.stderr)
    if errs:
        return 1
    text = render(data, git, data["matrix"]["pinned"])
    if args.check:
        try:
            current = out_path.read_text()
        except OSError:
            current = ""
        if current != text:
            diff = difflib.unified_diff(current.splitlines(), text.splitlines(), str(args.out), "regenerated", lineterm="")
            print("\n".join(list(diff)[:40]), file=sys.stderr)
            print(f"status_matrix: {args.out} is out of date; run python3 tools/status_matrix.py", file=sys.stderr)
            return 1
        print(f"status_matrix: {len(data['row'])} rows ok at {data['matrix']['pinned']}; {args.out} is current")
        return 0
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(text)
    print(f"status_matrix: wrote {args.out} ({len(data['row'])} rows at {data['matrix']['pinned']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
