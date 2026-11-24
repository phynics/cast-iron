import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { count, hitTitles, installLibrary, isMember, searchFor, waitControlled } from "./helpers";

// Haunts are a kind of their own (#618, schema 7). The pinned corpus (v0.1.43) holds 45 of them,
// all under `entries/haunt/` with the ids they had as rules. The spec reads the compiled
// library, so a corpus that drops or adds a haunt changes the expected numbers with it; the one
// fixed number is the 45 the owner decision names.

const corpus = loadCorpus();
const haunts = corpus.catalog.entries.filter((stub: any) => stub.kind === "haunt");
const BOOK = "horror-adventures";
const bookTitle: string = corpus.catalog.sources.find((source: any) => source.id === BOOK)!.title;
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Home lists Haunts as a category and the list holds the 45 haunts", async ({ page }) => {
  expect(haunts.length, "the pinned corpus has 45 haunts").toBe(45);
  await page.goto("#/");
  // Haunts sit under "More categories", with Traps and Afflictions.
  const category = page.locator("section[aria-labelledby=by-kind-more] a.cat").filter({
    has: page.locator(".cat-name", { hasText: /^Haunts$/ }),
  });
  await expect(category).toHaveCount(1);
  await expect(category.locator(".cat-count")).toHaveText(count(45));
  await category.click();
  await expect(page).toHaveURL(/#\/browse\/kind\/haunt$/);
  await expect(page.getByRole("heading", { level: 1, name: "Haunts", exact: true })).toBeVisible();
  await expect(page.locator("main .list-count")).toHaveText("45 haunts");
  await expect(page.locator(".entry-list a[href^='#/entry/ref/haunt/']")).toHaveCount(45);
  // Traps stay a list of their own: no haunt is among them.
  await page.goto("#/browse/kind/trap");
  await expect(page.getByRole("heading", { level: 1, name: "Traps", exact: true })).toBeVisible();
  await expect(page.locator(".entry-list a[href^='#/entry/ref/haunt/']")).toHaveCount(0);
});

test("A book names its haunts in In this book, and Contents no longer lists them", async ({ page }) => {
  const inBook = haunts.filter((stub: any) => isMember(stub, BOOK));
  expect(inBook.length, "Horror Adventures prints 26 haunts").toBe(26);

  await page.goto(`#/browse/book/${BOOK}`);
  await expect(page.getByRole("heading", { level: 1, name: bookTitle, exact: true })).toBeVisible();
  const link = page.locator(".book-in-this-book .jump", { has: page.locator(".book-kind-label", { hasText: /^Haunts$/ }) });
  await expect(link).toHaveCount(1);
  await expect(link.locator(".book-kind-count")).toHaveText("26");
  await expect(link).toHaveAttribute("href", `#/browse/kind/haunt?source=${BOOK}`);

  // Contents holds rules only: no haunt, however the corpus placed it in the book.
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(bookTitle)}`) });
  await expect(tree).toBeVisible();
  const hrefs: string[] = await tree.locator('a[href^="#/entry/"]').evaluateAll((links) =>
    links.map((a) => a.getAttribute("href") as string));
  expect(hrefs.filter((href) => href.startsWith("#/entry/ref/haunt/")), "Contents links no haunt").toEqual([]);
  const ids = new Set(haunts.map((stub: any) => stub.id));
  expect(hrefs.filter((href) => ids.has(decodeURIComponent(href.slice("#/entry/".length).split("?")[0])))).toEqual([]);

  // The link opens the book's haunts, with the book pill on top.
  await link.click();
  await expect(page).toHaveURL(new RegExp(`#/browse/kind/haunt\\?source=${escape(BOOK)}$`));
  await expect(page.getByRole("heading", { level: 1, name: "Haunts", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: `Remove filter: ${bookTitle}`, exact: true })).toBeVisible();
  await expect(page.locator(".entry-list a[href^='#/entry/ref/haunt/']")).toHaveCount(26);
});

test("A haunt page reads Haunt in its metadata line, not Rule", async ({ page }) => {
  const stub = haunts.find((candidate: any) => candidate.id === "ref/haunt/arcane-storm") ?? haunts[0];
  await page.goto(`#/entry/${stub.id}`);
  await expect(page.getByTestId("entry-title")).toHaveText(stub.title);
  const meta = page.locator(".entry-meta");
  await expect(meta).toHaveText(/^Haunt·\S/);
  await expect(meta.getByRole("link", { name: "Haunt", exact: true })).toHaveAttribute("href", /#\/browse\/kind\/haunt/);
  await expect(meta.getByRole("link", { name: "Rule", exact: true })).toHaveCount(0);
});

test("Search names Haunts in the Kind menu and ranks a haunt first for its own name", async ({ page }) => {
  await searchFor(page, "arcane storm");
  const trigger = page.locator(".filters #filter-btn-kind");
  await trigger.click();
  const panel = page.locator(`#${await trigger.getAttribute("aria-controls")}`);
  const row = panel.getByRole("checkbox", { name: /^Kind Haunts, / });
  await expect(row).toHaveCount(1);
  await expect(row).toHaveAccessibleName(/^Kind Haunts, [\d,]+ entr(?:y|ies)$/);

  // A haunt has no search aliases (the compiler gives aliases to rules only), so its own name
  // reaches it through its title, and it comes first.
  expect((await hitTitles(page))[0]).toBe("Arcane Storm");
  await expect(page.locator(".results .hit-link").first()).toHaveAttribute("href", "#/entry/ref/haunt/arcane-storm");

  await row.check();
  await expect(trigger).toHaveText("Kind: Haunts");
  expect(new URL(page.url()).hash).toContain("kind=haunt");
  await expect(page.locator(".results .hit").first()).toBeVisible();
});

test("A haunt opens offline", async ({ page, context }) => {
  await waitControlled(page);
  await context.setOffline(true);
  const stub = haunts.find((candidate: any) => candidate.id === "ref/haunt/arcane-storm") ?? haunts[0];
  await page.goto("#/browse/kind/haunt");
  await expect(page.getByRole("heading", { level: 1, name: "Haunts", exact: true })).toBeVisible();
  await page.locator(`.entry-list a[href="#/entry/${stub.id}"]`).click();
  await expect(page.getByTestId("entry-title")).toHaveText(stub.title);
  await expect(page.locator(".entry-meta")).toHaveText(/^Haunt·\S/);
  await expect(page.locator(".entry > .rich")).not.toBeEmpty();
});
