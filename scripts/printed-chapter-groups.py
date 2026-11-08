#!/usr/bin/env python3
"""Rebuild the top-level book groups from the printed chapter and appendix order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id or an entry file. Run it against a corpus checkout, then run
``cargo xtask corpus-fmt`` on the same directory to rewrite the canonical TOML:

    python3 scripts/printed-chapter-groups.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.

A dataset hub entry (an entry whose body only links to the sections a printed
book presents separately) becomes the opening entry of the first printed
chapter it links to. The linked sections follow as its children or as sibling
chapters, so no entry leaves the book tree.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

BOOKS_FILE = "books.toml"


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
# Helpers


def book(books: dict, source: str) -> dict:
    for candidate in books["books"]:
        if candidate["source"] == source:
            return candidate
    raise SystemExit(f"unknown book source: {source}")


def chapter(book_data: dict, title: str) -> dict | None:
    for candidate in book_data.get("chapters", []):
        if candidate["title"] == title:
            return candidate
    return None


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


def set_review(book_data: dict, index: int, text: str) -> None:
    review = book_data.setdefault("review", [])
    if index < len(review):
        review[index] = text
    else:
        review.append(text)


# ---------------------------------------------------------------------------
# Per-book transforms


def nest_split(book_data: dict, chapter_title: str, children_titles: list[str]) -> None:
    """Move printed sub-groups inside their printed chapter (idempotent)."""
    if chapter(book_data, chapter_title) is not None:
        return
    moved = [chapter(book_data, title) for title in children_titles]
    if any(node is None for node in moved):
        raise SystemExit(f"{book_data['source']}: missing chapter to nest")
    position = min(
        book_data["chapters"].index(node) for node in moved if node is not None
    )
    book_data["chapters"] = [
        node
        for node in book_data["chapters"]
        if node["title"] not in children_titles
    ]
    book_data["chapters"].insert(
        position, {"title": chapter_title, "children": list(moved)}
    )


def nesting_advanced_players_guide(book_data: dict) -> None:
    nest_split(book_data, "Classes", ["Base Classes", "Core Classes"])
    set_review(
        book_data,
        0,
        "Base Classes and Core Classes are dataset splits kept as sub-groups "
        "inside the printed Classes chapter.",
    )


def nesting_ultimate_campaign(book_data: dict) -> None:
    source = chapter(book_data, "Character Background")
    story = chapter(book_data, "Story Feats")
    if source is None or story is None:
        return
    book_data["chapters"] = [
        node for node in book_data["chapters"] if node is not story
    ]
    source.setdefault("children", []).append(story)
    set_review(
        book_data,
        0,
        "Story Feats is a dataset split kept as a sub-group inside the printed "
        "Character Background chapter.",
    )
    basis = book_data.get("basis", "")
    book_data["basis"] = basis.replace(
        " The dataset splits the printed Character Background chapter into "
        "Character Background and Story Feats.",
        "",
    )


def nesting_ultimate_combat(book_data: dict) -> None:
    classes = chapter(book_data, "Classes")
    archetypes = chapter(book_data, "Class Archetypes")
    if classes is None or archetypes is None:
        return
    book_data["chapters"] = [
        node for node in book_data["chapters"] if node is not archetypes
    ]
    classes.setdefault("children", []).append(archetypes)
    set_review(
        book_data,
        0,
        "Class Archetypes is a dataset split kept as a sub-group inside the "
        "printed Classes chapter.",
    )


def nesting_ultimate_magic(book_data: dict) -> None:
    mastering = chapter(book_data, "Mastering Magic")
    familiars = chapter(book_data, "New Familiars")
    if mastering is not None and familiars is not None:
        book_data["chapters"] = [
            node for node in book_data["chapters"] if node is not familiars
        ]
        children = mastering.setdefault("children", [])
        position = len(children)
        for index, node in enumerate(children):
            if node["title"] == "Spellbooks":
                position = index
                break
        children.insert(position, familiars)
    updates = chapter(book_data, "Updates")
    if updates is not None:
        updates["title"] = "Appendix"
    set_review(
        book_data,
        0,
        "New Familiars is a dataset split kept as a sub-group inside the "
        "printed Mastering Magic chapter; the printed appendix is the Appendix "
        "chapter.",
    )


def appendices_advanced_race_guide(book_data: dict) -> None:
    combined = chapter(book_data, "Age, Height & Weight")
    if combined is None:
        return
    age = None
    height = None
    rest: list[dict] = []
    for child in combined.get("children", []):
        if child.get("entry", "").endswith("/age/height-and-weight/age"):
            age = child
        elif child.get("entry", "").endswith(
            "/age/height-and-weight/appendix-2-height-and-weight"
        ):
            height = child
        else:
            rest.append(child)
    if age is None or height is None:
        raise SystemExit("advanced-race-guide: appendix children not found")
    position = book_data["chapters"].index(combined)
    replacement = [
        {
            "title": "Age",
            "entry": combined.get("entry"),
            "children": [age] + rest,
        },
        {"title": "Height and Weight", "entry": height.get("entry")},
    ]
    book_data["chapters"][position : position + 1] = replacement
    set_review(
        book_data,
        0,
        "Appendix 1 Age and Appendix 2 Height and Weight are separate top-level "
        "appendices, split from the dataset's combined chapter. The dataset has "
        "no chapter for Appendix 3: Spells.",
    )


def appendices_core_rulebook(book_data: dict) -> None:
    glossary = chapter(book_data, "Glossary")
    if glossary is None:
        return
    special = None
    conditions = None
    for child in glossary.get("children", []):
        if child.get("entry", "").endswith("/glossary/special-abilities"):
            special = child
        elif child.get("entry") == "ref/rule/conditions":
            conditions = child
    if special is None or conditions is None:
        raise SystemExit("core-rulebook: glossary children not found")
    position = book_data["chapters"].index(glossary)
    replacement = [
        {
            "title": "Special Abilities",
            "entry": glossary.get("entry"),
            "children": [special],
        },
        {"title": "Conditions", "entry": conditions.get("entry")},
    ]
    book_data["chapters"][position : position + 1] = replacement
    book_data["basis"] = (
        "The 15 chapter titles, their order, and their first pages are checked "
        "against the printed contents page of the Core Rulebook. Its appendices "
        "are Special Abilities, Conditions, Inspiring Reading, and Game Aids; "
        "the first two are top-level groups and the last two have no corpus "
        "entries. The dataset's Glossary aggregate opens Special Abilities. See "
        "docs/research/book-order-evidence.md. Canonical ids were substituted "
        "through the reviewed alias map; chapters and order are unchanged."
    )


def appendices_bestiary(book_data: dict) -> None:
    monsters = chapter(book_data, "Monsters")
    if monsters is not None:
        monsters["title"] = "Monsters A to Z"
    types = chapter(book_data, "Creature Types")
    if types is not None:
        types["title"] = "Monsters by Type"
    misc = chapter(book_data, "Miscellaneous Monster Rules")
    if misc is not None:
        as_pcs = None
        cohorts = None
        for child in misc.get("children", []):
            if child.get("entry", "").endswith("/monster-as-pcs"):
                as_pcs = child
            elif child.get("entry", "").endswith("/monster-cohorts"):
                cohorts = child
        if as_pcs is None or cohorts is None:
            raise SystemExit("bestiary: miscellaneous monster rules children not found")
        position = book_data["chapters"].index(misc)
        replacement = [
            {
                "title": "Monsters as PCs",
                "entry": misc.get("entry"),
                "children": [as_pcs],
            },
            {"title": "Monster Cohorts", "entry": cohorts.get("entry")},
        ]
        book_data["chapters"][position : position + 1] = replacement
    order = [
        "Introduction",
        "Monsters A to Z",
        "Universal Monster Rules",
        "Monster Creation",
        "Monster Advancement",
        "Monsters as PCs",
        "Monster Feats",
        "Monster Cohorts",
        "Monsters by Type",
        "Monster Roles",
        "Encounter Tables",
        "New entries (page order)",
    ]
    by_title = {node["title"]: node for node in book_data["chapters"]}
    if set(by_title) != set(order):
        raise SystemExit(
            "bestiary: unexpected chapters after transform: "
            + ", ".join(sorted(by_title))
        )
    book_data["chapters"] = [by_title[title] for title in order]
    set_review(
        book_data,
        0,
        "The dataset holds no entries for printed Appendix 3: Glossary, "
        "Appendix 7: Animal Companions, or Appendices 9-12. Appendices 4 and 6 "
        "are separate top-level appendices; Universal Monster Rules is a "
        "dataset appendix after Monsters A to Z.",
    )


def rename_monsters(book_data: dict) -> None:
    monsters = chapter(book_data, "Monsters")
    if monsters is not None:
        monsters["title"] = "Monsters A to Z"


GMG_CHAPTERS = [
    ("Introduction", "ref/rule/gamemastery-guide", 4),
    ("Getting Started", "ref/rule/getting-started", 6),
    ("Running a Game", "ref/rule/running-a-game", 22),
    ("Player Characters", "ref/rule/player-characters", 56),
    ("Nonplayer Characters", "ref/rule/non-player-characters", 78),
    ("Rewards", "ref/rule/rewards", 100),
    ("Creating a World", "ref/rule/creating-a-world", 138),
    ("Adventures", "ref/rule/adventures", 170),
    ("Advanced Topics", "ref/rule/advanced-topics", 228),
    ("NPC Gallery", "ref/rule/npc-gallery", 252),
]

GMG_PAGE = re.compile(r"GameMastery Guide pg (\d+)")


def gamemastery_guide(book_data: dict, corpus: Path) -> None:
    page_order = chapter(book_data, "Page order")
    if page_order is None:
        return
    items = page_order.get("children", [])
    page_cache: dict[str, int | None] = {}

    def page(entry_id: str | None) -> int | None:
        if entry_id is None:
            return None
        if entry_id not in page_cache:
            path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
            match = GMG_PAGE.search(path.read_text(encoding="utf-8"))
            page_cache[entry_id] = int(match.group(1)) if match else None
        return page_cache[entry_id]

    heading = {entry_id: title for title, entry_id, _start in GMG_CHAPTERS}
    titles = [title for title, _entry, _start in GMG_CHAPTERS]
    chapters: list[dict] = [{"title": title} for title in titles]
    index = 0
    last_page = GMG_CHAPTERS[0][2]
    for item in items:
        entry_id = item.get("entry")
        item_page = page(entry_id)
        if item_page is None:
            item_page = last_page
        last_page = item_page
        while (
            index + 1 < len(GMG_CHAPTERS)
            and item_page >= GMG_CHAPTERS[index + 1][2]
        ):
            index += 1
        if entry_id in heading:
            title = heading[entry_id]
            index = titles.index(title)
            chapters[index]["entry"] = entry_id
            if item.get("children"):
                chapters[index].setdefault("children", []).extend(item["children"])
            continue
        chapters[index].setdefault("children", []).append(item)
    book_data["chapters"] = chapters
    book_data["basis"] = (
        "The chapter titles, their order, and their first pages are checked "
        "against the printed contents page of the GameMastery Guide. Entries are "
        "grouped by the page each Pf Data record attributes to this publication. "
        "See docs/research/book-order-evidence.md. Canonical ids were substituted "
        "through the reviewed alias map; chapters and order are unchanged."
    )
    book_data["review"] = [
        "Nested placement is not checked against the printed subheadings.",
        "The printed Appendix (310) has no corpus entries.",
        "Entries on the same page are not in printed order.",
    ]


TRANSFORMS = {
    "advanced-players-guide": nesting_advanced_players_guide,
    "advanced-race-guide": appendices_advanced_race_guide,
    "bestiary": appendices_bestiary,
    "bestiary-2": rename_monsters,
    "bestiary-3": rename_monsters,
    "bestiary-4": rename_monsters,
    "core-rulebook": appendices_core_rulebook,
    "ultimate-campaign": nesting_ultimate_campaign,
    "ultimate-combat": nesting_ultimate_combat,
    "ultimate-magic": nesting_ultimate_magic,
}


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

    changed_sources = list(TRANSFORMS) + ["gamemastery-guide"]
    before = {source: book_entries(book(books, source)) for source in changed_sources}
    old_groups = {
        source: [node["title"] for node in book(books, source)["chapters"]]
        for source in changed_sources
    }

    for source, transform in TRANSFORMS.items():
        transform(book(books, source))
    gamemastery_guide(book(books, "gamemastery-guide"), args.corpus)

    after = {source: book_entries(book(books, source)) for source in changed_sources}
    changed = 0
    for source in changed_sources:
        new_groups = [
            node["title"] for node in book(books, source)["chapters"]
        ]
        if before[source] != after[source]:
            missing = sorted(before[source] - after[source])
            added = sorted(after[source] - before[source])
            print(f"{source}: entry set changed", file=sys.stderr)
            for entry in missing:
                print(f"  lost: {entry}", file=sys.stderr)
            for entry in added:
                print(f"  added: {entry}", file=sys.stderr)
            return 1
        if old_groups[source] != new_groups:
            changed += 1
        print(f"{source}:")
        print(f"  before: {', '.join(old_groups.get(source, [])) or '(none)'}")
        print(f"  after:  {', '.join(new_groups) or '(none)'}")
        print(f"  entries placed: {len(after[source])}")

    text = dump_books(books)
    if args.check:
        if text != original_text:
            print("books.toml is not in the printed-chapter shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {changed} changed books")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({changed} books restructured)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
