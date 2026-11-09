#!/usr/bin/env python3
"""Place the Game Mastery Guide tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/gmg-nested.py /path/to/cast-iron-data
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

BOOK = "game-mastery-guide"
# The corpus exported the same publication twice. The printed tree below used this
# page-order source id, which is not a publication id; the dataset-topic tree used
# the publication id. Merge the first into the second and retire this id.
RETIRED_BOOK = "gamemastery-guide"
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
# Tree helpers


def find_book(books: dict) -> dict:
    for book in books["books"]:
        if book["source"] == BOOK:
            return book
    raise SystemExit(f"unknown book source: {BOOK}")


def merge_duplicate_book(books: dict) -> list[dict]:
    """Merge the retired page-order tree into the printed tree.

    Returns the entry nodes that only the retired tree placed. The caller feeds
    them to ``transform``, which puts each at its printed section. Running this
    again on the merged corpus finds no retired tree and returns nothing.
    """
    printed = next(
        (book for book in books["books"] if book["source"] == RETIRED_BOOK), None
    )
    topic = next((book for book in books["books"] if book["source"] == BOOK), None)
    if printed is None or topic is None:
        return []
    placed = book_entries(printed)
    extras: list[dict] = []
    for chapter in topic["chapters"]:
        for node in flatten(chapter):
            entry_id = node.get("entry")
            if entry_id and entry_id not in placed:
                # Every extra is placed on its own; its old group is not kept.
                extras.append(
                    {
                        key: value
                        for key, value in node.items()
                        if key != "children"
                    }
                )
    books["books"].remove(topic)
    printed["source"] = BOOK
    return extras


def find_node(book: dict, entry_id: str) -> dict | None:
    def walk(node: dict) -> dict | None:
        if node.get("entry") == entry_id:
            return node
        for child in node.get("children", []):
            found = walk(child)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter)
        if found is not None:
            return found
    return None


def path_of(book: dict, node: dict) -> str:
    def walk(current: dict, trail: list[str]) -> list[str] | None:
        here = trail + [current["title"]]
        if current is node:
            return here
        for child in current.get("children", []):
            found = walk(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter, [])
        if found is not None:
            return " > ".join(found)
    return "(missing)"


def flatten(node: dict) -> list[dict]:
    found = [node]
    for child in node.get("children", []):
        found.extend(flatten(child))
    return found


def book_entries(book: dict) -> set[str]:
    placed: set[str] = set()
    for chapter in book.get("chapters", []):
        for node in flatten(chapter):
            if node.get("entry"):
                placed.add(node["entry"])
    return placed


def detach(book: dict, entry_id: str) -> dict:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")
    for chapter in book["chapters"]:
        for parent in flatten(chapter):
            if node in parent.get("children", []):
                parent["children"].remove(node)
                return node
    raise SystemExit(f"entry {entry_id} is already a chapter")


# ---------------------------------------------------------------------------
# Page evidence

GMG_PAGE = re.compile(r"GameMastery Guide(?: pg\.?| p\.) ?(\d+)")


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = GMG_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


# ---------------------------------------------------------------------------
# Printed sections
#
# Each chapter lists the printed section headings in printed order. The start
# page is the first page of that section. A heading entry becomes the section
# node; a printed heading the dataset has no entry for becomes a group node.

Section = tuple[str, int, str | None]

CHAPTERS: list[tuple[str, int]] = [
    ("Getting Started", 8),
    ("Running a Game", 24),
    ("Player Characters", 58),
    ("Nonplayer Characters", 80),
    ("Rewards", 102),
    ("Creating a World", 140),
    ("Adventures", 172),
    ("Advanced Topics", 230),
    ("NPC Gallery", 254),
]

SECTIONS: dict[str, list[Section]] = {
    "Getting Started": [
        ("Duties of a GM", 8, None),
        ("Tone and Maturity", 10, "ref/rule/tone-and-maturity"),
        ("The GM as Host", 12, "ref/rule/the-gm-as-host"),
        ("Rules of the House", 14, "ref/rule/rules-of-the-house"),
        ("Preparing to Run a Game", 16, "ref/rule/preparing-to-run-a-game"),
        ("Creating a Campaign Guide", 20, "ref/rule/creating-a-campaign-guide"),
    ],
    "Running a Game": [
        ("How to Run a Game", 24, None),
        ("The Art of GMing", 28, "ref/rule/the-art-of-gming"),
        ("The Science of GMing", 34, "ref/rule/the-science-of-gming"),
        ("GM Considerations", 46, "ref/rule/gm-considerations"),
        ("Tools for Game Masters", 52, "ref/rule/tools-for-game-masters"),
    ],
    "Player Characters": [
        ("Starting Characters", 58, "ref/rule/starting-characters"),
        ("New Players", 63, "ref/rule/new-players"),
        ("The Life of a Party", 64, "ref/rule/the-life-of-a-party"),
        ("Player Interactions", 70, "ref/rule/player-interactions"),
    ],
    "Nonplayer Characters": [
        ("Cast of Characters", 80, "ref/rule/cast-of-characters"),
        ("Making NPCs Unique", 84, "ref/rule/making-npcs-unique"),
        ("Life of an NPC", 86, "ref/rule/life-of-an-npc"),
        ("NPC Boons", 88, "ref/rule/npc-boons"),
        ("Villains", 90, "ref/rule/villains"),
        ("NPC Creator's Toolbox", 94, "ref/rule/npc-creators-toolbox"),
    ],
    "Rewards": [
        ("The Role of Rewards", 102, "ref/rule/the-role-of-rewards"),
        ("Starting Treasure", 108, "ref/rule/starting-treasure"),
        ("What is a Reward?", 110, "ref/rule/what-is-a-reward"),
        ("PCs Controlling Rewards", 114, "ref/rule/pcs-controlling-rewards"),
        ("Random Item Generators", 118, "ref/rule/random-item-generators"),
    ],
    "Creating a World": [
        ("World Building", 140, "ref/rule/world-building"),
        ("Detailing Your World", 142, "ref/rule/detailing-your-world"),
        ("Geography", 146, "ref/rule/geography"),
        ("Cultural Considerations", 148, "ref/rule/cultural-considerations"),
        ("The Primitive Society", 150, "ref/rule/the-primitive-society"),
        ("The Feudal Society", 152, "ref/rule/the-feudal-society"),
        ("The Rural/Agrarian Society", 154, "ref/rule/the-rural-agrarian-society"),
        ("The Cosmopolitan Society", 156, "ref/rule/the-cosmopolitan-society"),
        ("Other Societies", 158, "ref/rule/other-societies"),
        ("Technology", 160, "ref/rule/technology"),
        ("Time", 162, "ref/rule/time"),
        ("The Cosmos", 164, "ref/rule/the-cosmos"),
        ("The Planes", 166, "ref/rule/the-planes-gg"),
        ("Parallel Worlds", 168, "ref/rule/parallel-worlds"),
    ],
    "Adventures": [
        ("Elements of Adventure", 172, "ref/rule/elements-of-adventure"),
        ("Dungeons", 174, "ref/rule/dungeons-gg"),
        ("Planar", 184, "ref/rule/planar"),
        ("Taverns", 198, "ref/rule/taverns"),
        ("Urban", 202, "ref/rule/urban"),
        ("Water", 214, "ref/rule/water"),
        ("Wilderness", 220, "ref/rule/wilderness-gg"),
    ],
    "Advanced Topics": [
        ("Customizing Your Game", 230, "ref/rule/customizing-your-game"),
        ("Chases", 232, "ref/rule/chases"),
        ("Disasters", 234, "ref/rule/disasters"),
        ("Drugs and Addiction", 236, "ref/rule/drugs-and-addiction"),
        ("Fortune-Telling", 238, "ref/rule/fortune-telling"),
        ("Gambling and Games of Chance", 240, "ref/rule/gambling-and-games-of-chance"),
        ("Haunts", 242, "ref/rule/haunts"),
        ("Hazards", 244, None),
        ("Mysteries and Investigations", 246, "ref/rule/mysteries-and-investigations"),
        ("Puzzles and Riddles", 248, "ref/rule/puzzles-and-riddles"),
        ("Sanity and Madness", 250, "ref/rule/sanity-and-madness"),
    ],
}

# Printed NPC Gallery roles: the group heading, the first page of its
# statistics, and the first page of the next role.
NPC_ROLES: list[tuple[str, int, int]] = [
    ("Adventurers", 256, 258),
    ("Brigands", 258, 260),
    ("City Watch", 260, 262),
    ("Coliseum", 262, 264),
    ("Criminals I", 264, 266),
    ("Criminals II", 266, 268),
    ("Crusaders", 268, 270),
    ("Dungeon", 270, 272),
    ("Entertainers", 272, 274),
    ("Fighting School", 274, 276),
    ("Frontier", 276, 278),
    ("Heretics", 278, 280),
    ("Marauders", 280, 282),
    ("Mercenaries", 282, 284),
    ("Merchants", 284, 286),
    ("Military", 286, 288),
    ("Nobles", 288, 290),
    ("Road", 290, 292),
    ("Royalty", 292, 294),
    ("Sailors", 294, 296),
    ("Scholars", 296, 298),
    ("Seers", 298, 300),
    ("Street", 300, 302),
    ("Tavern", 302, 304),
    ("Temple", 304, 306),
    ("Tribe", 306, 308),
    ("Villagers", 308, 310),
]

# Sanity and Madness keeps its printed subsections, and the insanity types
# stay under Types of Insanity.
SANITY_ID = "ref/rule/sanity-and-madness"
SANITY_SUBSECTIONS = [
    "ref/rule/rule-sanity-and-madness/curing-insanity",
    "ref/rule/rule-sanity-and-madness/types-of-insanity",
]
TYPES_OF_INSANITY_ID = "ref/rule/rule-sanity-and-madness/types-of-insanity"
INSANITY_TYPES = [
    "ref/madness/amnesia",
    "ref/madness/dissociated-identity",
    "ref/madness/mania",
    "ref/madness/moral-insanity",
    "ref/madness/paranoid",
    "ref/madness/phobia",
    "ref/madness/schizophrenia",
]

# The NPC Gallery opens with the compiled boons section and the NPC table
# before the role groups.
NPC_OPENERS = [
    "ref/rule/rule-npc-gallery/npc-boons",
    "ref/rule/list-of-npcs",
]

# Entries the tree keeps under a heading that is not a chapter section.
EXPLICIT = set(SANITY_SUBSECTIONS) | set(INSANITY_TYPES) | set(NPC_OPENERS)


# ---------------------------------------------------------------------------
# Records that only the retired dataset-topic tree placed.
#
# They carry no GameMastery Guide page line, so page order cannot place them.
# Each sits at the end of the printed section whose title its dataset group
# names, or under the printed heading it fills. This is a placement by printed
# section, never by a guessed page.

HAZARDS = "hazards"
ROLE = "role:"
NPC_PREFIX = "ref/carried/game-mastery-guide/monsters/"

# The dataset's NPC folder names are the printed NPC Gallery role groups.
NPC_ROLE_BY_FOLDER = {
    "adventurers": "Adventurers",
    "brigands": "Brigands",
    "city-watch": "City Watch",
    "coliseum": "Coliseum",
    "criminals-i": "Criminals I",
    "criminals-ii": "Criminals II",
    "crusaders": "Crusaders",
    "dungeon": "Dungeon",
    "entertainers": "Entertainers",
    "fighting-school": "Fighting School",
    "frontier": "Frontier",
    "heretics": "Heretics",
    "marauders": "Marauders",
    "mercenaries": "Mercenaries",
    "merchants": "Merchants",
    "military": "Military",
    "nobles": "Nobles",
    "road": "Road",
    "royalty": "Royalty",
    "sailors": "Sailors",
    "scholars": "Scholars",
    "seers": "Seers",
    "street": "Street",
    "tavern": "Tavern",
    "temple": "Temple",
    "tribe": "Tribe",
    "villagers": "Villagers",
}

PSRD_TARGETS: dict[str, str] = {
    # Drugs and Addiction (printed p. 236).
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/addiction/minor-addiction": "ref/rule/addiction",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/addiction/moderate-addiction": "ref/rule/addiction",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/addiction/severe-addiction": "ref/rule/addiction",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/aether": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/dwarven-fire-ale": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/elven-absinthe": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/flayleaf": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/opium": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/pesh": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/scour": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/shiver": "ref/rule/drugs",
    "ref/carried/game-mastery-guide/rules/drugs-and-addiction/sample-drugs/zerk": "ref/rule/drugs",
    # Hazards (printed p. 244). The first record fills the printed heading.
    "ref/carried/game-mastery-guide/rules/hazards": HAZARDS,
    "ref/carried/game-mastery-guide/rules/hazards/hazards": HAZARDS,
    "ref/carried/game-mastery-guide/rules/hazards/sample-hazards/ear-seekers/ear-seekers": HAZARDS,
    "ref/carried/game-mastery-guide/rules/hazards/sample-hazards/poison-oak/poison-oak": HAZARDS,
    "ref/carried/game-mastery-guide/rules/hazards/sample-hazards/poison-oak/poison-oak-severe-exposure": HAZARDS,
    "ref/carried/game-mastery-guide/rules/hazards/sample-hazards/rot-grubs/rot-grubs": HAZARDS,
    # Sanity and Madness (printed p. 250).
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/going-insane": "ref/rule/sanity-and-madness",
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/types-of-insanity/amnesia": TYPES_OF_INSANITY_ID,
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/types-of-insanity/mania-phobia": TYPES_OF_INSANITY_ID,
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/types-of-insanity/multiple-personality-disorder": TYPES_OF_INSANITY_ID,
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/types-of-insanity/paranoia": TYPES_OF_INSANITY_ID,
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/types-of-insanity/psychosis": TYPES_OF_INSANITY_ID,
    "ref/carried/game-mastery-guide/rules/sanity-and-madness/types-of-insanity/schizophrenia": TYPES_OF_INSANITY_ID,
    # Printed sections that the dataset group names.
    "ref/carried/game-mastery-guide/rules/npcs": "ref/rule/npc-gallery",
    "ref/carried/game-mastery-guide/rules/planar-adventures": "ref/rule/planar",
    "ref/carried/game-mastery-guide/rules/settlements": "ref/rule/urban",
    # The Gamemastery hub prints with the Core Rulebook; in this book it opens the
    # Introduction, as its old page-order node did.
    "ref/rule/gamemastery": "ref/rule/gamemastery-guide",
}


def psrd_target(entry_id: str) -> str | None:
    """Where a record that only the retired tree placed belongs."""
    if entry_id.startswith(NPC_PREFIX):
        folder = entry_id[len(NPC_PREFIX) :].split("/", 1)[0]
        title = NPC_ROLE_BY_FOLDER.get(folder)
        if title is not None:
            return ROLE + title
    return PSRD_TARGETS.get(entry_id)


# ---------------------------------------------------------------------------
# The transform


def attach_psrd(book: dict, chapters: dict[str, dict], pool: dict[str, dict]) -> None:
    """Place each record that only the retired dataset-topic tree held.

    A record sits at the end of the printed section its dataset group names.
    Sorting by title keeps the result independent of the old tree order, so the
    check is stable.
    """
    pending: dict[str, list[dict]] = {}
    for entry_id in list(pool):
        target = psrd_target(entry_id)
        if target is None:
            continue
        pending.setdefault(target, []).append(pool.pop(entry_id))

    for target, nodes in pending.items():
        nodes.sort(key=lambda node: node["title"].casefold())
        ids = {node["entry"] for node in nodes}
        if target == HAZARDS:
            group = next(
                (
                    node
                    for chapter in chapters.values()
                    for node in flatten(chapter)
                    if node.get("title") == "Hazards" and node.get("entry") is None
                ),
                None,
            )
            if group is None:
                raise SystemExit("missing printed Hazards heading")
            hazard_rule = next(
                (
                    node
                    for node in nodes
                    if node["entry"]
                    == "ref/carried/game-mastery-guide/rules/hazards"
                ),
                nodes[0],
            )
            group["entry"] = hazard_rule["entry"]
            group["children"] = [
                child
                for child in group.get("children", [])
                if child.get("entry") not in ids
            ]
            group["children"].extend(
                node for node in nodes if node is not hazard_rule
            )
            continue
        if target.startswith(ROLE):
            title = target[len(ROLE) :]
            gallery = chapters["NPC Gallery"]
            parent = next(
                (node for node in gallery["children"] if node["title"] == title),
                None,
            )
            if parent is None:
                raise SystemExit(f"missing printed NPC role group {title}")
        else:
            parent = find_node(book, target)
            if parent is None:
                raise SystemExit(f"missing placement target {target}")
        parent["children"] = [
            child
            for child in parent.get("children", [])
            if child.get("entry") not in ids
        ]
        parent["children"].extend(nodes)


def transform(
    book: dict, corpus: Path, extras: list[dict] | None = None
) -> list[tuple[str, str, str, int | None]]:
    before_paths: dict[str, str] = {}
    for chapter in book["chapters"]:
        for node in flatten(chapter):
            if node.get("entry"):
                before_paths[node["entry"]] = path_of(book, node)
    for node in extras or []:
        before_paths[node["entry"]] = "(retired tree)"

    chapters = {chapter["title"]: chapter for chapter in book["chapters"]}
    pool: dict[str, dict] = {}
    held: dict[str, dict] = {}
    for chapter in book["chapters"]:
        chapter_entry = chapter.get("entry")
        for child in flatten(chapter)[1:]:
            entry_id = child.get("entry")
            if not entry_id or entry_id == chapter_entry:
                continue
            if entry_id in EXPLICIT:
                held[entry_id] = child
                continue
            pool[entry_id] = child
    for node in extras or []:
        pool[node["entry"]] = node

    # Build the eight printed chapters from their section headings.
    for index, (chapter_title, _start) in enumerate(CHAPTERS):
        if chapter_title == "NPC Gallery":
            continue
        chapter = chapters[chapter_title]
        chapter.pop("children", None)
        chapter["children"] = []
        sections = SECTIONS[chapter_title]
        chapter_end = CHAPTERS[index + 1][1]
        for section_index, (title, start, heading_id) in enumerate(sections):
            if heading_id is not None:
                heading = pool.pop(heading_id, None)
                if heading is None:
                    raise SystemExit(f"missing section heading {heading_id}")
                heading.pop("children", None)
            else:
                heading = {"title": title}
            end = sections[section_index + 1][1] if section_index + 1 < len(sections) else chapter_end
            members = [
                (entry_id, node)
                for entry_id, node in pool.items()
                if (page := page_of(corpus, entry_id)) is not None
                and start <= page < end
            ]
            members.sort(key=lambda item: page_of(corpus, item[0]))
            for entry_id, _node in members:
                del pool[entry_id]
            heading["children"] = [node for _entry_id, node in members]
            chapter["children"].append(heading)

    # Sanity and Madness keeps its printed subsections.
    sanity = find_node(book, SANITY_ID)
    if sanity is None:
        raise SystemExit("missing Sanity and Madness")
    subsections = [held.pop(entry_id) for entry_id in SANITY_SUBSECTIONS]
    for node in subsections:
        node.pop("children", None)
    types = next(
        node
        for node in subsections
        if node.get("entry") == TYPES_OF_INSANITY_ID
    )
    types["children"] = [held.pop(entry_id) for entry_id in INSANITY_TYPES]
    sanity["children"] = subsections + sanity.get("children", [])

    # NPC Gallery: openers, then the printed role groups.
    gallery = chapters["NPC Gallery"]
    gallery.pop("children", None)
    gallery["children"] = [held.pop(entry_id) for entry_id in NPC_OPENERS]
    for title, start, end in NPC_ROLES:
        group = {"title": title}
        members = [
            (entry_id, node)
            for entry_id, node in pool.items()
            if (page := page_of(corpus, entry_id)) is not None
            and start <= page < end
        ]
        members.sort(key=lambda item: page_of(corpus, item[0]))
        for entry_id, _node in members:
            del pool[entry_id]
        group["children"] = [node for _entry_id, node in members]
        gallery["children"].append(group)

    # Records that only the retired tree held, at their printed section.
    attach_psrd(book, chapters, pool)

    if pool:
        leftovers = ", ".join(sorted(pool))
        raise SystemExit(f"unplaced entries: {leftovers}")
    if held:
        leftovers = ", ".join(sorted(held))
        raise SystemExit(f"unplaced held entries: {leftovers}")

    after_paths: dict[str, str] = {}
    for chapter in book["chapters"]:
        for node in flatten(chapter):
            if node.get("entry"):
                after_paths[node["entry"]] = path_of(book, node)

    if set(before_paths) != set(after_paths):
        missing = sorted(set(before_paths) - set(after_paths))
        added = sorted(set(after_paths) - set(before_paths))
        raise SystemExit(f"entry set changed: lost {missing}, added {added}")

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
    extras = merge_duplicate_book(books)
    book = find_book(books)
    before = book_entries(book) | {node["entry"] for node in extras}
    moves = transform(book, args.corpus, extras)
    after = book_entries(book)
    if before != after:
        print(f"{BOOK}: entry set changed", file=sys.stderr)
        for entry in sorted(before - after):
            print(f"  lost: {entry}", file=sys.stderr)
        for entry in sorted(after - before):
            print(f"  added: {entry}", file=sys.stderr)
        return 1

    book["basis"] = (
        "The chapter titles, their order, and their first pages are checked "
        "against the printed contents page of the GameMastery Guide. Each "
        "chapter holds its printed section headings in page order. The corpus "
        "imported the same book twice; the second tree is merged into this "
        "publication id, and its records without a printed page sit at the end "
        "of the printed section their dataset group names. See "
        "docs/research/book-order-evidence.md. Canonical ids were substituted "
        "through the reviewed alias map; chapters and order are unchanged."
    )
    book["review"] = [
        "Nested placement follows the printed section headings; entries that share a printed page keep the dataset order.",
        "The records that only the retired page-order tree placed carry no GameMastery Guide page; they sit at the end of their printed section.",
        "The printed Appendix (310) has no corpus entries.",
    ]

    text = dump_books(books)
    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the nested Game Mastery Guide shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {BOOK}; entries placed: {len(after)}")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} moves, {len(after)} entries placed)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
