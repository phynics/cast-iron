#!/usr/bin/env python3
"""Dry run for issue #425: derive page-order books from `[[sources]]` and compare with books.toml.

Read-only. Stdlib only (Python 3.11+ for tomllib). It reads the single `books.toml` of corpus
formats 1 and 2, so run it on a corpus before `cargo xtask corpus-migrate-layout`. The compiler
now derives these books itself (`docs/corpus-format.md`, "Books (format 3)"); this script is the
independent check of that derivation. Rerun it on any checkout of the corpus:

    python3 -I scripts/page-order-derive.py <corpus-dir> [--out DIR]

It reads `publications.toml`, `books.toml` and every `entries/**/*.md`, and reports:

  1. a classification of every book:
       (a) hand-curated printed structure: a top-level chapter other than `Page order`,
           `Unpaged` or the catch-all `New entries (page order)`;
       (b) page-order list only: every top-level chapter is `Page order` or `Unpaged`;
       (c) anything else (no chapter at all, or only a catch-all);
  2. for every (b) book, the list that the proposed derivation would build, compared with the
     list in books.toml: membership, chapter (`Page order` against `Unpaged`), order, and nesting;
  3. the same derivation for the catch-all chapter of every (a) book;
  4. the size of books.toml by group.

The derivation (policy P):

  * Members of book B: every entry whose `source_ids` name B. (`corpus-check` already requires
    this for every placement, so a placement outside it cannot exist.)
  * Page of an entry in B: the first `[[sources]]` row for B that has a `page`, read as its
    leading integer (`12-14` and `12, 14` both give 12). No row for B, or no page: no page.
  * Chapter: `Page order` when the entry has a page, else `Unpaged`.
  * Order inside a chapter: page, then the tie-break, then the id. An entry with no page sorts
    after the paged ones by the tie-break alone.
  * Nesting: an entry whose `parent` chain reaches another member of B is nested under the
    nearest such member (its host), in the host's chapter, in the same order.

Output is deterministic: sorted, no timestamps. With `--out`, it writes `report.txt` and
`dry-run.json` (every difference, by book).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from collections import Counter, defaultdict
from pathlib import Path

CATCH_ALL = "New entries (page order)"
LISTING = ("Page order", "Unpaged")
TIEBREAKS = {
    "kind-title": lambda e: (e["kind"], e["title"].casefold(), e["id"]),
    "title": lambda e: (e["title"].casefold(), e["id"]),
    "id": lambda e: (e["id"],),
}
DEFAULT_TIEBREAK = "kind-title"
CLEAN_PAGE = re.compile(r"^\d+(-\d+)?$")


# ---------------------------------------------------------------- load


def load_entries(root: Path) -> dict[str, dict]:
    """Front matter of every entry: the keys the dry run needs."""
    entries: dict[str, dict] = {}
    for path in sorted((root / "entries").rglob("*.md")):
        text = path.read_text(encoding="utf-8")
        assert text.startswith("+++\n"), f"{path}: entry must start with +++"
        meta = tomllib.loads(text[4 : text.index("\n+++\n", 4)])
        entries[meta["id"]] = {
            "id": meta["id"],
            "kind": meta["kind"],
            "title": meta["title"],
            "source_ids": list(meta.get("source_ids", [])),
            "sources": list(meta.get("sources", [])),
            "parent": meta.get("parent"),
        }
    return entries


def book_blocks(text: str) -> dict[str, int]:
    """Bytes of each `[[books]]` block of books.toml, keyed by `source`."""
    starts = [m.start() for m in re.finditer(r"^\[\[books\]\]$", text, re.M)]
    sizes: dict[str, int] = {}
    for i, start in enumerate(starts):
        end = starts[i + 1] if i + 1 < len(starts) else len(text)
        block = text[start:end]
        source = re.search(r'^source = "([^"]+)"', block, re.M).group(1)
        sizes[source] = len(block.encode("utf-8"))
    return sizes


def catch_block_bytes(text: str, source: str) -> int:
    """Bytes of the catch-all chapter of book `source`: it is the last chapter of the block."""
    start = text.index(f'source = "{source}"')
    end = text.find("\n[[books]]\n", start)
    block = text[start : end if end != -1 else len(text)]
    marker = f'title = "{CATCH_ALL}"'
    at = block.find(marker)
    if at == -1:
        return 0
    head = block.rfind("\n[[books.chapters]]", 0, at)
    return len(block[head + 1 :].encode("utf-8"))


def walk(node: dict, host: str | None, chapter: str, out: list[dict]) -> None:
    """Preorder list of the entry nodes below `node`, each with its host entry."""
    entry = node.get("entry")
    if entry:
        out.append({"entry": entry, "host": host, "chapter": chapter})
    for child in node.get("children", []):
        walk(child, entry or host, chapter, out)


def current_list(book: dict, only: tuple[str, ...]) -> list[dict]:
    """Placements of the top-level chapters named in `only`, in book order."""
    out: list[dict] = []
    for chapter in book["chapters"]:
        if chapter["title"] in only:
            walk(chapter, None, chapter["title"], out)
    return out


def classify(book: dict) -> tuple[str, str]:
    """('a'|'b'|'c', signature of the top-level chapter titles)."""
    tops = [c["title"] for c in book.get("chapters", [])]
    real = [t for t in tops if t not in LISTING and t != CATCH_ALL]
    if real:
        return "a", "curated"
    listing = [t for t in tops if t in LISTING]
    if listing and CATCH_ALL not in tops:
        return "b", " + ".join(listing)
    return "c", " + ".join(tops) or "no chapter"


# ---------------------------------------------------------------- pages


def lead_page(page: str | None) -> int | None:
    match = re.match(r"\s*(\d+)", page or "")
    return int(match.group(1)) if match else None


def book_pages(entry: dict, book: str) -> list[str | None]:
    """The `page` of every `[[sources]]` row of `entry` for `book`, in row order."""
    return [row.get("page") for row in entry["sources"] if row["publication"] == book]


def page_of(entry: dict, book: str) -> int | None:
    for page in book_pages(entry, book):
        number = lead_page(page)
        if number is not None:
            return number
    return None


def page_flags(entry: dict, book: str) -> list[str]:
    """Why an entry's page in a book is not a plain number, for the report."""
    rows = book_pages(entry, book)
    flags = []
    if not rows:
        flags.append("no row")
    elif all(p is None for p in rows):
        flags.append("row without page")
    if len(rows) > 1:
        flags.append("several rows")
    for page in rows:
        if page is None:
            continue
        if "," in page:
            flags.append("page list")
        elif not CLEAN_PAGE.match(page):
            flags.append("malformed page")
    return sorted(set(flags))


