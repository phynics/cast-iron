import type { Locator, Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus, goOffline } from "./fixtures";
import { groupLetter, latestPrintingGroups, memberBooks, installLibrary, openBookGroups, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();
/**
 * A title the library publishes more than once under distinct books, other than the
 * books-only Rule kind. The library repeats titles that differ by corpus version, so the
 * group comes from the compiled catalog rather than a literal name.
 */
const BROWSABLE = new Set(["spell", "feat", "trait", "item", "creature", "class", "race",
  "prestige_class", "affliction", "archetype", "skill", "trap", "racial_trait"]);
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
type Stub = (typeof corpus.catalog.entries)[number];
const stubTitle = (id: string) => corpus.catalog.entries.find((stub: Stub) => stub.id === id)?.title as string;
/**
 * What the list calls a row's parent (#575): a racial trait's race and an archetype's class from
 * their relation, an affliction's parent from the catalog. A fold joins only entries of one kind,
 * title and parent.
 */
const parentIds = (stub: Stub): string[] => {
  const relation = stub.kind === "racial_trait" ? "part" : stub.kind === "archetype" ? "archetype" : null;
  if (relation) {
    return ((corpus.entries.get(stub.id)?.relations ?? []) as any[])
      .filter((r) => r.kind === relation)
      .map((r) => r.target as string)
      .sort();
  }
  return stub.kind === "affliction" && stub.parent ? [stub.parent] : [];
};
const parentTitle = (stub: Stub) => parentIds(stub).map(stubTitle).sort().join(", ");
const groups = new Map<string, typeof corpus.catalog.entries>();
for (const stub of corpus.catalog.entries) {
  if (!BROWSABLE.has(stub.kind)) continue;
  const key = `${stub.kind}\u0000${stub.title}\u0000${parentIds(stub).join(",")}`;
  groups.set(key, [...(groups.get(key) ?? []), stub]);
}
const variants = [...groups.values()].find((stubs) => {
  if (stubs.length < 2) return false;
  const bookSources = new Set(corpus.catalog.sources.map((source) => source.id));
  // Each variant belongs to exactly one book, and the books differ: turning one off hides only it.
  return stubs.every((stub) => memberBooks(stub).length === 1 && bookSources.has(stub.source))
    && new Set(stubs.map((stub) => stub.source)).size > 1;
})!;
const title = variants[0].title;
const kind = variants[0].kind;

test("Source variants retain distinct entries in browse and search after book filtering", async ({ page, context, server }) => {
  expect(variants.length).toBeGreaterThan(1);
  await installLibrary(page); await waitControlled(page); await goOffline(context, server);
  await page.goto(`#/browse/kind/${kind}?l=${encodeURIComponent(groupLetter(title))}`);
  // Other folds may share the title under another parent; the fold is the one that holds these links.
  const group = page.locator(".entry-list .source-variants").filter({ has: page.locator(`a[href="#/entry/${variants[0].id}"]`) });
  await expect(group).toHaveCount(1);
  // A parented list builds its rows once its lookup resolves; press only on the settled row.
  await expect(page.getByText("Loading…", { exact: true })).toHaveCount(0);
  await expect(group.locator("summary")).toBeVisible();
  await expect(group.locator("summary")).toContainText(new RegExp(`^${escape(title)} `));
  await group.locator("summary").press("Enter");
  for (const stub of variants) await expect(group.locator(`a[href="#/entry/${stub.id}"]`)).toBeVisible();
  await group.locator(`a[href="#/entry/${variants[0].id}"]`).click();
  await expect(page.locator("h1")).toHaveText(title);
  await page.locator(".bookmark-toggle").click();
  await page.goto(`#/search?q=${encodeURIComponent(title)}&kind=${kind}`);
  await waitResults(page, title);
  const searchGroup = page.locator(".results > .hit-group").filter({ has: page.locator("summary .hit-title", { hasText: new RegExp(`^${escape(title)}$`) }) });
  await expect(searchGroup).toHaveCount(1);
  await searchGroup.locator("summary").press("Enter");
  for (const stub of variants) await expect(searchGroup.locator(`a[href="#/entry/${stub.id}"]`)).toBeVisible();
  const disabled = variants[0].source;
  const source = corpus.catalog.sources.find((candidate) => candidate.id === disabled)!;
  await page.goto("#/books/manage");
  await openBookGroups(page);
  await page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).uncheck();
  await expect(page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeDisabled();
  await page.goto(`#/search?q=${encodeURIComponent(title)}&kind=${kind}`);
  await waitResults(page, title);
  await expect(page.locator(`.results a[href="#/entry/${variants[0].id}"]`)).toHaveCount(0);
  await page.goto("#/");
  await page.locator(`.home-bookmarks a[href="#/entry/${variants[0].id}"]`).click();
  await expect(page.locator("h1")).toHaveText(title);
  await expect(page.getByText(/This entry's book is turned off/)).toBeVisible();
  await expect(page.getByText("It stays in your Starred list.")).toBeVisible();
});

// ---- List rows name their parent, and the fold joins only printings of one rule (#575) ----

const stubsOf = (kind: string, title: string): Stub[] =>
  corpus.catalog.entries.filter((stub: Stub) => stub.kind === kind && stub.title === title);
const rowsTitled = (page: Page, title: string) =>
  page.locator(".entry-list > li").filter({ has: page.getByRole("link", { name: title, exact: true }) });
/** The muted lines of the rows, once the parents have loaded: each starts with a parent title. */
const mutedLines = (rows: Locator) => rows.locator(".row-source").allInnerTexts();

test("Racial traits that share a title are separate rows, each naming its own race", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/racial_trait?l=A");
  const attributes = stubsOf("racial_trait", "Attributes");
  const races = attributes.map(parentTitle);
  expect(new Set(races).size).toBeGreaterThanOrEqual(7);
  // The rows exist at once; the race joins the line when the relations have loaded.
  const rows = rowsTitled(page, "Attributes");
  await expect(rows).toHaveCount(attributes.length);
  await expect(page.locator(".source-variants").filter({ hasText: /^\s*Attributes/ })).toHaveCount(0);
  await expect
    .poll(async () => (await mutedLines(rows)).map((line) => line.split(" · ")[0]).sort())
    .toEqual([...races].sort());
  const lines = await mutedLines(rows);
  expect(new Set(lines).size, lines.join(" | ")).toBe(lines.length);
});

test("An alternate racial trait starts its line with Alternate and its fold names the race", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/racial_trait?l=A");
  // The trait printed in two books under one race is one fold, and the race says whose it is.
  const printings = stubsOf("racial_trait", "Ancient Enmity").filter((stub: Stub) => parentTitle(stub) === "Dwarf");
  expect(printings.length).toBeGreaterThan(1);
  const fold = page.locator(".entry-list .source-variants").filter({ has: page.locator(`a[href="#/entry/${printings[0].id}"]`) });
  await expect(fold).toHaveCount(1);
  await expect(fold.locator("summary .row-source")).toHaveText("Alternate · Dwarf");
  // A single alternate trait: the marker, the race, the book.
  const single = (corpus.catalog.entries as Stub[]).find(
    (stub) =>
      stub.kind === "racial_trait" &&
      stub.id.includes("/alternate-racial-traits/") &&
      groupLetter(stub.title) === "A" &&
      stubsOf("racial_trait", stub.title).filter((other) => parentTitle(other) === parentTitle(stub)).length === 1,
  )!;
  const row = page.locator(".entry-list > li").filter({ has: page.locator(`a[href="#/entry/${single.id}"]`) });
  await expect(row.locator(".row-source")).toHaveText(new RegExp(`^Alternate · ${escape(parentTitle(single))} · .+`));
  // Narrowed by race, the race is the menu's value: the marker stays and the race goes.
  await page.goto("#/browse/kind/racial_trait?facet=racial_trait_race:dwarf&l=A");
  const dwarf = page.locator(".entry-list > li .row-source");
  await expect(page.locator(".entry-list > li .row-source", { hasText: /^Alternate · / }).first()).toBeVisible();
  await expect(page.getByText("Loading…", { exact: true })).toHaveCount(0);
  await expect.poll(async () => (await dwarf.allInnerTexts()).filter((text) => text.includes("Dwarf"))).toEqual([]);
});

test("Afflictions that share a title name their creature", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/affliction?l=P");
  const poisons = stubsOf("affliction", "Poison (Ex)");
  expect(poisons.length).toBeGreaterThan(1);
  const rows = rowsTitled(page, "Poison (Ex)");
  await expect(rows).toHaveCount(poisons.length);
  await expect
    .poll(async () => (await mutedLines(rows)).map((line) => line.split(" · ")[0]).sort())
    .toEqual(poisons.map(parentTitle).sort());
  for (const line of await mutedLines(rows)) expect(line).toContain(" · ");
});

