#!/usr/bin/env python3
"""Deterministic HTTP route inventory for roko-serve.

Scans literal Axum `.route("/path", method(handler)...)` registrations under
``crates/roko-serve/src`` and expands every chained method/handler pair.

Reports:
  (a) all method registrations including aliases, and
  (b) canonical registrations after deduplicating (method, handler-symbol) pairs
      across the same parsed local router scope.

Limitations (documented, not defects):
  - Nested prefixes (`.nest("/api", ...)`) change effective runtime paths but
    do not affect the raw registration count.
  - `route_service`, feature gates, and runtime-composed paths are not analyzed.
  - Duplicate/conflict detection is per-parsed-local-router only.

Usage:
  python3 tools/http_route_inventory.py                  # report to stdout
  python3 tools/http_route_inventory.py --json           # JSON to stdout
  python3 tools/http_route_inventory.py --refresh        # write snapshot
  python3 tools/http_route_inventory.py --check-snapshot  # CI gate
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Optional

# ---------------------------------------------------------------------------
# Types
# ---------------------------------------------------------------------------

@dataclass
class RouteRegistration:
    """A single (method, path, handler) registration found in source."""
    source_file: str
    line: int
    router_fn: str
    path: str
    method: str
    handler: str

    def key(self) -> tuple[str, str]:
        """(method, handler) dedup key."""
        return (self.method, self.handler)


@dataclass
class UnparsedRegistration:
    """A `.route(` call that the parser could not fully expand."""
    source_file: str
    line: int
    raw_text: str
    reason: str


@dataclass
class Conflict:
    """Two registrations for the same (method, path) in the same router."""
    router_fn: str
    method: str
    path: str
    handlers: list[str]
    source_file: str


@dataclass
class InventoryResult:
    registrations: list[RouteRegistration] = field(default_factory=list)
    unparsed: list[UnparsedRegistration] = field(default_factory=list)
    conflicts: list[Conflict] = field(default_factory=list)
    aliases: list[RouteRegistration] = field(default_factory=list)
    # Override file entries (handler pairs that share the same handler but
    # are intentionally *not* aliases).
    overrides: dict[str, str] = field(default_factory=dict)

    @property
    def total(self) -> int:
        return len(self.registrations)

    @property
    def canonical(self) -> int:
        return self.total - len(self.aliases)


# ---------------------------------------------------------------------------
# Override fixture
# ---------------------------------------------------------------------------

OVERRIDE_FILE = Path(__file__).parent / "http_route_overrides.json"

def load_overrides() -> dict[str, str]:
    """Load reviewed handler-pair overrides that are NOT aliases."""
    if OVERRIDE_FILE.exists():
        with open(OVERRIDE_FILE) as f:
            return json.load(f)
    return {}


# ---------------------------------------------------------------------------
# Balanced-expression tokenizer
# ---------------------------------------------------------------------------

def find_matching_paren(text: str, start: int) -> int:
    """Return index of the closing paren matching the opening paren at *start*.

    Uses balanced-expression scanning: tracks nested parens, square brackets,
    curly braces, and string literals.
    """
    depth = 0
    i = start
    in_string: Optional[str] = None  # None | '"' | "'"
    while i < len(text):
        ch = text[i]
        # Handle escape sequences inside strings.
        if in_string and ch == '\\':
            i += 2
            continue
        if in_string:
            if ch == in_string:
                in_string = None
            i += 1
            continue
        if ch in ('"', "'"):
            in_string = ch
            i += 1
            continue
        if ch in ('(', '[', '{'):
            depth += 1
        elif ch in (')', ']', '}'):
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1  # unmatched


# ---------------------------------------------------------------------------
# Method-chain expander
# ---------------------------------------------------------------------------

# Recognized Axum method builder functions.
HTTP_METHODS = {
    'get', 'post', 'put', 'patch', 'delete', 'head', 'options', 'trace', 'any',
}

def extract_method_handler_pairs(method_expr: str) -> list[tuple[str, str]]:
    """Parse a chained method expression like ``get(foo).post(bar)`` into pairs.

    Returns a list of ``(METHOD, handler_symbol)`` tuples.
    """
    pairs: list[tuple[str, str]] = []
    i = 0
    text = method_expr.strip()
    while i < len(text):
        # Skip leading dots and whitespace.
        while i < len(text) and text[i] in (' ', '\t', '\n', '\r', '.'):
            i += 1
        if i >= len(text):
            break

        # Try to match a method name, optionally qualified with a module path
        # like `axum::routing::get(...)`.
        m = re.match(r'(?:[a-z_]+::)*([a-z_]+)\s*\(', text[i:])
        if not m:
            break  # unparsed tail
        method_name = m.group(1)
        paren_start = i + m.start() + len(m.group(0)) - 1  # index of '('
        paren_end = find_matching_paren(text, paren_start)
        if paren_end < 0:
            break  # unmatched paren
        handler_raw = text[paren_start + 1:paren_end].strip()
        # The handler may be a closure or complex expression; keep it verbatim
        # but collapse whitespace for dedup.
        handler = re.sub(r'\s+', ' ', handler_raw)
        if method_name in HTTP_METHODS:
            pairs.append((method_name.upper(), handler))
        i = paren_end + 1

    return pairs


# ---------------------------------------------------------------------------
# Source scanner
# ---------------------------------------------------------------------------

# Matches `.route("...", <method_expr>)` including multiline.
# We rely on balanced-expression matching rather than a single regex for the
# second argument.
ROUTE_CALL_RE = re.compile(r'\.route\s*\(')

def is_in_test_block(lines: list[str], line_idx: int) -> bool:
    """Heuristic: walk backward to see if we're inside a #[cfg(test)] mod."""
    for k in range(line_idx, -1, -1):
        stripped = lines[k].strip()
        if stripped.startswith('#[cfg(test)]'):
            return True
        # If we hit a top-level `pub fn routes` or similar before cfg(test),
        # we're in production code.
        if re.match(r'^pub(\(crate\))?\s+fn\s+\w+', stripped):
            return False
    return False


