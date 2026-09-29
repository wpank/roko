"""Where things are: the benchmark tree, the repository, the price snapshots, and the driver's import paths.

The driver is a flat directory of modules, not a package: `vb.py` runs as a script, and the tests import the modules
by name with `driver/` on `sys.path`. Importing this module also puts `families/` (for `common`) and `schema/` (for
`validate`) on `sys.path`, so every driver module that needs them imports `layout` first.
"""

from __future__ import annotations

import sys
from pathlib import Path

DRIVER_DIR = Path(__file__).resolve().parent
VB_ROOT = DRIVER_DIR.parent
REPO_ROOT = VB_ROOT.parents[1]
FAMILIES_DIR = VB_ROOT / "families"
SCHEMA_DIR = VB_ROOT / "schema"
ARMS_DIR = VB_ROOT / "arms"
STREAMS_DIR = VB_ROOT / "streams"
PRICES_DIR = REPO_ROOT / "config" / "prices"

for _path in (FAMILIES_DIR, SCHEMA_DIR):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))


def within(path: Path, root: Path) -> bool:
    """Whether `path` is `root` or lies inside it, with symlinks resolved in both."""
    path, root = Path(path).resolve(), Path(root).resolve()
    return path == root or root in path.parents