test("An archetype row names its class", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/archetype?l=A");
  const acrobats = stubsOf("archetype", "Acrobat");
  expect(acrobats.length).toBeGreaterThan(0);
  const rows = rowsTitled(page, "Acrobat");
  await expect(rows).toHaveCount(acrobats.length);
  await expect
    .poll(async () => (await mutedLines(rows)).map((line) => line.split(" · ")[0]).sort())
    .toEqual(acrobats.map(parentTitle).sort());
});

test("Every open fold lists its printings under links that never read the same", async ({ page }) => {
  await installLibrary(page);
  const lists: [string, boolean][] = [
    ["#/browse/kind/racial_trait?l=A", true],
    ["#/browse/kind/affliction?l=P", false],
    ["#/browse/kind/affliction?l=D", true],
    ["#/browse/kind/archetype?l=A", false],
    ["#/browse/kind/creature?l=A", true],
  ];
  for (const [hash, hasFolds] of lists) {
    await page.goto(hash);
    // The rows are built once the parents have loaded.
    await expect(page.locator(".entry-list > li").first()).toBeVisible();
    await expect(page.getByText("Loading…", { exact: true })).toHaveCount(0);
    const folds = page.locator(".entry-list .source-variants");
    if (hasFolds) await expect(folds.first(), hash).toBeVisible();
    await folds.evaluateAll((all) => all.forEach((fold) => ((fold as HTMLDetailsElement).open = true)));
    for (const fold of await folds.all()) {
      const texts = await fold.locator("ul a").allInnerTexts();
      expect(new Set(texts).size, `${hash}: ${texts.join(" | ")}`).toBe(texts.length);
      // A link names its book alone: the kind is the page's.
      for (const text of texts) expect(text, hash).not.toMatch(/, (Racial traits|Afflictions|Creatures|Archetypes)$/);
    }
  }
});

