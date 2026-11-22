import type { Locator } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import {
  bookSources,
  firstEntry,
  installLibrary,
  openQuickSearch,
  waitControlled,
  waitHomeIdle,
  waitReady,
  waitResults,
  writeMeta,
} from "./helpers";

// The entry header and the four chip roles in DESIGN.md: metadata is a text line, filters are the
// only pills, navigation is a rectangle, labels carry no border.
const corpus = loadCorpus();
const single = firstEntry("an entry in one book", (entry) => entry.kind === "feat" && entry.source_ids.length === 1);
const multi = firstEntry("an entry in several books", (entry) => entry.kind === "spell" && entry.source_ids.length > 2);

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

/** One line of text: shorter than 1.6 line heights, whatever the corpus titles are. */
async function expectOneLine(meta: Locator) {
  const lineHeight = await meta.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await meta.boundingBox())!.height).toBeLessThan(lineHeight * 1.6);
}

test("The metadata line is one line of links, and extra books open on demand", async ({ page }) => {
  await page.goto(`#/entry/${single.id}`);
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link")).toHaveCount(2);
  await expect(meta.locator(".meta-more")).toHaveCount(0);
  await expectOneLine(meta);
  await expect(page.getByRole("link", { name: "Related entries" })).toHaveCount(0);

  await page.goto(`#/entry/${multi.id}`);
  const extra = multi.source_ids.length - 1;
  const more = meta.locator(".meta-more");
  await expect(more.locator("summary")).toHaveText(`+${extra} more`);
  // Kind, first book, and the disclosure share one line.
  await expectOneLine(meta);
  // Closed, the extra books are out of the accessibility tree, so count anchors, not roles.
  await expect(more.locator("a")).toHaveCount(extra);
  await expect(more.locator("a").first()).toBeHidden();
  const summaryTop = (await more.locator("summary").boundingBox())!.y;
  await more.locator("summary").click();
  await expect(more.getByRole("link").first()).toBeVisible();
  // Opening the list keeps the summary on the first line; only the extra books expand below.
  expect(Math.abs((await more.locator("summary").boundingBox())!.y - summaryTop)).toBeLessThan(1);
  expect((await more.getByRole("link").first().boundingBox())!.y).toBeGreaterThan(summaryTop);
});

test("The star is an icon-only 44 by 44 toggle in the title row", async ({ page }) => {
  await page.goto(`#/entry/${multi.id}`);
  const star = page.locator(".bookmark-toggle");
  await expect(star).toBeEnabled();
  await expect(star).toHaveAccessibleName(`Star ${multi.title}`);
  await expect(star).toHaveAttribute("aria-pressed", "false");
  expect((await star.innerText()).trim()).toBe("");
  const box = (await star.boundingBox())!;
  expect(Math.round(box.width)).toBe(44);
  expect(Math.round(box.height)).toBe(44);
  const title = (await page.getByTestId("entry-title").boundingBox())!;
  expect(box.y).toBeLessThan(title.y + title.height);

  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "true");
  await expect(star).toHaveAccessibleName(`Unstar ${multi.title}`);
});

test("Keyboard focus is visible on the star and on metadata links", async ({ page }) => {
  await page.goto(`#/entry/${multi.id}`);
  await expect(page.locator(".bookmark-toggle")).toBeEnabled();
  const seen = new Set<string>();
  for (let i = 0; i < 40 && seen.size < 2; i++) {
    await page.keyboard.press("Tab");
    const state = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el) return null;
      const inHead = el.closest(".entry-head");
      if (!inHead) return null;
      const style = getComputedStyle(el);
      return {
        what: el.classList.contains("bookmark-toggle") ? "star" : "meta",
        ok: style.outlineStyle !== "none" && parseFloat(style.outlineWidth) >= 2,
      };
    });
    if (state) {
      expect(state.ok, `${state.what} focus ring`).toBe(true);
      seen.add(state.what);
    }
  }
  expect([...seen].sort()).toEqual(["meta", "star"]);
});

