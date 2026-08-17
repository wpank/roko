#!/usr/bin/env python3
"""Check bounded local Markdown paths and GitHub-style heading anchors.

The default scope is the maintained operator/developer documentation corpus. The
checker deliberately does not make network requests: external URLs, site-root
URLs, and non-file URI schemes are outside this gate.
"""

from __future__ import annotations

import argparse
import dataclasses
import html
import re
import sys
import tomllib
import unicodedata
from pathlib import Path
from urllib.parse import unquote, urlsplit


REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_INPUTS = ("README.md", "CLAUDE.md", "docker/README.md", "docs/v2")
MAX_REPORTED_FINDINGS = 256


@dataclasses.dataclass(frozen=True)
class Limits:
    max_files: int = 512
    max_file_bytes: int = 4 * 1024 * 1024
    max_total_bytes: int = 32 * 1024 * 1024
    max_links: int = 50_000


DEFAULT_LIMITS = Limits()


@dataclasses.dataclass(frozen=True, order=True)
class Finding:
    path: str
    line: int
    message: str

    def __str__(self) -> str:
        location = f"{self.path}:{self.line}" if self.line else self.path
        return f"{location}: {self.message}"


@dataclasses.dataclass(frozen=True)
class ParsedMarkdown:
    anchors: frozenset[str]
    links: tuple[tuple[int, str], ...]


@dataclasses.dataclass(frozen=True)
class SourceManifestRow:
    source: str
    owner: str


@dataclasses.dataclass(frozen=True)
class CoverageLedgerSpec:
    filename: str
    header: tuple[str, ...]
    source_column: int
    owner_column: int
    plan_directory: str


@dataclasses.dataclass(frozen=True)
class CoverageLedgerRow:
    source: str
    owner: str
    line: int


_FENCE_RE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
_ATX_HEADING_RE = re.compile(r"^ {0,3}(#{1,6})(?:[ \t]+|$)(.*)$")
_SETEXT_RE = re.compile(r"^ {0,3}(?:=+|-+)[ \t]*$")
_INLINE_LINK_RE = re.compile(r"!?\[[^\]\n]*\]\(([^\n)]*)\)")
_REFERENCE_TARGET_RE = re.compile(r"^ {0,3}\[[^\]\n]+\]:[ \t]*(\S+)")
_HTML_ANCHOR_RE = re.compile(
    r"<a\b[^>]*\b(?:id|name)\s*=\s*(?:\"([^\"]+)\"|'([^']+)'|([^\s>]+))",
    re.IGNORECASE,
)
_INLINE_CODE_RE = re.compile(r"(`+)(.*?)\1")
_IMAGE_RE = re.compile(r"!\[([^\]]*)\]\([^)]*\)")
_LINK_RE = re.compile(r"\[([^\]]+)\]\([^)]*\)")
_REFERENCE_LINK_RE = re.compile(r"\[([^\]]+)\]\[[^\]]*\]")
_HTML_TAG_RE = re.compile(r"<[^>]+>")
_AUTOLINK_RE = re.compile(
    r"<((?:[A-Za-z][A-Za-z0-9+.-]{1,31}:[^ <>]*)|"
    r"(?:[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9.-]+))>"
)
_STRONG_UNDERSCORE_RE = re.compile(r"(?<!\w)__(?=\S)(.+?)(?<=\S)__(?!\w)")
_EMPHASIS_UNDERSCORE_RE = re.compile(r"(?<!\w)_(?=\S)(.+?)(?<=\S)_(?!\w)")
_SOURCE_MANIFEST_ROW_RE = re.compile(
    r"^\| `((?:docs/v1|docs/v2|docs/v2-depth)/[^`]+\.md)` "
    r"\| [^|]+ \| `[^`]+` \| `([A-Z0-9][A-Z0-9-]+\.md)` \|$"
)
_SOURCE_MANIFEST_COUNT_RE = re.compile(r"^\| `docs/(?:v1|v2|v2-depth)` \| \d+ \|$")
_EXACT_CELL_RE = re.compile(r"^`?([^`|]+)`?$")

