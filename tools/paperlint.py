#!/usr/bin/env python3
"""paperlint: check that a paper's sections claim no more than the code and the evidence support.

Usage:
  python3 tools/paperlint.py [--strict] [--budget F] [--check-identifiers] [--require-status S] [--report] PATH...

PATH is a section file or a directory; a directory means its section files (NN-*.md, NNa-*.md, A-*.md and
appendix-*.md). Written for docs/whitepaper/ (budgets and conventions in its README.md) and for
tmp/cybernetic-harness/paper/sections/ (budgets in paper/OUTLINE.md, conventions in paper/00-README.md). Needs only
Python 3 and git, and never writes. Code identifiers, paths and commits are looked up at HEAD.

  --strict             every rule below:
      header       line 1 is not a "Status: …" header, the status is stub or skeleton, the word count is outside
                   0.5–1.3× the budget, or the header's budget differs from the outline's
      marker       a [[…]] marker left outside code; TOML headers such as [[task.verify]] are not markers
      citation     a [@key] missing from the paper's references.bib
      identifier   as for --check-identifiers
      status-tag   a status tag (WIRED, PARTIAL, … MISSING) in prose without @<commit>, or a TAG@<commit> whose commit
                   is not in HEAD's history. In a table or a code span a bare tag is a name, and so is a word that a
                   "<!-- paperlint: claim-levels WIRED PARTIAL -->" line declares as the file's own claim level
      number       a $ amount, percentage, N× ratio, "N of M" or N/M count in a paragraph, list item or table with
                   neither an inline [@key] nor a footnote naming a commit, work item, [@key], snapshot or rollup,
                   or an existing evidence/ file ("95% CI" and "step 3 of 11" are not counts); a sha256 that a
                   footnote gives with an evidence/ file must start the file's line in the SHA256SUMS beside it
      banned       a word the README's conventions ban (a bullet that says "banned", or `No "…"` under Conventions)
      link         a broken local link or figure path (tools/docs_integrity/check_markdown_links.py)
  --budget F           budget: the word count is more than F times the budget
  --check-identifiers  identifier: a backticked code identifier (CamelCase, snake_case, a::b) that `git grep -w`
                       can't find in crates/, or a repo path (its first segment tracked at HEAD, .roko/ aside) that
                       doesn't exist at HEAD. A use is covered when it sits inside [[AS-BUILT: …]], or its sentence
                       or table row says "(designed)", "(external)", MISSING, DOCS-ONLY or REMOVED. Covering the
                       first use in a file covers the later ones. A code span of 7–11 or 40 hex digits is a
                       commit and must be in HEAD's history, unless it follows "sha256".
  --require-status S   status: a file's status header doesn't say S (the final pass uses "reviewed")
  --report             print word counts, marker counts and findings per rule for each file, and exit 0

Word counts cover headings and prose after line 1, tables and captions included (docs/whitepaper/README.md). They
leave out code blocks, footnote definitions, HTML comments, a blockquote note right under the title, every section
whose heading is just "Claims ledger" or "Claims ledger (§…)" (its table is metadata for paper/tools/claims.py; a
numbered "4.1 Claims ledger" is prose), and the "Placeholder map" and "Alternative …" sections. A slot marker
(RESULT, FIG, TAB, CITE?, CITE-COMPANION, E1, FIELD) counts as one word; other markers count their words.

The budget comes from an outline table in OUTLINE.md or README.md, in the file's directory or up to two above it:
the column whose header says "budget" (words) or "pages" (times the outline's "N words per page", else 550). A row
naming several files splits its budget evenly, and a cell with no number (none, ∞, —) means no budget. Without an
outline row, the header's "budget N words" is used.

Exit status: 0 clean, 1 problems found, 2 bad usage or unreadable input.
"""
from __future__ import annotations

import argparse, bisect, json, re, subprocess, sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
RULES = ("header", "status", "budget", "marker", "citation", "identifier", "status-tag", "number", "banned", "link")
STRICT = {"header", "marker", "citation", "identifier", "status-tag", "number", "banned", "link"}
STRICT_RANGE = (0.5, 1.3)
UNWRITTEN = {"stub", "skeleton"}
WORDS_PER_PAGE = 550
SECTION_FILE = re.compile(r"^(?:\d{2}[a-z]?|[A-Z]|appendix)-[a-z0-9][a-z0-9-]*\.md$")
LEDGER = re.compile(r"claims ledger(?:\s*\([^()]*\))?", re.I)  # the whole heading: "Claims ledger (§3.1–§3.3)"
EDITORIAL = re.compile(r"^(?:placeholder map|alternative\b)", re.I)

HEADER = re.compile(r"^Status:\s*([A-Za-z][\w-]*)")
HEADER_BUDGET = re.compile(r"\bbudget\s*[:=]?\s*[~≈]?\s*(\d[\d,]*)\s*words?\b", re.I)
FENCE = re.compile(r"^\s*(`{3,}|~{3,})")
COMMENT = re.compile(r"<!--.*?-->", re.S)
LEVELS = re.compile(r"^[ \t]*<!--[ \t]*paperlint:[ \t]*claim-levels\b(.*?)-->[ \t]*$", re.M)  # a line of its own
CODE_SPAN = re.compile(r"(?<!`)(`+)(?!`)(.+?)(?<!`)\1(?!`)")
HEADING = re.compile(r"^ {0,3}(#{1,6})(?:[ \t]+|$)(.*)$")
TABLE_ROW = re.compile(r"^\s*(?:>\s*)*\|")
LIST_ITEM = re.compile(r"^\s*(?:>\s*)*(?:[-*+]|\d+[.)])\s+")
FOOTNOTE_DEF = re.compile(r"^\[\^([^\]\s]+)\]:")
FOOTNOTE_REF = re.compile(r"\[\^([^\]\s]+)\](?!:)")
SENTENCE_END = re.compile(r"[.!?][)\]\"'”’*_]*\s+(?=[^\sa-z])")
LINK_TARGET = re.compile(r"(?<=\])\([^)\n]*\)|<[a-z][a-z0-9+.-]*:[^>\s]*>|\bhttps?://\S+", re.I)

