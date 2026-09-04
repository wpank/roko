#!/usr/bin/env python3
"""Tests for the http_route_inventory.py parser.

Covers the acceptance criteria from backlog #315:
  - one method
  - chained methods on one path
  - multiline registration
  - exact duplicate in one router scope
  - same local path in distinct nested routers
  - handler-based alias/dedup counts
  - override classification
  - unparsed dynamic route
"""

import json
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

# Import the module under test.
sys.path.insert(0, str(Path(__file__).parent))
import http_route_inventory as inv


class TestBalancedParen(unittest.TestCase):
    def test_simple(self):
        text = "(hello)"
        self.assertEqual(inv.find_matching_paren(text, 0), 6)

    def test_nested(self):
        text = "(foo(bar))"
        self.assertEqual(inv.find_matching_paren(text, 0), 9)

    def test_string_content(self):
        text = '("pa)ren")'
        self.assertEqual(inv.find_matching_paren(text, 0), 9)

    def test_unmatched(self):
        text = "(oops"
        self.assertEqual(inv.find_matching_paren(text, 0), -1)


class TestMethodHandlerPairs(unittest.TestCase):
    def test_single_method(self):
        pairs = inv.extract_method_handler_pairs("get(list_items)")
        self.assertEqual(pairs, [("GET", "list_items")])

    def test_chained_methods(self):
        pairs = inv.extract_method_handler_pairs("get(list_items).post(create_item)")
        self.assertEqual(pairs, [("GET", "list_items"), ("POST", "create_item")])

    def test_qualified_method(self):
        pairs = inv.extract_method_handler_pairs("axum::routing::delete(remove)")
        self.assertEqual(pairs, [("DELETE", "remove")])

    def test_qualified_chained(self):
        pairs = inv.extract_method_handler_pairs(
            "axum::routing::patch(update).delete(remove)"
        )
        self.assertEqual(pairs, [("PATCH", "update"), ("DELETE", "remove")])

    def test_closure_handler(self):
        pairs = inv.extract_method_handler_pairs('get(|| async { "ok" })')
        self.assertEqual(len(pairs), 1)
        self.assertEqual(pairs[0][0], "GET")
        self.assertIn("async", pairs[0][1])

    def test_any_method(self):
        pairs = inv.extract_method_handler_pairs("any(catch_all)")
        self.assertEqual(pairs, [("ANY", "catch_all")])

    def test_whitespace_collapse(self):
        pairs = inv.extract_method_handler_pairs("get( list_items )")
        self.assertEqual(pairs[0][1], "list_items")