_COVERAGE_LEDGER_SPECS = (
    CoverageLedgerSpec(
        "status-quo-corpus.md",
        ("Source", "Downstream scan focus", "Coverage task", "Ledger state"),
        0,
        2,
        "DOC-status-quo-corpus",
    ),
    CoverageLedgerSpec(
        "docs-v1-kernel.md",
        ("Source", "Local task", "Status", "Backlog mapping / follow-up / reason"),
        0,
        1,
        "DOC-v1-kernel",
    ),
    CoverageLedgerSpec(
        "docs-v1-cognition.md",
        ("Source path", "Task id", "Status"),
        0,
        1,
        "DOC-v1-cognition",
    ),
    CoverageLedgerSpec(
        "docs-v1-ecosystem.md",
        ("Source path", "Task id"),
        0,
        1,
        "DOC-v1-ecosystem",
    ),
    CoverageLedgerSpec(
        "docs-v2-core.md",
        ("Source file", "Writer", "Read-only cross-checks", "Subject"),
        0,
        1,
        "DOC-v2-core",
    ),
    CoverageLedgerSpec(
        "docs-v2-depth.md",
        ("Source file", "Task"),
        0,
        1,
        "DOC-v2-depth",
    ),
)


def _inside_root(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def discover_files(
    root: Path, inputs: list[str], limits: Limits
) -> tuple[list[Path], list[Finding]]:
    root = root.resolve()
    files: set[Path] = set()
    findings: list[Finding] = []

    overflow = False
    for raw_input in inputs:
        candidate = (root / raw_input).resolve()
        display = raw_input
        if not _inside_root(candidate, root):
            findings.append(Finding(display, 0, "input escapes repository root"))
            continue
        if not candidate.exists():
            findings.append(Finding(display, 0, "input does not exist"))
            continue
        candidates = candidate.rglob("*.md") if candidate.is_dir() else (candidate,)
        for path in candidates:
            resolved = path.resolve()
            if not _inside_root(resolved, root):
                findings.append(
                    Finding(
                        path.relative_to(root).as_posix(),
                        0,
                        "Markdown file escapes repository root",
                    )
                )
                continue
            if resolved.is_file() and resolved.suffix.lower() == ".md":
                files.add(resolved)
                if len(files) > limits.max_files:
                    overflow = True
                    break
        if overflow:
            break

    ordered = sorted(files, key=lambda path: path.relative_to(root).as_posix())
    if overflow:
        findings.append(
            Finding(
                ".",
                0,
                f"Markdown file count exceeds limit {limits.max_files}",
            )
        )
        return ordered[: limits.max_files], findings
    return ordered, findings


def _read_markdown(
    path: Path, root: Path, limits: Limits
) -> tuple[str | None, Finding | None, int]:
    display = path.relative_to(root).as_posix()
    try:
        size = path.stat().st_size
    except OSError as error:
        return None, Finding(display, 0, f"cannot stat file: {error}"), 0
    if size > limits.max_file_bytes:
        return (
            None,
            Finding(display, 0, f"file size {size} exceeds limit {limits.max_file_bytes}"),
            size,
        )
    try:
        data = path.read_bytes()
    except OSError as error:
        return None, Finding(display, 0, f"cannot read file: {error}"), size
    try:
        return data.decode("utf-8"), None, len(data)
    except UnicodeDecodeError as error:
        return None, Finding(display, 0, f"file is not valid UTF-8: {error}"), len(data)


def _mask_inline_code(line: str) -> str:
    return _INLINE_CODE_RE.sub(lambda match: " " * len(match.group(0)), line)


def _plain_heading(value: str) -> str:
    protected: list[str] = []

    def protect(text: str) -> str:
        token = f"\x00{len(protected)}\x00"
        protected.append(text)
        return token

    def code_span(match: re.Match[str]) -> str:
        content = match.group(2).replace("\n", " ")
        if (
            content.startswith(" ")
            and content.endswith(" ")
            and not content.isspace()
        ):
            content = content[1:-1]
        return protect(content)

    value = value.rstrip()
    value = re.sub(r"[ \t]+#+[ \t]*$", "", value)
    value = _INLINE_CODE_RE.sub(code_span, value)
    value = re.sub(r"\\_", lambda _match: protect("_"), value)
    value = _IMAGE_RE.sub(lambda match: match.group(1), value)
    value = _LINK_RE.sub(lambda match: match.group(1), value)
    value = _REFERENCE_LINK_RE.sub(lambda match: match.group(1), value)
    value = _AUTOLINK_RE.sub(lambda match: match.group(1), value)
    value = _HTML_TAG_RE.sub("", value)
    value = re.sub(r"\\([\\`*{}\[\]()#+\-.!_>])", r"\1", value)
    # CommonMark permits intraword underscores as literals. Support the
    # unambiguous balanced `_emphasis_` and `__strong__` forms here; unmatched
    # delimiter runs remain literal and are subsequently retained by GitHub's
    # slug character rules.
    value = _STRONG_UNDERSCORE_RE.sub(lambda match: match.group(1), value)
    value = _EMPHASIS_UNDERSCORE_RE.sub(lambda match: match.group(1), value)
    value = value.replace("*", "").replace("~", "")
    for index, content in enumerate(protected):
        value = value.replace(f"\x00{index}\x00", content)
    return html.unescape(value).strip()


def github_slug(value: str) -> str:
    """Return the GitHub-style base slug for already-extracted heading text.

    GitHub keeps Unicode letters/numbers/marks plus ``-`` and ``_``, removes
    punctuation/symbols, lowercases, maps ASCII spaces to ``-``, and removes
    other whitespace.
    Duplicate suffixing is handled by ``parse_markdown``.
    """

    kept: list[str] = []
    for char in value.lower():
        if char == " ":
            kept.append("-")
        elif char.isspace():
            continue
        elif char in "-_" or unicodedata.category(char)[0] in {"L", "M", "N"}:
            kept.append(char)
    return "".join(kept)


def _link_destination(raw: str) -> str:
    raw = raw.strip()
    if raw.startswith("<"):
        end = raw.find(">", 1)
        return raw[1:end] if end >= 0 else raw
    return raw.split(maxsplit=1)[0] if raw else ""


def parse_markdown(
    text: str, max_links: int, *, collect_links: bool = True
) -> tuple[ParsedMarkdown, str | None]:
    anchors: set[str] = set()
    generated_anchors: set[str] = set()
    base_counts: dict[str, int] = {}
    links: list[tuple[int, str]] = []
    fence_char: str | None = None
    fence_len = 0
    previous_heading_candidate: tuple[int, str] | None = None

    def add_heading(value: str) -> None:
        base = github_slug(_plain_heading(value))
        candidate = base
        suffix = base_counts.get(base, 0)
        while candidate in generated_anchors:
            suffix += 1
            candidate = f"{base}-{suffix}"
        base_counts[base] = suffix
        generated_anchors.add(candidate)
        anchors.add(candidate)

    for line_number, line in enumerate(text.splitlines(), 1):
        fence = _FENCE_RE.match(line)
        if fence:
            marker = fence.group(1)
            if fence_char is None:
                fence_char = marker[0]
                fence_len = len(marker)
            elif marker[0] == fence_char and len(marker) >= fence_len:
                fence_char = None
                fence_len = 0
            previous_heading_candidate = None
            continue
        if fence_char is not None:
            continue

        heading = _ATX_HEADING_RE.match(line)
        if heading:
            add_heading(heading.group(2))
        elif _SETEXT_RE.match(line) and previous_heading_candidate is not None:
            add_heading(previous_heading_candidate[1])

        for match in _HTML_ANCHOR_RE.finditer(line):
            explicit = next(group for group in match.groups() if group is not None)
            anchors.add(html.unescape(explicit))

        if collect_links:
            visible = _mask_inline_code(line)
            for match in _INLINE_LINK_RE.finditer(visible):
                target = _link_destination(match.group(1))
                if target:
                    links.append((line_number, target))
            reference = _REFERENCE_TARGET_RE.match(visible)
            if reference:
                target = _link_destination(reference.group(1))
                if target:
                    links.append((line_number, target))
            if len(links) > max_links:
                return ParsedMarkdown(frozenset(anchors), tuple(links[:max_links])), (
                    f"link count exceeds limit {max_links}"
                )

        stripped = line.strip()
        if stripped and not heading and not _SETEXT_RE.match(line):
            previous_heading_candidate = (line_number, line)
        else:
            previous_heading_candidate = None

    return ParsedMarkdown(frozenset(anchors), tuple(links)), None


def check_paths(
    root: Path,
    inputs: list[str] | None = None,
    limits: Limits = DEFAULT_LIMITS,
) -> list[Finding]:
    root = root.resolve()
    files, findings = discover_files(root, inputs or list(DEFAULT_INPUTS), limits)
    parsed: dict[Path, ParsedMarkdown] = {}
    anchor_failures: dict[Path, str] = {}
    accounted_paths: set[Path] = set()
    total_bytes = 0
    total_links = 0

    for path in files:
        accounted_paths.add(path)
        text, read_error, size = _read_markdown(path, root, limits)
        total_bytes += size
        if total_bytes > limits.max_total_bytes:
            anchor_failures[path] = (
                f"Markdown bytes exceed aggregate limit {limits.max_total_bytes}"
            )
            findings.append(
                Finding(
                    ".",
                    0,
                    f"Markdown bytes exceed aggregate limit {limits.max_total_bytes}",
                )
            )
            break
        if read_error is not None:
            findings.append(read_error)
            anchor_failures[path] = read_error.message
            continue
        assert text is not None
        document, parse_error = parse_markdown(text, limits.max_links - total_links)
        total_links += len(document.links)
        parsed[path] = document
        if parse_error is not None:
            findings.append(Finding(path.relative_to(root).as_posix(), 0, parse_error))
            anchor_failures[path] = parse_error
            break

    for source in files:
        document = parsed.get(source)
        if document is None:
            continue
        display = source.relative_to(root).as_posix()
        for line_number, destination in document.links:
            decoded = html.unescape(destination)
            try:
                split = urlsplit(decoded)
            except ValueError:
                findings.append(Finding(display, line_number, "local link target is malformed"))
                continue
            if split.scheme or split.netloc or decoded.startswith("/"):
                continue
            raw_path = unquote(split.path)
            fragment = unquote(split.fragment)
            target = source if not raw_path else (source.parent / raw_path).resolve()
            if not _inside_root(target, root):
                findings.append(
                    Finding(
                        display,
                        line_number,
                        f"local link escapes repository: {destination}",
                    )
                )
                continue
            if not target.exists():
                findings.append(
                    Finding(
                        display,
                        line_number,
                        f"local link target does not exist: {destination}",
                    )
                )
                continue
            if fragment:
                target_document = parsed.get(target)
                target_failure = anchor_failures.get(target)
                if (
                    target_document is None
                    and target_failure is None
                    and target.suffix.lower() == ".md"
                ):
                    if target not in accounted_paths and len(accounted_paths) >= limits.max_files:
                        target_failure = "linked Markdown file count exceeds limit"
                    else:
                        text, read_error, size = _read_markdown(target, root, limits)
                    if target_failure is None and target not in accounted_paths:
                        accounted_paths.add(target)
                        total_bytes += size
                    if target_failure is None and total_bytes > limits.max_total_bytes:
                        target_failure = "linked Markdown bytes exceed aggregate limit"
                    elif target_failure is None and read_error is not None:
                        target_failure = read_error.message
                    elif target_failure is None:
                        assert text is not None
                        # Lazy targets are parsed for anchors only. Their outbound
                        # links are outside the selected corpus, so they do not
                        # consume or evade the corpus-wide link budget.
                        target_document, parse_error = parse_markdown(
                            text, 0, collect_links=False
                        )
                        if parse_error is not None:
                            target_failure = parse_error
                        else:
                            parsed[target] = target_document
                    if target_failure is not None:
                        anchor_failures[target] = target_failure
                if target_failure is not None:
                    findings.append(
                        Finding(
                            display,
                            line_number,
                            f"cannot inspect anchor target: {destination} ({target_failure})",
                        )
                    )
                    continue
                if target_document is not None and fragment not in target_document.anchors:
                    findings.append(
                        Finding(
                            display,
                            line_number,
                            f"local anchor does not exist: {destination}",
                        )
                    )

    return _bounded_findings(findings)


def _bounded_findings(findings: list[Finding]) -> list[Finding]:
    ordered = sorted(set(findings))
    if len(ordered) <= MAX_REPORTED_FINDINGS:
        return ordered
    omitted = len(ordered) - (MAX_REPORTED_FINDINGS - 1)
    return ordered[: MAX_REPORTED_FINDINGS - 1] + [
        Finding(".", 0, f"{omitted} additional finding(s) omitted")
    ]


def check_status_disposition_registry(
    root: Path, limits: Limits = DEFAULT_LIMITS
) -> list[Finding]:
    """Validate the fixed 109-file top-level status disposition contract."""

    root = root.resolve()
    status_dir = root / "tmp/status-quo"
    display = "tmp/status-quo/00-INDEX.md"
    findings: list[Finding] = []
    direct = sorted(status_dir.glob("*.md"), key=lambda path: path.name)
    if len(direct) != 109:
        findings.append(
            Finding(display, 0, f"top-level status Markdown count is {len(direct)}, expected 109")
        )

    required_special = {
        "00-INDEX.md",
        "DOC-MANIFEST.md",
        "MASTER-EXECUTION-CHECKLIST.md",
    }
    names = {path.name for path in direct}
    for missing in sorted(required_special - names):
        findings.append(Finding(display, 0, f"disposition registry file is missing: {missing}"))

    numbered: dict[int, list[str]] = {}
    for name in sorted(names - required_special):
        match = re.fullmatch(r"(\d{2,3})-[A-Z0-9][A-Z0-9-]*\.md", name)
        if match is None:
            findings.append(Finding(display, 0, f"unclassified top-level status document: {name}"))
            continue
        numbered.setdefault(int(match.group(1)), []).append(name)
    expected_numbers = set(range(1, 107))
    actual_numbers = set(numbered)
    if actual_numbers != expected_numbers:
        missing = ", ".join(str(number) for number in sorted(expected_numbers - actual_numbers))
        extra = ", ".join(str(number) for number in sorted(actual_numbers - expected_numbers))
        parts = (
            f"missing numbers: {missing}" if missing else "",
            f"extra numbers: {extra}" if extra else "",
        )
        detail = "; ".join(part for part in parts if part)
        findings.append(
            Finding(
                display,
                0,
                f"numbered status registry must be exactly 01..106 ({detail})",
            )
        )
    for number, matching_names in sorted(numbered.items()):
        if len(matching_names) != 1:
            findings.append(
                Finding(
                    display,
                    0,
                    f"status number {number:02d} has {len(matching_names)} documents",
                )
            )

    index_text, index_error, _ = _read_markdown(status_dir / "00-INDEX.md", root, limits)
    if index_error is not None:
        findings.append(
            Finding(
                display,
                0,
                f"cannot read disposition registry: {index_error.message}",
            )
        )
        return _bounded_findings(findings)
    assert index_text is not None
    required_registry_text = (
        "**Disposition: CURRENT NAVIGATION",
        "The 109 direct Markdown files under `tmp/status-quo/`",
        "| `00-INDEX.md` | 1 | `CURRENT-NAVIGATION`",
        "| `MASTER-EXECUTION-CHECKLIST.md` | 1 | `CURRENT-CONTROL`",
        "| `DOC-MANIFEST.md` | 1 | `GENERATED-HISTORICAL`",
        "| Numbered `01-*.md` through `106-*.md` | 106 | `HISTORICAL-AUDIT`",
        "| **Total** | **109** |",
    )
    for required in required_registry_text:
        if required not in index_text:
            findings.append(
                Finding(
                    display,
                    0,
                    f"disposition registry is missing contract text: {required}",
                )
            )

    manifest = status_dir / "DOC-MANIFEST.md"
    try:
        with manifest.open("rb") as stream:
            manifest_prefix = stream.read(4096).decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        findings.append(
            Finding(
                "tmp/status-quo/DOC-MANIFEST.md",
                0,
                f"cannot read disposition: {error}",
            )
        )
    else:
        if "**Disposition: GENERATED-HISTORICAL.**" not in manifest_prefix:
            findings.append(
                Finding(
                    "tmp/status-quo/DOC-MANIFEST.md",
                    0,
                    "missing GENERATED-HISTORICAL disposition",
                )
            )

    control = status_dir / "MASTER-EXECUTION-CHECKLIST.md"
    try:
        with control.open("rb") as stream:
            control_prefix = stream.read(4096).decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        findings.append(
            Finding(
                "tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md",
                0,
                f"cannot read control status: {error}",
            )
        )
    else:
        if "Status: active control document" not in control_prefix:
            findings.append(
                Finding(
                    "tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md",
                    0,
                    "missing active control-document status",
                )
            )

    return _bounded_findings(findings)


def _source_corpus(root: Path) -> dict[str, list[Path]]:
    return {
        "tmp/status-quo": sorted((root / "tmp/status-quo").glob("*.md")),
        "docs/v1": sorted((root / "docs/v1").rglob("*.md")),
        "docs/v2": sorted((root / "docs/v2").rglob("*.md")),
        "docs/v2-depth": sorted((root / "docs/v2-depth").rglob("*.md")),
    }


def parse_source_manifest(text: str) -> tuple[list[SourceManifestRow], list[str]]:
    """Return exact four-column manifest rows and malformed row candidates."""

    rows: list[SourceManifestRow] = []
    malformed: list[str] = []
    for line in text.splitlines():
        if not line.startswith("| `docs/"):
            continue
        match = _SOURCE_MANIFEST_ROW_RE.fullmatch(line)
        if match is None and not _SOURCE_MANIFEST_COUNT_RE.fullmatch(line):
            malformed.append(line)
        elif match is not None:
            rows.append(SourceManifestRow(match.group(1), match.group(2)))
    return rows, malformed


def validate_source_manifest(
    expected_sources: set[str], rows: list[SourceManifestRow], owner_names: set[str]
) -> list[str]:
    """Validate exact source membership and resolvable top-level owner cells."""

    errors: list[str] = []
    paths = [row.source for row in rows]
    path_set = set(paths)
    if len(paths) != len(path_set):
        errors.append(f"source manifest has {len(paths) - len(path_set)} duplicate row(s)")
    errors.extend(
        f"source missing from manifest: {source}"
        for source in sorted(expected_sources - path_set)
    )
    errors.extend(
        f"manifest source does not exist: {source}"
        for source in sorted(path_set - expected_sources)
    )
    errors.extend(
        "manifest owner does not resolve to a top-level status document: " + row.owner
        for row in rows
        if row.owner not in owner_names
    )
    return errors


def _markdown_table_cells(line: str) -> tuple[str, ...] | None:
    if not line.startswith("|") or not line.endswith("|"):
        return None
    return tuple(cell.strip() for cell in line[1:-1].split("|"))


def _exact_cell_value(cell: str) -> str | None:
    code = re.fullmatch(r"`([^`]+)`", cell)
    if code is not None:
        return code.group(1)
    plain = re.fullmatch(r"[^`|]+", cell)
    if plain is None or plain.group(0) != plain.group(0).strip():
        return None
    return plain.group(0)


def parse_coverage_ledger(
    text: str, spec: CoverageLedgerSpec
) -> tuple[list[CoverageLedgerRow], list[str]]:
    """Parse only the ledger's exact primary table schema."""

    lines = text.splitlines()
    rows: list[CoverageLedgerRow] = []
    errors: list[str] = []
    header_count = 0
    index = 0
    while index < len(lines):
        cells = _markdown_table_cells(lines[index])
        if cells != spec.header:
            index += 1
            continue
        header_count += 1
        if index + 1 >= len(lines):
            errors.append(f"line {index + 1}: primary table has no delimiter row")
            break
        delimiter = _markdown_table_cells(lines[index + 1])
        if delimiter is None or len(delimiter) != len(spec.header) or not all(
            re.fullmatch(r":?-{3,}:?", cell) for cell in delimiter
        ):
            errors.append(f"line {index + 2}: invalid primary table delimiter")
            index += 1
            continue
        index += 2
        while index < len(lines):
            cells = _markdown_table_cells(lines[index])
            if cells is None:
                break
            if len(cells) != len(spec.header):
                errors.append(
                    f"line {index + 1}: primary row has {len(cells)} cells, "
                    f"expected {len(spec.header)}"
                )
                index += 1
                continue
            source = _exact_cell_value(cells[spec.source_column])
            owner = _exact_cell_value(cells[spec.owner_column])
            if source is None or re.fullmatch(
                r"(?:tmp/status-quo|docs/v1|docs/v2|docs/v2-depth)/[^`|]+\.md",
                source,
            ) is None:
                errors.append(f"line {index + 1}: invalid exact source cell")
            elif owner is None or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]+", owner) is None:
                errors.append(f"line {index + 1}: invalid exact owner cell")
            else:
                rows.append(CoverageLedgerRow(source, owner, index + 1))
            index += 1
        continue
    if header_count == 0:
        errors.append("exact primary table header is missing")
    return rows, errors


