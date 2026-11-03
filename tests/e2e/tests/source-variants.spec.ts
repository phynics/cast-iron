import { test, expect, loadCorpus, goOffline } from "./fixtures";
import { installLibrary, waitControlled, searchFor } from "./helpers";

const corpus = loadCorpus();
const anatomists = corpus.catalog.entries.filter(e => e.kind === "trait" && e.title === "Anatomist");

test("Source variants retain distinct entries in browse and search after book filtering", async ({ page, context, server }) => {
  expect(anatomists.length).toBeGreaterThan(1);
  await installLibrary(page); await waitControlled(page); await goOffline(context, server);
  await page.goto("#/browse/kind/trait");
  const group = page.locator(".entry-list .source-variants").filter({ has: page.locator("summary", { hasText: /^Anatomist / }) });
  await expect(group).toHaveCount(1);
  await group.locator("summary").press("Enter");
  for (const stub of anatomists) await expect(group.locator(`a[href="#/entry/${stub.id}"]`)).toBeVisible();
  await group.locator(`a[href="#/entry/${anatomists[0].id}"]`).click();
  await expect(page.locator("h1")).toHaveText("Anatomist");
  await page.locator(".bookmark-toggle").click();
  await searchFor(page, "Anatomist");
  const searchGroup = page.locator(".results > .hit-group").filter({ has: page.locator("summary .hit-title", { hasText: /^Anatomist$/ }) });
  await expect(searchGroup).toHaveCount(1);
  await searchGroup.locator("summary").press("Enter");
  for (const stub of anatomists) await expect(searchGroup.locator(`a[href="#/entry/${stub.id}"]`)).toBeVisible();
  const source = corpus.catalog.sources.find(s => s.id === anatomists[0].source)!;
  await page.goto("#/books");
  await page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).uncheck();
  await expect(page.getByRole("checkbox", { name: `Active ${source.title}`, exact: true })).not.toBeDisabled();
  await searchFor(page, "Anatomist");
  await expect(page.locator(`.results a[href="#/entry/${anatomists[0].id}"]`)).toHaveCount(0);
  await page.goto("#/");
  await page.locator(`.home-bookmarks a[href="#/entry/${anatomists[0].id}"]`).click();
  await expect(page.locator("h1")).toHaveText("Anatomist");
  await expect(page.getByText(/This book is disabled/)).toBeVisible();
});