# [[RESULT H1: …]], [[AS-BUILT: …]], [[TODO: …]], [[CITE?: …]], [[FIG F1]] … (paper/00-README.md). A marker may wrap
# lines but not cross a blank line. Lower-case dotted names such as [[task.verify]] are TOML array tables.
MARKER = re.compile(r"\[\[((?:(?!\n[ \t]*\n).)*?)\]\]", re.S)
MARKER_KIND = re.compile(r"^\s*([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*\??)")
SLOTS = {"RESULT", "FIG", "TAB", "CITE?", "CITE-COMPANION", "E1", "FIELD"}  # stand for a value, figure or citation
TOML_TABLE = re.compile(r"^\s*[a-z_][a-z0-9_-]*(?:\.[a-z_][a-z0-9_-]*)*\s*$")

CITE_GROUP = re.compile(r"\[([^\[\]]*?)\]")
CITE_KEY = re.compile(r"(?:^|(?<=[\s;(\[]))-?@([A-Za-z0-9_][\w:.#$%&+?<>~/-]*)")
BIB_ENTRY = re.compile(r"@(\w+)\s*[{(]\s*([^,\s]+)\s*,")

TAG = re.compile(r"(?<![\w-])(BUILT-UNWIRED|DOCS-ONLY|WIRED|PARTIAL|ORPHANED|MISSING|REMOVED|BROKEN|UNPROVEN)(?![\w-])"
                 r"(?:@([0-9A-Za-z]+))?")
SHA = re.compile(r"^[0-9a-f]{7,40}$")
COMMIT_SPAN = re.compile(r"^(?=[0-9a-f]*\d)(?=[0-9a-f]*[a-f])(?:[0-9a-f]{7,11}|[0-9a-f]{40})$")  # README: 9-hex
SHA256_BEFORE = re.compile(r"sha-?\s?256\W{0,24}$", re.I)
HEX = re.compile(r"(?<![\w])[0-9a-f]{7,40}(?![\w])")
WORK_ID = re.compile(r"\b(?:gap|bug|reg|find|dec|spec|q)-[0-9a-f]{6,8}\b")
COVER_WORD = re.compile(r"\((?:designed|external)\b", re.I)
COVER_TAG = re.compile(r"(?<![\w-])(?:MISSING|DOCS-ONLY|REMOVED)(?![\w-])")

NUM = r"\d(?:[\d,]*\d)?(?:\.\d+)?"
NUMBER_CLAIMS = (
    ("$ amount", re.compile(rf"\$\s?{NUM}(?:[kKmMbB]\b)?")),
    ("percentage", re.compile(rf"(?<![\w.]){NUM}\s?(?:%|per\s?cent\b)"
                              r"(?!\s*(?:CI|confidence|credible|interval)\b)", re.I)),
    ("ratio", re.compile(rf"(?<![\w.]){NUM}×|\b\d+(?:\.\d+)?x\b")),
    ("count", re.compile(rf"(?<![\w.]){NUM}\s+(?:out\s+)?of\s+(?:the\s+|all\s+)?{NUM}")),
    ("count", re.compile(r"(?<![\w./-])\d+(?:\.\d+)?\s?/\s?\d+(?:\.\d+)?(?![\w/-]|\.\d)")),
)
ORDINAL = re.compile(r"(?:step|stage|phase|part|section|chapter|level|rung|wave|round|page|figure|table|item|§)\s*$",
                     re.I)  # "step 3 of 11" is a position, not a count
SOURCE_WORD = re.compile(r"\b(?:snapshot|rollup)\b", re.I)
EVIDENCE_FILE = re.compile(r"(?<![\w/.-])((?:[\w.-]+/)*evidence/[\w./-]*[\w-])")
DIGEST = re.compile(r"\W{0,4}sha-?\s?256\W{0,24}?\b([0-9a-f]{7,64})\b", re.I)  # after a path: "(sha256 `…`)"
SOURCE_ID = re.compile(r"`[^`\n]+`|\d{4}-\d{2}-\d{2}")

TOKEN = re.compile(r"\w+")
SNAKE = re.compile(r"_*[A-Za-z][A-Za-z0-9]*(?:_+[A-Za-z0-9]+)+_*")
ALNUM = re.compile(r"[A-Za-z][A-Za-z0-9]*")
CAPITALIZED = re.compile(r"[A-Z][a-z][a-z0-9]+")
IDENT_PATH = re.compile(r"[&*.#]*([A-Za-z_]\w*(?:(?:::|\.)[A-Za-z_]\w*)*)!?(?:\(.*\))?")
TEMPLATED = re.compile(r"[<>{}*…$~]|\.\.\.")


def blank(s: str) -> str:
    """Same length with newlines kept, so offsets and line numbers survive masking."""
    return re.sub(r"[^\n]", " ", s)


def is_identifier(tok: str, shaped: bool) -> bool:
    """A token that reads as code: snake_case, CamelCase or camelCase, or a Capitalized name when it is the span."""
    if not tok.isascii() or tok.isdigit():
        return False
    if SNAKE.fullmatch(tok):
        return True
    if ALNUM.fullmatch(tok) and re.search(r"[a-z]", tok) and re.search(r"(?<=.)[A-Z]", tok):
        return True
    return shaped and CAPITALIZED.fullmatch(tok) is not None


def split_path(word: str):
    """(path, [symbols]) for a word that names a repo-relative path, or None. Drops :line and #L suffixes."""
    word = word.strip("'\"()[],;")
    if "/" not in word or "://" in word or TEMPLATED.search(word):
        return None
    word = word.rstrip(".,;:")
    path, _, sym = word.partition("::")
    path = re.sub(r"(?::\d+(?:[-:]\d+)*|#L\d.*)$", "", path)
    if not path or path.startswith(("/", "./", "../", "-")) or "//" in path:
        return None
    syms = [s for s in (re.sub(r"\(.*$", "", s) for s in sym.split("::")) if s and is_identifier(s, True)]
    return path.rstrip("/"), syms