def find_enclosing_fn(lines: list[str], line_idx: int) -> str:
    """Walk backward to find the nearest `fn <name>` declaration."""
    for k in range(line_idx, -1, -1):
        m = re.match(r'\s*(?:pub(?:\(crate\))?\s+)?fn\s+(\w+)', lines[k])
        if m:
            return m.group(1)
    return "<unknown>"


def scan_file(filepath: Path, base_dir: Path) -> tuple[list[RouteRegistration], list[UnparsedRegistration]]:
    """Scan a single Rust source file for `.route(...)` calls."""
    regs: list[RouteRegistration] = []
    unparsed: list[UnparsedRegistration] = []

    content = filepath.read_text(encoding='utf-8', errors='replace')
    lines = content.splitlines()
    rel_path = str(filepath.relative_to(base_dir))

    for match in ROUTE_CALL_RE.finditer(content):
        offset = match.end()
        line_no = content[:match.start()].count('\n') + 1

        if is_in_test_block(lines, line_no - 1):
            continue

        # Find the opening paren of .route(
        paren_start = match.end() - 1
        paren_end = find_matching_paren(content, paren_start)
        if paren_end < 0:
            unparsed.append(UnparsedRegistration(
                source_file=rel_path, line=line_no,
                raw_text=content[match.start():match.start() + 80].replace('\n', ' '),
                reason="unmatched parenthesis",
            ))
            continue

        inner = content[paren_start + 1:paren_end].strip()

        # Split into path and method expression.
        # The path is the first string literal argument.
        path_match = re.match(r'"([^"]*)"', inner)
        if not path_match:
            unparsed.append(UnparsedRegistration(
                source_file=rel_path, line=line_no,
                raw_text=inner[:80].replace('\n', ' '),
                reason="could not extract path literal",
            ))
            continue

        route_path = path_match.group(1)
        method_expr = inner[path_match.end():].strip()
        # Strip leading comma.
        if method_expr.startswith(','):
            method_expr = method_expr[1:].strip()

        router_fn = find_enclosing_fn(lines, line_no - 1)

        pairs = extract_method_handler_pairs(method_expr)
        if not pairs:
            unparsed.append(UnparsedRegistration(
                source_file=rel_path, line=line_no,
                raw_text=method_expr[:80].replace('\n', ' '),
                reason="no method/handler pairs extracted",
            ))
            continue

        for method, handler in pairs:
            regs.append(RouteRegistration(
                source_file=rel_path,
                line=line_no,
                router_fn=router_fn,
                path=route_path,
                method=method,
                handler=handler,
            ))

    return regs, unparsed


# ---------------------------------------------------------------------------
# Alias and conflict detection
# ---------------------------------------------------------------------------

