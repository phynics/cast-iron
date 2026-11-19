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
PSRD-Data parse of Paizo's Pathfinder Reference Document. ``--legacy-books``
adds the chapter pages of those books on the legacy PRD
(https://legacy.aonprd.com/), crawled from its front page within each book
folder; spell and monster folders are skipped. Each carried entry and each
legacy page is one PRD document. A note shorter than four words (such as
"Bonus feat.") occurs all over the PRD, so its ``on_prd`` is ``unchecked``.

A note that is not on its entry's page is then matched against every other
AoN page that this run read (for example a table note of *detect evil* copied
into *detect chaos*). That is a finite set of pages, not all of AoN.

A note under four words is matched as its printed key, marker then note
("D domain spell"), because the bare words also occur in ordinary text.

Every score is the share of the note's shingles on ONE page or ONE document,
and the output names that page or document, so a reader can open it. A share
pooled over many pages does not show that any page carries the note.

A page *carries* a note only when the WHOLE note occurs on it as written: the
note's words, lower-cased and with punctuation and spacing normalised, form
one unbroken run of the page's words (``note_whole``). A share of 0.9 or more
alone is not enough: a page that omits the note's opening words ("This spell
may be prepared ..." against "may be prepared ...") still holds 0.9 of its
shingles. Such a row is ``fuzzy``.

AoN verdicts (``on_aon``):

* ``yes``: the entry's page carries the whole note.
* ``yes-other-page``: another single AoN page carries the whole note;
  ``aon_match`` names that page.
* ``fuzzy``: the best single page has 0.6 or more of the note but not the
  whole note. Quotations, paraphrases and notes that lose words land here;
  the row needs a hand look.
* ``partial``: the entry's page has more than 0.2 and less than 0.6, and no
  page reaches 0.6.
* ``no``: the entry's page (control >= 0.3) has 0.2 or less, and no other
  page reaches 0.6.
* ``unchecked``: no page with control >= 0.3, and no page reaches 0.6.

PRD verdicts (``on_prd``), from the best single document (``prd_doc``) and
the score pooled over all PRD text (``prd_pooled``): ``yes`` (one document
carries the whole note), ``fuzzy`` (best 0.6 or more, not whole), ``partial`` (best above 0.2, or pooled 0.6 or
more), ``no`` (best 0.2 or less and pooled under 0.6), ``unchecked`` (a note
under four words).

Outside paragraphs are read in full from the corpus at their TSV line, not
from the TSV's 160-character excerpt.

Politeness: one request at a time, a pause between requests (``--delay``,
default 2 s), a page cache (``CACHE_DIR``, kept out of the repository), and a
stop on HTTP 403 or 429 or on ten failed requests in a row. ``--offline``
reads the cache only and makes no request.

Usage, as run for the committed audit::

    python3 scripts/footnote-aon-check-nested.py CORPUS CACHE_DIR --out aon.tsv \\
        --legacy-books coreRulebook gameMasteryGuide ultimateEquipment \\
            ultimateCampaign ultimateCombat ultimateMagic advancedPlayersGuide \\
            occultAdventures unchained bestiary
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
# A page with this share of the note's shingles is tested for the whole note;
# only a page that holds the whole note carries it.
CARRIES = 0.9
# From this share up to CARRIES a page has most of the note: a candidate.
FUZZY = 0.6
# At or below this share the note is absent.
ABSENT = 0.2
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


def whole(words, page_words: str) -> bool:
    """True when the note's words form one unbroken run of the page's words."""
    return bool(words) and " " + " ".join(words) + " " in page_words


class Blocked(Exception):
    pass


class Fetcher:
    def __init__(self, cache: Path, delay: float, base: str = BASE, offline: bool = False):
        self.cache, self.delay, self.base, self.offline = cache, delay, base, offline
        cache.mkdir(parents=True, exist_ok=True)
        self.failures = 0
        self.requests = 0
        self.missing = 0
        # Every display page this run read, by path, for the other-page match.
        self.read: dict[str, str] = {}

    def get(self, path: str) -> str | None:
        key = self.cache / (hashlib.sha256((self.base + path).encode()).hexdigest()[:24] + ".html")
        if self.base == BASE and not key.exists():
            # Caches written before the base was part of the key.
            old = self.cache / (hashlib.sha256(path.encode()).hexdigest()[:24] + ".html")
            key = old if old.exists() else key
        if key.exists():
            data = key.read_text(encoding="utf-8", errors="replace")
            if data and not path.startswith("Search.aspx"):
                self.read[path] = data
            return data or None
        if self.offline:
            self.missing += 1
            return None
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
        if data and not path.startswith("Search.aspx"):
            self.read[path] = data
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


class Docs:
    """Named texts with an inverted shingle index: which document has most of a note."""

    def __init__(self):
        self.names: list[str] = []
        self.texts: list[str] = []
        self.index: dict[tuple, list[int]] = {}

    def add(self, name: str, words: list[str]):
        n = len(self.names)
        self.names.append(name)
        self.texts.append(" " + " ".join(words) + " ")
        for sh in shingles(words):
            self.index.setdefault(sh, []).append(n)

    def best(self, words: list[str], skip: str = "") -> tuple[float, str, float, bool]:
        """(best single-document share, its name, share pooled over all documents,
        whether that document holds the whole note).

        Among the documents with CARRIES or more, the first (by share, then
        name) that holds the whole note is preferred.
        """
        sh = shingles(words)
        if not sh:
            return 0.0, "", 0.0, False
        count: dict[int, int] = {}
        pooled = 0
        for s_ in sh:
            docs = self.index.get(s_)
            if docs:
                pooled += 1
                for d in set(docs):
                    count[d] = count.get(d, 0) + 1
        ranked = sorted(((c, self.names[d]) for d, c in count.items() if self.names[d] != skip),
                        key=lambda x: (-x[0], x[1]))
        if not ranked:
            return 0.0, "", pooled / len(sh), False
        by_name = {name: i for i, name in enumerate(self.names)}
        for c, name in ranked:
            if c / len(sh) < CARRIES:
                break
            if whole(words, self.texts[by_name[name]]):
                return c / len(sh), name, pooled / len(sh), True
        return ranked[0][0] / len(sh), ranked[0][1], pooled / len(sh), False


def prd_index(corpus: Path, docs: Docs):
    """The PSRD-Data carry-over entries, the corpus copy of the PRD, one document each."""
    for path in sorted((corpus / "entries" / "carried").rglob("*.md")):
        text = path.read_text(encoding="utf-8")
        m = re.search(r'^id = "([^"]+)"', text, re.M)
        docs.add(m.group(1) if m else str(path.relative_to(corpus)), norm(text))


LEGACY = "https://legacy.aonprd.com/"


def legacy_prd(fetch: "Fetcher", books: list[str], cap: int, docs: Docs) -> int:
    """Add the legacy PRD pages of ``books`` to ``docs``, following links within each book.

    Spell and monster folders are skipped: their pages carry no rule-table notes.
    """
    front = fetch.get("") or ""
    queue = sorted({h for h in re.findall(r'href\s*=\s*"([^"#:]+\.html)"', front)
                    if h.split("/")[0] in books})
    seen = set()
    while queue and len(seen) < cap:
        page = queue.pop(0)
        if page in seen:
            continue
        seen.add(page)
        raw = fetch.get(page) or ""
        docs.add(LEGACY + page, norm(html.unescape(re.sub(r"<[^>]+>", " ",
                                                          re.sub(r"<script.*?</script>", " ", raw, flags=re.S)))))
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
    return len(seen)


def paragraph_at(lines: list[str], line: int) -> str:
    """The paragraph that starts at 1-based body ``line``, as the outside scan reads it."""
    out = []
    for text in lines[line - 1:]:
        if not text.strip() or HEADING.match(text):
            break
        out.append(text.strip())
    return " ".join(out)


def aon_verdict(own: float, own_whole: bool, ctl: float, url: str,
                other: float, other_whole: bool) -> str:
    resolved = bool(url) and ctl >= CONTROL
    if resolved and own_whole:
        return "yes"
    if other_whole:
        return "yes-other-page"
    if max(own if resolved else 0.0, other) >= FUZZY:
        return "fuzzy"
    if not resolved:
        return "unchecked"
    return "no" if own <= ABSENT else "partial"


def prd_verdict(words: list[str], best: float, pooled: float, is_whole: bool) -> str:
    if len(words) < 4:
        return "unchecked"
    if is_whole:
        return "yes"
    if best >= FUZZY:
        return "fuzzy"
    if best > ABSENT or pooled >= FUZZY:
        return "partial"
    return "no"


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
    ap.add_argument("--offline", action="store_true", help="read the cache only; make no request")
    args = ap.parse_args()
    repo = Path(__file__).resolve().parent.parent
    rules = rule_index(repo)
    outside_tsv = repo / "docs/research/editorial-notes-outside-footnotes.tsv"
    outside: dict[str, list[tuple[str, str]]] = {}
    if outside_tsv.exists():
        with outside_tsv.open(encoding="utf-8") as fh:
            for r in csv.DictReader(fh, delimiter="\t"):
                outside.setdefault(r["entry_id"], []).append((r["line"], r["text"]))

    fetch = Fetcher(args.cache, args.delay, offline=args.offline)
    prd = Docs()
    prd_index(args.corpus, prd)
    if args.legacy_books:
        try:
            legacy_prd(Fetcher(args.cache / "legacy", args.delay, LEGACY, args.offline),
                       args.legacy_books, args.legacy_cap, prd)
        except Blocked as err:
            print("STOPPED legacy PRD crawl:", err)
            return 2
    rows, done, stopped = [], 0, ""
    pending: list[tuple[int, list[str], list[list[str]], float, float, str]] = []
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
        items += [("outside", line, [], paragraph_at(lines, int(line))) for line, _ in outside.get(eid, [])]
        for where, key, label, note in items:
            words = norm(note)
            # A note under four words ("Bonus feat.") also occurs in ordinary
            # text ("gains Toughness as a bonus feat"); require the printed key
            # form, marker then note ("D domain spell").
            probes = [norm(k) + words for k in label] if len(words) < 4 else [words]
            s = max((score(p_, pw, ps) for p_ in probes), default=0.0) if url else 0.0
            own_whole = bool(url) and any(whole(p_, pw) for p_ in probes)
            pending.append((len(rows), words, probes, s, own_whole, ctl, url))
            best, doc, pooled, prd_whole = prd.best(words) if len(words) >= 4 else (0.0, "", 0.0, False)
            rows.append({"entry_id": eid, "where": where, "key": key,
                         "aon_page": BASE + url if url and ctl >= CONTROL else "",
                         "aon_source": ("corpus" if front.get("aon") else "derived") if url and ctl >= CONTROL else "",
                         "note_score": f"{s:.2f}", "note_whole": "yes" if own_whole else "no",
                         "control_score": f"{ctl:.2f}",
                         "aon_match": "", "match_score": "", "match_whole": "", "on_aon": "",
                         "prd_score": f"{best:.2f}", "prd_pooled": f"{pooled:.2f}",
                         "prd_doc": doc if best > ABSENT else "",
                         "prd_whole": "yes" if prd_whole else "no",
                         "on_prd": prd_verdict(words, best, pooled, prd_whole)})

    # A note that is not on its entry's page may be on another AoN page, such
    # as the base spell of a "functions like" spell. Match it against each
    # display page this run read, one page at a time, and name the best one.
    pages = Docs()
    texts: dict[str, str] = {}
    for path_ in sorted(fetch.read):
        words_ = page_text(fetch.read[path_])[0]
        texts[BASE + path_] = words_
        pages.add(BASE + path_, words_.split())
    for idx, words, probes, own, own_whole, ctl, url in pending:
        row = rows[idx]
        page = row["aon_page"]
        if len(words) >= 4:
            other, match, _, other_whole = pages.best(words, skip=page)
        else:
            # A short note in its key form ("B bonus feat") must occur whole.
            hits = sorted(n for n, t in texts.items() if n != page
                          and any(whole(p_, t) for p_ in probes))
            other, match, other_whole = (1.0, hits[0], True) if hits else (0.0, "", False)
        verdict = aon_verdict(own, own_whole, ctl, url, other, other_whole)
        row["on_aon"] = verdict
        if verdict in ("yes-other-page", "fuzzy") and other >= max(own if page else 0.0, FUZZY):
            row["aon_match"], row["match_score"] = match, f"{other:.2f}"
            row["match_whole"] = "yes" if other_whole else "no"

    with args.out.open("w", encoding="utf-8", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]), delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    from collections import Counter
    print("entries", done, "requests", fetch.requests, "missing from cache", fetch.missing,
          "pages", len(fetch.read), Counter(r["on_aon"] for r in rows),
          "prd", Counter(r["on_prd"] for r in rows))
    if stopped:
        print("STOPPED:", stopped)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