def span_refs(content: str) -> list[tuple]:
    """What a backticked span claims exists: ("ident", name), ("path", path) or ("symbol", path, name)."""
    s = content.strip()
    if not s or "[[" in s or "]]" in s or "@" in s:
        return []
    refs: list[tuple] = []
    words = []
    for word in s.split():
        if "/" in word:
            p = split_path(word)
            if p:
                refs.append(("path", p[0]))
                refs.extend(("symbol", p[0], sym) for sym in p[1])
        else:
            words.append(word)
    shaped = len(words) == 1 and not refs and IDENT_PATH.fullmatch(words[0]) is not None
    for tok in TOKEN.findall(" ".join(words)):
        if is_identifier(tok, shaped):
            refs.append(("ident", tok))
    return list(dict.fromkeys(refs))


# ---------------------------------------------------------------- documents


@dataclass
class Block:
    start: int
    end: int
    kind: str  # para, table, heading, footnote
    label: str = ""


@dataclass
class Marker:
    start: int
    end: int
    kind: str
    body: str


@dataclass(frozen=True, order=True)
class Finding:
    path: str
    line: int
    rule: str
    message: str

    def __str__(self):
        return f"{self.path}:{self.line}: [{self.rule}] {self.message}"


class Doc:
    """A section file and its masked views. Every view has the text's length, so offsets carry across."""

    def __init__(self, path: Path, display: str):
        self.path, self.display = path, display
        self.text = path.read_text(encoding="utf-8")
        first = self.text.split("\n", 1)[0]
        m = HEADER.match(first)
        self.header = first if m else None
        self.status = m.group(1).lower() if m else None
        hb = HEADER_BUDGET.search(first) if m else None
        self.header_budget = int(hb.group(1).replace(",", "")) if hb else None
        # Masked views: `code` blanks the header, code blocks and HTML comments; `prose` also blanks inline code;
        # `bare` also blanks [[…]] markers.
        lines, fence, indented, prev_blank, last = [], None, False, True, None
        for i, line in enumerate(self.text.split("\n")):
            f = FENCE.match(line)
            if (i == 0 and self.header) or fence is not None:
                if fence is not None and f and f.group(1)[0] == fence[0] and len(f.group(1)) >= len(fence) \
                        and not line.strip().strip(fence[0]):
                    fence = None
                lines.append(blank(line))
                prev_blank = False
                continue
            if indented and (not line.strip() or line.startswith(("    ", "\t"))):
                lines.append(blank(line))
                continue
            indented = False
            if f:
                fence = f.group(1)
                lines.append(blank(line))
                continue
            # An indented code block follows a blank line, and not inside a list item or a footnote.
            if line.strip() and prev_blank and line.startswith(("    ", "\t")) and last not in ("list", "footnote"):
                indented = True
                lines.append(blank(line))
                continue
            lines.append(line)
            prev_blank = not line.strip()
            if line.strip() and not line.startswith(("    ", "\t")):
                last = "list" if LIST_ITEM.match(line) else "footnote" if FOOTNOTE_DEF.match(line) else "other"
        joined = "\n".join(lines)
        # The companion's claim levels (EFFECTIVE … UNKNOWN) share WIRED and PARTIAL with the status tags; a file
        # whose own scale does so says which words are its levels, and only their TAG@sha form is then a tag.
        self.levels = {t.group(1) for d in LEVELS.finditer(joined) for t in TAG.finditer(d.group(1))}
        self.code = COMMENT.sub(lambda c: blank(c.group(0)), joined)
        self.spans = [(c.start(2), c.group(2)) for c in CODE_SPAN.finditer(self.code)]
        self.prose = CODE_SPAN.sub(lambda c: blank(c.group(0)), self.code)
        self.markers: list[Marker] = []
        self.unclosed: list[int] = []
        handled = set()
        for mk in MARKER.finditer(self.prose):
            handled.add(mk.start())
            if TOML_TABLE.match(mk.group(1)):
                continue
            kind = MARKER_KIND.match(mk.group(1))
            self.markers.append(Marker(mk.start(), mk.end(), kind.group(1) if kind else "other", mk.group(1)))
        for m2 in re.finditer(r"\[\[", self.prose):
            if m2.start() not in handled and not any(k.start < m2.start() < k.end for k in self.markers):
                self.unclosed.append(m2.start())
        self.bare = self.prose
        for k in self.markers:
            self.bare = self.bare[:k.start] + blank(self.bare[k.start:k.end]) + self.bare[k.end:]
        self.line_starts = [0] + [i + 1 for i, ch in enumerate(self.text) if ch == "\n"]
        self.blocks = self._blocks()
        self.units = self._units()
        self.unit_starts = [u[0] for u in self.units]
        self._words: int | None = None

    def line_of(self, offset: int) -> int:
        return bisect.bisect_right(self.line_starts, offset)

    def _blocks(self) -> list[Block]:
        out: list[Block] = []
        cur = None
        off = 0
        for line in self.code.split("\n"):
            start, end = off, off + len(line)
            off = end + 1
            if not line.strip():
                cur = None
            elif HEADING.match(line):
                out.append(Block(start, end, "heading"))
                cur = None
            elif TABLE_ROW.match(line):
                if cur and cur.kind == "table":
                    cur.end = end
                else:
                    cur = Block(start, end, "table")
                    out.append(cur)
            elif FOOTNOTE_DEF.match(line):
                cur = Block(start, end, "footnote", FOOTNOTE_DEF.match(line).group(1))
                out.append(cur)
            elif LIST_ITEM.match(line) or cur is None or cur.kind in ("table", "heading"):
                cur = Block(start, end, "para")
                out.append(cur)
            else:
                cur.end = end
        return out

    def _units(self) -> list[tuple[int, int]]:
        """Sentences, table rows and headings: the scope of a "(designed)" or MISSING tag."""
        out = []
        for b in self.blocks:
            if b.kind == "table":
                off = b.start
                for line in self.code[b.start:b.end].split("\n"):
                    out.append((off, off + len(line)))
                    off += len(line) + 1
                continue
            s = b.start
            for m in SENTENCE_END.finditer(self.code, b.start, b.end):
                out.append((s, m.end()))
                s = m.end()
            out.append((s, b.end))
        return out

    def covered(self, offset: int) -> bool:
        """A use inside [[AS-BUILT: …]], or in a sentence or row tagged "(designed)", "(external)" or MISSING."""
        if any(k.kind == "AS-BUILT" and k.start <= offset < k.end for k in self.markers):
            return True
        i = bisect.bisect_right(self.unit_starts, offset) - 1
        if i < 0 or offset >= self.units[i][1]:
            return False
        unit = self.prose[self.units[i][0]:self.units[i][1]]
        return bool(COVER_WORD.search(unit) or COVER_TAG.search(unit))

    def refs(self) -> dict[tuple, list[int]]:
        """Every code reference in backticks, with the offsets of its uses in reading order."""
        out: dict[tuple, list[int]] = defaultdict(list)
        for off, content in self.spans:
            for ref in span_refs(content):
                out[ref].append(off)
        return out

    def word_count(self) -> int:
        """Words as the reader will see them: see the module docstring for what is left out."""
        if self._words is not None:
            return self._words
        chars = list(self.code)
        marker_lines = Counter()
        for k in self.markers:
            if k.kind in SLOTS:
                chars[k.start:k.end] = blank(self.code[k.start:k.end])
                marker_lines[self.line_of(k.start)] += 1
        for b in self.blocks:
            if b.kind == "footnote":
                chars[b.start:b.end] = blank(self.code[b.start:b.end])
        lines = "".join(chars).split("\n")
        skip = set()
        headings = []
        for i, line in enumerate(self.code.split("\n")):
            h = HEADING.match(line)
            if h:
                headings.append((i, len(h.group(1)), h.group(2).strip()))
        for n, (i, level, title) in enumerate(headings):
            if LEDGER.fullmatch(title) or EDITORIAL.match(re.sub(r"^[\d.§\s]+", "", title)):
                end = next((j for j, lv, _ in headings[n + 1:] if lv <= level), len(lines))
                skip.update(range(i, end))
        if headings:
            i = headings[0][0] + 1
            while i < len(lines) and not lines[i].strip():
                i += 1
            while i < len(lines) and lines[i].lstrip().startswith(">"):
                skip.add(i)
                i += 1
        words = 0
        for i, line in enumerate(lines):
            if i in skip:
                continue
            words += sum(1 for tok in line.split() if re.search(r"\w", tok))
            words += marker_lines.get(i + 1, 0)
        self._words = words
        return words


