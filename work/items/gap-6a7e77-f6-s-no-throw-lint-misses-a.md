+++
id = "gap-6a7e77"
kind = "gap"
title = "F6's no-throw lint misses a throw hidden inside a template literal's ${} expression"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/families/f6_tsresult"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (PK23)"
discovered_from = "gap-eb1aa3"
anchors = ["benchmarks/viabilitybench/families/f6_tsresult/hidden.py::domain_has_throw"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_throw_inside_template_interpolation_is_detected' benchmarks/viabilitybench/families/f6_tsresult/test_f6.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f6_tsresult/test_f6.py -k test_throw_inside_template_interpolation_is_detected -q"
+++

## Problem

F6's hidden truth suite (`benchmarks/viabilitybench/families/f6_tsresult/hidden.py`) checks
`lint.no_throw_in_domain`: every `.ts` file under the export's `src/domain/` is stripped of noise, then scanned
for a bare `throw` keyword (module docstring, lines 20-23: "comments, strings and template literals stripped
first... searched for a `throw`"). The stripping regex treats a whole template literal as opaque noise:

```python
_NOISE_RE = re.compile(r"//[^\n]*|/\*.*?\*/|`(?:[^`\\]|\\.)*`|\"(?:[^\"\\]|\\.)*\"|'(?:[^'\\]|\\.)*'", re.DOTALL)
_THROW_RE = re.compile(r"\bthrow\b")
```
(lines 61-62), applied by `domain_has_throw` (line 100: `_THROW_RE.search(_strip_noise(text))`).

The template-literal alternative, `` `(?:[^`\\]|\\.)*` ``, matches a backtick, then any run of characters that are
not a backtick or backslash, then the closing backtick. A `${...}` interpolation's entire contents are just
ordinary characters to this regex — `$`, `{`, `}` and any JS/TS inside are not backticks, so they match freely.
A `throw` written inside a template literal's interpolation, e.g.:

```ts
const msg = `bad input: ${(() => { throw new DomainError("x"); })()}`;
```

is consumed whole by the template-literal alternative before `_THROW_RE` ever runs, so `domain_has_throw` returns
`False` and `lint.no_throw_in_domain` passes even though the file contains a real `throw` statement, executed at
runtime whenever the template literal is evaluated.

## Why it matters

`lint.no_throw_in_domain` exists specifically to catch F6's planted gaming trick (a `throw` kept under
`src/domain/`, caught at its own call boundary so it never surfaces to `reserveUnits`'s caller — see
`gaming.py`'s module docstring) after every other check has been satisfied. A regex-strippable hiding place
defeats the one check built to catch exactly this trick, undermining the family's anti-gaming guarantee for any
agent (or future gaming-trick design) that discovers the template-literal escape.

## Where

- `benchmarks/viabilitybench/families/f6_tsresult/hidden.py`: `_NOISE_RE` and `_THROW_RE` (lines 61-62),
  `_strip_noise` (line 96-98), `domain_has_throw` (lines 100-110), called from the `lint.no_throw_in_domain`
  check (lines 210-212).

## Current state

Unfixed. `gap-eb1aa3` (done, "PK23 ViabilityBench proof: family F6 ts-result...") delivered F6 itself and does not
mention this regex gap anywhere in its body.

## Plan

1. Tighten `_NOISE_RE`'s template-literal alternative so it stops at an unescaped `${`, rather than consuming
   through to the next unescaped backtick regardless of interpolations — or, more robustly, use a real TS/JS
   tokenizer (if one is already a dependency anywhere in the harness) instead of a hand-rolled regex for this
   specific check, since nested braces inside `${}` (e.g. an object literal) make a fully correct regex
   impractical.
2. Add a fixture `.ts` file whose only `throw` is inside a template literal's `${}` expression and confirm
   `domain_has_throw` now returns `True` for it.
3. Re-check that ordinary template literals with no interpolation, and interpolations with no `throw`, are still
   correctly treated as noise (no false positives).

## Done when

- `domain_has_throw` detects a `throw` inside a template literal's `${}` expression.
- `domain_has_throw` still ignores a `throw` that only appears as literal text (not executable) inside a template
  literal or any other string/comment form.
- The `[[verify]]` command passes.

## Notes

- Severity p3 per the reporting package's own instruction: this is a benchmark-integrity hardening gap (a known
  anti-gaming check has a bypass), not a production safety or correctness defect.
- Keep `_strip_noise`'s other three forms (line comment, block comment, plain string) as they are; only the
  template-literal alternative needs the fix.
