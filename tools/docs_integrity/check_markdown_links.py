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


_FENCE_RE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
_ATX_HEADING_RE = re.compile(r"^ {0,3}(#{1,6})(?:[ \t]+|$)(.*)$")
_SETEXT_RE = re.compile(r"^ {0,3}(?:=+|-+)[ \t]*$")
_INLINE_LINK_RE = re.compile(r"!?\[[^\]\n]*\]\(([^\n)]*)\)")
# A `[^label]:` line is a GitHub footnote definition, not a reference link.
_REFERENCE_TARGET_RE = re.compile(r"^ {0,3}\[(?!\^)[^\]\n]+\]:[ \t]*(\S+)")
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
    if findings:
        for finding in findings:
            print(finding, file=sys.stderr)
        print(f"Markdown link check failed with {len(findings)} finding(s).", file=sys.stderr)
        return 1
    print("Markdown local path and anchor check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
