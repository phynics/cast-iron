#!/usr/bin/env python3
"""Read a built content library through its manifest.

A schema-8 library stores every file but `manifest.json` as one gzip member (`<name>.gz`), and
its manifest row carries the decoded hash and size under `gzip` (docs/decisions/
2026-11-27-precompressed-library.md). A row without `gzip` is a plain file, as in schema 1 to 7.
This module hides the difference: every file is named by its logical path (the path without
`.gz` when the row has `gzip`) and read decoded. `dist/content` holds only the stored files, so a
reader that opens `entries/*.json` there finds nothing.
"""

import gzip
import hashlib
import json
from pathlib import Path


class Library:
    def __init__(self, content):
        self.content = Path(content)
        self.manifest = json.loads((self.content / "manifest.json").read_text())
        self.version = self.manifest["content_version"]
        self.rows = {}
        for row in self.manifest["files"]:
            logical = row["path"][: -len(".gz")] if "gzip" in row else row["path"]
            self.rows[logical] = row

    def logical(self, name):
        """The logical path of `name`, which is relative to the version directory."""
        return f"{self.version}/{name}"

    def has(self, name):
        return self.logical(name) in self.rows

    def read(self, name):
        """The decoded bytes of the file `name`, relative to the version directory.

        Raises KeyError for a file the manifest does not list, and ValueError for a stored file
        or a decoded file that does not match its row.
        """
        row = self.rows[self.logical(name)]
        return self.decode(row)

    def decode(self, row):
        stored = (self.content / row["path"]).read_bytes()
        if len(stored) != row["bytes"] or hashlib.sha256(stored).hexdigest() != row["sha256"]:
            raise ValueError(f"Manifest mismatch: {row['path']}")
        if "gzip" not in row:
            return stored
        decoded = gzip.decompress(stored)
        want = row["gzip"]
        if len(decoded) != want["bytes"] or hashlib.sha256(decoded).hexdigest() != want["sha256"]:
            raise ValueError(f"Manifest mismatch (decoded): {row['path']}")
        return decoded

    def read_json(self, name):
        return json.loads(self.read(name))

    def verify(self):
        """One issue per row whose stored or decoded bytes do not match the manifest."""
        issues = []
        for row in self.manifest["files"]:
            try:
                self.decode(row)
            except FileNotFoundError:
                issues.append(f"Manifest file is missing: {row['path']}")
            except ValueError as error:
                issues.append(str(error))
            except (OSError, EOFError) as error:
                # gzip raises these for a stored file that is not one whole gzip member.
                issues.append(f"Manifest mismatch: {row['path']} ({error})")
        return issues

    def entry_shards(self):
        """The names of the entry shards, sorted: `entries/<kind>-<NNN>.json`."""
        prefix = self.logical("entries/")
        return sorted(
            logical[len(self.version) + 1 :]
            for logical in self.rows
            if logical.startswith(prefix) and logical.endswith(".json")
        )

    def entries(self):
        found = []
        for shard in self.entry_shards():
            found.extend(self.read_json(shard))
        return found
