#!/usr/bin/env python3
"""Check and fix the nested placement of the Advanced Race Guide tree.

The printed Advanced Race Guide groups each chapter into races and each race
into sections, and orders the entries inside them by page. This script moves
existing nodes inside the ``advanced-race-guide`` book of ``books.toml``; it
never changes an entry id, an entry file, or an anchor.

Run it against a corpus checkout, then run ``cargo xtask corpus-fmt`` on the
same directory to rewrite the canonical TOML:

    python3 scripts/arg-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

BOOKS_FILE = "books.toml"
SOURCE = "advanced-race-guide"

# Nodes that keep their chapter but move within it.
# (parent title, node title, where, reference title)
WITHIN_CHAPTER = [
    # The printed chapter reads Dwarves, Elves, Gnomes, Half-Elves, Half-Orcs,
    # Halflings, Humans (pp. 8-79).
    ("Core Races", "Half-Orcs", "before", "Halflings"),
]

# Dataset splits that print keeps inside a longer printed section.
# (holder parent title, node title, new parent title)
NEST = [
    # Step 2 (p. 215) covers the racial qualities; the dataset split holds the
    # quality descriptions. Step 3 (p. 220) covers the racial traits.
    ("Race Builder", "Racial Qualities", "Step 2: Racial Qualities"),
    ("Race Builder", "Racial Traits", "Step 3: Racial Traits"),
]

# Sections that print keeps under New Racial Rules, but the dataset placed
# under another heading of the same race.
# (race title, holder title, new parent title, sections in printed order)
UNWRAP = [
    # Catfolk New Racial Rules runs Equipment, Feats, Magic Items, Spells
    # (pp. 93-95). The rogue-talents heading sits under Racial Archetypes
    # (p. 92), not over the feats.
    (
        "Catfolk",
        "Catfolk Rogue Talents",
        "New Racial Rules",
        ["Catfolk Feats", "Catfolk Magic Items", "Catfolk Spells"],
    ),
]


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


def find(container: dict, title: str):
    """Return (parent, index) for the first node with this title, or None."""
    for index, child in enumerate(container.get("children", [])):
        if child["title"] == title:
            return container, index
        found = find(child, title)
        if found is not None:
            return found
    return None


def root(book_data: dict) -> dict:
    """Wrap the chapter list so the tree helpers can search from the top."""
    return {"children": book_data.setdefault("chapters", [])}


def node(container: dict, title: str) -> dict:
    found = find(container, title)
    if found is None:
        raise SystemExit(f"{SOURCE}: missing node: {title}")
    return found[0]["children"][found[1]]


def entries(node: dict) -> set[str]:
    found = set()
    if node.get("entry"):
        found.add(node["entry"])
    for child in node.get("children", []):
        found |= entries(child)
    return found


def book_entries(book_data: dict) -> set[str]:
    found: set[str] = set()
    for chapter in book_data.get("chapters", []):
        found |= entries(chapter)
    return found


# ---------------------------------------------------------------------------
# Transforms


def fix_nested(book_data: dict) -> list[dict]:
    moves: list[dict] = []

    for parent_title, node_title, where, reference in WITHIN_CHAPTER:
        parent = node(root(book_data), parent_title)
        found = find(parent, node_title)
        if found is None:
            continue
        holder, index = found
        reference_index = next(
            (i for i, child in enumerate(parent["children"]) if child["title"] == reference),
            None,
        )
        if reference_index is None:
            raise SystemExit(f"{SOURCE}: missing reference: {reference}")
        if where == "before" and index == reference_index - 1:
            continue
        if where == "after" and index == reference_index + 1:
            continue
        moved = holder["children"].pop(index)
        # Recompute the reference after the removal.
        reference_index = next(
            i for i, child in enumerate(parent["children"]) if child["title"] == reference
        )
        insert_at = reference_index if where == "before" else reference_index + 1
        parent["children"].insert(insert_at, moved)
        moves.append(
            {
                "node": node_title,
                "old": f"{parent_title}, position {index + 1}",
                "new": f"{parent_title}, {where} {reference}",
            }
        )

    for parent_title, node_title, new_parent_title in NEST:
        parent = node(root(book_data), parent_title)
        found = find(parent, node_title)
        if found is None:
            continue
        holder, index = found
        target = node(root(book_data), new_parent_title)
        if find(target, node_title) is not None:
            continue
        moved = holder["children"].pop(index)
        target.setdefault("children", []).append(moved)
        moves.append(
            {
                "node": node_title,
                "old": f"{parent_title}, position {index + 1}",
                "new": f"{new_parent_title}, last",
            }
        )

    for race_title, holder_title, new_parent_title, section_titles in UNWRAP:
        race = node(root(book_data), race_title)
        found = find(race, holder_title)
        if found is None:
            continue
        race_children = race["children"]
        holder = race_children[found[1]]
        target = node(race, new_parent_title)
        for section_title in section_titles:
            section = find(holder, section_title)
            if section is None:
                continue
            moved = holder["children"].pop(section[1])
            target.setdefault("children", []).append(moved)
            moves.append(
                {
                    "node": section_title,
                    "old": f"{race_title} / {holder_title}",
                    "new": f"{race_title} / {new_parent_title}, last",
                }
            )
        if holder.get("entry") is None and not holder.get("children"):
            race_children.remove(holder)
            moves.append(
                {
                    "node": holder_title,
                    "old": f"{race_title} / empty heading",
                    "new": "removed: the printed heading holds no placed entry",
                }
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

    text = dump_books(books)
    if args.check:
        if text != original_text:
            print("books.toml is not in the nested Advanced Race Guide shape", file=sys.stderr)
            for move in moves:
                print(f"  {move['node']}: {move['old']} -> {move['new']}", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {SOURCE}; entries placed: {len(after)}")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    for move in moves:
        print(f"{move['node']}: {move['old']} -> {move['new']}")
    print(f"wrote {books_path} ({len(moves)} moves, {len(after)} entries placed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