# ---------------------------------------------------------------- the paper around a file


class Repo:
    """Read-only git lookups at HEAD, cached and batched."""

    def __init__(self, root: Path):
        self.root = root
        self._idents: dict[str, bool] = {}
        self._paths: dict[str, bool] = {}
        self._symbols: dict[tuple[str, str], bool] = {}
        self._history: list[str] | None = None
        self._top: set[str] | None = None
        self._items: set[str] | None = None

    @classmethod
    def of(cls, path: Path) -> Repo | None:
        try:
            r = subprocess.run(["git", "-C", str(path.parent), "rev-parse", "--show-toplevel"], capture_output=True,
                               text=True)
            if r.returncode != 0:
                return None
            root = Path(r.stdout.strip()).resolve()
            head = subprocess.run(["git", "-C", str(root), "rev-parse", "--verify", "-q", "HEAD"], capture_output=True,
                                  text=True)
        except OSError:  # no git
            return None
        return cls(root) if head.returncode == 0 else None

    def git(self, *args: str, stdin: str | None = None) -> str:
        return subprocess.run(["git", "-C", str(self.root), *args], capture_output=True, text=True, input=stdin).stdout

    def top(self) -> set[str]:
        if self._top is None:
            self._top = set(self.git("ls-tree", "--name-only", "HEAD").split("\n")) - {""}
        return self._top

    def is_repo_path(self, path: str) -> bool:
        first = path.split("/", 1)[0]
        return first != ".roko" and first in self.top()

    def grep_words(self, names, path: str) -> set[str]:
        """The names found as whole words under `path` at HEAD. One alternation per call: `git grep -F` with many
        patterns is very slow. Names are identifiers, so they need no escaping."""
        found: set[str] = set()
        names = sorted(names)
        for i in range(0, len(names), 200):
            alt = "|".join(names[i:i + 200])
            r = subprocess.run(["git", "-C", str(self.root), "grep", "-I", "-h", "-o", "-P", "-e", rf"\b(?:{alt})\b",
                                "HEAD", "--", path], capture_output=True, text=True)
            if r.returncode > 1:  # git built without PCRE
                r = subprocess.run(["git", "-C", str(self.root), "grep", "-I", "-h", "-o", "-w", "-E", "-e", alt,
                                    "HEAD", "--", path], capture_output=True, text=True)
            found.update(r.stdout.split())
        return found

    def idents(self, names) -> set[str]:
        """The names `git grep -w` finds in crates/ at HEAD."""
        todo = {n for n in names if n not in self._idents}
        if todo:
            found = self.grep_words(todo, "crates/")
            for n in todo:
                self._idents[n] = n in found
        return {n for n in names if self._idents[n]}

    def paths(self, paths) -> set[str]:
        """The paths (files or directories) that exist at HEAD."""
        todo = sorted({p for p in paths if p not in self._paths})
        if todo:
            out = self.git("cat-file", "--batch-check", stdin="".join(f"HEAD:{p}\n" for p in todo)).split("\n")
            for p, line in zip(todo, out):
                self._paths[p] = bool(line) and not line.endswith(("missing", "ambiguous"))
            for p in todo[len(out):]:
                self._paths[p] = False
        return {p for p in paths if self._paths[p]}

    def symbols(self, path: str, names) -> set[str]:
        todo = {n for n in names if (path, n) not in self._symbols}
        if todo:
            found = self.grep_words(todo, path)
            for n in todo:
                self._symbols[(path, n)] = n in found
        return {n for n in names if self._symbols[(path, n)]}

    def in_history(self, sha: str) -> bool:
        """True when `sha` abbreviates a commit reachable from HEAD."""
        if self._history is None:
            self._history = sorted(self.git("rev-list", "HEAD").split())
        sha = sha.lower()
        i = bisect.bisect_left(self._history, sha)
        return i < len(self._history) and self._history[i].startswith(sha)

    def is_commit(self, sha: str) -> bool:
        return subprocess.run(["git", "-C", str(self.root), "cat-file", "-e", f"{sha}^{{commit}}"],
                              capture_output=True).returncode == 0

    def work_item(self, wid: str) -> bool:
        if self._items is None:
            names = self.git("ls-tree", "-r", "--name-only", "HEAD", "work/").split("\n")
            self._items = {m.group(0) for n in names for m in [WORK_ID.match(Path(n).name)] if m}
        return wid in self._items


