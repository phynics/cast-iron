#!/usr/bin/env python3
"""Inventory and classify the footnote blocks of a corpus checkout.

Research script for phynics/cast-iron-data#109. The build does not run it. It
reads ``entries/`` and ``publications.toml`` and never writes to the corpus.

A footnote block is a level-six heading with a ``{#footnote-...}`` anchor and
the paragraphs after it (docs/corpus-format.md, "Footnotes"). Blocks are keyed
by entry id and ``seq``, the block's position in the entry, because a few
entries repeat an anchor. For each block the script records the entry, the
marker, the note text, whether the entry body carries the marker, the
provenance, and a classification:

``printed``
    Printed text: the automatic print check found it in a source book
    (``print-match``, which includes close paraphrase), a hand check found the rule printed in other words
    (``paraphrase``), or it restates a printed marker key such as "Bonus feat."
    (``legend``).
``editorial``
    Not printed: commentary (``commentary``), a page citation the dataset
    added (``source-citation``), or a pointer the dataset wrote in place of a
    printed reference (``pointer``), or a dataset addition that joins material
    from other books (``compiled``).
``unclear``
    Neither the rules nor a check decides it.

Inputs, in rising precedence: the text signals; ``--reuse`` (an earlier output,
for its ``auto_print`` and ``review`` columns); ``--auto`` (the output of
``scripts/footnote-print-check-nested.py``); ``--review`` (a hand review TSV
with ``entry_id``, ``seq``, ``review``, ``subclass``, ``evidence``). A hand
review wins over the automatic check, and the automatic check wins over the
text signals for a block with no editorial signal.

Reuse is checked against the text. Each output row carries ``text_sha``, a
hash of the note's or paragraph's full text. A row of ``--reuse`` or
``--outside-review`` gives its print check, hand review and hand AoN/PRD
verdict to the current row only when the current text has the same hash; a
row whose text changed starts unreviewed. (An input without ``text_sha``
compares the 160-character ``text`` excerpt.) A reused hand AoN/PRD verdict
also needs the AoN and PRD check verdicts of ``--aon`` to equal the ones that
the reviewer saw (the row's ``aon_check`` and ``prd_check``). When they
differ, the row becomes a ``candidate`` with the basis ``conflict: ...`` and
keeps the old hand evidence, until a new hand verdict (``--aon-review``)
settles it. ``--aon-review`` verdicts are current and always apply.

``--outside-tsv`` also lists paragraphs outside the footnote blocks that carry
a strong editorial signal ("this app", "rules-as-written"). Printed text also
says "consult your GM", so each of those rows needs review.

Usage, to regenerate the committed TSV from itself::

    python3 scripts/editorial-footnotes-nested.py /path/to/cast-iron-data \\
        --reuse docs/research/editorial-footnotes.tsv \\
        --outside-review docs/research/editorial-notes-outside-footnotes.tsv \\
        --tsv docs/research/editorial-footnotes.tsv \\
        --outside-tsv docs/research/editorial-notes-outside-footnotes.tsv

At a new corpus pin, run ``scripts/footnote-print-check-nested.py`` first and
pass its output as ``--auto``; rows whose text changed lose their reviews (see
above). Run ``scripts/footnote-aon-check-nested.py`` after this script has
written ``--outside-tsv`` (it reads that file), then run this script again
with its output as ``--aon``. The script prints how many reviews it dropped
and how many hand verdicts conflict with the new checks.

Owner rule of 2026-11-19, as ``on_aon_prd``:

``yes``
    One named AoN page or PRD document carries the WHOLE note as written
    (its words form one unbroken run of the page's words after normalising
    case, punctuation and spacing; ``carried_by`` names it), or a hand check
    found it there.
``no``
    The entry's AoN page does not carry it, no other AoN page this run read
    carries 60 % of it, AND the PRD check ran and found it in no document.
``candidate``
    Needs review before anything is removed: a fuzzy match (60 % or more of
    the note's shingles on one page, but not the whole note), a partial
    match, a paraphrase, an AoN "no" with the PRD unchecked (a note under four
    words), an unresolved AoN page with a PRD "no", or a hand verdict that
    conflicts with a newer check.
``unchecked``
    Neither check could run: no AoN page, and a note under four words.

Owner decision of 2026-11-20 (cast-iron-data #117), as ``presence_basis = print``:
a note also stays when the printed book carries it. A block whose ``on_aon_prd``
is ``no`` and whose ``classification`` is ``printed`` (``print-match``,
``paraphrase`` or ``legend``) keeps ``on_aon_prd = no``, because neither AoN nor
the PRD carries it, but gets the basis ``print`` and the action ``keep``. A ``no``
block that is ``editorial`` or ``unclear`` stays ``remove``. See
docs/decisions/2026-11-20-printed-footnotes-stay.md.

The name ends in ``-nested.py`` so that the docs row of the AGENTS.md test
scope covers it.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import re
import sys
import tomllib
from collections import Counter
from pathlib import Path

HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*(?:\{#([^}]+)\})?\s*$")
FOOTNOTE_HEADING = re.compile(r"^######\s+(.*?)\s*\{#(footnote-[^}]+)\}\s*$")

# Strong editorial signals. One match makes a block editorial.
EDITORIAL = [
    ("consult-gm", re.compile(r"consult (?:with )?your gm", re.I)),
    ("rules-as-written", re.compile(r"rules[- ]as[- ]written|\bRAW\b|strict(?:ly)? read", re.I)),
    ("website", re.compile(r"website|web site|archives of nethys|\baon\b|d20pfsrd|paizo\.com|online", re.I)),
    ("paizo-3rd-person", re.compile(r"\bPaizo(?:'s)?\b(?! Inc)", re.I)),
    ("original-text", re.compile(r"original (?:text|printing|version|wording)|was written before|"
                                 r"(?:mis|re)printed|typo|erratum|errata|\bFAQ\b|developer", re.I)),
    ("editor-voice", re.compile(r"\b(?:we|our|us)\b|presumably|it is unclear|it'?s unclear|"
                                r"appears to be|seems to|probably|likely intended|"
                                r"no indication|does not specify|doesn't specify|not specified|"
                                r"is not given|isn't given|\bsic\b|this app\b|this entry\b|"
                                r"(?:is|as) a (?:non-official )?suggestion|non-official|unofficial|mathematically|"
                                r"feels like|verbatim", re.I)),
    ("source-critique", re.compile(r"original (?:source|blog post|blog)|orignal blog|"
                                   r"no such (?:spell|feat|hex|zombie|creature|ability)|"
                                   r"has not been published|have been published|"
                                   r"was not published|no description .{0,40}published|"
                                   r"(?:was|were) later|later (?:printed|published)|"
                                   r"no (?:level|alignment|cost)[\w -]{0,40}(?:given|provided)|"
                                   r"not (?:included|provided) in|no official|erratta|"
                                   r"the printed table|contradicts|2nd edition|"
                                   r"in the source text|added by other sourcebooks", re.I)),
]

# A note that matches one of these whole is a common printed footnote.
PRINTED = [
    re.compile(r"^(?:this is a )?bonus feats?\.?$", re.I),
    re.compile(r"^(?:mythic )?(?:domain|bloodline|patron|mystery|school|"
               r"spirit|inquisition|subdomain|psychic discipline|focus)"
               r"(?: spells?| spell-like abilit(?:y|ies))?\.?$", re.I),
    re.compile(r"^(?:mythic (?:feat|spell|ability|power)s?|mythic)\.?$", re.I),
    re.compile(r"^(?:see (?:above|below|text|page \d+)|see .{1,80})\.?$", re.I),
    re.compile(r"^(?:spell-like ability|racial (?:bonus|modifier)|"
               r"from (?:a |an )?\w[\w ']{0,40})\.?$", re.I),
]

BOOK_TITLE = re.compile(r"_([^_]{3,80})_|\*([^*]{3,80})\*")


def parse_entry(path: Path):
    text = path.read_text(encoding="utf-8")
    if not text.startswith("+++\n"):
        return None
    end = text.index("\n+++\n", 4)
    front = tomllib.loads(text[4:end])
    body = text[end + 5:]
    return front, body


def paragraphs_after(lines: list[str], start: int):
    """Return the note after the footnote heading at ``start``.

    The note is the one paragraph after the heading (docs/corpus-format.md,
    "Footnotes"); a later paragraph is body text, and the outside scan reads
    it. Returns ([note paragraph], end line index).
    """
    cur: list[str] = []
    i = start + 1
    while i < len(lines):
        line = lines[i]
        if HEADING.match(line) or line.startswith(":::"):
            break
        if not line.strip():
            if cur:
                break
            i += 1
            continue
        cur.append(line.strip())
        i += 1
    return ([" ".join(cur)] if cur else []), i


def strip_md(text: str) -> str:
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    return text.replace("\\", "")


def marker_forms(label: str, anchor: str, entry_id: str):
    plain = label.strip()
    if plain.startswith("^") and plain.endswith("^"):
        plain = plain[1:-1]
    return plain, [
        re.compile(re.escape(f"{entry_id}#{anchor})")),
        re.compile(r"\^" + re.escape(plain) + r"\^"),
    ]


CITATION = re.compile(r"^(?:[^.;]{3,90}? pg\.? ?[\d, -]+(?:, )?)+\.?$")


def classify(note: str):
    """Return (class, subclass, signals) from the note text alone."""
    flat = strip_md(note).strip()
    if CITATION.match(flat):
        return "editorial", "source-citation", ["citation-only"]
    signals = [name for name, rx in EDITORIAL if rx.search(flat)]
    if signals:
        return "editorial", "commentary", signals
    if any(rx.match(flat) for rx in PRINTED):
        return "printed", "legend", ["printed-pattern"]
    return "unclear", "", []


def later_books(note: str, entry_date: str | None, by_title: dict[str, str]):
    if not entry_date:
        return []
    found = []
    for m in BOOK_TITLE.finditer(note):
        title = (m.group(1) or m.group(2)).strip()
        date = by_title.get(title.lower())
        if date and date > entry_date:
            found.append(title)
    return found


def read_tsv(path: Path | None) -> list[dict[str, str]]:
    if not path:
        return []
    with path.open(encoding="utf-8") as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def outside_notes(lines: list[str], mask: set[int]):
    """Yield paragraphs outside the footnote blocks that carry a strong signal.

    A paragraph that starts with a superscript marker (``^1^ The original
    text ...``) is an inline note, the form the importer gave Pathfinder Data
    ``@FN`` notes; it is listed with the signal ``inline-note``.
    """
    para, start = [], 0
    for i, line in enumerate(lines + [""]):
        if i in mask or not line.strip() or HEADING.match(line):
            if para:
                text = strip_md(" ".join(para))
                hits = [n for n, rx in OUTSIDE if rx.search(text)]
                if re.match(r"\^[A-Za-z0-9]{1,2}\^ ", para[0]):
                    hits.insert(0, "inline-note")
                if hits:
                    yield start, hits, text
            para = []
            continue
        if not para:
            start = i
        para.append(line.strip())


# The TSV keeps an excerpt that identifies the note, not the whole text.
EXCERPT = 160

EXPANSIONS = {"D": "(domain spell)", "M": "(mythic spell)", "B": "(bonus spell)"}

# Signals for the scan outside footnote blocks. Printed text says "consult your
# GM" too, so this list is narrower than EDITORIAL and every hit needs review.
OUTSIDE = [
    ("this-app", re.compile(r"\bthis app\b", re.I)),
    ("rules-as-written", re.compile(r"rules[- ]as[- ]written", re.I)),
    ("website", re.compile(r"open-content website|paizo\.com|archives of nethys|d20pfsrd", re.I)),
    ("original-text", re.compile(r"the original (?:text|source)|no official errata|no errata", re.I)),
    ("consult-gm", re.compile(r"consult your gm", re.I)),
]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("corpus", type=Path)
    ap.add_argument("--auto", type=Path, help="output of footnote-print-check-nested.py")
    ap.add_argument("--review", type=Path, help="hand review: entry_id, seq, review, subclass, evidence")
    ap.add_argument("--reuse", type=Path, help="an earlier --tsv output; reuses its auto and review columns")
    ap.add_argument("--tsv", type=Path)
    ap.add_argument("--outside-tsv", type=Path)
    ap.add_argument("--outside-review", type=Path,
                    help="hand review of --outside-tsv rows: entry_id, line, review, evidence")
    ap.add_argument("--aon", type=Path, help="output of footnote-aon-check-nested.py")
    ap.add_argument("--aon-review", type=Path,
                    help="hand AoN/PRD verdicts: entry_id, where, key, on_aon_prd, aon_evidence")
    args = ap.parse_args()

    def text_sha(text: str) -> str:
        return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]

    def same_text(old: dict[str, str], text: str) -> bool:
        """True when an earlier output row was made from this exact text.

        A row without `text_sha` is never reused: its excerpt cannot show that
        the text after the first EXCERPT characters is unchanged."""
        return bool(old.get("text_sha")) and old["text_sha"] == text_sha(text)

    def verdicts(check: str) -> str:
        return check.split(" ", 1)[0]

    # Earlier output rows, keyed by (entry_id, "block" or "outside", seq or
    # line). They are used only after same_text() accepts them.
    prior: dict[tuple[str, str, str], dict[str, str]] = {}
    for row in read_tsv(args.reuse):
        prior[(row["entry_id"], "block", row["seq"])] = row
    for row in read_tsv(args.outside_review):
        prior[(row["entry_id"], "outside", row["line"])] = row
    # This run's AoN/PRD checks: (aon_check, prd_check, carried_by).
    fresh: dict[tuple[str, str, str], tuple[str, str, str]] = {}
    for row in read_tsv(args.aon):
        a = f'{row["on_aon"]} note={row["note_score"]} control={row["control_score"]}'
        if row["aon_page"]:
            a += f' page={row["aon_page"]}'
        if row["aon_match"]:
            a += f' match={row["aon_match"]} ({row["match_score"]})'
        p = f'{row["on_prd"]} best={row["prd_score"]} pooled={row["prd_pooled"]}'
        if row["prd_doc"]:
            p += f' doc={row["prd_doc"]}'
        # The page or document that carries the note, or else the one that
        # holds most of it.
        ra, rp = row["on_aon"], row["on_prd"]
        if ra == "yes":
            by = row["aon_page"]
        elif ra == "yes-other-page":
            by = row["aon_match"]
        elif rp == "yes":
            by = row["prd_doc"]
        elif ra == "fuzzy":
            by = row["aon_match"] or row["aon_page"]
        elif rp in ("fuzzy", "partial"):
            by = row["prd_doc"]
        else:
            by = row["aon_page"] if ra == "partial" else ""
        fresh[(row["entry_id"], row["where"], row["key"])] = (a, p, by)
    hand: dict[tuple[str, str, str], tuple[str, str]] = {}
    for row in read_tsv(args.aon_review):
        hand[(row["entry_id"], row["where"], row["key"])] = (
            row.get("carried_by", ""), f'{row["on_aon_prd"]}: {row["aon_evidence"]}')
    stats: Counter = Counter()

    def presence(key, text):
        old = prior.get(key)
        if old is not None and not same_text(old, text):
            stats["text changed: earlier row not reused"] += 1
            old = None
        if key in fresh:
            a, p, by = fresh[key]
        elif old is not None and old.get("aon_check"):
            a, p, by = old["aon_check"], old["prd_check"], old.get("carried_by", "")
        else:
            a, p, by = "", "", ""
        ev, conflict = "", ""
        if key not in hand and old is not None and old.get("presence_basis", "").startswith("conflict:"):
            # An unresolved conflict keeps its original check context and stays
            # a candidate until an explicit --aon-review row settles it.
            stats["hand verdict conflicts with the new check"] += 1
            return {"aon_check": old["aon_check"], "prd_check": old["prd_check"],
                    "aon_evidence": old.get("aon_evidence", ""), "on_aon_prd": "candidate",
                    "presence_basis": old["presence_basis"], "carried_by": old.get("carried_by", ""),
                    "action": "review"}
        if key in hand:
            hand_by, ev = hand[key]
            by = hand_by or by
        elif old is not None and old.get("aon_evidence"):
            old_ev = f'{old["on_aon_prd"]}: {old["aon_evidence"]}'
            if (verdicts(a), verdicts(p)) == (verdicts(old["aon_check"]), verdicts(old["prd_check"])):
                ev, by = old_ev, old.get("carried_by", "") or by
            else:
                stats["hand verdict conflicts with the new check"] += 1
                ev = old_ev
                conflict = (f'conflict: hand {old["on_aon_prd"]} on aon {verdicts(old["aon_check"])}'
                            f'/prd {verdicts(old["prd_check"])}, now {verdicts(a)}/{verdicts(p)}')
        on_aon = verdicts(a) or "unchecked"
        on_prd = verdicts(p) or "unchecked"
        if conflict:
            verdict, basis = "candidate", conflict
        elif ev:
            verdict, note = ev.split(": ", 1)
            basis = "hand: " + note.split(":", 1)[0] if verdict == "candidate" else "hand"
        elif on_aon in ("yes", "yes-other-page") or on_prd == "yes":
            verdict = "yes"
            basis = {"yes": "aon-page", "yes-other-page": "aon-other-page"}.get(on_aon, "prd-document")
        elif on_aon == "no" and on_prd == "no":
            verdict, basis = "no", "aon-no+prd-no"
        elif on_aon == "unchecked" and on_prd == "unchecked":
            verdict, basis = "unchecked", "aon-unresolved+prd-unchecked"
        else:
            verdict = "candidate"
            if "fuzzy" in (on_aon, on_prd):
                basis = "fuzzy"
            elif "partial" in (on_aon, on_prd):
                basis = "partial"
            elif on_prd == "unchecked":
                basis = "aon-no+prd-unchecked"
            else:
                basis = "aon-unresolved+prd-no"
        action = {"yes": "keep", "no": "remove", "candidate": "review"}.get(verdict, "check")
        return {"aon_check": a, "prd_check": p, "aon_evidence": ev.split(": ", 1)[1] if ev else "",
                "on_aon_prd": verdict, "presence_basis": basis,
                "carried_by": by if verdict in ("yes", "candidate") else "", "action": action}

    pubs = tomllib.loads((args.corpus / "publications.toml").read_text(encoding="utf-8"))
    pub_date = {p["id"]: p.get("publication_date") for p in pubs["publication"]}
    pub_title = {p["id"]: p["title"] for p in pubs["publication"]}
    by_title = {p["title"].lower(): p.get("publication_date") for p in pubs["publication"]}

    # Print checks and hand reviews of this run; earlier ones come from
    # ``prior`` once the text matches.
    auto: dict[tuple[str, str], str] = {}
    review: dict[tuple[str, str], tuple[str, str, str]] = {}
    for row in read_tsv(args.auto):
        auto[(row["entry_id"], row["seq"])] = (
            f'{row["auto_verdict"]} note={row["note_score"]} control={row["control_score"]}'
            + (f' in {row["books"]}' if row["books"] else ""))
    for row in read_tsv(args.review):
        review[(row["entry_id"], row["seq"])] = (row["review"], row["subclass"], row["evidence"])

    def earlier(key, text):
        old = prior.get(key)
        return old if old is not None and same_text(old, text) else None
    rows, outside = [], []
    bodies: dict[str, str] = {}
    blockless: list[tuple[str, str]] = []
    for path in sorted((args.corpus / "entries").rglob("*.md")):
        parsed = parse_entry(path)
        if not parsed:
            continue
        front, body = parsed
        entry_id = front["id"]
        lines = body.split("\n")
        notes = []
        for idx, line in enumerate(lines):
            m = FOOTNOTE_HEADING.match(line)
            if m:
                paras, end = paragraphs_after(lines, idx)
                notes.append((idx, end, m.group(1), m.group(2), paras))
        mask = set()
        for idx, end, *_ in notes:
            mask.update(range(idx, end))
        prov = front.get("provenance", [])
        datasets = ";".join(sorted({p.get("dataset", "") for p in prov}))
        records = ";".join(f'{p.get("path", "")}#{p.get("record", "")}' for p in prov)
        category = path.relative_to(args.corpus / "entries").parts[0]
        for start, hits, text in outside_notes(lines, mask):
            okey = (entry_id, "outside", str(start + 1))
            old = earlier(okey, text)
            rv, ev = (old["review"], old["evidence"]) if old else ("", "")
            outside.append({"entry_id": entry_id, "category": category, "line": str(start + 1),
                            "signals": ",".join(hits), "datasets": datasets,
                            "review": rv, "evidence": ev,
                            **presence(okey, text),
                            "text_sha": text_sha(text),
                            "text": text.replace("\t", " ")[:EXCERPT]})
        if not notes:
            for m in re.finditer(r"\^([A-Za-z0-9]{1,2})\^", "\n".join(lines)):
                blockless.append((entry_id, m.group(1)))
            continue
        rest = "\n".join(l for i, l in enumerate(lines) if i not in mask)
        bodies[entry_id] = rest
        labels = {marker_forms(n[2], n[3], entry_id)[0] for n in notes}
        for m in re.finditer(r"\^([A-Za-z0-9]{1,2})\^", rest):
            if m.group(1) not in labels:
                blockless.append((entry_id, m.group(1)))
        sources = front.get("sources", [])
        first = sources[0] if sources else {}
        source_id = first.get("publication") or (front.get("source_ids") or [""])[0]
        entry_date = pub_date.get(source_id)
        for seq, (idx, end, label, anchor, paras) in enumerate(notes, 1):
            plain, forms = marker_forms(label, anchor, entry_id)
            linked = bool(forms[0].search(rest))
            bare = bool(forms[1].search(rest))
            note = " ".join(paras)
            flat = strip_md(note)
            cls, sub, signals = classify(note)
            later = later_books(note, entry_date, by_title)
            key = (entry_id, str(seq))
            old = earlier((entry_id, "block", str(seq)), flat)
            if old is not None:
                auto.setdefault(key, old["auto_print"])
                if old["review"]:
                    review.setdefault(key, (old["review"], old["subclass"], old["evidence"]))
            a = auto.get(key, "")
            evidence = ""
            if key in review:
                cls, sub, evidence = review[key]
            elif cls == "editorial":
                evidence = "signal: " + ",".join(signals)
            elif a.startswith("printed"):
                cls, sub, evidence = "printed", "print-match", "print check: " + a
            elif cls == "printed":
                evidence = "printed marker key (legend); not print-checked at this place"
            rows.append({
                "entry_id": entry_id,
                "seq": str(seq),
                "category": category,
                "kind": front.get("kind", ""),
                "anchor": anchor,
                "marker": plain,
                "marker_in_body": "linked" if linked else ("plain" if bare else "absent"),
                "raw_residue": "yes" if re.search(r"\\?\|[A-Za-z0-9]\\?~", note) else "no",
                "datasets": datasets,
                "records": records,
                "source": pub_title.get(source_id, source_id),
                "source_page": first.get("page", ""),
                "signals": ",".join(signals + [f"later:{t}" for t in later]),
                "auto_print": a,
                "review": review.get(key, ("", "", ""))[0],
                "classification": cls,
                "subclass": sub,
                "evidence": evidence,
                **presence((entry_id, "block", str(seq)), flat),
                "text_sha": text_sha(flat),
                "text": flat.replace("\t", " ")[:EXCERPT],
            })

    # Owner decision of 2026-11-20: a note that neither AoN nor the PRD carries
    # still stays when the printed book carries it.
    for r in rows:
        if r["on_aon_prd"] == "no" and r["classification"] == "printed":
            r["presence_basis"], r["action"] = "print", "keep"

    # Why a marker is absent. The importer wrote "#D", "#M" and "#B" spell
    # markers as text ("(domain spell)") and still emitted the note; a section
    # cut from a shared record keeps the notes but not the marker.
    marked = {}
    for r in rows:
        if r["marker_in_body"] != "absent":
            marked.setdefault(r["text"], set()).add(r["entry_id"])
    for r in rows:
        cause = ""
        if r["marker_in_body"] == "absent":
            exp = EXPANSIONS.get(r["marker"])
            if exp and exp in bodies[r["entry_id"]].lower():
                cause = "marker-expanded-inline"
            elif marked.get(r["text"], set()) - {r["entry_id"]}:
                cause = "shared-record-copy"
            else:
                cause = "marker-dropped"
        r["orphan_cause"] = cause

    # Book prose that merely contains a signal word is not a note; the owner
    # rule does not apply to it.
    for o in outside:
        if o["review"] == "printed":
            o["action"] = "not a note"

    for path, data in ((args.tsv, rows), (args.outside_tsv, outside)):
        if path and data:
            with path.open("w", encoding="utf-8", newline="") as fh:
                w = csv.DictWriter(fh, fieldnames=list(data[0]), delimiter="\t", lineterminator="\n")
                w.writeheader()
                w.writerows(data)

    entries = {r["entry_id"] for r in rows}
    print(f"blocks {len(rows)} in entries {len(entries)}")
    for key in ("classification", "subclass", "marker_in_body", "raw_residue", "datasets"):
        print(key, dict(sorted(Counter(r[key] for r in rows).items())))
    for cls in ("printed", "editorial", "unclear"):
        sel = [r for r in rows if r["classification"] == cls]
        print(cls, "entries", len({r["entry_id"] for r in sel}),
              "by category", dict(Counter(r["category"] for r in sel).most_common()))
    absent = [r for r in rows if r["marker_in_body"] == "absent"]
    print("orphan blocks by class", dict(Counter(r["classification"] for r in absent)))
    print("orphan causes", dict(Counter(r["orphan_cause"] for r in rows if r["orphan_cause"])))
    print("plain markers without a block", len(blockless), "in entries", len({e for e, _ in blockless}),
          dict(Counter(l for _, l in blockless).most_common(8)))
    print("outside paragraphs", len(outside), "entries", len({o["entry_id"] for o in outside}),
          dict(Counter(o["signals"] for o in outside).most_common()))
    print("on_aon_prd", dict(Counter(r["on_aon_prd"] for r in rows)),
          "entries", {v: len({r["entry_id"] for r in rows if r["on_aon_prd"] == v})
                      for v in ("yes", "no", "candidate", "unchecked")})
    print("presence_basis", dict(sorted(Counter((r["on_aon_prd"], r["presence_basis"]) for r in rows).items())))
    print("outside presence_basis", dict(sorted(Counter((o["on_aon_prd"], o["presence_basis"], o["review"])
                                                        for o in outside).items())))
    print("on_aon_prd x classification", dict(Counter((r["on_aon_prd"], r["classification"]) for r in rows)))
    print("on_aon_prd x subclass", dict(Counter((r["on_aon_prd"], r["subclass"]) for r in rows)))
    print("orphans x on_aon_prd", dict(Counter((r["orphan_cause"], r["on_aon_prd"]) for r in rows if r["orphan_cause"])))
    print("outside on_aon_prd x review", dict(Counter((o["on_aon_prd"], o["review"]) for o in outside)))
    print("reuse", dict(sorted(stats.items())) or "all earlier rows matched their text")
    print("outside review", dict(Counter(o["review"] for o in outside)),
          "editorial entries", len({o["entry_id"] for o in outside if o["review"] == "editorial"}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
