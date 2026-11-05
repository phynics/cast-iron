import type { Page } from "@playwright/test";
import { test, expect, loadCorpus } from "./fixtures";
import { goOffline } from "./fixtures";
import { hitTitles, installLibrary, pct, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();

/** The chip whose label starts with `label`, and the count it shows. */
const chipOf = (page: Page, label: RegExp) => page.locator(".filters .chip", { hasText: label });
const chipCount = async (page: Page, label: RegExp) =>
  Number(await chipOf(page, label).locator(".chip-count").innerText());

/** Every entry that the results list renders, after opening all pages and publication groups. */
async function renderedEntries(page: Page) {
  const more = page.getByRole("button", { name: "Show more results" });
  while (await more.isVisible()) await more.click();
  return page.locator(".results li.hit:not(.hit-group)").count();
}

test("Filter chips count the entries that choosing them shows", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  const kindTotals = new Map<string, number>();
  for (const e of corpus.entries.values()) kindTotals.set(e.kind, (kindTotals.get(e.kind) ?? 0) + 1);

  // Without a query the chips are the totals for the active books.
  await expect(page.locator(".search-page")).toBeVisible();
  await expect.poll(() => chipCount(page, /^Spells/)).toBe(kindTotals.get("spell"));
  await expect.poll(() => chipCount(page, /^Feats/)).toBe(kindTotals.get("feat"));

  // A query narrows every chip. The chip for a kind equals the results of choosing that kind.
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");
  const spells = await chipCount(page, /^Spells/);
  const feats = await chipCount(page, /^Feats/);
  expect(spells).toBeGreaterThan(0);
  expect(spells).toBeLessThan(kindTotals.get("spell")!);
  await chipOf(page, /^Spells/).click();
  await expect(page.locator(".results-region")).toHaveAttribute("data-search-ms", /.+/);
  await expect.poll(() => renderedEntries(page)).toBe(spells);
  // The kind chips ignore their own selection, so an alternative keeps its count.
  expect(await chipCount(page, /^Spells/)).toBe(spells);
  expect(await chipCount(page, /^Feats/)).toBe(feats);

  // A facet value equals the results of choosing it, on top of the query and the kind.
  const school = page.locator(".filter-group", { hasText: "School" }).locator(".chip", { hasText: /^evocation\d+$/i });
  const evocation = Number(await school.locator(".chip-count").innerText());
  expect(evocation).toBeGreaterThan(0);
  expect(evocation).toBeLessThanOrEqual(spells);
  await school.click();
  await expect.poll(() => page.url()).toContain("school:evocation");
  await expect.poll(() => renderedEntries(page)).toBe(evocation);
  // A publication chip now counts what that book adds to the same selection.
  const books = page.locator('.filters .filter-group:has(legend:text-is("Publication")) .chip:not([hidden])');
  for (const chip of (await books.all()).slice(0, 4)) {
    const n = Number(await chip.locator(".chip-count").innerText());
    await chip.click();
    await expect.poll(() => renderedEntries(page)).toBe(n);
    await chip.click();
    await expect.poll(() => renderedEntries(page)).toBe(evocation);
  }
});

test("Chip counts keep their last value while a query is pending", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");
  const countsOf = () =>
    page.evaluate(() => [...document.querySelectorAll(".filters .chip-count")].map((el) => el.textContent));
  const widths = () =>
    page.evaluate(() => [...document.querySelectorAll(".filters .chip")].map((el) => el.getBoundingClientRect().width));
  const before = await countsOf();
  const widthsBefore = await widths();
  expect(before.length).toBeGreaterThan(3);
  expect(before.every((text) => /^\d+$/.test(text ?? ""))).toBe(true);
  // The debounce has not fired yet when this script reads the chips back.
  const during = await page.evaluate(() => {
    const input = document.getElementById("search-input") as HTMLInputElement;
    input.value = "fireb";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return [...document.querySelectorAll(".filters .chip-count")].map((el) => el.textContent);
  });
  expect(during).toEqual(before);
  await waitResults(page, "fireb");
  expect(await countsOf()).not.toEqual(before);
  expect((await widths()).length).toBe(widthsBefore.length);
});