@dataclass
class Budget:
    words: int | None  # None: the outline says the file has no budget (∞, —)
    source: str


class Paper:
    """What a file's checks read around it: outline budgets, the bibliography, banned words and the repository."""

    def __init__(self):
        self._outlines: dict[Path, dict[str, Budget]] = {}
        self._bibs: dict[Path, tuple[set[str], dict[str, str]]] = {}
        self._banned: dict[Path, list[str]] = {}
        self._repos: dict[Path, Repo | None] = {}

    def repo(self, doc: Doc) -> Repo | None:
        d = doc.path.parent
        if d not in self._repos:
            self._repos[d] = Repo.of(doc.path)
        return self._repos[d]

    def _dirs(self, doc: Doc):
        repo = self.repo(doc)
        d = doc.path.parent
        for _ in range(3):
            yield d
            if (repo and d == repo.root) or d.parent == d:
                return
            d = d.parent

    def outline_budget(self, doc: Doc) -> Budget | None:
        for d in self._dirs(doc):
            for name in ("OUTLINE.md", "README.md"):
                f = d / name
                if f.is_file() and f != doc.path:
                    if f not in self._outlines:
                        self._outlines[f] = parse_outline(f)
                    if doc.path.name in self._outlines[f]:
                        return self._outlines[f][doc.path.name]
        return None

    def budget(self, doc: Doc) -> Budget | None:
        b = self.outline_budget(doc)
        if b is None and doc.header_budget is not None:
            b = Budget(doc.header_budget, "its status header")
        return b

    def bib(self, doc: Doc):
        """(keys, aliases, path) of the nearest references.bib, or None."""
        for d in self._dirs(doc):
            for f in (d / "references.bib", d / "bibliography" / "references.bib"):
                if f.is_file():
                    if f not in self._bibs:
                        keys = {m.group(2) for m in BIB_ENTRY.finditer(f.read_text(encoding="utf-8"))
                                if m.group(1).lower() not in ("comment", "string", "preamble")}
                        aliases = {}
                        a = f.parent / "key-aliases.json"
                        if a.is_file():
                            try:
                                aliases = json.loads(a.read_text(encoding="utf-8"))
                            except ValueError:
                                pass
                        self._bibs[f] = (keys, aliases)
                    return (*self._bibs[f], f)
        return None

    def banned(self, doc: Doc) -> list[str]:
        for d in self._dirs(doc):
            for name in ("README.md", "00-README.md"):
                f = d / name
                if f.is_file():
                    if f not in self._banned:
                        self._banned[f] = parse_banned(f.read_text(encoding="utf-8"))
                    if self._banned[f]:
                        return self._banned[f]
        return []


def table_cells(line: str) -> list[str]:
    line = line.strip()
    if line.startswith("|"):
        line = line[1:]
    if line.endswith("|") and not line.endswith("\\|"):
        line = line[:-1]
    return [c.strip() for c in re.split(r"(?<!\\)\|", line)]


def parse_outline(path: Path) -> dict[str, Budget]:
    """File name -> budget, from every table with a budget (words) or pages column that names .md files."""
    text = path.read_text(encoding="utf-8")
    rate = re.search(r"(\d[\d,]*)\s+words\s+per\s+page", text, re.I)
    per_page = int(rate.group(1).replace(",", "")) if rate else WORDS_PER_PAGE
    out: dict[str, Budget] = {}
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        if not (TABLE_ROW.match(lines[i]) and i + 1 < len(lines) and re.match(r"^\s*\|?\s*:?-{3,}", lines[i + 1])):
            i += 1
            continue
        head = [c.lower() for c in table_cells(lines[i])]
        col = next((n for n, c in enumerate(head) if "budget" in c), None)
        if col is None:
            col = next((n for n, c in enumerate(head) if "page" in c), None)
        pages = col is not None and "page" in head[col]
        file_col = next((n for n, c in enumerate(head) if "file" in c), None)
        i += 2
        while i < len(lines) and TABLE_ROW.match(lines[i]):
            cells = table_cells(lines[i])
            i += 1
            if col is None or col >= len(cells):
                continue
            where = [cells[file_col]] if file_col is not None and file_col < len(cells) else \
                [c for n, c in enumerate(cells) if n != col]
            names = []
            for cell in where:
                names += [Path(m).name for m in re.findall(r"[\w./-]+\.md\b", cell)]
                if names:
                    break
            names = list(dict.fromkeys(names))
            if not names:
                continue
            num = re.search(r"\d[\d,]*(?:\.\d+)?", cells[col])
            if num:
                value = float(num.group(0).replace(",", ""))
                src = f"{path.name}: {value:g} pages × {per_page} words" if pages else f"{path.name}: {value:g} words"
                src += f", split over {len(names)} files" if len(names) > 1 else ""
                for n in names:
                    out[n] = Budget(round(value * (per_page if pages else 1) / len(names)), src)
            else:
                for n in names:
                    out[n] = Budget(None, f"{path.name}: {cells[col] or 'empty'}")
    return out


