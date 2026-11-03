import { test, expect, loadCorpus } from "./fixtures";
import { goOffline } from "./fixtures";
import { hitTitles, installLibrary, pct, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();

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
  await page.locator(".filters .chip", { hasText: /^evocation$/i }).click();
  await page.locator(".filter-group", { hasText: "Spell level" }).locator(".chip", { hasText: /^3$/ }).click();
  await expect.poll(() => page.url()).toMatch(/spell_level:3/);
  const expected = [...corpus.entries.values()]
    .filter((e) => e.kind === "spell" && e.fields.school === "evocation" && e.fields.levels.some((l: any) => l.level === 3))
    .map((e) => e.title)
    .sort();
  expect(expected.length).toBeGreaterThan(10);
  while (await page.locator(".results .hit").count() < expected.length) {
    const before = await page.locator(".results .hit").count();
    await page.getByRole("button", { name: "Show more results" }).click();
    await expect(page.locator(".results .hit")).toHaveCount(Math.min(before + 50, expected.length));
  }
  await expect(page.locator(".results .hit")).toHaveCount(expected.length);
  expect((await hitTitles(page)).sort()).toEqual(expected);
  const metas = await page.locator(".results .hit-meta").allInnerTexts();
  expect(metas.every((m) => /^Spell/.test(m))).toBe(true);

  // Speed: p95 of the engine's own timing over 20 queries stays under 100 ms.
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
  expect(p95).toBeLessThan(100);

  // Nothing but the browser's own service worker update check (sw.js, refused) reached the server.
  expect((await server.requests()).filter((p) => !p.startsWith("sw.js"))).toEqual([]);
});
