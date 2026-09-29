"""Shared setup for the driver's tests: no test inherits a provider key or a key file from the shell that runs pytest.

`vb run` refuses to start while its environment holds a provider key, and reads keys from `$VB_KEY_FILE`, else
`~/.config/viabilitybench/keys` (bug-979a06). A developer's exported `CEREBRAS_API_KEY`, or their real key file, would
then decide what a test does. So every test starts without the arm files' key variables, and with `VB_KEY_FILE` naming
a file that does not exist. A test that needs keys makes a key file and passes `--key-file`.
"""

from __future__ import annotations

import pytest

import secret


@pytest.fixture(autouse=True)
def no_inherited_provider_keys(monkeypatch, tmp_path_factory):
    for name in secret.arm_key_names():
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv(secret.KEYS_FILE_ENV, str(tmp_path_factory.getbasetemp() / "no-key-file"))
