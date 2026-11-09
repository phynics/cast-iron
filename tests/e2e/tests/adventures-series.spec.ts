import { test, expect, loadCorpus } from "./fixtures";
import { installLibrary, waitHomeIdle } from "./helpers";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const numberKey = (value: string | undefined) => (value ?? "").match(/\d+/g)?.map(Number) ?? [];
const compareNumbers = (a: any, b: any) => {
  const left = numberKey(a.series_number);
  const right = numberKey(b.series_number);
  for (let index = 0; index < Math.max(left.length, right.length); index++) {
    const diff = (left[index] ?? 0) - (right[index] ?? 0);
    if (diff) return diff;
  }
  return a.title.localeCompare(b.title);
};
const apVolumes = (name: string) =>
  sources
    .filter((source) => source.series_kind === "adventure-path" && source.series === name)
    .sort(compareNumbers);
const volumeTitle = (source: any) => {
  const rest = source.title.replace(/^Pathfinder #\d+:\s*/, "");
  return `#${source.series_number} ${rest}`;
};
const moduleBooks = () =>
  sources
    .filter((source) => source.series_kind === "module")
    .sort((a, b) => {
      const left = /^\d{4}/.test(a.publication_date ?? "") ? a.publication_date : null;
      const right = /^\d{4}/.test(b.publication_date ?? "") ? b.publication_date : null;
      if (left === null || right === null) return Number(left === null) - Number(right === null);
      return left.localeCompare(right) || a.title.localeCompare(b.title);
    });
const societyBooks = () =>
  sources.filter((source) => source.series_kind === "society").sort(compareNumbers);
const seasonsOf = () =>
  [...new Set(societyBooks().map((source) => source.season as number))].sort((a, b) => a - b);

/** Open the Adventures tier on Books, leaving its series blocks at their default state. */
async function openAdventures(page: any) {
  await page.goto("#/books");
  const tier = page.locator('.book-tier[data-tier="Adventures"]');
  const toggle = tier.locator(".tier-toggle");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(tier.locator(".adventure-group").first()).toBeVisible();
  return tier;
}

/** Open Manage books and the Adventures tier, with every series group at its default state. */
async function openManageAdventures(page: any) {
  await page.goto("#/books/manage");
  const tier = page.locator('.book-tier[data-tier="Adventures"]');
  const toggle = tier.locator(".tier-toggle");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  return tier;
}

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
  await waitHomeIdle(page);
});

test("Adventures shows Adventure Path groups, and Rise of the Runelords opens its volumes in order", async ({ page }) => {
  const tier = await openAdventures(page);
  const paths = tier.locator('.adventure-group[data-series="adventure-paths"]');
  await expect(paths).toBeVisible();
  const group = paths.locator('.adventure-series[data-series="Rise of the Runelords"]');
  await expect(group).toBeVisible();
  const volumes = apVolumes("Rise of the Runelords");
  expect(volumes.length).toBeGreaterThan(0);
  await expect(group.locator(".series-count")).toHaveText("(" + volumes.length + ")");

  const toggle = group.locator(".series-toggle");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  // It is a disclosure: closing it hides the volumes, and opening it brings them back.
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(group.locator(".book-directory-item").first()).toBeHidden();
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");

  const rows = group.locator(".book-directory-item");
  await expect(rows).toHaveCount(volumes.length);
  const ids = await rows.evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
  expect(ids).toEqual(volumes.map((source) => source.id));
  const titles = await rows.locator(".cat-name").allInnerTexts();
  expect(titles).toEqual(volumes.map(volumeTitle));
  await expect(rows.first().locator(".cat-name")).toHaveText("#1 Burnt Offerings");
  await expect(rows.nth(1).locator(".cat-name")).toHaveText("#2 The Skinsaw Murders");
});

test("Modules are one dated list without the printed prefix", async ({ page }) => {
  const tier = await openAdventures(page);
  const group = tier.locator('.adventure-group[data-series="modules"]');
  await expect(group).toBeVisible();
  const modules = moduleBooks();
  expect(modules.length).toBeGreaterThan(0);
  await expect(group.locator(".series-count")).toHaveText("(" + modules.length + ")");

  const rows = group.locator(".book-directory-item");
  await expect(rows).toHaveCount(modules.length);
  const ids = await rows.evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
  expect(ids).toEqual(modules.map((source) => source.id));
  const dates = ids.map((id) => sources.find((source) => source.id === id)?.publication_date as string | undefined);
  const dated = dates.filter((date): date is string => /^\d{4}/.test(date ?? ""));
  expect([...dated].sort()).toEqual(dated);
  if (dates.includes(undefined)) {
    const firstUndated = dates.indexOf(undefined);
    expect(dates.slice(firstUndated).every((date) => date === undefined)).toBe(true);
  }
  const titles = await rows.locator(".cat-name").allInnerTexts();
  expect(titles.some((title) => title.startsWith("Pathfinder Module:"))).toBe(false);
  await expect(rows.first().locator(".cat-name")).toHaveText(modules[0].title);
});

test("Pathfinder Society groups by season and lists its scenarios by number", async ({ page }) => {
  const tier = await openAdventures(page);
  const society = tier.locator('.adventure-group[data-series="society"]');
  await expect(society).toBeVisible();
  const seasons = seasonsOf();
  expect(seasons.length).toBeGreaterThan(0);
  await expect(society.locator(".adventure-season")).toHaveCount(seasons.length);
  await expect(society.locator(":scope > .series-head .series-count")).toHaveText(
    "(" + societyBooks().length + ")",
  );

  for (const season of seasons) {
    const group = society.locator(`.adventure-season[data-series="season-${season}"]`);
    await expect(group).toHaveCount(1);
    const books = societyBooks().filter((source) => source.season === season);
    await expect(group.locator(".series-count")).toHaveText("(" + books.length + ")");
    const toggle = group.locator(".series-toggle");
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    const ids = await group
      .locator(".book-directory-item")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
    expect(ids).toEqual(books.map((source) => source.id));
    expect(await group.locator(".book-directory-item").count()).toBe(books.length);
  }
});

test("An Adventure Path's bulk toggle changes its N of M active tally", async ({ page }) => {
  const tier = await openManageAdventures(page);
  const group = tier.locator(
    '.adventure-group[data-series="adventure-paths"] .adventure-series[data-series="Rise of the Runelords"]',
  );
  await expect(group).toBeVisible();
  const volumes = apVolumes("Rise of the Runelords");
  const count = group.locator(".series-count");
  await expect(count).toHaveText(`${volumes.length} of ${volumes.length} active`);

  // Every series group on Manage books carries the tally, not only a path.
  const modules = tier.locator('.adventure-group[data-series="modules"]');
  await expect(modules.locator(".series-count")).toHaveText(
    `${moduleBooks().length} of ${moduleBooks().length} active`,
  );

  const off = group.getByRole("button", { name: `Turn off all Rise of the Runelords books`, exact: true });
  await expect(off).toHaveCount(1);
  await off.click();
  await expect(count).toHaveText(`0 of ${volumes.length} active`);
  await expect(group.getByRole("button", { name: `Turn on all Rise of the Runelords books`, exact: true })).toHaveCount(1);

  const on = group.getByRole("button", { name: `Turn on all Rise of the Runelords books`, exact: true });
  await on.click();
  await expect(count).toHaveText(`${volumes.length} of ${volumes.length} active`);
  await expect(group.getByRole("button", { name: `Turn off all Rise of the Runelords books`, exact: true })).toHaveCount(1);
});