def parse_doc_plan(data: bytes) -> tuple[dict[str, frozenset[str]], list[str]]:
    """Parse task IDs and their exact context.read_files paths."""

    try:
        document = tomllib.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        return {}, [f"invalid tasks TOML: {error}"]
    tasks = document.get("task")
    if not isinstance(tasks, list):
        return {}, ["tasks TOML has no [[task]] array"]
    contexts: dict[str, frozenset[str]] = {}
    errors: list[str] = []
    for number, task in enumerate(tasks, 1):
        if not isinstance(task, dict):
            errors.append(f"task {number} is not a table")
            continue
        task_id = task.get("id")
        if not isinstance(task_id, str) or not task_id:
            errors.append(f"task {number} has no non-empty string id")
            continue
        if task_id in contexts:
            errors.append(f"duplicate task id: {task_id}")
            continue
        context = task.get("context", {})
        if not isinstance(context, dict):
            errors.append(f"task {task_id} context is not a table")
            continue
        read_files = context.get("read_files", [])
        if not isinstance(read_files, list):
            errors.append(f"task {task_id} context.read_files is not an array")
            continue
        paths: set[str] = set()
        for read_file in read_files:
            if not isinstance(read_file, dict) or not isinstance(read_file.get("path"), str):
                errors.append(f"task {task_id} has an invalid context.read_files entry")
                continue
            paths.add(read_file["path"])
        contexts[task_id] = frozenset(paths)
    return contexts, errors