def classify(registrations: list[RouteRegistration], overrides: dict[str, str]) -> tuple[list[RouteRegistration], list[Conflict]]:
    """Identify aliases (same method+handler exposed on multiple paths) and
    conflicts (same method+path with different handlers in the same router).

    Returns (aliases, conflicts).
    """
    aliases: list[RouteRegistration] = []
    conflicts: list[Conflict] = []

    # Group by (method, handler) to find aliases.
    by_key: dict[tuple[str, str], list[RouteRegistration]] = {}
    for reg in registrations:
        by_key.setdefault(reg.key(), []).append(reg)
    for key, group in by_key.items():
        if len(group) > 1:
            override_key = f"{key[0]}:{key[1]}"
            if override_key in overrides:
                continue  # reviewed non-alias
            # First occurrence is canonical; the rest are aliases.
            for reg in group[1:]:
                aliases.append(reg)

    # Group by (router_fn, source_file, method, path) for conflict detection.
    by_scope: dict[tuple[str, str, str, str], list[RouteRegistration]] = {}
    for reg in registrations:
        scope_key = (reg.source_file, reg.router_fn, reg.method, reg.path)
        by_scope.setdefault(scope_key, []).append(reg)
    for scope_key, group in by_scope.items():
        if len(group) > 1:
            handlers = list({r.handler for r in group})
            if len(handlers) > 1:
                conflicts.append(Conflict(
                    router_fn=scope_key[1],
                    method=scope_key[2],
                    path=scope_key[3],
                    handlers=sorted(handlers),
                    source_file=scope_key[0],
                ))

    return aliases, conflicts


# ---------------------------------------------------------------------------
# Snapshot
# ---------------------------------------------------------------------------

SNAPSHOT_FILE = Path(__file__).parent / "http_route_inventory.snapshot.json"


def build_snapshot(result: InventoryResult) -> dict:
    """Build a JSON-serializable snapshot dict."""
    return {
        "total_registrations": result.total,
        "canonical_registrations": result.canonical,
        "alias_count": len(result.aliases),
        "conflict_count": len(result.conflicts),
        "unparsed_count": len(result.unparsed),
        "registrations": [
            {
                "source_file": r.source_file,
                "line": r.line,
                "router_fn": r.router_fn,
                "path": r.path,
                "method": r.method,
                "handler": r.handler,
            }
            for r in sorted(result.registrations, key=lambda r: (r.source_file, r.line, r.method))
        ],
        "aliases": [
            {
                "source_file": r.source_file,
                "line": r.line,
                "path": r.path,
                "method": r.method,
                "handler": r.handler,
            }
            for r in sorted(result.aliases, key=lambda r: (r.source_file, r.line))
        ],
        "unparsed": [
            {
                "source_file": u.source_file,
                "line": u.line,
                "reason": u.reason,
                "raw_text": u.raw_text,
            }
            for u in sorted(result.unparsed, key=lambda u: (u.source_file, u.line))
        ],
        "conflicts": [
            {
                "router_fn": c.router_fn,
                "method": c.method,
                "path": c.path,
                "handlers": c.handlers,
                "source_file": c.source_file,
            }
            for c in sorted(result.conflicts, key=lambda c: (c.source_file, c.path))
        ],
    }


def write_snapshot(result: InventoryResult) -> Path:
    snapshot = build_snapshot(result)
    with open(SNAPSHOT_FILE, 'w') as f:
        json.dump(snapshot, f, indent=2, sort_keys=False)
        f.write('\n')
    return SNAPSHOT_FILE


def check_snapshot(result: InventoryResult) -> list[str]:
    """Compare current scan against the stored snapshot.

    Returns a list of error messages. Empty list means the check passed.
    """
    errors: list[str] = []

    if result.conflicts:
        for c in result.conflicts:
            errors.append(
                f"conflict: {c.method} {c.path} in {c.source_file}::{c.router_fn} "
                f"has handlers {c.handlers}"
            )

    if result.unparsed:
        for u in result.unparsed:
            errors.append(
                f"unparsed: {u.source_file}:{u.line} — {u.reason}: {u.raw_text}"
            )

    if not SNAPSHOT_FILE.exists():
        errors.append(f"snapshot file not found: {SNAPSHOT_FILE}")
        return errors

    with open(SNAPSHOT_FILE) as f:
        stored = json.load(f)

    current = build_snapshot(result)

    if stored["total_registrations"] != current["total_registrations"]:
        errors.append(
            f"total registration count changed: snapshot={stored['total_registrations']} "
            f"current={current['total_registrations']}; run --refresh to update"
        )

    if stored["canonical_registrations"] != current["canonical_registrations"]:
        errors.append(
            f"canonical registration count changed: snapshot={stored['canonical_registrations']} "
            f"current={current['canonical_registrations']}; run --refresh to update"
        )

    return errors


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def scan_serve_crate(workspace_root: Path) -> InventoryResult:
    """Scan all non-test Rust source under crates/roko-serve/src."""
    serve_src = workspace_root / "crates" / "roko-serve" / "src"
    if not serve_src.is_dir():
        print(f"error: {serve_src} is not a directory", file=sys.stderr)
        sys.exit(1)

    overrides = load_overrides()

    all_regs: list[RouteRegistration] = []
    all_unparsed: list[UnparsedRegistration] = []

    for rs_file in sorted(serve_src.rglob("*.rs")):
        regs, unparsed = scan_file(rs_file, workspace_root)
        all_regs.extend(regs)
        all_unparsed.extend(unparsed)

    aliases, conflicts = classify(all_regs, overrides)

    return InventoryResult(
        registrations=all_regs,
        unparsed=all_unparsed,
        conflicts=conflicts,
        aliases=aliases,
        overrides=overrides,
    )


