import importlib.util
from pathlib import Path
import unittest
import tempfile
import json
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("audit_book_content", Path(__file__).parents[2] / "scripts/audit-book-content.py")
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)

PROSE = " ".join(f"word{i}" for i in range(40))


def para(text=PROSE):
    return {"t": "paragraph", "content": [{"t": "text", "v": text}]}


def heading(title="Rule", anchor="rule"):
    return {"t": "heading", "level": 2, "anchor": anchor, "content": [{"t": "text", "v": title}]}


def entry(id, title, body, source="book-a"):
    return {"id": id, "title": title, "kind": "rule", "source_ids": [source], "body": body}


class PassageAuditTests(unittest.TestCase):
    def test_parser_confines_opening_to_first_article_section(self):
        parser = audit.ArticleParser()
        parser.feed('<h1>Navigation</h1><span id="MainContent_DetailedOutput"><a>Breadcrumb</a>'
                    '<h1><a>Rule</a></h1><b>Source</b> <a>Book A pg. 42</a><br>'
                    'Opening <em>prose</em> stays together.<br><br>Second paragraph.'
                    '<h2>Next</h2><b>Source</b> Book A pg. 43<br>Other content.</span><p>Footer</p>')
        self.assertEqual(parser.result(), {"title": "Rule", "source": "Book A pg. 42",
                                         "paragraph": "Opening prose stays together.", "section_paragraph_count": 2})

    def test_root_and_leading_heading_are_one_position(self):
        idx = audit.build_index([entry("a/rule", "Rule", [heading(), para()])])
        selected, candidates, _ = audit.match_passage({"title": "Rule", "paragraph": PROSE}, "book-a", idx)
        self.assertEqual(selected, {"entry_id": "a/rule", "anchor": None})
        self.assertEqual(len(candidates), 1)

    def test_dedicated_article_beats_intro_copy_with_same_source_and_passage(self):
        idx = audit.build_index([
            entry("a/introduction", "Introduction", [para("Different introduction."), heading(), para()]),
            entry("a/rule", "Rule", [heading(), para()]),
        ])
        selected, candidates, reason = audit.match_passage({"title": "Rule", "paragraph": PROSE}, "book-a", idx)
        self.assertEqual(selected["entry_id"], "a/rule")
        self.assertEqual(len(candidates), 2)
        self.assertEqual(reason, "exact_source_opening_passage_and_dedicated_article_title")

    def test_cross_book_passage_cannot_supply_a_target(self):
        idx = audit.build_index([entry("b/rule", "Rule", [para()], source="book-b")])
        selected, candidates, reason = audit.match_passage({"title": "Rule", "paragraph": PROSE}, "book-a", idx)
        self.assertIsNone(selected)
        self.assertEqual(candidates, [])
        self.assertEqual(reason, "opening_passage_not_matched")

    def test_repeated_passage_without_unique_dedicated_article_stays_ambiguous(self):
        idx = audit.build_index([entry("a/one", "One", [heading(), para()]), entry("a/two", "Two", [heading(), para()])])
        selected, candidates, reason = audit.match_passage({"title": "Rule", "paragraph": PROSE}, "book-a", idx)
        self.assertIsNone(selected)
        self.assertEqual(len(candidates), 2)
        self.assertEqual(reason, "ambiguous_opening_passage")

    def test_short_prose_cannot_become_weak_match(self):
        selected, _, reason = audit.match_passage({"title": "Rule", "paragraph": "A short generic sentence."}, "book-a", {})
        self.assertIsNone(selected)
        self.assertEqual(reason, "no_opening_prose_or_too_short")

    def test_only_reviewed_source_aliases_are_accepted(self):
        catalog = {"sources": [{"id": "book-a", "title": "Game Mastery Guide"}]}
        labels = audit.expected_source_labels(catalog, {"aliases": [{"alias": "GameMastery Guide", "canonical_book_id": "book-a", "evidence": "Reviewed spelling"}]})
        self.assertIn(audit.source_label("GameMastery Guide pg. 42"), labels["book-a"])
        with self.assertRaises(ValueError):
            audit.expected_source_labels(catalog, {"aliases": [{"alias": "Other", "canonical_book_id": "book-b", "evidence": ""}]})

    def test_existing_unheaded_paragraph_maps_to_its_whole_entry(self):
        idx = audit.build_index([entry("a/words", "Words of Power", [para("Different introduction."), para(), para("Closing text.")])])
        selected, _, _ = audit.match_passage({"title": "Optional", "paragraph": PROSE}, "book-a", idx)
        self.assertEqual(selected, {"entry_id": "a/words", "anchor": None})

    def test_existing_paragraph_uses_its_nearest_heading_context(self):
        idx = audit.build_index([entry("a/article", "Article", [para("Introduction."), heading("Section", "section"), para("Section introduction."), para()])])
        selected, _, _ = audit.match_passage({"title": "Alias", "paragraph": PROSE}, "book-a", idx)
        self.assertEqual(selected, {"entry_id": "a/article", "anchor": "section"})

    def test_chapter_context_requires_unique_named_candidate_and_shared_headings(self):
        chapter = {"title": "Magic", "entry": "a/magic", "children": [
            {"title": title, "children": []} for title in ["Casting Spells", "Spell Descriptions", "Arcane Spells"]]}
        books = {"books": [{"source": "book-a", "chapters": [chapter]}]}
        article = {"headings": ["Magic", "Casting Spells", "Spell Descriptions", "Arcane Spells"]}
        row = {"label": "Magic", "candidate_entry_ids": ["a/magic"]}
        target = audit.chapter_target(article, "book-a", row, books, {})
        self.assertEqual(target[0], {"entry_id": "a/magic", "anchor": None})
        self.assertEqual(len(target[2]), 3)
        self.assertIsNone(audit.chapter_target(article, "book-b", row, books, {}))
        self.assertIsNone(audit.chapter_target({"headings": article["headings"][:3]}, "book-a", row, books, {}))
        books["books"][0]["chapters"].append(chapter)
        self.assertIsNone(audit.chapter_target(article, "book-a", row, books, {}))

    def test_supplemental_candidates_require_source_and_full_token_boundaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "json").mkdir()
            description = lambda text: {"name": "Rule", "sources": ["Book A"], "description": [text]}
            payload = {"correct": description(PROSE), "prefix_only": description(PROSE.replace("word23", "word230")),
                       "other_book": {"name": "Rule", "sources": ["Book B"], "description": [PROSE]}}
            (root / "json" / "rules.json").write_text(json.dumps(payload))
            url = "https://aonprd.com/Rules.aspx?ID=1"
            (root / "rules-1.html").write_text('<span id="MainContent_DetailedOutput"><h1>Rule</h1><b>Source</b> Book A pg. 1<br>' + PROSE + '</span>')
            rows = [{"source": "book-a", "url": url, "label": "Rule", "status": "opening_passage_not_matched"}]
            with patch.object(audit.subprocess, "check_output", return_value="08ea71f24375908d0190c822fd1fd080797667a9"):
                result = audit.supplemental_records(root, rows, root, {"book-a": {"book a"}})
            self.assertEqual([item["json_pointer"] for item in result["rows"][0]["candidates"]], ["/correct"])
            with patch.object(audit.subprocess, "check_output", return_value="wrong-pin"), self.assertRaisesRegex(ValueError, "must be pinned"):
                audit.supplemental_records(root, rows, root, {"book-a": {"book a"}})


if __name__ == "__main__":
    unittest.main()