# ---------------------------------------------------------------- derive


def derive(book: str, members: set[str], entries: dict[str, dict], tiebreak: str) -> list[dict]:
    """The derived list of `book`: preorder placements with host and chapter."""
    key = TIEBREAKS[tiebreak]

    def host_of(entry_id: str) -> str | None:
        seen = set()
        parent = entries[entry_id]["parent"]
        while parent and parent in entries and parent not in seen:
            if parent in members:
                return parent
            seen.add(parent)
            parent = entries[parent]["parent"]
        return None

    children: dict[str | None, list[str]] = defaultdict(list)
    for entry_id in members:
        children[host_of(entry_id)].append(entry_id)

    def order(ids: list[str]) -> list[str]:
        return sorted(
            ids,
            key=lambda i: (
                page_of(entries[i], book) is None,
                page_of(entries[i], book) or 0,
                key(entries[i]),
            ),
        )

    out: list[dict] = []

    def emit(entry_id: str, host: str | None, chapter: str) -> None:
        out.append({"entry": entry_id, "host": host, "chapter": chapter})
        for child in order(children.get(entry_id, [])):
            emit(child, entry_id, chapter)

    for root in order(children.get(None, [])):
        chapter = "Page order" if page_of(entries[root], book) is not None else "Unpaged"
        emit(root, None, chapter)
    # Roots were sorted paged-first, so the preorder is Page order then Unpaged already.
    return out


def lis_length(seq: list[int]) -> int:
    """Length of the longest increasing subsequence."""
    from bisect import bisect_left

    tails: list[int] = []
    for value in seq:
        i = bisect_left(tails, value)
        if i == len(tails):
            tails.append(value)
        else:
            tails[i] = value
    return len(tails)