test("Chips keep one form per role", async ({ page }) => {
  // Navigation: Quick search's recently viewed entries are plain link rows, not pills or boxes.
  await page.goto(`#/entry/${single.id}`);
  await expect(page.getByTestId("entry-title")).toBeVisible();
  await page.goto("#/");
  await openQuickSearch(page);
  const recent = page.locator("#quick-search-list-viewed .quick-search-row").first();
  await expect(recent).toBeVisible();
  expect(await recent.evaluate((el) => getComputedStyle(el).borderTopWidth)).toBe("0px");
  expect(await recent.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("0px");
  await page.keyboard.press("Escape");

  // Filters: the only pill; a selected one is filled and carries a check, and keeps its count.
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const chip = page.locator(".filters .chip", { hasText: /^Spells/ });
  const pill = chip.locator("span").first();
  expect(await pill.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("999px");
  expect(await pill.evaluate((el) => getComputedStyle(el, "::before").content)).toBe("none");
  await chip.click();
  await expect(chip.locator(".chip-count")).toHaveText(/^[\d,]+$/);
  expect(await pill.evaluate((el) => getComputedStyle(el, "::before").content)).toBe('""');
  const selected = await pill.evaluate((el) => {
    const s = getComputedStyle(el);
    return { background: s.backgroundColor, accent: getComputedStyle(document.documentElement).getPropertyValue("--accent") };
  });
  expect(selected.background).not.toBe("rgba(0, 0, 0, 0)");

  // A list page filters through a menu button, not a pill: 6 px corners and the page flow.
  await page.goto("#/browse/kind/creature");
  const menu = page.getByRole("button", { name: /^Type: / });
  await expect(menu).toBeVisible();
  expect(await menu.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("6px");
  expect(corpus.catalog.entries.length).toBeGreaterThan(0);
});

test("A label has no border and cannot be mistaken for a link", async ({ page }) => {
  // The feat chain screen marks repeating links with a label.
  await page.goto("#/feats/chains");
  const label = page.locator(".tag").first();
  if ((await label.count()) === 0) test.skip(true, "This library has no repeating chain links");
  await expect(label).toBeVisible();
  const style = await label.evaluate((el) => {
    const s = getComputedStyle(el);
    return { border: s.borderTopWidth, radius: s.borderRadius, cursor: s.cursor, link: !!el.closest("a") };
  });
  expect(style.border).toBe("0px");
  expect(style.radius).toBe("0px");
  expect(style.cursor).not.toBe("pointer");
});

// #446: the References panel spans from the contents row to the book row of the entry grid. A
// spanning item shares its extra height equally among the auto rows it crosses, so the empty
// contents row grew to about 200px above a short item entry. Only the slack row may grow.
test("A short item entry has no blank band between the header and its text at desktop width", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  for (const id of ["ref/eq-weapon/longsword", "ref/eq-weapon/dagger", "ref/eq-armor/chain-shirt"]) {
    const entry = corpus.entries.get(id);
    expect(entry, `${id} is in the corpus`).toBeTruthy();
    await page.goto(`#/entry/${id}`);
    await expect(page.getByRole("heading", { level: 1, name: entry!.title })).toBeVisible();
    const head = (await page.locator(".entry > .entry-head").boundingBox())!;
    const body = (await page.locator(".entry > .rich").first().boundingBox())!;
    const refs = (await page.locator(".entry > .entry-refs").boundingBox())!;
    // The body starts a normal paragraph gap under the header, whatever the panel's height.
    expect(body.y - (head.y + head.height), `${id}: gap under the header`).toBeLessThan(48);
    // The panel sits beside the body, level with it.
    expect(Math.abs(refs.y - body.y), `${id}: panel and body tops`).toBeLessThan(48);
  }
});

// #574: the metadata line names the parent of an entry, and the facts line names the typed facts
// that classify it. No end panel repeats either.
const factTexts = (page: any) => page.locator(".entry-head > .entry-facts span:not(.meta-sep):not(.sr-only)").allInnerTexts();

test("A feat names its types in the facts line, and a feat without types has none", async ({ page }) => {
  const typed = firstEntry("a feat with two types", (entry) => entry.kind === "feat" && (entry.fields?.feat_types ?? []).length >= 2);
  await page.goto(`#/entry/${typed.id}`);
  await expect(page.getByTestId("entry-title")).toHaveText(typed.title);
  expect(await factTexts(page)).toEqual(typed.fields.feat_types);
  await expect(page.locator(".entry-meta").getByRole("link").first()).toHaveText("Feat");

  const power = corpus.entries.get("ref/feat/power-attack");
  expect(power, "Power Attack is in the corpus").toBeTruthy();
  await page.goto("#/entry/ref/feat/power-attack");
  await expect(page.locator(".entry-head > .entry-facts")).toHaveText("Combat");

  const plain = firstEntry("a feat without types", (entry) => entry.kind === "feat" && (entry.fields?.feat_types ?? []).length === 0);
  await page.goto(`#/entry/${plain.id}`);
  await expect(page.getByTestId("entry-title")).toHaveText(plain.title);
  await expect(page.locator(".entry-head > .entry-facts")).toHaveCount(0);
});

test("A trait names its category in the metadata line and its detail in the facts line", async ({ page }) => {
  await page.goto("#/entry/ref/trait/reactionary");
  const meta = page.locator(".entry-meta");
  const kind = meta.getByRole("link", { name: "Combat trait", exact: true });
  await expect(kind).toHaveAttribute("href", /facet=trait_category:combat/);
  await expect(page.locator("#rel-trait")).toHaveCount(0);
  await expect(page.locator(".entry-head > .entry-facts")).toHaveCount(0);
  await kind.click();
  await expect(page.getByRole("button", { name: "Category: Combat" })).toBeVisible();

  const detailed = firstEntry("a trait with a detail", (entry) => entry.kind === "trait" && !!entry.fields?.detail);
  await page.goto(`#/entry/${detailed.id}`);
  await expect(page.getByTestId("entry-title")).toHaveText(detailed.title);
  expect(await factTexts(page)).toEqual([detailed.fields.detail]);
});

test("An archetype names its class in the metadata line and what it replaces in the facts line", async ({ page }) => {
  await page.goto("#/entry/ref/arc-alchemist/aerochemist");
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: "Archetype", exact: true })).toBeVisible();
  await expect(meta.getByRole("link", { name: "Alchemist", exact: true })).toBeVisible();
  await expect(page.locator("#rel-archetype-of")).toHaveCount(0);
  expect((await factTexts(page))[0]).toMatch(/^Replaces /);
});

