import type { Page } from "@playwright/test";
import { test, installedTest, expect, loadCorpus } from "./fixtures";
import { goOffline } from "./fixtures";
import { bodyText, corpusEntry, hitTitles, installLibrary, pct, showAllResults, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

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

installedTest("Filter chips count the entries that choosing them shows", async ({ page }) => {
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
  const books = page.locator('.filters .filter-group:has(legend:text-is("Publication")) .chip');
  const labels = await books.locator("input").evaluateAll((inputs) =>
    inputs.slice(0, 4).map((input) => input.getAttribute("aria-label")),
  );
  for (const label of labels) {
    const chip = books.filter({ has: page.getByRole("checkbox", { name: label!, exact: true }) });
    const n = Number(await chip.locator(".chip-count").innerText());
    await chip.getByRole("checkbox", { name: label!, exact: true }).click();
    await expect.poll(() => renderedEntries(page)).toBe(n);
    await chip.getByRole("checkbox", { name: label!, exact: true }).click();
    await expect.poll(() => renderedEntries(page)).toBe(evocation);
  }
});

test("Publication filter hides empty books, pages the rest, and keeps selections in the URL", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");

  const group = page.locator('.filters .filter-group:has(legend:text-is("Publication"))');
  const chips = group.locator(".chip");
  const summary = group.locator(".publication-summary");
  await expect(summary).toHaveText(/Showing 12 of \d+ publications with results/);
  const totalWithResults = Number((await summary.innerText()).match(/of (\d+)/)![1]);
  expect(totalWithResults).toBeLessThan(corpus.catalog.sources.length);
  await expect(chips).toHaveCount(12);
  const firstPageCounts = await chips.locator(".chip-count").allInnerTexts();
  expect(firstPageCounts.every((count) => Number(count) > 0)).toBe(true);

  // The name box and every chip are keyboard reachable, and a chip announces its count.
  const filterBox = group.getByRole("searchbox", { name: "Filter publications by name" });
  await filterBox.focus();
  expect(await filterBox.evaluate((el) => document.activeElement === el)).toBe(true);
  await page.keyboard.press("Tab");
  const focused = await page.evaluate(() => {
    const el = document.activeElement as HTMLInputElement | null;
    return { type: el?.type, label: el?.getAttribute("aria-label") };
  });
  expect(focused.type).toBe("checkbox");
  expect(focused.label).toMatch(/entries$/);

  // The first page is capped, and "Show all" exposes every publication with results only.
  const more = group.locator("button.publication-more");
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect(more).toContainText("Show all publications");
  await more.click();
  await expect(more).toHaveAttribute("aria-expanded", "true");
  await expect(more).toContainText("Show fewer publications");
  await expect(summary).toHaveText(`Showing all ${totalWithResults} publications with results.`);
  await expect(chips).toHaveCount(totalWithResults);
  const expanded = await chips.locator("input").evaluateAll((inputs) =>
    inputs.map((input) => ({
      label: input.getAttribute("aria-label") ?? "",
      count: Number((input.parentElement?.querySelector(".chip-count") as HTMLElement | null)?.textContent ?? "0"),
    })),
  );
  expect(expanded.every((chip) => chip.count > 0)).toBe(true);
  const outsideFirstPage = expanded[12];
  expect(outsideFirstPage, "the corpus needs more than one page of publications").toBeTruthy();

  // Collapsing again hides the rest; the name box finds one of them without "Show all".
  await more.click();
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect(more).toContainText("Show all publications");
  await expect(chips).toHaveCount(12);
  const outside = group.getByRole("checkbox", { name: outsideFirstPage.label, exact: true });
  await filterBox.fill(outsideFirstPage.label.replace(/, \d+ entr(y|ies)$/, ""));
  await expect(outside).toHaveCount(1);
  await expect(outside.locator("xpath=..").locator(".chip-count")).toHaveText(String(outsideFirstPage.count));

  // Selecting a publication writes it to the URL and keeps the chip visible after a reload.
  await filterBox.fill("");
  await expect(chips).toHaveCount(12);
  const chosen = expanded[0];
  const chosenChip = group.getByRole("checkbox", { name: chosen.label, exact: true });
  await chosenChip.click();
  const chosenTitle = chosen.label.replace(/, \d+ entr(y|ies)$/, "");
  const selectedSource = corpus.catalog.sources.find((source: any) => source.title === chosenTitle);
  expect(selectedSource, `catalog source for ${chosenTitle}`).toBeTruthy();
  const selected = () => {
    const hash = new URL(page.url()).hash;
    return new URLSearchParams(hash.split("?")[1] ?? "").getAll("source");
  };
  await expect.poll(selected).toContain(selectedSource.id);

  // Let the filtered search finish before reloading. A reload while it is still reading cached
  // entry shards abandons a Cache API read, which WebKit reports as a page error at teardown.
  // Waiting also proves the publication really filters the results, not just the URL.
  await expect
    .poll(async () => {
      const metas = await page.locator(".results li.hit:not(.hit-group) .hit-meta").allTextContents();
      return metas.length > 0 && metas.every((meta) => meta.endsWith(`· ${chosenTitle}`));
    })
    .toBe(true);

  await page.reload();
  await waitResults(page, "fire");
  const reloaded = group.getByRole("checkbox", { name: chosen.label, exact: true });
  await expect(reloaded).toBeVisible();
  await expect(reloaded).toBeChecked();

  // A selected publication with no match stays visible at zero so it can be cleared, then goes.
  await page.locator("#search-input").fill("no-such-rule-zzzz");
  await waitResults(page, "no-such-rule-zzzz");
  await expect.poll(() => chips.count()).toBe(1);
  const zero = chips.first();
  await expect(zero.locator("input")).toBeChecked();
  await expect(zero.locator(".chip-count")).toHaveText("0");
  await zero.locator("input").click();
  await expect.poll(() => chips.count()).toBe(0);
});

