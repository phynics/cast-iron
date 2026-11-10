#!/usr/bin/env python3
"""Place the Monster Codex tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/monster-codex-nested.py /path/to/cast-iron-data
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

BOOK = "monster-codex"
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


def parent_of(book: dict, node: dict) -> dict | None:
    def walk(current: dict) -> dict | None:
        if node in current.get("children", []):
            return current
        for child in current.get("children", []):
            found = walk(child)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        if chapter is node:
            return None
        found = walk(chapter)
        if found is not None:
            return found
    return None


def detach(book: dict, entry_id: str) -> dict:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")
    parent = parent_of(book, node)
    if parent is None:
        raise SystemExit(f"entry {entry_id} is already a chapter")
    parent["children"].remove(node)
    return node


def attach(parent: dict, node: dict) -> None:
    parent.setdefault("children", []).append(node)


def move(book: dict, entry_id: str, target: dict) -> None:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")
    if parent_of(book, node) is target:
        return
    attach(target, detach(book, entry_id))


def chapter(book: dict, title: str) -> dict:
    for node in book["chapters"]:
        if node["title"] == title:
            return node
    raise SystemExit(f"missing chapter {title}")


def resolve_path(root: dict, path: list[str]) -> dict:
    """Find a title path, creating a bare heading when it is missing."""
    node = root
    for title in path:
        child = next((c for c in node.get("children", []) if c["title"] == title), None)
        if child is None:
            child = {"title": title, "children": []}
            node.setdefault("children", []).append(child)
        node = child
    return node


def reorder(parent: dict, order: list[str]) -> None:
    by_title: dict[str, dict] = {}
    for child in parent.get("children", []):
        by_title.setdefault(child["title"], child)
    head = [by_title[title] for title in order if title in by_title]
    parent["children"] = head + [
        child for child in parent.get("children", []) if child not in head
    ]


# ---------------------------------------------------------------------------
# Page evidence

MONSTER_CODEX_PAGE = re.compile(r"Monster Codex\s*(?:pg\.?|p\.?|/)\s*(\d+)")

# Entries whose source line carries no page or names another book, but whose
# printed section is in the Monster Codex.
PAGE_OVERRIDES: dict[str, int] = {
    "ref/affliction/vampire-vampire-warrior/vishkanya-venom": 240,
    "ref/companion/gorthek": 172,
    "ref/companion/riding-rat": 177,
    "ref/companion/yzobu": 124,
    "ref/discovery/blackstar-bomb": 32,
    "ref/discovery/void-bomb": 32,
    "ref/eq-misc/flying-straps-masterwork": 105,
    "ref/eq-misc/flying-straps-standard": 105,
    "ref/eq-misc/riding-rat-combat-trained": 177,
    "ref/eq-misc/riding-rat-common": 177,
    "ref/eq-weapon/firework-signal-rocket": 117,
    "ref/eq-weapon/heavy-slavers-crossbow": 45,
    "ref/eq-weapon/light-slavers-crossbow": 45,
    "ref/feat/strangler-mc": 20,
    "ref/family/vampire-servant": 238,
    "ref/hex/mud-witch": 105,
    "ref/monster/gnoll-chosen-of-lamashtu": 98,
    "ref/monster/gnoll-eye-of-lamashtu": 97,
    "ref/oraclecurse/cold-blooded": 140,
    "ref/oraclecurse/infested": 104,
    "ref/spell/scale-spikes-greater": 141,
    "ref/template/mutant-goblin": 112,
    "ref/template/vampire-minion-addle-minded": 236,
    "ref/template/vampire-minion-repeatedly-drained": 236,
    "ref/template/vampire-spawn": 244,
}


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    found = MONSTER_CODEX_PAGE.search(path.read_text(encoding="utf-8"))
    return int(found.group(1)) if found else None


# ---------------------------------------------------------------------------
# The transform


# Entries that print in a race chapter, in printed section order.
NEW_RULES_ORDER: dict[str, list[str]] = {
    "Boggards": ["Boggard Feats", "Boggard Spell", "Boggard Magic Items"],
    "Bugbears": ["Bugbear Archetype", "Bugbear Feats", "Bugbear Spell", "Bugbear Magic Items"],
    "Drow": ["Drow Alchemist Discoveries", "Drow Feats", "Drow Equipment", "Drow Magic Items"],
    "Duergar": ["Duergar Feats", "Duergar Equipment", "Duergar Spells", "Duergar Magic Item"],
    "Fire Giants": ["Fire Giant Oracle Mystery", "Fire Giant Feat", "Fire Giant Magic Items"],
    "Frost Giants": ["Frost Giant Feats", "Frost Giant Spells", "Frost Giant Magic Items"],
    "Ghouls": ["Ghoul Sorcerer Bloodline", "Ghoul Feats", "Ghoul Spells"],
    "Gnolls": ["Gnoll Archetypes", "Gnoll Feats", "Gnoll Equipment", "Gnoll Magic Items"],
    "Goblins": [
        "Goblin Archetype",
        "Goblin Oracle Curse",
        "Goblin Witch Hex",
        "Goblin Equipment",
        "Goblin Spells",
    ],
    "Hobgoblins": ["Hobgoblin Archetype", "Hobgoblin Feats", "Hobgoblin Equipment"],
    "Kobolds": ["Kobold Archetypes", "Kobold Companions", "Kobold Traps", "Kobold Feats"],
    "Lizardfolk": [
        "Lizardfolk Archetype",
        "Lizardfolk Oracle Curse",
        "Lizardfolk Feats",
        "Lizardfolk Spells",
    ],
    "Ogres": ["Ogre Simple Templates", "Ogre Giantkin Template", "Ogre Feats"],
    "Orcs": ["Orc Feats", "Orc Equipment", "Orc Magic Items"],
    "Ratfolk": ["Ratfolk Feats", "Ratfolk Equipment", "Ratfolk Magic Items"],
    "Sahuagin": ["Sahuagin Feats", "Sahuagin Spells"],
    "Serpentfolk": ["Serpentfolk Feats", "Serpentfolk Spells", "Serpentfolk Magic Items"],
    "Troglodytes": ["Troglodyte Spells", "Troglodyte Magic Items"],
    "Trolls": [
        "Troll Archetype",
        "Troll Feats",
        "Troll Equipment",
        "Troll Spells",
        "Troll Magic Items",
    ],
    "Vampires": [
        "Vampire Minion Simple Templates",
        "Vampire Feats",
        "Vampire Equipment",
        "Vampire Magic Items",
    ],
}

# entry id -> path under the race chapter.  The path may name an existing
# heading or entry-bearing node (for example a carried stat block).
RACE_MOVES: dict[str, list[tuple[str, list[str]]]] = {
    "Boggards": [
        ("ref/feat/extra-croaking", ["New Rules", "Boggard Feats"]),
        ("ref/feat/powerful-tongue", ["New Rules", "Boggard Feats"]),
        ("ref/feat/sonic-croak", ["New Rules", "Boggard Feats"]),
        ("ref/feat/stunning-croak", ["New Rules", "Boggard Feats"]),
        ("ref/feat/throat-pouch", ["New Rules", "Boggard Feats"]),
    ],
    "Bugbears": [
        ("ref/arc-antipaladin/fearmonger", ["New Rules", "Bugbear Archetype"]),
        ("ref/feat/bushwhack", ["New Rules", "Bugbear Feats"]),
        ("ref/feat/hurtful", ["New Rules", "Bugbear Feats"]),
        ("ref/feat/pile-on", ["New Rules", "Bugbear Feats"]),
        ("ref/feat/shocking-bellow", ["New Rules", "Bugbear Feats"]),
        ("ref/feat/staggering-blow", ["New Rules", "Bugbear Feats"]),
        ("ref/feat/strangler-mc", ["New Rules", "Bugbear Feats"]),
        ("ref/feat/visceral-threat", ["New Rules", "Bugbear Feats"]),
    ],
    "Drow": [
        ("ref/discovery/blackstar-bomb", ["New Rules", "Drow Alchemist Discoveries"]),
        ("ref/discovery/void-bomb", ["New Rules", "Drow Alchemist Discoveries"]),
        ("ref/feat/angelbane-strike", ["New Rules", "Drow Feats"]),
        ("ref/feat/shadow-shroud", ["New Rules", "Drow Feats"]),
        ("ref/feat/snipers-lantern", ["New Rules", "Drow Feats"]),
    ],
    "Duergar": [
        ("ref/feat/gray-dwarf-magic", ["New Rules", "Duergar Feats"]),
        ("ref/feat/innate-flexibility", ["New Rules", "Duergar Feats"]),
        ("ref/feat/tough-as-iron", ["New Rules", "Duergar Feats"]),
        ("ref/eq-weapon/heavy-slavers-crossbow", ["New Rules", "Duergar Equipment", "Slaver's Crossbow"]),
        ("ref/eq-weapon/light-slavers-crossbow", ["New Rules", "Duergar Equipment", "Slaver's Crossbow"]),
    ],
    "Fire Giants": [
        ("ref/mystery/apocalypse", ["New Rules", "Fire Giant Oracle Mystery"]),
        ("ref/feat/smoking-boulder", ["New Rules", "Fire Giant Feat"]),
    ],
    "Frost Giants": [
        ("ref/feat/ancestral-enmity", ["New Rules", "Frost Giant Feats"]),
        ("ref/feat/awesome-charge", ["New Rules", "Frost Giant Feats"]),
        ("ref/feat/born-of-frost", ["New Rules", "Frost Giant Feats"]),
        ("ref/feat/chilled-rock", ["New Rules", "Frost Giant Feats"]),
        ("ref/feat/cleaving-sweep", ["New Rules", "Frost Giant Feats"]),
        ("ref/feat/icy-stare", ["New Rules", "Frost Giant Feats"]),
        ("ref/feat/sure-on-ice", ["New Rules", "Frost Giant Feats"]),
    ],
    "Ghouls": [
        ("ref/sorcbloodline/ghoul", ["New Rules", "Ghoul Sorcerer Bloodline"]),
        ("ref/feat/bag-of-bones", ["New Rules", "Ghoul Feats"]),
        ("ref/feat/corpse-companion", ["New Rules", "Ghoul Feats"]),
        ("ref/feat/old-as-dust", ["New Rules", "Ghoul Feats"]),
        ("ref/feat/sleeper", ["New Rules", "Ghoul Feats"]),
        ("ref/feat/warren-digger", ["New Rules", "Ghoul Feats"]),
    ],
    "Gnolls": [
        ("ref/arc-witch/bouda", ["New Rules", "Gnoll Archetypes"]),
        ("ref/arc-barbarian/pack-rager", ["New Rules", "Gnoll Archetypes"]),
        ("ref/feat/coordinated-reposition", ["New Rules", "Gnoll Feats"]),
        ("ref/feat/disarm-partner", ["New Rules", "Gnoll Feats"]),
        ("ref/feat/improved-disarm-partner", ["New Rules", "Gnoll Feats"]),
        ("ref/feat/snapping-flank", ["New Rules", "Gnoll Feats"]),
        ("ref/feat/snapping-jaws", ["New Rules", "Gnoll Feats"]),
        (
            "ref/monster/gnoll-eye-of-lamashtu",
            ["Example Gnolls", "Gnoll Devotees", "Gnoll Devotee"],
        ),
        (
            "ref/monster/gnoll-chosen-of-lamashtu",
            ["Example Gnolls", "Gnoll Devotees", "Gnoll Cleric"],
        ),
    ],
    "Goblins": [
        ("ref/arc-alchemist/winged-marauder", ["New Rules", "Goblin Archetype"]),
        ("ref/oraclecurse/infested", ["New Rules", "Goblin Oracle Curse"]),
        ("ref/hex/mud-witch", ["New Rules", "Goblin Witch Hex"]),
        (
            "ref/eq-misc/flying-straps-masterwork",
            ["New Rules", "Goblin Equipment", "Flying Straps"],
        ),
        (
            "ref/eq-misc/flying-straps-standard",
            ["New Rules", "Goblin Equipment", "Flying Straps"],
        ),
        ("ref/template/mutant-goblin", ["Example Goblins", "Mutant Goblin"]),
    ],
    "Hobgoblins": [
        ("ref/arc-alchemist/grenadier-mc", ["New Rules", "Hobgoblin Archetype"]),
        ("ref/feat/bred-commander", ["New Rules", "Hobgoblin Feats"]),
        ("ref/feat/chain-challenge", ["New Rules", "Hobgoblin Feats"]),
        ("ref/feat/commander-of-goblinkind", ["New Rules", "Hobgoblin Feats"]),
        ("ref/feat/motivated-march", ["New Rules", "Hobgoblin Feats"]),
        ("ref/feat/spirit-of-the-corps", ["New Rules", "Hobgoblin Feats"]),
        ("ref/feat/warmonger", ["New Rules", "Hobgoblin Feats"]),
        (
            "ref/eq-weapon/firework-signal-rocket",
            ["New Rules", "Hobgoblin Equipment", "Signal Rocket"],
        ),
        ("ref/companion/yzobu", ["Example Hobgoblins", "Yzobu"]),
    ],
    "Kobolds": [
        ("ref/arc-alchemist/alchemical-trapper", ["New Rules", "Kobold Archetypes"]),
        ("ref/arc-bard/dragon-yapper", ["New Rules", "Kobold Archetypes"]),
        ("ref/companion/cave-salamander", ["New Rules", "Kobold Companions"]),
        ("ref/companion/riding-python", ["New Rules", "Kobold Companions"]),
        ("ref/feat/draconic-magic", ["New Rules", "Kobold Feats"]),
        ("ref/feat/slurk-rider", ["New Rules", "Kobold Feats"]),
    ],
    "Lizardfolk": [
        ("ref/arc-druid/ancient-guardian", ["New Rules", "Lizardfolk Archetype"]),
        ("ref/oraclecurse/cold-blooded", ["New Rules", "Lizardfolk Oracle Curse"]),
        ("ref/feat/aquatic-adaption", ["New Rules", "Lizardfolk Feats"]),
        ("ref/feat/dangerous-tail", ["New Rules", "Lizardfolk Feats"]),
        ("ref/feat/swift-swimmer", ["New Rules", "Lizardfolk Feats"]),
        (
            "ref/spell/scale-spikes-greater",
            ["New Rules", "Lizardfolk Spells", "Scale Spikes, Greater"],
        ),
    ],
    "Ogres": [
        ("ref/template/pickin-ogre", ["New Rules", "Ogre Simple Templates"]),
        ("ref/template/thicken-ogre", ["New Rules", "Ogre Simple Templates"]),
        ("ref/template/shaggra-ogre", ["New Rules", "Ogre Simple Templates"]),
        ("ref/template/giantkin-ogre", ["New Rules", "Ogre Giantkin Template"]),
        ("ref/feat/corrupted-flesh", ["New Rules", "Ogre Feats"]),
        ("ref/feat/fetid-breath", ["New Rules", "Ogre Feats"]),
        ("ref/feat/gluttonous-gobbler", ["New Rules", "Ogre Feats"]),
        ("ref/feat/night-stalker", ["New Rules", "Ogre Feats"]),
        ("ref/feat/ogre-crush", ["New Rules", "Ogre Feats"]),
        ("ref/feat/raging-brute", ["New Rules", "Ogre Feats"]),
        ("ref/feat/savage-critical", ["New Rules", "Ogre Feats"]),
        ("ref/feat/vestigial-head", ["New Rules", "Ogre Feats"]),
    ],
    "Orcs": [
        ("ref/feat/horn-rider", ["New Rules", "Orc Feats"]),
        ("ref/feat/horn-riders-charge", ["New Rules", "Orc Feats"]),
        ("ref/feat/mutual-hatred", ["New Rules", "Orc Feats"]),
        ("ref/feat/scarred-legion", ["New Rules", "Orc Feats"]),
        ("ref/companion/gorthek", ["Example Orcs", "Gorthek"]),
    ],
    "Ratfolk": [
        ("ref/feat/gnawer", ["New Rules", "Ratfolk Feats"]),
        ("ref/feat/pack-rat", ["New Rules", "Ratfolk Feats"]),
        ("ref/feat/shared-stash", ["New Rules", "Ratfolk Feats"]),
        ("ref/feat/sharptooth", ["New Rules", "Ratfolk Feats"]),
        (
            "ref/eq-misc/riding-rat-combat-trained",
            ["New Rules", "Ratfolk Equipment", "Riding Rat"],
        ),
        (
            "ref/eq-misc/riding-rat-common",
            ["New Rules", "Ratfolk Equipment", "Riding Rat"],
        ),
        ("ref/companion/riding-rat", ["New Rules", "Ratfolk Equipment", "Riding Rat"]),
    ],
    "Sahuagin": [
        ("ref/feat/aquatic-advantage", ["New Rules", "Sahuagin Feats"]),
        ("ref/feat/blood-tide", ["New Rules", "Sahuagin Feats"]),
        ("ref/feat/greater-blood-frenzy", ["New Rules", "Sahuagin Feats"]),
    ],
    "Serpentfolk": [
        ("ref/feat/innate-arcana", ["New Rules", "Serpentfolk Feats"]),
        ("ref/feat/serpentine-compression", ["New Rules", "Serpentfolk Feats"]),
        ("ref/feat/tail-weapon", ["New Rules", "Serpentfolk Feats"]),
        ("ref/feat/telepathic-distraction", ["New Rules", "Serpentfolk Feats"]),
        ("ref/feat/venomous-spray", ["New Rules", "Serpentfolk Feats"]),
    ],
    "Trolls": [
        ("ref/arc-druid/troll-fury", ["New Rules", "Troll Archetype"]),
        ("ref/feat/cooperative-rend", ["New Rules", "Troll Feats"]),
        ("ref/feat/great-rend", ["New Rules", "Troll Feats"]),
        ("ref/feat/mighty-bite", ["New Rules", "Troll Feats"]),
        ("ref/feat/raging-regeneration", ["New Rules", "Troll Feats"]),
        ("ref/feat/regenerate-muscles", ["New Rules", "Troll Feats"]),
        ("ref/feat/sprinting-troll", ["New Rules", "Troll Feats"]),
    ],
    "Vampires": [
        (
            "ref/template/vampire-minion-addle-minded",
            ["New Rules", "Vampire Minion Simple Templates"],
        ),
        (
            "ref/template/vampire-minion-repeatedly-drained",
            ["New Rules", "Vampire Minion Simple Templates"],
        ),
        ("ref/feat/aversion-tolerance", ["New Rules", "Vampire Feats"]),
        ("ref/feat/vampiric-companion", ["New Rules", "Vampire Feats"]),
        ("ref/affliction/vampire-vampire-warrior/vishkanya-venom", ["Example Vampires", "Vampire Warrior"]),
        ("ref/template/vampire-spawn", ["Example Vampires", "Vampire Spawn"]),
    ],
}

# New Monster stat blocks that print inside the chapter's Creature Statistics
# section, in printed order after the NPC stat blocks.
CREATURE_STATISTICS_MOVES: dict[str, list[str]] = {
    "Bugbears": [
        "ref/carried/monster-codex/rules/bugbears/bugbear-bloodmage",
        "ref/carried/monster-codex/rules/bugbears/bugbear-tyrant",
    ],
    "Duergar": ["ref/carried/monster-codex/rules/duergar/beetles"],
    "Fire Giants": ["ref/carried/monster-codex/rules/fire-giants/steam-hog"],
    "Frost Giants": ["ref/carried/monster-codex/rules/frost-giants/svathurim"],
}

# Simple class templates print in Appendix 1, in printed order.
CLASS_TEMPLATES = [
    "ref/template/barbarian",
    "ref/template/bard",
    "ref/template/cleric",
    "ref/template/druid",
    "ref/template/fighter",
    "ref/template/monk",
    "ref/template/paladin",
    "ref/template/ranger",
    "ref/template/rogue",
    "ref/template/sorcerer",
    "ref/template/wizard",
]

# Entries that belong to the book tree but print in another book.  They keep
# their place in the residual page-order chapter because the Monster Codex has
# no printed section for them.
CROSS_BOOK_ENTRIES = {
    "ref/class/barbarian",
    "ref/class/bard",
    "ref/class/fighter",
    "ref/class/ranger",
    "ref/monster/ogre",
    "ref/monster/sahuagin",
    "ref/monster/troglodyte",
    "ref/race/duergar",
    "ref/race/goblin",
    "ref/race/ratfolk",
    "ref/rule/class-barbarian/barbarian",
    "ref/rule/class-bard/bard",
    "ref/rule/class-fighter/fighter",
    "ref/rule/class-ranger/ranger",
    "ref/template/vampire",
}


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    before: dict[str, str] = {}

    def walk_paths(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            before[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_paths(child, here)

    for top in book["chapters"]:
        walk_paths(top, [])

    # Goblins: the dataset split "Goblin Archetype" prints inside New Rules,
    # and its equipment and spell headings are printed New Rules sections of
    # their own.
    goblins = chapter(book, "Goblins")
    goblin_rules = next(
        child for child in goblins["children"] if child.get("entry") == "ref/carried/monster-codex/rules/goblins/new-rules"
    )
    goblin_archetype = find_node(book, "ref/carried/monster-codex/rules/goblins/goblin-archetype")
    if goblin_archetype is None:
        raise SystemExit("missing Goblin Archetype entry")
    detach(book, "ref/carried/monster-codex/rules/goblins/goblin-archetype")
    for title in ("Goblin Equipment", "Goblin Spells"):
        node = next(
            (child for child in goblin_archetype.get("children", []) if child["title"] == title),
            None,
        )
        if node is not None:
            goblin_archetype["children"].remove(node)
            attach(goblin_rules, node)
    attach(goblin_rules, goblin_archetype)

    # Printed race-chapter moves.
    for chapter_title, moves in RACE_MOVES.items():
        root = chapter(book, chapter_title)
        for entry_id, path in moves:
            move(book, entry_id, resolve_path(root, path))

    # New Monster stat blocks belong to the printed Creature Statistics section.
    for chapter_title, entry_ids in CREATURE_STATISTICS_MOVES.items():
        root = chapter(book, chapter_title)
        example = resolve_path(root, [f"Example {chapter_title}"])
        for entry_id in entry_ids:
            move(book, entry_id, example)

    # The printed "Vampire Servants" heading opens the family entry.
    vampire_servants = resolve_path(chapter(book, "Vampires"), ["Example Vampires", "Vampire Servants"])
    family_node = find_node(book, "ref/family/vampire-servant")
    if family_node is not None:
        node = detach(book, "ref/family/vampire-servant")
        vampire_servants.setdefault("children", []).insert(0, node)
    elif not any(
        child.get("entry") == "ref/family/vampire-servant"
        for child in vampire_servants.get("children", [])
    ):
        raise SystemExit("missing Vampire Servant family entry")

    # Appendix 1 prints the simple class templates.
    appendix = chapter(book, "Appendix")
    class_templates = resolve_path(appendix, ["Modifying Monsters", "Simple Class Templates"])
    for entry_id in CLASS_TEMPLATES:
        move(book, entry_id, class_templates)

    # Printed order inside each race chapter's New Rules section.
    for chapter_title, order in NEW_RULES_ORDER.items():
        root = chapter(book, chapter_title)
        rules = next(
            (child for child in root["children"] if child.get("entry", "").endswith("/new-rules")),
            None,
        )
        if rules is None:
            raise SystemExit(f"missing New Rules under {chapter_title}")
        reorder(rules, order)

    after: dict[str, str] = {}

    def walk_after(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            after[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_after(child, here)

    for top in book["chapters"]:
        walk_after(top, [])

    if set(before) != set(after):
        print(f"BEFORE COUNT {len(before)} AFTER COUNT {len(after)}", file=sys.stderr)
        lost = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        print(f"LOST {lost}", file=sys.stderr)
        print(f"ADDED {added}", file=sys.stderr)
        raise SystemExit("entry set changed")

    residual = next(
        (node for node in book["chapters"] if node["title"] == "New entries (page order)"),
        None,
    )
    if residual is None:
        raise SystemExit("missing residual page-order chapter")
    unexpected = sorted(
        child.get("entry", child["title"])
        for child in residual.get("children", [])
        if child.get("entry") not in CROSS_BOOK_ENTRIES
    )
    if unexpected:
        raise SystemExit(f"unexpected residual entries: {unexpected}")

    moves = []
    for entry_id in sorted(before):
        page = page_of(corpus, entry_id)
        title = after[entry_id].split(" > ")[-1]
        if before[entry_id] != after[entry_id]:
            moves.append((title, before[entry_id], after[entry_id], page))
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
    text = dump_books(books)

    for title, old, new, page in moves:
        print(f"move\t{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the Monster Codex nested-placement shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {len(moves)} Monster Codex moves", file=sys.stderr)
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} Monster Codex moves)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