def text_report(result: InventoryResult) -> str:
    """Human-readable text report."""
    lines = []
    lines.append("=" * 72)
    lines.append("HTTP Route Inventory — roko-serve")
    lines.append("=" * 72)
    lines.append("")
    lines.append(f"Total method+path registrations: {result.total}")
    lines.append(f"Aliases (same method+handler on multiple paths): {len(result.aliases)}")
    lines.append(f"Canonical (non-alias) registrations: {result.canonical}")
    lines.append(f"Unparsed registrations: {len(result.unparsed)}")
    lines.append(f"Conflicts (same method+path, different handler): {len(result.conflicts)}")
    lines.append("")

    # Group by source file.
    by_file: dict[str, list[RouteRegistration]] = {}
    for reg in result.registrations:
        by_file.setdefault(reg.source_file, []).append(reg)
    for src_file in sorted(by_file):
        regs = sorted(by_file[src_file], key=lambda r: r.line)
        lines.append(f"--- {src_file} ({len(regs)} registrations) ---")
        for reg in regs:
            alias_marker = " [alias]" if reg in result.aliases else ""
            lines.append(f"  L{reg.line:>4}  {reg.method:>7} {reg.path}  -> {reg.handler}{alias_marker}")
        lines.append("")

    if result.aliases:
        lines.append("--- Aliases ---")
        for a in result.aliases:
            lines.append(f"  {a.method} {a.path} -> {a.handler} ({a.source_file}:{a.line})")
        lines.append("")

    if result.unparsed:
        lines.append("--- Unparsed ---")
        for u in result.unparsed:
            lines.append(f"  {u.source_file}:{u.line} — {u.reason}: {u.raw_text}")
        lines.append("")

    if result.conflicts:
        lines.append("--- Conflicts ---")
        for c in result.conflicts:
            lines.append(f"  {c.method} {c.path} in {c.source_file}::{c.router_fn}: {c.handlers}")
        lines.append("")

    lines.append("Limitations:")
    lines.append("  - Nested .nest() prefixes are not resolved to effective paths.")
    lines.append("  - Feature-gated routes (#[cfg(...)]) are included as-parsed.")
    lines.append("  - route_service, runtime-composed, and dynamic routes are not analyzed.")
    lines.append("  - Duplicate detection is per-parsed local router scope only.")
    lines.append("")

    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description="Deterministic HTTP route inventory for roko-serve")
    parser.add_argument("--json", action="store_true", help="JSON output to stdout")
    parser.add_argument("--refresh", action="store_true", help="Write snapshot file")
    parser.add_argument("--check-snapshot", action="store_true", help="CI gate: fail on drift/conflicts/unparsed")
    parser.add_argument("--workspace", type=Path, default=None,
                        help="Workspace root (default: auto-detect from script location)")
    args = parser.parse_args()

    if args.workspace:
        workspace_root = args.workspace
    else:
        # Auto-detect: this script lives in tools/ under workspace root.
        workspace_root = Path(__file__).resolve().parent.parent

    result = scan_serve_crate(workspace_root)

    if args.check_snapshot:
        errors = check_snapshot(result)
        if errors:
            print("FAIL: route inventory check failed:", file=sys.stderr)
            for e in errors:
                print(f"  - {e}", file=sys.stderr)
            sys.exit(1)
        else:
            print(f"OK: {result.total} registrations ({result.canonical} canonical), snapshot matches")
            sys.exit(0)

    if args.refresh:
        path = write_snapshot(result)
        print(f"Snapshot written to {path}")
        print(f"  Total: {result.total}, Canonical: {result.canonical}, Aliases: {len(result.aliases)}")
        return

    if args.json:
        json.dump(build_snapshot(result), sys.stdout, indent=2)
        print()
    else:
        print(text_report(result))


if __name__ == "__main__":
    main()