def parse_banned(text: str) -> list[str]:
    """Words and phrases banned under a Conventions or Style heading: quoted terms on a line that says "banned",
    and after `No "…"` (the research draft's style rules)."""
    out: list[str] = []
    level = None
    items: list[str] = []
    for line in text.split("\n"):
        h = HEADING.match(line)
        if h:
            if level is not None and len(h.group(1)) <= level:
                level = None
            if re.search(r"convention|style", h.group(2), re.I):
                level = len(h.group(1))
            continue
        if level is None:
            continue
        if not line.strip() or LIST_ITEM.match(line) or not items:
            items.append(line)
        else:
            items[-1] += " " + line
    for item in items:
        quoted = re.findall(r"\"([^\"\n]+)\"|“([^”\n]+)”", item)
        terms = [a or b for a, b in quoted]
        if re.search(r"\bbanned\b", item, re.I):
            out += terms + re.findall(r"`([^`\n]+)`", item)
        elif re.search(r"\bNo\s+[\"“]", item):
            out += terms
    return list(dict.fromkeys(t.strip() for t in out if t.strip()))


# ---------------------------------------------------------------- rules


def check_header(doc: Doc, paper: Paper, words: int) -> list[str | tuple[int, str]]:
    if doc.header is None:
        return ['line 1 must be a status header, e.g. "Status: draft · budget 600 words · owner gap-xxxxxx"']
    out = []
    if doc.status in UNWRITTEN:
        out.append(f"status is {doc.status}: the section is not written yet")
    outline = paper.outline_budget(doc)
    if outline and outline.words is not None and doc.header_budget is not None and outline.words != doc.header_budget:
        out.append(f"the header's budget ({doc.header_budget} words) differs from {outline.source}: {outline.words}")
    b = paper.budget(doc)
    if b and b.words:
        lo, hi = STRICT_RANGE
        ratio = words / b.words
        if not lo <= ratio <= hi:
            out.append(f"{words} words is {ratio:.2f}× the budget of {b.words} ({b.source}); keep it within {lo}–{hi}×")
    return out


def check_budget(doc: Doc, paper: Paper, words: int, factor: float) -> list[str]:
    b = paper.budget(doc)
    if b is None:
        return [f"no word budget for {doc.path.name}: add it to the outline table (OUTLINE.md or README.md)"]
    if b.words and words > factor * b.words:
        return [f"{words} words is {words / b.words:.2f}× the budget of {b.words} ({b.source}); the limit is {factor}×,"
                f" so cut {words - int(factor * b.words)} words"]
    return []


def check_markers(doc: Doc) -> list[tuple[int, str]]:
    out = [(k.start, f"leftover marker [[{' '.join(k.body.split())[:70]}]]") for k in doc.markers]
    out += [(off, "unclosed [[ marker") for off in doc.unclosed]
    return out


def citations(doc: Doc):
    """(offset, key) for every [@key] citation in prose."""
    for g in CITE_GROUP.finditer(doc.prose):
        if doc.prose[g.end():g.end() + 1] == "(" or g.group(1).startswith("^"):
            continue
        for k in CITE_KEY.finditer(g.group(1)):
            yield g.start(), k.group(1).rstrip(".:,;")


def check_citations(doc: Doc, paper: Paper) -> list[tuple[int, str]]:
    cites = list(citations(doc))
    if not cites:
        return []
    bib = paper.bib(doc)
    if bib is None:
        return [(cites[0][0], "no references.bib found for this file's citations")]
    keys, aliases, path = bib
    out = []
    for off, key in cites:
        if key not in keys:
            hint = f"; it is an alias of {aliases[key]}, cite that key" if key in aliases else ""
            out.append((off, f"[@{key}] is not in {path.name}{hint}"))
    return out


def check_identifiers(doc: Doc, repo: Repo | None) -> list[tuple[int, str]]:
    refs = doc.refs()
    if not refs:
        return []
    if repo is None:
        return [(0, "not in a git repository with a HEAD commit, so code identifiers can't be checked")]
    idents = repo.idents({r[1] for r in refs if r[0] == "ident"})
    paths = repo.paths({r[1] for r in refs if r[0] in ("path", "symbol") and repo.is_repo_path(r[1])})
    wanted = defaultdict(set)
    for r in refs:
        if r[0] == "symbol" and r[1] in paths:
            wanted[r[1]].add(r[2])
    symbols = {(path, s) for path, names in wanted.items() for s in repo.symbols(path, names)}
    out = []
    for ref, offs in refs.items():
        if ref[0] == "ident":
            if ref[1] in idents:
                continue
            what = f"`{ref[1]}` is not in crates/ at HEAD"
        else:
            if not repo.is_repo_path(ref[1]):
                continue
            if ref[0] == "path":
                if ref[1] in paths:
                    continue
                what = f"path `{ref[1]}` does not exist at HEAD"
            else:
                if ref[1] not in paths or (ref[1], ref[2]) in symbols:
                    continue
                what = f"`{ref[2]}` is not in `{ref[1]}` at HEAD"
        if doc.covered(offs[0]):
            continue
        uses = f" ({len(offs)} uses)" if len(offs) > 1 else ""
        out.append((offs[0], f"{what}{uses}; if it is not built, tag its first use: wrap it in [[AS-BUILT: …]], or put"
                             f" \"(designed)\" or \"(external)\" in its sentence"))
    return out


def check_commits(doc: Doc, repo: Repo | None) -> list[tuple[int, str]]:
    """A code span of 7–11 or 40 hex digits, letters and digits both, is a commit and must be in HEAD's history.
    A span right after "sha256" is a file digest (the README gives digests as 12 hex digits)."""
    out = []
    for off, content in doc.spans:
        sha = content.strip()
        if not COMMIT_SPAN.match(sha) or SHA256_BEFORE.search(doc.code[max(0, off - 40):off - 1]):
            continue
        if repo is None:
            out.append((off, f"commit `{sha}`: not in a git repository, so it can't be checked"))
        elif not repo.in_history(sha):
            why = "is not an ancestor of HEAD" if repo.is_commit(sha) else "is not a commit in this repository"
            out.append((off, f"commit `{sha}` {why}"))
    return out


