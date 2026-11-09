#!/usr/bin/env python3
"""Place the Ultimate Combat tree by printed chapter, section, and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or a linked anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/ultimate-combat-nested.py /path/to/cast-iron-data
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

BOOK = "ultimate-combat"
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


def flatten(node: dict) -> list[dict]:
    found = [node]
    for child in node.get("children", []):
        found.extend(flatten(child))
    return found


def title_key(node: dict) -> tuple[str, ...]:
    return tuple(re.findall(r"[a-z0-9]+", node["title"].lower()))


# ---------------------------------------------------------------------------
# Page and front-matter evidence

UC_PAGE = re.compile(r"Ultimate Combat(?: pg\.?| p\.) ?(\d+)")


class Corpus:
    def __init__(self, root: Path) -> None:
        self.root = root
        self._front: dict[str, dict] = {}
        self._page: dict[str, int | None] = {}

    def front(self, entry_id: str) -> dict:
        if entry_id not in self._front:
            path = self.root / "entries" / (entry_id[len("ref/") :] + ".md")
            text = path.read_text(encoding="utf-8")
            self._front[entry_id] = tomllib.loads(text.split("+++", 2)[1])
        return self._front[entry_id]

    def kind(self, entry_id: str) -> str:
        return self.front(entry_id)["kind"]

    def group(self, entry_id: str) -> str | None:
        return self.front(entry_id).get("fields", {}).get("group")

    def page(self, entry_id: str) -> int | None:
        if entry_id not in self._page:
            path = self.root / "entries" / (entry_id[len("ref/") :] + ".md")
            match = UC_PAGE.search(path.read_text(encoding="utf-8"))
            self._page[entry_id] = int(match.group(1)) if match else None
        return self._page[entry_id]


# ---------------------------------------------------------------------------
# Printed structure

# Classes (Ultimate Combat pp. 6-75). The printed chapter holds the three
# classes, then one Class Archetypes section with each class and its options.
BASE_CLASSES = [
    ("Gunslinger", "ref/class/gunslinger", "Deeds", "gunslinger-deed"),
    ("Ninja", "ref/class/ninja", "Ninja Tricks", "ninja-trick"),
    ("Samurai", "ref/class/samurai", "Samurai Orders", None),
]

# (slug, title, option heading and group) in printed class order.
CLASS_ARCHETYPES = [
    ("alchemist", "Alchemist", "Discoveries", "alchemist-discovery"),
    ("barbarian", "Barbarian", "Rage Powers", "rage-power"),
    ("bard", "Bard", None, None),
    ("cavalier", "Cavalier", "Cavalier Orders", None),
    ("cleric", "Cleric", None, None),
    ("druid", "Druid", None, None),
    ("fighter", "Fighter", None, None),
    ("gunslinger", "Gunslinger", None, None),
    ("inquisitor", "Inquisitor", "Inquisitions", "inquisition"),
    ("magus", "Magus", "Magus Arcana", "magus-arcana"),
    ("monk", "Monk", None, None),
    ("paladin", "Paladin", None, None),
    ("ranger", "Ranger", None, None),
    ("rogue", "Rogue", "Rogue Talents", "rogue-talent"),
    ("wizard", "Wizard", None, None),
]

ARCHETYPE_PREFIX = {
    "alchemist": ["ref/arc-alchemist/"],
    "barbarian": ["ref/arc-barbarian/"],
    "bard": ["ref/arc-bard/"],
    "cavalier": ["ref/arc-cavalier/"],
    "cleric": ["ref/arc-cleric/"],
    "druid": ["ref/arc-druid/"],
    "fighter": ["ref/arc-fighter/"],
    "gunslinger": ["ref/arc-gunslinger/"],
    "inquisitor": ["ref/arc-inquisitor/"],
    "magus": ["ref/arc-magus/"],
    "monk": ["ref/arc-monk/"],
    "paladin": ["ref/arc-paladin/", "ref/arc-antipaladin/"],
    "ranger": ["ref/arc-ranger/"],
    "rogue": ["ref/arc-rogue/"],
    "wizard": ["ref/arc-wizard/"],
}

ARCHETYPE_HEADING = {
    "alchemist": ("Alchemist Archetypes", "alchemist-archetypes"),
    "barbarian": ("Barbarian Archetypes", "barbarian-archetypes"),
    "bard": ("Bard Archetypes", "bard-archetypes"),
    "cavalier": ("Cavalier Archetypes", "cavalier-archetypes"),
    "cleric": ("Cleric Archetypes", "cleric-archetypes"),
    "druid": ("Druid Archetypes", "druid-archetypes"),
    "fighter": ("Fighter Archetypes", "fighter-archetypes"),
    "gunslinger": ("Gunslinger Archetypes", "gunslinger-archetypes"),
    "inquisitor": ("Inquisitor Archetypes", "inquisitor-archetypes"),
    "magus": ("Magus Archetypes", "magus-archetypes"),
    "monk": ("Monk Archetypes", "monk-archetypes"),
    "paladin": ("Paladin Archetypes", "paladin-archetypes"),
    "ranger": ("Ranger Archetypes", "ranger-archetypes"),
    "rogue": ("Rogue Archetypes", "rogue-archetypes"),
    "wizard": ("Wizard Archetypes", "wizard-archetypes"),
}

# Mastering Combat (pp. 124-167). The printed sections are listed in page
# order; item lists follow the printed descriptions, not the dataset order.
EASTERN_ARMOR = [
    "ref/eq-armor/do-maru",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-armor/four-mirror-armor",
    "ref/eq-armor/four-mirror",
    "ref/eq-armor/haramaki",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-armor/kikko-armor",
    "ref/eq-armor/kikko",
    "ref/eq-armor/kusari-gusoku",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-armor/lamellar-armor",
    "ref/eq-armor/lamellar-horn",
    "ref/eq-armor/lamellar-iron",
    "ref/eq-armor/lamellar-leather",
    "ref/eq-armor/lamellar-steel",
    "ref/eq-armor/lamellar-stone",
    "ref/eq-armor/lamellar-cuirass",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-armor/mountain-pattern-armor",
    "ref/eq-armor/mountain-pattern",
    "ref/eq-armor/o-yoroi",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-armor/silken-ceremonial-armor",
    "ref/eq-armor/silken-ceremonial",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-armor/stone-coat",
    "ref/eq-armor/tatami-do",
]

EASTERN_WEAPONS = [
    "ref/eq-weapon/arrow-iron-tipped-distance",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-weapons/weapon-descriptions/arrow/whistling",
    "ref/eq-weapon/whistling-arrows",
    "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons/eastern-weapons/weapon-descriptions/bamboo-shaft-arrow",
    "ref/eq-weapon/bamboo-shaft-arrows",
    "ref/eq-weapon/bo-staff",
    "ref/eq-weapon/nine-ring-broadsword",
    "ref/eq-weapon/butterfly-sword",
    "ref/eq-weapon/dan-bong",
    "ref/eq-weapon/double-chicken-saber",
    "ref/eq-weapon/emei-piercer",
    "ref/eq-weapon/fighting-fan",
    "ref/eq-weapon/hooked-lance",
    "ref/eq-weapon/iron-brush",
    "ref/eq-weapon/jutte",
    "ref/eq-weapon/double-chained-kama",
    "ref/eq-weapon/katana",
    "ref/eq-weapon/katana-double-walking-stick",
    "ref/eq-weapon/kerambit",
    "ref/eq-weapon/kusarigama",
    "ref/eq-weapon/kyoketsu-shoge",
    "ref/eq-weapon/lungchuan-tamo",
    "ref/eq-weapon/meteor-hammer",
    "ref/eq-weapon/monks-spade",
    "ref/eq-weapon/naginata",
    "ref/eq-weapon/nine-section-whip",
    "ref/eq-weapon/nodachi",
    "ref/eq-weapon/poisoned-sand-tube",
    "ref/eq-weapon/rope-dart",
    "ref/eq-weapon/sansetsukon",
    "ref/eq-weapon/shang-gou",
    "ref/eq-weapon/sibat",
    "ref/eq-weapon/seven-branched-sword",
    "ref/eq-weapon/tekko-kagi",
    "ref/eq-weapon/tetsubo",
    "ref/eq-weapon/tiger-fork",
    "ref/eq-weapon/tonfa",
    "ref/eq-weapon/tri-point-double-edged-sword",
    "ref/eq-weapon/tube-arrow-shooter",
    "ref/eq-weapon/urumi",
    "ref/eq-weapon/wakizashi",
    "ref/eq-weapon/wushu-dart",
]

WEAPON_SPECIAL_FEATURES = [
    "ref/rule/weapon-special-features",
    "ref/misc/blocking",
    "ref/misc/deadly",
    "ref/misc/distracting",
    "ref/misc/grapple",
]

EARLY_FIREARMS = [
    "ref/eq-weapon/blunderbuss",
    "ref/eq-weapon/buckler-gun",
    "ref/eq-weapon/culverin",
    "ref/eq-weapon/double-hackbut",
    "ref/eq-weapon/fire-lance",
    "ref/eq-weapon/musket",
    "ref/eq-weapon/axe-musket",
    "ref/eq-weapon/double-barreled-musket",
    "ref/eq-weapon/warhammer-musket",
    "ref/eq-weapon/pepperbox",
    "ref/eq-weapon/pistol",
    "ref/eq-weapon/coat-pistol",
    "ref/eq-weapon/dagger-pistol",
    "ref/eq-weapon/double-barreled-pistol",
    "ref/eq-weapon/dragon-pistol",
    "ref/eq-weapon/sword-cane-pistol",
]

ADVANCED_FIREARMS = [
    "ref/eq-weapon/revolver",
    "ref/eq-weapon/rifle",
    "ref/eq-weapon/pepperbox-rifle",
    "ref/eq-weapon/shotgun",
    "ref/eq-weapon/double-barreled-shotgun",
]

ALCHEMICAL_CARTRIDGE_GROUP = (
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-descriptions/firearm-ammunition-and-adventuring-gear/"
    "alchemical-cartridges"
)

ALCHEMICAL_CARTRIDGE_VARIANTS = [
    ALCHEMICAL_CARTRIDGE_GROUP + "/dragons-breath-cartridge",
    ALCHEMICAL_CARTRIDGE_GROUP + "/entangling-shot-cartridge",
    ALCHEMICAL_CARTRIDGE_GROUP + "/flare-cartridge",
    ALCHEMICAL_CARTRIDGE_GROUP + "/paper-cartridge",
    ALCHEMICAL_CARTRIDGE_GROUP + "/salt-shot-cartridge",
]

AMMUNITION = [
    ALCHEMICAL_CARTRIDGE_GROUP,
    "ref/eq-weapon/alchemical-cartridge",
    "ref/eq-weapon/alchemical-cartridge-dragons-breath",
    "ref/eq-weapon/alchemical-cartridge-entangling-shot",
    "ref/eq-weapon/alchemical-cartridge-flare",
    "ref/eq-weapon/alchemical-cartridge-paper-bullet-or-pellet",
    "ref/eq-weapon/alchemical-cartridge-salt-shot",
    "ref/eq-weapon/black-powder-dose",
    "ref/eq-weapon/black-powder-keg",
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-descriptions/firearm-ammunition-and-adventuring-gear/bullet",
    "ref/eq-weapon/firearm-bullet",
    "ref/eq-weapon/bullet-adamantine",
    "ref/eq-weapon/pitted-bullet",
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-descriptions/firearm-ammunition-and-adventuring-gear/bullet/silver",
    "ref/eq-weapon/firearm-bullet-silver",
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-descriptions/firearm-ammunition-and-adventuring-gear/gunsmiths-kit",
    "ref/eq-weapon/gunsmiths-kit",
    "ref/eq-misc/gunsmiths-kit",
    "ref/eq-weapon/metal-cartridge",
    "ref/eq-weapon/pellets-handful",
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-descriptions/firearm-ammunition-and-adventuring-gear/powder-horn",
    "ref/eq-weapon/powder-horn",
    "ref/eq-misc/powder-horn",
]

FIREARM_AMMUNITION_SPECIAL = ["ref/magic-enh/dry-load"]

SPECIFIC_FIREARM_AMMUNITION = [
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-magic-items/specific-firearm-ammunition/burrowing-bullet",
    "ref/magic-weapon/burrowing-bullet-greater",
    "ref/magic-weapon/burrowing-bullet-lesser",
    "ref/magic-weapon/tracer-bullet",
]

MAGIC_FIREARM_ABILITIES = [
    "ref/magic-enh/lucky",
    "ref/magic-enh/lucky-greater",
    "ref/magic-enh/reliable",
    "ref/magic-enh/reliable-greater",
]

FIGURINE_GROUP = (
    "ref/carried/ultimate-combat/rules/mastering-combat/firearms/"
    "firearm-magic-items/wondrous-items/figurine-of-wondrous-power"
)

FIGURINE_VARIANTS = [
    "ref/magic-wondrous/figurine-of-wondrous-power-basalt-dragon",
    "ref/magic-wondrous/figurine-of-wondrous-power-black-jade-raven",
    "ref/magic-wondrous/figurine-of-wondrous-power-bronze-griffon",
    "ref/magic-wondrous/figurine-of-wondrous-power-coral-hippocampus",
    "ref/magic-wondrous/figurine-of-wondrous-power-ebony-fly",
    "ref/magic-wondrous/figurine-of-wondrous-power-emerald-frog",
    "ref/magic-wondrous/figurine-of-wondrous-power-glass-walrus",
    "ref/magic-wondrous/figurine-of-wondrous-power-golden-lions",
    "ref/magic-wondrous/figurine-of-wondrous-power-hematite-courgar",
    "ref/magic-wondrous/figurine-of-wondrous-power-ivory-goats",
    "ref/magic-wondrous/figurine-of-wondrous-power-lapis-camel",
    "ref/magic-wondrous/figurine-of-wondrous-power-marble-elephant",
    "ref/magic-wondrous/figurine-of-wondrous-power-obsidian-raven",
    "ref/magic-wondrous/figurine-of-wondrous-power-obsidian-steed",
    "ref/magic-wondrous/figurine-of-wondrous-power-onyx-dog",
    "ref/magic-wondrous/figurine-of-wondrous-power-pearl-dolphin",
    "ref/magic-wondrous/figurine-of-wondrous-power-ruby-butterfly",
    "ref/magic-wondrous/figurine-of-wondrous-power-serpentine-eagle",
    "ref/magic-wondrous/figurine-of-wondrous-power-serpentine-owl",
    "ref/magic-wondrous/figurine-of-wondrous-power-silver-raven",
    "ref/magic-wondrous/figurine-of-wondrous-power-slate-spider",
    "ref/magic-wondrous/figurine-of-wondrous-power-tin-imp",
]

WONDROUS_FIREARM_ITEMS = [
    "ref/magic-wondrous/amulet-of-bullet-protection",
    "ref/magic-wondrous/amulet-of-bullet-protection-1",
    "ref/magic-wondrous/amulet-of-bullet-protection-2",
    "ref/magic-wondrous/amulet-of-bullet-protection-3",
    "ref/magic-wondrous/amulet-of-bullet-protection-4",
    "ref/magic-wondrous/amulet-of-bullet-protection-5",
    "ref/magic-wondrous/dry-load-powder-horn",
    "ref/magic-wondrous/far-reaching-sight",
    FIGURINE_GROUP,
    "ref/magic-wondrous/figurine-of-wondrous-power",
] + FIGURINE_VARIANTS + [
    "ref/magic-wondrous/oil-of-silence",
    "ref/magic-wondrous/see-invisibility-sight",
]

GLADIATOR_WEAPONS = [
    "ref/eq-weapon/aklys",
    "ref/eq-weapon/amentum",
    "ref/eq-weapon/flying-blade",
    "ref/eq-weapon/gladius",
    "ref/eq-weapon/hooked-axe",
    "ref/eq-weapon/knuckle-axe",
    "ref/eq-weapon/javelin",
    "ref/carried/ultimate-combat/rules/mastering-combat/gladiator-weapons/"
    "gladiator-weapon-descriptions/madu",
    "ref/eq-weapon/pata",
    "ref/eq-weapon/quadrens",
    "ref/eq-weapon/scizore",
    "ref/eq-weapon/scorpion-whip",
    "ref/eq-weapon/shotel",
    "ref/eq-weapon/sica",
    "ref/carried/ultimate-combat/rules/mastering-combat/gladiator-weapons/"
    "gladiator-weapon-descriptions/throwing-shield",
]

PRIMITIVE_MATERIALS = [
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/primitive-materials/bone",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/primitive-materials/bronze",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/primitive-materials/gold",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/primitive-materials/obsidian",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/primitive-materials/stone",
]

STONE_AGE = [
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/atlatl",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/atlatl-dart",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/mere-club",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/taiaha",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/tepoztopilli",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/terbutje",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/terbutje/great",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-stone-age/wahaika",
]

BRONZE_AGE = [
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-bronze-age/harpoon",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-bronze-age/kestros",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-bronze-age/kestros-dart",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-bronze-age/mattock",
    "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons/the-bronze-age/rhomphaia",
]

DUELS = [
    "ref/rule/duel-combat",
    "ref/rule/starting-a-duel",
    "ref/rule/dueling-counter",
    "ref/rule/dueling-dodge",
    "ref/rule/dueling-parry",
    "ref/rule/dueling-resolve",
    "ref/rule/duel-results",
    "ref/rule/duels-and-performance-combat",
]

PERFORMANCE_COMBAT = [
    "ref/rule/crowd-attitudes",
    "ref/rule/types-of-performance-combat",
    "ref/rule/performance-combat-check",
    "ref/rule/affecting-the-crowds-attitude",
    "ref/rule/free-action-or-immediate-action-checks",
    "ref/rule/swift-action-checks",
    "ref/rule/determining-a-winner",
    "ref/rule/mandatory-checks",
    "ref/rule/serialized-performance-combat",
    "ref/rule/victory-points",
]

SIEGE_WEAPONS = [
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/ballista",
    "ref/eq-siegeengine/ballista-light",
    "ref/eq-siegeengine/ballista-heavy",
    "ref/eq-siegeengine/ballista-gate-breaker",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/bombard",
    "ref/eq-siegeengine/bombard-light",
    "ref/eq-siegeengine/bombard-standard",
    "ref/eq-siegeengine/bombard-heavy",
    "ref/eq-siegeengine/cannon",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/catapult",
    "ref/eq-siegeengine/catapult-light",
    "ref/eq-siegeengine/catapult-standard",
    "ref/eq-siegeengine/catapult-heavy",
    "ref/eq-siegeengine/cannon-fiends-mouth",
    "ref/eq-siegeengine/firedrake",
    "ref/eq-siegeengine/firewyrm",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/springal",
    "ref/eq-siegeengine/springal-arrow",
    "ref/eq-siegeengine/springal-rocket",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/trebuchet",
    "ref/eq-siegeengine/trebuchet-light",
    "ref/eq-siegeengine/trebuchet-standard",
    "ref/eq-siegeengine/trebuchet-heavy",
]

SIEGE_AMMUNITION = [
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/special-siege-engine-ammunition/alchemical-fire",
    "ref/eq-siegeengine/alchemists-fire",
    "ref/eq-siegeengine/blast-shot",
    "ref/eq-siegeengine/bomb",
    "ref/eq-siegeengine/chain-shot",
    "ref/eq-siegeengine/liquid-ice",
    "ref/eq-siegeengine/plague-bundle",
    "ref/eq-siegeengine/smoke-shot",
]

CLOSE_ASSAULT = [
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/close-assault-siege-engines/bridge/assault",
    "ref/eq-siegeengine/bridge-assault-large",
    "ref/eq-siegeengine/bridge-assault-huge",
    "ref/eq-siegeengine/bridge-assault-gargantuan",
    "ref/eq-siegeengine/bridge-assault-colossal",
    "ref/eq-siegeengine/corvus",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/close-assault-siege-engines/gallery",
    "ref/eq-siegeengine/gallery-large",
    "ref/eq-siegeengine/gallery-huge",
    "ref/eq-siegeengine/gallery-gargantuan",
    "ref/eq-siegeengine/gallery-colossal",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/close-assault-siege-engines/ladder/escalade",
    "ref/eq-siegeengine/ladder-escalade-large",
    "ref/eq-siegeengine/ladder-escalade-huge",
    "ref/eq-siegeengine/ladder-escalade-gargantuan",
    "ref/eq-siegeengine/ladder-escalade-colossal",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/close-assault-siege-engines/rams",
    "ref/eq-siegeengine/ram-large",
    "ref/eq-siegeengine/ram-huge",
    "ref/eq-siegeengine/ram-gargantuan",
    "ref/eq-siegeengine/ram-colossal",
    "ref/carried/ultimate-combat/rules/mastering-combat/siege-engines/"
    "siege-weapon-descriptions/close-assault-siege-engines/siege-tower",
    "ref/eq-siegeengine/siege-tower-large",
    "ref/eq-siegeengine/siege-tower-huge",
    "ref/eq-siegeengine/siege-tower-gargantuan",
    "ref/eq-siegeengine/siege-tower-colossal",
]

# Vehicles (pp. 168-187).
FULL_VEHICLE_RULES = [
    "ref/rule/drivers-and-occupants",
    "ref/rule/propulsion-and-driving-checks",
    "ref/rule/vehicle-size-and-space",
    "ref/rule/vehicle-facing-and-movement",
    "ref/rule/driving-vehicles",
    "ref/carried/ultimate-combat/rules/vehicles/maneuvering-a-vehicle",
    "ref/rule/optional-rule-wide-turns",
    "ref/rule/vehicles-in-combat",
    "ref/carried/ultimate-combat/rules/vehicles/turning-a-vehicle",
    "ref/rule/vic-initiative-and-movement",
    "ref/rule/vic-threatening-and-line-of-sight",
    "ref/rule/vic-jumping-on-or-off-vehicle",
    "ref/rule/vic-taking-control-of-a-vehicle",
    "ref/rule/vic-attacks-against-vehicles",
    "ref/rule/vic-vehicle-combat-maneuvers",
    "ref/rule/vic-damaging-a-vehicle",
    "ref/rule/propulsion-devices",
    "ref/rule/driving-devices",
]

LAND_VEHICLES = [
    "ref/rule/vehicle-carriage",
    "ref/rule/vehicle-cart",
    "ref/rule/vehicle-light-chariot",
    "ref/rule/vehicle-medium-chariot",
    "ref/rule/vehicle-heavy-chariot",
    "ref/rule/vehicle-sleigh",
    "ref/rule/vehicle-steam-giant",
    "ref/rule/vehicle-light-wagon",
    "ref/rule/vehicle-medium-wagon",
    "ref/rule/vehicle-heavy-wagon",
]

WATER_VEHICLES = [
    "ref/rule/wv-galley",
    "ref/rule/wv-keelboat",
    "ref/rule/wv-longship",
    "ref/rule/wv-rowboat",
    "ref/rule/wv-sailing-ship",
    "ref/rule/wv-warship",
]

# Variant Rules (pp. 188-207).
ARMOR_AS_DR = [
    "ref/rule/defense",
    "ref/rule/armor-as-dr",
    "ref/rule/critical-hits-and-defense",
]

CALLED_SHOTS = [
    "ref/rule/making-called-shots",
    "ref/rule/additional-rules",
    "ref/rule/called-shot-effects",
    "ref/rule/arm",
    "ref/rule/chest",
    "ref/rule/ear",
    "ref/rule/eye",
    "ref/rule/hand",
    "ref/rule/head",
    "CALLED_SHOT_FEATS",
    "ref/rule/heart",
    "ref/rule/leg",
    "ref/rule/neck",
    "ref/rule/vitals",
]

CALLED_SHOT_FEATS = [
    "ref/feat/improved-called-shot",
    "ref/feat/greater-called-shot",
]

PIECEMEAL_ARMOR = [
    "ref/rule/armor-pieces-and-armor-suits",
    "ref/rule/armor-pieces-and-proficiency",
    "MASTERWORK",
    "ref/rule/armor-pieces-for-unusual-creatures",
    "ref/rule/armor-and-armor-piece-hit-points-and-hardness",
    "ref/rule/getting-into-and-out-of-armor-pieces",
    "ARMOR_PIECE_DESCRIPTIONS",
]

MASTERWORK_MATERIALS = [
    "ref/rule/adamantine",
    "ref/rule/dragonhide",
    "ref/rule/mithral",
]

ARMOR_PIECE_DESCRIPTIONS = [
    "ref/rule/arms",
    "ref/rule/legs",
    "ref/rule/torso",
]

WOUNDS_AND_VIGOR = [
    "ref/rule/determining-wound-points-and-vigor-points",
    "ref/rule/wound-points",
    "ref/rule/vigor-points",
    "ref/rule/regaining-wound-points-and-vigor-points",
    "ref/rule/attacks-that-deal-wound-point-damage",
    "ref/rule/other-considerations-uc",
    "ref/rule/wounds-and-vigor-variant-feats",
]

# Entries whose printed home is a section of another book. They stay in the
# residual page-ordered chapter until a later decision places them.
CROSS_BOOK_HUBS = {
    "ref/rule/alternate-classes",
    "ref/rule/base-classes",
    "ref/rule/feats-class",
    "ref/rule/feats-other",
    "ref/rule/advanced-gm-topics",
    "ref/rule/optional-combat-changes",
    "ref/rule/specific-rulesets",
    "ref/rule/weapon-descriptions",
    "ref/ability/sneak-attack",
}

# Pf Data cartridge records that do not print in Ultimate Combat's firearm
# table; their page attribution is secondary.
CROSS_BOOK_CARTRIDGES = {
    "ref/eq-weapon/alchemical-cartridge-gray-dust",
    "ref/eq-weapon/alchemical-cartridge-gray-dust-shot",
    "ref/eq-weapon/alchemical-cartridge-molten-shot",
    "ref/eq-weapon/alchemical-cartridge-repellent",
    "ref/eq-weapon/alchemical-cartridge-smoke-cartridge",
    "ref/eq-weapon/alchemical-cartridge-toxin",
}


# ---------------------------------------------------------------------------
# The transform


def spell_sort_key(node: dict) -> tuple:
    title = node["title"].strip()
    # The carried entry misspells the printed spell; keep it beside the
    # printed name and list the duplicate for a follow-up.
    title = {"Adjuring Step": "Abjuring Step"}.get(title, title)
    base = title
    suffix = ""
    prefixes = ("Greater Communal", "Greater", "Improved", "Lesser", "Mass", "Communal")
    suffixes = ("Greater Communal", "Greater", "Improved", "Lesser", "Mass", "Communal")
    match = re.match(r"^(%s)\s+(.*)$" % "|".join(prefixes), title, re.I)
    if match:
        suffix = match.group(1).lower()
        base = match.group(2)
    else:
        match = re.match(r"^(.*),\s*(%s)$" % "|".join(suffixes), title, re.I)
        if match:
            base = match.group(1)
            suffix = match.group(2).lower()
    rank = {
        "": 0,
        "lesser": 1,
        "improved": 2,
        "greater": 3,
        "mass": 4,
        "communal": 5,
        "greater communal": 6,
    }[suffix]
    base_key = tuple(re.findall(r"[a-z0-9]+", base.lower()))
    return (base_key, rank, title_key(node))


def transform(book: dict, corpus: Corpus) -> dict[str, str]:
    chapters = {chapter["title"]: chapter for chapter in book["chapters"]}
    all_nodes: list[dict] = []
    for chapter in book["chapters"]:
        all_nodes.extend(flatten(chapter))

    by_entry: dict[str, dict] = {}
    anchor_nodes: dict[str, dict] = {}
    for node in all_nodes:
        if node.get("entry"):
            if node["entry"] in by_entry:
                raise SystemExit(f"duplicate entry in tree: {node['entry']}")
            by_entry[node["entry"]] = node
        if node.get("anchor"):
            if node["anchor"] in anchor_nodes:
                raise SystemExit(f"duplicate anchor in tree: {node['anchor']}")
            anchor_nodes[node["anchor"]] = node

    original_order = [
        node["entry"] for node in all_nodes if node.get("entry")
    ]
    original_set = set(original_order)
    before_paths = {entry: _path_of(book, entry) for entry in original_order}

    for node in all_nodes:
        node["children"] = []

    assigned: set[str] = set()
    used_anchors: set[str] = set()

    def entry(entry_id: str, children: list[dict] | None = None) -> dict:
        if entry_id not in by_entry:
            raise SystemExit(f"missing entry: {entry_id}")
        if entry_id in assigned:
            raise SystemExit(f"entry placed twice: {entry_id}")
        assigned.add(entry_id)
        node = by_entry[entry_id]
        node["children"] = list(children or [])
        return node

    def heading(
        title: str,
        anchor: str | None = None,
        children: list[dict] | None = None,
    ) -> dict:
        if anchor is not None:
            if anchor not in anchor_nodes:
                raise SystemExit(f"missing anchor node: {anchor}")
            node = anchor_nodes[anchor]
            if node["title"] != title:
                raise SystemExit(f"anchor {anchor} has title {node['title']!r}")
            used_anchors.add(anchor)
            node["children"] = list(children or [])
            return node
        return {"title": title, "children": list(children or [])}

    def page_of(entry_id: str) -> int | None:
        return corpus.page(entry_id)

    def page_title_key(entry_id: str) -> tuple:
        return (page_of(entry_id) or 10**9, title_key(by_entry[entry_id]))

    def option_group(group: str) -> list[str]:
        ids = [
            entry_id
            for entry_id in original_order
            if corpus.group(entry_id) == group
        ]
        if not ids:
            raise SystemExit(f"empty option group: {group}")
        if any(page_of(entry_id) is not None for entry_id in ids):
            ids.sort(key=page_title_key)
        else:
            ids.sort(key=lambda entry_id: title_key(by_entry[entry_id]))
        return ids

    # --- Classes ---------------------------------------------------------
    classes = []
    for title, class_id, heading_title, group in BASE_CLASSES:
        children = []
        if group is not None:
            children.append(
                heading(
                    heading_title,
                    children=[entry(e) for e in option_group(group)],
                )
            )
        else:
            samurai_orders = [
                entry_id
                for entry_id in original_order
                if corpus.group(entry_id) == "order" and page_of(entry_id) == 21
            ]
            samurai_orders.sort(key=page_title_key)
            children.append(
                heading(
                    heading_title,
                    children=[entry("ref/ability/orders")]
                    + [entry(e) for e in samurai_orders],
                )
            )
        classes.append(entry(class_id, children))

    class_archetypes = entry(
        "ref/carried/ultimate-combat/rules/class-archetypes"
    )
    class_archetypes_children = [
        entry("ref/carried/ultimate-combat/rules/class-archetypes/class-archetypes")
    ]
    for slug, title, option_title, group in CLASS_ARCHETYPES:
        class_node_id = (
            "ref/carried/ultimate-combat/rules/class-archetypes/" + slug
        )
        class_children = []
        if option_title is not None:
            if slug == "cavalier":
                orders = [
                    entry_id
                    for entry_id in original_order
                    if corpus.group(entry_id) == "order"
                    and page_of(entry_id) is not None
                    and page_of(entry_id) >= 34
                ]
                orders.sort(key=page_title_key)
                class_children.append(
                    heading(option_title, children=[entry(e) for e in orders])
                )
            else:
                class_children.append(
                    heading(
                        option_title,
                        children=[entry(e) for e in option_group(group)],
                    )
                )
        archetypes = [
            entry_id
            for entry_id in original_order
            if any(entry_id.startswith(prefix) for prefix in ARCHETYPE_PREFIX[slug])
        ]
        archetypes.sort(key=page_title_key)
        if archetypes:
            heading_title, anchor = ARCHETYPE_HEADING[slug]
            class_children.append(
                heading(
                    heading_title,
                    anchor=anchor,
                    children=[entry(e) for e in archetypes],
                )
            )
        class_archetypes_children.append(
            entry(class_node_id, class_children)
        )
    class_archetypes["children"] = class_archetypes_children
    assigned.add("ref/carried/ultimate-combat/rules/class-archetypes")
    classes.append(class_archetypes)
    chapters["Classes"]["children"] = classes

    # --- Feats -----------------------------------------------------------
    feats_group = entry(
        "ref/carried/ultimate-combat/rules/feats",
        [
            entry("ref/carried/ultimate-combat/rules/feats/types-of-feats"),
            entry("ref/carried/ultimate-combat/rules/feats/feat-descriptions"),
        ],
    )
    feats = [
        entry_id
        for entry_id in original_order
        if corpus.kind(entry_id) == "feat"
        and entry_id not in CALLED_SHOT_FEATS
    ]
    feats.sort(key=lambda entry_id: title_key(by_entry[entry_id]))
    chapters["Feats"]["children"] = [feats_group] + [entry(e) for e in feats]

    # --- Mastering Combat ------------------------------------------------
    mastering = chapters["Mastering Combat"]
    mastering_children = [
        entry("ref/carried/ultimate-combat/rules/mastering-combat/mastering-combat")
    ]

    eastern_armor = heading(
        "Eastern Armor", anchor="eastern-armor",
        children=[entry(e) for e in EASTERN_ARMOR],
    )
    weapon_special = heading(
        "Weapon Special Features",
        children=[entry(e) for e in WEAPON_SPECIAL_FEATURES],
    )
    weapon_descriptions = heading(
        "Weapon Descriptions", anchor="weapon-descriptions",
        children=[entry(e) for e in EASTERN_WEAPONS],
    )
    eastern_weapons = heading(
        "Eastern Weapons", anchor="eastern-weapons",
        children=[weapon_special, weapon_descriptions],
    )
    mastering_children.append(
        entry(
            "ref/carried/ultimate-combat/rules/mastering-combat/eastern-armor-and-weapons",
            [eastern_armor, eastern_weapons],
        )
    )

    ammo_children = []
    for entry_id in AMMUNITION:
        if entry_id == ALCHEMICAL_CARTRIDGE_GROUP:
            ammo_children.append(
                entry(
                    entry_id,
                    children=[entry(e) for e in ALCHEMICAL_CARTRIDGE_VARIANTS],
                )
            )
        else:
            ammo_children.append(entry(entry_id))
    firearm_descriptions = heading(
        "Firearm Descriptions", anchor="firearm-descriptions",
        children=[
            entry("ref/rule/firearm-description"),
            entry("ref/misc/scatter"),
            heading(
                "Early Firearms", anchor="early-firearms-4",
                children=[entry(e) for e in EARLY_FIREARMS],
            ),
            heading(
                "Advanced Firearms", anchor="advanced-firearms-4",
                children=[entry(e) for e in ADVANCED_FIREARMS],
            ),
            heading(
                "Firearm Ammunition and Adventuring Gear",
                anchor="firearm-ammunition-and-adventuring-gear",
                children=ammo_children,
            ),
        ],
    )
    firearm_magic = heading(
        "Firearm Magic Items", anchor="firearm-magic-items",
        children=[
            heading(
                "Firearm Ammunition Special Ability",
                anchor="firearm-ammunition-special-ability",
                children=[entry(e) for e in FIREARM_AMMUNITION_SPECIAL],
            ),
            heading(
                "Specific Firearm Ammunition",
                anchor="specific-firearm-ammunition",
                children=[entry(e) for e in SPECIFIC_FIREARM_AMMUNITION],
            ),
            heading(
                "Magic Firearm Special Abilities",
                anchor="magic-firearm-special-abilities",
                children=[entry(e) for e in MAGIC_FIREARM_ABILITIES],
            ),
            heading(
                "Wondrous Items", anchor="wondrous-items",
                children=[entry(e) for e in WONDROUS_FIREARM_ITEMS],
            ),
        ],
    )
    mastering_children.append(
        entry(
            "ref/rule/firearms",
            [
                entry("ref/rule/firearms-in-your-campaign"),
                entry("ref/rule/firearm-rules"),
                firearm_descriptions,
                firearm_magic,
            ],
        )
    )
    mastering_children.append(
        entry(
            "ref/carried/ultimate-combat/rules/mastering-combat/gladiator-weapons",
            [
                heading(
                    "Gladiator Weapon Descriptions",
                    anchor="gladiator-weapon-descriptions",
                    children=[entry(e) for e in GLADIATOR_WEAPONS],
                )
            ],
        )
    )
    mastering_children.append(
        entry(
            "ref/carried/ultimate-combat/rules/mastering-combat/primitive-armor-and-weapons",
            [
                heading(
                    "Primitive Materials", anchor="primitive-materials",
                    children=[entry(e) for e in PRIMITIVE_MATERIALS],
                ),
                heading(
                    "The Stone Age", anchor="the-stone-age",
                    children=[entry(e) for e in STONE_AGE],
                ),
                entry("ref/misc/broken"),
                heading(
                    "The Bronze Age", anchor="the-bronze-age",
                    children=[entry(e) for e in BRONZE_AGE],
                ),
            ],
        )
    )
    mastering_children.append(
        entry("ref/rule/duels", [entry(e) for e in DUELS])
    )
    mastering_children.append(
        entry(
            "ref/rule/performance-combat",
            [entry(e) for e in PERFORMANCE_COMBAT],
        )
    )
    siege = entry(
        "ref/rule/siege-engines-uc",
        [
            entry("ref/rule/siege-engine-rules"),
            entry("ref/rule/siege-engine-qualities"),
            heading(
                "Siege Weapon Descriptions",
                anchor="siege-weapon-descriptions",
                children=[
                    *[entry(e) for e in SIEGE_WEAPONS],
                    heading(
                        "Special Siege Engine Ammunition",
                        anchor="special-siege-engine-ammunition",
                        children=[entry(e) for e in SIEGE_AMMUNITION],
                    ),
                    heading(
                        "Close Assault Siege Engines",
                        anchor="close-assault-siege-engines",
                        children=[entry(e) for e in CLOSE_ASSAULT],
                    ),
                ],
            ),
            entry("ref/rule/structure-hardness-and-hit-points"),
            entry("ref/rule/buildings-uc"),
            entry("ref/rule/gates"),
            entry("ref/rule/walls-uc"),
        ],
    )
    mastering_children.append(siege)
    mastering["children"] = mastering_children
    assigned.add("ref/carried/ultimate-combat/rules/mastering-combat")

    # --- Vehicles --------------------------------------------------------
    full_rules = entry(
        "ref/rule/full-vehicle-rules",
        [
            entry("ref/rule/vic-initiative-and-movement", [
                entry("ref/rule/rule-vic-initiative-and-movement/movement"),
            ])
            if entry_id == "ref/rule/vic-initiative-and-movement"
            else entry(entry_id)
            for entry_id in FULL_VEHICLE_RULES
        ],
    )
    chapters["Vehicles"]["children"] = [
        entry("ref/rule/vehicles"),
        entry("ref/rule/vehicle-basics"),
        full_rules,
        entry("ref/rule/vehicle-statistics"),
        entry("ref/rule/land-vehicles", [entry(e) for e in LAND_VEHICLES]),
        entry("ref/rule/water-vehicles", [entry(e) for e in WATER_VEHICLES]),
        entry("ref/rule/air-vehicles"),
    ]

    # --- Variant Rules ---------------------------------------------------
    variant = chapters["Variant Rules"]
    variant_children = [
        entry("ref/carried/ultimate-combat/rules/variant-rules/variant-rules"),
        entry("ref/rule/armor-as-damage-reduction", [
            entry(e) for e in ARMOR_AS_DR
        ]),
    ]
    called_children = []
    for item in CALLED_SHOTS:
        if item == "CALLED_SHOT_FEATS":
            called_children.append(
                heading(
                    "Called Shot Feats", anchor="called-shot-feats",
                    children=[entry(e) for e in CALLED_SHOT_FEATS],
                )
            )
        else:
            called_children.append(entry(item))
    variant_children.append(
        entry("ref/rule/called-shots", called_children)
    )
    piecemeal_children = []
    for item in PIECEMEAL_ARMOR:
        if item == "MASTERWORK":
            piecemeal_children.append(
                entry(
                    "ref/rule/masterwork-special-material-and-magic-armor",
                    [entry(e) for e in MASTERWORK_MATERIALS],
                )
            )
        elif item == "ARMOR_PIECE_DESCRIPTIONS":
            piecemeal_children.append(
                entry(
                    "ref/rule/armor-piece-descriptions",
                    [entry(e) for e in ARMOR_PIECE_DESCRIPTIONS],
                )
            )
        else:
            piecemeal_children.append(entry(item))
    variant_children.append(
        entry("ref/rule/piecemeal-armor", piecemeal_children)
    )
    variant_children.append(
        entry(
            "ref/rule/wounds-and-vigor",
            [entry(e) for e in WOUNDS_AND_VIGOR],
        )
    )
    variant["children"] = variant_children
    assigned.add("ref/carried/ultimate-combat/rules/variant-rules")

    # --- Spells ----------------------------------------------------------
    spell_lists = entry(
        "ref/carried/ultimate-combat/rules/spell-lists",
        [
            entry("ref/carried/ultimate-combat/rules/spell-lists/order-of-presentation"),
            entry("ref/carried/ultimate-combat/rules/spell-lists/hit-dice"),
            entry("ref/carried/ultimate-combat/rules/spell-lists/caster-level"),
            entry("ref/carried/ultimate-combat/rules/spell-lists/creatures-and-characters"),
            entry("ref/carried/ultimate-combat/rules/spell-lists/communal-spells"),
        ],
    )
    spells = [
        entry_id
        for entry_id in original_order
        if corpus.kind(entry_id) == "spell"
        and (
            (page_of(entry_id) is not None and page_of(entry_id) >= 221)
            or entry_id.startswith("ref/carried/ultimate-combat/spells/")
        )
    ]
    spells.sort(key=lambda entry_id: spell_sort_key(by_entry[entry_id]))
    chapters["Spells"]["children"] = [spell_lists] + [entry(e) for e in spells]

    # --- Residual page-ordered chapter -----------------------------------
    remaining = [entry_id for entry_id in original_order if entry_id not in assigned]
    chapters["New entries (page order)"]["children"] = [
        entry(e) for e in remaining
    ]

    missing = original_set - assigned
    extra = assigned - original_set
    if missing or extra:
        raise SystemExit(
            f"entry set changed; missing={sorted(missing)} extra={sorted(extra)}"
        )
    if set(anchor_nodes) != used_anchors:
        raise SystemExit(
            "anchor set changed; missing="
            + repr(sorted(set(anchor_nodes) - used_anchors))
        )

    after_paths = {entry_id: _path_of(book, entry_id) for entry_id in original_order}
    moves = {
        entry_id: (before_paths[entry_id], after_paths[entry_id])
        for entry_id in original_order
        if before_paths[entry_id] != after_paths[entry_id]
    }
    return moves


def _path_of(book: dict, entry_id: str) -> str:
    def walk(node: dict, trail: list[str]) -> str | None:
        here = trail + [node["title"]]
        if node.get("entry") == entry_id:
            return " > ".join(here)
        for child in node.get("children", []):
            found = walk(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter, [])
        if found is not None:
            return found
    return "(missing)"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus", type=Path)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero unless books.toml already matches the target",
    )
    args = parser.parse_args()

    books_path = args.corpus / BOOKS_FILE
    original = books_path.read_text(encoding="utf-8")
    books = tomllib.loads(original)
    book = find_book(books)
    corpus = Corpus(args.corpus)
    moves = transform(book, corpus)
    rendered = dump_books(books)

    if args.check:
        if rendered != original:
            print(
                f"books.toml is not canonical for {BOOK}; "
                f"{len(moves)} entry paths would move",
                file=sys.stderr,
            )
            return 1
        print(
            f"books.toml is canonical for {BOOK}; "
            f"entries placed: {len(flatten_book(book))}"
        )
        return 0

    books_path.write_text(rendered, encoding="utf-8")
    print(f"books.toml rewritten for {BOOK}; entry paths moved: {len(moves)}")
    return 0


def flatten_book(book: dict) -> list[dict]:
    found: list[dict] = []
    for chapter in book.get("chapters", []):
        found.extend(flatten(chapter))
    return [node for node in found if node.get("entry")]


if __name__ == "__main__":
    raise SystemExit(main())
