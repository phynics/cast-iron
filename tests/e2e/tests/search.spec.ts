import type { Page } from "@playwright/test";
import { test, installedTest, expect, loadCorpus } from "./fixtures";
import { goOffline } from "./fixtures";
import { bodyText, bookOwners, corpusEntry, count, hitTitles, installLibrary, num, openBookGroups, openQuickSearch, pct, quickField, quickOptions, quickSearch, quickSearchFor, searchIcon, showAllResults, suggestionTitles, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The chip whose label starts with `label`, and the count it shows. */
const chipOf = (page: Page, label: RegExp) => page.locator(".filters .chip", { hasText: label });
const chipCount = async (page: Page, label: RegExp) =>
  num(await chipOf(page, label).locator(".chip-count").innerText());

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
  const school = page.locator(".filter-group", { hasText: "School" }).locator(".chip", { hasText: /^evocation[\d,]+$/i });
  const evocation = num(await school.locator(".chip-count").innerText());
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
    const n = num(await chip.locator(".chip-count").innerText());
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
  expect(firstPageCounts.every((text) => num(text) > 0)).toBe(true);

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
      count: Number(((input.parentElement?.querySelector(".chip-count") as HTMLElement | null)?.textContent ?? "0").replace(/,/g, "")),
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
  await expect(outside.locator("xpath=..").locator(".chip-count")).toHaveText(count(outsideFirstPage.count));

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

  // A close click while the box forces the tier open is not a new choice: it reopens the tier,
  // and clearing the box still returns to the reader's earlier open answer (#141).
  await box.click();
  await page.keyboard.type(partial);
  await expect(adventures).toHaveAttribute("open", "");
  await adventures.locator("summary").click();
  await expect(adventures).toHaveAttribute("open", "");
  await box.click();
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
  expect(before.every((text) => /^[\d,]+$/.test(text ?? ""))).toBe(true);
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
  await expect(quickField(page)).toBeFocused();
  await page.keyboard.type(prefix);
  await page.keyboard.press("Enter");
  await expect(page.locator(".search-page")).toBeVisible();
  await expect(page.locator("#search-input")).toHaveValue(prefix);
  // The app leaves the caret at the end of the handed-over text; Playwright's own focus in
  // WebKit moves it to the start, so go to the end before typing on.
  await page.locator("#search-input").press("End");
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

// ---- Quick search (#164) -----------------------------------------------------------------

installedTest("The header search icon opens Quick search, Enter shows the results, and there is one field", async ({ page }) => {
  await installLibrary(page);
  await expect(page.locator("#home-title")).toBeVisible();

  // The header offers an icon, not a field, and no search field exists until the layer opens.
  const icon = searchIcon(page);
  await expect(icon).toBeVisible();
  const box = (await icon.boundingBox())!;
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
  await expect(page.locator("header.app-header input")).toHaveCount(0);
  await expect(page.locator("input[type=search]:visible")).toHaveCount(0);

  const layer = await openQuickSearch(page);
  await expect(layer).toHaveAttribute("aria-modal", "true");
  await expect(page.locator("input[type=search]:visible")).toHaveCount(1);
  const field = quickField(page);
  await field.fill("grapple");
  await expect(layer.getByRole("button", { name: "Clear search" })).toBeVisible();
  await page.keyboard.press("Enter");
  await waitResults(page, "grapple");
  await expect(quickSearch(page)).toBeHidden();
  expect(page.url()).toContain("#/search?q=grapple");

  // The results page has the same single field, with no second field and no Search button.
  await expect(page.getByRole("searchbox", { name: "Search the rules" })).toHaveCount(1);
  await expect(page.locator("#search-input")).toHaveValue("grapple");
  await expect(page.locator(".search-page button[type=submit]")).toHaveCount(0);
  await expect(page.locator("header.app-header input")).toHaveCount(0);

  // Back from the results is the page that opened the layer, not a hidden layer.
  await page.goBack();
  await expect(page.locator("#home-title")).toBeVisible();
  await expect(quickSearch(page)).toBeHidden();
});

installedTest("Quick search clears with its x and ignores an empty Enter", async ({ page }) => {
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  const field = quickField(page);
  await page.keyboard.press("Enter");
  await expect(layer).toBeVisible();
  expect(page.url()).not.toContain("#/search");
  await field.fill("fire");
  await layer.getByRole("button", { name: "Clear search" }).click();
  await expect(field).toHaveValue("");
  await expect(field).toBeFocused();
  await expect(layer.getByRole("button", { name: "Clear search" })).toHaveCount(0);
});

installedTest("Quick search leaves no stale query in the header after searches and an entry", async ({ page }) => {
  await installLibrary(page);
  await quickSearchFor(page, "wizard discovery");
  for (const query of ["arcane discovery", "ancient red dragon"]) {
    await page.locator("#search-input").fill(query);
    await waitResults(page, query);
  }
  await page.locator(".results .hit-link").first().click();
  await expect(page.locator("h1").first()).toBeVisible();
  await expect(page.locator("header.app-header input")).toHaveCount(0);
  // Opening the layer again starts from an empty field, and Books shows no query either.
  const layer = await openQuickSearch(page);
  await expect(quickField(page)).toHaveValue("");
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();
  await page.goto("#/books");
  await expect(page.locator("header.app-header input")).toHaveCount(0);
});

installedTest("Cancel, Escape and Back close Quick search on the same page, scroll and focus", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await expect(page.locator(".book-directory-row").first()).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 600));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);
  const url = page.url();

  const closers: [string, () => Promise<void>][] = [
    ["Cancel", async () => { await quickSearch(page).getByRole("button", { name: "Cancel" }).click(); }],
    // With text in the field the first Escape closes the suggestion list, the second the layer.
    ["Escape", async () => { await page.keyboard.press("Escape"); await page.keyboard.press("Escape"); }],
    ["Back", async () => { await page.goBack(); }],
  ];
  for (const [name, close] of closers) {
    const layer = await openQuickSearch(page, { keyboard: true });
    await quickField(page).fill("fire");
    await close();
    await expect(layer, name).toBeHidden();
    expect(page.url(), name).toBe(url);
    expect(await page.evaluate(() => Math.round(window.scrollY)), name).toBe(before);
    await expect(searchIcon(page), name).toBeFocused();
  }
  // Closing three times leaves no layer behind: one Back leaves the page the layer was opened on.
  await page.goBack();
  expect(page.url()).not.toBe(url);
});