def compare(book: str, cur: list[dict], der: list[dict], entries: dict[str, dict], tiebreak: str) -> dict:
    """Differences between the current and the derived list of one book."""
    cur_by = {p["entry"]: p for p in cur}
    der_by = {p["entry"]: p for p in der}
    only_cur = sorted(set(cur_by) - set(der_by))
    only_der = sorted(set(der_by) - set(cur_by))
    common = [e for e in (p["entry"] for p in cur) if e in der_by]

    chapter_moves = [
        {
            "entry": e,
            "current": cur_by[e]["chapter"],
            "derived": der_by[e]["chapter"],
            "flags": page_flags(entries[e], book),
        }
        for e in common
        if cur_by[e]["chapter"] != der_by[e]["chapter"]
    ]
    nest_diffs = [
        {"entry": e, "current_host": cur_by[e]["host"], "derived_host": der_by[e]["host"]}
        for e in common
        if cur_by[e]["host"] != der_by[e]["host"]
    ]

    # Order: compare the entries that agree on chapter and host, group by (chapter, host).
    stable = [
        e
        for e in common
        if cur_by[e]["chapter"] == der_by[e]["chapter"] and cur_by[e]["host"] == der_by[e]["host"]
    ]
    groups: dict[tuple[str, str | None], list[str]] = defaultdict(list)
    for e in stable:
        groups[(cur_by[e]["chapter"], cur_by[e]["host"])].append(e)
    der_pos = {p["entry"]: i for i, p in enumerate(der)}
    moved = 0
    inversions: list[dict] = []
    for (chapter, host), ids in sorted(groups.items(), key=lambda kv: (kv[0][0], kv[0][1] or "")):
        positions = [der_pos[e] for e in ids]
        moved += len(ids) - lis_length(positions)
        for x, y in zip(ids, ids[1:]):
            if der_pos[x] > der_pos[y]:
                px, py = page_of(entries[x], book), page_of(entries[y], book)
                flags = page_flags(entries[x], book) + page_flags(entries[y], book)
                if px is not None and px == py:
                    reason = "same page"
                elif px is None and py is None:
                    reason = "no page on both"
                else:
                    reason = "different pages"
                inversions.append(
                    {
                        "before": x,
                        "after": y,
                        "reason": reason,
                        "pages": [px, py],
                        "flags": sorted(set(flags)),
                    }
                )
    exact = not (only_cur or only_der or chapter_moves or nest_diffs or inversions)
    return {
        "book": book,
        "tiebreak": tiebreak,
        "current": len(cur),
        "derived": len(der),
        "only_current": only_cur,
        "only_derived": only_der,
        "chapter_moves": chapter_moves,
        "nest_diffs": nest_diffs,
        "moved": moved,
        "inversions": inversions,
        "exact": exact,
        "exact_set": not (only_cur or only_der),
    }


def pct(n: int, d: int) -> str:
    return f"{100 * n / d:.1f}%" if d else "n/a"


