#!/usr/bin/env python3
"""Place the Technology Guide tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or a linked anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/technology-guide-nested.py /path/to/cast-iron-data
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

BOOK = "technology-guide"
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

TG_PAGE = re.compile(r"Technology Guide\s+(?:pg\.?|p\.)\s*(\d+)")

# Entries the compiler carries without a Sources line. The page is the printed
# heading or item description they belong to.
PAGE_OVERRIDES: dict[str, int] = {
    # Chapter openers and dataset section hubs.
    "ref/carried/technology-guide/rules/technology-in-the-world": 4,
    "ref/carried/technology-guide/rules/technology-in-the-world/skills": 5,
    "ref/carried/technology-guide/rules/technology-in-the-world/feats": 6,
    "ref/carried/technology-guide/rules/technology-in-the-world/spells": 8,
    "ref/carried/technology-guide/rules/technology-in-the-world/archtypes": 12,
    "ref/carried/technology-guide/rules/technology-in-the-world/crafting": 16,
    "ref/carried/technology-guide/rules/equipment": 18,
    "ref/carried/technology-guide/rules/equipment/introduction": 19,
    "ref/carried/technology-guide/rules/equipment/weapons": 20,
    "ref/carried/technology-guide/rules/equipment/armor": 29,
    "ref/carried/technology-guide/rules/equipment/gear": 39,
    "ref/carried/technology-guide/rules/hazards-and-artifacts": 54,
    "ref/carried/technology-guide/rules/hazards-and-artifacts/hazards": 55,
    "ref/carried/technology-guide/rules/hazards-and-artifacts/artifical-intelligence": 58,
    "ref/carried/technology-guide/rules/hazards-and-artifacts/artifacts": 60,
    # An item whose printed description starts on its variants' page.
    "ref/carried/technology-guide/rules/equipment/armor/scatterlight-suit": 30,
    "ref/carried/technology-guide/rules/equipment/pharmaceuticals/hemochem": 34,
    "ref/carried/technology-guide/rules/equipment/cybertech/cyberfiber-muscles": 36,
    "ref/carried/technology-guide/rules/equipment/cybertech/dermal-plating": 37,
    "ref/carried/technology-guide/rules/equipment/cybertech/thoracic-nanite-chamber": 38,
    "ref/carried/technology-guide/rules/equipment/cybertech/wirejack-tendons": 38,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/access-card": 39,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/chipfinder": 40,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/cortex-gun": 42,
    "ref/tech-misc/e-pick": 43,
    "ref/tech-misc/force-field": 45,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/hologram-generator": 47,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/lock-coder": 49,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/nanite-hypogun": 50,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/skillchip": 52,
    "ref/carried/technology-guide/rules/equipment/gear/technological-gear/on-signals/veemod": 53,
    # PSRD spells the Pf Data records name without a page.
    "ref/carried/technology-guide/rules/technology-in-the-world/spells/discharge/greater": 9,
    "ref/carried/technology-guide/rules/technology-in-the-world/spells/make-whole/greater": 10,
    "ref/carried/technology-guide/rules/technology-in-the-world/spells/remove-radioactivity/greater": 11,
    # The aggregate template prints on its heading page.
    "ref/template/aggregate": 59,
}

# The two dataset group headings that an entry now carries. Their anchors were
# synthesised from the title and no entry link targets them.
DROPPED_HEADINGS: set[tuple[str, str | None]] = {
    ("On Signals", "on-signals"),
    ("Traps and Technology", "traps-and-technology"),
}

# Entries with no Technology Guide page stay in the page-ordered final chapter.
RESIDUAL_ENTRIES: list[str] = [
    # The base spell the Technology Guide's greater version refers to.
    "ref/spell/make-whole",
    # A cross-book hub with no Technology Guide section.
    "ref/rule/gm-specific-campaigns",
]


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = TG_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


# ---------------------------------------------------------------------------
# Printed structure
#
# Each node is ("entry", id, children) or ("heading", title, anchor, children).
# Every chapter below is a printed chapter; every heading below is a printed
# heading. Entries that share a printed page keep the dataset order.

E = "entry"
H = "heading"


def entry(entry_id: str, children: list | None = None) -> tuple:
    return (E, entry_id, children or [])


def heading(title: str, anchor: str | None, children: list) -> tuple:
    return (H, title, anchor, children)


CARRIED = "ref/carried/technology-guide/rules"

# Chapter 1: Technology in the World (pp. 4-17).
SKILLS = [
    "ref/skill/craft-any",
    "ref/skill/disable-device",
    "ref/skill/linguistics",
    "ref/skill/heal",
    "ref/skill/knowledge-engineering",
    "ref/skill/knowledge-geography",
]

FEATS = [
    "ref/rule/technological-feats",
    "ref/feat/craft-cybernetics",
    "ref/feat/craft-pharmaceutical",
    "ref/feat/craft-technological-arms-and-armor",
    "ref/feat/craft-technological-item",
    "ref/feat/robots-bane",
    "ref/feat/scavengers-luck",
    "ref/feat/technologist",
    "ref/feat/technology-adept",
    "ref/feat/technophobe",
    "ref/feat/wrest-charge",
]

SPELLS = [
    "ref/spell/antitech-field",
    "ref/spell/destroy-robot",
    "ref/spell/detect-radiation",
    "ref/spell/discharge",
    f"{CARRIED}/technology-in-the-world/spells/discharge/greater",
    "ref/spell/discharge-greater",
    "ref/spell/infuse-robot",
    "ref/spell/irradiate",
    "ref/spell/magic-circle-against-technology",
    f"{CARRIED}/technology-in-the-world/spells/make-whole/greater",
    "ref/spell/make-whole-greater",
    "ref/spell/memory-of-function",
    "ref/spell/protection-from-technology",
    "ref/spell/rebuke-technology",
    "ref/spell/recharge",
    "ref/spell/remove-radioactivity",
    f"{CARRIED}/technology-in-the-world/spells/remove-radioactivity/greater",
    "ref/spell/remove-radioactivity-greater",
    "ref/spell/technomancy",
]

ARCHETYPES = [
    "ref/arc-fighter/cyber-soldier",
    "ref/arc-cleric/iron-priest",
    "ref/arc-rogue/numerian-scavenger",
    "ref/arc-barbarian/savage-technologist",
    "ref/arc-gunslinger/techslinger",
]

CRAFTING = [
    "ref/rule/crafting-high-tech-items",
    "ref/rule/crafting-laboratories",
    "ref/rule/hybrid-items",
    "ref/rule/pricing-and-creating-new-technological-items",
]

# Chapter 2: Technological Equipment (pp. 18-53).
WEAPONS = [
    "ref/tech-weapon/arc-pistol",
    "ref/tech-weapon/arc-rifle",
    "ref/tech-weapon/atom-gun",
    "ref/tech-weapon/autograpnel",
    "ref/tech-weapon/chainsaw",
    "ref/tech-weapon/dart-gun",
    "ref/tech-weapon/death-ray",
    "ref/tech-weapon/emp-pistol",
    "ref/tech-weapon/emp-rifle",
    "ref/tech-weapon/flare-gun",
    "ref/tech-weapon/gravity-pistol",
    "ref/tech-weapon/gravity-rifle",
    "ref/tech-weapon/grenade-launcher",
    "ref/tech-weapon/id-rifle",
    "ref/tech-weapon/laser-pistol",
    "ref/tech-weapon/laser-rifle",
    "ref/tech-weapon/laser-torch",
    "ref/tech-weapon/mindrender",
    "ref/tech-weapon/monowhip",
    "ref/tech-weapon/nuclear-resonator",
    "ref/tech-weapon/null-blade",
    "ref/tech-weapon/plasmathrower",
    "ref/tech-weapon/rail-gun",
    "ref/tech-weapon/rocket-launcher",
    "ref/tech-weapon/sonic-pistol",
    "ref/tech-weapon/sonic-rifle",
    "ref/tech-weapon/stun-baton",
    "ref/tech-weapon/stun-gun",
    "ref/tech-weapon/vortex-gun",
    "ref/tech-weapon/x-laser",
    "ref/tech-weapon/zero-pistol",
    "ref/tech-weapon/zero-rifle",
]

ARMOR = [
    "ref/tech-armor/chameleon-suit",
    "ref/tech-armor/deep-diving-suit",
    "ref/tech-armor/gravity-suit",
    "ref/tech-armor/hard-light-shield",
    "ref/tech-armor/heavy-weapon-harness",
    "ref/tech-armor/hev-suit",
    "ref/tech-armor/inssuit",
    "ref/tech-armor/nanite-ablative-armor",
    "ref/tech-armor/panic-suit",
    f"{CARRIED}/equipment/armor/scatterlight-suit",
    *[f"ref/tech-armor/scatterlight-suit-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-armor/smart-armor-compact-form",
    "ref/tech-armor/smart-armor-expanded-form",
    "ref/tech-armor/spacesuit",
]

PHARMACEUTICALS = [
    "ref/tech-pharma/baseline",
    "ref/tech-pharma/cardioamp",
    "ref/tech-pharma/cureall",
    f"{CARRIED}/equipment/pharmaceuticals/hemochem",
    "ref/tech-pharma/hemochem-grade-i",
    "ref/tech-pharma/hemochem-grade-ii",
    "ref/tech-pharma/hemochem-grade-iii",
    "ref/tech-pharma/hemochem-grade-iv",
    "ref/tech-pharma/hemochem-grave-v",
    "ref/tech-pharma/hype",
    "ref/tech-pharma/torpinal",
    "ref/tech-pharma/vive",
    "ref/tech-pharma/zortaphen",
]

CYBERTECH = [
    "ref/tech-cyber/biofilter",
    "ref/tech-cyber/cranial-bomb",
    "ref/tech-cyber/cyberart",
    f"{CARRIED}/equipment/cybertech/cyberfiber-muscles",
    *[f"ref/tech-cyber/cyberfiber-muscles-mark-{mark}" for mark in ("i", "ii", "iii")],
    "ref/tech-cyber/cybernetic-arm",
    "ref/tech-cyber/cybernetic-ears",
    "ref/tech-cyber/cybernetic-eyes",
    "ref/tech-cyber/cybernetic-legs",
    f"{CARRIED}/equipment/cybertech/dermal-plating",
    *[f"ref/tech-cyber/dermal-plating-mark-{mark}" for mark in ("i", "ii", "iii", "iv", "v")],
    "ref/tech-cyber/frenzychip",
    "ref/tech-cyber/implanted-weaponry",
    "ref/tech-cyber/skillslot",
    f"{CARRIED}/equipment/cybertech/thoracic-nanite-chamber",
    *[f"ref/tech-cyber/thoracic-nanite-chamber-mark-{mark}" for mark in ("i", "ii", "iii")],
    f"{CARRIED}/equipment/cybertech/wirejack-tendons",
    *[f"ref/tech-cyber/wirejack-tendons-mark-{mark}" for mark in ("i", "ii", "iii")],
]

GEAR = [
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/access-card",
    *[f"ref/tech-misc/access-card-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/battery",
    "ref/tech-misc/camera",
    "ref/tech-misc/chemalyzer",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/chipfinder",
    *[f"ref/tech-misc/chipfinder-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/clonepod",
    "ref/tech-misc/commset",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/cortex-gun",
    *[f"ref/tech-misc/cortex-gun-mark-{mark}" for mark in ("i", "ii", "iii", "iv", "v")],
    "ref/tech-misc/cylex",
    "ref/tech-misc/detonator",
    "ref/tech-misc/e-pick",
    *[f"ref/tech-misc/e-pick-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    # The On Signals sidebar prints on p. 43.
    "ref/misc/signal",
    "ref/tech-misc/emergency-beacon",
    "ref/tech-misc/emergency-raft",
    "ref/tech-misc/emergency-shelter",
    "ref/tech-misc/filter-mask",
    "ref/tech-misc/fire-extinguisher",
    "ref/tech-misc/flashlight",
    "ref/tech-misc/force-field",
    *[f"ref/tech-misc/force-field-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/goo-tube",
    "ref/tech-misc/gravity-clip",
    "ref/tech-weapon/grenade",
    *[f"ref/tech-weapon/{kind}-grenade" for kind in (
        "arc", "atom", "bang", "bio", "concussion", "emp", "flash", "flechette",
        "fragmentation", "gas", "gravity", "inferno", "plasma", "soft", "sonic", "zero",
    )],
    "ref/tech-misc/grippers",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/hologram-generator",
    *[f"ref/tech-misc/hologram-generator-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/inertial-dampening-belt",
    "ref/tech-misc/ion-tape",
    "ref/tech-misc/jetpack",
    "ref/tech-misc/laser-sight",
    "ref/tech-misc/lighter",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/lock-coder",
    *[f"ref/tech-misc/lock-coder-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/magboots",
    "ref/tech-misc/medlance",
    "ref/tech-misc/motion-tracker",
    "ref/tech-misc/nanite-canister",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/nanite-hypogun",
    *[f"ref/tech-misc/nanite-hypogun-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/neurocam",
    "ref/tech-misc/power-cable",
    "ref/tech-misc/power-receiver",
    "ref/tech-misc/quantum-box",
    "ref/tech-misc/radiation-detector",
    "ref/tech-misc/robojack",
    "ref/tech-misc/signal-booster",
    "ref/tech-misc/signal-jammer",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/skillchip",
    *[f"ref/tech-misc/skillchip-mark-{mark}" for mark in ("i", "ii", "iii", "iv", "v")],
    "ref/tech-misc/tracker-chip",
    "ref/tech-misc/trauma-pack",
    "ref/tech-misc/trauma-pack-plus",
    f"{CARRIED}/equipment/gear/technological-gear/on-signals/veemod",
    *[f"ref/tech-misc/veemod-{color}" for color in (
        "black", "blue", "brown", "gray", "green", "orange", "prismatic", "red", "white",
    )],
    "ref/tech-misc/veemod-goggles",
    "ref/tech-misc/zipstick",
]

# Chapter 3: Technological Hazards and Artifacts (pp. 54-64).
GLITCHES = [
    "ref/rule/armor-glitches",
    "ref/rule/general-glitches",
    "ref/rule/pharmaceutical-glitches",
    "ref/rule/weapon-glitches",
]

ARTIFACTS = [
    "ref/tech-artifact/autodoc",
    "ref/tech-artifact/extinction-wave-device",
    "ref/tech-artifact/powered-armor",
    "ref/tech-artifact/psychic-imprinter",
    "ref/tech-artifact/rebirthing-chamber",
    "ref/tech-artifact/temporal-accelerator",
]

GENERATORS = [
    "ref/tech-artifact/fission-reactor",
    "ref/tech-artifact/fusion-reactor",
    "ref/tech-artifact/geothermal-generator",
    "ref/tech-artifact/graviton-reactor",
    "ref/tech-artifact/solar-generator",
]

CHAPTERS = [
    entry(
        f"{CARRIED}/technology-in-the-world",
        [
            entry(f"{CARRIED}/technology-in-the-world/skills", [entry(i) for i in SKILLS]),
            entry(f"{CARRIED}/technology-in-the-world/feats", [entry(i) for i in FEATS]),
            entry(f"{CARRIED}/technology-in-the-world/spells", [entry(i) for i in SPELLS]),
            entry(
                f"{CARRIED}/technology-in-the-world/archtypes",
                [
                    heading(
                        "Technology-Themed Archetypes",
                        "technology-themed-archetypes",
                        [entry(i) for i in ARCHETYPES],
                    )
                ],
            ),
            entry("ref/pclass/technomancer"),
            entry(f"{CARRIED}/technology-in-the-world/crafting", [entry(i) for i in CRAFTING]),
        ],
    ),
    entry(
        f"{CARRIED}/equipment",
        [
            entry(f"{CARRIED}/equipment/introduction"),
            entry("ref/rule/technological-equipment"),
            entry(
                f"{CARRIED}/equipment/weapons",
                [
                    entry("ref/rule/tech-weapons"),
                    entry("ref/misc/automatic"),
                    entry("ref/misc/semi-automatic"),
                    entry("ref/misc/slow-firing"),
                    entry("ref/misc/touch"),
                    *[entry(i) for i in WEAPONS],
                ],
            ),
            entry(
                f"{CARRIED}/equipment/armor",
                [entry("ref/rule/tech-armor"), *[entry(i) for i in ARMOR]],
            ),
            entry(
                "ref/rule/pharmaceuticals",
                [entry(i) for i in PHARMACEUTICALS],
            ),
            entry(
                "ref/rule/cybertech",
                [entry(i) for i in CYBERTECH],
            ),
            entry(
                f"{CARRIED}/equipment/gear",
                [
                    heading(
                        "Technological Gear",
                        "technological-gear",
                        [entry("ref/rule/tech-misc"), *[entry(i) for i in GEAR]],
                    )
                ],
            ),
        ],
    ),
    entry(
        f"{CARRIED}/hazards-and-artifacts",
        [
            entry(
                f"{CARRIED}/hazards-and-artifacts/hazards",
                [
                    entry(
                        "ref/rule/timeworn-technology",
                        [heading("Timeworn Technology Glitches", "timeworn-technology-glitches", [entry(i) for i in GLITCHES])],
                    ),
                    entry("ref/rule/radiation"),
                    entry(
                        "ref/rule/traps-and-technology",
                        [
                            entry(f"{CARRIED}/hazards-and-artifacts/hazards/timeworn-technology-glitches/traps-and-technology/proximity-mine"),
                            entry(f"{CARRIED}/hazards-and-artifacts/hazards/timeworn-technology-glitches/traps-and-technology/electrified-door"),
                        ],
                    ),
                    entry("ref/eq-material/skymetal"),
                ],
            ),
            entry(
                f"{CARRIED}/hazards-and-artifacts/artifical-intelligence",
                [
                    entry("ref/rule/artificial-intelligence"),
                    heading(
                        "Artificial Intelligences",
                        "artificial-intelligences",
                        [
                            entry("ref/rule/creating-an-ai"),
                            heading(
                                "Sample AIs",
                                "sample-ais",
                                [
                                    entry(f"{CARRIED}/hazards-and-artifacts/artifical-intelligence/artificial-intelligences/sample-ais/analyst-ai"),
                                    entry(f"{CARRIED}/hazards-and-artifacts/artifical-intelligence/artificial-intelligences/sample-ais/security-ai"),
                                ],
                            ),
                            heading(
                                "Aggregate Template",
                                "aggregate-template",
                                [
                                    entry("ref/template/aggregate"),
                                    heading(
                                        "Example Aggregate",
                                        "example-aggregate",
                                        [
                                            entry(f"{CARRIED}/hazards-and-artifacts/artifical-intelligence/artificial-intelligences/example-aggregate/security-mannequin")
                                        ],
                                    ),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
            entry(
                f"{CARRIED}/hazards-and-artifacts/artifacts",
                [
                    entry("ref/rule/tech-artifacts"),
                    heading("Technological Artifacts", "technological-artifacts", [entry(i) for i in ARTIFACTS]),
                    heading("Power Generators", "power-generators", [entry(i) for i in GENERATORS]),
                ],
            ),
        ],
    ),
]

BASIS = (
    "The main chapter titles, their order, and their first pages are checked "
    "against the printed contents page of Technology Guide. Each chapter holds "
    "its printed section headings and entries in page order. See "
    "docs/research/book-order-evidence.md. Canonical ids were substituted "
    "through the reviewed alias map; chapters and order are unchanged."
)

REVIEW = [
    "Each chapter holds its printed sections and page order; entries that share a printed page keep the dataset order.",
    "The printed Introduction has no corpus chapter. The printed Technological Equipment and Technological Hazards and Artifacts titles replace the dataset's Equipment and Hazards and Artifacts.",
    "The dataset group headings On Signals and Traps and Technology are gone because an entry now carries each printed heading; no entry id or entry file changed.",
    "A canonical duplicate of a dataset section intro opens the printed section it repeats: Technological Adventures, Technological Feats, Feats and Crafting High-Tech Items, Artificial Intelligence, Tech Artifacts, and Miscellaneous Gear.",
    "The final New entries (page order) chapter keeps only entries with no Technology Guide page: the base Make Whole spell, which Technology Guide refers to but does not print, and the cross-book Specific Campaign Types hub.",
    "Canonical ids were substituted through sources/canonical/id-map.toml. A legacy entry with no installed target is omitted and listed in coverage.",
]


# ---------------------------------------------------------------------------
# Tree building


def index_nodes(book: dict) -> tuple[dict[str, dict], dict[tuple[str, str | None], dict]]:
    by_entry: dict[str, dict] = {}
    by_heading: dict[tuple[str, str | None], dict] = {}
    for chapter in book.get("chapters", []):
        for node in flatten(chapter):
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


def build(spec: tuple, by_entry: dict[str, dict], by_heading: dict, used: set[str]) -> dict:
    kind = spec[0]
    if kind == E:
        entry_id, children = spec[1], spec[2]
        node = by_entry.get(entry_id)
        if node is None:
            raise SystemExit(f"missing entry node {entry_id}")
        if entry_id in used:
            raise SystemExit(f"entry {entry_id} placed twice")
        used.add(entry_id)
    else:
        title, anchor, children = spec[1], spec[2], spec[3]
        node = by_heading.get((title, anchor))
        if node is None:
            if anchor != "aggregate-template":
                raise SystemExit(f"missing heading node {title!r}")
            node = {"title": title, "anchor": anchor}
        used.add(f"heading:{title}:{anchor or ''}")
    if children:
        node["children"] = [build(child, by_entry, by_heading, used) for child in children]
    else:
        node.pop("children", None)
    return node


def build_tree(book: dict) -> dict:
    by_entry, by_heading = index_nodes(book)
    chapter_nodes = {id(node) for node in book["chapters"]}
    existing_headings = {
        key for key, node in by_heading.items() if id(node) not in chapter_nodes
    }
    spec_headings = set(_spec_headings(CHAPTERS))
    unexpected = existing_headings - spec_headings - DROPPED_HEADINGS
    if unexpected:
        raise SystemExit(f"unexpected heading nodes: {sorted(unexpected)}")

    used: set[str] = set()
    chapters = [build(spec, by_entry, by_heading, used) for spec in CHAPTERS]

    # The residual chapter keeps only the entries with no printed page.
    residual = next(
        (node for node in book["chapters"] if node["title"] == "New entries (page order)"),
        None,
    )
    if residual is None:
        raise SystemExit("missing New entries (page order) chapter")

    before = set(by_entry)
    leftover = [entry_id for entry_id in before if entry_id not in used]
    if sorted(leftover) != sorted(RESIDUAL_ENTRIES):
        raise SystemExit(
            "residual chapter mismatch: "
            + ", ".join(sorted(leftover))
            + " (expected "
            + ", ".join(sorted(RESIDUAL_ENTRIES))
            + ")"
        )
    residual["children"] = [by_entry[entry_id] for entry_id in RESIDUAL_ENTRIES]
    chapters.append(residual)

    book["chapters"] = chapters
    book["basis"] = BASIS
    book["review"] = REVIEW
    return book


def _spec_headings(specs: list[tuple]) -> list[tuple[str, str | None]]:
    found: list[tuple[str, str | None]] = []
    for spec in specs:
        if spec[0] == H:
            found.append((spec[1], spec[2]))
            found.extend(_spec_headings(spec[3]))
        else:
            found.extend(_spec_headings(spec[2]))
    return found


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    before_paths = placed_entries(book)
    build_tree(book)
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
    text = dump_books(books)

    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the Technology Guide nested-placement shape", file=sys.stderr)
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