installedTest("A typed address closes Quick search, and Back restores the scroll offset and keeps both history fields", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await expect(page.locator(".book-directory-row").first()).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 600));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);
  const booksUrl = page.url();
  const state = () => page.evaluate(() => history.state as { scrollId?: number; quickSearch?: boolean } | null);

  // One entry for Books carries only the scroll id; the entry that Quick search pushes copies it
  // and adds its own flag.
  const first = await state();
  expect(typeof first?.scrollId).toBe("number");
  const layer = await openQuickSearch(page, { keyboard: true });
  const pushed = await state();
  expect(pushed).toEqual({ scrollId: first!.scrollId, quickSearch: true });

  // A typed address (a hash change the layer did not make) closes the layer on the new page.
  await page.evaluate(() => { location.hash = "#/settings"; });
  await expect(page.locator("[data-testid=library-facts]")).toBeVisible();
  await expect(layer).toBeHidden();

  // Back returns to the entry Quick search pushed: Books, at the saved offset, with both fields.
  await page.goBack();
  await expect(page.locator(".book-directory-row").first()).toBeVisible();
  expect(page.url()).toBe(booksUrl);
  await expect(layer).toBeHidden();
  await expect.poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before)).toBeLessThanOrEqual(2);
  expect(await state()).toEqual({ scrollId: first!.scrollId, quickSearch: true });

  // Back once more is the Books entry under it, with its own field untouched.
  await page.goBack();
  expect(page.url()).toBe(booksUrl);
  expect(await state()).toEqual({ scrollId: first!.scrollId });
  await expect.poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before)).toBeLessThanOrEqual(2);
});