def validate_source_ownership(
    expected_sources: set[str],
    ledger_rows: list[tuple[str, CoverageLedgerRow]],
    plan_contexts: dict[str, dict[str, frozenset[str]]],
) -> list[str]:
    """Validate exact ledger membership and named-task context ownership."""

    errors: list[str] = []
    by_source: dict[str, list[tuple[str, CoverageLedgerRow]]] = {}
    for plan_directory, row in ledger_rows:
        by_source.setdefault(row.source, []).append((plan_directory, row))
    for source, occurrences in sorted(by_source.items()):
        if len(occurrences) > 1:
            errors.append(f"duplicate ledger source: {source}")
    for source in sorted(expected_sources - by_source.keys()):
        errors.append(f"source missing from coverage ledgers: {source}")
    for source in sorted(by_source.keys() - expected_sources):
        errors.append(f"coverage ledger has extra source: {source}")
    for plan_directory, row in ledger_rows:
        tasks = plan_contexts.get(plan_directory)
        if tasks is None:
            errors.append(f"owner plan is missing: {plan_directory}")
            continue
        paths = tasks.get(row.owner)
        if paths is None:
            errors.append(
                f"ledger owner task is missing: {plan_directory}/{row.owner} for {row.source}"
            )
        elif row.source not in paths:
            errors.append(
                f"owner context omits source: {plan_directory}/{row.owner} -> {row.source}"
            )
    return errors


