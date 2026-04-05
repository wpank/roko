"""Unit tests for the pure functions in swebench_run.py."""
import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from swebench_run import (  # type: ignore
    oracle_files, extract_patch, find_latest_agent_output, build_config,
)


def test_oracle_files_parses_single_file():
    patch = """diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-old
+new
"""
    assert oracle_files(patch) == ["src/lib.rs"]


def test_oracle_files_parses_multiple_files():
    patch = """diff --git a/foo.py b/foo.py
@@ -1 +1 @@
-x
+y
diff --git a/bar/baz.py b/bar/baz.py
@@ -1 +1 @@
-a
+b
"""
    assert oracle_files(patch) == ["foo.py", "bar/baz.py"]


def test_oracle_files_empty_patch():
    assert oracle_files("") == []
    assert oracle_files("not a diff") == []


def test_extract_patch_from_diff_fence():
    text = """Here's the fix:

```diff
diff --git a/foo.py b/foo.py
--- a/foo.py
+++ b/foo.py
@@ -1 +1 @@
-broken
+fixed
```

Done."""
    patch = extract_patch(text)
    assert patch.startswith("diff --git a/foo.py")
    assert "+fixed" in patch
    assert "Done." not in patch


def test_extract_patch_from_patch_fence():
    text = """```patch
diff --git a/x.py b/x.py
@@ -1 +1 @@
-a
+b
```"""
    assert extract_patch(text).startswith("diff --git a/x.py")


def test_extract_patch_unfenced():
    # Model forgot the fence — fall back to `diff --git` prefix.
    text = """I'll fix it by editing foo.py.
diff --git a/foo.py b/foo.py
--- a/foo.py
+++ b/foo.py
@@ -1 +1 @@
-x
+y
"""
    patch = extract_patch(text)
    assert patch.startswith("diff --git a/foo.py")
    assert "+y" in patch


def test_extract_patch_returns_empty_when_no_diff():
    assert extract_patch("I don't know how to fix this.") == ""
    assert extract_patch("") == ""


def test_extract_patch_prefers_diff_fence_over_raw():
    text = """explanation: diff --git is a git command.
```diff
diff --git a/real.py b/real.py
@@ -1 +1 @@
-x
+y
```"""
    # Should pull from the fenced block, not the "diff --git" word in prose.
    patch = extract_patch(text)
    assert patch.startswith("diff --git a/real.py")


def test_find_latest_agent_output_prefers_cleaned():
    with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
        # Raw agent_output, older
        json.dump({
            "kind": "agent_output", "created_at_ms": 100,
            "body": {"format": "text", "data": "raw text"},
            "tags": {},
        }, f); f.write("\n")
        # Cleaned agent_output, same time
        json.dump({
            "kind": "agent_output", "created_at_ms": 100,
            "body": {"format": "text", "data": "cleaned text"},
            "tags": {"cleaned": "true"},
        }, f); f.write("\n")
        # Prompt signal (should be ignored)
        json.dump({
            "kind": "prompt", "created_at_ms": 200,
            "body": {"format": "text", "data": "the prompt"},
            "tags": {},
        }, f); f.write("\n")
        path = Path(f.name)
    assert find_latest_agent_output(path) == "cleaned text"
    path.unlink()


def test_find_latest_agent_output_picks_latest_by_timestamp():
    with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
        json.dump({
            "kind": "agent_output", "created_at_ms": 100,
            "body": {"format": "text", "data": "old"},
            "tags": {"cleaned": "true"},
        }, f); f.write("\n")
        json.dump({
            "kind": "agent_output", "created_at_ms": 200,
            "body": {"format": "text", "data": "new"},
            "tags": {"cleaned": "true"},
        }, f); f.write("\n")
        path = Path(f.name)
    assert find_latest_agent_output(path) == "new"
    path.unlink()


def test_build_config_includes_all_files():
    toml = build_config(
        role="test role",
        model_cmd="ollama",
        model_args=["run", "llama3.2:latest"],
        files=["a.py", "dir/b.py"],
        token_budget=10000,
        timeout_ms=60000,
        file_hard_cap=2000,
    )
    assert 'command = "ollama"' in toml
    assert 'args = ["run", "llama3.2:latest"]' in toml
    assert 'path = "a.py"' in toml
    assert 'path = "dir/b.py"' in toml
    assert "hard_cap = 2000" in toml
    assert "token_budget = 10000" in toml


def test_build_config_escapes_role_quotes():
    # Role containing quotes must be escaped in the TOML output.
    toml = build_config(
        role='a "quoted" role',
        model_cmd="cat", model_args=[], files=[],
        token_budget=100, timeout_ms=100, file_hard_cap=100,
    )
    assert r'a \"quoted\" role' in toml


def test_build_config_no_clean_output():
    toml = build_config(
        role="r", model_cmd="cat", model_args=[], files=["a.py"],
        token_budget=100, timeout_ms=100, file_hard_cap=100,
        clean_output=False,
    )
    assert "clean_output = false" in toml


def test_build_config_no_file_injection():
    toml = build_config(
        role="r", model_cmd="cat", model_args=[], files=["a.py", "b.py"],
        token_budget=100, timeout_ms=100, file_hard_cap=100,
        inject_files=False,
    )
    assert "files = []" in toml
    assert 'path = "a.py"' not in toml


def test_build_config_no_hard_cap():
    toml = build_config(
        role="r", model_cmd="cat", model_args=[], files=["a.py"],
        token_budget=100, timeout_ms=100, file_hard_cap=100,
        use_hard_cap=False,
    )
    assert 'path = "a.py"' in toml
    assert "hard_cap" not in toml


def test_build_config_all_ablations_together():
    toml = build_config(
        role="r", model_cmd="cat", model_args=[], files=["a.py"],
        token_budget=100, timeout_ms=100, file_hard_cap=100,
        clean_output=False, inject_files=False, use_hard_cap=False,
    )
    assert "clean_output = false" in toml
    assert "files = []" in toml
    assert "hard_cap" not in toml


if __name__ == "__main__":
    # Tiny test runner so we don't require pytest.
    import inspect
    mod = sys.modules[__name__]
    tests = [
        (name, fn) for name, fn in inspect.getmembers(mod, inspect.isfunction)
        if name.startswith("test_")
    ]
    failures = 0
    for name, fn in tests:
        try:
            fn()
            print(f"  ok  {name}")
        except AssertionError as e:
            failures += 1
            print(f"  FAIL {name}: {e}")
    print(f"\n{len(tests) - failures}/{len(tests)} tests passed")
    sys.exit(1 if failures else 0)