installedTest("Quick search keeps Tab inside the layer and honors reduced motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  await quickField(page).fill("fire");
  expect(await layer.evaluate((el) => getComputedStyle(el).animationName)).toBe("none");
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press("Tab");
    expect(await page.evaluate(() => !!document.activeElement?.closest("dialog.quick-search"))).toBe(true);
  }
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press("Shift+Tab");
    expect(await page.evaluate(() => !!document.activeElement?.closest("dialog.quick-search"))).toBe(true);
  }
});

installedTest("The header icon on the results page focuses its one field", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  await page.locator("#search-input").blur();
  await searchIcon(page).click();
  await expect(page.locator("#search-input")).toBeFocused();
  await expect(quickSearch(page)).toBeHidden();
});

// ---- Quick search suggestions and recent searches (#165) ----------------------------------

installedTest("Quick search suggests entries while typing and opens one from the keyboard", async ({ page }) => {
  await installLibrary(page);
  const spell = corpusEntry("spell", "Fireball");
  const layer = await openQuickSearch(page);
  const field = quickField(page);
  await expect(field).toHaveAttribute("role", "combobox");
  await expect(field).toHaveAccessibleName("Search the rules");
  await field.fill("firebal");

  // The first suggestion is the spell, with its kind and book on a muted line.
  const options = quickOptions(page);
  await expect(options.first().locator(".quick-search-name")).toHaveText(spell.title);
  await expect(options.first().locator(".quick-search-meta")).toHaveText(/^Spell · \S/);
  const count = await options.count();
  expect(count).toBeGreaterThan(1);
  expect(count).toBeLessThanOrEqual(9);
  await expect(options.last()).toHaveText("See all results for “firebal”");
  for (const option of await options.all()) {
    expect(Math.round((await option.boundingBox())!.height)).toBeGreaterThanOrEqual(44);
  }
  await expect(field).toHaveAttribute("aria-expanded", "true");
  await expect(field).not.toHaveAttribute("aria-activedescendant", /./);
  await expect(layer.getByRole("listbox", { name: "Suggestions" })).toBeVisible();

  // `fireb` scores Firebrand and Fireball within one percent of each other; the suggestions
  // break that near tie by the shorter completion, so Fireball is first. `firebrand` is
  // Firebrand. Each step starts from a list with another first row, so a stale list cannot pass.
  const suggested = await suggestionTitles(page);
  expect(suggested[0]).toBe(spell.title);
  await field.fill("firebrand");
  await expect(options.last()).toHaveText("See all results for “firebrand”");
  await expect(options.first().locator(".quick-search-name")).toHaveText("Firebrand");
  await field.fill("fireb");
  await expect(options.last()).toHaveText("See all results for “fireb”");
  await expect(options.first().locator(".quick-search-name")).toHaveText(spell.title);
  await expect(options.first().locator(".quick-search-meta")).toHaveText(/^Spell · \S/);
  // Back to `firebal`: wait until the list no longer holds the Firebrand rows of `fireb`.
  await field.fill("firebal");
  await expect.poll(async () => (await suggestionTitles(page)).includes("Firebrand")).toBe(false);
  await expect(options.first().locator(".quick-search-name")).toHaveText(spell.title);
  await expect(options.last()).toHaveText("See all results for “firebal”");

  // Down highlights the first row and names it through aria-activedescendant.
  await page.keyboard.press("ArrowDown");
  await expect(options.first()).toHaveAttribute("aria-selected", "true");
  const id = await options.first().getAttribute("id");
  await expect(field).toHaveAttribute("aria-activedescendant", id!);
  await page.keyboard.press("ArrowUp");
  await expect(field).not.toHaveAttribute("aria-activedescendant", /./);
  await page.keyboard.press("ArrowUp");
  await expect(options.last()).toHaveAttribute("aria-selected", "true");
  for (let i = 1; i < count; i++) await page.keyboard.press("ArrowUp");
  await expect(options.first()).toHaveAttribute("aria-selected", "true");

  // Enter on a highlighted suggestion opens the entry, and Back returns to the page.
  const home = page.url();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  await expect(layer).toBeHidden();
  expect(page.url()).toContain(`#/entry/${spell.id}`);
  await page.goBack();
  expect(page.url()).toBe(home);
});