def check_source_coverage_registry(root: Path, limits: Limits = DEFAULT_LIMITS) -> list[Finding]:
    """Validate deterministic corpus counts and exact source ownership coverage."""

    root = root.resolve()
    display = "tmp/status-quo/backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md"
    findings: list[Finding] = []
    corpora = _source_corpus(root)
    expected_counts = {
        "tmp/status-quo": 109,
        "docs/v1": 417,
        "docs/v2": 35,
        "docs/v2-depth": 185,
    }
    for name, expected in expected_counts.items():
        actual = len(corpora[name])
        if actual != expected:
            findings.append(
                Finding(
                    display,
                    0,
                    f"{name} corpus count is {actual}, expected {expected}",
                )
            )
    sources = [path for name in expected_counts for path in corpora[name]]
    if len(sources) != 746:
        findings.append(Finding(display, 0, f"source corpus count is {len(sources)}, expected 746"))

    ledger_root = root / "tmp/status-quo/backlog/source-coverage"
    plan_root = root / "tmp/status-quo/backlog/plans"
    actual_ledger_names = {path.name for path in ledger_root.glob("*.md")}
    expected_ledger_names = {spec.filename for spec in _COVERAGE_LEDGER_SPECS}
    for missing in sorted(expected_ledger_names - actual_ledger_names):
        findings.append(Finding(display, 0, f"coverage ledger is missing: {missing}"))
    for extra in sorted(actual_ledger_names - expected_ledger_names):
        findings.append(Finding(display, 0, f"unexpected coverage ledger: {extra}"))
    actual_plan_names = {
        path.parent.name for path in plan_root.glob("DOC-*/tasks.toml") if path.is_file()
    }
    expected_plan_names = {spec.plan_directory for spec in _COVERAGE_LEDGER_SPECS}
    for missing in sorted(expected_plan_names - actual_plan_names):
        findings.append(Finding(display, 0, f"DOC plan is missing: {missing}"))
    for extra in sorted(actual_plan_names - expected_plan_names):
        findings.append(Finding(display, 0, f"unexpected DOC plan: {extra}"))

    input_bytes = 0

    def bounded_read(path: Path, kind: str) -> bytes | None:
        nonlocal input_bytes
        try:
            size = path.stat().st_size
        except OSError as error:
            findings.append(Finding(display, 0, f"cannot stat {kind} {path}: {error}"))
            return None
        if size > limits.max_file_bytes or input_bytes + size > limits.max_total_bytes:
            findings.append(Finding(display, 0, f"{kind} input exceeds byte limits: {path}"))
            return None
        try:
            data = path.read_bytes()
        except OSError as error:
            findings.append(Finding(display, 0, f"cannot read {kind} {path}: {error}"))
            return None
        input_bytes += len(data)
        return data

    summary_data = bounded_read(root / display, "coverage summary")
    try:
        summary_text = summary_data.decode("utf-8") if summary_data is not None else ""
    except UnicodeDecodeError as error:
        findings.append(Finding(display, 0, f"coverage summary is not UTF-8: {error}"))
        summary_text = ""
    source_manifest_path = root / "tmp/status-quo/80-SOURCE-DOC-MANIFEST.md"
    source_manifest_data = bounded_read(source_manifest_path, "source manifest")
    try:
        source_manifest_text = (
            source_manifest_data.decode("utf-8") if source_manifest_data is not None else ""
        )
    except UnicodeDecodeError as error:
        findings.append(
            Finding(
                "tmp/status-quo/80-SOURCE-DOC-MANIFEST.md",
                0,
                f"source manifest is not UTF-8: {error}",
            )
        )
        source_manifest_text = ""
    required_count_contracts = (
        (summary_text, "| `tmp/status-quo/*.md` | 109 |"),
        (summary_text, "| `docs/v2/**/*.md` | 35 |"),
        (summary_text, "| **Total** | **746** |"),
    )
    for text, required in required_count_contracts:
        if required not in text:
            findings.append(
                Finding(
                    display,
                    0,
                    f"source registry is missing count contract: {required}",
                )
            )

    manifest_rows, malformed_rows = parse_source_manifest(source_manifest_text)
    for row in malformed_rows:
        findings.append(
            Finding(
                "tmp/status-quo/80-SOURCE-DOC-MANIFEST.md",
                0,
                f"malformed source manifest row: {row[:160]}",
            )
        )
    expected_manifest_paths = {
        path.relative_to(root).as_posix()
        for name in ("docs/v1", "docs/v2", "docs/v2-depth")
        for path in corpora[name]
    }
    status_names = {path.name for path in corpora["tmp/status-quo"]}
    findings.extend(
        Finding("tmp/status-quo/80-SOURCE-DOC-MANIFEST.md", 0, error)
        for error in validate_source_manifest(
            expected_manifest_paths, manifest_rows, status_names
        )
    )
    manifest_count_contracts = (
        "| `docs/v1` | 417 |",
        "| `docs/v2` | 35 |",
        "| `docs/v2-depth` | 185 |",
        "| Total | 637 |",
    )
    for required in manifest_count_contracts:
        if required not in source_manifest_text:
            findings.append(
                Finding(
                    "tmp/status-quo/80-SOURCE-DOC-MANIFEST.md",
                    0,
                    f"source manifest is missing count contract: {required}",
                )
            )
    ledger_rows: list[tuple[str, CoverageLedgerRow]] = []
    plan_contexts: dict[str, dict[str, frozenset[str]]] = {}
    global_task_ids: dict[str, str] = {}
    for spec in _COVERAGE_LEDGER_SPECS:
        ledger_path = ledger_root / spec.filename
        ledger_data = bounded_read(ledger_path, "coverage ledger")
        if ledger_data is not None:
            try:
                ledger_text = ledger_data.decode("utf-8")
            except UnicodeDecodeError as error:
                findings.append(
                    Finding(spec.filename, 0, f"coverage ledger is not UTF-8: {error}")
                )
            else:
                rows, errors = parse_coverage_ledger(ledger_text, spec)
                ledger_rows.extend((spec.plan_directory, row) for row in rows)
                findings.extend(
                    Finding(spec.filename, 0, f"coverage ledger: {error}")
                    for error in errors
                )

        plan_path = plan_root / spec.plan_directory / "tasks.toml"
        plan_data = bounded_read(plan_path, "DOC plan")
        if plan_data is not None:
            contexts, errors = parse_doc_plan(plan_data)
            plan_contexts[spec.plan_directory] = contexts
            findings.extend(
                Finding(plan_path.relative_to(root).as_posix(), 0, error) for error in errors
            )
            for task_id in contexts:
                prior = global_task_ids.setdefault(task_id, spec.plan_directory)
                if prior != spec.plan_directory:
                    findings.append(
                        Finding(
                            display,
                            0,
                            f"duplicate task id across DOC plans: {task_id} ({prior}, {spec.plan_directory})",
                        )
                    )

    expected_sources = {path.relative_to(root).as_posix() for path in sources}
    findings.extend(
        Finding(display, 0, error)
        for error in validate_source_ownership(expected_sources, ledger_rows, plan_contexts)
    )

    return _bounded_findings(findings)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        default=list(DEFAULT_INPUTS),
        help="repository-relative Markdown files/directories (maintained corpus by default)",
    )
    args = parser.parse_args(argv)
    findings = check_paths(REPO_ROOT, args.paths)
    if tuple(args.paths) == DEFAULT_INPUTS:
        findings.extend(check_status_disposition_registry(REPO_ROOT))
        findings.extend(check_source_coverage_registry(REPO_ROOT))
        findings = _bounded_findings(findings)
    if findings:
        for finding in findings:
            print(finding, file=sys.stderr)
        print(f"Markdown link check failed with {len(findings)} finding(s).", file=sys.stderr)
        return 1
    print("Markdown local path and anchor check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