test("Publication filter groups publications by product line with Adventures collapsed", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");

  const group = page.locator('.filters .filter-group:has(legend:text-is("Publication"))');
  const tiers = group.locator(".publication-group");
  const summary = group.locator(".publication-summary");
  const tierOrder = ["Rulebooks", "Player Companions", "Campaign Setting", "Adventures", "Other"];
  const shownTiers = () => tiers.evaluateAll((els) => els.map((el) => el.getAttribute("data-tier")));
  const inOrder = (names: (string | null)[]) => {
    const positions = names.map((name) => tierOrder.indexOf(name ?? ""));
    expect(positions.every((position) => position >= 0)).toBe(true);
    expect(positions).toEqual([...positions].sort((a, b) => a - b));
  };

  // Rulebooks lead the list, and the first page still spends the same twelve-publication limit.
  await expect(summary).toHaveText(/Showing 12 of \d+ publications with results/);
  expect((await shownTiers())[0]).toBe("Rulebooks");
  inOrder(await shownTiers());
  await expect(group.locator(".chip")).toHaveCount(12);

  // A group is labelled, and each chip keeps its own name.
  const labelled = await group.locator('.publication-group [role="group"], .publication-group[role="group"]').evaluateAll((els) =>
    els.map((el) => {
      const id = el.getAttribute("aria-labelledby");
      return id ? document.getElementById(id)?.textContent?.trim() : null;
    }),
  );
  expect(labelled.length).toBeGreaterThan(0);
  expect(labelled.every((name) => !!name)).toBe(true);

  // Adventures take no budget, but the heading is there at once. It starts closed and its
  // summary counts the publications with results that it holds.
  const adventures = group.locator('details.publication-group[data-tier="Adventures"]');
  await expect(adventures, "the corpus needs an Adventures publication with results for 'fire'").toHaveCount(1);
  await expect(adventures).not.toHaveAttribute("open", "");
  const adventureTitle = adventures.locator("summary");
  await expect(adventureTitle).toHaveText(/^Adventures \(\d+\)$/);
  const adventureCount = Number((await adventureTitle.innerText()).match(/\((\d+)\)/)![1]);
  await expect(adventures.locator(".chip")).toHaveCount(0);
  expect((await adventureTitle.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  expect(await adventureTitle.evaluate((el) => getComputedStyle(el).borderRadius)).not.toBe("999px");
  expect(await adventureTitle.evaluate((el) => getComputedStyle(el).userSelect)).toBe("none");

  // The summary is keyboard operable. Opening it shows the chips, and choosing one filters the
  // results and keeps the group open after the counts refresh.
  await adventureTitle.focus();
  await page.keyboard.press("Enter");
  await expect(adventures).toHaveAttribute("open", "");
  await expect(adventures.locator(".chip")).toHaveCount(adventureCount);
  const chip = adventures.locator(".chip").first();
  await expect(chip).toBeVisible();
  const label = (await chip.locator("input").getAttribute("aria-label"))!;
  const title = label.replace(/, \d+ entr(y|ies)$/, "");
  const source = corpus.catalog.sources.find((candidate: any) => candidate.title === title);
  expect(source, `catalog source for ${title}`).toBeTruthy();
  await chip.locator("input").click();
  await expect
    .poll(async () => {
      const metas = await page.locator(".results li.hit:not(.hit-group) .hit-meta").allTextContents();
      return metas.length > 0 && metas.every((meta) => meta.endsWith(`· ${title}`));
    })
    .toBe(true);
  await expect(adventures).toHaveAttribute("open", "");
  await expect(adventures.getByRole("checkbox", { name: label, exact: true })).toBeChecked();
  inOrder(await shownTiers());

  // A selected Adventure keeps its group open, even after a reload, and cannot be closed.
  await page.reload();
  await waitResults(page, "fire");
  const reloaded = group.locator('details.publication-group[data-tier="Adventures"]');
  await expect(reloaded).toHaveAttribute("open", "");
  await expect(reloaded.getByRole("checkbox", { name: label, exact: true })).toBeChecked();
  // Count the toggle events, so the wait ends when the close has been seen and answered.
  await reloaded.evaluate((el) => {
    (window as any).__toggles = 0;
    el.addEventListener("toggle", () => (window as any).__toggles++);
  });
  await reloaded.locator("summary").click();
  await expect.poll(() => page.evaluate(() => (window as any).__toggles)).toBeGreaterThan(0);
  await expect(reloaded).toHaveAttribute("open", "");
  const hash = new URL(page.url()).hash;
  expect(new URLSearchParams(hash.split("?")[1] ?? "").getAll("source")).toContain(source.id);
});

test("Publication filter keeps a closed heading with a count for every tier the budget skipped", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");

  const group = page.locator('.filters .filter-group:has(legend:text-is("Publication"))');
  const summary = group.locator(".publication-summary");
  const chips = group.locator(".chip");

  // The twelve slots go to Rulebooks, which keep their chips.
  await expect(summary).toHaveText(/Showing 12 of \d+ publications with results/);
  const total = Number((await summary.innerText()).match(/of (\d+)/)![1]);
  await expect(chips).toHaveCount(12);
  const rulebooks = group.locator('.publication-group[data-tier="Rulebooks"]');
  await expect(rulebooks.locator(".chip")).toHaveCount(12);
  const rulebookTitle = await rulebooks.locator(".publication-group-title").innerText();
  const rulebookTotal = Number(rulebookTitle.match(/^Rulebooks \(12 of (\d+)\)$/)![1]);

  // Every other tier with results is a closed heading with its count, in tier order.
  const headings = group.locator("details.publication-group > summary");
  await expect(headings).toHaveText([
    /^Player Companions \(\d+\)$/,
    /^Campaign Setting \(\d+\)$/,
    /^Adventures \(\d+\)$/,
    /^Other \(\d+\)$/,
  ]);
  const held = (await headings.allInnerTexts()).map((text) => Number(text.match(/\((\d+)\)/)![1]));
  // The counts reconcile with the summary line: every publication with results is in a group.
  expect(rulebookTotal + held.reduce((sum, n) => sum + n, 0)).toBe(total);
  for (const details of await group.locator("details.publication-group").all()) {
    await expect(details).not.toHaveAttribute("open", "");
    await expect(details.locator(".chip")).toHaveCount(0);
  }
  // Each heading is a labelled group once its chips are listed.
  const companions = group.locator('details.publication-group[data-tier="Player Companions"]');
  const companionCount = held[0];

  // A real click opens the tier and lists all of its publications, with no "Show all".
  await companions.locator("summary").click();
  await expect(companions).toHaveAttribute("open", "");
  await expect(companions.locator(".chip")).toHaveCount(companionCount);
  await expect(summary).toHaveText(`Showing ${12 + companionCount} of ${total} publications with results.`);
  await expect(group.locator("button.publication-more")).toHaveAttribute("aria-expanded", "false");
  const grouped = await companions.locator('[role="group"]').getAttribute("aria-labelledby");
  expect(await page.locator(`#${grouped}`).innerText()).toBe(await companions.locator("summary").innerText());

  // Choosing one of them filters the results, and the tier stays open after the counts refresh.
  const chip = companions.locator(".chip").first();
  const label = (await chip.locator("input").getAttribute("aria-label"))!;
  const title = label.replace(/, \d+ entr(y|ies)$/, "");
  await chip.locator("input").click();
  await expect
    .poll(async () => {
      const metas = await page.locator(".results li.hit:not(.hit-group) .hit-meta").allTextContents();
      return metas.length > 0 && metas.every((meta) => meta.endsWith(`· ${title}`));
    })
    .toBe(true);
  await expect(companions).toHaveAttribute("open", "");
  await expect(companions.locator(".chip")).toHaveCount(companionCount);
  await expect(companions.getByRole("checkbox", { name: label, exact: true })).toBeChecked();

  // "Show all" and the headings do not fight: Show all lists and opens every tier.
  await group.locator("button.publication-more").click();
  await expect(summary).toHaveText(new RegExp(`^Showing all \\d+ publications with results\\.$`));
  for (const details of await group.locator("details.publication-group").all()) {
    await expect(details).toHaveAttribute("open", "");
  }
});