installedTest("Quick search suggestions match the results page and tap opens an entry", async ({ page }) => {
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  await quickField(page).fill("cure light");
  // Suggestions arrive after a pause in typing; until then only "See all results" is listed.
  await expect(quickOptions(page)).not.toHaveCount(1);
  await expect(quickOptions(page).last()).toHaveText("See all results for “cure light”");
  const suggested = await suggestionTitles(page);
  expect(suggested.length).toBeGreaterThan(0);
  // "See all results" shows the same entries, best first.
  await quickOptions(page).last().click();
  await waitResults(page, "cure light");
  const results = await hitTitles(page);
  expect(results.slice(0, suggested.length)).toEqual(suggested);
  expect(page.url()).toContain("#/search?q=cure%20light");

  // A tap on a suggestion opens the entry.
  await page.goto("#/");
  await openQuickSearch(page);
  await quickField(page).fill("cure light");
  await expect(quickOptions(page)).not.toHaveCount(1);
  const first = suggested[0];
  await quickOptions(page).first().click();
  await expect(page.getByRole("heading", { level: 1, name: first })).toBeVisible();
  await expect(layer).toBeHidden();
});

installedTest("Escape closes the suggestion list first, then Quick search", async ({ page }) => {
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  const field = quickField(page);
  await field.fill("fireb");
  await expect(quickOptions(page).first()).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(layer).toBeVisible();
  await expect(field).toHaveAttribute("aria-expanded", "false");
  await expect(quickOptions(page)).toHaveCount(0);
  // Typing again, or Down, brings the list back.
  await page.keyboard.press("ArrowDown");
  await expect(field).toHaveAttribute("aria-expanded", "true");
  await expect(quickOptions(page).first()).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();
});

installedTest("Quick search offers the starter searches until a search is submitted", async ({ page }) => {
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  await expect(layer.getByRole("heading", { name: "Try searching for" })).toBeVisible();
  await expect(quickOptions(page)).toHaveText(["fireball", "grapple", "cure light", "dragon"]);
  await expect(layer.getByRole("button", { name: "Clear", exact: true })).toHaveCount(0);
  await quickOptions(page).filter({ hasText: "grapple" }).click();
  await waitResults(page, "grapple");
  // A starter is a submitted search, so it is remembered.
  await page.goto("#/");
  await openQuickSearch(page);
  await expect(layer.getByRole("heading", { name: "Recent searches" })).toBeVisible();
  await expect(quickOptions(page)).toHaveText(["grapple"]);
});

