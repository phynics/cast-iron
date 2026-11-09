#!/usr/bin/env python3
"""Place the NPC Codex tree by printed class section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/npc-codex-nested.py /path/to/cast-iron-data
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

BOOK = "npc-codex"
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


def find_book(books: dict) -> dict:
    for book in books["books"]:
        if book["source"] == BOOK:
            return book
    raise SystemExit(f"unknown book source: {BOOK}")


def flatten(node: dict) -> list[dict]:
    found = [node]
    for child in node.get("children", []):
        found.extend(flatten(child))
    return found


def find_node(book: dict, entry_id: str) -> dict | None:
    for chapter in book.get("chapters", []):
        for node in flatten(chapter):
            if node.get("entry") == entry_id:
                return node
    return None


def path_of(book: dict, node: dict) -> str:
    def walk(current: dict, trail: list[str]) -> str | None:
        here = trail + [current["title"]]
        if current is node:
            return " > ".join(here)
        for child in current.get("children", []):
            found = walk(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter, [])
        if found is not None:
            return found
    return "(missing)"


def placed_entries(book: dict) -> dict[str, str]:
    paths: dict[str, str] = {}
    for chapter in book.get("chapters", []):
        for node in flatten(chapter):
            if node.get("entry"):
                paths[node["entry"]] = path_of(book, node)
    return paths


# ---------------------------------------------------------------------------
# Page evidence

NPC_CODEX_PAGE = re.compile(r"NPC Codex\s+pg\.?\s*(\d+)")

# The source line is the printed page for the shipped NPCs. These entries print
# on a different page than their source line gives, or carry no source line.
PAGE_OVERRIDES: dict[str, int] = {
    # The Eldritch Knight source lines are shifted across the four stat blocks.
    "ref/npc/champion-of-magic": 221,
    "ref/npc/spirit-avenger": 222,
    "ref/npc/mithral-wizard": 223,
    # The carried PSRD creature has no source line; it prints between the
    # monk at p. 101 and the monk at p. 103.
    "ref/carried/npc-codex/rules/core-class-npcs/monk/monks/poisonous-performer": 102,
}


def page_of(corpus: Path, entry_id: str) -> int | None:
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = NPC_CODEX_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


def link_order(corpus: Path, entry_id: str) -> dict[str, int]:
    """The dataset order of the NPC links in a class entry; the same-page tie."""
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return {}
    found = re.findall(r'target = "([^"]+)"', path.read_text(encoding="utf-8"))
    return {target: index for index, target in enumerate(found)}


# ---------------------------------------------------------------------------
# Printed structure
#
# The printed contents page lists the chapters, the classes inside them, and
# the first page of each class. The page ranges end where the next class or
# chapter starts. Every class section prints its NPCs from level 1 up, two or
# more stat blocks per page.

ClassSection = tuple[str, str, str, str, int, int]

# (chapter title, class title, class entry, list heading, first page, last page)
CLASSES: list[ClassSection] = [
    (
        "Core Classes",
        "Barbarian",
        "ref/carried/npc-codex/rules/core-class-npcs/barbarian",
        "Barbarians",
        10,
        25,
    ),
    (
        "Core Classes",
        "Bard",
        "ref/carried/npc-codex/rules/core-class-npcs/bard",
        "Bards",
        26,
        43,
    ),
    (
        "Core Classes",
        "Cleric",
        "ref/carried/npc-codex/rules/core-class-npcs/cleric",
        "Clerics",
        44,
        61,
    ),
    (
        "Core Classes",
        "Druid",
        "ref/carried/npc-codex/rules/core-class-npcs/druid",
        "Druids",
        62,
        79,
    ),
    (
        "Core Classes",
        "Fighter",
        "ref/carried/npc-codex/rules/core-class-npcs/fighter",
        "Fighters",
        80,
        95,
    ),
    (
        "Core Classes",
        "Monk",
        "ref/carried/npc-codex/rules/core-class-npcs/monk",
        "Monks",
        96,
        111,
    ),
    (
        "Core Classes",
        "Paladin",
        "ref/carried/npc-codex/rules/core-class-npcs/paladin",
        "Paladins",
        112,
        127,
    ),
    (
        "Core Classes",
        "Ranger",
        "ref/carried/npc-codex/rules/core-class-npcs/ranger",
        "Rangers",
        128,
        143,
    ),
    (
        "Core Classes",
        "Rogue",
        "ref/carried/npc-codex/rules/core-class-npcs/rogue",
        "Rogues",
        144,
        159,
    ),
    (
        "Core Classes",
        "Sorcerer",
        "ref/carried/npc-codex/rules/core-class-npcs/sorcerer",
        "Sorcerers",
        160,
        177,
    ),
    (
        "Core Classes",
        "Wizard",
        "ref/carried/npc-codex/rules/core-class-npcs/wizard",
        "Wizards",
        178,
        197,
    ),
    (
        "Prestige Classes",
        "Arcane Archer",
        "ref/carried/npc-codex/rules/prestige-class-npcs/arcane-archer",
        "Arcane Archers",
        200,
        203,
    ),
    (
        "Prestige Classes",
        "Arcane Trickster",
        "ref/carried/npc-codex/rules/prestige-class-npcs/arcane-trickster",
        "Arcane Tricksters",
        204,
        207,
    ),
    (
        "Prestige Classes",
        "Assassin",
        "ref/carried/npc-codex/rules/prestige-class-npcs/assassin",
        "Assassins",
        208,
        211,
    ),
    (
        "Prestige Classes",
        "Dragon Disciple",
        "ref/carried/npc-codex/rules/prestige-class-npcs/dragon-disciple",
        "Dragon Disciples",
        212,
        215,
    ),
    (
        "Prestige Classes",
        "Duelist",
        "ref/carried/npc-codex/rules/prestige-class-npcs/duelist",
        "Duelists",
        216,
        219,
    ),
    (
        "Prestige Classes",
        "Eldritch Knight",
        "ref/carried/npc-codex/rules/prestige-class-npcs/eldritch-knight",
        "Eldritch Knights",
        220,
        223,
    ),
    (
        "Prestige Classes",
        "Loremaster",
        "ref/carried/npc-codex/rules/prestige-class-npcs/loremaster",
        "Loremasters",
        224,
        227,
    ),
    (
        "Prestige Classes",
        "Mystic Theurge",
        "ref/carried/npc-codex/rules/prestige-class-npcs/mystic-theurge",
        "Mystic Theurges",
        228,
        231,
    ),
    (
        "Prestige Classes",
        "Pathfinder Chronicler",
        "ref/carried/npc-codex/rules/prestige-class-npcs/pathfinder-chronicler",
        "Pathfinder Chroniclers",
        232,
        235,
    ),
    (
        "Prestige Classes",
        "Shadowdancer",
        "ref/carried/npc-codex/rules/prestige-class-npcs/shadowdancer",
        "Shadowdancers",
        236,
        241,
    ),
    (
        "NPC Classes",
        "Adept",
        "ref/carried/npc-codex/rules/npc-class-npcs/adept",
        "Adepts",
        244,
        249,
    ),
    (
        "NPC Classes",
        "Aristocrat",
        "ref/carried/npc-codex/rules/npc-class-npcs/aristocrat",
        "Aristocrats",
        250,
        255,
    ),
    (
        "NPC Classes",
        "Commoner",
        "ref/carried/npc-codex/rules/npc-class-npcs/commoner",
        "Commoners",
        256,
        259,
    ),
    (
        "NPC Classes",
        "Expert",
        "ref/carried/npc-codex/rules/npc-class-npcs/expert",
        "Experts",
        260,
        265,
    ),
    (
        "NPC Classes",
        "Warrior",
        "ref/carried/npc-codex/rules/npc-class-npcs/warrior",
        "Warriors",
        266,
        273,
    ),
]

# The printed chapter opening entry. The chapter node itself is the dataset hub;
# the overview text follows as its first child.
CHAPTERS: dict[str, str] = {
    "Core Classes": "ref/carried/npc-codex/rules/core-class-npcs/core-classes",
    "Prestige Classes": "ref/carried/npc-codex/rules/prestige-class-npcs/prestige-classes",
    "NPC Classes": "ref/carried/npc-codex/rules/npc-class-npcs/npc-classes",
}

BASIS = (
    "The main chapter titles, their order, and their first pages are checked "
    "against the printed contents page of the NPC Codex. Each class section "
    "holds its NPCs in printed page order. See "
    "docs/research/book-order-evidence.md. Canonical ids were substituted "
    "through the reviewed alias map; chapters and order are unchanged."
)

REVIEW = [
    "Each class section holds its NPCs in printed page order; entries that share a printed page keep the dataset order. The dataset's plural class list groups stay inside their class section.",
    "The printed chapter 4: Iconic Characters has no corresponding chapter in the dataset.",
    "Canonical ids were substituted through sources/canonical/id-map.toml. A legacy entry with no installed target is omitted and listed in coverage.",
]


def class_of(page: int) -> ClassSection | None:
    for section in CLASSES:
        if section[4] <= page <= section[5]:
            return section
    return None


def group_of(class_node: dict, title: str) -> dict:
    children = class_node.setdefault("children", [])
    groups = [child for child in children if child.get("title") == title and not child.get("entry")]
    if len(groups) > 1:
        raise SystemExit(f"duplicate group {title!r} under {class_node['title']!r}")
    if groups:
        return groups[0]
    group = {"title": title, "anchor": title.lower().replace(" ", "-")}
    children.append(group)
    return group


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    before_paths = placed_entries(book)

    chapters = {chapter["title"]: chapter for chapter in book["chapters"]}
    for chapter_title in CHAPTERS:
        if chapter_title not in chapters:
            raise SystemExit(f"missing chapter {chapter_title!r}")

    # Pool every NPC that currently sits under one of the printed class groups.
    pool: dict[str, dict] = {}
    tie: dict[str, dict[str, int]] = {}
    for _chapter_title, _class_title, class_entry, group_title, _start, _end in CLASSES:
        class_node = find_node(book, class_entry)
        if class_node is None:
            raise SystemExit(f"missing class {class_entry}")
        tie[group_title] = link_order(corpus, class_entry)
        group = group_of(class_node, group_title)
        for child in list(group.get("children", [])):
            entry_id = child.get("entry")
            if not entry_id:
                raise SystemExit(f"group {group_title!r} holds a bare heading")
            if entry_id in pool:
                raise SystemExit(f"entry {entry_id} appears under two classes")
            pool[entry_id] = child

    # Assign each NPC to the class whose printed pages contain it, then order
    # each class by page. Entries on one page follow the dataset link order.
    target: dict[str, list[dict]] = {section[3]: [] for section in CLASSES}
    for entry_id, node in pool.items():
        page = page_of(corpus, entry_id)
        if page is None:
            raise SystemExit(f"no printed page for {entry_id}")
        section = class_of(page)
        if section is None:
            raise SystemExit(f"{entry_id} prints on p. {page}, outside every class")
        target[section[3]].append(node)

    for title in target:
        target[title].sort(
            key=lambda node: (
                page_of(corpus, node["entry"]) or 10**9,
                tie[title].get(node["entry"], 10**9),
            )
        )

    # Rebuild each chapter as its printed opening plus the printed class order.
    for chapter_title, chapter in chapters.items():
        if chapter_title not in CHAPTERS:
            continue
        class_entries = [section[2] for section in CLASSES if section[0] == chapter_title]
        class_titles = [section[1] for section in CLASSES if section[0] == chapter_title]
        overview_entry = CHAPTERS[chapter_title]
        overview = find_node(book, overview_entry)
        if overview is None:
            raise SystemExit(f"missing chapter overview {overview_entry}")
        overview.pop("children", None)

        nodes: list[dict] = [overview]
        for class_entry, class_title in zip(class_entries, class_titles):
            class_node = find_node(book, class_entry)
            assert class_node is not None
            if class_node["title"] != class_title:
                raise SystemExit(
                    f"class {class_entry} has title {class_node['title']!r}, expected {class_title!r}"
                )
            nodes.append(class_node)
        chapter["children"] = nodes

    # Each class holds exactly its list group with the printed NPCs.
    for _chapter_title, _class_title, class_entry, group_title, _start, _end in CLASSES:
        class_node = find_node(book, class_entry)
        assert class_node is not None
        group = group_of(class_node, group_title)
        group["children"] = target[group_title]
        extra = [child for child in class_node.get("children", []) if child is not group]
        if extra:
            raise SystemExit(
                f"class {class_entry} holds unexpected sibling {extra[0].get('title')!r}"
            )

    after_paths = placed_entries(book)
    if set(before_paths) != set(after_paths):
        lost = sorted(set(before_paths) - set(after_paths))
        added = sorted(set(after_paths) - set(before_paths))
        raise SystemExit(f"entry set changed: lost {lost}, added {added}")

    moves = []
    for entry_id in sorted(before_paths):
        if before_paths[entry_id] != after_paths[entry_id]:
            title = after_paths[entry_id].split(" > ")[-1]
            moves.append(
                (
                    title,
                    before_paths[entry_id],
                    after_paths[entry_id],
                    page_of(corpus, entry_id),
                )
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
    book = find_book(books)
    moves = transform(book, args.corpus)

    book["basis"] = BASIS
    book["review"] = REVIEW
    text = dump_books(books)

    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the NPC Codex nested-placement shape", file=sys.stderr)
            return 1
        print(
            f"books.toml is canonical for {BOOK}; {len(moves)} moves",
            file=sys.stderr,
        )
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(
        f"wrote {books_path} ({len(moves)} moves, {len(placed_entries(book))} entries placed)",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