test("The Publication name box opens Adventures for its matches, then returns to the reader's choice", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");

  const group = page.locator('.filters .filter-group:has(legend:text-is("Publication"))');
  const box = group.getByRole("searchbox", { name: "Filter publications by name" });
  const adventures = group.locator('details.publication-group[data-tier="Adventures"]');

  // Adventures have a heading from the first view. Open it once to read a title, then close it,
  // so the reader's stored answer is "closed".
  await expect(adventures).toHaveCount(1);
  await expect(adventures).not.toHaveAttribute("open", "");
  await adventures.locator("summary").click();
  await expect(adventures).toHaveAttribute("open", "");
  const label = (await adventures.locator(".chip input").first().getAttribute("aria-label"))!;
  const title = label.replace(/, \d+ entr(y|ies)$/, "");
  const partial = title.slice(0, Math.min(title.length, 18));
  const match = adventures.getByRole("checkbox", { name: label, exact: true });
  await adventures.locator("summary").click();
  await expect(adventures).not.toHaveAttribute("open", "");

  // Typing a partial title opens Adventures so that the match is visible without a click.
  await box.click();
  await page.keyboard.type(partial);
  await expect(adventures).toHaveAttribute("open", "");
  await expect(match).toBeVisible();

  // Clearing the box returns to the stored choice, which is closed.
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.press("Backspace");
  await expect(box).toHaveValue("");
  await expect(adventures).not.toHaveAttribute("open", "");
  await expect(match).toBeHidden();

  // An Adventures group that the reader opened before typing stays open after the box clears.
  await adventures.locator("summary").click();
  await expect(adventures).toHaveAttribute("open", "");
  await box.click();
  await page.keyboard.type(partial);
  await expect(adventures).toHaveAttribute("open", "");
  await expect(match).toBeVisible();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.press("Backspace");
  await expect(box).toHaveValue("");
  await expect(adventures).toHaveAttribute("open", "");
  await expect(match).toBeVisible();
});