class TestScanFile(unittest.TestCase):
    """Test the file scanner against synthetic Rust source files."""

    def _scan_source(self, source: str) -> tuple[list[inv.RouteRegistration], list[inv.UnparsedRegistration]]:
        """Write source to a temp file and scan it."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            rs_file = tmp_path / "test.rs"
            rs_file.write_text(source)
            return inv.scan_file(rs_file, tmp_path)

    def test_single_route(self):
        """Fixture: one method."""
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route("/items", get(list_items))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 1)
        self.assertEqual(regs[0].method, "GET")
        self.assertEqual(regs[0].path, "/items")
        self.assertEqual(regs[0].handler, "list_items")
        self.assertEqual(len(unparsed), 0)

    def test_chained_methods_one_path(self):
        """Fixture: chained methods on one path."""
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route("/items", get(list_items).post(create_item))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 2)
        self.assertEqual(regs[0].method, "GET")
        self.assertEqual(regs[1].method, "POST")
        self.assertEqual(regs[0].path, "/items")
        self.assertEqual(regs[1].path, "/items")

    def test_multiline_registration(self):
        """Fixture: multiline registration."""
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route(
                        "/items/{id}",
                        get(show_item)
                            .put(update_item)
                            .delete(delete_item),
                    )
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 3)
        methods = {r.method for r in regs}
        self.assertEqual(methods, {"GET", "PUT", "DELETE"})
        self.assertEqual(len(unparsed), 0)

    def test_exact_duplicate_in_one_scope(self):
        """Fixture: exact duplicate (method, path) in one router scope.

        Two routes with the same path and method but DIFFERENT handlers in
        the same router function should be detected as a conflict.
        """
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route("/items", get(list_items_v1))
                    .route("/items", get(list_items_v2))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 2)
        aliases, conflicts = inv.classify(regs, {})
        self.assertEqual(len(conflicts), 1)
        self.assertIn("list_items_v1", conflicts[0].handlers)
        self.assertIn("list_items_v2", conflicts[0].handlers)

    def test_same_path_in_distinct_routers(self):
        """Fixture: same local path in distinct nested routers.

        Two different functions can register the same path without conflict
        because they represent different router scopes.
        """
        source = textwrap.dedent('''\
            fn api_routes() -> Router {
                Router::new()
                    .route("/status", get(api_status))
            }

            fn admin_routes() -> Router {
                Router::new()
                    .route("/status", get(admin_status))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 2)
        aliases, conflicts = inv.classify(regs, {})
        # Different handler names -> no alias, different router_fn -> no conflict.
        self.assertEqual(len(aliases), 0)
        self.assertEqual(len(conflicts), 0)

    def test_handler_based_alias_dedup(self):
        """Fixture: handler-based alias detection.

        Same handler on multiple paths is classified as an alias.
        """
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route("/items/list", get(list_items))
                    .route("/items", get(list_items))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 2)
        aliases, conflicts = inv.classify(regs, {})
        self.assertEqual(len(aliases), 1)

    def test_override_classification(self):
        """Fixture: override entries suppress alias classification."""
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route("/bench/run", post(start_run))
                    .route("/bench/runs", post(start_run))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 2)

        # Without override, they are aliases.
        aliases, _ = inv.classify(regs, {})
        self.assertEqual(len(aliases), 1)

        # With override, they are not aliases.
        aliases, _ = inv.classify(regs, {"POST:start_run": "intentional dual path"})
        self.assertEqual(len(aliases), 0)

    def test_test_block_excluded(self):
        """Routes inside #[cfg(test)] blocks are excluded."""
        source = textwrap.dedent('''\
            fn routes() -> Router {
                Router::new()
                    .route("/real", get(real_handler))
            }

            #[cfg(test)]
            mod tests {
                fn test_routes() -> Router {
                    Router::new()
                        .route("/test-only", get(test_handler))
                }
            }
        ''')
        regs, unparsed = self._scan_source(source)
        self.assertEqual(len(regs), 1)
        self.assertEqual(regs[0].path, "/real")

    def test_unparsed_dynamic_route(self):
        """Fixture: unparsed dynamic route.

        A route call with a non-literal path cannot be parsed.
        """
        source = textwrap.dedent('''\
            fn routes() -> Router {
                let path = format!("/dynamic/{}", name);
                Router::new()
                    .route(&path, get(dynamic_handler))
            }
        ''')
        regs, unparsed = self._scan_source(source)
        # The dynamic path uses &path, not a string literal, so it's unparsed.
        self.assertEqual(len(regs), 0)
        self.assertEqual(len(unparsed), 1)
        self.assertIn("could not extract path literal", unparsed[0].reason)


class TestSnapshotRoundTrip(unittest.TestCase):
    def test_snapshot_build(self):
        """Snapshot serializes and can be compared."""
        regs = [
            inv.RouteRegistration("a.rs", 1, "routes", "/x", "GET", "handler_a"),
            inv.RouteRegistration("a.rs", 2, "routes", "/y", "GET", "handler_a"),
        ]
        aliases, conflicts = inv.classify(regs, {})
        result = inv.InventoryResult(
            registrations=regs, unparsed=[], conflicts=conflicts, aliases=aliases,
        )
        snapshot = inv.build_snapshot(result)
        self.assertEqual(snapshot["total_registrations"], 2)
        self.assertEqual(snapshot["canonical_registrations"], 1)
        self.assertEqual(snapshot["alias_count"], 1)

        # Round-trip through JSON.
        text = json.dumps(snapshot)
        restored = json.loads(text)
        self.assertEqual(restored["total_registrations"], snapshot["total_registrations"])


if __name__ == "__main__":
    unittest.main()
