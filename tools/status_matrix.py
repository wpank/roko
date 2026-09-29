#!/usr/bin/env python3
"""Status matrix: every Roko mechanism with its status tag at one pinned commit (whitepaper appendix).

The data file holds one row per mechanism: its group, name, status tag and verdict, code anchors (`path::symbol`),
evidence (commits, tests or work items) and the work item that would change its tag. This tool checks every row
against the pinned commit and renders `docs/whitepaper/appendix-status-matrix.md` and the whitepaper's Figure 3
(`docs/whitepaper/figures/fig3-status-matrix.svg`) from it. The matrix is a dated snapshot; the work graph in
work/items stays the live status.

Usage:
  status_matrix.py                # check the rows at the pinned commit, write the appendix and Figure 3
  status_matrix.py --check        # check the rows, regenerate both in memory, fail on any difference
  status_matrix.py --probe-head   # list rows whose anchors no longer exist at HEAD (exit 1 if any)

Options (paths are relative to --repo):
  --repo DIR      repository to read (default: the checkout this script is in)
  --data PATH     default docs/whitepaper/data/mechanisms.toml
  --out PATH      default docs/whitepaper/appendix-status-matrix.md
  --svg PATH      Figure 3; default docs/whitepaper/figures/fig3-status-matrix.svg

The other figures in Figure 3's directory are drawn by hand. Each status mark in them carries `data-row` and
`data-tag` attributes, and the root element `data-pin`. `--check` fails when a mark's tag differs from its row, or a
figure's pin from the matrix's; writing only warns.

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
import xml.etree.ElementTree as ET
from pathlib import Path
from xml.sax.saxutils import escape

REPO = Path(__file__).resolve().parents[1]
DATA = Path("docs/whitepaper/data/mechanisms.toml")
OUT = Path("docs/whitepaper/appendix-status-matrix.md")
SVG = Path("docs/whitepaper/figures/fig3-status-matrix.svg")

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
ON_PATH = ("WIRED", "PARTIAL", "BROKEN")  # the tags of mechanisms that run on the production path

# How the figures draw a tag, so they read in greyscale as well as colour: a fill or pattern on the `.key` shape (a
# Figure 3 square, a legend key, a chip's swatch in the hand-drawn figures, which copy these rules), and the tag's
# name beside it. A class per tag: WIRED is `.wired`, BUILT-UNWIRED `.built-unwired`.
SVG_STYLE = """\
  <defs>
    <pattern id="fill-partial" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
      <rect width="6" height="6" fill="#fde9b8"/>
      <rect width="3" height="6" fill="#c98500"/>
    </pattern>
    <pattern id="fill-broken" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
      <rect width="6" height="6" fill="#f7dada"/>
      <rect width="1.6" height="6" fill="#d03b3b"/>
      <rect width="6" height="1.6" fill="#d03b3b"/>
    </pattern>
    <pattern id="fill-built-unwired" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(-45)">
      <rect width="6" height="6" fill="#efeee9"/>
      <rect width="1.4" height="6" fill="#8f8d86"/>
    </pattern>
  </defs>
  <style>
    text { font-family: 'Helvetica Neue', Helvetica, Arial, 'Liberation Sans', Arimo, sans-serif; fill: #0b0b0b; }
    .note { fill: #52514e; }
    .wired .key { fill: #0ca30c; }
    .partial .key { fill: url(#fill-partial); }
    .broken .key { fill: url(#fill-broken); }
    .orphaned .key { fill: #cfcdc6; }
    .built-unwired .key { fill: url(#fill-built-unwired); }
    .docs-only .key, .missing .key, .removed .key, .unproven .key { fill: #ffffff; stroke: #898781; }
    .missing .key { stroke-dasharray: 3 2; }
    .docs-only .key, .removed .key, .unproven .key { stroke-dasharray: 1 2; }
    .partial .id, .broken .id, .built-unwired .id { paint-order: stroke; stroke: #ffffff; stroke-width: 3px; stroke-linejoin: round; }
    .docs-only .id, .missing .id, .removed .id, .unproven .id { fill: #52514e; }
    .removed .id { text-decoration: line-through; }
  </style>"""
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


def render_svg(data: dict, pin: str) -> str:
    """Figure 3: one square per row, labelled with its id. Each group's squares are sorted by tag, so a group reads as
    a bar of how much of it runs; the legend counts the rows per tag."""
    rows, groups = data["row"], data["group"]
    used = [t for t in TAGS if any(r["tag"] == t for r in rows)]
    legend = [(label, tags) for label, tags in (("On the production path", [t for t in used if t in ON_PATH]),
                                                 ("Not on the path", [t for t in used if t not in ON_PATH])) if tags]
    width, cell, pitch, line = 880, (46, 26), 50, 34
    left = 62 + round(max(len(g["name"]) for g in groups) * 13 * 0.52)  # past the longest group name
    per_line = max(1, (width - 16 - left + pitch - cell[0]) // pitch)
    members = {g["id"]: sorted((r for r in rows if r["group"] == g["id"]),
                               key=lambda r: (list(TAGS).index(r["tag"]), int(re.sub(r"\D", "", r["id"]) or 0)))
               for g in groups}
    count = {t: sum(r["tag"] == t for r in rows) for t in used}
    keys, y = [], 16  # legend keys (label, tag, x, y); each label starts a line, and a full line wraps
    for label, tags in legend:
        x = left
        for t in tags:
            w = 30 + len(f"{t} {count[t]}") * 8.2  # 12px capitals
            if x > left and x + w > width - 16:
                x, y = left, y + 24
            keys.append((label, t, x, y))
            label, x = None, round(x + w + 20, 1)
        y += 24
    top = y + 14
    lines = [max(1, -(-len(members[g["id"]]) // per_line)) for g in groups]
    height = top + line * sum(lines) + 22
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" '
           f'role="img" aria-labelledby="title desc" data-pin="{pin}">',
           f'  <title id="title">Status matrix at {pin}</title>',
           f'  <desc id="desc">{len(rows)} mechanisms in {len(groups)} groups, one square per row of the appendix, '
           f'sorted by tag: {", ".join(f"{t} {count[t]}" for t in used)}.</desc>',
           SVG_STYLE,
           f'  <rect width="{width}" height="{height}" rx="8" fill="#ffffff"/>']
    for label, t, x, ky in keys:
        if label:
            out.append(f'  <text class="note" x="16" y="{ky + 12}" font-size="12">{label}</text>')
        out.append(f'  <g class="{t.lower()}"><rect class="key" x="{x + 0.5:g}" y="{ky + 0.5}" width="22" height="14" '
                   f'rx="2"/><text x="{x + 30:g}" y="{ky + 12}" font-size="12">{t} {count[t]}</text></g>')
    y = top
    for g, n in zip(groups, lines):
        out.append(f'  <text x="16" y="{y + 18}" font-size="13" font-weight="bold">{escape(g["id"])}</text>'
                   f'<text x="50" y="{y + 18}" font-size="13">{escape(g["name"])}</text>')
        for k, r in enumerate(members[g["id"]]):
            x, cy = left + pitch * (k % per_line), y + line * (k // per_line)
            out.append(f'  <g class="{r["tag"].lower()}" data-row="{r["id"]}" data-tag="{r["tag"]}">'
                       f'<title>{r["id"]} {escape(r["name"])}: {r["tag"]}</title>'
                       f'<rect class="key" x="{x + 0.5}" y="{cy + 0.5}" width="{cell[0] - 1}" height="{cell[1] - 1}" '
                       f'rx="3"/><text class="id" x="{x + cell[0] // 2}" y="{cy + 17}" font-size="11.5" '
                       f'text-anchor="middle">{r["id"]}</text></g>')
        y += line * n
    out.append(f'  <text class="note" x="16" y="{y + 10}" font-size="11">Status tags at {pin}. One square per row '
               f'of the appendix, sorted by tag within each group.</text>')
    return "\n".join(out) + "\n</svg>\n"


def mark_problems(path: Path, rows: dict[str, str], pin: str) -> list[str]:
    """A hand-drawn figure's status marks against the matrix: its `data-pin` and each `data-row` mark's `data-tag`."""
    try:
        root = ET.parse(path).getroot()
    except (OSError, ET.ParseError) as e:
        return [f"{path.name}: cannot read it as SVG: {e}"]
    errs = []
    if root.get("data-pin") not in (None, pin):
        errs.append(f"{path.name}: drawn at {root.get('data-pin')}, but the matrix is pinned at {pin}")
    for el in root.iter():
        rid, tag = el.get("data-row"), el.get("data-tag")
        if rid is None:
            continue
        if rid not in rows:
            errs.append(f"{path.name}: mark {rid} names no row of the matrix")
        elif tag != rows[rid]:
            errs.append(f"{path.name}: mark {rid} says {tag}, but the matrix says {rows[rid]}")
    return errs


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
    mode.add_argument("--check", action="store_true",
                      help="fail if a row is bad, the appendix or Figure 3 is out of date, or a figure's mark is wrong")
    mode.add_argument("--probe-head", action="store_true", help="list rows whose anchors are gone at HEAD")
    ap.add_argument("--repo", type=Path, default=REPO)
    ap.add_argument("--data", type=Path, default=DATA)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--svg", type=Path, default=SVG, help="Figure 3, drawn from the matrix")
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
    pin, svg_path = data["matrix"]["pinned"], repo / args.svg
    outputs = [(args.out, out_path, render(data, git, pin)), (args.svg, svg_path, render_svg(data, pin))]
    tags = {r["id"]: r["tag"] for r in data["row"]}
    marks = [m for f in sorted(svg_path.parent.glob("*.svg")) if f != svg_path for m in mark_problems(f, tags, pin)]
    if args.check:
        stale = []
        for rel, path, text in outputs:
            try:
                current = path.read_text()
            except OSError:
                current = ""
            if current != text:
                diff = difflib.unified_diff(current.splitlines(), text.splitlines(), str(rel), "regenerated", lineterm="")
                print("\n".join(list(diff)[:40]), file=sys.stderr)
                stale.append(str(rel))
        if stale:
            print(f"status_matrix: {' and '.join(stale)} {'is' if len(stale) == 1 else 'are'} out of date; run "
                  "python3 tools/status_matrix.py", file=sys.stderr)
        for m in marks:
            print(f"status_matrix: {m}; redraw the figure's marks by hand", file=sys.stderr)
        if stale or marks:
            return 1
        print(f"status_matrix: {len(data['row'])} rows ok at {pin}; {args.out} and {args.svg} are current")
        return 0
    for m in marks:
        print(f"status_matrix: warning: {m}", file=sys.stderr)
    for _, path, text in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    print(f"status_matrix: wrote {args.out} and {args.svg} ({len(data['row'])} rows at {pin})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
