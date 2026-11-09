#!/usr/bin/env python3
"""Place the Ultimate Campaign tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/ultimate-campaign-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.

The residual ``New entries (page order)`` chapter keeps the cross-book category
hubs whose Ultimate Campaign attribution is a secondary source and which have
no printed Ultimate Campaign section.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

BOOK = "ultimate-campaign"
BOOKS_FILE = "books.toml"


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


def find_book(books: dict) -> dict:
    for book in books["books"]:
        if book["source"] == BOOK:
            return book
    raise SystemExit(f"unknown book source: {BOOK}")


def walk(node: dict):
    yield node
    for child in node.get("children", []):
        yield from walk(child)


def find_node(book: dict, entry_id: str) -> dict | None:
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry") == entry_id:
                return node
    return None


def find_title(node: dict, title: str) -> dict | None:
    for candidate in walk(node):
        if candidate.get("title") == title:
            return candidate
    return None


def parent_of(book: dict, node: dict) -> dict | None:
    for chapter in book.get("chapters", []):
        for candidate in walk(chapter):
            if node in candidate.get("children", []):
                return candidate
    return None


def detach(book: dict, node: dict) -> None:
    parent = parent_of(book, node)
    if parent is None:
        raise SystemExit(f"{BOOK}: node {node.get('title')} is already a chapter")
    parent["children"].remove(node)
    if not parent["children"]:
        parent.pop("children", None)


def take(book: dict, entry_id: str) -> dict:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"{BOOK}: missing entry {entry_id}")
    detach(book, node)
    return node


def book_entries(book: dict) -> set[str]:
    placed: set[str] = set()
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry"):
                placed.add(node["entry"])
    return placed


def path_of(book: dict, node: dict) -> str:
    def walk_path(current: dict, trail: list[str]) -> list[str] | None:
        here = trail + [current["title"]]
        if current is node:
            return here
        for child in current.get("children", []):
            found = walk_path(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk_path(chapter, [])
        if found is not None:
            return " > ".join(found)
    return "(missing)"


UC_PAGE = re.compile(r"Ultimate Campaign(?: pg\.?| p\.) ?(\d+)")


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = UC_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


# ---------------------------------------------------------------------------
# Printed sections

INTRO = "ref/carried/ultimate-campaign/rules/character-background/character-background"
TRAITS = "ref/carried/ultimate-campaign/rules/character-background/traits"
CS_INTRO = "ref/carried/ultimate-campaign/rules/campaign-systems/campaign-systems"
ALIGNMENT = "ref/carried/ultimate-campaign/rules/campaign-systems/alignment"
LINEAGE = "ref/carried/ultimate-campaign/rules/campaign-systems/lineage"

CLASS_TABLES = [
    "ref/rule/rule-class-background-tables/alchemist",
    "ref/rule/rule-class-background-tables/barbarian",
    "ref/rule/rule-class-background-tables/bard",
    "ref/rule/rule-class-background-tables/cleric",
    "ref/rule/rule-class-background-tables/druid",
    "ref/rule/rule-class-background-tables/fighter",
    "ref/rule/rule-class-background-tables/inquisitor",
    "ref/rule/rule-class-background-tables/magus",
    "ref/rule/rule-class-background-tables/monk",
    "ref/rule/rule-class-background-tables/paladin",
    "ref/rule/rule-class-background-tables/ranger",
    "ref/rule/rule-class-background-tables/rogue",
    "ref/rule/rule-class-background-tables/sorcerer",
    "ref/rule/rule-class-background-tables/wizard",
]

STORY_FEATS = [
    "ref/feat/accursed", "ref/feat/arisen", "ref/feat/battlefield-healer",
    "ref/feat/champion", "ref/feat/damned", "ref/feat/deny-the-reaper",
    "ref/feat/eldritch-researcher", "ref/feat/fearless-zeal",
    "ref/feat/feral-heart", "ref/feat/foeslayer", "ref/feat/forgotten-past",
    "ref/feat/glimpse-beyond", "ref/feat/innocent-blood",
    "ref/feat/liberator", "ref/feat/lost-legacy", "ref/feat/magnum-opus",
    "ref/feat/monument-builder", "ref/feat/nation-builder", "ref/feat/nemesis",
    "ref/feat/prophet", "ref/feat/redemption", "ref/feat/shamed",
    "ref/feat/stronghold", "ref/feat/thief-of-legend", "ref/feat/town-tamer",
    "ref/feat/true-love", "ref/feat/unforgotten", "ref/feat/vengeance",
]

ACTIVITIES = [
    "ref/rule/add-spells-to-your-spellbook", "ref/rule/construct-buildings",
    "ref/rule/craft-magic-items", "ref/rule/craft-mundane-items",
    "ref/rule/earn-capital", "ref/rule/earn-xp", "ref/rule/gather-information",
    "ref/rule/heal-others", "ref/rule/lead-your-kingdom",
    "ref/rule/promote-a-business", "ref/rule/recruit-for-an-organization",
    "ref/rule/replace-your-animal-companion", "ref/rule/replace-your-familiar",
    "ref/rule/research-a-spell", "ref/rule/research-facts-and-lore",
    "ref/rule/rest", "ref/rule/retrain", "ref/rule/run-a-business",
    "ref/rule/scheme-for-an-upcoming-adventure", "ref/rule/train-an-animal",
]

ROOMS = [
    "ref/misc/alchemy-lab", "ref/misc/altar", "ref/misc/animal-pen",
    "ref/misc/armory", "ref/misc/artisans-workshop", "ref/misc/auditorium",
    "ref/misc/ballroom", "ref/misc/bar", "ref/misc/bath",
    "ref/misc/battle-ring", "ref/misc/bedroom", "ref/misc/bell-tower",
    "ref/misc/book-repository", "ref/misc/brewery", "ref/misc/bunks",
    "ref/misc/burial-ground", "ref/misc/cell", "ref/misc/ceremonial-room",
    "ref/misc/classroom", "ref/misc/clockwork-shop", "ref/misc/common-room",
    "ref/misc/confessional", "ref/misc/courtyard", "ref/misc/crypt",
    "ref/misc/defensive-wall", "ref/misc/dock", "ref/misc/dojo",
    "ref/misc/drawbridge", "ref/misc/escape-route", "ref/misc/false-front",
    "ref/misc/farmland", "ref/misc/forge", "ref/misc/game-room",
    "ref/misc/garden", "ref/misc/gatehouse", "ref/misc/gauntlet",
    "ref/misc/greenhouse", "ref/misc/guard-post", "ref/misc/habitat",
    "ref/misc/hatchery", "ref/misc/infirmary", "ref/misc/kitchen",
    "ref/misc/labyrinth", "ref/misc/laundry", "ref/misc/lavatory",
    "ref/misc/leather-workshop", "ref/misc/lodging",
    "ref/misc/magical-repository", "ref/misc/mill-room", "ref/misc/nursery",
    "ref/misc/observation-dome", "ref/misc/office", "ref/misc/pit",
    "ref/misc/printer", "ref/misc/reliquary", "ref/misc/sanctum",
    "ref/misc/sauna", "ref/misc/scriptorium", "ref/misc/scrying-room",
    "ref/misc/secret-room", "ref/misc/sewer-access", "ref/misc/sewing-room",
    "ref/misc/shack", "ref/misc/sitting-room", "ref/misc/sports-field",
    "ref/misc/stall", "ref/misc/statue", "ref/misc/storage",
    "ref/misc/storefront", "ref/misc/summmoning-chamber",
    "ref/misc/throne-room", "ref/misc/tollbooth", "ref/misc/torture-chamber",
    "ref/misc/trap", "ref/misc/trophy-room", "ref/misc/vault",
    "ref/misc/war-room", "ref/misc/workstation",
    "ref/rule/room-augmentations",
]

TEAMS = [
    "ref/misc/acolyte", "ref/misc/apprentice", "ref/misc/archers",
    "ref/misc/bureaucrats", "ref/misc/cavalry-archers",
    "ref/misc/craftspeople", "ref/misc/cutpurses", "ref/misc/drivers",
    "ref/misc/elite-archers", "ref/misc/elite-guards",
    "ref/misc/elite-soldiers", "ref/misc/guards", "ref/misc/laborers",
    "ref/misc/lackeys", "ref/misc/mage", "ref/misc/priest",
    "ref/misc/robbers", "ref/misc/sage", "ref/misc/sailors",
    "ref/misc/scofflaws", "ref/misc/soldiers",
]

BUILDING_EVENTS = [
    "ref/rule/alchemist-events", "ref/rule/bardic-college-events",
    "ref/rule/casters-tower-events", "ref/rule/castle-events",
    "ref/rule/dance-hall-events", "ref/rule/guildhall-events",
    "ref/rule/herbalist-events", "ref/rule/house-events",
    "ref/rule/inn-events", "ref/rule/library-events",
    "ref/rule/magical-academy-events", "ref/rule/magic-shop-events",
    "ref/rule/menagerie-events", "ref/rule/military-academy-events",
    "ref/rule/monastery-events", "ref/rule/shop-events",
    "ref/rule/smithy-events", "ref/rule/stable-events",
    "ref/rule/tavern-events", "ref/rule/temple-events",
    "ref/rule/theater-events",
]

ORGANIZATION_EVENTS = [
    "ref/rule/cabal-events", "ref/rule/cult-events",
    "ref/rule/mercenary-company-events", "ref/rule/thieves-guild-events",
]

LEADERS = [
    "ref/rule/consort", "ref/rule/councilor", "ref/rule/ruler",
    "ref/rule/general", "ref/rule/grand-diplomat", "ref/rule/heir",
    "ref/rule/high-priest", "ref/rule/magister",
    "ref/rule/who-rolls-the-kingdom-check", "ref/rule/marshal",
    "ref/rule/royal-enforcer", "ref/rule/spymaster",
    "ref/rule/treasurer", "ref/rule/viceroy", "ref/rule/warden",
]

TERRAIN = [
    "ref/rule/aqueduct", "ref/rule/bridge", "ref/rule/canal",
    "ref/rule/farm", "ref/rule/fishery", "ref/rule/fort",
    "ref/rule/highway", "ref/rule/mine", "ref/rule/quarry",
    "ref/rule/road", "ref/rule/sawmill", "ref/rule/special-terrain",
    "ref/rule/watchtower",
]

KINGDOM_BUILDINGS = [
    "ref/rule/building-descriptions",
    "ref/rule/kbuildings1", "ref/rule/kbuildings2", "ref/rule/kbuildings3",
    "ref/rule/kbuildings4", "ref/rule/kbuildings5", "ref/rule/kbuildings6",
    "ref/rule/kbuildings7", "ref/rule/gaining-experience-for-leadership",
]

OPTIONAL_KINGDOM = [
    "ref/rule/abandoned-buildings", "ref/rule/deities-and-holy-sites",
    "ref/rule/diplomatic-edicts", "ref/rule/special-edicts",
    "ref/rule/type-of-diplomatic-relationships",
    "ref/rule/expanding-settlement-modifiers", "ref/rule/exploration-edicts",
    "ref/rule/fame-and-infamy", "ref/rule/relationships-with-multiple-kingdoms",
    "ref/rule/declaring-independence", "ref/rule/forms-of-government",
    "ref/rule/independence-and-unification", "ref/rule/forming-a-union",
    "ref/rule/leadership-role-skills", "ref/rule/settlement-sizes",
    "ref/rule/trade-edicts", "ref/rule/vassalage-edicts",
]

SPECIAL_ABILITIES = [
    "ref/rule/rule-special-abilities-ucp/amphibious",
    "ref/rule/rule-special-abilities-ucp/bleed",
    "ref/rule/rule-special-abilities-ucp/blindsense",
    "ref/rule/rule-special-abilities-ucp/blindsight",
    "ref/rule/rule-special-abilities-ucp/breath-weapon",
    "ref/rule/rule-special-abilities-ucp/disease",
    "ref/rule/rule-special-abilities-ucp/earth-glide",
    "ref/rule/rule-special-abilities-ucp/energy-drain",
    "ref/rule/rule-special-abilities-ucp/fast-healing",
    "ref/rule/rule-special-abilities-ucp/fear",
    "ref/rule/rule-special-abilities-ucp/ferocity",
    "ref/rule/rule-special-abilities-ucp/flight",
    "ref/rule/rule-special-abilities-ucp/grab",
    "ref/rule/rule-special-abilities-ucp/immunity",
    "ref/rule/rule-special-abilities-ucp/incorporeal",
    "ref/rule/rule-special-abilities-ucp/light-blindness",
    "ref/rule/rule-special-abilities-ucp/light-sensitivity",
    "ref/rule/rule-special-abilities-ucp/low-light-vision",
    "ref/rule/rule-special-abilities-ucp/paralysis",
    "ref/rule/rule-special-abilities-ucp/rake",
    "ref/rule/rule-special-abilities-ucp/regeneration",
    "ref/rule/rule-special-abilities-ucp/rend",
    "ref/rule/rule-special-abilities-ucp/rock-catching",
    "ref/rule/rule-special-abilities-ucp/rock-throwing",
    "ref/rule/rule-special-abilities-ucp/scent",
    "ref/rule/rule-special-abilities-ucp/spell-resistance",
    "ref/rule/rule-special-abilities-ucp/tremorsense",
    "ref/rule/rule-special-abilities-ucp/trip",
    "ref/rule/rule-special-abilities-ucp/unnatural-aura",
]


RELIGION_TRAITS = [
    "ref/trait/calistrian-courtesan", "ref/trait/child-of-nature",
    "ref/trait/corpse-cannibal", "ref/trait/demon-hunter",
    "ref/trait/divine-warrior", "ref/trait/ear-for-music",
    "ref/trait/eyes-and-ears-of-the-city", "ref/trait/flame-of-the-dawnflower",
    "ref/trait/fortified-drinker", "ref/trait/guardian-of-the-forge",
    "ref/trait/magic-is-life", "ref/trait/pain-is-pleasure",
    "ref/trait/patient-optimist", "ref/trait/shadow-whispers",
    "ref/trait/starchild", "ref/trait/undead-slayer-pharasma",
    "ref/trait/veteran-of-battle", "ref/trait/voice-of-monsters",
    "ref/trait/wisdom-in-the-flesh", "ref/trait/wrecking-wrath",
]


def e(*entry_ids: str) -> list:
    return [("e", entry) for entry in entry_ids]


def ec(entry_id: str, children: list) -> tuple:
    return ("ec", entry_id, children)


def g(title: str, children: list) -> tuple:
    return ("g", title, children)


def chapter_specs() -> dict[str, list]:
    return {
        "Character Background": [
            ("keep", INTRO),
            ec("ref/rule/how-to-create-a-background", e(
                "ref/rule/brainstorming-your-background",
                "ref/rule/character-background",
                "ref/rule/creating-a-unique-character-concept",
            )),
            ec("ref/rule/early-life", e(
                "ref/rule/circumstances-of-birth",
                "ref/rule/using-this-section",
                "ref/rule/family",
                "ref/rule/magic-2",
                "ref/rule/region",
                "ref/rule/social-rank-and-education",
            )),
            ec("ref/rule/adolescence", e(
                "ref/rule/class-event",
                "ref/rule/friends-allies-and-influences",
                "ref/rule/initiation-rites",
                "ref/rule/major-childhood-event",
                "ref/rule/religion-and-ritual",
                "ref/rule/duty-and-responsibilities",
                "ref/rule/first-love",
            )),
            ec("ref/rule/adulthood", e(
                "ref/rule/conflict-and-behavior",
                "ref/rule/vulnerability",
                "ref/rule/friends-associates-and-companions",
                "ref/rule/quick-establishment",
            )),
            ec("ref/rule/background-generator", [
                ec("ref/rule/step-1-homeland-family-childhood", e(
                    "ref/rule/dwarf-background",
                    "ref/rule/elf-background",
                    "ref/rule/gnome-background",
                    "ref/rule/half-elf-background",
                    "ref/rule/halfling-background",
                    "ref/rule/half-orc-background",
                    "ref/rule/human-background",
                    "ref/rule/siblings",
                    "ref/rule/birth-circumstances-and-more",
                    "ref/rule/unusual-homeland",
                )),
                ec("ref/rule/step-2-adolescence-and-training", [
                    ec("ref/rule/class-background-tables", e(*CLASS_TABLES)),
                    ("e", "ref/rule/influential-associates-table"),
                ]),
                ec("ref/rule/step-3-moral-conflicts-relationships-and-drawbacks", e(
                    "ref/rule/romantic-relationships",
                    "ref/rule/character-drawback-table",
                )),
            ]),
            ec(TRAITS, [
                ("keep-title", "Basic Traits"),
                ("keep-title", "Race Traits"),
                ("keep-title", "Regional Traits"),
                g("Religion Traits", e(*RELIGION_TRAITS)),
                ("keep-title", "Drawbacks"),
            ]),
            g("Story Feats", [
                ("e", "ref/rule/feats-roleplaying"),
                *e(*STORY_FEATS),
            ]),
        ],
        "Downtime": [
            ec("ref/rule/downtime", [
                ("e", "ref/rule/downtime-overview"),
                ("e", "ref/rule/downtime-terminology"),
                ("e", "ref/rule/downtime-and-kingdom-building"),
                ec("ref/rule/gaining-capital", e(
                    "ref/rule/working-weekends",
                    "ref/rule/converting-capital",
                    "ref/rule/moving-capital",
                    "ref/rule/spending-limits",
                    "ref/rule/using-followers",
                )),
                ec("ref/rule/downtime-phases", e(
                    "ref/rule/spend-capital-to-boost-checks",
                    "ref/rule/upkeep-phase",
                    "ref/rule/activity-phase",
                    "ref/rule/income-phase",
                    "ref/rule/event-phase",
                )),
            ]),
            ec("ref/rule/downtime-activities", e(*ACTIVITIES)),
            ec("ref/rule/managers", e(
                "ref/rule/example-managers",
                "ref/rule/reading-a-manager-stat-block",
            )),
            ec("ref/rule/rooms-and-teams", [
                ("e", "ref/rule/construction-examples"),
                ("e", "ref/rule/reading-a-unit-statblock"),
                ("e", "ref/rule/construction-and-recruitment-delays"),
                ("e", "ref/rule/moving-an-organization"),
                ("e", "ref/rule/restrictions-on-earnings"),
                g("Rooms", e(*ROOMS)),
                g("Teams", e(*TEAMS)),
            ]),
            ec("ref/rule/buildings-and-organizations", [
                ec("ref/rule/buildings", e(
                    "ref/rule/buildings1", "ref/rule/buildings2",
                    "ref/rule/buildings3", "ref/rule/buildings4",
                    "ref/rule/buildings5", "ref/rule/buildings6",
                    "ref/rule/buildings7",
                )),
                ("e", "ref/rule/organizations"),
            ]),
            ec("ref/rule/downtime-events", [
                ec("ref/rule/generic-building-events", e(*BUILDING_EVENTS)),
                ec("ref/rule/organization-events", e(*ORGANIZATION_EVENTS)),
            ]),
        ],
        "Campaign Systems": [
            ("keep", CS_INTRO),
            ec(ALIGNMENT, e(
                "ref/rule/lawful-good",
                "ref/rule/the-nine-alignments",
                "ref/rule/chaotic-good",
                "ref/rule/lawful-neutral",
                "ref/rule/neutral",
                "ref/rule/neutral-good",
                "ref/rule/changing-alignment-ucp",
                "ref/rule/chaotic-evil",
                "ref/rule/chaotic-neutral",
                "ref/rule/lawful-evil",
                "ref/rule/neutral-evil",
            )),
            ec("ref/rule/bargaining", e(
                "ref/rule/bargaining-rules",
                "ref/rule/steps-of-bargaining",
                "ref/rule/examples-of-bargaining",
                "ref/rule/other-elements-of-bargaining",
            )),
            ec("ref/rule/rule-special-abilities-ucp/companions", [
                ("e", "ref/rule/aspects-of-control"),
                ("e", "ref/rule/companions"),
                ("e", "ref/rule/controlling-companions"),
                ("e", "ref/rule/issues-of-control"),
                ("e", "ref/rule/advancing-companions"),
                ("e", "ref/rule/intelligent-animals"),
                ("e", "ref/rule/remembering-companions"),
                ("e", "ref/rule/animal-companion"),
                ("e", "ref/rule/companion-plot-hooks"),
                ("e", "ref/rule/cohort"),
                ("e", "ref/rule/eidolon"),
                ("e", "ref/rule/familiar"),
                ("e", "ref/rule/follower"),
                ec("ref/rule/reviving-and-replacing-companions", e(
                    "ref/rule/finding-a-replacement",
                    "ref/rule/reviving-a-dead-companion",
                )),
            ]),
            ec("ref/rule/contacts", e(
                "ref/rule/risk",
                "ref/rule/trust",
                "ref/rule/negotiation-checks",
                "ref/rule/gaining-cultivating-and-losing-contacts",
                "ref/rule/types-of-contacts",
            )),
            ec("ref/rule/exploration-ucp", [
                ("e", "ref/rule/sandbox-exploration"),
                ec("ref/rule/movement-ucp", e(
                    "ref/rule/keep-things-interesting",
                    "ref/rule/getting-lost-ucp",
                )),
                ("e", "ref/rule/random-encounters"),
                ("e", "ref/rule/planned-encounters"),
                ec("ref/rule/hex-terrain-types", e(
                    "ref/rule/desert", "ref/rule/forest", "ref/rule/hill",
                    "ref/rule/marsh", "ref/rule/mountain", "ref/rule/plain",
                    "ref/rule/settlement", "ref/rule/water-ucp",
                )),
                ("e", "ref/rule/random-map-generation"),
                ("e", "ref/rule/exploration-hazards"),
            ]),
            ec("ref/rule/honor", e(
                "ref/rule/honor-points",
                "ref/rule/consequences-of-losing-honor",
                "ref/rule/gaining-and-losing-honor",
                "ref/rule/spending-honor",
                "ref/rule/chivalric-code",
                "ref/rule/criminal-code",
                "ref/rule/political-code",
                "ref/rule/samurai-code",
                "ref/rule/tribal-code",
            )),
            ec("ref/rule/investment", e(
                "ref/rule/investment-mechanics",
                "ref/rule/example-investments-and-problems",
            )),
            ec(LINEAGE, [
                ("e", "ref/rule/complicated-and-dysfunctional-families"),
                ("e", "ref/rule/developing-your-family"),
                ("e", "ref/rule/lineage"),
                ("e", "ref/rule/family-values"),
                ("e", "ref/rule/including-families-in-the-game"),
                ("e", "ref/rule/family-members-as-foes"),
                ("e", "ref/rule/inheritance"),
                ("e", "ref/rule/support-and-obligation"),
                ("e", "ref/rule/death-in-the-family"),
                ("e", "ref/rule/famous-lineages"),
            ]),
            ec("ref/rule/magic-item-creation-ucp", e(
                "ref/rule/cooperative-crafting",
                "ref/rule/pricing-new-items",
                "ref/rule/upgrading-items",
                "ref/rule/recharging-charged-items",
                "ref/rule/altering-existing-items",
                "ref/rule/adjusting-character-wealth-by-level",
                "ref/rule/creating-items-for-profit",
                "ref/rule/talismanic-components",
                "ref/rule/components-as-treasure",
                "ref/rule/example-components",
                "ref/rule/requiring-components",
                "ref/rule/using-components",
            )),
            ec("ref/rule/relationships", e(
                "ref/rule/relationship-levels",
                "ref/rule/example-relationships",
                "ref/rule/growing-relationships",
                "ref/rule/parent",
                "ref/rule/reversing-relationships",
                "ref/rule/childhood-rival",
                "ref/rule/sibiling",
                "ref/rule/spouse",
            )),
            ec("ref/rule/reputation-and-fame", e(
                "ref/rule/fame",
                "ref/rule/sphere-of-influence",
                "ref/rule/fame-1",
                "ref/rule/fame-10",
                "ref/rule/prestige-points",
                "ref/rule/fame-20",
                "ref/rule/fame-30",
                "ref/rule/fame-40",
                "ref/rule/fame-55",
                "ref/rule/alter-egos-aliases-and-secret-identities",
                "ref/rule/shared-fame",
            )),
            ec("ref/rule/retirement", e(
                "ref/rule/relationship-to-the-pcs",
                "ref/rule/taking-a-different-path",
                "ref/rule/the-simple-life",
                "ref/rule/betrayal",
            )),
            ec("ref/rule/retraining", e(
                "ref/rule/ability-score-increase",
                "ref/rule/archetype",
                "ref/rule/class-feature",
                "ref/rule/class-level",
                "ref/rule/new-language",
                "ref/rule/feat",
                "ref/rule/hit-points-ucp",
                "ref/rule/racial-trait",
                "ref/rule/skill-ranks",
                "ref/rule/spells-known",
            )),
            ec("ref/rule/taxation", e(
                "ref/rule/types-of-services",
                "ref/rule/types-of-taxes",
                "ref/rule/compliance",
            )),
            ec("ref/rule/young-characters", e(
                "ref/rule/creating-young-characters",
                "ref/rule/leaving-youth-behind",
                "ref/rule/considerations-of-youth",
            )),
        ],
        "Kingdoms and War": [
            ("keep", "ref/rule/kingdoms-and-war"),
            ec("ref/rule/kingdom-building", [
                ("e", "ref/rule/kingdom-building-overview"),
                ("e", "ref/rule/kingdom-terminology"),
                ("e", "ref/rule/kingdom-building-quick-reference"),
                ("e", "ref/rule/founding-a-kingdom"),
                ec("ref/rule/leadership-roles", e(*LEADERS)),
                ec("ref/rule/build-points", e(
                    "ref/rule/starting-from-scratch",
                    "ref/rule/wealthy-sponsor",
                )),
                ec("ref/rule/kingdom-turn-sequence", e(
                    "ref/rule/kingdom-edict-phase",
                    "ref/rule/kingdom-upkeep-phase",
                    "ref/rule/capital-city",
                    "ref/rule/kingdom-income-phase",
                    "ref/rule/kingdom-event-phase",
                )),
                ec("ref/rule/edicts", e(
                    "ref/rule/holiday-edicts",
                    "ref/rule/improvement-edicts",
                    "ref/rule/promotion-edicts",
                    "ref/rule/taxation-edicts",
                )),
                ("e", "ref/rule/losing-hexes"),
                ec("ref/rule/terrain-improvements", e(*TERRAIN)),
                ec("ref/rule/settlements-and-districts", e(
                    "ref/rule/founding-a-settlement",
                    "ref/rule/buildings-in-the-kingdom",
                    "ref/rule/claiming-water-and-islands",
                    "ref/rule/magic-items-in-settlements",
                )),
                g("Buildings", e(*KINGDOM_BUILDINGS)),
                ec("ref/rule/events", e(
                    "ref/rule/events-beneficial-kingdom",
                    "ref/rule/events-beneficial-settlement",
                    "ref/rule/events-dangerous-kingdom",
                    "ref/rule/events-dangerous-settlement",
                )),
                ec("ref/rule/optional-kingdom-rules", e(*OPTIONAL_KINGDOM)),
            ]),
            ec("ref/rule/mass-combat", [
                ("e", "ref/rule/mass-combat-overview"),
                ec("ref/rule/army-statistics", e(
                    "ref/rule/mass-combat-quick-reference-guide",
                )),
                ec("ref/rule/battle-phases", e(
                    "ref/rule/attacking-and-taking-damage",
                )),
                ("e", "ref/rule/beyond-the-kingdom"),
                ("e", "ref/rule/army-tactics"),
                ("e", "ref/rule/battlefield-conditions"),
                ("e", "ref/rule/more-than-two-armies"),
                ("e", "ref/rule/running-mass-combat"),
                ("e", "ref/rule/strategy-track"),
                ("e", "ref/rule/rout"),
                ("e", "ref/rule/victory-rout-or-defeat"),
                ("e", "ref/rule/recovery"),
                ec("ref/rule/commanders", e(
                    "ref/rule/boons",
                    "ref/rule/kingdom-leader-commanders",
                    "ref/rule/losing-commanders",
                    "ref/rule/recruiting-armies",
                )),
                ec("ref/rule/army-resources", e(
                    "ref/rule/transfering-resources",
                )),
                ec("ref/rule/special-abilities-ucp", e(*SPECIAL_ABILITIES)),
                ("e", "ref/rule/player-characters-in-battles"),
                ec("ref/rule/optional-mass-combat-rules", e(
                    "ref/rule/combining-armies",
                    "ref/rule/reforming-an-army",
                    "ref/rule/reserve-army",
                    "ref/rule/splitting-an-army",
                )),
                ec("ref/rule/sample-armies", e(
                    "ref/rule/army-examples-common-humanoid",
                    "ref/rule/army-examples-human",
                    "ref/rule/army-examples-monstrous",
                    "ref/rule/army-examples-uncommon-humanoid",
                    "ref/rule/army-examples-undead",
                )),
            ]),
        ],
    }


def build(spec: list, book: dict, root: dict) -> list[dict]:
    out: list[dict] = []
    for item in spec:
        kind = item[0]
        if kind == "e":
            node = take(book, item[1])
            if node.get("children"):
                raise SystemExit(
                    f"{BOOK}: entry {item[1]} still holds children; make it ec"
                )
            out.append(node)
        elif kind == "keep":
            node = find_node(book, item[1])
            if node is None:
                raise SystemExit(f"{BOOK}: missing entry {item[1]}")
            out.append(node)
        elif kind == "keep-title":
            node = find_title(root, item[1])
            if node is None:
                raise SystemExit(f"{BOOK}: missing group {item[1]}")
            out.append(node)
        elif kind == "ec":
            node = find_node(book, item[1])
            if node is None:
                raise SystemExit(f"{BOOK}: missing entry {item[1]}")
            children = build(item[2], book, node)
            detach(book, node)
            if children:
                node["children"] = children
            else:
                node.pop("children", None)
            out.append(node)
        elif kind == "g":
            node = find_title(root, item[1])
            if node is None or node.get("entry") is not None:
                node = {"title": item[1]}
            children = build(item[2], book, node)
            if children:
                node["children"] = children
            else:
                node.pop("children", None)
            out.append(node)
        else:
            raise SystemExit(f"{BOOK}: bad spec {item!r}")
    return out


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    titles = {
        node["entry"]: node["title"]
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }
    before_paths = {
        node["entry"]: path_of(book, node)
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }

    chapters = {chapter["title"]: chapter for chapter in book["chapters"]}
    for title, spec in chapter_specs().items():
        chapter = chapters[title]
        chapter["children"] = build(spec, book, chapter)

    after_paths = {
        node["entry"]: path_of(book, node)
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }

    if set(before_paths) != set(after_paths):
        missing = sorted(set(before_paths) - set(after_paths))
        added = sorted(set(after_paths) - set(before_paths))
        raise SystemExit(f"entry set changed: lost {missing}, added {added}")

    moves = []
    for entry_id in sorted(before_paths):
        if before_paths[entry_id] != after_paths[entry_id]:
            moves.append((
                titles[entry_id],
                before_paths[entry_id],
                after_paths[entry_id],
                page_of(corpus, entry_id),
            ))
    return moves


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    books_path = args.corpus / BOOKS_FILE
    original_text = books_path.read_text(encoding="utf-8")
    books = tomllib.loads(original_text)
    book = find_book(books)
    before = book_entries(book)
    moves = transform(book, args.corpus)
    after = book_entries(book)
    if before != after:
        print(f"{BOOK}: entry set changed", file=sys.stderr)
        return 1

    book["basis"] = (
        "The chapter titles, their order, and their first pages are checked "
        "against the printed contents page of Ultimate Campaign. Each chapter "
        "holds its printed section headings, and each entry sits in its "
        "printed section in page order. See docs/research/book-order-evidence.md. "
        "Canonical ids were substituted through the reviewed alias map; chapters "
        "and order are unchanged."
    )
    book["review"] = [
        "The printed Downtime Tracking Sheet (p. 130) and the Kingdom Building sheet and icon headings (pp. 224-227) have no corpus entries.",
        "The dataset's Story Feats split stays inside the printed Character Background chapter, below Drawbacks.",
        "Entries that share a printed page keep the dataset order inside that page.",
        "Canonical ids were substituted through sources/canonical/id-map.toml. A legacy entry with no installed target is omitted and listed in coverage.",
    ]

    text = dump_books(books)
    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the nested Ultimate Campaign shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {BOOK}; entries placed: {len(after)}")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} moves, {len(after)} entries placed)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
