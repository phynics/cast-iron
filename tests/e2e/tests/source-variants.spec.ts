import { test, expect, loadCorpus, goOffline } from "./fixtures";
import { bookOwners, groupLetter, installLibrary, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();
const owners = bookOwners();
/**
 * A title the library publishes more than once under distinct books, other than the
 * books-only Rule kind. The canonical and PSRD libraries repeat different titles, so the
 * group comes from the compiled catalog rather than a literal name.
 */
const BROWSABLE = new Set(["spell", "feat", "trait", "item", "creature", "class", "race",
  "prestige_class", "affliction", "archetype", "skill", "trap", "racial_trait"]);
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const groups = new Map<string, typeof corpus.catalog.entries>();
for (const stub of corpus.catalog.entries) {
  if (!BROWSABLE.has(stub.kind)) continue;
  const key = `${stub.kind}\u0000${stub.title}`;
  groups.set(key, [...(groups.get(key) ?? []), stub]);
}
const variants = [...groups.values()].find((stubs) => {
  if (stubs.length < 2) return false;
  const bookSources = new Set(corpus.catalog.sources.map((source) => source.id));
  return stubs.every((stub) => owners.has(stub.id) && bookSources.has(owners.get(stub.id)!))
    && new Set(stubs.map((stub) => owners.get(stub.id))).size > 1;
})!;
const title = variants[0].title;
const kind = variants[0].kind;

test("Source variants retain distinct entries in browse and search after book filtering", async ({ page, context, server }) => {
  expect(variants.length).toBeGreaterThan(1);
  await installLibrary(page); await waitControlled(page); await goOffline(context, server);
  await page.goto(`#/browse/kind/${kind}?l=${encodeURIComponent(groupLetter(title))}`);
  const group = page.locator(".entry-list .source-variants").filter({ has: page.locator("summary", { hasText: new RegExp(`^${escape(title)} `) }) });
  await expect(group).toHaveCount(1);
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
  const disabled = owners.get(variants[0].id) ?? variants[0].source;
  const source = corpus.catalog.sources.find((candidate) => candidate.id === disabled)!;
  await page.goto("#/books");
  await page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).uncheck();
  await expect(page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeDisabled();
  await page.goto(`#/search?q=${encodeURIComponent(title)}&kind=${kind}`);
  await waitResults(page, title);
  await expect(page.locator(`.results a[href="#/entry/${variants[0].id}"]`)).toHaveCount(0);
  await page.goto("#/");
  await page.locator(`.home-bookmarks a[href="#/entry/${variants[0].id}"]`).click();
  await expect(page.locator("h1")).toHaveText(title);
  await expect(page.getByText(/This book is disabled/)).toBeVisible();
});
