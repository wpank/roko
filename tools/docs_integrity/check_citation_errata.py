#!/usr/bin/env python3
"""Report known citation errata that are still written in the docs.

The companion audit of 2026-09-28 (tag audit/baseline-2026-09-28) checked a stratified sample of the works cited in
docs/ against the arXiv, DataCite and Crossref registries. citation_errata.json, next to this file, lists the works it
found wrong, with severity major or minor: fabricated (class I1), a wrong identifier (I2), chimeric (I3), or a real
work cited with wrong metadata (I0; codes M1 title, M2a first author, M2b co-authors, M2c placeholder author, M3 year,
M4 venue, M5 missing identifier). For each work it gives anchors (the arXiv IDs and titles that identify it), the
fragments that are wrong, and the correct work where the audit established it. It was derived from the audit's
adjudication records (gitignored, under tmp/cybernetic-harness/companion-audit/data/adjudication/); its "notes" list
the entries changed by hand. To retire a false positive, edit the work's "bad" fragments.

A line is about a work when it, or the line before or after it (citations wrap), holds one of the work's anchors. On
such a line the checker reports:
- any mention of a fabricated work (I1);
- a wrong arXiv ID;
- a wrong author label. A label naming no author list (a placeholder such as "Anonymous" or a system name such as
  "SiriuS") counts only when a year follows it, as in "[SiriuS, 2025]" or "Anonymous (2025)";
- a wrong title in title position (after *, **, " or “), unless the correct title is on the line too (or wraps onto
  the next one);
- a wrong year after the author label (M3), a wrong venue (M4), or a literal fragment the manifest lists;
- a bibliography entry (a line starting with **[) without an arXiv ID or DOI when the work has one (M5).
In a citations.json file it checks each record's fields instead. The audit adjudicated a sample, so a clean run means
that no known erratum remains, not that every citation is right.

With --prose it also reads Markdown prose one section at a time (the text from one heading to the next). A section
that mentions a work, by an anchor or by a name in the work's "prose" field, may not describe it anywhere with a wrong
title, a wrong author list or a phrase that field lists; a section that mentions a fabricated work is reported at each
mention.

Usage:
  python3 tools/docs_integrity/check_citation_errata.py [--prose] [--summary] [--manifest FILE] [PATH ...]

PATH is a file or a directory (default docs/v3 and docs/v1; docs/v1 is deprecated but still published, so its
citations get the same minimal corrections). Exit status: 0 clean, 1 errata found, 2 bad input.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import unicodedata
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
MANIFEST = HERE / "citation_errata.json"
ARXIV_ID = re.compile(r"(?<!\d)(?<!\d\.)(\d{4}\.\d{4,5})(?:v\d+)?(?!\d)")
TITLE_OPEN = r"(?:\*\*|\*|\"|“|')"
TITLE_END = r"(?=\s*(?:[.*\"”,)\]!?]|$))"  # the whole title, not the start of a longer one
ENTRY = re.compile(r"^\s*(?:[-*]\s+)?\*\*\[")
HAS_ID = re.compile(r"arxiv|doi\.org|\bdoi\b|10\.\d{4,9}/", re.I)
HEADING = re.compile(r"^#{1,6}\s")
FENCE = re.compile(r"^\s*(```|~~~)")


def loose(text: str) -> str:
    """A literal that may wrap: any run of whitespace matches any other."""
    return r"\s+".join(re.escape(part) for part in text.split())


def trie_regex(words: list[str]) -> re.Pattern | None:
    """One regex for many literals, built as a trie so that each position tries one branch per character."""
    trie: dict = {}
    for w in words:
        node = trie
        for ch in w:
            node = node.setdefault(ch, {})
        node[""] = True

    def pattern(node: dict) -> str:
        end = node.get("") is True
        alts = [re.escape(ch) + pattern(sub) for ch, sub in sorted(node.items()) if ch != ""]
        if not alts:
            return ""
        body = alts[0] if len(alts) == 1 else "(?:" + "|".join(alts) + ")"
        return f"(?:{body})?" if end else body

    return re.compile(pattern(trie)) if words else None


def fold(s: str | None) -> str:
    """Lower case without diacritics or repeated spaces, for comparing names and titles."""
    s = "".join(c for c in unicodedata.normalize("NFKD", s or "") if not unicodedata.combining(c))
    return re.sub(r"\s+", " ", s).strip().lower()


@dataclass
class Finding:
    path: str
    line: int
    key: str
    tags: str
    message: str

    def __str__(self) -> str:
        return f"{self.path}:{self.line}: {self.key} [{self.tags}] {self.message}"


class Work:
    def __init__(self, w: dict):
        self.key = w["key"]
        self.cls = w.get("class") or ""
        self.codes = w.get("codes") or []
        self.tags = " ".join([self.cls] + self.codes)
        self.correct = w.get("correct") or None
        bad = w.get("bad") or {}
        self.all = bool(bad.get("all"))
        self.bad_arxiv = set(bad.get("arxiv") or [])
        self.anchor_ids = set(w.get("anchors", {}).get("arxiv") or []) | self.bad_arxiv
        self.anchor_titles = [t for t in w.get("anchors", {}).get("titles") or [] if len(t) >= 8]
        self.correct_title = fold((self.correct or {}).get("title"))
        self.bad_titles = {fold(t) for t in bad.get("titles") or []}
        self.rules: list[tuple[re.Pattern, str]] = []
        for a in bad.get("authors") or []:
            esc = re.escape(a)
            if re.search(r"et al|[,;&]", a):
                rx = re.compile(rf"(?<![\w-]){esc}(?!\w)")
            else:  # a placeholder or a system name: wrong only as an author label, followed by a year
                rx = re.compile(rf"(?<![\w-]){esc}(?:\s*,\s*|\s+|\s*\(\s*)(?:19|20)\d\d(?!\d)")
            self.rules.append((rx, f'wrong author "{a}": {self.cite_as()}'))
        for t in bad.get("titles") or []:
            rx = re.compile(TITLE_OPEN + loose(t) + TITLE_END, re.I)
            self.rules.append((rx, f'wrong title "{t}": use "{(self.correct or {}).get("title") or "the real title"}"'
                                   if self.correct else f'fabricated title "{t}"'))
        for y in bad.get("years") or []:
            names = [re.escape(x.split(",")[0].split(" et al")[0].strip()) for x in bad.get("authors") or []]
            if (self.correct or {}).get("label"):
                names.append(re.escape(self.correct["label"].split(" et al")[0].split(" & ")[0]))
            names = [n for n in names if n]
            if names:
                rx = re.compile(rf"(?<![\w-])(?:{'|'.join(names)})[^\n()\[\]]{{0,30}}?[(,\s]\s*{re.escape(y)}(?!\d)")
                self.rules.append((rx, f"wrong year {y}: use {(self.correct or {}).get('year')}"))
        for v in bad.get("venues") or []:
            self.rules.append((re.compile(rf"(?<![\w]){re.escape(v)}(?![\w.]\w)"),
                               f'wrong venue "{v}": use "{(self.correct or {}).get("venue")}"'))
        for f in bad.get("fragments") or []:
            self.rules.append((re.compile(re.escape(f)), f'wrong citation text "{f}": {self.cite_as()}'))
        self.missing_id = bool(bad.get("missing_id")) and bool(self.correct) and bool(
            self.correct.get("arxiv") or self.correct.get("doi"))
        prose = w.get("prose") or {}
        names = prose.get("names") or []
        self.names_rx = re.compile("|".join(rf"(?<![\w-]){loose(n)}(?![\w-])" for n in names)) if names else None
        self.prose_rules: list[tuple[re.Pattern, str]] = []
        for t in bad.get("titles") or []:
            if self.correct_title and fold(t) in self.correct_title:
                continue  # a short form of the real title is a name, not a wrong title
            self.prose_rules.append((re.compile(loose(t)), f'described by a title it does not have, "{t}": '
                                                               f'{self.cite_as()}'))
        # author lists and initials are specific enough to check across a section; a bare "X et al." is not
        for a in [x for x in bad.get("authors") or [] if re.search(r"[,;&]", x)] + list(bad.get("fragments") or []):
            self.prose_rules.append((re.compile(rf"(?<![\w-]){loose(a)}(?!\w)"), f'wrong citation text "{a}": '
                                                                                f'{self.cite_as()}'))
        for phrase in prose.get("phrases") or []:
            self.prose_rules.append((re.compile(loose(phrase), re.I), f'"{phrase}" does not describe the real work: '
                                                                     f'{self.cite_as()}'))

    def cite_as(self) -> str:
        c = self.correct
        if not c:
            return "no such work; remove the citation"
        ident = f", arXiv:{c['arxiv']}" if c.get("arxiv") else (f", doi:{c['doi']}" if c.get("doi") else "")
        return f'cite as {c.get("label") or "its real authors"} ({c.get("year")}), "{c.get("title")}"{ident}'


class Errata:
    def __init__(self, manifest: dict):
        self.works = [Work(w) for w in manifest["works"]]
        self.by_id: dict[str, list[Work]] = {}
        by_title: dict[str, list[Work]] = {}
        for w in self.works:
            for i in w.anchor_ids:
                self.by_id.setdefault(i, []).append(w)
            for t in w.anchor_titles:
                by_title.setdefault(fold(t), []).append(w)
        self.by_title = by_title
        trie = trie_regex(sorted(by_title))
        # a zero-width match at every word start where some title begins; the regex engine does the scanning
        self.title_scan = re.compile(rf"(?<![a-z0-9])(?=({trie.pattern}))") if trie else None

    def about(self, window: str) -> list[Work]:
        found: dict[str, Work] = {}
        for m in ARXIV_ID.finditer(window):
            for w in self.by_id.get(m.group(1), []):
                found[w.key] = w
        if self.title_scan:
            text = fold(window)
            for m in self.title_scan.finditer(text):
                i = m.start()
                # the trie matches the longest title; shorter titles that are prefixes of it are anchors too
                for j in range(i + 8, m.end(1) + 1):
                    for w in self.by_title.get(text[i:j], []):
                        found[w.key] = w
        return list(found.values())


def sections(lines: list[str]):
    """(start, end) line ranges from one Markdown heading to the next; headings inside code fences don't count."""
    start, fenced = 0, False
    for i, line in enumerate(lines):
        if FENCE.match(line):
            fenced = not fenced
        elif not fenced and HEADING.match(line) and i > start:
            yield start, i
            start = i
    yield start, len(lines)