test("Creatures with one title still fold, since they have no parent", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/creature?l=A");
  const fold = page.locator(".entry-list .source-variants").filter({ has: page.locator("summary", { hasText: /^\s*Acolyte / }) });
  await expect(fold).toHaveCount(1);
  await expect(fold.locator("summary .row-source")).toHaveCount(0);
});

// ---- A fold lists the latest printing first, as search does (#637) ----

test("An item fold lists the newest printing first and names itself by it", async ({ page }) => {
  const thirdParty = new Set<string>(
    corpus.catalog.sources.filter((source: any) => source.third_party).map((source: any) => source.id as string),
  );
  // Several printings of one product line with distinct dates, where neither the id order nor
  // the book-title order would put the newest first, so only the new order passes.
  const group = latestPrintingGroups(corpus, thirdParty, { kind: "item", sameText: false }).find(
    (candidate) => candidate.newest.id !== candidate.byBook.id,
  );
  expect(group, "the library must keep an item in several books of one product line, with distinct dates").toBeTruthy();
  const { stubs, newest } = group!;
  const bookOf = (stub: Stub) => corpus.catalog.sources.find((source) => source.id === stub.source)!.title as string;
  await installLibrary(page);
  await page.goto(`#/browse/kind/item?l=${encodeURIComponent(groupLetter(newest.title))}`);
  const fold = page.locator(".entry-list .source-variants").filter({ has: page.locator(`a[href="#/entry/${newest.id}"]`) });
  await expect(fold).toHaveCount(1);
  await expect(fold.locator("summary")).toContainText(new RegExp(`^${escape(newest.title)} `));
  await fold.locator("summary").press("Enter");
  // Each link names its book, newest printing first; a repeated name only gains a number.
  const links = await fold.locator("ul a").evaluateAll((anchors) => anchors.map((a) => a.getAttribute("href")));
  expect(links).toEqual(stubs.map((stub: Stub) => `#/entry/${stub.id}`));
  await expect(fold.locator("ul a").first()).toHaveText(new RegExp(`^${escape(bookOf(newest))}( \\(\\d+\\))?$`));
});
