#!/usr/bin/env python3
"""Test publication license decisions and the library reader with synthetic inputs."""
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "publication_license", Path(__file__).with_name("publication-license.py")
)
publication_license = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publication_license)

reader_spec = importlib.util.spec_from_file_location(
    "library_reader", Path(__file__).with_name("library-reader.py")
)
library_reader = importlib.util.module_from_spec(reader_spec)
reader_spec.loader.exec_module(library_reader)

CUP = publication_license.COMMUNITY_USE
OGL = "ogl-1.0a"

# Raw `publication_licenses` rows, as the corpus `legal.toml` carries them.
LICENSE_ROWS = [
    {"publication_id": "blog-a", "license_refs": [CUP]},
    {"publication_id": "blog-b", "license_refs": [CUP]},
    {"publication_id": "book-ogl", "license_refs": [OGL]},
]


class CupPublicationTests(unittest.TestCase):
    def test_only_rows_naming_community_use_count(self):
        self.assertEqual(
            publication_license.cup_publications(LICENSE_ROWS), {"blog-a", "blog-b"}
        )

    def test_no_rows(self):
        self.assertEqual(publication_license.cup_publications([]), set())


class LicenseBasisTests(unittest.TestCase):
    def accepted(self, refs, source_ids):
        cup_sources = publication_license.cup_publications(LICENSE_ROWS)
        return publication_license.has_license_basis(
            {"license_refs": refs, "source_ids": source_ids}, cup_sources
        )

    def test_cup_only_entry_with_all_sources_cup_is_accepted(self):
        self.assertTrue(self.accepted([CUP], ["blog-a", "blog-b"]))

    def test_ogl_entry_is_accepted(self):
        self.assertTrue(self.accepted([OGL, CUP], ["book-a"]))

    def test_no_license_refs_is_rejected(self):
        self.assertFalse(self.accepted([], ["blog-a"]))

    def test_cup_ref_without_source_cup_row_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["book-a"]))

    def test_cup_ref_with_ogl_only_license_row_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["book-ogl"]))

    def test_mixed_cup_and_ogl_default_sources_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["blog-a", "book-a"]))

    def test_cup_ref_without_sources_is_rejected(self):
        self.assertFalse(self.accepted([CUP], []))

    def test_unknown_source_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["unknown"]))


class EntryIssueTests(unittest.TestCase):
    """The screen's diagnostics, through the function the screen calls."""

    KNOWN = {"blog-a", "blog-b", "book-a", "book-ogl"}

    def issues(self, refs, source_ids):
        cup_sources = publication_license.cup_publications(LICENSE_ROWS)
        entry = {"id": "ref/x", "license_refs": refs, "source_ids": source_ids}
        return publication_license.entry_issues(entry, self.KNOWN, cup_sources)

    def test_ogl_entry_with_unknown_source_reports_only_unreviewed_source(self):
        self.assertEqual(self.issues([OGL], ["unknown"]), ["Unreviewed source: ref/x"])

    def test_cup_entry_whose_unknown_source_has_a_cup_row_reports_only_unreviewed_source(self):
        rows = LICENSE_ROWS + [{"publication_id": "unknown", "license_refs": [CUP]}]
        entry = {"id": "ref/x", "license_refs": [CUP], "source_ids": ["unknown"]}
        issues = publication_license.entry_issues(
            entry, self.KNOWN, publication_license.cup_publications(rows)
        )
        self.assertEqual(issues, ["Unreviewed source: ref/x"])

    def test_cup_entry_with_non_cup_source_reports_only_missing_license(self):
        self.assertEqual(self.issues([CUP], ["book-a"]), ["Missing OGL reference: ref/x"])

    def test_source_less_entry_reports_both(self):
        self.assertEqual(
            self.issues([CUP], []),
            ["Unreviewed source: ref/x", "Missing OGL reference: ref/x"],
        )

    def test_clean_entries_report_nothing(self):
        self.assertEqual(self.issues([OGL], ["book-a"]), [])
        self.assertEqual(self.issues([CUP], ["blog-a"]), [])


VERSION = "vabc"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write_library(root, files, gzipped):
    """A library under `root`. `files` maps a name to its decoded bytes.

    `gzipped` stores each file as `<name>.gz` with a `gzip` row, as schema 8 does, and writes only
    the stored layer (what `dist/content` holds). Otherwise every file is plain, as in schema 7.
    """
    rows = []
    for name, decoded in sorted(files.items()):
        stored = gzip.compress(decoded, 6, mtime=0) if gzipped else decoded
        path = f"{VERSION}/{name}" + (".gz" if gzipped else "")
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(stored)
        row = {"path": path, "sha256": sha(stored), "bytes": len(stored)}
        if gzipped:
            row["gzip"] = {"sha256": sha(decoded), "bytes": len(decoded)}
        rows.append(row)
    manifest = {
        "schema_version": 8 if gzipped else 7,
        "content_version": VERSION,
        "total_bytes": sum(row["bytes"] for row in rows),
        "files": rows,
        "entry_count": 2,
        "datasets": [],
    }
    (root / "manifest.json").write_text(json.dumps(manifest))
    return manifest


