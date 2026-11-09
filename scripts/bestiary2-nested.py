#!/usr/bin/env python3
"""Place the Bestiary 2 tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/bestiary2-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md. The tree
helpers are shared with ``scripts/bestiary-nested.py``.
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
import tomllib
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location(
    "bestiary_nested", _HERE / "bestiary-nested.py"
)
assert _SPEC is not None and _SPEC.loader is not None
_COMMON = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_COMMON)

attach = _COMMON.attach
detach = _COMMON.detach
dump_books = _COMMON.dump_books
extract = _COMMON.extract
find_node = _COMMON.find_node
move = _COMMON.move
parent_of = _COMMON.parent_of
section = _COMMON.section
sort_children = _COMMON.sort_children

BOOK = "bestiary-2"
BOOKS_FILE = "books.toml"

# ---------------------------------------------------------------------------
# Page evidence

BESTIARY2_PAGE = re.compile(r"Bestiary 2(?: pg\.?| p\.) ?(\d+)")
BESTIARY2_SLASH_PAGE = re.compile(r"Bestiary 2/(\d+)")

# Heading pages that print without a page on the source line, split stat
# blocks that carry no source line, and source lines that name another page.
PAGE_OVERRIDES: dict[str, int] = {
    # Printed family headings.
    "ref/family/aeon": 8,
    "ref/family/agathion": 15,
    "ref/family/daemon": 62,
    "ref/family/gremlin": 141,
    "ref/family/inevitable": 161,
    "ref/family/nightshade": 199,
    "ref/family/protean": 213,
    "ref/family/qlippoth": 218,
    "ref/family/dragon-primal": 92,
    "ref/family/dragon-primal-brine": 94,
    "ref/family/dragon-primal-cloud": 96,
    "ref/family/dragon-primal-crystal": 98,
    "ref/family/dragon-primal-magma": 100,
    "ref/family/dragon-primal-umbral": 102,
    # Appendix 3 creature subtype heading page.
    "ref/subtype/grippli": 307,
    # Source lines that name another page. Akata prints on 23, not 22; the
    # giant amoeba prints on 24, not 23; the giant mosquito source has a
    # duplicated digit.
    "ref/monster/akata": 23,
    "ref/monster/amoeba-giant-amoeba": 24,
    "ref/monster/mosquito-giant-mosquito": 193,
    # Split stat blocks with no source line.
    "ref/carried/bestiary-2/monsters/dragon/primal/brine/dragon/primal/brine": 94,
    "ref/carried/bestiary-2/monsters/dragon/primal/cloud/dragon/primal/cloud": 96,
    "ref/carried/bestiary-2/monsters/dragon/primal/crystal/dragon/primal/crystal": 98,
    "ref/carried/bestiary-2/monsters/dragon/primal/magma/dragon/primal/magma": 100,
    "ref/carried/bestiary-2/monsters/dragon/primal/umbral/umbral-dragon": 102,
    "ref/carried/bestiary-2/monsters/lycanthrope/werebear/werebear-human-form": 181,
    "ref/carried/bestiary-2/monsters/lycanthrope/werebear/werebear-hybrid-form": 181,
    "ref/carried/bestiary-2/monsters/lycanthrope/wereboar/wereboar-human-form": 182,
    "ref/carried/bestiary-2/monsters/lycanthrope/wereboar/wereboar-hybrid-form": 182,
    "ref/carried/bestiary-2/monsters/lycanthrope/weretiger/weretiger-human-form": 183,
    "ref/carried/bestiary-2/monsters/lycanthrope/weretiger/weretiger-hybrid-form": 183,
    "ref/carried/bestiary-2/monsters/primate/baboon": 212,
    "ref/carried/bestiary-2/monsters/swarm/primate/monkey": 212,
    "ref/carried/bestiary-2/monsters/ray/stingray": 232,
    "ref/carried/bestiary-2/monsters/megafauna/gylptodon": 186,
    "ref/carried/bestiary-2/monsters/spriggan/spriggan": 257,
    "ref/carried/bestiary-2/monsters/ogrekin": 204,
    "ref/carried/bestiary-2/monsters/petitioner": 208,
    "ref/carried/bestiary-2/monsters/ravener": 230,
    "ref/carried/bestiary-2/monsters/worm-that-walks": 286,
    "ref/carried/bestiary-2/monsters/zombie/juju": 291,
}


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    text = path.read_text(encoding="utf-8")
    found = BESTIARY2_PAGE.search(text) or BESTIARY2_SLASH_PAGE.search(text)
    return int(found.group(1)) if found else None


def node_page(corpus: Path, node: dict) -> int | None:
    page = page_of(corpus, node.get("entry"))
    if page is not None:
        return page
    pages = [page_of(corpus, child.get("entry")) for child in node.get("children", [])]
    known = [value for value in pages if value is not None]
    return min(known) if known else None


def node_order(node: dict, order: dict[str, int]) -> int:
    values = [order.get(child.get("entry", ""), 10**9) for child in node.get("children", [])]
    own = order.get(node.get("entry") or "", 10**9)
    return min([own] + values)


def page_key(corpus: Path, order: dict[str, int], node: dict):
    page = node_page(corpus, node)
    return (page if page is not None else 10**9, node_order(node, order))


# ---------------------------------------------------------------------------
# The transform


# Printed family headings.  The family entry becomes the heading; the listed
# monsters sit under it in printed page order.
FAMILY_GROUPS: dict[str, list[str]] = {
    "ref/family/aeon": [
        "ref/monster/aeon-akhana",
        "ref/monster/aeon-bythos",
        "ref/monster/aeon-paracletus",
        "ref/monster/aeon-pleroma",
        "ref/monster/aeon-theletos",
    ],
    "ref/family/agathion": [
        "ref/monster/agathion-avoral",
        "ref/monster/agathion-cetaceal",
        "ref/monster/agathion-draconal",
        "ref/monster/agathion-leonal",
        "ref/monster/agathion-silvanshee",
        "ref/monster/agathion-vulpinal",
    ],
    "ref/family/daemon": [
        "ref/monster/daemon-astradaemon",
        "ref/monster/daemon-cacodaemon",
        "ref/monster/daemon-ceustodaemon",
        "ref/monster/daemon-derghodaemon",
        "ref/monster/daemon-hydrodaemon",
        "ref/monster/daemon-leukodaemon",
        "ref/monster/daemon-meladaemon",
        "ref/monster/daemon-olethrodaemon",
        "ref/monster/daemon-piscodaemon",
        "ref/monster/daemon-purrodaemon",
        "ref/monster/daemon-thanadaemon",
    ],
    "ref/family/gremlin": [
        "ref/monster/gremlin-jinkin",
        "ref/monster/gremlin-nuglub",
        "ref/monster/gremlin-pugwampi",
        "ref/monster/gremlin-vexgit",
    ],
    "ref/family/inevitable": [
        "ref/monster/inevitable-arbiter",
        "ref/monster/inevitable-kolyarut",
        "ref/monster/inevitable-lhaksharut",
        "ref/monster/inevitable-marut",
        "ref/monster/inevitable-zelekhut",
    ],
    "ref/family/nightshade": [
        "ref/monster/nightshade-nightcrawler",
        "ref/monster/nightshade-nightwalker",
        "ref/monster/nightshade-nightwave",
        "ref/monster/nightshade-nightwing",
    ],
    "ref/family/protean": [
        "ref/monster/protean-imentesh",
        "ref/monster/protean-keketar",
        "ref/monster/protean-naunet",
        "ref/monster/protean-voidworm",
    ],
    "ref/family/qlippoth": [
        "ref/monster/qlippoth-augnagar",
        "ref/monster/qlippoth-chernobue",
        "ref/monster/qlippoth-cythnigot",
        "ref/monster/qlippoth-iathavos",
        "ref/monster/qlippoth-nyogoth",
        "ref/monster/qlippoth-shoggti",
        "ref/monster/qlippoth-thulgant",
    ],
}

# Primal dragons: the family heading holds one heading per colour, and each
# colour holds its base stat block and the printed age variants.
DRAGON_GROUP = "ref/family/dragon-primal"
DRAGON_COLOURS = [
    (
        "ref/family/dragon-primal-brine",
        "brine",
        "ref/carried/bestiary-2/monsters/dragon/primal/brine/dragon/primal/brine",
    ),
    (
        "ref/family/dragon-primal-cloud",
        "cloud",
        "ref/carried/bestiary-2/monsters/dragon/primal/cloud/dragon/primal/cloud",
    ),
    (
        "ref/family/dragon-primal-crystal",
        "crystal",
        "ref/carried/bestiary-2/monsters/dragon/primal/crystal/dragon/primal/crystal",
    ),
    (
        "ref/family/dragon-primal-magma",
        "magma",
        "ref/carried/bestiary-2/monsters/dragon/primal/magma/dragon/primal/magma",
    ),
    (
        "ref/family/dragon-primal-umbral",
        "umbral",
        "ref/carried/bestiary-2/monsters/dragon/primal/umbral/umbral-dragon",
    ),
]

# Each elemental entry holds the size variants the dataset splits out.
ELEMENTALS = [
    ("ref/monster/ice-elemental", "ref/carried/bestiary-2/monsters/elemental/ice"),
    (
        "ref/monster/lightning-elemental",
        "ref/carried/bestiary-2/monsters/elemental/lightning",
    ),
    ("ref/monster/magma-elemental", "ref/carried/bestiary-2/monsters/elemental/magma"),
    ("ref/monster/mud-elemental", "ref/carried/bestiary-2/monsters/elemental/mud"),
]
ELEMENTAL_SIZES = ["small", "medium", "large", "huge", "greater", "elder"]

# Each golem prints its construction rules inside the same entry.
GOLEMS = [
    (
        "ref/monster/golem-adamantine-golem",
        "ref/carried/bestiary-2/monsters/golem/adamantine/construction/adamantine-golem",
    ),
    (
        "ref/monster/golem-alchemical-golem",
        "ref/carried/bestiary-2/monsters/golem/alchemical/construction/alchemical-golem",
    ),
    (
        "ref/monster/golem-carrion-golem",
        "ref/carried/bestiary-2/monsters/golem/carrion/construction/carrion-golem",
    ),
    (
        "ref/monster/golem-clockwork-golem",
        "ref/carried/bestiary-2/monsters/golem/clockwork/construction/clockwork-golem",
    ),
    (
        "ref/monster/golem-glass-golem",
        "ref/carried/bestiary-2/monsters/golem/glass/construction/glass-golem",
    ),
    (
        "ref/monster/golem-mithral-golem",
        "ref/carried/bestiary-2/monsters/golem/mithral/construction/mithral-golem",
    ),
]

# Templates that print with one example stat block in one entry.  The
# creature node is the printed heading; the template rules sit under it.
TEMPLATE_EXAMPLES: dict[str, str] = {
    "ref/template/ogrekin": "ref/carried/bestiary-2/monsters/ogrekin",
    "ref/template/petitioner": "ref/carried/bestiary-2/monsters/petitioner",
    "ref/template/ravener": "ref/carried/bestiary-2/monsters/ravener",
    "ref/template/worm-that-walks": "ref/carried/bestiary-2/monsters/worm-that-walks",
    "ref/template/juju-zombie": "ref/carried/bestiary-2/monsters/zombie/juju",
}

# Dataset splits and duplicate stat blocks that print inside one entry.
SPLITS: dict[str, list[str]] = {
    "ref/carried/bestiary-2/monsters/primate/baboon": [
        "ref/monster/baboon",
        "ref/companion/baboon",
    ],
    "ref/carried/bestiary-2/monsters/ray/stingray": [
        "ref/monster/ray-stingray",
        "ref/companion/stingray",
    ],
    "ref/monster/ray-manta-ray": ["ref/companion/manta-ray"],
    "ref/monster/megafauna-glyptodon": [
        "ref/carried/bestiary-2/monsters/megafauna/gylptodon",
        "ref/companion/glyptodon",
    ],
    "ref/carried/bestiary-2/monsters/spriggan/spriggan": [
        "ref/monster/spriggan-spriggan-normal-size",
        "ref/monster/spriggan-spriggan-large-size",
    ],
    "ref/monster/giant-dragonfly": [
        "ref/monster/giant-dragonfly-giant-dragonfly-nymph"
    ],
    "ref/monster/dinosaur-allosaurus": ["ref/companion/allosaurus"],
    "ref/monster/dinosaur-parasaurolophus": ["ref/companion/parasaurolophus"],
    "ref/monster/dinosaur-tylosaurus": ["ref/companion/tylosaurus"],
    "ref/monster/gar": ["ref/companion/gar"],
    "ref/monster/herd-animal-ram": ["ref/companion/ram"],
    "ref/monster/hippopotamus": ["ref/companion/hippopotamus"],
    "ref/monster/megafauna-arsinoitherium": ["ref/companion/arsinoitherium"],
    "ref/monster/megafauna-megaloceros": ["ref/companion/megaloceros"],
    "ref/monster/megafauna-megatherium": ["ref/companion/megatherium"],
    "ref/monster/turtle-giant-snapping-turtle": [
        "ref/companion/turtle-giant-snapping"
    ],
}

# Printed headings that the dataset splits into stat blocks with no heading
# entry of their own.  The heading is synthesized, like the Bestiary golem
# Construction heading.
SYNTHESIZED_GROUPS: list[tuple[str, list[str]]] = [
    (
        "Merrow",
        [
            "ref/monster/merrow-freshwater-merrow",
            "ref/monster/merrow-saltwater-merrow",
        ],
    ),
    (
        "Lycanthrope, Werebear",
        [
            "ref/carried/bestiary-2/monsters/lycanthrope/werebear/werebear-human-form",
            "ref/carried/bestiary-2/monsters/lycanthrope/werebear/werebear-hybrid-form",
        ],
    ),
    (
        "Lycanthrope, Wereboar",
        [
            "ref/carried/bestiary-2/monsters/lycanthrope/wereboar/wereboar-human-form",
            "ref/carried/bestiary-2/monsters/lycanthrope/wereboar/wereboar-hybrid-form",
        ],
    ),
    (
        "Lycanthrope, Weretiger",
        [
            "ref/carried/bestiary-2/monsters/lycanthrope/weretiger/weretiger-human-form",
            "ref/carried/bestiary-2/monsters/lycanthrope/weretiger/weretiger-hybrid-form",
        ],
    ),
]

# The printed Appendix 3 Glossary universal monster rules and creature
# subtypes. Only the entries with no other route into the tree move here; the
# remaining subtype records stay residual because a placed monster links to
# them.
UNIVERSAL_MONSTER_RULES = [
    "ref/umr/lycanthropic-empathy",
]
MONSTER_SUBTYPES = [
    "ref/subtype/grippli",
]

# The residual chapter holds only entries whose printed home is an appendix
# that this book tree does not carry, plus cross-book hubs.
RESIDUAL_ENTRIES = {
    "ref/template/entropic",
    "ref/template/resolute",
    "ref/rule/step-2-target-statistics",
    "ref/subtype/nightshade",
    "ref/subtype/aeon",
    "ref/subtype/agathion",
    "ref/subtype/air",
    "ref/subtype/angel",
    "ref/subtype/aquatic",
    "ref/subtype/archon",
    "ref/subtype/augmented",
    "ref/subtype/azata",
    "ref/subtype/chaotic",
    "ref/subtype/cold",
    "ref/subtype/daemon",
    "ref/subtype/dark-folk",
    "ref/subtype/demon",
    "ref/subtype/devil",
    "ref/subtype/earth",
    "ref/subtype/elemental",
    "ref/subtype/evil",
    "ref/subtype/extraplanar",
    "ref/subtype/fire",
    "ref/subtype/giant",
    "ref/subtype/gnome",
    "ref/subtype/good",
    "ref/subtype/incorporeal",
    "ref/subtype/inevitable",
    "ref/subtype/lawful",
    "ref/subtype/native",
    "ref/subtype/protean",
    "ref/subtype/qlippoth",
    "ref/subtype/shapechanger",
    "ref/subtype/swarm",
    "ref/subtype/water",
    "ref/family/elemental",
    "ref/carried/bestiary/rules/universal-monster-rules/all-around-vision",
    "ref/carried/bestiary/rules/universal-monster-rules/amorphous",
    "ref/carried/bestiary/rules/universal-monster-rules/attach",
    "ref/carried/bestiary/rules/universal-monster-rules/blood-drain",
    "ref/carried/bestiary/rules/universal-monster-rules/blood-rage",
    "ref/carried/bestiary/rules/universal-monster-rules/compression",
    "ref/carried/bestiary/rules/universal-monster-rules/curse-of-lycanthropy",
    "ref/carried/bestiary/rules/universal-monster-rules/earth-glide",
    "ref/carried/bestiary/rules/universal-monster-rules/engulf",
    "ref/carried/bestiary/rules/universal-monster-rules/entrap",
    "ref/carried/bestiary/rules/universal-monster-rules/fast-swallow",
    "ref/carried/bestiary/rules/universal-monster-rules/freeze",
    "ref/carried/bestiary/rules/universal-monster-rules/heat",
    "ref/carried/bestiary/rules/universal-monster-rules/hold-breath",
    "ref/carried/bestiary/rules/universal-monster-rules/jet",
    "ref/carried/bestiary/rules/universal-monster-rules/keen-scent",
    "ref/carried/bestiary/rules/universal-monster-rules/lifesense",
    "ref/carried/bestiary/rules/universal-monster-rules/multiweapon-mastery",
    "ref/carried/bestiary/rules/universal-monster-rules/natural-invisibility",
    "ref/carried/bestiary/rules/universal-monster-rules/negative-energy-affinity",
    "ref/carried/bestiary/rules/universal-monster-rules/no-breath",
    "ref/carried/bestiary/rules/universal-monster-rules/see-in-darkness",
    "ref/carried/bestiary/rules/universal-monster-rules/sound-mimicry",
    "ref/carried/bestiary/rules/universal-monster-rules/split",
    "ref/carried/bestiary/rules/universal-monster-rules/strangle",
    "ref/carried/bestiary/rules/universal-monster-rules/sunlight-powerlessness",
    "ref/carried/bestiary/rules/universal-monster-rules/undersized-weapons",
    "ref/carried/bestiary/rules/universal-monster-rules/unnatural-aura",
}


def find_book(books: dict) -> dict:
    for book in books["books"]:
        if book["source"] == BOOK:
            return book
    raise SystemExit(f"unknown book source: {BOOK}")


def ensure_group(monsters: dict, title: str) -> dict:
    for child in monsters.get("children", []):
        if child.get("title") == title and not child.get("entry"):
            return child
    node = {"title": title, "children": []}
    attach(monsters, node)
    return node


def ensure_top_chapter(book: dict, title: str) -> dict:
    for node in book["chapters"]:
        if node["title"] == title:
            return node
    residual = next(
        index
        for index, node in enumerate(book["chapters"])
        if node["title"] == "New entries (page order)"
    )
    node = {"title": title, "children": []}
    book["chapters"].insert(residual, node)
    return node


def ensure_construction(node: dict) -> dict:
    for child in node.get("children", []):
        if child.get("title") == "Construction" and not child.get("entry"):
            return child
    construction = {"title": "Construction", "children": []}
    attach(node, construction)
    return construction


def find_anchor(book: dict, entry_id: str, title: str, anchor: str) -> dict:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")

    def walk(current: dict) -> dict | None:
        if current.get("title") == title and current.get("anchor") == anchor:
            return current
        for child in current.get("children", []):
            found = walk(child)
            if found is not None:
                return found
        return None

    found = walk(node)
    if found is None:
        raise SystemExit(f"missing anchor {title} under {entry_id}")
    return found


def walk_entries(node: dict, trail: list[str], out: dict[str, str]) -> None:
    here = trail + [node["title"]]
    if node.get("entry"):
        out[node["entry"]] = " > ".join(here)
    for child in node.get("children", []):
        walk_entries(child, here, out)


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None, str]]:
    before: dict[str, str] = {}
    order: dict[str, int] = {}
    for top in book["chapters"]:
        walk_entries(top, [], before)

    def walk_order(node: dict) -> None:
        if node.get("entry"):
            order.setdefault(node["entry"], len(order))
        for child in node.get("children", []):
            walk_order(child)

    for top in book["chapters"]:
        walk_order(top)

    monsters = _COMMON.chapter(book, "Monsters A to Z")
    before_seq = [
        child["entry"] for child in monsters.get("children", []) if child.get("entry")
    ]

    # 1. Template rules sit under the creature that heads the printed entry.
    for template_id, creature_id in TEMPLATE_EXAMPLES.items():
        creature = find_node(book, creature_id)
        if creature is None:
            raise SystemExit(f"missing template example {creature_id}")
        move(book, template_id, creature)

    # 2. Dataset splits and duplicate stat blocks sit inside their entry.
    for base_id, child_ids in SPLITS.items():
        base = find_node(book, base_id)
        if base is None:
            raise SystemExit(f"missing split base {base_id}")
        for child_id in child_ids:
            move(book, child_id, base)
        # A split whose base is a residual duplicate moves the whole entry
        # into Monsters A to Z.
        move(book, base_id, monsters)

    # 3. Golem construction rules sit under a Construction heading.
    for golem_id, construction_id in GOLEMS:
        golem = find_node(book, golem_id)
        if golem is None:
            raise SystemExit(f"missing golem {golem_id}")
        move(book, construction_id, ensure_construction(golem))

    # 4. Elemental size variants sit under the base elemental.
    for base_id, prefix in ELEMENTALS:
        base = find_node(book, base_id)
        if base is None:
            raise SystemExit(f"missing elemental {base_id}")
        for size in ELEMENTAL_SIZES:
            move(book, f"{prefix}/{size}", base)

    # 5. Primal dragons: family -> colour -> base and age variants.
    dragon = detach(book, DRAGON_GROUP)
    for colour_id, colour, base_id in DRAGON_COLOURS:
        colour_node = extract(book, dragon, colour_id)
        attach(colour_node, extract(book, colour_node, base_id))
        for age in ("young", "adult", "ancient"):
            attach(
                colour_node,
                extract(
                    book,
                    colour_node,
                    f"ref/monster/dragon-primal-{colour}-{age}-{colour}-dragon",
                ),
            )
        attach(dragon, colour_node)
    sort_children(dragon, lambda child: page_key(corpus, order, child))
    attach(monsters, dragon)

    # 6. Printed family headings.
    for family_id, member_ids in FAMILY_GROUPS.items():
        family = detach(book, family_id)
        for member_id in member_ids:
            attach(family, extract(book, family, member_id))
        sort_children(family, lambda child: page_key(corpus, order, child))
        attach(monsters, family)

    # 7. Printed headings that the dataset splits with no heading entry.
    for title, member_ids in SYNTHESIZED_GROUPS:
        group = ensure_group(monsters, title)
        for member_id in member_ids:
            move(book, member_id, group)

    # 8. Animate Dream's printed curse split.
    curse = find_anchor(book, "ref/monster/animate-dream", "Nightmare Curse", "nightmare-curse")
    move(book, "ref/affliction/animate-dream/nightmare-curse", curse)

    # 9. Monsters A to Z in printed page order.  A stable sort keeps the
    # dataset order of nodes that share a printed page.
    sort_children(monsters, lambda child: page_key(corpus, order, child))

    # 10. Printed Appendix 3 Glossary entries with no other route.
    universal = ensure_top_chapter(book, "Universal Monster Rules")
    for entry_id in UNIVERSAL_MONSTER_RULES:
        move(book, entry_id, universal)
    subtypes = ensure_top_chapter(book, "Monster Subtypes")
    for entry_id in MONSTER_SUBTYPES:
        move(book, entry_id, subtypes)

    after: dict[str, str] = {}
    for top in book["chapters"]:
        walk_entries(top, [], after)

    if set(before) != set(after):
        print(f"BEFORE COUNT {len(before)} AFTER COUNT {len(after)}", file=sys.stderr)
        lost = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        print(f"LOST {lost}", file=sys.stderr)
        print(f"ADDED {added}", file=sys.stderr)
        raise SystemExit("entry set changed")

    residual = _COMMON.chapter(book, "New entries (page order)")
    residual_ids: set[str] = set()

    def walk_ids(node: dict) -> None:
        if node.get("entry"):
            residual_ids.add(node["entry"])
        for child in node.get("children", []):
            walk_ids(child)

    walk_ids(residual)
    if residual_ids != RESIDUAL_ENTRIES:
        unexpected = sorted(residual_ids - RESIDUAL_ENTRIES)
        missing = sorted(RESIDUAL_ENTRIES - residual_ids)
        print(f"UNEXPECTED RESIDUAL {unexpected}", file=sys.stderr)
        print(f"MISSING RESIDUAL {missing}", file=sys.stderr)
        raise SystemExit("residual chapter changed")

    after_seq = [
        child["entry"] for child in monsters.get("children", []) if child.get("entry")
    ]

    common = set(before_seq) & set(after_seq)
    before_rank = {
        entry_id: index for index, entry_id in enumerate(before_seq) if entry_id in common
    }
    after_rank = {
        entry_id: index for index, entry_id in enumerate(after_seq) if entry_id in common
    }
    reordered: set[str] = set()
    kept = [entry_id for entry_id in before_seq if entry_id in common]
    for i, left in enumerate(kept):
        for right in kept[i + 1 :]:
            if (before_rank[left] < before_rank[right]) != (
                after_rank[left] < after_rank[right]
            ):
                reordered.add(left)
                reordered.add(right)

    moves = []
    for entry_id in sorted(before):
        page = page_of(corpus, entry_id)
        title = after[entry_id].split(" > ")[-1]
        if before[entry_id] != after[entry_id]:
            moves.append((title, before[entry_id], after[entry_id], page, "move"))
        elif entry_id in reordered:
            moves.append((title, before[entry_id], after[entry_id], page, "order"))
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

    for title, old, new, page, kind in moves:
        print(f"{kind}\t{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print(
                "books.toml is not in the Bestiary 2 nested-placement shape",
                file=sys.stderr,
            )
            return 1
        print(f"books.toml is canonical for {len(moves)} Bestiary 2 moves", file=sys.stderr)
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} Bestiary 2 moves)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