test("Equal relevance ranks the Rulebook entry before an Adventure entry", async ({ page }) => {
  // Two entries whose titles share the query word score exactly the same in the compiled
  // library. Their alphabetical order would put the Adventure Path entry first; the product-line
  // tier puts the Rulebook entry first.
  const rulebook = corpusEntry("rule", "Distracting Visitor");
  const adventure = corpusEntry("feat", "Distracting Explosion");
  const category = (entry: any) =>
    corpus.catalog.sources.find((source: any) => source.id === entry.source_ids[0])?.category;
  expect(category(rulebook), "the Rulebook fixture must come from an RPG book").toBe("RPG");
  expect(category(adventure), "the lower-tier fixture must come from an Adventure Path").toBe("Adventure Path");
  expect(adventure.title < rulebook.title, "the fixtures must tie in the opposite of tier order").toBe(true);
  // The tie holds only while the query word is in the titles and nowhere in the bodies. Search
  // matches a document word that starts with the query word, so "distraction" in a body does not
  // count, but "distracting" does: it adds body relevance, and then relevance decides instead of
  // the tier.
  // See also the native pin in crates/search/tests/canonical_ranking.rs.
  for (const entry of [rulebook, adventure]) {
    expect(
      bodyText(entry).toLowerCase(),
      `${entry.title} now has a word starting with "distracting" in its body, so the two no longer tie on relevance. Pick another fixture pair that scores exactly equal.`,
    ).not.toMatch(/\bdistracting/);
  }

  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("Distracting");
  await waitResults(page, "Distracting");
  await showAllResults(page);
  const titles = await hitTitles(page);
  expect(titles).toContain(rulebook.title);
  expect(titles).toContain(adventure.title);
  expect(titles.indexOf(rulebook.title)).toBeLessThan(titles.indexOf(adventure.title));
});

test("Leaving Search during a cold index load does not raise a page error", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fireball");
  // The query reaches the route after the typing pause, then the search starts reading and
  // decoding search.bin. Leave while that read is in flight; the guard fixture fails the test
  // if the abandoned task touches a disposed value.
  await expect.poll(() => page.url()).toContain("q=fireball");
  await page.goto("#/");
  await expect(page.locator("#home-title")).toBeVisible();
  await page.waitForTimeout(500);
});