test("Search operates entirely offline", async ({ page, context, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await goOffline(context, server);
  await page.reload();
  await expect(page.locator("#home-title")).toBeVisible();
  await expect(page.locator(".badge")).toBeVisible(); // the offline badge
  await server.clearLog();

  // Results are live: typing is enough, there is no submit.
  await page.keyboard.press("/");
  await expect(page.locator("#search-input")).toBeFocused();
  await page.keyboard.type("fir");
  await page.keyboard.press("Enter");
  await expect(page.locator(".search-page")).toBeVisible();
  await page.locator("#search-input").pressSequentially("e");
  await waitResults(page, "fire");
  const fire = await hitTitles(page);
  expect(fire).toContain("Fireball");
  expect(fire.slice(0, 5).some(title => title.toLowerCase().startsWith("fire"))).toBe(true);
  await expect(page.locator(".results .hit-title mark").first()).toBeVisible();
  await expect(page.locator(".result-count")).toHaveAttribute("aria-live", "polite");

  await page.locator("#search-input").fill("fireball");
  await waitResults(page, "fireball");
  expect((await hitTitles(page))[0]).toBe("Fireball");

  // Filters: school = evocation and spell level 3 give the set that the compiled data has.
  await page.goto("#/search");
  await page.locator("#search-input").fill("");
  await page.locator(".filters .chip", { hasText: /^Spells/ }).click();
  await page.locator(".filters .chip", { hasText: /^evocation\d+$/i }).click();
  await page.locator(".filter-group", { hasText: "Spell level" }).locator(".chip", { hasText: /^3\d+$/ }).click();
  await expect.poll(() => page.url()).toMatch(/spell_level:3/);
  const expected = [...corpus.entries.values()]
    .filter((e) => e.kind === "spell" && e.fields.school === "evocation" && e.fields.levels.some((l: any) => l.level === 3))
    .map((e) => e.title)
    .sort();
  const expectedGroups = [...new Set(expected)];
  expect(expectedGroups.length).toBeGreaterThan(10);
  while (await page.locator(".results > .hit").count() < expectedGroups.length) {
    const before = await page.locator(".results > .hit").count();
    await page.getByRole("button", { name: "Show more results" }).click();
    await expect(page.locator(".results > .hit")).toHaveCount(Math.min(before + 50, expectedGroups.length));
  }
  await expect(page.locator(".results > .hit")).toHaveCount(expectedGroups.length);
  expect((await page.locator(".results > .hit > .hit-link > .hit-title, .results > .hit > details > summary .hit-title").allInnerTexts()).sort()).toEqual(expectedGroups);
  const metas = await page.locator(".results .hit-meta").allInnerTexts();
  expect(metas.every((m) => /^Spell/.test(m))).toBe(true);

  // Speed: the engine's own timing over 20 queries is recorded as an annotation. A shared
  // CI runner is not a latency gate, so only a loose smoke bound is asserted; a p95 near it
  // means the index failed to load or the engine regressed, not that the runner was busy.
  const queries = [
    "fire", "cure light wounds", "grapple", "dragon", "sword", "armor class", "turn undead", "magic missile", "wizard", "bless",
    "invisibility", "potion", "goblin", "attack of opportunity", "ranger", "stealth", "charm", "weapon focus", "fly", "summon",
  ];
  await page.goto("#/search");
  const times: number[] = [];
  for (const query of queries) {
    await page.locator("#search-input").fill(query);
    await waitResults(page, query);
    times.push(Number(await page.locator(".results-region").getAttribute("data-search-ms")));
  }
  expect(times).toHaveLength(20);
  expect(times.every((t) => Number.isFinite(t))).toBe(true);
  const p95 = pct(times, 0.95);
  test.info().annotations.push({ type: "search p95 ms", description: `${p95} (max ${Math.max(...times)})` });
  expect(p95).toBeLessThan(250);

  // Nothing but the browser's own service worker update check (sw.js, refused) reached the server.
  expect((await server.requests()).filter((p) => !p.startsWith("sw.js"))).toEqual([]);
});