def check_prose(path: Path, rel: str, errata: Errata) -> list[Finding]:
    lines = path.read_text(encoding="utf-8", errors="replace").split("\n")
    out: list[Finding] = []
    for a, b in sections(lines):
        block = "\n".join(lines[a:b])
        found = {w.key: w for w in errata.about(block)}
        found.update({w.key: w for w in errata.works if w.names_rx and w.names_rx.search(block)})
        for w in found.values():
            if w.all:
                for n in range(a, b):
                    if w in errata.about(lines[n]) or (w.names_rx and w.names_rx.search(lines[n])):
                        out.append(Finding(rel, n + 1, w.key, w.tags, "mentions a fabricated work: remove it"))
                continue
            for rx, msg in w.prose_rules:
                for m in rx.finditer(block):
                    out.append(Finding(rel, a + block.count("\n", 0, m.end()) + 1, w.key, w.tags, msg))
    return out


def check_markdown(path: Path, rel: str, errata: Errata) -> list[Finding]:
    lines = path.read_text(encoding="utf-8", errors="replace").split("\n")
    out: list[Finding] = []
    prev = ""
    for n, cur in enumerate(lines, 1):
        window = f"{prev} {cur}" if prev else cur
        wide = fold(f"{window} {lines[n]}" if n < len(lines) else window)  # a correct title may wrap onto the next line
        start = len(prev) + 1 if prev else 0  # a fragment is reported on the line where it ends
        works = errata.about(f"{window} {lines[n]}" if n < len(lines) else window)
        here = {w.key for w in errata.about(cur)} if any(w.all for w in works) else set()
        for w in works:
            seen = set()
            if w.all:
                if w.key in here:
                    out.append(Finding(rel, n, w.key, w.tags, "fabricated reference (no such work): remove the citation"))
                continue
            for m in ARXIV_ID.finditer(window):
                if m.group(1) in w.bad_arxiv and m.end() > start:
                    c = w.correct or {}
                    fix = f"use arXiv:{c['arxiv']}" if c.get("arxiv") else "no such paper; drop the identifier"
                    out.append(Finding(rel, n, w.key, w.tags, f"wrong arXiv ID {m.group(1)}: {fix}"))
            for rx, msg in w.rules:
                for m in rx.finditer(window):
                    if m.end() <= start or msg in seen:
                        continue
                    if msg.startswith("wrong title") and w.correct_title and w.correct_title in wide:
                        continue
                    seen.add(msg)
                    out.append(Finding(rel, n, w.key, w.tags, msg))
            if w.missing_id and ENTRY.match(cur) and not HAS_ID.search(cur):
                c = w.correct
                ident = f"arXiv:{c['arxiv']}" if c.get("arxiv") else f"doi:{c['doi']}"
                out.append(Finding(rel, n, w.key, w.tags, f"entry has no identifier: add {ident}"))
        prev = cur
    return out


