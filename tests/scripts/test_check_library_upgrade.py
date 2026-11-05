import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_library_upgrade", Path(__file__).parents[2] / "scripts" / "check-library-upgrade.py"
)
upgrade = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(upgrade)


def entry(entry_id, title, kind="rule", **fields):
    return {
        "id": entry_id,
        "title": title,
        "kind": kind,
        "fields": fields,
        "source_ids": ["book-a"],
        "body": [],
    }


def write_library(
    root,
    entries,
    *,
    version="v1",
    schema=3,
    aliases=None,
    relations=None,
    spells=None,
    books=None,
    unplaced=None,
):
    root = Path(root)
    directory = root / version
    directory.mkdir(parents=True)
    (directory / "entries").mkdir()
    manifest = {
        "schema_version": schema,
        "content_version": version,
        "entry_count": len(entries),
    }
    (root / "manifest.json").write_text(json.dumps(manifest))
    (directory / "entries" / "entries-000.json").write_text(json.dumps(entries))
    if aliases is not None:
        (directory / "aliases.json").write_text(json.dumps(aliases))
    (directory / "relations.json").write_text(
        json.dumps({"edges": relations or [], "counts": {"part": len(relations or [])}})
    )
    (directory / "lists.json").write_text(json.dumps({"spells": spells or []}))
    if books is not None:
        (directory / "books.json").write_text(json.dumps({"books": books}))
    if unplaced is not None:
        (directory / "coverage.json").write_text(
            json.dumps(
                {
                    "relations": {
                        "book_order": {
                            "placed": len(entries) - len(unplaced),
                            "unplaced": unplaced,
                        }
                    }
                }
            )
        )
    return root


def aliases(entries=(), retired=()):
    return {
        "format": 1,
        "legacy": {"count": len(entries) + len(retired), "sha256": "0" * 64},
        "entries": [dict(row) for row in entries],
        "retired": [dict(row) for row in retired],
    }


class UpgradeCheckTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.old = self.root / "old"
        self.new = self.root / "new"

    def tearDown(self):
        self.tmp.cleanup()

    def run_check(self):
        return upgrade.check(self.old, self.new)

    def test_no_alias_map_keeps_the_exact_title_and_id_check(self):
        write_library(self.old, [entry("a/one", "One"), entry("a/two", "Two")])
        write_library(self.new, [entry("a/one", "One!"), entry("a/three", "Three")])

        report, failures = self.run_check()

        self.assertNotIn("alias_map", report)
        self.assertEqual(report["missing_ids"], ["a/two"])
        self.assertEqual(report["changed_titles"], ["a/one"])
        self.assertTrue(failures)

    def test_alias_cutover_resolves_one_step_and_counts_retirements(self):
        write_library(
            self.old,
            [
                entry("legacy/alpha", "Dragon, Red", "creature"),
                entry("legacy/beta", "Gone", "rule"),
            ],
        )
        write_library(
            self.new,
            [entry("ref/monster/red-dragon", "Red Dragon", "creature")],
            aliases=aliases(
                entries=[
                    {
                        "old": "legacy/alpha",
                        "target": "ref/monster/red-dragon",
                        "basis": "direct",
                    }
                ],
                retired=[
                    {
                        "old": "legacy/beta",
                        "reason": "no_source",
                        "note": "no counterpart",
                    }
                ],
            ),
            books=[
                {
                    "source": "book-a",
                    "chapters": [
                        {"title": "Red Dragon", "entry": "ref/monster/red-dragon"}
                    ],
                }
            ],
            unplaced=[],
        )

        report, failures = self.run_check()

        self.assertEqual(failures, [])
        self.assertTrue(report["alias_map"])
        self.assertEqual(report["alias_bases"], {"direct": 1})
        self.assertEqual(report["retired_by_reason"], {"no_source": 1})
        self.assertEqual(report["changed_titles"], [])

    def test_missing_and_unresolvable_ids_both_fail(self):
        write_library(
            self.old,
            [entry("legacy/a", "A"), entry("legacy/b", "B"), entry("legacy/c", "C")],
        )
        write_library(
            self.new,
            [entry("ref/rule/b", "B")],
            aliases=aliases(
                entries=[
                    {"old": "legacy/a", "target": "ref/rule/missing", "basis": "direct"},
                    {"old": "legacy/b", "target": "ref/rule/b", "basis": "direct"},
                ]
            ),
            unplaced=[],
        )

        report, failures = self.run_check()

        self.assertEqual(report["missing_ids"], ["legacy/c"])
        self.assertEqual(
            report["unresolvable_ids"], [{"old": "legacy/a", "target": "ref/rule/missing"}]
        )
        self.assertTrue(any("unresolvable" in failure for failure in failures))
        self.assertTrue(any("missing" in failure for failure in failures))

    def test_ids_that_were_never_published_are_reported_not_failed(self):
        write_library(
            self.old,
            [entry("legacy/b", "B"), entry("legacy/never-shipped", "Never Shipped")],
        )
        write_library(
            self.new,
            [entry("ref/rule/b", "B")],
            aliases=aliases(
                entries=[{"old": "legacy/b", "target": "ref/rule/b", "basis": "direct"}]
            ),
            unplaced=[],
        )

        report, failures = upgrade.check(self.old, self.new, published={"legacy/b"})

        self.assertEqual(report["missing_ids"], [])
        self.assertEqual(report["unpublished_old_ids"], ["legacy/never-shipped"])
        self.assertFalse(any("missing" in failure for failure in failures), failures)
        # Without the published set the same id is a missing, failing id.
        report, failures = self.run_check()
        self.assertEqual(report["missing_ids"], ["legacy/never-shipped"])
        self.assertTrue(any("missing" in failure for failure in failures))

    def test_a_chain_and_a_double_disposition_fail(self):
        write_library(self.old, [entry("legacy/a", "A"), entry("legacy/b", "B")])
        write_library(
            self.new,
            [entry("ref/rule/b", "B")],
            aliases=aliases(
                entries=[
                    {"old": "legacy/a", "target": "legacy/b", "basis": "direct"},
                    {"old": "legacy/b", "target": "ref/rule/b", "basis": "direct"},
                ],
                retired=[{"old": "legacy/a", "reason": "no_source"}],
            ),
            unplaced=[],
        )

        report, failures = self.run_check()

        joined = "\n".join(report["alias_problems"])
        self.assertIn("appears in both entries and retired", joined)
        self.assertIn("chains are forbidden", joined)
        self.assertTrue(any("alias map" in failure for failure in failures))

    def test_title_fit_accepts_documented_spellings_and_reports_shared_changes(self):
        write_library(
            self.old,
            [
                entry("old/aeon", "Aeon, Akhana", "creature"),
                entry("old/chain", "Chain (10 ft.)", "item"),
                entry("old/shared", "Alpha", "rule"),
                entry("old/direct", "Gamma", "rule"),
            ],
        )
        write_library(
            self.new,
            [
                entry("ref/monster/akhana", "Akhana (Aeon)", "creature"),
                entry("ref/item/chain", "Chain", "item"),
                entry("ref/rule/beta", "Beta", "rule"),
                entry("ref/rule/delta", "Delta", "rule"),
            ],
            aliases=aliases(
                entries=[
                    {"old": "old/aeon", "target": "ref/monster/akhana", "basis": "direct"},
                    {"old": "old/chain", "target": "ref/item/chain", "basis": "direct"},
                    {"old": "old/shared", "target": "ref/rule/beta", "basis": "shared"},
                    {"old": "old/direct", "target": "ref/rule/delta", "basis": "direct"},
                ]
            ),
            books=[
                {
                    "source": "book-a",
                    "chapters": [
                        {
                            "title": "Entries",
                            "children": [
                                {"title": "Akhana", "entry": "ref/monster/akhana"},
                                {"title": "Chain", "entry": "ref/item/chain"},
                                {"title": "Beta", "entry": "ref/rule/beta"},
                                {"title": "Delta", "entry": "ref/rule/delta"},
                            ],
                        }
                    ],
                }
            ],
            unplaced=[],
        )

        report, failures = self.run_check()

        self.assertEqual(len(report["changed_titles"]), 1)
        self.assertEqual(report["changed_titles"][0]["old"], "old/direct")
        self.assertEqual(
            [row["old"] for row in report["reported_title_changes"]], ["old/shared"]
        )
        self.assertTrue(any("unexpected title change" in failure for failure in failures))

    def test_item_parenthetical_variants_are_not_title_fits(self):
        self.assertIsNotNone(upgrade.title_fit("Chain (10 ft.)", "Chain", "item"))
        self.assertIsNone(upgrade.title_fit("Lock (average)", "Lock", "item"))
        self.assertIsNone(upgrade.title_fit("Tent (large)", "Tent", "item"))

    def test_retired_ids_are_counted_by_reason_not_reported_missing(self):
        write_library(self.old, [entry("legacy/rights", "Removed")])
        write_library(
            self.new,
            [],
            aliases=aliases(
                retired=[{"old": "legacy/rights", "reason": "rights", "note": "PI"}]
            ),
            unplaced=[],
        )

        report, failures = self.run_check()

        self.assertEqual(failures, [])
        self.assertEqual(report["retired_by_reason"], {"rights": 1})
        self.assertEqual(report["missing_ids"], [])

    def test_books_may_repeat_an_entry_and_unplaced_entries_are_accounted(self):
        write_library(
            self.old,
            [entry("legacy/a", "A"), entry("legacy/b", "B"), entry("legacy/c", "C")],
        )
        write_library(
            self.new,
            [entry("ref/a", "A"), entry("ref/b", "B"), entry("ref/c", "C")],
            aliases=aliases(
                entries=[
                    {"old": "legacy/a", "target": "ref/a", "basis": "direct"},
                    {"old": "legacy/b", "target": "ref/b", "basis": "direct"},
                    {"old": "legacy/c", "target": "ref/c", "basis": "direct"},
                ]
            ),
            books=[
                {
                    "source": "book-a",
                    "chapters": [
                        {
                            "title": "Page order",
                            "children": [
                                {"title": "A", "entry": "ref/a"},
                                {"title": "B", "entry": "ref/b"},
                            ],
                        }
                    ],
                },
                {
                    "source": "book-b",
                    "chapters": [
                        {"title": "Page order", "children": [{"title": "A", "entry": "ref/a"}]}
                    ],
                },
            ],
            unplaced=[{"entry": "ref/c", "reason": "no page number"}],
        )

        report, failures = self.run_check()

        self.assertEqual(failures, [])
        self.assertEqual(report["reported_duplicate_book_placements"], 1)

    def test_dangling_book_relation_and_list_references_fail(self):
        write_library(self.old, [entry("legacy/a", "A")])
        write_library(
            self.new,
            [entry("ref/a", "A")],
            aliases=aliases(
                entries=[{"old": "legacy/a", "target": "ref/a", "basis": "direct"}]
            ),
            relations=[["ref/a", "part", "ref/gone"]],
            spells=[["ref/spell/missing", "Missing Spell"]],
            books=[
                {
                    "source": "book-a",
                    "chapters": [{"title": "Gone", "entry": "ref/missing"}],
                }
            ],
            unplaced=[],
        )

        report, failures = self.run_check()

        joined = "\n".join(report["dangling_references"])
        self.assertIn("relations.json: ref/gone", joined)
        self.assertIn("lists.json: ref/spell/missing", joined)
        self.assertIn("books.json: ref/missing", joined)
        self.assertTrue(any("dangling reference" in failure for failure in failures))

    def test_repeated_runs_produce_the_same_report(self):
        write_library(self.old, [entry("legacy/a", "A")])
        write_library(
            self.new,
            [entry("ref/a", "A")],
            aliases=aliases(
                entries=[{"old": "legacy/a", "target": "ref/a", "basis": "direct"}]
            ),
            unplaced=[],
        )

        first = self.run_check()
        second = self.run_check()

        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))


if __name__ == "__main__":
    unittest.main()
