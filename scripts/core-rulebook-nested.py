#!/usr/bin/env python3
"""Check and fix the nested placement of the Core Rulebook tree.

The printed Core Rulebook groups its rules into chapters and sections, and
orders the entries inside them by page. This script moves existing nodes inside
the ``core-rulebook`` book of ``books.toml``; it never changes an entry id, an
entry file, or an anchor.

Run it against a corpus checkout, then run ``cargo xtask corpus-fmt`` on the
same directory to rewrite the canonical TOML:

    python3 scripts/core-rulebook-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

BOOKS_FILE = "books.toml"
SOURCE = "core-rulebook"

# Entries that print places in another chapter. The value is the printed
# chapter that the entry opens.
OPEN_CHAPTER = {
    "ref/rule/prestige-classes": ("Classes", "Prestige Classes"),
    "ref/rule/creating-npcs": ("Gamemastering", "Creating NPCs"),
}

# Nodes that keep their chapter but move within it.
# (parent title, node title, where, reference title)
WITHIN_CHAPTER = [
    ("Magic", "Magic and Spellcasting", "first", None),
    ("Races", "Half-Orcs", "before", "Halflings"),
    ("Feats", "Great Fortitude", "before", "Greater Bull Rush"),
    ("Feats", "Spellbreaker", "after", "Spell Penetration"),
]

ROMAN = {
    "i": 1, "ii": 2, "iii": 3, "iv": 4, "v": 5,
    "vi": 6, "vii": 7, "viii": 8, "ix": 9,
}


# ---------------------------------------------------------------------------
# TOML writer for the BookIndex shape


def _string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _array(values: list[str]) -> str:
    return "[" + ", ".join(_string(value) for value in values) + "]"


def _write_node(node: dict, path: str, out: list[str]) -> None:
    out.append(f"[[{path}]]")
    out.append(f"title = {_string(node['title'])}")
    if node.get("entry") is not None:
        out.append(f"entry = {_string(node['entry'])}")
    if node.get("anchor") is not None:
        out.append(f"anchor = {_string(node['anchor'])}")
    if node.get("review") is not None:
        out.append(f"review = {_string(node['review'])}")
    if node.get("collapse"):
        out.append("collapse = true")
    out.append("")
    for child in node.get("children", []):
        _write_node(child, path + ".children", out)


def dump_books(books: dict) -> str:
    out: list[str] = []
    for book in books["books"]:
        out.append("[[books]]")
        out.append(f"source = {_string(book['source'])}")
        out.append(f"title = {_string(book['title'])}")
        if book.get("basis") is not None:
            out.append(f"basis = {_string(book['basis'])}")
        if book.get("review") is not None:
            out.append(f"review = {_array(book['review'])}")
        out.append("")
        for chapter in book.get("chapters", []):
            _write_node(chapter, "books.chapters", out)
    return "\n".join(out).rstrip("\n") + "\n"


# ---------------------------------------------------------------------------
# Tree helpers


def book(books: dict) -> dict:
    for candidate in books["books"]:
        if candidate["source"] == SOURCE:
            return candidate
    raise SystemExit(f"unknown book source: {SOURCE}")


def chapter(book_data: dict, title: str) -> dict:
    for candidate in book_data.get("chapters", []):
        if candidate["title"] == title:
            return candidate
    raise SystemExit(f"{SOURCE}: missing chapter: {title}")


def entries(node: dict) -> set[str]:
    found = set()
    if node.get("entry"):
        found.add(node["entry"])
    for child in node.get("children", []):
        found |= entries(child)
    return found


def book_entries(book_data: dict) -> set[str]:
    found: set[str] = set()
    for node in book_data.get("chapters", []):
        found |= entries(node)
    return found


def insert_relative(book_data: dict, parent_title: str, node: dict, where: str, reference: str | None) -> None:
    parent = chapter(book_data, parent_title)
    children = parent.setdefault("children", [])
    if where == "first":
        children.insert(0, node)
        return
    if where not in ("before", "after") or reference is None:
        raise SystemExit(f"{SOURCE}: bad target for {node['title']}")
    for index, child in enumerate(children):
        if child["title"] == reference:
            children.insert(index if where == "before" else index + 1, node)
            return
    raise SystemExit(f"{SOURCE}: missing reference: {reference}")


# ---------------------------------------------------------------------------
# Printed order


def reordered(before: list[dict], after: list[dict]) -> list[dict]:
    """Return the nodes that leave the longest common subsequence."""
    n, m = len(before), len(after)
    table = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            if before[i - 1] is after[j - 1]:
                table[i][j] = table[i - 1][j - 1] + 1
            else:
                table[i][j] = max(table[i - 1][j], table[i][j - 1])
    keep: set[int] = set()
    i, j = n, m
    while i and j:
        if before[i - 1] is after[j - 1]:
            keep.add(id(before[i - 1]))
            i -= 1
            j -= 1
        elif table[i - 1][j] >= table[i][j - 1]:
            i -= 1
        else:
            j -= 1
    return [node for node in before if id(node) not in keep]


def sort_key(title: str) -> str:
    """Return the key print uses inside an alphabetical section."""
    text = title.lower().replace("\u2019", "'")
    text = re.sub(r"[^a-z0-9 ]", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    # "Greater X", "Lesser X" and "Mass X" are alphabetized under X.
    for prefix in ("greater ", "lesser ", "mass "):
        if text.startswith(prefix):
            text = text[len(prefix):] + " " + prefix.strip()
            break
    # Spell chains use roman numerals: I comes before II, not after IX.
    match = re.match(r"^(.*) (i|ii|iii|iv|v|vi|vii|viii|ix)$", text)
    if match:
        text = f"{match.group(1)} {ROMAN[match.group(2)]:02d}"
    return text


# ---------------------------------------------------------------------------
# Transforms


def fix_nested(book_data: dict) -> list[str]:
    moves: list[str] = []

    for entry_id, (old_parent, new_chapter) in OPEN_CHAPTER.items():
        target = chapter(book_data, new_chapter)
        if target.get("entry") == entry_id:
            continue
        # Find the node by entry id inside the old chapter.
        old = chapter(book_data, old_parent)
        found = None
        for child in old.get("children", []):
            if child.get("entry") == entry_id:
                found = child
                break
        if found is None:
            continue
        old["children"].remove(found)
        target["entry"] = entry_id
        moves.append(f"{found['title']}: {old_parent} -> {new_chapter} (opening entry)")

    for parent_title, node_title, where, reference in WITHIN_CHAPTER:
        parent = chapter(book_data, parent_title)
        found = None
        for index, child in enumerate(parent.get("children", [])):
            if child["title"] == node_title:
                found = index
                break
        if found is None:
            continue
        # Already in place?
        if where == "first" and found == 0:
            continue
        if where == "before":
            ref = next((i for i, c in enumerate(parent["children"]) if c["title"] == reference), None)
            if ref is not None and found == ref - 1:
                continue
        if where == "after":
            ref = next((i for i, c in enumerate(parent["children"]) if c["title"] == reference), None)
            if ref is not None and found == ref + 1:
                continue
        node = parent["children"].pop(found)
        insert_relative(book_data, parent_title, node, where, reference)
        target = "first" if where == "first" else f"{where} {reference}"
        moves.append(f"{node_title}: {parent_title} -> {target}")

    spells = chapter(book_data, "Spells")
    children = spells.get("children", [])
    hub = [child for child in children if child["title"] == "Spells"]
    rest = [child for child in children if child["title"] != "Spells"]
    ordered = sorted(rest, key=lambda child: sort_key(child["title"]))
    if ordered != rest:
        spells["children"] = hub + ordered
        for child in reordered(rest, ordered):
            moves.append(
                f"{child['title']}: Spells -> printed alphabetical position"
            )
    return moves


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus", type=Path, help="corpus checkout that holds books.toml")
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when books.toml is not in the target shape",
    )
    args = parser.parse_args()

    books_path = args.corpus / BOOKS_FILE
    original_text = books_path.read_text(encoding="utf-8")
    books = tomllib.loads(original_text)
    book_data = book(books)
    before = book_entries(book_data)

    moves = fix_nested(book_data)
    after = book_entries(book_data)
    if before != after:
        print(f"{SOURCE}: entry set changed", file=sys.stderr)
        for entry in sorted(before - after):
            print(f"  lost: {entry}", file=sys.stderr)
        for entry in sorted(after - before):
            print(f"  added: {entry}", file=sys.stderr)
        return 1

    book_data["review"] = [
        "Canonical ids were substituted through sources/canonical/id-map.toml. "
        "A legacy entry with no installed target is omitted and listed in coverage."
    ]

    text = dump_books(books)
    if args.check:
        if text != original_text:
            print("books.toml is not in the nested Core Rulebook shape", file=sys.stderr)
            for move in moves:
                print(f"  {move}", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {SOURCE}; entries placed: {len(after)}")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    for move in moves:
        print(move)
    print(f"wrote {books_path} ({len(moves)} moves, {len(after)} entries placed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