def surnames(authors: list[str]) -> list[str]:
    out = []
    for a in authors or []:
        a = re.sub(r"[,\s]+et\.? al\.?$", "", a.strip())
        out.append(fold(a.split(",")[0] if "," in a else (a.split() or [""])[-1]))
    return [s for s in out if s and s not in ("al.", "et al.")]


def check_citations_json(path: Path, rel: str, errata: Errata) -> list[Finding]:
    text = path.read_text(encoding="utf-8")
    try:
        records = json.loads(text)
    except json.JSONDecodeError as e:
        return [Finding(rel, e.lineno, "-", "json", f"not valid JSON: {e.msg}")]
    lines = text.split("\n")
    out = []
    for r in records if isinstance(records, list) else []:
        rid = str(r.get("id", ""))
        line = next((i for i, l in enumerate(lines, 1) if f'"id": "{rid}"' in l), 1)
        works = {w.key: w for w in errata.by_id.get(str(r.get("arxiv_id") or ""), [])}
        for w in errata.by_title.get(fold(r.get("title")), []):
            works[w.key] = w
        for w in works.values():
            if w.all:
                out.append(Finding(rel, line, w.key, w.tags, "fabricated reference (no such work): remove the record"))
                continue
            if str(r.get("arxiv_id") or "") in w.bad_arxiv:
                out.append(Finding(rel, line, w.key, w.tags, f"wrong arXiv ID {r.get('arxiv_id')}"))
            t = fold(r.get("title"))
            if t in w.bad_titles and t != w.correct_title:
                out.append(Finding(rel, line, w.key, w.tags, f'wrong title "{r.get("title")}": use '
                                                             f'"{(w.correct or {}).get("title")}"'))
            real = surnames((w.correct or {}).get("authors") or [])
            doc = surnames(r.get("authors") or [])
            if real and doc and ({"M2a", "M2b", "M2c", "coauthor_mismatch"} & set(w.codes) or w.cls == "I3"):
                if doc[0] != real[0] or any(s not in real for s in doc):
                    out.append(Finding(rel, line, w.key, w.tags, f"wrong authors {r.get('authors')}: {w.cite_as()}"))
            if "M3" in w.codes and w.correct and str(r.get("year")) != str(w.correct.get("year")):
                out.append(Finding(rel, line, w.key, w.tags, f"wrong year {r.get('year')}: use {w.correct.get('year')}"))
    return out


