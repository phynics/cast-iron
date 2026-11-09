#!/usr/bin/env python3
"""Check and fix the nested placement of the Advanced Race Guide tree.

The printed Advanced Race Guide groups each chapter into races and each race
into sections, and orders the entries inside them by page. This script moves
existing nodes inside the ``advanced-race-guide`` book of ``books.toml``; it
never changes an entry id, an entry file, or an anchor.

It also places the ARG-primary entries of the residual
``New entries (page order)`` group in their printed sections: the Race Builder
qualities and traits, each race's racial archetypes, and the racial equipment,
feats, magic items, and spells. Multi-source race hubs stay residual by
decision; secondary-source and cross-book entries stay residual.

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
import re
import sys
import tomllib
from pathlib import Path

BOOKS_FILE = "books.toml"
SOURCE = "advanced-race-guide"

# Nodes that keep their chapter but move within it.
# (parent title, node title, where, reference title)
WITHIN_CHAPTER = [
    # The printed chapter reads Dwarves, Elves, Gnomes, Half-Elves, Half-Orcs,
    # Halflings, Humans (pp. 10-79).
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

# Printed race section ranges, from the contents page and the race headings.
# (race node title, first printed page, last printed page)
RACE_PAGES = [
    ("Dwarves", 10, 19),
    ("Elves", 20, 29),
    ("Gnomes", 30, 39),
    ("Half-Elves", 40, 49),
    ("Half-Orcs", 50, 59),
    ("Halflings", 60, 69),
    ("Humans", 70, 79),
    ("Aasimars", 84, 89),
    ("Catfolk", 90, 95),
    ("Dhampirs", 96, 101),
    ("Drow", 102, 107),
    ("Fetchlings", 108, 113),
    ("Goblins", 114, 119),
    ("Hobgoblins", 120, 125),
    ("Ifrits", 126, 131),
    ("Kobolds", 132, 137),
    ("Orcs", 138, 143),
    ("Oreads", 144, 149),
    ("Ratfolk", 150, 155),
    ("Sylphs", 156, 161),
    ("Tengus", 162, 167),
    ("Tieflings", 168, 173),
    ("Undines", 174, 179),
    ("Changelings", 184, 185),
    ("Duergar", 186, 187),
    ("Gillmen", 188, 189),
    ("Gripplis", 190, 191),
    ("Kitsune", 192, 193),
    ("Merfolk", 194, 195),
    ("Nagaji", 196, 197),
    ("Samsarans", 198, 199),
    ("Strix", 200, 201),
    ("Sulis", 202, 203),
    ("Svirfneblin", 204, 205),
    ("Vanaras", 206, 207),
    ("Vishkanyas", 208, 209),
    ("Wayangs", 210, 211),
]

# Race-tree labels that differ from the printed adjective.
RACE_ADJECTIVE = {
    "Dwarves": "Dwarven",
    "Elves": "Elven",
    "Gnomes": "Gnome",
    "Half-Elves": "Half-Elven",
    "Half-Orcs": "Half-Orc",
    "Halflings": "Halfling",
    "Humans": "Human",
    "Aasimars": "Aasimar",
    "Dhampirs": "Dhampir",
    "Fetchlings": "Fetchling",
    "Goblins": "Goblin",
    "Hobgoblins": "Hobgoblin",
    "Ifrits": "Ifrit",
    "Kobolds": "Kobold",
    "Orcs": "Orc",
    "Oreads": "Oread",
    "Sylphs": "Sylph",
    "Tengus": "Tengu",
    "Tieflings": "Tiefling",
    "Undines": "Undine",
    "Changelings": "Changeling",
    "Gillmen": "Gillman",
    "Gripplis": "Grippli",
    "Samsarans": "Samsaran",
    "Vanaras": "Vanara",
    "Vishkanyas": "Vishkanya",
    "Wayangs": "Wayang",
}

# The printed Race Builder Step 2 (p. 215) reads Type Quality, then the type
# groups, then Base Speed, Size, Ability Score, and Language Quality
# (pp. 215-219). Entries that share a page keep the dataset order.
STEP2 = [
    ("Type Quality", ["Mid-RP Types", "High-RP Types", "Low-RP Types"]),
    ("Base Speed Quality", []),
    ("Size Quality", []),
    ("Ability Score Quality", []),
    ("Language Quality", []),
]

# The printed Step 3 (p. 220) runs nine trait categories, each with its
# standard, advanced, and monstrous sections (pp. 220-239).
STEP3 = [
    ("Ability Score Racial Traits", ["Advanced Ability Score Traits"]),
    (
        "Defense Racial Traits",
        ["Standard Defense Traits", "Advanced Defense Traits", "Monstrous Defense Traits"],
    ),
    ("Feat and Skill Traits", ["Standard Feat and Skill Traits", "Advanced Feat and Skill Traits"]),
    (
        "Magical Racial Traits",
        ["Standard Magical Traits", "Advanced Magical Traits", "Monstrous Magical Traits"],
    ),
    (
        "Movement Racial Traits",
        ["Standard Movement Traits", "Advanced Movement Traits", "Monstrous Movement Traits"],
    ),
    (
        "Offensive Racial Traits",
        ["Standard Offensive Traits", "Advanced Offensive Traits", "Monstrous Offensive Traits"],
    ),
    (
        "Senses Racial Traits",
        ["Standard Senses Traits", "Advanced Senses Traits", "Monstrous Senses Traits"],
    ),
    ("Weakness Racial Traits", ["Standard Weakness Traits", "Advanced Weakness Traits"]),
    (
        "Other Racial Traits",
        ["Standard Other Traits", "Advanced Other Traits", "Monstrous Other Traits"],
    ),
]

RACE_BUILDER = [title for title, _ in STEP2] + [child for _, children in STEP2 for child in children]
RACE_BUILDER += [title for title, _ in STEP3] + [child for _, children in STEP3 for child in children]

# ARG-primary records that print already carries as a base entry. They are
# nested under that base entry; a later duplicate pass decides whether to merge.
VARIANT_PARENT = {
    "ref/eq-misc/tear-away-dilettantes-outfit": "ref/eq-misc/dilettantes-outfit",
    "ref/eq-misc/sherrifs-whistle": "ref/carried/advanced-race-guide/races/gnomes/new-racial-rules/gnome-equipment/sheriffs-whistle",
    "ref/eq-weapon/shadowcloy-flask": "ref/carried/advanced-race-guide/races/fetchlings/new-racial-rules/fetchling-equipment/shadowcloy",
    "ref/eq-misc/fetters-common": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-equipment/fetters",
    "ref/eq-misc/fetters-masterwork": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-equipment/fetters",
    "ref/eq-misc/manacles-with-barbs-common": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-equipment/manacle-barbs",
    "ref/eq-misc/manacles-with-barbs-masterwork": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-equipment/manacle-barbs",
    "ref/eq-misc/manacles-with-barbs-mithral": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-equipment/manacle-barbs",
    "ref/eq-misc/manacles-with-barbs-quickcatch": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-equipment/manacle-barbs",
    "ref/magic-wondrous/hobgoblin-battle-standard-despair": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-magic-items/hobgoblin-battle-standard",
    "ref/magic-wondrous/hobgoblin-battle-standard-ferocity": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-magic-items/hobgoblin-battle-standard",
    "ref/magic-wondrous/hobgoblin-battle-standard-iron-resolve": "ref/carried/advanced-race-guide/races/hobgoblins/new-racial-rules/hobgoblin-magic-items/hobgoblin-battle-standard",
    "ref/eq-weapon/alchemical-coal-blinding-cinders": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/alchemical-coal",
    "ref/eq-weapon/alchemical-coal-choking-smoke": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/alchemical-coal",
    "ref/eq-weapon/kobold-tail": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/kobold-tail-attachments",
    "ref/eq-weapon/kobold-tail-long-lash": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/kobold-tail-attachments",
    "ref/eq-weapon/kobold-tail-pounder": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/kobold-tail-attachments",
    "ref/eq-weapon/kobold-tail-razored": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/kobold-tail-attachments",
    "ref/eq-weapon/kobold-tail-spiked": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/kobold-tail-attachments",
    "ref/eq-weapon/kobold-tail-sweeper": "ref/carried/advanced-race-guide/races/kobolds/new-racial-rules/kobold-equipment/kobold-tail-attachments",
    "ref/eq-misc/fleshgem-decorative": "ref/carried/advanced-race-guide/races/oreads/new-racial-rules/oread-equipment/fleshgem",
    "ref/eq-misc/fleshgem-spikes": "ref/carried/advanced-race-guide/races/oreads/new-racial-rules/oread-equipment/fleshgem",
    "ref/eq-misc/stink-ink-common": "ref/carried/advanced-race-guide/races/ratfolk/new-racial-rules/ratfolk-equipment/stink-ink",
    "ref/eq-weapon/tailblade-ratfolk": "ref/carried/advanced-race-guide/races/ratfolk/new-racial-rules/ratfolk-equipment/tailblade",
    "ref/magic-wondrous/kitsune-star-gem-1st": "ref/carried/advanced-race-guide/races/kitsune/new-racial-rules/kitsune-magic-items/kitsune-star-gem",
    "ref/magic-wondrous/kitsune-star-gem-2nd": "ref/carried/advanced-race-guide/races/kitsune/new-racial-rules/kitsune-magic-items/kitsune-star-gem",
    "ref/magic-wondrous/kitsune-star-gem-3rd": "ref/carried/advanced-race-guide/races/kitsune/new-racial-rules/kitsune-magic-items/kitsune-star-gem",
    "ref/eq-misc/shadow-stencil-set": "ref/carried/advanced-race-guide/races/wayangs/new-racial-rules/wayang-equipment/shadow-stencils",
    "ref/spell/binding-earth-mass": "ref/carried/advanced-race-guide/races/oreads/new-racial-rules/oread-spells/binding-earth/mass",
    "ref/spell/blessing-of-luck-and-resolve-mass": "ref/carried/advanced-race-guide/races/halflings/new-racial-rules/halfling-spells/blessing-of-luck-and-resolve/mass",
}

# ARG-primary class options print inside a per-race section. The last path
# element is an existing archetype entry or a group created below.
CLASS_OPTION_TARGET = {
    "ref/companion/carnivorous-flower": ("Elves", "Treesinger (Druid)"),
    "ref/companion/crawling-vine": ("Elves", "Treesinger (Druid)"),
    "ref/companion/puffball": ("Elves", "Treesinger (Druid)"),
    "ref/companion/treant-sapling": ("Elves", "Treesinger (Druid)"),
    "ref/order/order-of-the-paw": ("Halflings", "Racial Archetypes"),
    "ref/sorcbloodline/imperious": ("Humans", "Racial Archetypes & Bloodlines"),
    "ref/sorcbloodline/kobold-sorcerer": ("Kobolds", "Racial Archetypes"),
    "ref/discovery/defoliant-bomb": ("Half-Elves", "Bramble Brewer"),
    "ref/discovery/chameleon": ("Gripplis", "Grippli Discoveries"),
    "ref/discovery/deadly-excretions": ("Gripplis", "Grippli Discoveries"),
    "ref/discovery/underwater-demolition": ("Gripplis", "Grippli Discoveries"),
    "ref/discovery/darkness-bomb": ("Svirfneblin", "Svirfneblin Discoveries"),
    "ref/discovery/glassfoot-bomb": ("Svirfneblin", "Svirfneblin Discoveries"),
    "ref/discovery/fire-brand": ("Goblins", "Goblin Discoveries"),
    "ref/discovery/rag-doll-mutagen": ("Goblins", "Goblin Discoveries"),
    "ref/discovery/rocket-bomb": ("Goblins", "Goblin Discoveries"),
    "ref/discovery/scrap-bomb": ("Goblins", "Goblin Discoveries"),
    "ref/discovery/greater-plague-bomb": ("Ratfolk", "Ratfolk Discoveries"),
    "ref/discovery/lingering-plague": ("Ratfolk", "Ratfolk Discoveries"),
    "ref/talent/deadly-scratch": ("Catfolk", "Catfolk Rogue Talents"),
    "ref/talent/disarming-luck": ("Catfolk", "Catfolk Rogue Talents"),
    "ref/talent/graceful-faller": ("Catfolk", "Catfolk Rogue Talents"),
    "ref/talent/nimble-climber-catfolk": ("Catfolk", "Catfolk Rogue Talents"),
    "ref/talent/single-minded-appraiser": ("Catfolk", "Catfolk Rogue Talents"),
    "ref/talent/vicious-claws": ("Catfolk", "Catfolk Rogue Talents"),
    "ref/talent/false-friend": ("Kitsune", "Kitsune Trickster"),
    "ref/talent/obfuscate-story": ("Kitsune", "Kitsune Trickster"),
    "ref/talent/steal-the-story": ("Kitsune", "Kitsune Trickster"),
    "ref/evolution/shadow-blend": ("Fetchlings", "Shadow Caller"),
    "ref/evolution/shadow-form": ("Fetchlings", "Shadow Caller"),
}

# ARG-primary records that print already carries as a chapter or race entry.
# They stay residual and are reported as duplicates for a follow-up.
DUPLICATE_LEFT = {
    "ref/rule/core-races",
    "ref/rule/less-common-races",
}

# Multi-source race records and their standard racial traits. The chapter
# already holds the printed race section through the carried race entry.
MULTI_SOURCE = ("ref/race/", "ref/racial-trait/", "ref/rule/race-")


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


def child(container: dict, title: str):
    for candidate in container.get("children", []):
        if candidate["title"] == title:
            return candidate
    return None


def find_entry(container: dict, entry_id: str):
    for index, candidate in enumerate(container.get("children", [])):
        if candidate.get("entry") == entry_id:
            return container, index
        found = find_entry(candidate, entry_id)
        if found is not None:
            return found
    return None


def take(container: dict, entry_id: str):
    """Remove and return the node with this entry id, or None."""
    found = find_entry(container, entry_id)
    if found is None:
        return None
    parent, index = found
    return parent["children"].pop(index)


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


def reorder(parent: dict, titles: list[str]) -> None:
    by_title = {candidate["title"]: candidate for candidate in parent.get("children", [])}
    head = [by_title[title] for title in titles if title in by_title]
    rest = [candidate for candidate in parent.get("children", []) if candidate["title"] not in titles]
    parent["children"] = head + rest


def ensure_group(
    parent: dict,
    title: str,
    anchor: str,
    *,
    before: str | None = None,
) -> dict:
    existing = child(parent, title)
    if existing is not None:
        return existing
    group = {"title": title, "anchor": anchor, "children": []}
    children = parent.setdefault("children", [])
    index = len(children)
    if before is not None:
        index = next((i for i, item in enumerate(children) if item["title"] == before), index)
    children.insert(index, group)
    return group


# ---------------------------------------------------------------------------
# Entry evidence


ARG_PAGE = re.compile(r"Advanced Race Guide(?: pg\.?| p\.) ?(\d+)")
SOURCES = re.compile(r"^\*\*Sources:\*\* (.+?)(?:\\|$)", re.M | re.DOTALL)


def entry_path(corpus: Path, entry_id: str) -> Path:
    return corpus / "entries" / (entry_id[len("ref/") :] + ".md")


def front_matter(corpus: Path, entry_id: str) -> dict:
    text = entry_path(corpus, entry_id).read_text(encoding="utf-8")
    return tomllib.loads(text.split("+++", 2)[1])


def sources_line(corpus: Path, entry_id: str) -> str:
    text = entry_path(corpus, entry_id).read_text(encoding="utf-8")
    match = SOURCES.search(text)
    return " ".join(match.group(1).split()) if match else ""


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    match = ARG_PAGE.search(entry_path(corpus, entry_id).read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


def kind_of(corpus: Path, entry_id: str) -> str:
    return front_matter(corpus, entry_id)["kind"]


def is_arg_primary(corpus: Path, entry_id: str) -> bool:
    line = sources_line(corpus, entry_id)
    if line.startswith("Advanced Race Guide"):
        return True
    if line:
        return False
    front = front_matter(corpus, entry_id)
    return front.get("source_ids") == ["advanced-race-guide"] and front.get("kind") in {
        "class_option",
        "affliction",
    }


def race_for_page(page: int | None) -> str | None:
    if page is None:
        return None
    for title, first, last in RACE_PAGES:
        if first <= page <= last:
            return title
    return None


def insert_by_page(container: dict, node_data: dict, page: int | None, corpus: Path) -> None:
    children = container.setdefault("children", [])
    index = len(children)
    if page is not None:
        for i, candidate in enumerate(children):
            candidate_page = page_of(corpus, candidate.get("entry"))
            if candidate_page is not None and candidate_page > page:
                index = i
                break
    children.insert(index, node_data)


def nrr_section(race: dict, suffix: str, adjective: str) -> dict:
    new_rules = child(race, "New Racial Rules")
    if new_rules is None:
        raise SystemExit(f"{SOURCE}: missing New Racial Rules for {race['title']}")
    for candidate in new_rules.get("children", []):
        if candidate["title"].endswith(suffix):
            return candidate
    title = f"{adjective}{suffix}"
    order = [" Equipment", " Special Materials", " Feats", " Magic Items", " Spells"]
    rank = order.index(suffix)
    before = None
    for candidate in new_rules.get("children", []):
        for index, section in enumerate(order):
            if candidate["title"].endswith(section) and index > rank:
                before = candidate["title"]
                break
        if before is not None:
            break
    return ensure_group(new_rules, title, title.lower().replace(" ", "-"), before=before)


def race_archetypes(race_title: str) -> dict:
    race = node(root(_book_data), race_title)
    title = "Racial Archetypes & Bloodlines" if race_title == "Humans" else "Racial Archetypes"
    anchor = "racial-archetypes-bloodlines" if race_title == "Humans" else "racial-archetypes"
    return ensure_group(race, title, anchor, before="New Racial Rules")


def race_discoveries(race_title: str, title: str) -> dict:
    race = node(root(_book_data), race_title)
    return ensure_group(race, title, title.lower().replace(" ", "-"), before="New Racial Rules")


# ---------------------------------------------------------------------------
# Transforms

_book_data: dict = {}


def fix_nested(book_data: dict, corpus: Path) -> list[dict]:
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

    residual = node(root(book_data), "New entries (page order)")

    def place(node_data: dict, target: dict, page: int | None, section: str) -> None:
        insert_by_page(target, node_data, page, corpus)
        moves.append(
            {
                "node": node_data["title"],
                "old": "New entries (page order)",
                "new": section,
                "page": page,
            }
        )

    def take_entry(entry_id: str) -> dict | None:
        return take(root(book_data), entry_id)

    # 1. Race Builder: Step 2 qualities and Step 3 trait categories.
    step2 = node(root(book_data), "Step 2: Racial Qualities")
    step3 = node(root(book_data), "Step 3: Racial Traits")
    for title, children in STEP2:
        holder = child(step2, title)
        if holder is None:
            holder = take_entry_by_title(root(book_data), title)
        if holder is None:
            raise SystemExit(f"{SOURCE}: missing Race Builder section: {title}")
        if child(step2, title) is None:
            step2.setdefault("children", []).append(holder)
        for child_title in children:
            moved = take_entry_by_title(root(book_data), child_title)
            if moved is None:
                raise SystemExit(f"{SOURCE}: missing Race Builder subsection: {child_title}")
            holder.setdefault("children", []).append(moved)
        reorder(holder, children)
    reorder(step2, ["Racial Qualities"] + [title for title, _ in STEP2])

    for title, children in STEP3:
        holder = child(step3, title) or take_entry_by_title(root(book_data), title)
        if holder is None:
            raise SystemExit(f"{SOURCE}: missing Race Builder category: {title}")
        if child(step3, title) is None:
            step3.setdefault("children", []).append(holder)
        for child_title in children:
            moved = take_entry_by_title(root(book_data), child_title)
            if moved is None:
                raise SystemExit(f"{SOURCE}: missing Race Builder subsection: {child_title}")
            holder.setdefault("children", []).append(moved)
        reorder(holder, children)
    reorder(step3, ["Racial Traits"] + [title for title, _ in STEP3])

    # 2. Variant records nest under the base entry that print already places.
    for entry_id, base_id in VARIANT_PARENT.items():
        moved = take_entry(entry_id)
        if moved is None:
            continue
        base = find_entry(root(book_data), base_id)
        if base is None:
            raise SystemExit(f"{SOURCE}: missing variant base: {base_id}")
        base[0]["children"][base[1]].setdefault("children", []).append(moved)
        moves.append(
            {
                "node": moved["title"],
                "old": "New entries (page order)",
                "new": f"under {base_id.rsplit('/', 1)[-1]}",
                "page": page_of(corpus, entry_id),
            }
        )

    # 3. ARG-primary archetypes, equipment, feats, magic items, and spells.
    for node_data in list(residual.get("children", [])):
        entry_id = node_data.get("entry")
        if not entry_id:
            continue
        if entry_id in VARIANT_PARENT or entry_id in CLASS_OPTION_TARGET:
            continue
        if entry_id in DUPLICATE_LEFT or entry_id in RACE_BUILDER:
            continue
        if entry_id.startswith(MULTI_SOURCE):
            continue
        if not is_arg_primary(corpus, entry_id):
            continue
        kind = kind_of(corpus, entry_id)
        page = page_of(corpus, entry_id)
        race_title = race_for_page(page)
        if race_title is None:
            continue

        if kind == "archetype":
            moved = take_entry(entry_id)
            if moved is None:
                continue
            target = race_archetypes(race_title)
            place(moved, target, page, f"{race_title} / Racial Archetypes")
        elif kind in {"item", "feat", "spell"}:
            moved = take_entry(entry_id)
            if moved is None:
                continue
            race = node(root(book_data), race_title)
            adjective = RACE_ADJECTIVE.get(race_title, race_title)
            if kind == "item":
                suffix = " Equipment" if entry_id.startswith("ref/eq-") else " Magic Items"
            elif kind == "feat":
                suffix = " Feats"
            else:
                suffix = " Spells"
            target = nrr_section(race, suffix, adjective)
            place(moved, target, page, f"{race_title} / New Racial Rules / {target['title']}")

    # 4. Class options print inside their race's archetype or discovery section.
    for entry_id, (race_title, target_title) in CLASS_OPTION_TARGET.items():
        moved = take_entry(entry_id)
        if moved is None:
            continue
        race = node(root(book_data), race_title)
        target = child(race, target_title)
        archetype_group = target_title in {"Racial Archetypes", "Racial Archetypes & Bloodlines"}
        if target is None:
            archetypes = child(race, "Racial Archetypes") or child(
                race, "Racial Archetypes & Bloodlines"
            )
            if archetypes is not None:
                target = child(archetypes, target_title)
        if target is None:
            if archetype_group:
                target = race_archetypes(race_title)
            elif target_title == "Catfolk Rogue Talents":
                target = ensure_group(
                    race,
                    target_title,
                    "catfolk-rogue-talents",
                    before="Racial Archetypes",
                )
            elif target_title.endswith("Discoveries"):
                target = race_discoveries(race_title, target_title)
            elif target_title == "Treesinger (Druid)":
                archetypes = race_archetypes(race_title)
                target = ensure_group(archetypes, target_title, "treesinger-druid")
            else:
                raise SystemExit(f"{SOURCE}: missing class-option target: {target_title}")
        page = page_of(corpus, entry_id)
        if archetype_group:
            insert_by_page(target, moved, page, corpus)
        else:
            target.setdefault("children", []).append(moved)
        moves.append(
            {
                "node": moved["title"],
                "old": "New entries (page order)",
                "new": f"{race_title} / {target['title']}",
                "page": page,
            }
        )

    return moves



def take_entry_by_title(container: dict, title: str):
    found = find(container, title)
    if found is None:
        return None
    parent, index = found
    return parent["children"].pop(index)


def main() -> int:
    global _book_data
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
    _book_data = book_data
    before = book_entries(book_data)

    moves = fix_nested(book_data, args.corpus)
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