installedTest("Recent searches persist offline, stay unique and newest first, and can be removed or cleared", async ({ page, context, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await quickSearchFor(page, "grapple");
  await quickSearchFor(page, "fireball");
  // A search from the results page's own field is remembered too.
  await page.locator("#search-input").fill("dragon");
  await waitResults(page, "dragon");
  await page.locator("#search-input").press("Enter");
  // The same query in another case moves to the front and does not repeat.
  await quickSearchFor(page, "Grapple");

  await goOffline(context, server);
  await page.goto("#/");
  await page.reload();
  await expect(page.locator("#home-title")).toBeVisible();
  const layer = await openQuickSearch(page);
  await expect(layer.getByRole("heading", { name: "Recent searches" })).toBeVisible();
  await expect(quickOptions(page)).toHaveText(["Grapple", "dragon", "fireball"]);

  // Remove one with its own button; the field keeps focus.
  await layer.getByRole("button", { name: "Remove “dragon” from recent searches" }).click();
  await expect(quickOptions(page)).toHaveText(["Grapple", "fireball"]);
  await expect(quickField(page)).toBeFocused();

  // A highlighted recent search is forgotten with Delete.
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Delete");
  await expect(quickOptions(page)).toHaveText(["Grapple"]);

  // The list survives closing and a reload, then Clear removes everything.
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();
  await page.reload();
  await expect(page.locator("#home-title")).toBeVisible();
  await openQuickSearch(page);
  await expect(quickOptions(page)).toHaveText(["Grapple"]);
  await layer.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(layer.getByRole("heading", { name: "Try searching for" })).toBeVisible();
  await expect(quickOptions(page)).toHaveText(["fireball", "grapple", "cure light", "dragon"]);

  // Tapping a recent search runs it.
  await page.keyboard.press("Escape");
  await quickSearchFor(page, "wizard");
  await page.goto("#/");
  await openQuickSearch(page);
  await quickOptions(page).first().click();
  await waitResults(page, "wizard");
  // The search was written to storage; read it back so no write is still in flight when the
  // test ends (WebKit reports "Context is stopped" for one that is).
  await page.goto("#/");
  await openQuickSearch(page);
  await expect(quickOptions(page)).toHaveText(["wizard"]);
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();
});

installedTest("The highlight is dropped when the suggestions change under it", async ({ page }) => {
  await installLibrary(page);
  await openQuickSearch(page);
  const field = quickField(page);
  const options = quickOptions(page);
  await field.fill("fire");
  await expect(options.first()).toBeVisible();
  // Move the highlight to the last row while the lookup for the new text is still pending
  // (suggestions wait for a pause in typing), so the rows change under it.
  await field.fill("fireball trap");
  for (let i = 0; i < 12; i++) await page.keyboard.press("ArrowDown");
  await expect(options.last()).toHaveText("See all results for “fireball trap”");
  await expect(options.first().locator(".quick-search-name")).toHaveText(/^Fireball Trap/);
  // Whatever happened in between, the field never names a row that is not in the list.
  const named = await field.getAttribute("aria-activedescendant");
  if (named) await expect(page.locator(`#${named}`)).toHaveCount(1);
  // Typing clears the highlight, and with none Enter searches the text rather than a row.
  await field.fill("fireball trap");
  await expect(field).not.toHaveAttribute("aria-activedescendant", /./);
  await page.keyboard.press("Enter");
  await waitResults(page, "fireball trap");
});

installedTest("Suggestions are announced only while the list is open, and the icon only promises a dialog where it opens one", async ({ page }) => {
  await installLibrary(page);
  await expect(searchIcon(page)).toHaveAttribute("aria-haspopup", "dialog");
  const layer = await openQuickSearch(page);
  const status = layer.getByRole("status");
  await quickField(page).fill("fireb");
  await expect(status).toHaveText(/^\d+ suggestions?$/);
  await page.keyboard.press("Escape");
  await expect(quickField(page)).toHaveAttribute("aria-expanded", "false");
  await expect(status).toHaveText("");
  await page.keyboard.press("ArrowDown");
  await expect(status).toHaveText(/^\d+ suggestions?$/);
  await page.keyboard.press("Enter");
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  await expect(searchIcon(page)).not.toHaveAttribute("aria-haspopup", /./);
});

// ---- Scoped Quick search from a list page (#193) -----------------------------------------

/**
 * A word that is in a few feat titles and in many titles of other kinds, so a feat-only search
 * and an everything search return visibly different lists.
 */
function sharedWord(): string {
  const wordsOf = (title: string) => new Set(title.toLowerCase().split(/[^a-z]+/).filter((word) => word.length >= 5));
  const feats = new Map<string, number>();
  const others = new Map<string, number>();
  for (const entry of entries) {
    for (const word of wordsOf(entry.title)) {
      const into = entry.kind === "feat" ? feats : others;
      into.set(word, (into.get(word) ?? 0) + 1);
    }
  }
  const candidates = [...feats.entries()]
    .filter(([word, n]) => n <= 3 && (others.get(word) ?? 0) >= 12)
    .sort((a, b) => (others.get(b[0]) ?? 0) - (others.get(a[0]) ?? 0));
  if (!candidates.length) throw new Error("The compiled corpus has no word shared by a few feats and many other entries.");
  return candidates[0][0];
}

const scopePill = (page: Page, name = "Feats") => page.getByRole("button", { name: `Remove scope: ${name}`, exact: true });

installedTest("The header icon on a list page opens Quick search scoped to that kind, and one tap removes the scope", async ({ page }) => {
  await installLibrary(page);
  const word = sharedWord();
  await page.goto("#/browse/kind/feat");
  await expect(page.getByRole("heading", { level: 1, name: "Feats" })).toBeVisible();

  const layer = await openQuickSearch(page);
  const field = quickField(page);
  await expect(field).toHaveAttribute("placeholder", "Search feats");
  await expect(field).toHaveAttribute("aria-label", "Search feats");
  const pill = scopePill(page);
  await expect(pill).toBeVisible();
  await expect(layer.getByRole("group", { name: "Search scope" })).toContainText("Feats");
  const box = (await pill.boundingBox())!;
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);

  // The suggestions honor the scope: every row is a feat.
  await field.fill(word);
  await expect(quickOptions(page).last()).toHaveText(`See all results for “${word}”`);
  await expect.poll(async () => (await suggestionTitles(page)).length).toBeGreaterThan(0);
  const metas = await layer.locator('[role="option"]:not(.quick-search-all) .quick-search-meta').allInnerTexts();
  expect(metas.length).toBeGreaterThan(0);
  for (const meta of metas) expect(meta).toMatch(/^Feat · /);

  // Removing the scope keeps the text, returns focus to the field, and widens the suggestions.
  await pill.click();
  await expect(pill).toHaveCount(0);
  await expect(field).toHaveValue(word);
  await expect(field).toBeFocused();
  await expect(field).toHaveAttribute("placeholder", "Search spells, feats, rules…");
  await expect(field).toHaveAttribute("aria-label", "Search the rules");
  await expect.poll(async () =>
    (await layer.locator('[role="option"]:not(.quick-search-all) .quick-search-meta').allInnerTexts()).some((meta) => !meta.startsWith("Feat · ")),
  ).toBe(true);

  // Enter now searches everything.
  await page.keyboard.press("Enter");
  await waitResults(page, word);
  expect(page.url()).not.toContain("kind=");
  await expect(scopePill(page)).toHaveCount(0);
  await expect.poll(async () => (await page.locator(".results .hit-meta").allInnerTexts()).some((meta) => !meta.startsWith("Feat"))).toBe(true);
});