test("A racial trait names its kind and its race, and the race stays linked when its book is off", async ({ page }) => {
  const books = bookSources();
  const sourcesOf = (entry: any) => entry.source_ids as string[];
  const pair = [...corpus.entries.values()]
    .filter((entry) => entry.kind === "racial_trait")
    .map((entry) => ({ entry, race: (entry.relations ?? []).find((edge: any) => edge.kind === "part") }))
    .map(({ entry, race }) => ({ entry, race: race && corpus.entries.get(race.target) }))
    .find(({ entry, race }) => race && sourcesOf(race).length === 1 && !sourcesOf(entry).includes(sourcesOf(race)[0]));
  if (!pair) test.skip(true, "This library has no racial trait whose race has its own book");
  const { entry, race } = pair!;
  const kind = (entry.fields?.alternate ? "Alternate" : "Standard") + " racial trait";

  await page.goto(`#/entry/${entry.id}`);
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: kind, exact: true })).toHaveAttribute("href", /#\/browse\/kind\/racial_trait/);
  const raceLink = meta.locator(`a[href="#/entry/${race.id}"]`);
  await expect(raceLink).toHaveText(race.title);
  await expect(meta.locator(".tag")).toHaveCount(0);
  await expect(page.locator("#rel-part")).toHaveCount(0);

  // The race's own book off: the link stays and the plain Inactive label follows it.
  await waitControlled(page);
  await page.goto("#/");
  await waitHomeIdle(page);
  await writeMeta(page, "disabled_books", [sourcesOf(race)[0]].filter((id) => books.includes(id)));
  await page.reload();
  await waitReady(page);
  await page.goto(`#/entry/${entry.id}`);
  await expect(page.getByTestId("entry-title")).toHaveText(entry.title);
  await expect(raceLink).toHaveText(race.title);
  await expect(meta.locator("a[href^='#/entry/ref/'] + .tag")).toHaveText("Inactive");
});
