#!/usr/bin/env python3
"""Run one unittest suite inside a census tree and write per-test results as JSON (the plan slice's test runner).

`slicekit.py` starts it as `python -I runner.py --start DIR --json OUT` with the census tree as the working directory.
`-I` keeps the tree's own files off `sys.path` until `unittest` is imported, so a planted `unittest.py` or
`sitecustomize.py` cannot replace the runner's machinery. Only then is the tree put on `sys.path`, so the suite can
import the feature's package. The runner imports nothing from the benchmark, since agent code runs in its process.

Output: {"ran": n, "tests": [{"id", "status": pass|fail|error|skip, "reqs": [...], "detail"}]}. A test module that
fails to import shows up as an `error` row, so a missing module fails the suite rather than shrinking it. `reqs` are
the requirement ids that start the test's docstring ("R1, R4: ...").
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import unittest

TAG_RE = re.compile(r"^\s*(R\d+(?:\s*,\s*R\d+)*)\s*:")
DETAIL_CHARS = 2000


class Recorder(unittest.TestResult):
    def __init__(self) -> None:
        super().__init__()
        self.rows: list[dict] = []

    def _row(self, test: unittest.TestCase, status: str, detail: str = "") -> None:
        doc = getattr(getattr(test, "test_case", test), "_testMethodDoc", None) or ""  # a subtest's parent's doc
        tags = TAG_RE.match(doc)
        reqs = [tag.strip() for tag in tags[1].split(",")] if tags else []
        self.rows.append({"id": test.id(), "status": status, "reqs": reqs, "detail": detail[-DETAIL_CHARS:]})

    def addSuccess(self, test):
        super().addSuccess(test)
        self._row(test, "pass")

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self._row(test, "fail", self._exc_info_to_string(err, test))

    def addError(self, test, err):
        super().addError(test, err)
        self._row(test, "error", self._exc_info_to_string(err, test))

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self._row(test, "skip", reason)

    def addExpectedFailure(self, test, err):
        super().addExpectedFailure(test, err)
        self._row(test, "fail", "expected failure: the suite must not mark tests as expected to fail")

    def addUnexpectedSuccess(self, test):
        super().addUnexpectedSuccess(test)
        self._row(test, "fail", "unexpected success")

    def addSubTest(self, test, subtest, err):
        super().addSubTest(test, subtest, err)
        if err is not None:
            status = "fail" if issubclass(err[0], test.failureException) else "error"
            self._row(subtest, status, self._exc_info_to_string(err, test))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--start", required=True, help="the suite's directory, relative to the working directory")
    parser.add_argument("--json", required=True, help="where to write the results")
    args = parser.parse_args(argv)
    sys.path.insert(0, os.getcwd())
    suite = unittest.TestLoader().discover(start_dir=args.start, top_level_dir=os.getcwd())
    result = Recorder()
    suite.run(result)
    with open(args.json, "w", encoding="utf-8") as handle:
        json.dump({"ran": result.testsRun, "tests": result.rows}, handle, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