installedTest("Enter in a scoped Quick search keeps the scope on the results page, where it can be removed", async ({ page }) => {
  await installLibrary(page);
  const word = sharedWord();
  await page.goto("#/browse/kind/feat");
  await openQuickSearch(page);
  await quickField(page).fill(word);
  await page.keyboard.press("Enter");
  await waitResults(page, word);
  await expect(quickSearch(page)).toBeHidden();
  expect(page.url()).toContain(`#/search?q=${word}&kind=feat`);

  // Every result is a feat, the scope shows, and the Feats choice is pressed.
  const metas = await page.locator(".results .hit-meta").allInnerTexts();
  expect(metas.length).toBeGreaterThan(0);
  for (const meta of metas) expect(meta).toMatch(/^Feat/);
  const pill = scopePill(page);
  await expect(pill).toBeVisible();
  await expect(page.getByRole("button", { name: "Feats", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".search-page").getByRole("button", { name: /^Remove scope/ })).toHaveCount(1);

  // One tap removes it: the text stays, the URL loses the kind, and the results widen.
  await pill.click();
  await expect(pill).toHaveCount(0);
  await expect.poll(() => new URL(page.url()).hash).toBe(`#/search?q=${word}`);
  await waitResults(page, word);
  await expect(page.locator("#search-input")).toHaveValue(word);
  await expect(page.getByRole("button", { name: "All entries", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("button", { name: "All entries", exact: true })).toBeFocused();
  await expect.poll(async () => (await page.locator(".results .hit-meta").allInnerTexts()).some((meta) => !meta.startsWith("Feat"))).toBe(true);
});

installedTest("A scope that the Feats choice does not offer still shows and removes on the results page", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/trait");
  const layer = await openQuickSearch(page);
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search traits");
  await expect(scopePill(page, "Traits")).toBeVisible();
  await layer.getByRole("button", { name: "Remove scope: Traits" }).click();
  await expect(scopePill(page, "Traits")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();

  await page.goto("#/search?q=bully&kind=trait");
  await waitResults(page, "bully");
  await expect(scopePill(page, "Traits")).toBeVisible();
  await scopePill(page, "Traits").click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/search?q=bully");
});

installedTest("Back from a scoped search returns to the list, and the scope follows the page, not a memory", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const listHash = new URL(page.url()).hash;
  const layer = await openQuickSearch(page);
  await quickField(page).fill("grapple");
  await page.keyboard.press("Enter");
  await waitResults(page, "grapple");
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe(listHash);
  await expect(layer).toBeHidden();

  // Escape closes the layer on the same list page; the scope is read again at the next opening.
  await openQuickSearch(page);
  await expect(scopePill(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();
  expect(new URL(page.url()).hash).toBe(listHash);
  await page.goto("#/");
  await openQuickSearch(page);
  await expect(scopePill(page)).toHaveCount(0);
  await expect(quickSearch(page).getByRole("group", { name: "Search scope" })).toHaveCount(0);
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search spells, feats, rules…");
  // Outside a list page a search is unscoped.
  await quickField(page).fill("grapple");
  await page.keyboard.press("Enter");
  await waitResults(page, "grapple");
  expect(page.url()).not.toContain("kind=");
});

installedTest("The / shortcut on a list page opens the same scoped Quick search", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/spell");
  await expect(page.getByRole("heading", { level: 1, name: "Spells" })).toBeVisible();
  await page.locator("main").focus();
  await page.keyboard.press("/");
  await expect(quickSearch(page)).toBeVisible();
  await expect(scopePill(page, "Spells")).toBeVisible();
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search spells");
});

installedTest("On a filtered feat list Quick search is scoped to the facet, and the results line names the scope", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat?facet=feat_type:Combat");
  await expect(page.getByRole("button", { name: "Type: Combat" })).toBeVisible();
  const layer = await openQuickSearch(page);
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search Combat feats");
  await expect(quickField(page)).toHaveAttribute("aria-label", "Search Combat feats");
  await expect(scopePill(page, "Combat feats")).toBeVisible();
  await quickField(page).fill("grapple");
  await page.keyboard.press("Enter");
  await waitResults(page, "grapple");
  expect(new URL(page.url()).hash).toBe("#/search?q=grapple&kind=feat&facet=feat_type:Combat");

  // The status line says where the results are from, and one control widens it to the kind.
  const status = page.locator(".result-count");
  await expect(status).toHaveText(/^(Top [\d,]+ of )?[\d,]+ results? in Combat feats.* · Show all feats/);
  await expect(status.getByRole("button", { name: "Show all feats" })).toBeVisible();
  await status.getByRole("button", { name: "Show all feats" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/search?q=grapple&kind=feat");
  await waitResults(page, "grapple");
  // A kind alone has nothing to widen: the line names the scope and offers no Show all.
  await expect(status).toHaveText(/^(Top [\d,]+ of )?[\d,]+ results? in Feats(?!.*Show all)/);
  await expect(status.getByRole("button")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Feats", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(layer).toBeHidden();
});

installedTest("A recent search remembers its scope: the row shows it, tapping restores it, and the same words in another scope are another row", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  // "grapple" in Combat feats, in Feats, and everywhere.
  await page.goto("#/browse/kind/feat?facet=feat_type:Combat");
  await expect(page.getByRole("button", { name: "Type: Combat" })).toBeEnabled();
  await openQuickSearch(page);
  await quickField(page).fill("grapple");
  await page.keyboard.press("Enter");
  await waitResults(page, "grapple");
  await page.goto("#/browse/kind/feat");
  await openQuickSearch(page);
  await quickField(page).fill("grapple");
  await page.keyboard.press("Enter");
  await waitResults(page, "grapple");
  await quickSearchFor(page, "grapple");
  await page.goto("#/");
  await page.reload();
  const layer = await openQuickSearch(page);
  await expect(layer.getByRole("heading", { name: "Recent searches" })).toBeVisible();
  await expect(quickOptions(page)).toHaveCount(3);
  const rows = await quickOptions(page).evaluateAll((els) => els.map((el) => ({
    name: el.querySelector(".quick-search-name")?.textContent,
    scope: el.querySelector(".quick-search-meta")?.textContent ?? null,
  })));
  expect(rows).toEqual([
    { name: "grapple", scope: null },
    { name: "grapple", scope: "Feats" },
    { name: "grapple", scope: "Combat feats" },
  ]);

  // Tapping the scoped row opens the results in that scope, from a page that has none.
  await quickOptions(page).nth(2).click();
  await waitResults(page, "grapple");
  expect(new URL(page.url()).hash).toBe("#/search?q=grapple&kind=feat&facet=feat_type:Combat");
  await expect(scopePill(page, "Combat feats")).toBeVisible();
  // It moved to the front; the three rows stay three.
  await page.goto("#/");
  await openQuickSearch(page);
  await expect(quickOptions(page)).toHaveCount(3);
  await expect(quickOptions(page).first().locator(".quick-search-meta")).toHaveText("Combat feats");
  // Removing one takes out that scope only.
  await layer.getByRole("button", { name: "Remove “grapple in Feats” from recent searches" }).click();
  await expect(quickOptions(page)).toHaveCount(2);
  await page.keyboard.press("Escape");
  await expect(layer).toBeHidden();
});

test("An inactive first source is replaced by the active book that still shows the entry", async ({ page }) => {
  const owners = bookOwners();
  const sourceTitle = (id: string) =>
    corpus.catalog.sources.find((source: any) => source.id === id)?.title as string | undefined;
  const candidate = entries.find((entry: any) => {
    const owner = owners.get(entry.id);
    return (
      owner &&
      entry.source_ids.length > 1 &&
      entry.source_ids[0] !== owner &&
      sourceTitle(entry.source_ids[0]) &&
      sourceTitle(owner)
    );
  });
  expect(candidate, "a multi-source entry whose book placement differs from its first source").toBeTruthy();
  const entry = candidate!;
  const owner = owners.get(entry.id)!;
  const first = entry.source_ids[0] as string;
  const firstTitle = sourceTitle(first)!;
  const ownerTitle = sourceTitle(owner)!;

  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill(entry.title);
  await waitResults(page, entry.title);
  await showAllResults(page);
  const row = page.locator(`.results a.hit-link[href^="#/entry/${entry.id}"]`).first();
  await expect(row).toBeVisible();
  // With every book active the compiler's first source still names the entry.
  await expect(row.locator(".hit-meta")).toHaveText(new RegExp(`· ${escape(firstTitle)}$`));

  // Turning off only that attribution source leaves the placement book active, so the entry
  // stays eligible and its label must name the active book instead.
  await page.goto("#/books/active");
  await openBookGroups(page);
  const toggle = page.locator(`.book-directory-item[data-book-id="${first}"]`)
    .getByRole("checkbox", { name: `Active ${firstTitle}`, exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  await page.goto("#/search");
  await page.locator("#search-input").fill(entry.title);
  await waitResults(page, entry.title);
  await showAllResults(page);
  const updated = page.locator(`.results a.hit-link[href^="#/entry/${entry.id}"]`).first();
  await expect(updated.locator(".hit-meta")).toHaveText(new RegExp(`· ${escape(ownerTitle)}$`));
  await updated.click();
  await expect(page.locator(".entry-meta .meta-link").nth(1)).toHaveText(ownerTitle);
});

test("Search names hidden matches in an inactive book and includes it in one tap", async ({ page }) => {
  const dragon = corpusEntry("creature", "Ancient Red Dragon");
  const bookId = bookOwners().get(dragon.id) ?? dragon.source_ids[0];
  const book = corpus.catalog.sources.find((source: any) => source.id === bookId)!;
  expect(book, "the dragon's placement book is in the catalog").toBeTruthy();

  await installLibrary(page);
  await page.goto("#/books/active");
  await openBookGroups(page);
  const toggle = page.locator(`.book-directory-item[data-book-id="${bookId}"]`)
    .getByRole("checkbox", { name: `Active ${book.title}`, exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  await page.goto("#/search");
  await page.locator("#search-input").fill("ancient red dragon");
  await waitResults(page, "ancient red dragon");
  const hint = page.locator(".inactive-books-hint");
  await expect(hint).toBeVisible();
  await expect(hint).toContainText(`inactive books (${book.title})`);
  await expect(hint.getByRole("button", { name: `Include ${book.title}`, exact: true })).toBeVisible();

  // One tap includes the book and the same search now finds the dragon.
  await hint.getByRole("button", { name: `Include ${book.title}`, exact: true }).click();
  await expect(page.locator(`.results a[href^="#/entry/${dragon.id}"]`)).toBeVisible();
  await expect(hint).toHaveCount(0);
});
