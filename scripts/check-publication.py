#!/usr/bin/env python3
"""Screen a built rules library and record evidence, not a legal clearance.

Run after cargo xtask build-site. The named-material list is deliberately
explicit and incomplete. Named material is allowed only where the entry carries
the Community Use notice path. Any new book needs a separate licensing review.
"""

import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
CONTENT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "dist/content"
manifest = json.loads((CONTENT / "manifest.json").read_text())
library = CONTENT / manifest["content_version"]
catalog = json.loads((library / "catalog.json").read_text())
legal = json.loads((library / "legal.json").read_text())

issues = []
for item in manifest["files"]:
    data = (CONTENT / item["path"]).read_bytes()
    if len(data) != item["bytes"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
        issues.append(f"Manifest mismatch: {item['path']}")

sources = {source["id"] for source in catalog["sources"]}
# The sixteen core books that the first release published must stay in the library. Every
# other publication comes from the corpus `publications.toml`, which `corpus-check` validates
# against each entry's `source_ids`; the compiler builds the catalog from that file.
CORE_SOURCES = {
    "advanced-players-guide", "advanced-race-guide", "bestiary", "bestiary-2", "bestiary-3",
    "bestiary-4", "core-rulebook", "game-mastery-guide", "monster-codex", "mythic-adventures",
    "npc-codex", "technology-guide", "ultimate-campaign", "ultimate-combat",
    "ultimate-equipment", "ultimate-magic",
}
if not CORE_SOURCES <= sources:
    issues.append("Library is missing a core publication: " + ", ".join(sorted(CORE_SOURCES - sources)))
if any(not source["copyright"] for source in catalog["sources"]):
    issues.append("Missing source copyright notice")
if "Cast Iron uses trademarks" not in legal["community_use_notice"]:
    issues.append("Missing filled-in Community Use notice")
if "paizo.com/licenses/communityuse" not in legal["community_use_notice"]:
    issues.append("Outdated Community Use policy URL")
if not any("15. COPYRIGHT NOTICE" in item["text"] for item in legal["licenses"]):
    issues.append("Missing OGL text")
titles = "\n".join(item["title"] for item in legal["section_15"])
for required in ("Core Rulebook", "Bestiary", "Mythic Adventures", "System Reference Document", "Tome of Horrors"):
    if required not in titles:
        issues.append(f"Missing expected attribution: {required}")

# Screen displayed rule content, excluding required legal/source notices.
named_material = re.compile(
    r"\b(?:Golarion|Absalom|Cheliax|Varisia|Aroden|Abadar|Asmodeus|Cayden|"
    r"Desna|Erastil|Gorum|Gozreh|Iomedae|Irori|Lamashtu|Nethys|Norgorber|"
    r"Pharasma|Rovagug|Sarenrae|Shelyn|Torag|Urgathoa|Zon-Kuthon|Valeros|"
    r"Seoni|Ezren|Merisiel|Kyra|Harsk|Seelah)\b", re.IGNORECASE
)


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for child in value:
            yield from strings(child)
    elif isinstance(value, dict):
        for child in value.values():
            yield from strings(child)


def nodes(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from nodes(child)
    elif isinstance(value, list):
        for child in value:
            yield from nodes(child)


COMMUNITY_USE = "legal#community-use"
outbound = re.compile(r"https?://|pathfinderwiki", re.IGNORECASE)
cup_named = 0
counts = {}
entries = []
for file in sorted((library / "entries").glob("*.json")):
    entries.extend(json.loads(file.read_text()))
for entry in entries:
    counts[entry["kind"]] = counts.get(entry["kind"], 0) + 1
    if not set(entry["source_ids"]) <= sources or not entry["source_ids"]:
        issues.append(f"Unreviewed source: {entry['id']}")
    if "ogl-1.0a" not in entry["license_refs"]:
        issues.append(f"Missing OGL reference: {entry['id']}")
    displayed = [entry["title"], entry["body"], entry["fields"], entry.get("tags", [])]
    text = "\n".join(strings(displayed))
    matches = sorted(set(named_material.findall(text)))
    # Proper names are descriptive references under the Community Use Policy, so
    # they may appear only in an entry that carries that notice path.
    if matches and COMMUNITY_USE not in entry["license_refs"]:
        issues.append(f"Named material without the Community Use notice in {entry['id']}: {', '.join(matches)}")
    if matches:
        cup_named += 1
    if outbound.search(text):
        issues.append(f"Outbound address in displayed text of {entry['id']}")
    if any(node.get("t") in {"image", "raw_html"} for node in nodes(entry["body"])):
        issues.append(f"Review artwork or raw HTML in {entry['id']}")
if len(entries) != manifest["entry_count"] or len(entries) != len(catalog["entries"]):
    issues.append("Entry count mismatch")

report = {
    "content_version": manifest["content_version"],
    "entry_count": len(entries),
    "sources": sorted(sources),
    "by_kind": dict(sorted(counts.items())),
    "community_use_named_entries": cup_named,
    "issues": issues,
    "limitations": "Finite named-material screen and structural checks only. Named material is accepted only on entries that carry the Community Use notice path; descriptive Product Identity prose is screened by the retired import (tag psrd-last-buildable) before the corpus was written. Does not classify every sentence or establish legal permission.",
}
output = ROOT / "generated/reports/publication.json"
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
print(json.dumps(report, indent=2, sort_keys=True))
raise SystemExit(bool(issues))
