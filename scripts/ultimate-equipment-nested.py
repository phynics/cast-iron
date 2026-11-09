#!/usr/bin/env python3
"""Place the Ultimate Equipment tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or a linked anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/ultimate-equipment-nested.py /path/to/cast-iron-data
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

BOOK = "ultimate-equipment"
BOOKS_FILE = "books.toml"
CARRIED = "ref/carried/ultimate-equipment/rules"

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


def walk(node: dict):
    yield node
    for child in node.get("children", []):
        yield from walk(child)


def flatten(node: dict) -> list[dict]:
    return list(walk(node))


def path_of(book: dict, node: dict) -> str:
    def walk_path(current: dict, trail: list[str]) -> str | None:
        here = trail + [current["title"]]
        if current is node:
            return " > ".join(here)
        for child in current.get("children", []):
            found = walk_path(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk_path(chapter, [])
        if found is not None:
            return found
    return "(missing)"


def placed_entries(book: dict) -> dict[str, str]:
    paths: dict[str, str] = {}
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry"):
                paths[node["entry"]] = path_of(book, node)
    return paths


# ---------------------------------------------------------------------------
# Page evidence

UE_PAGE = re.compile(r"Ultimate Equipment\s*(?:pg\.?|p\.?)\s*(\d+)", re.I)

# Printed chapter, section, and subsection openers. Their entries are dataset
# hubs, so their own text carries no page. The page is the printed heading.
CONTAINER_PAGES: dict[str, int] = {
    f"{CARRIED}/arms-and-armor": 6,
    f"{CARRIED}/arms-and-armor/arms-and-armor": 6,
    f"{CARRIED}/arms-and-armor/armor": 8,
    f"{CARRIED}/arms-and-armor/weapons": 16,
    f"{CARRIED}/arms-and-armor/special-materials": 48,
    f"{CARRIED}/gear": 54,
    f"{CARRIED}/gear/adventuring-gear": 56,
    f"{CARRIED}/gear/tools-and-skill-kits": 76,
    f"{CARRIED}/gear/animals/mounts/and-related-gear": 80,
    f"{CARRIED}/gear/clothing": 88,
    f"{CARRIED}/gear/entertainment-and-trade-goods": 92,
    f"{CARRIED}/gear/food-and-drink": 94,
    f"{CARRIED}/gear/lodging-and-services": 98,
    f"{CARRIED}/gear/alchemical-remedies": 100,
    f"{CARRIED}/gear/alchemical-tools": 102,
    f"{CARRIED}/gear/alchemical-weapons": 106,
    f"{CARRIED}/gear/poisons": 110,
    f"{CARRIED}/magic-arms-and-armor": 112,
    f"{CARRIED}/magic-arms-and-armor/armor-special-abilities": 114,
    f"{CARRIED}/magic-arms-and-armor/specific-magic-armor-and-shields": 124,
    f"{CARRIED}/magic-arms-and-armor/weapon-special-abilities": 134,
    f"{CARRIED}/magic-arms-and-armor/specific-magic-weapons": 150,
    f"{CARRIED}/rings/rods-and-staves": 164,
    f"{CARRIED}/rings/rods-and-staves/rings/rods/and-staves": 164,
    f"{CARRIED}/rings/rods-and-staves/rings": 166,
    f"{CARRIED}/rings/rods-and-staves/rods": 178,
    f"{CARRIED}/rings/rods-and-staves/staves": 192,
    f"{CARRIED}/wondrous-items": 204,
    f"{CARRIED}/wondrous-items/slotted-wondrous-items": 206,
    f"{CARRIED}/wondrous-items/other-wondrous-items": 206,
    f"{CARRIED}/wondrous-items/special-wondrous-items": 206,
    f"{CARRIED}/wondrous-items/activation": 206,
    f"{CARRIED}/wondrous-items/extradimensional-spaces": 207,
    f"{CARRIED}/wondrous-items/belt": 208,
    f"{CARRIED}/wondrous-items/body": 214,
    f"{CARRIED}/wondrous-items/chest": 220,
    f"{CARRIED}/wondrous-items/eyes": 224,
    f"{CARRIED}/wondrous-items/feet": 228,
    f"{CARRIED}/wondrous-items/hands": 234,
    f"{CARRIED}/wondrous-items/head": 240,
    f"{CARRIED}/wondrous-items/headbands": 248,
    f"{CARRIED}/wondrous-items/neck": 254,
    f"{CARRIED}/wondrous-items/shoulders": 262,
    f"{CARRIED}/wondrous-items/wrists": 270,
    f"{CARRIED}/wondrous-items/slotless": 276,
    f"{CARRIED}/artifacts-and-other-items": 326,
    f"{CARRIED}/artifacts-and-other-items/artifacts": 328,
    f"{CARRIED}/artifacts-and-other-items/cursed-items": 340,
    f"{CARRIED}/artifacts-and-other-items/intelligent-items": 352,
    f"{CARRIED}/appendix": 360,
    "ref/rule/treasure-generator": 360,
    "ref/misc/magic-items-by-category": 360,
    "ref/rule/step-1-determine-treasure-budget": 360,
    "ref/rule/step-2-determine-treasure-type": 360,
    "ref/rule/step-3-generate-treasure": 362,
    "ref/rule/treasure-types": 362,
    "ref/rule/type-a": 362,
    "ref/rule/type-b": 363,
    "ref/rule/type-c": 363,
    "ref/rule/type-d": 364,
    "ref/rule/type-e": 365,
    "ref/rule/type-f": 365,
    "ref/rule/type-g": 366,
    "ref/rule/type-h": 367,
    "ref/rule/type-i": 368,
    "ref/rule/type-e-masterwork": 370,
    "ref/rule/type-d-potions-and-oils": 370,
    "ref/rule/type-d-scrolls": 372,
    "ref/rule/tds-0-1-arcane": 372,
    "ref/rule/tds-2-arcane": 373,
    "ref/rule/tds-3-arcane": 374,
    "ref/rule/tds-4-arcane": 374,
    "ref/rule/tds-5-arcane": 375,
    "ref/rule/tds-6-arcane": 376,
    "ref/rule/tds-7-arcane": 376,
    "ref/rule/tds-8-arcane": 377,
    "ref/rule/tds-9-arcane": 377,
    "ref/rule/tds-0-1-divine": 378,
    "ref/rule/tds-2-divine": 378,
    "ref/rule/tds-3-divine": 379,
    "ref/rule/tds-4-divine": 380,
    "ref/rule/tds-5-divine": 380,
    "ref/rule/tds-6-divine": 381,
    "ref/rule/tds-7-divine": 381,
    "ref/rule/tds-8-divine": 382,
    "ref/rule/tds-9-divine": 382,
    "ref/rule/type-d-wands": 382,
    "ref/rule/tdw-1": 383,
    "ref/rule/tdw-2": 384,
    "ref/rule/tdw-3": 385,
    "ref/rule/tdw-4": 386,
    "ref/rule/common-cursed-item-effects": 340,
    "ref/rule/item-quirks": 355,
}

# Base items whose own PSRD text has no Sources line. The page is the printed
# heading the item opens or the page its variant rows share.
ITEM_PAGES: dict[str, int] = {
    # Chapter 1
    f"{CARRIED}/arms-and-armor/armor/armor-and-shields/lamellar": 9,
    f"{CARRIED}/arms-and-armor/armor/armor-and-shields/madu": 9,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/arrows": 22,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/bamboo-shaft-arrow": 22,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/heavy-shield": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/light-shield": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/madu": 20,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/shortsword": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/spiked-armor": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/spiked-heavy-shield": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/spiked-light-shield": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/throwing-shield": 18,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/unarmed-strike": 17,
    f"{CARRIED}/arms-and-armor/weapons/weapon-descriptions/whistling-arrow": 22,
    # Chapter 2
    f"{CARRIED}/gear/adventuring-gear/coffin": 56,
    f"{CARRIED}/gear/adventuring-gear/dungeoneering-kit": 56,
    f"{CARRIED}/gear/adventuring-gear/grappling-arrow": 56,
    "ref/eq-misc/pathfinders-kit": 57,
    f"{CARRIED}/gear/adventuring-gear/holy-symbol/compartment": 57,
    f"{CARRIED}/gear/adventuring-gear/holy-symbol/flask": 57,
    f"{CARRIED}/gear/adventuring-gear/lock": 57,
    f"{CARRIED}/gear/adventuring-gear/manacles": 57,
    f"{CARRIED}/gear/adventuring-gear/poison-pill-ring": 57,
    f"{CARRIED}/gear/adventuring-gear/pot": 57,
    f"{CARRIED}/gear/adventuring-gear/powder-horn": 57,
    f"{CARRIED}/gear/adventuring-gear/prosthetic": 57,
    f"{CARRIED}/gear/adventuring-gear/riding-kit": 57,
    f"{CARRIED}/gear/adventuring-gear/shovel": 57,
    f"{CARRIED}/gear/adventuring-gear/signal-whistle": 57,
    f"{CARRIED}/gear/adventuring-gear/silent-whistle": 57,
    f"{CARRIED}/gear/adventuring-gear/snorkel": 57,
    f"{CARRIED}/gear/adventuring-gear/stilts": 58,
    f"{CARRIED}/gear/adventuring-gear/survival-kit": 58,
    f"{CARRIED}/gear/adventuring-gear/tent": 58,
    f"{CARRIED}/gear/adventuring-gear/tripwire-trap": 58,
    f"{CARRIED}/gear/adventuring-gear/waffle-iron": 58,
    f"{CARRIED}/gear/adventuring-gear/waterproof-lantern": 57,
    f"{CARRIED}/gear/adventuring-gear/wire-saw": 58,
    f"{CARRIED}/gear/tools-and-skill-kits/artisans-tools": 77,
    f"{CARRIED}/gear/tools-and-skill-kits/bear-trap/bear-trap": 77,
    f"{CARRIED}/gear/tools-and-skill-kits/musical-instrument": 77,
    f"{CARRIED}/gear/tools-and-skill-kits/parasol": 77,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/axe-beak": 81,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/cat": 80,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/dire-bat/riding": 80,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/dog": 81,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/griffon": 81,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/hippogriff": 81,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/lizard": 80,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/owl": 80,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/rat": 80,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/saddle": 81,
    f"{CARRIED}/gear/animals/mounts/and-related-gear/animals/mounts/and-related-gear/squirrel": 80,
    f"{CARRIED}/gear/clothing/pocketed-scarf": 89,
    f"{CARRIED}/gear/clothing/reinforced-scarf": 89,
    f"{CARRIED}/gear/clothing/tear-away-clothing": 89,
    f"{CARRIED}/gear/entertainment-and-trade-goods/entertainment/loaded-dice": 92,
    f"{CARRIED}/gear/food-and-drink/grog": 95,
    f"{CARRIED}/gear/food-and-drink/meals": 95,
    f"{CARRIED}/gear/food-and-drink/tea": 95,
    f"{CARRIED}/gear/food-and-drink/wine": 95,
    f"{CARRIED}/gear/lodging-and-services/bath": 99,
    f"{CARRIED}/gear/lodging-and-services/hireling": 99,
    f"{CARRIED}/gear/lodging-and-services/inn-stay": 99,
    f"{CARRIED}/gear/lodging-and-services/scribe": 99,
    f"{CARRIED}/gear/alchemical-tools/invisible-ink": 103,
    f"{CARRIED}/gear/alchemical-tools/weapon-blanch": 103,
    f"{CARRIED}/gear/alchemical-weapons/banshee-ballerina-firework": 107,
    f"{CARRIED}/gear/alchemical-weapons/flame-fountain-firework": 107,
    f"{CARRIED}/gear/alchemical-weapons/paper-candle-firework": 107,
    f"{CARRIED}/gear/alchemical-weapons/skyrocket-firework": 107,
    f"{CARRIED}/gear/alchemical-weapons/star-candle-firework": 107,
    f"{CARRIED}/gear/alchemical-weapons/starfountain-firework": 107,
    # Chapter 3
    f"{CARRIED}/magic-arms-and-armor/specific-magic-armor-and-shields/specific-magic-armor/adamatine-breastplate": 124,
    f"{CARRIED}/magic-arms-and-armor/specific-magic-armor-and-shields/specific-magic-armor/giant-hide-armor": 127,
    f"{CARRIED}/magic-arms-and-armor/specific-magic-weapons/burrowing-bullet": 153,
    f"{CARRIED}/magic-arms-and-armor/specific-magic-weapons/mace-of-smiting": 157,
    f"{CARRIED}/magic-arms-and-armor/specific-magic-weapons/slaying-arrow": 160,
    # Chapter 4
    f"{CARRIED}/rings/rods-and-staves/rings/dungeon-ring": 166,
    f"{CARRIED}/rings/rods-and-staves/rings/ring-of-inner-fortitude": 171,
    f"{CARRIED}/rings/rods-and-staves/rings/ring-of-revelation": 173,
    # Chapter 5
    f"{CARRIED}/wondrous-items/wrists/wrists-slot-wondrous-items/bonebreaker-gauntlets": 270,
    f"{CARRIED}/wondrous-items/wrists/wrists-slot-wondrous-items/gauntlets-of-skill-at-arms": 273,
    f"{CARRIED}/wondrous-items/wrists/wrists-slot-wondrous-items/vambraces-of-the-genie": 275,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/bag-of-tricks": 282,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/book-of-extended-summoning": 283,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/boro-bead": 284,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/goblin-fire-drum": 300,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/golem-manual": 300,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/ioun-stone": 305,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/knights-pennon": 307,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/lords-banner": 308,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/page-of-spell-knowledge": 314,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/stone-horse": 320,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/stone-of-good-luck": 321,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/strand-of-prayer-beads": 321,
    f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/volatile-vaporizer": 324,
    # Chapter 6
    "ref/magic-artifact/hammer-of-thunderbolts": 330,
    f"{CARRIED}/artifacts-and-other-items/artifacts/major-artifacts/the-orbs-of-dragonkind": 338,
    f"{CARRIED}/artifacts-and-other-items/artifacts/major-artifacts/the-shadowstaff": 339,
    f"{CARRIED}/artifacts-and-other-items/cursed-items/specific-cursed-items/net-of-snaring": 348,
    f"{CARRIED}/artifacts-and-other-items/cursed-items/specific-cursed-items/one-way-window": 348,
    f"{CARRIED}/artifacts-and-other-items/cursed-items/specific-cursed-items/stone-of-weight": 351,
    # Appendix
    "ref/rule/advanced-gm-topics": None,
    "ref/trait/savage-breaker": None,
}

# Entries without a page that still sit in a printed section.
ITEM_PAGES.update(
    {
        "ref/rule/other-goods-and-services": 56,
        "ref/eq-misc/ink-black": 57,
        "ref/eq-misc/ink-non-black": 57,
        "ref/eq-misc/powder-horn": 57,
        "ref/eq-weapon/powder-horn": 57,
        "ref/eq-misc/wineskin": 75,
        "ref/eq-misc/burglars-outfit": 89,
        "ref/eq-misc/courtesans-outfit": 89,
        "ref/eq-misc/dilettantes-outfit": 89,
        "ref/eq-misc/doctors-outfit": 89,
        "ref/eq-misc/executioners-outfit": 89,
        "ref/eq-misc/squires-outfit": 89,
        "ref/eq-misc/tear-away-artisans-outfit": 89,
        "ref/eq-misc/tear-away-burglars-outfit": 89,
        "ref/eq-misc/tear-away-cold-weather-outfit": 89,
        "ref/eq-misc/tear-away-courtesans-outfit": 89,
        "ref/eq-misc/tear-away-courtiers-outfit": 89,
        "ref/eq-misc/tear-away-dilettantes-outfit": 89,
        "ref/eq-misc/tear-away-doctors-outfit": 89,
        "ref/eq-misc/tear-away-entertainers-outfit": 89,
        "ref/eq-misc/tear-away-executioners-outfit": 89,
        "ref/eq-misc/tear-away-explorers-outfit": 89,
        "ref/eq-misc/tear-away-hot-weather-outfit": 89,
        "ref/eq-misc/tear-away-monks-outfit": 89,
        "ref/eq-misc/tear-away-nobles-outfit": 89,
        "ref/eq-misc/tear-away-peasants-outfit": 89,
        "ref/eq-misc/tear-away-pickpockets-outfit": 89,
        "ref/eq-misc/tear-away-royal-outfit": 89,
        "ref/eq-misc/tear-away-scholars-outfit": 89,
        "ref/eq-misc/tear-away-squires-outfit": 89,
        "ref/eq-misc/tear-away-travelers-outfit": 89,
        "ref/monster/yak": 81,
        "ref/rule/common-cursed-item-effects": 340,
        "ref/rule/item-quirks": 355,
        # The dataset rows that print as variants of a named base item.
        "ref/eq-armor/madu-shield-leather": 9,
        "ref/eq-armor/madu-shield-steel": 9,
        "ref/eq-misc/arrow-grappling": 56,
        "ref/eq-misc/perfume-common-per-dose": 57,
        "ref/eq-misc/perfume-exotic-per-dose": 57,
        "ref/eq-misc/whistle-signal": 57,
        "ref/eq-misc/silent-whistle": 57,
        "ref/eq-misc/gecko-riding-combat-trained": 81,
        "ref/eq-misc/meal-common-per-day": 95,
        "ref/eq-misc/meal-good-per-day": 95,
        "ref/eq-misc/meal-poor-per-day": 95,
        "ref/eq-weapon/firework-banshee-ballerina": 107,
        "ref/eq-weapon/firework-flame-fountain": 107,
        "ref/eq-weapon/firework-paper-candle": 107,
        "ref/eq-weapon/firework-skyrocket": 107,
        "ref/eq-weapon/firework-star-candle": 107,
        "ref/eq-weapon/firework-starfountain": 107,
        "ref/magic-artifact/orbs-of-dragonkind": 338,
        "ref/magic-artifact/shadowstaff": 339,
    }
)


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in CONTAINER_PAGES:
        return CONTAINER_PAGES[entry_id]
    if entry_id in ITEM_PAGES:
        return ITEM_PAGES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = UE_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


# ---------------------------------------------------------------------------
# Printed structure
#
# Each tuple is a container. ``entry`` opens a printed section or subsection;
# ``heading`` is a printed heading with no entry of its own; ``items`` collects
# the remaining entries of a printed section in page order.

CHAPTERS = [
    (
        "entry",
        f"{CARRIED}/arms-and-armor",
        [
            ("entry", f"{CARRIED}/arms-and-armor/arms-and-armor", []),
            (
                "entry",
                f"{CARRIED}/arms-and-armor/armor",
                [("heading", "Armor and Shields", "armor-and-shields", [("items", 9, 15)])],
            ),
            (
                "entry",
                f"{CARRIED}/arms-and-armor/weapons",
                [
                    ("heading", "Weapon Descriptions", "weapon-descriptions", [("items", 17, 40)]),
                    ("heading", "Firearms", "firearms", [("items", 41, 46)]),
                ],
            ),
            (
                "entry",
                f"{CARRIED}/arms-and-armor/special-materials",
                [
                    ("items", 48, 51),
                    ("heading", "Primitive Materials", "primitive-materials", [("items", 52, 53)]),
                ],
            ),
        ],
    ),
    (
        "entry",
        f"{CARRIED}/gear",
        [
            ("entry", f"{CARRIED}/gear/adventuring-gear", [("items", 56, 75)]),
            ("entry", f"{CARRIED}/gear/tools-and-skill-kits", [("items", 76, 79)]),
            (
                "entry",
                f"{CARRIED}/gear/animals/mounts/and-related-gear",
                [
                    (
                        "heading",
                        "Animals, Mounts, and Related Gear",
                        "animals-mounts-and-related-gear",
                        [("items", 80, 86)],
                    ),
                    ("heading", "Transports", "transports", [("items", 87, 87)]),
                ],
            ),
            ("entry", f"{CARRIED}/gear/clothing", [("items", 88, 91)]),
            (
                "entry",
                f"{CARRIED}/gear/entertainment-and-trade-goods",
                [
                    ("heading", "Entertainment", "entertainment", [("items", 92, 92)]),
                    ("items", 93, 93),
                ],
            ),
            ("entry", f"{CARRIED}/gear/food-and-drink", [("items", 94, 97)]),
            ("entry", f"{CARRIED}/gear/lodging-and-services", [("items", 98, 99)]),
            ("entry", f"{CARRIED}/gear/alchemical-remedies", [("items", 100, 101)]),
            ("entry", f"{CARRIED}/gear/alchemical-tools", [("items", 102, 105)]),
            ("entry", f"{CARRIED}/gear/alchemical-weapons", [("items", 106, 109)]),
            ("entry", f"{CARRIED}/gear/poisons", [("items", 110, 111)]),
        ],
    ),
    (
        "entry",
        f"{CARRIED}/magic-arms-and-armor",
        [
            ("entry", f"{CARRIED}/magic-arms-and-armor/armor-special-abilities", [("items", 114, 123)]),
            (
                "entry",
                f"{CARRIED}/magic-arms-and-armor/specific-magic-armor-and-shields",
                [
                    ("heading", "Specific Magic Armor", "specific-magic-armor", [("items", 124, 129)]),
                    ("heading", "Specific Magic Shields", "specific-magic-shields", [("items", 130, 133)]),
                ],
            ),
            (
                "entry",
                f"{CARRIED}/magic-arms-and-armor/weapon-special-abilities",
                [
                    (
                        "heading",
                        "Magic Weapon Special Ability Descriptions",
                        "magic-weapon-special-ability-descriptions",
                        [("items", 134, 149)],
                    )
                ],
            ),
            ("entry", f"{CARRIED}/magic-arms-and-armor/specific-magic-weapons", [("items", 150, 163)]),
        ],
    ),
    (
        "heading",
        "Rings, Rods, and Staves",
        None,
        [
            (
                "entry",
                f"{CARRIED}/rings/rods-and-staves",
                [
                    ("entry", f"{CARRIED}/rings/rods-and-staves/rings/rods/and-staves", []),
                    ("entry", f"{CARRIED}/rings/rods-and-staves/rings", [("items", 166, 177)]),
                    (
                        "entry",
                        f"{CARRIED}/rings/rods-and-staves/rods",
                        [
                            ("heading", "Rods", "rods", [("items", 178, 186)]),
                            ("heading", "Metamagic Rods", "metamagic-rods-2", [("items", 187, 191)]),
                        ],
                    ),
                    ("entry", f"{CARRIED}/rings/rods-and-staves/staves", [("items", 192, 203)]),
                ],
            )
        ],
    ),
    (
        "entry",
        f"{CARRIED}/wondrous-items",
        [
            ("entry", f"{CARRIED}/wondrous-items/slotted-wondrous-items", []),
            ("entry", f"{CARRIED}/wondrous-items/other-wondrous-items", []),
            ("entry", f"{CARRIED}/wondrous-items/special-wondrous-items", []),
            ("entry", f"{CARRIED}/wondrous-items/activation", []),
            ("items", 206, 206),
            ("entry", f"{CARRIED}/wondrous-items/extradimensional-spaces", []),
            (
                "entry",
                f"{CARRIED}/wondrous-items/belt",
                [("heading", "Belt Slot Wondrous Items", "belt-slot-wondrous-items", [("items", 208, 213)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/body",
                [("heading", "Body Slot Wondrous Items", "body-slot-wondrous-items", [("items", 214, 219)])],
            ),
            ("heading", "Chest Slot Wondrous Items", "chest-slot-wondrous-items", [("items", 220, 223)]),
            (
                "entry",
                f"{CARRIED}/wondrous-items/eyes",
                [("heading", "Eyes Slot Wondrous Items", "eyes-slot-wondrous-items", [("items", 224, 227)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/feet",
                [("heading", "Feet Slot Wondrous Items", "feet-slot-wondrous-items", [("items", 228, 233)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/hands",
                [("heading", "Hands Slot Wondrous Items", "hands-slot-wondrous-items", [("items", 234, 239)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/head",
                [("heading", "Head Slot Wondrous Items", "head-slot-wondrous-items", [("items", 240, 247)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/headbands",
                [("heading", "Headband Slot Wondrous Items", "headband-slot-wondrous-items", [("items", 248, 253)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/neck",
                [("heading", "Neck Slot Wondrous Items", "neck-slot-wondrous-items", [("items", 254, 261)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/shoulders",
                [("heading", "Shoulders Slot Wondrous Items", "shoulders-slot-wondrous-items", [("items", 262, 269)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/wrists",
                [("heading", "Wrists Slot Wondrous Items", "wrists-slot-wondrous-items", [("items", 270, 275)])],
            ),
            (
                "entry",
                f"{CARRIED}/wondrous-items/slotless",
                [("heading", "Slotless Wondrous Items", "slotless-wondrous-items", [("items", 276, 325)])],
            ),
        ],
    ),
    (
        "entry",
        f"{CARRIED}/artifacts-and-other-items",
        [
            (
                "entry",
                f"{CARRIED}/artifacts-and-other-items/artifacts",
                [
                    ("heading", "Minor Artifacts", "minor-artifacts", [("items", 328, 334)]),
                    ("heading", "Major Artifacts", "major-artifacts", [("items", 335, 339)]),
                ],
            ),
            (
                "entry",
                f"{CARRIED}/artifacts-and-other-items/cursed-items",
                [
                    ("entry", "ref/rule/common-cursed-item-effects", [("items", 340, 347)]),
                    (
                        "heading",
                        "Specific Cursed Items",
                        "specific-cursed-items-2",
                        [("items", 348, 351)],
                    ),
                ],
            ),
            (
                "entry",
                f"{CARRIED}/artifacts-and-other-items/intelligent-items",
                [
                    ("entry", "ref/rule/item-quirks", [("items", 352, 355)]),
                    (
                        "heading",
                        "Specific Intelligent Items",
                        "specific-intelligent-items",
                        [("items", 356, 359)],
                    ),
                ],
            ),
        ],
    ),
    (
        "entry",
        f"{CARRIED}/appendix",
        [
            (
                "entry",
                "ref/rule/treasure-generator",
                [
                    ("entry", "ref/misc/magic-items-by-category", []),
                    ("entry", "ref/rule/step-1-determine-treasure-budget", []),
                    ("entry", "ref/rule/step-2-determine-treasure-type", []),
                    ("entry", "ref/rule/step-3-generate-treasure", []),
                    (
                        "entry",
                        "ref/rule/treasure-types",
                        [
                            ("entry", "ref/rule/type-a", []),
                            ("entry", "ref/rule/type-b", []),
                            ("entry", "ref/rule/type-c", []),
                            ("entry", "ref/rule/type-d", []),
                            ("entry", "ref/rule/type-e", []),
                            ("entry", "ref/rule/type-f", []),
                            ("entry", "ref/rule/type-g", []),
                            ("entry", "ref/rule/type-h", []),
                            ("entry", "ref/rule/type-i", []),
                        ],
                    ),
                    ("entry", "ref/rule/type-e-masterwork", []),
                    ("entry", "ref/rule/type-d-potions-and-oils", []),
                    (
                        "entry",
                        "ref/rule/type-d-scrolls",
                        [
                            ("entry", "ref/rule/tds-0-1-arcane", []),
                            ("entry", "ref/rule/tds-2-arcane", []),
                            ("entry", "ref/rule/tds-3-arcane", []),
                            ("entry", "ref/rule/tds-4-arcane", []),
                            ("entry", "ref/rule/tds-5-arcane", []),
                            ("entry", "ref/rule/tds-6-arcane", []),
                            ("entry", "ref/rule/tds-7-arcane", []),
                            ("entry", "ref/rule/tds-8-arcane", []),
                            ("entry", "ref/rule/tds-9-arcane", []),
                            ("entry", "ref/rule/tds-0-1-divine", []),
                            ("entry", "ref/rule/tds-2-divine", []),
                            ("entry", "ref/rule/tds-3-divine", []),
                            ("entry", "ref/rule/tds-4-divine", []),
                            ("entry", "ref/rule/tds-5-divine", []),
                            ("entry", "ref/rule/tds-6-divine", []),
                            ("entry", "ref/rule/tds-7-divine", []),
                            ("entry", "ref/rule/tds-8-divine", []),
                            ("entry", "ref/rule/tds-9-divine", []),
                        ],
                    ),
                    (
                        "entry",
                        "ref/rule/type-d-wands",
                        [
                            ("entry", "ref/rule/tdw-1", []),
                            ("entry", "ref/rule/tdw-2", []),
                            ("entry", "ref/rule/tdw-3", []),
                            ("entry", "ref/rule/tdw-4", []),
                        ],
                    ),
                ],
            ),
            ("heading", "Appendix 2: Gems and Jewelry", None, [("items", 388, 389)]),
        ],
    ),
]

RESIDUAL_TITLE = "New entries (page order)"
RESIDUAL_ENTRIES = [
    "ref/rule/advanced-gm-topics",
    "ref/trait/savage-breaker",
]

# Printed variants that share a base item but not a parenthetical title.
NEST_UNDER: dict[str, str] = {
    "ref/eq-armor/madu-shield-leather": f"{CARRIED}/arms-and-armor/armor/armor-and-shields/madu",
    "ref/eq-armor/madu-shield-steel": f"{CARRIED}/arms-and-armor/armor/armor-and-shields/madu",
    "ref/eq-misc/arrow-grappling": f"{CARRIED}/gear/adventuring-gear/grappling-arrow",
    "ref/eq-misc/perfume-common-per-dose": "ref/eq-misc/perfume-cologne",
    "ref/eq-misc/perfume-exotic-per-dose": "ref/eq-misc/perfume-cologne",
    "ref/eq-misc/whistle-signal": f"{CARRIED}/gear/adventuring-gear/signal-whistle",
    "ref/eq-misc/silent-whistle": f"{CARRIED}/gear/adventuring-gear/silent-whistle",
    "ref/eq-misc/gecko-riding-combat-trained": "ref/eq-misc/gecko-riding",
    "ref/eq-misc/meal-common-per-day": f"{CARRIED}/gear/food-and-drink/meals",
    "ref/eq-misc/meal-good-per-day": f"{CARRIED}/gear/food-and-drink/meals",
    "ref/eq-misc/meal-poor-per-day": f"{CARRIED}/gear/food-and-drink/meals",
    "ref/eq-weapon/firework-banshee-ballerina": f"{CARRIED}/gear/alchemical-weapons/banshee-ballerina-firework",
    "ref/eq-weapon/firework-flame-fountain": f"{CARRIED}/gear/alchemical-weapons/flame-fountain-firework",
    "ref/eq-weapon/firework-paper-candle": f"{CARRIED}/gear/alchemical-weapons/paper-candle-firework",
    "ref/eq-weapon/firework-skyrocket": f"{CARRIED}/gear/alchemical-weapons/skyrocket-firework",
    "ref/eq-weapon/firework-star-candle": f"{CARRIED}/gear/alchemical-weapons/star-candle-firework",
    "ref/eq-weapon/firework-starfountain": f"{CARRIED}/gear/alchemical-weapons/starfountain-firework",
    "ref/magic-wondrous/figurine-of-wondrous-power-basalt-dragon": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-black-jade-raven": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-bronze-griffon": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-coral-hippocampus": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-ebony-fly": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-emerald-frog": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-glass-walrus": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-golden-lions": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-hematite-courgar": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-ivory-goats": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-lapis-camel": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-marble-elephant": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-obsidian-raven": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-obsidian-steed": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-onyx-dog": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-pearl-dolphin": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-ruby-butterfly": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-serpentine-eagle": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-serpentine-owl": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-silver-raven": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-slate-spider": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-wondrous/figurine-of-wondrous-power-tin-imp": "ref/magic-wondrous/figurine-of-wondrous-power",
    "ref/magic-artifact/orbs-of-dragonkind": f"{CARRIED}/artifacts-and-other-items/artifacts/major-artifacts/the-orbs-of-dragonkind",
    "ref/magic-artifact/shadowstaff": f"{CARRIED}/artifacts-and-other-items/artifacts/major-artifacts/the-shadowstaff",
}

IOUN_BASE = f"{CARRIED}/wondrous-items/slotless/slotless-wondrous-items/ioun-stone"
CUBE_BASE = "ref/magic-wondrous/cube-of-force"
for stone in (
    "clear-spindle",
    "dark-blue-rhomboid",
    "deep-red-sphere",
    "dusty-rose-prism",
    "incandescent-blue-sphere",
    "iridescent-spindle",
    "lavender-and-green-ellipsoid",
    "orange-prism",
    "pale-blue-rhomboid",
    "pale-green-prism",
    "pale-lavender-ellipsoid",
    "pearly-white-spindle",
    "pink-and-green-sphere",
    "pink-rhomboid",
    "scarlet-and-blue-sphere",
    "vibrant-purple-prism",
):
    NEST_UNDER[f"ref/magic-ioun/{stone}"] = IOUN_BASE
for cube in (
    "cube-of-varied-force-walls",
    "discriminating-cube",
    "independent-cube-of-force",
    "rechargeable-cube-of-force",
    "remote-activation-cube",
):
    NEST_UNDER[f"ref/magic-wondrous/{cube}"] = CUBE_BASE

BASIS = (
    "The main chapter titles, their order, and their first pages are checked "
    "against the printed contents page of Ultimate Equipment. Each chapter "
    "holds its printed section headings and entries in page order. See "
    "docs/research/book-order-evidence.md. Canonical ids were substituted "
    "through the reviewed alias map; chapters and order are unchanged."
)

REVIEW = [
    "Each chapter holds its printed sections and page order; entries that share a printed page keep the dataset order.",
    "Magic item variants sit under their base item: parenthetical rows, the ioun stones, the cube of force variants, and the metamagic-rod rows.",
    "The printed Appendix 2: Gems and Jewelry has its own heading inside the Appendices chapter. Appendix 3: Art Objects has no corpus entries and no group.",
    "The residual New entries (page order) chapter keeps only the two cross-book hubs with no Ultimate Equipment page: Advanced GM Topics and Savage Breaker.",
    "Canonical ids were substituted through sources/canonical/id-map.toml. A legacy entry with no installed target is omitted and listed in coverage.",
]


# ---------------------------------------------------------------------------
# Tree building


def index_nodes(book: dict) -> tuple[dict[str, dict], dict[tuple[str, str | None], dict]]:
    by_entry: dict[str, dict] = {}
    by_heading: dict[tuple[str, str | None], dict] = {}
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry"):
                if node["entry"] in by_entry:
                    raise SystemExit(f"duplicate entry node {node['entry']}")
                by_entry[node["entry"]] = node
            else:
                key = (node["title"], node.get("anchor"))
                if key in by_heading:
                    raise SystemExit(f"duplicate heading node {key}")
                by_heading[key] = node
    return by_entry, by_heading


def container_ids(spec: tuple) -> list[str]:
    kind = spec[0]
    if kind == "entry":
        found = [spec[1]]
        for child in spec[2]:
            found.extend(container_ids(child))
        return found
    if kind == "heading":
        found = []
        for child in spec[3]:
            found.extend(container_ids(child))
        return found
    return []


def leaf_ranges(spec: tuple) -> list[tuple[int, int]]:
    if spec[0] == "items":
        return [(spec[1], spec[2])]
    found: list[tuple[int, int]] = []
    children = spec[2] if spec[0] == "entry" else spec[3] if spec[0] == "heading" else []
    for child in children:
        found.extend(leaf_ranges(child))
    return found


def build(
    spec: tuple,
    by_entry: dict[str, dict],
    by_heading: dict[tuple[str, str | None], dict],
    items_by_range: dict[tuple[int, int], list[dict]],
    used_entries: set[str],
    used_headings: set[tuple[str, str | None]],
) -> dict:
    kind = spec[0]
    children: list[dict] = []
    if kind == "items":
        return None  # handled by the parent
    if kind == "entry":
        entry_id = spec[1]
        node = by_entry.get(entry_id)
        if node is None:
            raise SystemExit(f"missing entry node {entry_id}")
        used_entries.add(entry_id)
        for child in spec[2]:
            if child[0] == "items":
                children.extend(items_by_range.get((child[1], child[2]), []))
            else:
                children.append(build(child, by_entry, by_heading, items_by_range, used_entries, used_headings))
    else:
        title, anchor = spec[1], spec[2]
        node = by_heading.get((title, anchor))
        if node is None:
            node = {"title": title, "anchor": anchor}
        used_headings.add((title, anchor))
        for child in spec[3]:
            if child[0] == "items":
                children.extend(items_by_range.get((child[1], child[2]), []))
            else:
                children.append(build(child, by_entry, by_heading, items_by_range, used_entries, used_headings))
    if children:
        node["children"] = children
    else:
        node.pop("children", None)
    return node


def base_for(
    entry_id: str,
    title: str,
    by_title_leaf: dict[tuple[str, tuple[int, int]], str],
    leaf: tuple[int, int],
) -> str | None:
    if entry_id in NEST_UNDER:
        return NEST_UNDER[entry_id]
    match = re.match(r"^(.*?)\s*\((.*)\)$", title)
    if not match:
        return None
    prefix = match.group(1)
    return by_title_leaf.get((prefix.lower(), leaf))


def nest_items(items: list[dict], leaf: tuple[int, int], order: dict[int, int], page: dict[str, int], by_title_leaf):
    children_of: dict[str, list[dict]] = {}
    roots: list[dict] = []
    by_entry_id = {node["entry"]: node for node in items}
    for node in items:
        entry_id = node["entry"]
        parent_id = base_for(entry_id, node["title"], by_title_leaf, leaf)
        if parent_id and parent_id in by_entry_id and parent_id != entry_id:
            children_of.setdefault(parent_id, []).append(node)
        else:
            roots.append(node)

    def sort_key(node: dict):
        return (page.get(node["entry"], 0), order[id(node)])

    def attach(node: dict) -> None:
        entry_id = node["entry"]
        kids = children_of.get(entry_id)
        if kids:
            kids.sort(key=sort_key)
            node["children"] = kids
            for kid in kids:
                attach(kid)
        else:
            node.pop("children", None)

    roots.sort(key=sort_key)
    for root in roots:
        attach(root)
    return roots


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    books = book
    before_paths = placed_entries(books)
    by_entry, by_heading = index_nodes(book)
    order = {id(node): index for index, node in enumerate(walk_all_chapters(book))}
    pages: dict[str, int] = {}
    for entry_id in by_entry:
        value = page_of(corpus, entry_id)
        if value is not None:
            pages[entry_id] = value

    # Fill a missing base page from a printed variant on the same page.
    for entry_id, node in by_entry.items():
        if entry_id in pages:
            continue
        match = re.match(r"^(.*?)\s*\((.*)\)$", node["title"])
        if not match:
            continue
        prefix = match.group(1).lower()
        for other_id, other in by_entry.items():
            if other_id == entry_id or other["title"].lower() != prefix:
                continue
            if other_id in pages:
                pages[entry_id] = pages[other_id]
                break

    # Metamagic rods print in their own section after the ordinary rods.
    for entry_id, node in by_entry.items():
        if node["title"].startswith("Rod of Metamagic") or "/metamagic-rods/" in entry_id:
            pages[entry_id] = pages.get(entry_id, 187)
            if entry_id.startswith("ref/magic-rod/") and entry_id not in ITEM_PAGES:
                pages.setdefault(entry_id, 187)

    # Resolve the rod base rows to the carried base item with the printed title.
    chapter_titles = {
        node["title"].lower(): entry_id
        for entry_id, node in by_entry.items()
        if not entry_id.startswith("ref/magic-rod/")
    }
    for entry_id, node in list(by_entry.items()):
        match = re.match(r"^Rod of Metamagic, (.+)$", node["title"])
        if not match:
            continue
        target = chapter_titles.get(f"{match.group(1)} metamagic rod".lower())
        if target and target != entry_id:
            NEST_UNDER.setdefault(entry_id, target)

    all_containers = set()
    for spec in CHAPTERS:
        all_containers.update(container_ids(spec))

    entries_by_range: dict[tuple[int, int], list[dict]] = {}
    by_title_leaf: dict[tuple[str, tuple[int, int]], str] = {}
    unplaced: list[str] = []
    for entry_id, node in by_entry.items():
        if entry_id in all_containers:
            continue
        if entry_id in RESIDUAL_ENTRIES:
            continue
        value = pages.get(entry_id)
        if value is None:
            unplaced.append(entry_id)
            continue
        leaf = None
        if node["title"].startswith("Rod of Metamagic") or "/metamagic-rods/" in entry_id:
            leaf = (187, 191)
        else:
            for spec in CHAPTERS:
                for candidate in leaf_ranges(spec):
                    if candidate[0] <= value <= candidate[1]:
                        leaf = candidate
                        break
                if leaf:
                    break
        if leaf is None:
            unplaced.append(entry_id)
            continue
        entries_by_range.setdefault(leaf, []).append(node)
        by_title_leaf[(node["title"].lower(), leaf)] = entry_id

    if unplaced:
        raise SystemExit("entries with no printed section: " + ", ".join(sorted(unplaced)))

    built: dict[tuple[int, int], list[dict]] = {}
    for leaf, items in entries_by_range.items():
        items = nest_items(items, leaf, order, pages, by_title_leaf)
        built[leaf] = items

    used_entries: set[str] = set()
    used_headings: set[tuple[str, str | None]] = set()
    chapters: list[dict] = []
    for spec in CHAPTERS:
        chapters.append(build(spec, by_entry, by_heading, built, used_entries, used_headings))

    placed_ids = {
        node["entry"]
        for chapter in chapters
        for node in walk(chapter)
        if node.get("entry")
    }
    missing = sorted(set(by_entry) - placed_ids - set(RESIDUAL_ENTRIES))
    if missing:
        raise SystemExit("entries not placed: " + ", ".join(missing))

    residual = by_heading.get((RESIDUAL_TITLE, None))
    if residual is None:
        residual = {"title": RESIDUAL_TITLE}
    else:
        used_headings.add((RESIDUAL_TITLE, None))
    residual["children"] = [by_entry[entry_id] for entry_id in RESIDUAL_ENTRIES]
    for entry_id in RESIDUAL_ENTRIES:
        by_entry[entry_id].pop("children", None)
    chapters.append(residual)

    unused_headings = set(by_heading) - used_headings
    if unused_headings:
        raise SystemExit("heading nodes dropped: " + str(sorted(unused_headings)))

    book["chapters"] = chapters
    book["basis"] = BASIS
    book["review"] = REVIEW

    after_paths = placed_entries(book)
    if set(before_paths) != set(after_paths):
        lost = sorted(set(before_paths) - set(after_paths))
        added = sorted(set(after_paths) - set(before_paths))
        raise SystemExit(f"entry set changed: lost {lost}, added {added}")

    moves = []
    for entry_id in sorted(before_paths):
        if before_paths[entry_id] != after_paths[entry_id]:
            title = after_paths[entry_id].split(" > ")[-1]
            moves.append((title, before_paths[entry_id], after_paths[entry_id], pages.get(entry_id)))
    return moves


def walk_all_chapters(book: dict):
    for chapter in book.get("chapters", []):
        yield from walk(chapter)


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
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the Ultimate Equipment nested-placement shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {BOOK}; {len(moves)} moves", file=sys.stderr)
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