def check_tags(doc: Doc, repo: Repo | None) -> list[tuple[int, str]]:
    """Tags in prose need @<commit>. In a table or code a bare tag is a name (a column, a legend row, a mention), as
    is a claim level the file declares; a TAG@sha has its commit checked everywhere."""
    tables = [(b.start, b.end) for b in doc.blocks if b.kind == "table"]
    found = [(t.start(), t.group(1), t.group(2)) for t in TAG.finditer(doc.bare)
             if t.group(2) or not (t.group(1) in doc.levels or any(s <= t.start() < e for s, e in tables))]
    found += [(off + t.start(), t.group(1), t.group(2)) for off, content in doc.spans for t in TAG.finditer(content)
              if t.group(2)]
    out = []
    for start, tag, sha in sorted(found):
        if sha is None:
            out.append((start, f"status tag {tag} has no @<commit>"))
        elif not SHA.match(sha):
            out.append((start, f"{tag}@{sha}: the commit must be 7–40 lower-case hex digits"))
        elif repo is None:
            out.append((start, f"{tag}@{sha}: not in a git repository, so the commit can't be checked"))
        elif not repo.in_history(sha):
            why = "is not an ancestor of HEAD" if repo.is_commit(sha) else "is not a commit in this repository"
            out.append((start, f"{tag}@{sha}: {sha} {why}"))
    return out


def names_source(text: str, repo: Repo | None, here: Path) -> bool:
    """A footnote names a source: a [@key], a commit in HEAD's history, a work item, a snapshot or rollup id, or a
    frozen file under evidence/ that exists (next to the section, or from the repository root)."""
    if CITE_KEY.search(text):
        return True
    if SOURCE_WORD.search(text) and SOURCE_ID.search(text):
        return True
    roots = [here] + ([repo.root] if repo else [])
    if any((r / f).is_file() for f in EVIDENCE_FILE.findall(text) for r in roots):
        return True
    if repo is None:
        return False
    return any(repo.in_history(h) for h in HEX.findall(text)) or any(repo.work_item(w) for w in WORK_ID.findall(text))


def sha256sums(d: Path) -> dict[str, str]:
    """File name -> digest, from the `shasum -a 256` lines of d/SHA256SUMS."""
    f = d / "SHA256SUMS"
    out = {}
    for line in f.read_text(encoding="utf-8").splitlines() if f.is_file() else []:
        digest, _, name = line.strip().partition(" ")
        if name.strip():
            out[name.strip().lstrip("*")] = digest.lower()
    return out


def wrong_digests(text: str, roots: list[Path]) -> list[tuple[int, str]]:
    """A sha256 written right after a frozen file's path must start that file's line in the SHA256SUMS beside it."""
    out = []
    for m in EVIDENCE_FILE.finditer(text):
        d = DIGEST.match(text, m.end())
        f = next((r / m.group(1) for r in roots if (r / m.group(1)).is_file()), None)
        listed = sha256sums(f.parent).get(f.name) if d and f else None
        if listed and not listed.startswith(d.group(1).lower()):
            out.append((d.start(1), f"sha256 `{d.group(1)}` does not match `{m.group(1)}`"
                                    f" (SHA256SUMS: `{listed[:12]}…`)"))
    return out


def check_numbers(doc: Doc, repo: Repo | None) -> list[tuple[int, str]]:
    notes = {b.label: doc.code[b.start:b.end] for b in doc.blocks if b.kind == "footnote"}
    sourced = {label: names_source(text, repo, doc.path.parent) for label, text in notes.items()}
    roots = [doc.path.parent] + ([repo.root] if repo else [])
    out = []
    for b in doc.blocks:
        if b.kind == "footnote":
            out += [(b.start + off, f"[^{b.label}]: {msg}")
                    for off, msg in wrong_digests(doc.code[b.start:b.end], roots)]
            continue
        text = LINK_TARGET.sub(lambda m: blank(m.group(0)), doc.bare[b.start:b.end])
        hits = sorted((m.start(), what, m.group(0)) for what, rx in NUMBER_CLAIMS for m in rx.finditer(text)
                      if not (what == "count" and ORDINAL.search(text[:m.start()])))
        if not hits:
            continue
        prose = doc.prose[b.start:b.end]
        if any(CITE_KEY.search(g.group(1)) for g in CITE_GROUP.finditer(prose)):
            continue
        if any(sourced.get(ref) for ref in FOOTNOTE_REF.findall(prose)):
            continue
        off, what, claim = hits[0]
        more = f" (and {len(hits) - 1} more)" if len(hits) > 1 else ""
        out.append((b.start + off, f"{what} \"{claim.strip()}\"{more} has no source: add a footnote naming its commit,"
                                   " snapshot, rollup key, [@key] or work item"))
    return out


def check_banned(doc: Doc, paper: Paper) -> list[tuple[int, str]]:
    terms = paper.banned(doc)
    if not terms:
        return []
    text = LINK_TARGET.sub(lambda m: blank(m.group(0)), doc.bare)
    text = CITE_GROUP.sub(lambda g: blank(g.group(0)) if "@" in g.group(1) else g.group(0), text)
    out = []
    for term in terms:
        rx = re.compile(r"(?<![\w-])" + r"\s+".join(map(re.escape, term.split())) + r"(?![\w-])", re.I)
        out += [(m.start(), f"banned word \"{term}\" (the README's conventions)") for m in rx.finditer(text)]
    return out


def check_links(doc: Doc, repo: Repo | None) -> list[tuple[int, str]]:
    sys.path.insert(0, str(TOOLS / "docs_integrity"))
    try:
        import check_markdown_links as cml
    finally:
        sys.path.pop(0)
    root = repo.root if repo else doc.path.parent
    rel = doc.path.relative_to(root).as_posix()
    lines = doc.text.split("\n")
    out = []
    for f in cml.check_paths(root, [rel]):
        dest = f.message.rsplit(": ", 1)[-1]
        line = lines[f.line - 1] if 0 < f.line <= len(lines) else ""
        # The checker reads any "[label]: x" line as a link definition: footnotes ([^1]: …) and a citation that
        # opens a line ([@key]: …) are not links, and neither is LaTeX such as [Z_t=L](U_t-\hat m_L).
        ref = re.match(r"^ {0,3}\[[\^@][^\]\n]*\]:[ \t]*(\S+)", CODE_SPAN.sub(lambda c: blank(c.group(0)), line))
        if (ref and ref.group(1) == dest) or "\\" in dest:
            continue
        out.append((doc.line_starts[f.line - 1] if f.line else 0, f.message))
    return out


