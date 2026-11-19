#!/usr/bin/env python3
"""Check whether Archives of Nethys carries each corpus footnote.

Research script for phynics/cast-iron-data#109. The build does not run it.
Owner rule of 2026-11-19: a footnote stays when its text appears on Archives
of Nethys (AoN) or in the Paizo PRD, and goes when neither carries it.

For each entry with a footnote block (and each entry listed in
``docs/research/editorial-notes-outside-footnotes.tsv``) the script fetches
one AoN page and measures two scores, as ``footnote-print-check-nested.py``
does for the books:

* ``note``: the share of the note's four-word shingles found on the page;
* ``control``: the same share for the entry's own body text. A high control
  proves that the page is the entry's page.

The page is the entry's ``aon`` path when the corpus stores one. Otherwise
the script derives at most two candidate URLs from the entry's kind and title
and keeps the first whose control reaches 0.3; rule entries use the AoN rule
navigation in ``docs/research/aon-rule-sections.json``. A derived URL is
research evidence only; the corpus stores no guessed path.

When no derived URL reaches that control, the script asks AoN search for the
title and tries up to two display links whose name equals the title.

The PRD check needs no request: the corpus's ``ref/carried/`` entries are the
PSRD-Data parse of Paizo's Pathfinder Reference Document, so ``on_prd`` is
``yes`` when the note's shingles reach 0.6 against that text. It covers only
the books that Paizo put in the PRD, and only their carried entries.
``--legacy-books`` adds the chapter pages of those books on the legacy PRD
(https://legacy.aonprd.com/), crawled from its front page within each book folder (spell and monster
folders skipped), to
the same PRD text. A note shorter than four words (such as
"Bonus feat.") occurs all over the PRD, so its ``on_prd`` is ``unchecked``.

A note that is not on its entry's page is then matched against every cached
AoN page; at 0.6 or more its verdict is ``yes-other-page`` (for example a
table note of *detect evil* copied into *detect chaos*).

A note under four words is matched as its printed key, marker then note
("D domain spell"), because the bare words also occur in ordinary text.

Verdicts per note: ``yes`` (note >= 0.6), ``no`` (note <= 0.2 with control
>= 0.3), ``partial`` (between), ``unchecked`` (no page with control >= 0.3).

Politeness: one request at a time, a pause between requests (``--delay``,
default 2 s), a page cache (``CACHE_DIR``, kept out of the repository), and a
stop on HTTP 403 or 429 or on ten failed requests in a row.

Usage::

    python3 scripts/footnote-aon-check-nested.py CORPUS CACHE_DIR --out aon.tsv
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import html
import json
import re
import sys
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://aonprd.com/"
# A page counts as the entry's page when this share of the entry's body
# shingles occurs on it. Wrong pages score near 0.
CONTROL = 0.3
FOOTNOTE_HEADING = re.compile(r"^######\s+(.*?)\s*\{#(footnote-[^}]+)\}\s*$")
HEADING = re.compile(r"^#{1,6}\s")

# Display pages by corpus category; {t} is the URL-quoted title.
PAGES = {
    "monster": ["MonsterDisplay.aspx?ItemName={t}"],
    "npc": ["NPCDisplay.aspx?ItemName={t}"],
    "faith": ["DeityDisplay.aspx?ItemName={t}"],
    "spell": ["SpellDisplay.aspx?ItemName={t}"],
    "feat": ["FeatDisplay.aspx?ItemName={t}"],
    "pclass": ["PrestigeClassesDisplay.aspx?ItemName={t}"],
    "template": ["MonsterTemplates.aspx?ItemName={t}", "MonsterTemplates.aspx?ItemName={t}%20Creature"],
    "family": ["MonsterFamily.aspx?ItemName={t}"],
    "magic-wondrous": ["MagicWondrousDisplay.aspx?FinalName={t}", "MagicWondrousDisplay.aspx?ItemName={t}"],
    "magic-artifact": ["MagicArtifactsDisplay.aspx?ItemName={t}"],
    "magic-rod": ["MagicRodsDisplay.aspx?FinalName={t}", "MagicRodsDisplay.aspx?ItemName={t}"],
    "magic-staff": ["MagicStavesDisplay.aspx?ItemName={t}"],
    "magic-poison": ["EquipmentMiscDisplay.aspx?ItemName={t}"],
    "eq-spellbook": ["EquipmentMiscDisplay.aspx?ItemName={t}"],
    "eq-misc": ["EquipmentMiscDisplay.aspx?ItemName={t}"],
    "eq-reagent": ["EquipmentMiscDisplay.aspx?ItemName={t}"],
    "eq-weapon": ["EquipmentWeaponsDisplay.aspx?ItemName={t}"],
    "eq-armor": ["EquipmentArmorDisplay.aspx?ItemName={t}"],
    "drug": ["DrugsDisplay.aspx?ItemName={t}"],
    "haunt": ["HauntsDisplay.aspx?ItemName={t}"],
    "discovery": ["ClassDisplay.aspx?ItemName=Alchemist"],
}


def norm(text: str) -> list[str]:
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = text.lower().replace("’", "'").replace("‘", "'")
    return re.sub(r"[^a-z0-9]+", " ", text).split()


def shingles(words, n=4):
    return {tuple(words[i:i + n]) for i in range(len(words) - n + 1)}


def score(words, page_words, page_sh) -> float:
    if not words:
        return 0.0
    if len(words) < 4:
        return 1.0 if " " + " ".join(words) + " " in page_words else 0.0
    sh = shingles(words)
    return sum(1 for s in sh if s in page_sh) / len(sh)


class Blocked(Exception):
    pass


class Fetcher:
    def __init__(self, cache: Path, delay: float, base: str = BASE):
        self.cache, self.delay, self.base = cache, delay, base
        cache.mkdir(parents=True, exist_ok=True)
        self.failures = 0
        self.requests = 0

    def get(self, path: str) -> str | None:
        key = self.cache / (hashlib.sha256((self.base + path).encode()).hexdigest()[:24] + ".html")
        if self.base == BASE and not key.exists():
            # Caches written before the base was part of the key.
            old = self.cache / (hashlib.sha256(path.encode()).hexdigest()[:24] + ".html")
            key = old if old.exists() else key
        if key.exists():
            data = key.read_text(encoding="utf-8", errors="replace")
            return data or None
        time.sleep(self.delay)
        self.requests += 1
        req = urllib.request.Request(self.base + path, headers={"User-Agent": "Mozilla/5.0 (research client)"})
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                data = resp.read().decode("utf-8", errors="replace")
            self.failures = 0
        except urllib.error.HTTPError as err:
            if err.code in (403, 429):
                raise Blocked(f"HTTP {err.code} on {path}")
            self.failures += 1
            data = ""
        except (urllib.error.URLError, TimeoutError) as err:
            self.failures += 1
            if self.failures >= 10:
                raise Blocked(f"ten failures in a row; last: {err}")
            return None
        if self.failures >= 10:
            raise Blocked("ten failed requests in a row")
        key.write_text(data, encoding="utf-8")
        return data or None


def page_text(raw: str) -> tuple[str, set]:
    raw = re.sub(r"<script.*?</script>|<style.*?</style>", " ", raw, flags=re.S)
    main = raw.find("MainContent")
    raw = raw[main:] if main >= 0 else raw
    words = norm(html.unescape(re.sub(r"<[^>]+>", " ", raw)))
    return " " + " ".join(words) + " ", shingles(words)


def split_entry(body: str):
    lines = body.split("\n")
    notes, keep, i = [], [], 0
    while i < len(lines):
        m = FOOTNOTE_HEADING.match(lines[i])
        if m:
            j, para, seen = i + 1, [], False
            while j < len(lines):
                if HEADING.match(lines[j]) or lines[j].startswith(":::"):
                    break
                if not lines[j].strip():
                    if seen:
                        break
                    j += 1
                    continue
                seen = True
                para.append(lines[j])
                j += 1
            # Key letters: the label, and the anchor suffix, which keeps the
            # source letter when the importer renumbered the label
            # ("^1^ {#footnote-d}").
            keys = {m.group(1).strip("^ "), m.group(2).removeprefix("footnote-")}
            notes.append((sorted(k for k in keys if len(k) <= 2), " ".join(para)))
            i = j
            continue
        line = lines[i]
        if not line.startswith((":::", "|", "#")) and ":" not in line[:25]:
            keep.append(line)
        i += 1
    return notes, lines, "\n".join(keep)


def rule_index(repo: Path) -> dict[str, str]:
    data = json.loads((repo / "docs/research/aon-rule-sections.json").read_text(encoding="utf-8"))
    out: dict[str, str] = {}

    def walk(node):
        if isinstance(node, dict):
            if "label" in node and "url" in node:
                out.setdefault(node["label"].lower(), node["url"].replace(BASE, ""))
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)
    walk(data)
    return out


def candidates(front: dict, category: str, rules: dict[str, str]) -> list[str]:
    if front.get("aon"):
        return [front["aon"]]
    title = front.get("title", "")
    q = urllib.parse.quote(title)
    if category == "rule":
        hit = rules.get(title.lower())
        return [hit] if hit else []
    if category.startswith("arc-"):
        cls = category[4:].replace("-", " ").title()
        return [f"ArchetypeDisplay.aspx?FixedName={urllib.parse.quote(cls + ' ' + title)}"]
    return [p.format(t=q) for p in PAGES.get(category, [])]


def squash(name: str) -> str:
    return re.sub(r"[^a-z0-9]", "", html.unescape(name).lower())


def search_candidates(fetch: "Fetcher", front: dict, category: str) -> list[str]:
    """Display links from AoN search whose name equals the entry title.

    Names compare without spaces and punctuation, so "Figurine of Wondrous
    Power (Obsidian Steed)" matches AoN's "Figurine of Wondrous PowerObsidian
    Steed". A title such as "Dispater (Archdevil)" also matches by its base.
    """
    title = front.get("title", "")
    base = re.sub(r"\s*\(.*\)$", "", title)
    raw = fetch.get("Search.aspx?Query=" + urllib.parse.quote(base))
    if not raw:
        return []
    wanted = {squash(title), squash(base), squash(title.replace("(", "").replace(")", ""))}
    prefer = [p.split("?")[0] for p in PAGES.get(category, [])]
    found = []
    for href in sorted(set(re.findall(r'href="([A-Za-z]+\.aspx\?[A-Za-z]+=[^"]+)"', raw))):
        if squash(href.split("=", 1)[1]) in wanted:
            found.append(html.unescape(href).replace(" ", "%20"))
    found.sort(key=lambda h: (h.split("?")[0] not in prefer, h))
    return found[:2]


def prd_index(corpus: Path) -> tuple[str, set]:
    """Text of the PSRD-Data carry-over entries, the corpus copy of the PRD."""
    words: list[str] = []
    for path in sorted((corpus / "entries" / "carried").rglob("*.md")):
        words += norm(path.read_text(encoding="utf-8")) + ["|"]
    return " " + " ".join(words) + " ", shingles(words)


LEGACY = "https://legacy.aonprd.com/"


def legacy_prd(fetch: "Fetcher", books: list[str], cap: int) -> tuple[str, set]:
    """Text of the legacy PRD pages of ``books``, following links within each book.

    Spell and monster folders are skipped: their pages carry no rule-table notes.
    """
    front = fetch.get("") or ""
    queue = sorted({h for h in re.findall(r'href\s*=\s*"([^"#:]+\.html)"', front)
                    if h.split("/")[0] in books})
    seen, words = set(), []
    while queue and len(seen) < cap:
        page = queue.pop(0)
        if page in seen:
            continue
        seen.add(page)
        raw = fetch.get(page) or ""
        words += norm(html.unescape(re.sub(r"<[^>]+>", " ", re.sub(r"<script.*?</script>", " ", raw, flags=re.S)))) + ["|"]
        folder = page.rsplit("/", 1)[0]
        for h in re.findall(r'href\s*=\s*"([^"#:]+\.html)"', raw):
            if h.startswith("/") or ".." in h:
                continue
            full = f"{folder}/{h}"
            # Rule tables only: spell and monster pages carry no rule-table notes.
            if re.search(r"spells?/|monsters?/|bestiary/[a-z]", full, re.I) and not full.startswith("bestiary/u"):
                continue
            if full not in seen and full.split("/")[0] in books:
                queue.append(full)
    print("legacy PRD pages", len(seen))
    return " " + " ".join(words) + " ", shingles(words)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("corpus", type=Path)
    ap.add_argument("cache", type=Path)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--delay", type=float, default=2.0)
    ap.add_argument("--limit", type=int, default=0, help="check only the first N entries (for a trial)")
    ap.add_argument("--ids", nargs="*", default=[], help="check only these entry ids (for a trial)")
    ap.add_argument("--legacy-books", nargs="*", default=[],
                    help="legacy PRD book folders to crawl, such as coreRulebook")
    ap.add_argument("--legacy-cap", type=int, default=1000)
    args = ap.parse_args()
    repo = Path(__file__).resolve().parent.parent
    rules = rule_index(repo)
    outside_tsv = repo / "docs/research/editorial-notes-outside-footnotes.tsv"
    outside: dict[str, list[tuple[str, str]]] = {}
    if outside_tsv.exists():
        with outside_tsv.open(encoding="utf-8") as fh:
            for r in csv.DictReader(fh, delimiter="\t"):
                outside.setdefault(r["entry_id"], []).append((r["line"], r["text"]))

    fetch = Fetcher(args.cache, args.delay)
    prd_words, prd_sh = prd_index(args.corpus)
    if args.legacy_books:
        try:
            lw, ls = legacy_prd(Fetcher(args.cache / "legacy", args.delay, LEGACY),
                                args.legacy_books, args.legacy_cap)
            prd_words += lw
            prd_sh |= ls
            # Pages cached by earlier crawls of the same site count too.
            for cached in sorted((args.cache / "legacy").glob("*.html")):
                prd_sh |= page_text(cached.read_text(encoding="utf-8", errors="replace"))[1]
        except Blocked as err:
            print("STOPPED legacy PRD crawl:", err)
            return 2
    rows, done, stopped = [], 0, ""
    pending: list[tuple[int, list[str]]] = []
    for path in sorted((args.corpus / "entries").rglob("*.md")):
        text = path.read_text(encoding="utf-8")
        if not text.startswith("+++\n"):
            continue
        end = text.index("\n+++\n", 4)
        front = tomllib.loads(text[4:end])
        eid = front["id"]
        body = text[end + 5:]
        if "{#footnote-" not in body and eid not in outside:
            continue
        if args.ids and eid not in args.ids:
            continue
        if args.limit and done >= args.limit:
            break
        done += 1
        category = path.relative_to(args.corpus / "entries").parts[0]
        notes, lines, rest = split_entry(body)
        best = ("", 0.0, "", set())
        try:
            for cand in candidates(front, category, rules):
                raw = fetch.get(cand)
                if not raw:
                    continue
                pw, ps = page_text(raw)
                ctl = score(norm(rest)[:400], pw, ps)
                if ctl > best[1]:
                    best = (cand, ctl, pw, ps)
                if ctl >= CONTROL:
                    break
            if best[1] < CONTROL:
                for cand in search_candidates(fetch, front, category):
                    raw = fetch.get(cand)
                    if not raw:
                        continue
                    pw, ps = page_text(raw)
                    ctl = score(norm(rest)[:400], pw, ps)
                    if ctl > best[1]:
                        best = (cand, ctl, pw, ps)
                    if ctl >= CONTROL:
                        break
        except Blocked as err:
            stopped = str(err)
            break
        url, ctl, pw, ps = best
        items = [("block", str(seq), label, note) for seq, (label, note) in enumerate(notes, 1)]
        items += [("outside", line, [], t) for line, t in outside.get(eid, [])]
        for where, key, label, note in items:
            words = norm(note)
            # A note under four words ("Bonus feat.") also occurs in ordinary
            # text ("gains Toughness as a bonus feat"); require the printed key
            # form, marker then note ("D domain spell").
            probes = [norm(k) + words for k in label] if len(words) < 4 else [words]
            s = max((score(p_, pw, ps) for p_ in probes), default=0.0) if url else 0.0
            if not url or ctl < CONTROL:
                verdict = "unchecked"
            elif s >= 0.6:
                verdict = "yes"
            elif s <= 0.2:
                verdict = "no"
            else:
                verdict = "partial"
            pending.append((len(rows), words))
            p = score(norm(note), prd_words, prd_sh)
            rows.append({"entry_id": eid, "where": where, "key": key, "aon_page": url if ctl >= CONTROL else "",
                         "aon_source": ("corpus" if front.get("aon") else "derived") if url and ctl >= CONTROL else "",
                         "note_score": f"{s:.2f}", "control_score": f"{ctl:.2f}", "on_aon": verdict,
                         "prd_score": f"{p:.2f}",
                         "on_prd": "unchecked" if len(norm(note)) < 4 else ("yes" if p >= 0.6 else "no")})

    # A note that is not on its entry's page may be on another AoN page, such
    # as the base spell of a "functions like" spell. Search every cached page.
    pool: set = set()
    for cached in sorted(args.cache.glob("*.html")):  # AoN pages only, not legacy/
        pool |= page_text(cached.read_text(encoding="utf-8", errors="replace"))[1]
    for idx, words in pending:
        row = rows[idx]
        if row["on_aon"] == "yes" or len(words) < 4:
            continue
        sh = shingles(words)
        other = sum(1 for x in sh if x in pool) / len(sh)
        if other >= 0.6:
            row["on_aon"] = "yes-other-page"
            row["note_score"] = f"{other:.2f}"

    with args.out.open("w", encoding="utf-8", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=["entry_id", "where", "key", "aon_page", "aon_source",
                                           "note_score", "control_score", "on_aon", "prd_score", "on_prd"],
                           delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    from collections import Counter
    print("entries", done, "requests", fetch.requests, Counter(r["on_aon"] for r in rows),
          "prd", Counter(r["on_prd"] for r in rows))
    if stopped:
        print("STOPPED:", stopped)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