FILES = {
    "catalog.json": b'{"entries":[],"sources":[]}',
    "legal.json": b'{"community_use_notice":"x"}',
    "entries/spell-000.json": b'[{"id":"ref/spell/a","kind":"spell"}]',
    "entries/feat-000.json": b'[{"id":"ref/feat/b","kind":"feat"}]',
    "search.bin": b"\x00\x01" * 500,
}


class LibraryReaderTests(unittest.TestCase):
    def library(self, gzipped):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        write_library(root, FILES, gzipped)
        return root

    def test_a_gzip_library_is_read_by_logical_path(self):
        library = library_reader.Library(self.library(gzipped=True))
        self.assertEqual(library.read_json("catalog.json"), {"entries": [], "sources": []})
        self.assertEqual(library.read("search.bin"), FILES["search.bin"])
        self.assertTrue(library.has("legal.json"))
        self.assertFalse(library.has("legal.json.gz"))

    def test_a_plain_library_is_read_the_same_way(self):
        library = library_reader.Library(self.library(gzipped=False))
        self.assertEqual(library.read_json("legal.json"), {"community_use_notice": "x"})
        self.assertEqual(library.verify(), [])

    def test_entry_shards_are_found_in_a_gz_only_tree(self):
        # `dist/content` holds only `.gz` files: a glob of `entries/*.json` finds nothing there.
        root = self.library(gzipped=True)
        self.assertEqual(list((root / VERSION / "entries").glob("*.json")), [])
        library = library_reader.Library(root)
        self.assertEqual(
            library.entry_shards(), ["entries/feat-000.json", "entries/spell-000.json"]
        )
        self.assertEqual([e["id"] for e in library.entries()], ["ref/feat/b", "ref/spell/a"])

    def test_a_matching_library_has_no_issues(self):
        for gzipped in (True, False):
            self.assertEqual(library_reader.Library(self.library(gzipped)).verify(), [], gzipped)

    def test_a_damaged_stored_file_is_reported(self):
        root = self.library(gzipped=True)
        stored = root / VERSION / "search.bin.gz"
        data = bytearray(stored.read_bytes())
        data[12] ^= 1
        stored.write_bytes(bytes(data))
        issues = library_reader.Library(root).verify()
        self.assertEqual(issues, [f"Manifest mismatch: {VERSION}/search.bin.gz"])

    def test_a_decoded_hash_that_does_not_match_is_reported(self):
        root = self.library(gzipped=True)
        manifest = json.loads((root / "manifest.json").read_text())
        for row in manifest["files"]:
            if row["path"].endswith("search.bin.gz"):
                row["gzip"]["sha256"] = "0" * 64
        (root / "manifest.json").write_text(json.dumps(manifest))
        issues = library_reader.Library(root).verify()
        self.assertEqual(issues, [f"Manifest mismatch (decoded): {VERSION}/search.bin.gz"])

    def test_a_decoded_size_that_does_not_match_is_reported(self):
        root = self.library(gzipped=True)
        manifest = json.loads((root / "manifest.json").read_text())
        manifest["files"][0]["gzip"]["bytes"] += 1
        (root / "manifest.json").write_text(json.dumps(manifest))
        self.assertEqual(len(library_reader.Library(root).verify()), 1)

    def test_a_stored_file_that_is_not_gzip_is_reported_not_raised(self):
        root = self.library(gzipped=True)
        manifest = json.loads((root / "manifest.json").read_text())
        row = manifest["files"][0]
        junk = b"not gzip at all"
        (root / row["path"]).write_bytes(junk)
        row["sha256"], row["bytes"] = sha(junk), len(junk)
        (root / "manifest.json").write_text(json.dumps(manifest))
        issues = library_reader.Library(root).verify()
        self.assertEqual(len(issues), 1)
        self.assertIn(row["path"], issues[0])

    def test_a_missing_file_is_reported(self):
        root = self.library(gzipped=True)
        (root / VERSION / "legal.json.gz").unlink()
        self.assertEqual(
            library_reader.Library(root).verify(),
            [f"Manifest file is missing: {VERSION}/legal.json.gz"],
        )

    def test_a_file_the_manifest_does_not_list_is_a_key_error(self):
        library = library_reader.Library(self.library(gzipped=True))
        with self.assertRaises(KeyError):
            library.read("other.json")


if __name__ == "__main__":
    unittest.main()