installedTest("Chip counts keep their last value while a query is pending", async ({ page }) => {
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
  // The result region settles before every chip's reactive text is painted; the canonical
  // library has hundreds of chips, so wait for the new numbers instead of one frame.
  await expect.poll(countsOf).not.toEqual(before);
  expect((await widths()).length).toBe(widthsBefore.length);
});

test("Search operates entirely offline", async ({ page, context, server }) => {
  test.setTimeout(300_000);
  await installLibrary(page);
  await waitControlled(page);
  await goOffline(context, server);
  await page.reload();
  await expect(page.locator("#home-title")).toBeVisible();
  await expect(page.locator(".badge")).toBeVisible(); // the offline badge
  await server.clearLog();

  // Results are live: typing is enough, there is no submit.
  const spell = corpusEntry("spell", "Fireball");
  const prefix = spell.title.slice(0, 3).toLowerCase();
  await page.keyboard.press("/");
  await expect(page.locator("#search-input")).toBeFocused();
  await page.keyboard.type(prefix);
  await page.keyboard.press("Enter");
  await expect(page.locator(".search-page")).toBeVisible();
  await page.locator("#search-input").pressSequentially(spell.title.slice(prefix.length).toLowerCase());
  await waitResults(page, spell.title.toLowerCase());
  const titles = await hitTitles(page);
  expect(titles).toContain(spell.title);
  expect(titles.slice(0, 5).some(title => title.toLowerCase().startsWith(prefix))).toBe(true);
  await expect(page.locator(".results .hit-title mark").first()).toBeVisible();
  await expect(page.locator(".result-count")).toHaveAttribute("aria-live", "polite");

  await page.locator("#search-input").fill(spell.title);
  await waitResults(page, spell.title);
  expect((await hitTitles(page))[0]).toBe(spell.title);

  // Filters: choose a school and spell level the compiled data has, then compare the
  // grouped result set with the catalog's own grouping.
  const combinations = new Map<string, { school: string; level: number; titles: Set<string> }>();
  for (const entry of entries) {
    if (entry.kind !== "spell" || !entry.fields?.school) continue;
    for (const level of entry.fields.levels ?? []) {
      const key = `${entry.fields.school}\u0000${level.level}`;
      const group = combinations.get(key) ?? { school: entry.fields.school, level: level.level, titles: new Set() };
      group.titles.add(entry.title);
      combinations.set(key, group);
    }
  }
  const all = [...combinations.values()];
  const usable = all.filter((group) => /^[a-z]+$/.test(group.school));
  const chosen = (usable.filter((group) => group.titles.size <= 60).sort((a, b) => b.titles.size - a.titles.size)[0]
    ?? usable.sort((a, b) => b.titles.size - a.titles.size)[0]);
  const expectedGroups = [...chosen.titles].sort();
  expect(expectedGroups.length).toBeGreaterThan(1);
  await page.goto("#/search");
  await page.locator("#search-input").fill("");
  await page.locator(".filters .chip", { hasText: /^Spells/ }).click();
  const schoolChip = page.locator(".filters .chip", { hasText: new RegExp(`^${escape(chosen.school)}\\d*$`, "i") });
  await expect(schoolChip.first()).toBeVisible();
  await schoolChip.first().click();
  const levelChip = page.locator(".filter-group", { hasText: "Spell level" }).locator(".chip", { hasText: new RegExp(`^${chosen.level}\\d*$`) });
  await expect(levelChip.first()).toBeVisible();
  await levelChip.first().click();
  await expect.poll(() => page.url()).toContain(`spell_level:${chosen.level}`);
  await showAllResults(page);
  await expect(page.locator(".results > .hit")).toHaveCount(expectedGroups.length);
  expect((await page.locator(".results > .hit > .hit-link > .hit-title, .results > .hit > details > summary .hit-title").allInnerTexts()).sort()).toEqual(expectedGroups);
  // One line per result, as for titles above: a grouped title's per-publication rows sit
  // inside a closed <details>, and hidden elements report an empty innerText.
  const metas = await page
    .locator(".results > .hit > .hit-link > .hit-meta, .results > .hit > details > summary .hit-meta")
    .allInnerTexts();
  expect(metas).toHaveLength(expectedGroups.length);
  expect(metas.every((m) => /^Spell/.test(m)), metas.join(" | ")).toBe(true);

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
