import fs from "node:fs";
import path from "node:path";
import { ROOT, loadCorpus } from "./fixtures";
import { bookTrees, chapterIds, type BookTree, type TocNode } from "./helpers";

/**
 * Cases for the saved positional book addresses (`?a=toc-2-26`, `?c=2.26`) that the library's
 * table `sources/legacy-sections.json` keeps opening (#546, #550).
 *
 * The table is permanent and the books are not: a corpus pin adds and removes rows, so a case
 * named after one book's row (Ranger in Blood of the Beast) goes stale when that book stops
 * printing it. These helpers read the table and the compiled books and derive the cases, so the
 * specs keep proving the same two things whatever the pinned corpus holds.
 */
export interface PublishedAddress {
  /** The book, by its source id. */
  source: string;
  /** The published anchor: `toc-<chapter>-<row>`. */
  anchor: string;
  /** The same position in the Contents form: `<chapter>.<row>`. */
  contents: string;
  /** The permanent id of the chapter page that the address named. */
  chapterId: string;
  /** The stable anchor that the table gives the row, `row-<entry id>`. */
  stable: string;
  /** The title of the row, when the row is still in the book. */
  title?: string;
  /** The row's place in the chapter now, against `published`, its place when it was saved. */
  now?: number;
  published: number;
}

interface LegacyFile {
  books: Record<string, Record<string, string>>;
  chapters: Record<string, Record<string, string>>;
}

/** Mirror of `slugify` for an entry id, as `section_anchors` builds `row-<entry>`. */
const slug = (text: string) => text.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");

function legacyTable(): LegacyFile {
  return JSON.parse(fs.readFileSync(path.join(ROOT, "sources/legacy-sections.json"), "utf8"));
}

/** `toc-2-26` as chapter and row, 1-based; `null` for a row below the second level. */
function twoLevels(anchor: string): [number, number] | null {
  const parts = anchor.match(/^toc-(\d+)-(\d+)$/);
  return parts ? [Number(parts[1]), Number(parts[2])] : null;
}

/** Every row anchor that some node of the book can carry, by `entry` alone: no `--`, no suffix. */
function entryAnchors(book: BookTree): Set<string> {
  const out = new Set<string>();
  const walk = (node: TocNode) => {
    if (node.entry) out.add(`row-${slug(node.entry)}`);
    for (const child of node.children ?? []) walk(child);
  };
  for (const chapter of book.chapters) walk(chapter);
  return out;
}

function occurrences(node: TocNode, entry: string): number {
  return (
    (node.entry === entry ? 1 : 0) +
    (node.children ?? []).reduce((sum, child) => sum + occurrences(child, entry), 0)
  );
}

/** Published two-level addresses in sorted order: book, then address, so a choice is stable. */
function* published(): Generator<{ book: BookTree; anchor: string; stable: string; chapter: number; row: number; chapterId: string; chapterIndex: number }> {
  const table = legacyTable();
  for (const book of [...bookTrees()].sort((a, b) => a.source.localeCompare(b.source))) {
    const rowsOfChapters = chapterIds(book);
    const rows = Object.entries(table.books[book.source] ?? {}).sort(([a], [b]) => a.localeCompare(b, "en", { numeric: true }));
    for (const [anchor, stable] of rows) {
      const levels = twoLevels(anchor);
      const chapterId = levels && table.chapters[book.source]?.[`toc-${levels[0]}`];
      const at = rowsOfChapters.find((candidate) => candidate.id === chapterId);
      if (!levels || !chapterId || !at) continue;
      yield { book, anchor, stable, chapter: levels[0], row: levels[1], chapterId, chapterIndex: at.path[0] };
    }
  }
}

/**
 * A published address whose row is still in its chapter page, in a different place than when it
 * was saved: read by position the address now names another row, so a pass proves that the
 * table remaps it, not that the row stayed where it was. The first such address in book and
 * address order; the row is a rule that Contents always lists, that no heading anchor renames,
 * and that its chapter holds once, so its anchor is `row-<entry id>` as the table gave it.
 */
export function remappedAddress(): PublishedAddress {
  const kinds = new Map<string, string>(loadCorpus().catalog.entries.map((stub: any) => [stub.id, stub.kind]));
  for (const { book, anchor, stable, chapter, row, chapterId, chapterIndex } of published()) {
    const node = book.chapters[chapterIndex];
    const children = node.children ?? [];
    const now = children.findIndex(
      (child) =>
        child.entry &&
        !child.anchor &&
        kinds.get(child.entry) === "rule" &&
        `row-${slug(child.entry)}` === stable &&
        occurrences(node, child.entry) === 1,
    );
    if (now < 0 || (now === row - 1 && chapterIndex === chapter - 1)) continue;
    return {
      source: book.source,
      anchor,
      contents: `${chapter}.${row}`,
      chapterId,
      stable,
      title: children[now].title,
      now,
      published: row - 1,
    };
  }
  throw new Error(
    "no published address in sources/legacy-sections.json names a rule row that moved within its chapter; " +
      "the remap checks need one",
  );
}

/**
 * A published address whose entry row no node of its book carries now, as when a pin leaves the
 * entry out of the book. The first such address in book and address order. The app opens the chapter
 * (or the top of Contents) for it, with no dead anchor and nothing marked.
 */
export function goneAddress(): PublishedAddress {
  const present = new Map<string, Set<string>>();
  for (const { book, anchor, stable, chapter, row, chapterId } of published()) {
    // A heading row with no entry (`row-h-...`) is named by its title, which this does not read.
    if (!stable.startsWith("row-ref-")) continue;
    if (!present.has(book.source)) present.set(book.source, entryAnchors(book));
    const carried = [...present.get(book.source)!].some(
      (base) => stable === base || stable.startsWith(`${base}--`) || new RegExp(`^${base}-\\d+$`).test(stable),
    );
    if (carried) continue;
    return { source: book.source, anchor, contents: `${chapter}.${row}`, chapterId, stable, published: row - 1 };
  }
  throw new Error(
    "every published address in sources/legacy-sections.json still has its row in its book; " +
      "the gone-row checks need one that left",
  );
}