def files(paths: list[str]) -> list[Path]:
    out = []
    for p in paths:
        q = Path(p)
        if not q.is_absolute():
            q = ROOT / q
        if q.is_dir():
            out += sorted(x for x in q.rglob("*") if x.suffix == ".md" or x.name == "citations.json")
        elif q.exists():
            out.append(q)
        else:
            raise FileNotFoundError(p)
    return [x for x in out if "node_modules" not in x.parts and ".vitepress" not in x.parts]


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("paths", nargs="*", default=["docs/v3", "docs/v1"])
    ap.add_argument("--manifest", default=str(MANIFEST))
    ap.add_argument("--summary", action="store_true", help="print counts by file and by class after the findings")
    ap.add_argument("--prose", action="store_true", help="also check how Markdown prose describes each work")
    a = ap.parse_args(argv)
    try:
        errata = Errata(json.loads(Path(a.manifest).read_text(encoding="utf-8")))
        targets = files(a.paths)
    except (OSError, ValueError, KeyError) as e:
        print(f"check_citation_errata: {e}", file=sys.stderr)
        return 2
    findings: list[Finding] = []
    for f in targets:
        rel = str(f.relative_to(ROOT)) if f.is_relative_to(ROOT) else str(f)
        if f.name == "citations.json":
            findings += check_citations_json(f, rel, errata)
            continue
        findings += check_markdown(f, rel, errata)
        if a.prose:
            findings += check_prose(f, rel, errata)
    seen: set[tuple] = set()
    findings = [x for x in findings if (x.path, x.line, x.key, x.message) not in seen
                and not seen.add((x.path, x.line, x.key, x.message))]
    for x in findings:
        print(x)
    if a.summary:
        by_file = Counter(x.path for x in findings)
        by_class = Counter(x.tags.split()[0] for x in findings)
        for p, n in sorted(by_file.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"  {n:4} {p}")
        print("  by class: " + ", ".join(f"{k} {v}" for k, v in sorted(by_class.items())))
    works = len({x.key for x in findings})
    print(f"check_citation_errata: {len(findings)} errata in {len(set(x.path for x in findings))} files "
          f"({works} works; {len(targets)} files checked)" if findings else
          f"check_citation_errata: clean ({len(targets)} files checked against {len(errata.works)} works)")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