# ---------------------------------------------------------------- main


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("corpus", type=Path)
    ap.add_argument("--out", type=Path, help="write report.txt and dry-run.json here")
    ap.add_argument("--tiebreak", choices=sorted(TIEBREAKS), default=DEFAULT_TIEBREAK)
    ap.add_argument("--limit", type=int, default=15, help="rows in each top-N table")
    args = ap.parse_args()
    root: Path = args.corpus

    pubs = {
        p["id"]: p
        for p in tomllib.loads((root / "publications.toml").read_text(encoding="utf-8"))["publication"]
    }
    books_text = (root / "books.toml").read_text(encoding="utf-8")
    books = tomllib.loads(books_text)["books"]
    sizes = book_blocks(books_text)
    entries = load_entries(root)
    members_of: dict[str, set[str]] = defaultdict(set)
    for e in entries.values():
        for source in e["source_ids"]:
            members_of[source].add(e["id"])

    lines: list[str] = []
    out = lines.append
    result: dict = {"tiebreak": args.tiebreak}

    # ------------------------------------------------ 1. classification
    groups: dict[str, list[dict]] = defaultdict(list)
    signature: Counter = Counter()
    for book in books:
        kind, sig = classify(book)
        groups[kind].append(book)
        signature[(kind, sig)] += 1
    out("# 1. Classification")
    out(f"books in books.toml: {len(books)}; publications: {len(pubs)}; entries: {len(entries)}")
    for kind in "abc":
        out(f"({kind}): {len(groups[kind])}")
    out("")
    out("| group | top-level chapters | books |")
    out("|---|---|---:|")
    for (kind, sig), n in sorted(signature.items()):
        out(f"| ({kind}) | {sig} | {n} |")
    out("")
    out("(a) books, with their top-level chapter count, catch-all rows and size in books.toml:")
    out("| book | chapters | placements | in catch-all | books.toml bytes | catch-all bytes |")
    out("|---|---:|---:|---:|---:|---:|")
    a_rows = []
    catch_bytes_total = 0
    for book in groups["a"]:
        placements: list[dict] = []
        for chapter in book["chapters"]:
            walk(chapter, None, chapter["title"], placements)
        catch = sum(1 for p in placements if p["chapter"] == CATCH_ALL)
        real_top = [c for c in book["chapters"] if c["title"] not in LISTING and c["title"] != CATCH_ALL]
        catch_bytes = catch_block_bytes(books_text, book["source"])
        catch_bytes_total += catch_bytes
        a_rows.append((book["title"], len(real_top), len(placements), catch, sizes[book["source"]]))
        out(f"| {book['title']} | {len(real_top)} | {len(placements)} | {catch} | {sizes[book['source']]:,} | {catch_bytes:,} |")
    out("")
    out("(c) books:")
    for book in groups["c"]:
        out(f"  - {book['title']} ({[c['title'] for c in book.get('chapters', [])]})")
    total_bytes = sum(sizes.values())
    b_bytes = sum(sizes[b["source"]] for b in groups["b"])
    a_bytes = sum(sizes[b["source"]] for b in groups["a"])
    out("")
    out(f"books.toml bytes: total {total_bytes:,}; (a) {a_bytes:,} ({pct(a_bytes, total_bytes)}); "
        f"(b) {b_bytes:,} ({pct(b_bytes, total_bytes)})")
    out(f"catch-all chapters of the (a) books: {catch_bytes_total:,} bytes; "
        f"curated trees without them: {a_bytes - catch_bytes_total:,} bytes "
        f"({pct(a_bytes - catch_bytes_total, total_bytes)} of today's file)")
    result["classification"] = {k: [b["source"] for b in v] for k, v in sorted(groups.items())}
    result["sizes"] = {"total": total_bytes, "a": a_bytes, "b": b_bytes}

    # ------------------------------------------------ 2. tie-break evidence
    # Within every run of top-level `Page order` entries that share a page, which key reproduces
    # the current order?
    out("")
    out("# 2. Tie-break evidence (same-page runs of the current `Page order` chapters)")
    runs = 0
    hits: Counter = Counter()
    bad_runs: list[str] = []
    for book in groups["b"]:
        src = book["source"]
        top = [p for p in current_list(book, ("Page order",)) if p["host"] is None]
        i = 0
        while i < len(top):
            page = page_of(entries[top[i]["entry"]], src)
            j = i
            while j < len(top) and page_of(entries[top[j]["entry"]], src) == page:
                j += 1
            if page is not None and j - i > 1:
                runs += 1
                run = [top[k]["entry"] for k in range(i, j)]
                for name, fn in TIEBREAKS.items():
                    if run == sorted(run, key=lambda e: fn(entries[e])):
                        hits[name] += 1
            i = j
    out(f"runs of two or more entries on one page: {runs}")
    for name in sorted(TIEBREAKS):
        out(f"  {name}: reproduces {hits[name]} runs ({pct(hits[name], runs)})")
    result["tiebreak_runs"] = {"runs": runs, **dict(hits)}

    # ------------------------------------------------ 3. dry run for (b)
    out("")
    out(f"# 3. Dry run for the (b) books (tie-break `{args.tiebreak}`)")
    comparisons = []
    for book in groups["b"]:
        src = book["source"]
        cur = current_list(book, LISTING)
        members = members_of.get(src, set())
        der = derive(src, members, entries, args.tiebreak)
        cmp = compare(src, cur, der, entries, args.tiebreak)
        cmp["title"] = book["title"]
        cmp["kind"] = classify(book)[1]
        comparisons.append(cmp)
    n = len(comparisons)
    exact = [c for c in comparisons if c["exact"]]
    exact_set = [c for c in comparisons if c["exact_set"]]
    out(f"books: {n}")
    out(f"exact match (same entries, chapters, order and nesting): {len(exact)} ({pct(len(exact), n)})")
    out(f"same entry set (differences may remain in order, chapter, nesting): {len(exact_set)}")
    cur_total = sum(c["current"] for c in comparisons)
    der_total = sum(c["derived"] for c in comparisons)
    out(f"placements: current {cur_total:,}; derived {der_total:,}")
    only_cur = sum(len(c["only_current"]) for c in comparisons)
    only_der = sum(len(c["only_derived"]) for c in comparisons)
    out(f"entries only in the current list: {only_cur:,}; only in the derived list: {only_der:,}")
    moves = [m for c in comparisons for m in c["chapter_moves"]]
    out(f"chapter moves (same entry, other chapter): {len(moves):,} "
        f"({Counter((m['current'], m['derived']) for m in moves)})")
    nests = [m for c in comparisons for m in c["nest_diffs"]]
    out(f"nesting differences: {len(nests):,}")
    inv = [m for c in comparisons for m in c["inversions"]]
    out(f"adjacent order inversions: {len(inv):,} by reason {dict(Counter(i['reason'] for i in inv))}")
    out(f"entries out of place in the order (n minus longest increasing run): "
        f"{sum(c['moved'] for c in comparisons):,}")

    out("")
    out("Why the sets differ")
    only_cur_reasons: Counter = Counter()
    only_der_reasons: Counter = Counter()
    for c in comparisons:
        for e in c["only_current"]:
            only_cur_reasons["names the book in source_ids" if c["book"] in entries[e]["source_ids"]
                             else "does not name the book in source_ids"] += 1
        for e in c["only_derived"]:
            ent = entries[e]
            flags = page_flags(ent, c["book"])
            only_der_reasons[(ent["kind"], "has a page" if page_of(ent, c["book"]) else "no page")] += 1
    out(f"  only current: {dict(only_cur_reasons) or 'none'}")
    out("  only derived, by kind and page: " + ", ".join(f"{k}: {v}" for k, v in sorted(only_der_reasons.items())))

    out("")
    out("Chapter moves by reason (current chapter -> derived chapter)")
    reason_counts: Counter = Counter()
    for m in moves:
        reason_counts[(m["current"], m["derived"], tuple(m["flags"]))] += 1
    for (cur_ch, der_ch, flags), count in sorted(reason_counts.items(), key=lambda kv: (-kv[1], kv[0])):
        out(f"  {count:6,}  {cur_ch} -> {der_ch}: {', '.join(flags) or 'has a page'}")

    out("")
    out("Order inversions by reason and page flags")
    inv_counts: Counter = Counter()
    for i in inv:
        inv_counts[(i["reason"], tuple(i["flags"]))] += 1
    for (reason, flags), count in sorted(inv_counts.items(), key=lambda kv: (-kv[1], kv[0])):
        out(f"  {count:6,}  {reason}: {', '.join(flags) or 'plain pages'}")

    out("")
    out("Nesting differences")
    nest_counts: Counter = Counter()
    for m in nests:
        nest_counts[("host only in current" if m["derived_host"] is None
                     else "host only in derived" if m["current_host"] is None
                     else "other host")] += 1
    out(f"  {dict(nest_counts) or 'none'}")
    # How often does `parent` explain the current nesting?
    nested_cur = 0
    nested_by_parent = 0
    for book in groups["b"]:
        for p in current_list(book, LISTING):
            if p["host"]:
                nested_cur += 1
                nested_by_parent += entries[p["entry"]]["parent"] == p["host"]
    out(f"  current nested placements: {nested_cur:,}; host is the entry's `parent`: {nested_by_parent:,}")

    out("")
    out("Entries with no page in their book, by cause")
    cause: Counter = Counter()
    for book in groups["b"]:
        for e in members_of.get(book["source"], ()):
            if page_of(entries[e], book["source"]) is None:
                cause[tuple(page_flags(entries[e], book["source"]))] += 1
    for flags, count in sorted(cause.items(), key=lambda kv: (-kv[1], kv[0])):
        out(f"  {count:6,}  {', '.join(flags)}")
    out("")
    out("Several pages (rows or lists) for one book, among members of (b) books")
    several: Counter = Counter()
    for book in groups["b"]:
        for e in members_of.get(book["source"], ()):
            flags = page_flags(entries[e], book["source"])
            for flag in ("several rows", "page list", "malformed page"):
                if flag in flags:
                    several[flag] += 1
    out(f"  {dict(sorted(several.items()))}")

    # How far does books.toml still tell the page of an entry that has no row? The nearest paged
    # neighbours in the current `Page order` list bracket it; equal neighbours fix it exactly.
    out("")
    out("Entries that move from `Page order` to `Unpaged` (no row for the book): can the neighbours fix the page?")
    bracket: Counter = Counter()
    bracket_books: Counter = Counter()
    bracket_detail: list[dict] = []
    by_title = {c["book"]: c["title"] for c in comparisons}
    for book in groups["b"]:
        src = book["source"]
        moved_ids = {m["entry"] for c in comparisons if c["book"] == src for m in c["chapter_moves"]}
        if not moved_ids:
            continue
        top = [p for p in current_list(book, ("Page order",)) if p["host"] is None]
        pages = [page_of(entries[p["entry"]], src) for p in top]
        for i, p in enumerate(top):
            if p["entry"] not in moved_ids:
                continue
            lo = next((q for q in reversed(pages[:i]) if q is not None), None)
            hi = next((q for q in pages[i + 1 :] if q is not None), None)
            if lo is not None and lo == hi:
                how = "exact: neighbours share one page"
            elif lo is not None and hi is not None:
                how = "range: neighbours differ"
            else:
                how = "open: no neighbour on one side"
            bracket[how] += 1
            bracket_books[src] += 1
            bracket_detail.append({"book": src, "entry": p["entry"], "low": lo, "high": hi, "how": how})
    for how, count in sorted(bracket.items()):
        out(f"  {count:6,}  top-level entries, {how}")
    nested_moves = sum(len(c["chapter_moves"]) for c in comparisons) - sum(bracket.values())
    out(f"  {nested_moves:6,}  nested entries (they follow their host, which is counted above or not paged)")
    out(f"  books affected: {len(bracket_books)}; largest: "
        + ", ".join(f"{by_title[b]} {n}" for b, n in bracket_books.most_common(8)))
    result["bracket"] = bracket_detail

    out("")
    out("Books by what differs")
    categories: Counter = Counter()
    for c in comparisons:
        kinds = []
        if c["only_current"] or c["only_derived"]:
            kinds.append("different entry set")
        if c["chapter_moves"]:
            kinds.append("page missing from [[sources]]")
        if c["nest_diffs"]:
            kinds.append("nesting")
        if any(i["reason"] == "different pages" for i in c["inversions"]):
            kinds.append("entry placed away from its own page")
        if any(i["reason"] == "same page" for i in c["inversions"]):
            kinds.append("tie order on one page")
        if any(i["reason"] == "no page on both" for i in c["inversions"]):
            kinds.append("Unpaged order")
        categories[tuple(kinds) or ("exact",)] += 1
    for kinds, count in sorted(categories.items(), key=lambda kv: (-kv[1], kv[0])):
        out(f"  {count:4}  {' + '.join(kinds)}")

    # Chapter routes are slugs of the chapter title, so a chapter that appears or vanishes changes
    # the set of addresses of a book.
    out("")
    out("Chapters that appear or vanish (a chapter address is the slug of its title)")
    shape: Counter = Counter()
    shape_books: dict[str, list[str]] = defaultdict(list)
    for book, c in zip(groups["b"], comparisons):
        cur_chapters = sorted({p["chapter"] for p in current_list(book, LISTING)})
        der_src = derive(book["source"], members_of.get(book["source"], set()), entries, args.tiebreak)
        der_chapters = sorted({p["chapter"] for p in der_src})
        if cur_chapters != der_chapters:
            change = f"{' + '.join(cur_chapters) or 'none'} -> {' + '.join(der_chapters) or 'none'}"
            shape[change] += 1
            shape_books[change].append(book["title"])
    for change, count in sorted(shape.items(), key=lambda kv: (-kv[1], kv[0])):
        out(f"  {count:4}  {change}")
    result["chapter_shape"] = {k: sorted(v) for k, v in shape_books.items()}

    # Node titles: a node carries the entry title, except where the corpus authored another.
    out("")
    out("Node titles that differ from the entry title (books.toml node title, entry title)")
    title_diffs = []
    group_nodes = []

    def scan(node: dict, book: str) -> None:
        for child in node.get("children", []):
            entry = child.get("entry")
            if entry and child["title"] != entries[entry]["title"]:
                kinds = sorted({r["kind"] for r in entries[entry].get("relations", [])})
                title_diffs.append({"book": book, "entry": entry, "node": child["title"],
                                    "entry_title": entries[entry]["title"]})
            if not entry:
                group_nodes.append({"book": book, "node": child["title"], "children": len(child.get("children", []))})
            scan(child, book)

    for book in groups["b"]:
        for chapter in book["chapters"]:
            scan(chapter, book["title"])
    out(f"  node title differs from entry title: {len(title_diffs)} ({dict(Counter(t['book'] for t in title_diffs))})")
    out(f"  grouping nodes (a node with no entry) below a listing chapter: {len(group_nodes)} "
        f"{[(g['book'], g['node']) for g in group_nodes]}")
    result["title_diffs"] = title_diffs
    result["group_nodes"] = group_nodes

    # Contents leaves out every kind that has its own list page (schema `hidden_from_contents`), so a
    # reader sees only the `rule` rows of a page-order book. Compare the rule rows alone.
    out("")
    out("What Contents shows: the rule rows only (every other kind is left out of Contents)")
    vis_books = 0
    vis_rows = 0
    vis_moves = 0
    vis_moved_books: Counter = Counter()
    vis_order_books = 0
    vis_order_entries = 0
    vis_new_chapters = 0
    vis_lost_chapters = 0
    rule_rows_now = 0
    rule_rows_derived = 0
    visible_detail = []
    for book, c in zip(groups["b"], comparisons):
        src = book["source"]
        cur = [dict(p, host=None) for p in current_list(book, LISTING) if entries[p["entry"]]["kind"] == "rule"]
        der = [dict(p, host=None) for p in derive(src, members_of.get(src, set()), entries, args.tiebreak)
               if entries[p["entry"]]["kind"] == "rule"]
        rule_rows_now += len(cur)
        rule_rows_derived += len(der)
        if not cur and not der:
            continue
        vis_books += 1
        vis_rows += len(cur)
        cmp = compare(src, cur, der, entries, args.tiebreak)
        vis_moves += len(cmp["chapter_moves"])
        if cmp["chapter_moves"]:
            vis_moved_books[book["title"]] = len(cmp["chapter_moves"])
        vis_order_entries += cmp["moved"]
        vis_order_books += bool(cmp["moved"] or cmp["inversions"])
        now_chapters = {p["chapter"] for p in cur}
        new_chapters = {p["chapter"] for p in der}
        vis_new_chapters += len(new_chapters - now_chapters)
        vis_lost_chapters += len(now_chapters - new_chapters)
        visible_detail.append({"book": src, "rules_now": len(cur), "rules_derived": len(der),
                               "chapter_moves": len(cmp["chapter_moves"]), "order_moved": cmp["moved"],
                               "only_current": len(cmp["only_current"]), "only_derived": len(cmp["only_derived"])})
    out(f"  (b) books with at least one rule row: {vis_books}")
    out(f"  rule rows: current {rule_rows_now:,}; derived {rule_rows_derived:,}")
    out(f"  rule rows that change chapter (Page order -> Unpaged): {vis_moves:,} in {len(vis_moved_books)} books; "
        f"largest: " + ", ".join(f"{t} {n}" for t, n in vis_moved_books.most_common(6)))
    out(f"  books whose rule rows change order: {vis_order_books}; rule rows out of place: {vis_order_entries:,}")
    out(f"  chapter rows a book gains / loses in Contents (only chapters that hold a rule are listed): "
        f"{vis_new_chapters} / {vis_lost_chapters}")
    result["visible"] = visible_detail

    out("")
    out(f"Books by number of differences (top {args.limit})")
    out("| book | current | derived | only current | only derived | chapter moves | nesting | moved | exact |")
    out("|---|---:|---:|---:|---:|---:|---:|---:|---|")
    ranked = sorted(
        comparisons,
        key=lambda c: -(len(c["only_current"]) + len(c["only_derived"]) + len(c["chapter_moves"])
                        + len(c["nest_diffs"]) + c["moved"]),
    )
    for c in ranked[: args.limit]:
        out(f"| {c['title']} | {c['current']} | {c['derived']} | {len(c['only_current'])} | "
            f"{len(c['only_derived'])} | {len(c['chapter_moves'])} | {len(c['nest_diffs'])} | "
            f"{c['moved']} | {'yes' if c['exact'] else 'no'} |")

    out("")
    out("Every (b) book")
    out("| book | shape | current | derived | only cur | only der | ch. moves | nest | moved | exact |")
    out("|---|---|---:|---:|---:|---:|---:|---:|---:|---|")
    for c in sorted(comparisons, key=lambda c: c["title"]):
        out(f"| {c['title']} | {c['kind']} | {c['current']} | {c['derived']} | {len(c['only_current'])} | "
            f"{len(c['only_derived'])} | {len(c['chapter_moves'])} | {len(c['nest_diffs'])} | "
            f"{c['moved']} | {'yes' if c['exact'] else 'no'} |")
    result["b"] = comparisons

    # ------------------------------------------------ 4. catch-all of the (a) books
    out("")
    out("# 4. Catch-all chapter of the (a) books (derive the unplaced members, sorted by page)")
    out("| book | catch-all now | unplaced members | only now | only derived | order moved |")
    out("|---|---:|---:|---:|---:|---:|")
    catch_rows = []
    for book in groups["a"]:
        src = book["source"]
        placements: list[dict] = []
        for chapter in book["chapters"]:
            walk(chapter, None, chapter["title"], placements)
        placed = {p["entry"] for p in placements if p["chapter"] != CATCH_ALL}
        cur = [p for p in placements if p["chapter"] == CATCH_ALL]
        members = members_of.get(src, set()) - placed
        der = derive(src, members, entries, args.tiebreak)
        # the catch-all is one chapter, so compare by entry order only
        cur_ids = [p["entry"] for p in cur]
        der_ids = [p["entry"] for p in der]
        only_c = sorted(set(cur_ids) - set(der_ids))
        only_d = sorted(set(der_ids) - set(cur_ids))
        common = [e for e in cur_ids if e in set(der_ids)]
        pos = {e: i for i, e in enumerate(der_ids)}
        moved = len(common) - lis_length([pos[e] for e in common])
        catch_rows.append(
            {"book": src, "current": len(cur_ids), "derived": len(der_ids), "only_current": only_c,
             "only_derived": only_d, "moved": moved}
        )
        out(f"| {book['title']} | {len(cur_ids)} | {len(der_ids)} | {len(only_c)} | {len(only_d)} | {moved} |")
    result["catch_all"] = catch_rows

    # ------------------------------------------------ 5. book order and tie-break portability
    out("")
    out("# 5. Book order")
    curated = [b["source"] for b in groups["a"]]
    listing_order = [b["source"] for b in books if b["source"] not in set(curated)]
    titles = {b["source"]: b["title"] for b in books}

    def folded(source: str) -> tuple[str, str]:
        return (re.sub(r"[^a-z0-9]", "", titles[source].lower()), source)

    out(f"curated books sit at positions {[i for i, b in enumerate(books) if b['source'] in set(curated)]} of {len(books)}")
    out(f"the {len(listing_order)} other books are in the order of `title` folded to lower case letters and digits, "
        f"then id: {listing_order == sorted(listing_order, key=folded)}")
    where: dict[str, list[tuple[str, str]]] = defaultdict(list)

    def collect(node: dict, source: str, top: str) -> None:
        if node.get("entry"):
            where[node["entry"]].append((source, top))
        for child in node.get("children", []):
            collect(child, source, top)

    for book in books:
        for chapter in book["chapters"]:
            collect(chapter, book["source"], chapter["title"])

    def first_book(order: list[str]) -> dict[str, str]:
        position = {s: i for i, s in enumerate(order)}
        out_map = {}
        for entry, placed in where.items():
            printed = [s for s, top in placed if top not in LISTING and top != CATCH_ALL]
            out_map[entry] = min(printed or [s for s, _ in placed], key=position.__getitem__)
        return out_map

    now = first_book([b["source"] for b in books])
    proposed = sorted(curated, key=folded) + sorted(listing_order, key=folded)
    new = first_book(proposed)
    out(f"entries whose first book (printed chapter first, then listing) changes if the curated books come "
        f"first and each group sorts by that key: {sum(1 for e in now if now[e] != new[e])}")
    everyone = first_book(sorted((b["source"] for b in books), key=folded))
    out(f"... and if all books sorted by that key with no curated-first rule: "
        f"{sum(1 for e in now if now[e] != everyone[e])}")
    odd = [e["id"] for e in entries.values() if e["title"].casefold() != e["title"].lower()]
    out(f"titles where Python casefold and lower case differ (a Rust `to_lowercase` tie-break must agree): {len(odd)}")
    result["book_order"] = {"curated": curated, "listing_in_order_of_key": listing_order == sorted(listing_order, key=folded)}

    text = "\n".join(lines) + "\n"
    if args.out:
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "report.txt").write_text(text, encoding="utf-8")
        (args.out / "dry-run.json").write_text(
            json.dumps(result, indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
