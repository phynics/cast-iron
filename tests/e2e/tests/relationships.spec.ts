import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { bookReadingOrder, bookTrees, firstBookPlacement, installLibrary, listedChapterTitles, openBookContents, openAllRows, openBookEntry, waitControlled } from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
/** The text of inline nodes, like `plain_text` in the client. */
const plainText = (inlines: any[] = []): string =>
  inlines.map((inline) => (inline.t === "text" ? inline.v : plainText(inline.c))).join("");
const related = (entry: (typeof entries)[number], kind: string) =>
  entry.relations?.filter((r: any) => r.kind === kind) ?? [];

test.beforeEach(async ({ page, context }) => {
  await installLibrary(page);
  await waitControlled(page);
  await context.setOffline(true);
});

test("race racial traits and class archetypes link in both directions", async ({ page }) => {
  const trait = entries.find((e) => e.kind === "racial_trait" && related(e, "part").length > 0)!;
  const traitId = trait.id;
  const race = corpus.entries.get(related(trait, "part")[0].target)!;
  await page.goto(`#/entry/${race.id}`);
  await expect(page.getByRole("heading", { name: race.title, level: 1 })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Racial traits/ })).toBeVisible();
  // A race can list one trait from two books, so pick the link by the exact entry, not by its title.
  await openAllRows(page.locator(".entry-rel"));
  await page.locator(`.entry-rel a[href="#/entry/${traitId}"]`).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${traitId.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("heading", { name: trait.title, level: 1 })).toBeVisible();
  await expect(page.getByRole("heading", { name: /racial trait of/i })).toBeVisible();
  await page.locator(`.entry-rel a[href="#/entry/${race.id}"]`).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${race.id.replaceAll("/", "\\/")}$`));

  const classEntry = entries.find((e) => e.kind === "class" &&
    entries.some((a) => a.kind === "archetype" && related(a, "archetype").some((r: any) => r.target === e.id)))!;
  const archetype = entries.find((a) => a.kind === "archetype" && related(a, "archetype").some((r: any) => r.target === classEntry.id))!;
  await page.goto(`#/entry/${classEntry.id}`);
  // The class page lists its archetypes in the Archetypes section (#170, part D).
  await page.locator(".arch-list").waitFor();
  await page.locator(`.arch-list a[href="#/entry/${archetype.id}"]`).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${archetype.id.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("heading", { name: /An archetype of/ })).toBeVisible();
  await page.locator(`.entry-rel a[href="#/entry/${classEntry.id}"]`).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${classEntry.id.replaceAll("/", "\\/")}$`));
});

test("spell lists filter by class, level, and query; search supports source filters", async ({ page }) => {
  const classLevels = new Map<string, number>();
  for (const entry of entries) {
    for (const level of entry.fields?.levels ?? []) classLevels.set(level.class, (classLevels.get(level.class) ?? 0) + 1);
  }
  const listClass = [...classLevels.entries()].sort((a, b) => b[1] - a[1])[0][0];
  const spell = entries.find((entry) => entry.kind === "spell" &&
    entry.fields?.levels?.some((level: any) => level.class === listClass))!;
  const level = spell.fields.levels.find((level: any) => level.class === listClass).level;
  await page.goto(`#/spells/${encodeURIComponent(listClass)}`);
  await expect(page.getByRole("heading", { name: `${listClass} spells`, level: 1 })).toBeVisible();
  await page.locator("#list-filter").fill(spell.title);
  await expect(page.locator(`.spell-rows a[href="#/entry/${spell.id}"]`)).toBeVisible();
  const matching = entries.filter((entry) => entry.kind === "spell" &&
    entry.title.toLowerCase().includes(spell.title.toLowerCase()) &&
    entry.fields?.levels?.some((level: any) => level.class === listClass));
  await expect(page.locator('.spell-rows a[href^="#/entry/"]')).toHaveCount(matching.length);
  // The name field is route state (#581): typing replaces the history entry, so the typed
  // states add none.
  const hash = () => decodeURIComponent(new URL(page.url()).hash);
  await expect.poll(hash).toContain(`name=${spell.title}`);
  // The write to the address leaves focus in the field.
  await expect(page.locator("#list-filter")).toBeFocused();
  const narrowedCount = (await page.locator(".list-count").innerText()).trim();
  expect(narrowedCount).toMatch(/ of /);
  // Back from a spell shows the typed name and the narrowed count.
  await page.locator(`.spell-rows a[href="#/entry/${spell.id}"]`).click();
  await expect(page).toHaveURL(/#\/entry\//);
  await page.goBack();
  await expect(page.locator("#list-filter")).toHaveValue(spell.title);
  await expect(page.locator(".list-count")).toHaveText(narrowedCount);
  // A school chosen in the filter menu keeps the name.
  const schoolMenu = page.getByRole("button", { name: /^School: / });
  await schoolMenu.click();
  await page.locator(`#${await schoolMenu.getAttribute("aria-controls")}`).getByRole("checkbox").first().check();
  await expect.poll(hash).toMatch(/school=/);
  expect(hash()).toContain(`name=${spell.title}`);
  // A level link, clicked, keeps the name and the school.
  await page.locator(`a.lvl[aria-label^="${level === 0 ? "Cantrips" : `Level ${level}`},"]`).click();
  await expect.poll(hash).toContain(`/${level}?`);
  expect(hash()).toContain(`name=${spell.title}`);
  expect(hash()).toMatch(/school=/);
  await expect(page.locator("#list-filter")).toHaveValue(spell.title);
  const entriesBefore = await page.evaluate(() => history.length);
  await page.locator("#list-filter").fill("no-such-spell");
  await expect(page.getByRole("status").filter({ hasText: /No spell on this list matches/ })).toBeVisible();
  await expect(page).toHaveURL(/name=no-such-spell/);
  expect(await page.evaluate(() => history.length)).toBe(entriesBefore);
  // A reload reads the name back from the address.
  await page.reload();
  await expect(page.locator("#list-filter")).toHaveValue("no-such-spell");
  await expect(page.getByRole("status").filter({ hasText: /No spell on this list matches/ })).toBeVisible();
  // Show all clears the name, and Back returns to the narrowed list.
  await page.getByRole("link", { name: "Show all spells" }).click();
  await expect(page).not.toHaveURL(/name=/);
  await expect(page.locator("#list-filter")).toHaveValue("");
  await page.goBack();
  await expect(page).toHaveURL(/name=no-such-spell/);
  await expect(page.locator("#list-filter")).toHaveValue("no-such-spell");

  await page.goto(`#/spells/${encodeURIComponent(listClass)}/${level}`);
  const heading = level === 0 ? `${listClass} cantrips` : `${listClass} spells, level ${level}`;
  await expect(page.getByRole("heading", { name: heading, level: 1 })).toBeVisible();
  await expect(page.locator(`.spell-rows a[href="#/entry/${spell.id}"]`)).toBeVisible();
  await expect(page.locator(".level-group")).toHaveAttribute("data-level", String(level));

  const stub = corpus.catalog.entries.find((candidate) => candidate.id === spell.id)!;
  await page.goto(`#/search?q=${encodeURIComponent(spell.title)}&kind=spell&source=${encodeURIComponent(stub.source)}`);
  await expect(page.locator(".results-region")).toHaveAttribute("data-query", spell.title);
  await expect(page.locator(".results .hit").first()).toBeVisible();
  await expect(page.locator(".results .hit-title").filter({ hasText: new RegExp(`^${spell.title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) }).first()).toBeVisible();
  const resultHrefs = await page.locator(".results .hit-link").evaluateAll((nodes) => nodes.map((n) => (n as HTMLAnchorElement).hash));
  expect(resultHrefs.length).toBeGreaterThan(0);
  for (const href of resultHrefs) {
    const id = href.replace(/^#\/entry\//, "").split("?")[0];
    expect(corpus.catalog.entries.find((e) => e.id === id)?.source).toBe(stub.source);
  }
});

test("numbered spell series list each member at its own level (#402)", async ({ page }) => {
  const sna = (n: number) => `ref/spell/summon-natures-ally-${n}`;
  const sm = (n: number) => `ref/spell/summon-monster-${n}`;
  const levelOf = (id: string, cls: string) =>
    corpus.entries.get(id)!.fields.levels.find((row: any) => row.class === cls)?.level;
  // The compiled corpus must already hold the printed levels, not the base spell's.
  expect(levelOf(sna(2), "Druid")).toBe(2);
  expect(levelOf(sna(5), "Druid")).toBe(5);
  expect(levelOf(sna(9), "Druid")).toBe(9);
  expect(levelOf(sm(2), "Wizard")).toBe(2);

  // A spell row at a level, selected by the exact entry, since titles repeat across books.
  const row = (level: number, id: string) =>
    page.locator(`.level-group[data-level="${level}"] .spell-rows a[href="#/entry/${id}"]`);
  const openLevel = async (cls: string, level: number) => {
    await page.goto(`#/spells/${cls}/${level}`);
    await expect(page.getByRole("heading", { name: `${cls} spells, level ${level}`, level: 1 })).toBeVisible();
  };

  // Druid: each Summon Nature's Ally sits at its own level only.
  await openLevel("Druid", 1);
  await expect(row(1, sna(1))).toHaveCount(1);
  for (const n of [2, 5, 9]) await expect(row(1, sna(n))).toHaveCount(0);
  await openLevel("Druid", 2);
  await expect(row(2, sna(2))).toHaveCount(1);
  await openLevel("Druid", 5);
  await expect(row(5, sna(5))).toHaveCount(1);
  await openLevel("Druid", 9);
  await expect(row(9, sna(9))).toHaveCount(1);

  // Wizard: Summon Monster II sits at level 2 and not at level 1.
  await openLevel("Wizard", 1);
  await expect(row(1, sm(1))).toHaveCount(1);
  await expect(row(1, sm(2))).toHaveCount(0);
  await openLevel("Wizard", 2);
  await expect(row(2, sm(2))).toHaveCount(1);

  // The base spell's Variants panel names each variant with its own levels.
  await page.goto(`#/entry/${sna(1)}`);
  const section = page.locator(".rel-variants");
  await expect(section.getByRole("heading", { name: "Variants", level: 2 })).toBeVisible();
  for (const [id, level] of [[sna(2), 2], [sna(5), 5], [sna(9), 9]] as const) {
    const variant = section.locator(".rel-list li").filter({ has: page.locator(`a[href="#/entry/${id}"]`) });
    await expect(variant).toHaveCount(1);
    await expect(variant.locator(".rel-meta")).toHaveText(new RegExp(`^Druid ${level}(,| ·)`));
  }
});

test("feat pages preserve printed prerequisites and label alternatives honestly", async ({ page }) => {
  const dependedOn = new Set(entries.flatMap((e) => related(e, "prerequisite").map((r: any) => r.target)));
  const feat = entries.find((e) => e.kind === "feat" && e.fields.prerequisites &&
    related(e, "prerequisite").some((r: any) => r.label === "or") && dependedOn.has(e.id))!;
  expect(feat, "a feat with an either-or prerequisite that other feats name").toBeTruthy();
  await page.goto(`#/entry/${feat.id}`);
  await expect(page.locator("#rel-prereq")).toHaveText("Requirements");
  // The body prints the `Prerequisites` line once. The panel repeats it only when the body has none.
  const printsPrerequisites = feat.body.some((block: any) =>
    (block.t === "paragraph" && block.content?.some((inline: any) =>
      inline.t === "strong" && plainText(inline.c).trimStart().startsWith("Prerequisite"))) ||
    (block.t === "heading" && plainText(block.content).trimStart().startsWith("Prerequisite")));
  if (printsPrerequisites) await expect(page.locator(".prereq-text")).toHaveCount(0);
  else await expect(page.locator(".prereq-text")).toContainText(feat.fields.prerequisites);
  await expect(page.getByText("Feats it names", { exact: true })).toBeVisible();
  const expected = related(feat, "prerequisite");
  for (const edge of expected) {
    const target = corpus.entries.get(edge.target)!;
    const link = page.locator(`.rel-prereq a[href="#/entry/${target.id}"]`);
    await expect(link).toBeVisible();
    if (edge.label === "or") await expect(link.locator("xpath=..")).toContainText("or");
  }
  await expect(page.getByRole("heading", { name: /Leads to/ })).toBeVisible();

  const chain = page.locator(".entry-rel .chain").first();
  if (await chain.count()) {
    const labels = await chain.getByRole("link").allInnerTexts();
    expect(labels.length).toBeGreaterThan(0);
  }
});

test("every feat chain Open the feat link has a 24 px hit area", async ({ page }) => {
  await page.goto("#/feats/chains");
  const heads = page.locator(".chain-head details");
  await expect(heads.first()).toBeVisible();
  const headCount = await heads.count();
  expect(headCount).toBeGreaterThan(0);

  // Every link lives inside its chain's disclosure. Open them all, then measure each one.
  await page.evaluate(() => {
    for (const details of document.querySelectorAll<HTMLDetailsElement>(".chain-head details")) details.open = true;
  });
  const heights = await page
    .locator(".chain-open a")
    .evaluateAll((links) => links.map((link) => link.getBoundingClientRect().height));
  expect(heights).toHaveLength(headCount);
  expect(heights.filter((height) => height < 24)).toEqual([]);
});

test("category hub exposes requested reference areas and dedicated lenses", async ({ page }) => {
  await page.goto("#/browse");
  const names = ["Spells", "Feats", "Classes", "Prestige classes", "Races", "Traits", "Items", "Creatures"];
  for (const name of names) await expect(page.getByRole("link", { name: new RegExp(name) }).first()).toBeVisible();
  await page.goto("#/browse/kind/creature");
  await expect(page.getByRole("heading", { name: "Creatures", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li").first()).toBeVisible();
  await page.goto("#/browse/kind/trait");
  await expect(page.getByRole("heading", { name: "Traits", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li").first()).toBeVisible();
  for (const [slug, title] of [["item", "Items"], ["prestige_class", "Prestige classes"]]) {
    await page.goto(`#/browse/kind/${slug}`);
    await expect(page.getByRole("heading", { name: title, level: 1 })).toBeVisible();
    await expect(page.locator(slug === "rule" ? ".tree" : ".entry-list li").first()).toBeVisible();
  }
  await page.goto("#/feats/chains");
  await expect(page.getByRole("heading", { name: "Feat chains", level: 1 })).toBeVisible();
  await expect(page.locator(".chain-heads .chain-head").first()).toBeVisible();
});

test("book contents keep their order and highlight a chapter path without disclosures", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => chapter.children?.length))!;
  await page.goto("#/browse/books");
  await openBookContents(page, book.title);
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${book.title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`) })).toBeVisible();
  await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();
  const tocTitles = await page.locator(".toc-top > li").evaluateAll((items) => items.map((item) => {
    const branch = item.querySelector(":scope > .toc-row .toc-title");
    const leaf = item.querySelector(":scope > a");
    return (branch?.textContent ?? leaf?.textContent ?? "").trim();
  }));
  expect(tocTitles).toEqual(listedChapterTitles(book));
  // Every branch is expanded; nothing on the page is a disclosure.
  await expect(page.locator(".toc [aria-expanded], .toc details, .toc summary")).toHaveCount(0);
  await expect(page.locator(".toc-top > .toc-branch > .toc-list > li").first()).toBeVisible();

  // A chapter path in the address still marks the row it names, and Back walks the history.
  const selectedUrl = `${page.url()}?c=1`;
  await page.goto(selectedUrl);
  await expect(page.locator("#toc-1")).toHaveAttribute("aria-current", "location");
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).not.toContain("?c=");
  await page.goForward();
  await expect(page.locator("#toc-1")).toHaveAttribute("aria-current", "location");
});

test("a book bar steps from a contents entry through the book, and Back returns", async ({ page }) => {
  // Pick a rules entry that Contents lists, with a neighbour on each side in the same book and
  // whose book bar follows that book (the first placement).
  let found: { book: ReturnType<typeof bookTrees>[number]; order: string[]; index: number } | undefined;
  for (const book of bookTrees()) {
    const order = bookReadingOrder(book);
    for (let index = 1; index < order.length - 1; index++) {
      const entry = corpus.entries.get(order[index]);
      if (entry?.kind !== "rule") continue;
      if (firstBookPlacement(entry.id)?.source !== book.source) continue;
      found = { book, order, index };
      break;
    }
    if (found) break;
  }
  if (!found) throw new Error("The compiled corpus has no book with a rules entry between two neighbours.");
  const { book, order, index } = found;
  const current = corpus.entries.get(order[index])!;
  const previous = corpus.entries.get(order[index - 1])!;
  const next = corpus.entries.get(order[index + 1])!;

  // Open the entry from the contents, not by typing its address.
  await openBookEntry(page, book, current.id);
  await expect(page.getByRole("heading", { level: 1, name: current.title, exact: true })).toBeVisible();
  await expect(page.getByTestId("book-bar")).toBeVisible();

  // Next follows the book's contents order, and the next page names this entry as Previous.
  const nextLink = page.locator('.book-steps a[rel="next"]');
  await expect(nextLink).toContainText(next.title);
  await nextLink.click();
  await expect(page.getByRole("heading", { level: 1, name: next.title, exact: true })).toBeVisible();
  await expect(page.locator('.book-steps a[rel="prev"]')).toContainText(current.title);
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`#/entry/${current.id.replaceAll("/", "\\/")}(\\?book=[^&]+)?$`));
  await expect(page.getByRole("heading", { level: 1, name: current.title, exact: true })).toBeVisible();

  // Previous walks the other way, and Back returns to this entry again.
  const prevLink = page.locator('.book-steps a[rel="prev"]');
  await expect(prevLink).toContainText(previous.title);
  await prevLink.click();
  await expect(page.getByRole("heading", { level: 1, name: previous.title, exact: true })).toBeVisible();
  await expect(page.locator('.book-steps a[rel="next"]')).toContainText(current.title);
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`#/entry/${current.id.replaceAll("/", "\\/")}(\\?book=[^&]+)?$`));
  await expect(page.getByRole("heading", { level: 1, name: current.title, exact: true })).toBeVisible();

  // The neighbours are adjacent in the contents order, which is what the bar stepped through.
  expect(order.indexOf(previous.id) + 1).toBe(index);
  expect(order.indexOf(next.id) - 1).toBe(index);
});
