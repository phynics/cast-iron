import fs from "node:fs";
import path from "node:path";
import type { Locator, Page } from "@playwright/test";
import { GENERATED, installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// The archetype list of a class page and its compatibility filter (#170, part D; decision
// `2026-11-22-archetype-list-and-compatibility`). One row per archetype: name, book and what it
// replaces. `Keep` (features) and `Combine with` (an archetype) narrow the list. Expectations come
// from the compiled corpus: the relations of each archetype and `trades.json`.

const corpus = loadCorpus();
const WIZARD = "ref/class/wizard";
const FIGHTER = "ref/class/fighter";
const WIDTHS = [390, 1280];

interface FeatureRef {
  anchor: string;
  class?: string;
  levels?: number[];
  power?: string;
}
interface TradeRow {
  op: "replaces" | "alters";
  printed: string;
  feature?: FeatureRef;
}
const trades = new Map<string, TradeRow[]>(
  (JSON.parse(fs.readFileSync(path.join(GENERATED, corpus.version, "trades.json"), "utf8")).entries as any[]).map(
    (row) => [row.id, row.trades],
  ),
);

interface Expected {
  id: string;
  title: string;
  book: string;
  meta: string;
  trades: TradeRow[] | undefined;
  checked: boolean;
}

/** The archetypes of a class as the page lists them, from the relations and `trades.json`. */
function archetypesOf(classId: string): Expected[] {
  const sourceTitle = (id: string) => {
    const stub = corpus.catalog.entries.find((candidate) => candidate.id === id)!;
    return corpus.catalog.sources.find((source) => source.id === stub.source)!.title as string;
  };
  return [...corpus.entries.values()]
    .filter(
      (entry) =>
        entry.kind === "archetype" &&
        (entry.relations ?? []).some((edge: any) => edge.kind === "archetype" && edge.target === classId),
    )
    .map((entry) => {
      const own = trades.get(entry.id);
      const line = (op: "replaces" | "alters") => {
        const names = (own ?? []).filter((trade) => trade.op === op).map((trade) => trade.printed);
        return names.length ? `${op === "replaces" ? "Replaces" : "Alters"} ${names.join(", ")}` : null;
      };
      return {
        id: entry.id,
        title: entry.title,
        book: sourceTitle(entry.id),
        meta: [line("replaces"), line("alters")].filter(Boolean).join(" · "),
        trades: own,
        checked: own !== undefined && own.every((trade) => trade.feature !== undefined),
      };
    });
}

const overlaps = (a: FeatureRef, b: FeatureRef) =>
  a.anchor === b.anchor &&
  !(a.class && b.class && a.class !== b.class) &&
  (!a.levels?.length || !b.levels?.length || a.levels.some((level) => b.levels!.includes(level))) &&
  (a.power == null || b.power == null || a.power === b.power);

const resolved = (row: Expected) => (row.trades ?? []).flatMap((trade) => (trade.feature ? [trade.feature] : []));

/** What the filter must show: the rows it proves compatible, and the rows it cannot check. */
function expectedNarrowed(rows: Expected[], keep: string[], withId?: string) {
  const chosen = rows.find((row) => row.id === withId);
  const compatible: Expected[] = [];
  const unchecked: Expected[] = [];
  for (const row of rows) {
    if (row.id === withId) continue;
    if (!row.checked) {
      unchecked.push(row);
      continue;
    }
    const keeps = keep.every((anchor) => resolved(row).every((feature) => feature.anchor !== anchor));
    const combines = !chosen || !resolved(chosen).some((a) => resolved(row).some((b) => overlaps(a, b)));
    if (keeps && combines) compatible.push(row);
  }
  return { compatible, unchecked };
}

const titles = (rows: Expected[]) => rows.map((row) => row.title).sort();
const list = (page: Page) => page.locator(".arch-list");
const rowsOf = (page: Page) => list(page).locator("ul.arch-rows").first().locator("li.arch-row");
const countLine = (page: Page) => list(page).locator(".list-count");
const rowTitles = async (locator: Locator) => (await locator.locator("a").allTextContents()).sort();
const overflow = (page: Page) =>
  page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
const menu = (page: Page, field: string) => page.locator(`#filter-btn-${field}`);
const panel = (page: Page, field: string) => page.locator(`#filter-panel-${field}`);

async function openClass(page: Page, id: string, width: number) {
  await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
  await page.goto(`#/entry/${id}?s=archetypes`);
  await expect(page.getByRole("heading", { level: 1, name: corpus.entries.get(id)!.title })).toBeVisible();
  await expect(list(page)).toBeVisible();
}

test("A class page lists its archetypes as rows in place of the blurbs and the matrix", async ({ page }) => {
  await installLibrary(page);
  for (const id of [WIZARD, FIGHTER]) {
    const expected = archetypesOf(id);
    expect(expected.length, id).toBeGreaterThan(10);
    for (const width of WIDTHS) {
      const label = `${id} at ${width}px`;
      await openClass(page, id, width);
      const rows = rowsOf(page);
      await expect(rows, label).toHaveCount(expected.length);
      expect(await rowTitles(rows), label).toEqual(titles(expected));
      await expect(countLine(page), label).toHaveText(`${expected.length} archetypes`);
      // Each row: the name links to the archetype's own page, then its book and what it replaces.
      for (const row of expected.slice(0, 6)) {
        const item = rows.filter({ has: page.getByRole("link", { name: row.title, exact: true }) }).first();
        await expect(item.getByRole("link", { name: row.title, exact: true }), label).toHaveAttribute(
          "href",
          `#/entry/${row.id}`,
        );
        await expect(item.locator(".row-source"), label).toHaveText(row.book);
        if (row.meta) await expect(item.locator(".arch-meta"), label).toHaveText(row.meta);
        else await expect(item.locator(".arch-meta"), label).toHaveCount(0);
        // The browse rows' own target: 44 px on a touch screen, 36 px with a fine pointer.
        const coarse = await page.evaluate(() => matchMedia("(pointer: coarse)").matches);
        expect((await item.getByRole("link").first().boundingBox())!.height, label).toBeGreaterThanOrEqual(coarse ? 44 : 36);
      }
      // The printed blurbs, the X matrix and the old panel are gone; the section's text stays.
      const body = page.locator(".rich");
      await expect(body.getByText("Modifies or Replaces:"), label).toHaveCount(0);
      await expect(body.getByRole("heading", { name: "Archetypes by Feature" }), label).toHaveCount(0);
      await expect(body.locator("table").filter({ hasText: "Archetype" }), label).toHaveCount(0);
      await expect(page.locator(".entry-rel").getByRole("heading", { name: /^Archetypes/ }), label).toHaveCount(0);
      await expect(body.getByText("each class draws upon a central archetype"), label).toBeAttached();
      // The list sits in the Archetypes section: after its heading, before the next section.
      const heading = page.locator('[data-class-section="archetypes"]');
      await expect(heading, label).toHaveText("Archetypes");
      const [headingBox, listBox] = await Promise.all([heading.boundingBox(), list(page).boundingBox()]);
      expect(listBox!.y, label).toBeGreaterThan(headingBox!.y);
      expect(listBox!.y - headingBox!.y, label).toBeLessThan(400);
      expect(await overflow(page), label).toBeLessThanOrEqual(0);
    }
  }
});

test("Keep narrows the list to the archetypes that leave the chosen features alone", async ({ page }) => {
  await installLibrary(page);
  const expected = archetypesOf(WIZARD);
  for (const width of WIDTHS) {
    const label = `Wizard at ${width}px`;
    await openClass(page, WIZARD, width);
    const keep = menu(page, "keep");
    await expect(keep, label).toHaveText("Keep: Any feature");
    expect((await keep.boundingBox())!.height, label).toBeGreaterThanOrEqual(44);
    await expect(keep, label).toHaveAttribute("aria-expanded", "false");
    await keep.click();
    await expect(keep, label).toHaveAttribute("aria-expanded", "true");
    const choices = panel(page, "keep");
    await expect(choices, label).toBeVisible();
    // Tick one feature: Arcane Bond, which several archetypes replace.
    await choices.getByRole("checkbox", { name: /^Arcane Bond, \d+ archetypes?$/ }).check();
    await expect(keep, label).toHaveText("Keep: Arcane Bond");
    // The panel stays open while the reader ticks several rows.
    await expect(choices, label).toBeVisible();
    let want = expectedNarrowed(expected, ["arcane-bond"]);
    expect(want.compatible.length, label).toBeGreaterThan(0);
    expect(want.compatible.length, label).toBeLessThan(expected.length);
    expect(await rowTitles(rowsOf(page)), label).toEqual(titles(want.compatible));
    await expect(countLine(page), label).toContainText(`${want.compatible.length} of ${expected.length} archetypes`);
    await expect(countLine(page), label).toContainText("Show all");
    // Two features.
    await choices.getByRole("checkbox", { name: /^Scribe Scroll, \d+ archetypes?$/ }).check();
    await expect(keep, label).toHaveText("Keep: 2 chosen");
    want = expectedNarrowed(expected, ["arcane-bond", "scribe-scroll"]);
    expect(await rowTitles(rowsOf(page)), label).toEqual(titles(want.compatible));
    // Rows the data cannot place are listed apart, never among the compatible ones.
    const unchecked = page.locator(".arch-unchecked");
    if (want.unchecked.length) {
      await expect(unchecked, label).toHaveText("Not checked");
      expect(await rowTitles(list(page).locator("ul.arch-rows").nth(1)), label).toEqual(titles(want.unchecked));
    } else {
      await expect(unchecked, label).toHaveCount(0);
    }
    // Escape closes the panel and returns focus to the button.
    await page.keyboard.press("Escape");
    await expect(choices, label).toBeHidden();
    await expect(keep, label).toBeFocused();
    // Show all clears the choice.
    await countLine(page).getByRole("button", { name: "Show all" }).click();
    await expect(keep, label).toHaveText("Keep: Any feature");
    await expect(countLine(page), label).toHaveText(`${expected.length} archetypes`);
    await expect(rowsOf(page), label).toHaveCount(expected.length);
    await expect(unchecked, label).toHaveCount(0);
    expect(await overflow(page), label).toBeLessThanOrEqual(0);
  }
});

test("Combine with lists the archetypes that change no feature the chosen one changes", async ({ page }) => {
  await installLibrary(page);
  const expected = archetypesOf(FIGHTER);
  // An archetype that changes a feature in whole and one that changes only levels of another.
  const chosen = expected.find((row) => row.checked && (row.trades ?? []).length >= 2)!;
  expect(chosen, "a fighter archetype with two checked trades").toBeTruthy();
  for (const width of WIDTHS) {
    const label = `Fighter at ${width}px`;
    await openClass(page, FIGHTER, width);
    const button = menu(page, "with");
    await expect(button, label).toHaveText("Combine with: Any archetype");
    await button.click();
    const choices = panel(page, "with");
    // More than ten archetypes: the panel has a find field and a status line.
    const find = choices.getByRole("searchbox", { name: "Find an archetype" });
    await expect(find, label).toBeFocused();
    await find.fill(chosen.title.toLowerCase());
    await expect(choices.getByRole("status"), label).toContainText("archetype");
    await choices.getByRole("button", { name: chosen.title, exact: true }).first().click();
    await expect(choices, label).toBeHidden();
    await expect(button, label).toHaveText(`Combine with: ${chosen.title}`);
    const want = expectedNarrowed(expected, [], chosen.id);
    expect(want.compatible.length, label).toBeGreaterThan(0);
    expect(want.compatible.length, label).toBeLessThan(expected.length - 1);
    expect(await rowTitles(rowsOf(page)), label).toEqual(titles(want.compatible));
    // The chosen archetype is not listed, and no listed one shares a feature with it.
    await expect(rowsOf(page).getByRole("link", { name: chosen.title, exact: true }), label).toHaveCount(0);
    await expect(countLine(page), label).toContainText(`${want.compatible.length} of ${expected.length} archetypes`);
    await countLine(page).getByRole("button", { name: "Show all" }).click();
    await expect(button, label).toHaveText("Combine with: Any archetype");
    await expect(rowsOf(page), label).toHaveCount(expected.length);
    expect(await overflow(page), label).toBeLessThanOrEqual(0);
  }
});