def lint(doc: Doc, paper: Paper, rules: set[str], factor: float | None, require: str | None) -> list[Finding]:
    repo = paper.repo(doc)
    words = doc.word_count()
    found: list[tuple[str, int, str]] = []

    def add(rule, items):
        for it in items:
            off, msg = it if isinstance(it, tuple) else (0, it)
            found.append((rule, off, msg))

    if "header" in rules:
        add("header", check_header(doc, paper, words))
    if "status" in rules and require and doc.status != require.lower():
        add("status", [f"status is {doc.status or 'missing'}, not {require}"])
    if "budget" in rules and factor is not None:
        add("budget", check_budget(doc, paper, words, factor))
    if "marker" in rules:
        add("marker", check_markers(doc))
    if "citation" in rules:
        add("citation", check_citations(doc, paper))
    if "identifier" in rules:
        add("identifier", check_identifiers(doc, repo) + check_commits(doc, repo))
    if "status-tag" in rules:
        add("status-tag", check_tags(doc, repo))
    if "number" in rules:
        add("number", check_numbers(doc, repo))
    if "banned" in rules:
        add("banned", check_banned(doc, paper))
    if "link" in rules:
        add("link", check_links(doc, repo))
    return sorted(Finding(doc.display, doc.line_of(off), rule, msg) for rule, off, msg in found)


# ---------------------------------------------------------------- command line


def section_files(path: Path) -> list[Path]:
    return sorted(p for p in path.iterdir() if p.is_file() and SECTION_FILE.match(p.name))


def display(path: Path) -> str:
    try:
        return path.relative_to(Path.cwd().resolve()).as_posix()
    except ValueError:
        return str(path)


def report(doc: Doc, paper: Paper, findings: list[Finding], totals: Counter, rule_totals: Counter) -> None:
    words = doc.word_count()
    b = paper.budget(doc)
    if b is None:
        budget = "no budget found"
    elif b.words is None:
        budget = "no budget"
    else:
        budget = f"budget {b.words} ({words / b.words:.2f}×, {b.source})"
    kinds = Counter(k.kind for k in doc.markers)
    totals.update(kinds)
    rules = Counter(f.rule for f in findings)
    rule_totals.update(rules)
    print(doc.display)
    print(f"  status {doc.status or 'missing'} · {words} words · {budget}")
    print("  markers  " + (" · ".join(f"{k} {n}" for k, n in sorted(kinds.items())) or "none"))
    print("  findings " + (" · ".join(f"{r} {rules[r]}" for r in RULES if rules[r]) or "none"))


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="paperlint.py", description=__doc__.split("\n")[0],
                                 epilog="Rules and conventions: see the module docstring (tools/paperlint.py).")
    ap.add_argument("paths", nargs="+", metavar="PATH")
    ap.add_argument("--strict", action="store_true", help="apply every rule (the whitepaper's gate)")
    ap.add_argument("--budget", type=float, metavar="F",
                    help="fail when a file's word count exceeds F times its budget")
    ap.add_argument("--check-identifiers", action="store_true",
                    help="fail on backticked identifiers missing from crates/ and paths missing at HEAD")
    ap.add_argument("--require-status", metavar="S", help="fail unless every file's status header says S")
    ap.add_argument("--report", action="store_true", help="print counts per file and per rule, and exit 0")
    a = ap.parse_args(argv)
    if a.budget is not None and a.budget <= 0:
        ap.error("--budget must be positive")
    if not (a.strict or a.budget is not None or a.check_identifiers or a.require_status or a.report):
        ap.error("nothing to check: pass --strict, --budget F, --check-identifiers, --require-status S or --report")

    files: list[Path] = []
    for raw in a.paths:
        p = Path(raw).resolve()
        if p.is_dir():
            found = section_files(p)
            if not found:
                print(f"paperlint: no section files in {raw}", file=sys.stderr)
                return 2
            files += found
        elif p.is_file():
            files.append(p)
        else:
            print(f"paperlint: {raw}: no such file or directory", file=sys.stderr)
            return 2
    try:
        docs = [Doc(p, display(p)) for p in dict.fromkeys(files)]
    except (OSError, UnicodeDecodeError) as e:
        print(f"paperlint: {e}", file=sys.stderr)
        return 2

    rules = set(RULES) if a.report else set(STRICT) if a.strict else set()
    if a.check_identifiers:
        rules.add("identifier")
    if a.budget is not None:
        rules.add("budget")
    if a.require_status:
        rules.add("status")

    paper = Paper()
    by_repo: dict[Path, tuple[Repo, set[str]]] = {}
    if "identifier" in rules:  # one git grep for every file's identifiers
        for d in docs:
            repo = paper.repo(d)
            if repo:
                by_repo.setdefault(repo.root, (repo, set()))[1].update(r[1] for r in d.refs() if r[0] == "ident")
        for repo, names in by_repo.values():
            repo.idents(names)

    findings: list[Finding] = []
    totals, rule_totals = Counter(), Counter()
    for d in docs:
        got = lint(d, paper, rules, a.budget, a.require_status)
        findings += got
        if a.report:
            report(d, paper, got, totals, rule_totals)
    if a.report:
        print(f"TOTAL {len(docs)} files")
        print("  markers  " + (" · ".join(f"{k} {n}" for k, n in sorted(totals.items())) or "none"))
        print("  findings " + (" · ".join(f"{r} {rule_totals[r]}" for r in RULES if rule_totals[r]) or "none"))
        return 0
    for f in findings:
        print(f)
    if findings:
        n = len({f.path for f in findings})
        print(f"paperlint: {len(findings)} problem{'s' * (len(findings) != 1)} in {n} of {len(docs)} files")
        return 1
    print(f"paperlint: {len(docs)} file{'s' * (len(docs) != 1)} clean")
    return 0


if __name__ == "__main__":
    sys.exit(main())
