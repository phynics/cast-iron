#!/usr/bin/env python3
"""Check corpus footnote blocks against the printed books.

Research script for phynics/cast-iron-data#109. The build does not run it.
It needs a directory of the owner's PDFs and ``pdftotext`` on ``PATH``. It
never writes to the corpus and never writes PDF names or extracted text into
its output: the output names the book title only.

For each footnote block of each entry, the script takes the entry's source
publications, finds their PDFs by title, and measures two scores over the
whole text of those books:

* ``note``: the share of the note's four-word shingles that occur in the book
  (for a note shorter than four words, 1 when the whole note occurs, else 0);
* ``control``: the same share for the entry's own body text (the paragraphs
  outside the footnote blocks and the stat block). A high control proves that
  the right book was searched, so a low note score then means "not printed".

Verdicts: ``printed`` (note >= 0.6), ``absent`` (note <= 0.2 and control >=
0.5), ``partial`` (between the two, to review by hand), ``inconclusive``
(control < 0.5: another edition, or reflowed text), ``no-pdf`` (no source
book in the PDF set).

Usage::

    python3 scripts/footnote-print-check-nested.py CORPUS PDF_DIR CACHE_DIR \\
        --out checks.tsv

``CACHE_DIR`` holds the extracted text. Keep it out of the repository.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import re
import subprocess
import sys
import tomllib
from pathlib import Path

FOOTNOTE_HEADING = re.compile(r"^######\s+(.*?)\s*\{#(footnote-[^}]+)\}\s*$")
HEADING = re.compile(r"^#{1,6}\s")
SKIP_FILES = re.compile(r"artwork|poster map|map folio|pawns|cards", re.I)


def squash(title: str) -> str:
    title = title.lower().replace("&", "and")
    title = re.sub(r"\(.*?\)", "", title)
    title = re.sub(r"^[a-z]{2,4}\d[\w-]*\s+", "", title)  # a leading product code
    title = re.sub(r"\b(the|a|an|pathfinder|campaign setting|player companion|"
                   r"roleplaying game|rpg)\b", "", title)
    return re.sub(r"[^a-z0-9]", "", title)


def norm(text: str) -> list[str]:
    text = text.replace("ﬁ", "fi").replace("ﬂ", "fl").replace("ﬀ", "ff")
    text = re.sub(r"-\n(?=[a-z])", "", text)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = text.lower().replace("’", "'").replace("‘", "'")
    text = re.sub(r"[^a-z0-9]+", " ", text)
    return text.split()


def shingles(words: list[str], n: int = 4) -> set[tuple[str, ...]]:
    return {tuple(words[i:i + n]) for i in range(len(words) - n + 1)}


class Library:
    def __init__(self, pdf_dir: Path, cache: Path):
        self.cache = cache
        cache.mkdir(parents=True, exist_ok=True)
        self.by_key: dict[str, Path] = {}
        for pdf in sorted(pdf_dir.rglob("*.pdf")):
            if SKIP_FILES.search(pdf.name):
                continue
            self.by_key.setdefault(squash(pdf.stem), pdf)
        self.loaded: dict[Path, tuple[str, set]] = {}

    def find(self, title: str) -> Path | None:
        key = squash(title)
        if key in self.by_key:
            return self.by_key[key]
        # "Pathfinder #44: Trial of the Beast" and similar: match the subtitle.
        if ":" in title:
            sub = squash(title.split(":", 1)[1])
            if sub and sub in self.by_key:
                return self.by_key[sub]
        return None

    def text(self, pdf: Path) -> tuple[str, set]:
        if pdf not in self.loaded:
            out = self.cache / (hashlib.sha256(str(pdf).encode()).hexdigest()[:20] + ".txt")
            if not out.exists():
                subprocess.run(["pdftotext", "-q", str(pdf), str(out)], check=False)
            raw = out.read_text(encoding="utf-8", errors="replace") if out.exists() else ""
            words = norm(raw)
            self.loaded[pdf] = (" " + " ".join(words) + " ", shingles(words))
        return self.loaded[pdf]


def score(words: list[str], books: list[tuple[str, set]]) -> float:
    if not words:
        return 0.0
    if len(words) < 4:
        needle = " " + " ".join(words) + " "
        return 1.0 if any(needle in flat for flat, _ in books) else 0.0
    sh = shingles(words)
    found = sum(1 for s in sh if any(s in bs for _, bs in books))
    return found / len(sh)


def split_entry(body: str):
    lines = body.split("\n")
    notes, keep, i = [], [], 0
    in_block = False
    while i < len(lines):
        line = lines[i]
        if line.startswith(":::"):
            in_block = not in_block if line.strip() == ":::" or line.startswith("::: ") else in_block
            i += 1
            continue
        m = FOOTNOTE_HEADING.match(line)
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
            notes.append((m.group(2), " ".join(para)))
            i = j
            continue
        if not in_block and not HEADING.match(line) and not line.startswith("|"):
            keep.append(line)
        i += 1
    return notes, "\n".join(keep)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("corpus", type=Path)
    ap.add_argument("pdf_dir", type=Path)
    ap.add_argument("cache", type=Path)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    pubs = tomllib.loads((args.corpus / "publications.toml").read_text(encoding="utf-8"))
    title = {p["id"]: p["title"] for p in pubs["publication"]}
    lib = Library(args.pdf_dir, args.cache)

    rows = []
    for path in sorted((args.corpus / "entries").rglob("*.md")):
        text = path.read_text(encoding="utf-8")
        if "{#footnote-" not in text or not text.startswith("+++\n"):
            continue
        end = text.index("\n+++\n", 4)
        front = tomllib.loads(text[4:end])
        notes, rest = split_entry(text[end + 5:])
        pub_ids = [s.get("publication") for s in front.get("sources", [])] or front.get("source_ids", [])
        found = [(title.get(p, p), lib.find(title.get(p, p))) for p in pub_ids]
        pdfs = [(t, f) for t, f in found if f]
        books = [lib.text(f) for _, f in pdfs]
        control = score(norm(rest)[:400], books) if books else 0.0
        for seq, (anchor, note) in enumerate(notes, 1):
            words = norm(note)
            s = score(words, books) if books else 0.0
            if not books:
                verdict = "no-pdf"
            elif s >= 0.6:
                verdict = "printed"
            elif control < 0.5:
                verdict = "inconclusive"
            elif s <= 0.2:
                verdict = "absent"
            else:
                verdict = "partial"
            rows.append({
                "entry_id": front["id"],
                "seq": str(seq),
                "anchor": anchor,
                "books": "; ".join(t for t, _ in pdfs),
                "note_score": f"{s:.2f}",
                "control_score": f"{control:.2f}",
                "auto_verdict": verdict,
            })

    with args.out.open("w", encoding="utf-8", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]), delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    from collections import Counter
    print(Counter(r["auto_verdict"] for r in rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
