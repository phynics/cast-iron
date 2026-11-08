import fs from "node:fs";
import path from "node:path";
import { GENERATED, installedTest as test, expect, loadCorpus } from "./fixtures";
import {
  bookOwners,
  count,
  corpusEntry,
  installLibrary,
  openBookGroups,
  openQuickSearch,
  quickOptions,
  seedHome,
  waitControlled,
  waitHomeIdle,
} from "./helpers";

const corpus = loadCorpus();
const owners = bookOwners();

interface OptionGroup {
  slug: string;
  label: string;
  singular: string;
  classes: string[];
  entries: string[];
}
interface OptionGroupIndex {
  groups: OptionGroup[];
}

function optionGroups(): OptionGroupIndex {
  return JSON.parse(
    fs.readFileSync(path.join(GENERATED, corpus.version, "option-groups.json"), "utf8"),
  );
}

const groups = optionGroups();
const discoveries = groups.groups.find((group) => group.slug === "arcane-discovery")!;
const schools = groups.groups.find((group) => group.slug === "arcane-school")!;
const allOptions = corpus.catalog.entries.filter((stub) => stub.kind === "class_option");
const wizard = corpusEntry("class", "Wizard");
const arcaneBuilder = corpusEntry("class_option", "Arcane Builder");
const ultimateMagic = "ultimate-magic";

/** The book the client shows an entry under: its first placement, else its catalog source. */
const ownerOf = (id: string) =>
  owners.get(id) ?? corpus.entries.get(id)?.source_ids[0] ?? "";

const groupLink = (page: import("@playwright/test").Page, label: string) =>
  page.locator(".entry-rel .jump-list").getByRole("link", { name: new RegExp(`^${label} `) });

test("The Wizard page offers its class-option groups and opens the filtered list", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/entry/${wizard.id}`);
  await expect(page.getByRole("heading", { level: 1, name: wizard.title })).toBeVisible();

  const options = page.locator('.entry-rel section[aria-labelledby="rel-class-options"]');
  await expect(options.getByRole("heading", { level: 2, name: "Class options" })).toBeVisible();
  const discoveryLink = groupLink(page, discoveries.label);
  await expect(discoveryLink).toBeVisible();
  await expect(discoveryLink.locator(".jump-count")).toHaveText(count(discoveries.entries.length));
  await expect(discoveryLink).toHaveAttribute(
    "href",
    `#/browse/kind/class_option?facet=group:${discoveries.slug}`,
  );
  expect((await discoveryLink.boundingBox())!.height).toBeGreaterThanOrEqual(44);

  await discoveryLink.click();
  await expect.poll(() => new URL(page.url()).hash).toBe(
    `#/browse/kind/class_option?facet=group:${discoveries.slug}`,
  );
  await expect(page.getByRole("heading", { level: 1, name: "Class options" })).toBeVisible();
  const menu = page.getByRole("button", { name: `Group: ${discoveries.label}`, exact: true });
  await expect(menu).toBeVisible();
  await expect(page.locator(".list-count")).toContainText(
    `${count(discoveries.entries.length)} of ${count(allOptions.length)} class options`,
  );
  // The group is the selected filter, so a row's muted line is only its book.
  const firstRow = page.locator(".entry-list > li").first();
  await expect(firstRow.locator("a")).toBeVisible();
  await expect(firstRow.locator(".row-source")).toHaveText(/^(?!.* · )/);

  // 44 px targets, and the list never scrolls sideways at 390 px.
  expect((await menu.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
  ).toBe(true);
});

test("A group shared by several classes names the class it was opened from", async ({ page }) => {
  await installLibrary(page);
  const shared = groups.groups.find((group) => group.classes.length > 1)!;
  const classId = shared.classes[0];
  const classEntry = corpus.entries.get(classId)!;
  expect(shared.entries.length).toBeGreaterThan(0);

  await page.goto(`#/entry/${classId}`);
  await expect(page.getByRole("heading", { level: 1, name: classEntry.title })).toBeVisible();
  const link = groupLink(page, shared.label);
  await expect(link).toHaveAttribute(
    "href",
    new RegExp(`facet=group:${shared.slug}&facet=class_scope:${encodeURIComponent(classId).replace(/%2F/g, "%2F")}`),
  );
  await link.click();
  await expect(page.locator(".scope-pill")).toHaveText(classEntry.title);
  await expect(page.locator(".scope-pill")).toHaveAttribute(
    "aria-label",
    `Remove filter: ${classEntry.title}`,
  );
  await page.locator(".scope-pill").click();
  await expect(page.locator(".scope-pill")).toHaveCount(0);
  await expect.poll(() => new URL(page.url()).hash).toBe(
    `#/browse/kind/class_option?facet=group:${shared.slug}`,
  );
});

test("Letter navigation keeps a shared group's class scope", async ({ page }) => {
  await installLibrary(page);
  const shared = groups.groups.find(
    (group) => group.classes.length > 1 && group.entries.length > 200,
  )!;
  const classId = shared.classes[0];
  const classEntry = corpus.entries.get(classId)!;

  await page.goto(`#/entry/${classId}`);
  await groupLink(page, shared.label).click();
  await expect(page.locator(".scope-pill")).toHaveText(classEntry.title);

  const next = page.locator('.letter-btn:not([aria-current="true"])').first();
  await expect(next).toBeVisible();
  expect(await next.getAttribute("href")).toContain(
    `facet=class_scope:${encodeURIComponent(classId)}`,
  );
  await next.click();
  await expect.poll(() => new URL(page.url()).hash).toContain("facet=class_scope:");
  await expect(page.locator(".scope-pill")).toHaveText(classEntry.title);
});

test("The group menu filters the class-option list in place", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/browse/kind/class_option?facet=group:${discoveries.slug}`);
  const total = count(allOptions.length);
  await expect(page.locator(".list-count")).toContainText(
    `${count(discoveries.entries.length)} of ${total} class options`,
  );

  const menu = page.getByRole("button", { name: `Group: ${discoveries.label}`, exact: true });
  await menu.click();
  const panel = page.locator(`#${await menu.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  const find = panel.getByRole("searchbox", { name: "Find a group" });
  await expect(find).toBeFocused();
  await find.fill("Arcane schools");
  const choice = panel.getByRole("link", { name: `Arcane schools, ${count(schools.entries.length)}` });
  await expect(choice).toBeVisible();
  await choice.click();

  await expect.poll(() => new URL(page.url()).hash).toBe(
    `#/browse/kind/class_option?facet=group:${schools.slug}`,
  );
  await expect(page.getByRole("button", { name: `Group: ${schools.label}`, exact: true })).toBeFocused();
  await expect(page.locator(".list-count")).toContainText(
    `${count(schools.entries.length)} of ${total} class options`,
  );
  // Rows in the whole list name their group before the book; the filter names it already.
  await page.getByRole("button", { name: `Group: ${schools.label}`, exact: true }).click();
  await page.locator(`#filter-panel-group`).getByRole("link", { name: "All groups" }).click();
  await expect(page.locator(".list-count")).toContainText(`${total} class options`);
  await expect(page.locator(".entry-list .row-source").first()).toContainText(" · ");
});

test("An option's metadata line names its group and links its only class", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/entry/${arcaneBuilder.id}`);
  await expect(page.getByRole("heading", { level: 1, name: arcaneBuilder.title })).toBeVisible();
  const meta = page.locator(".entry-meta");
  await expect(
    meta.getByRole("link", { name: discoveries.singular, exact: true }),
  ).toHaveAttribute(
    "href",
    `#/browse/kind/class_option?facet=group:${discoveries.slug}`,
  );
  // Arcane discoveries serve the Wizard alone, so the group links its one class.
  await expect(meta.getByRole("link", { name: wizard.title, exact: true })).toHaveAttribute(
    "href",
    `#/entry/${wizard.id}`,
  );
});

test("Search and Quick search label a class option with its group", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/search?q=${encodeURIComponent(arcaneBuilder.title)}`);
  const hit = page.locator(".results .hit").filter({
    has: page.locator(".hit-title", { hasText: new RegExp(`^${arcaneBuilder.title}$`) }),
  }).first();
  await expect(hit).toBeVisible();
  await expect(hit.locator(".hit-meta")).toHaveText(
    `${discoveries.singular} · Ultimate Magic`,
  );

  await page.goto("#/");
  await waitHomeIdle(page);
  const layer = await openQuickSearch(page);
  await layer.getByRole("combobox", { name: "Search the rules" }).fill(arcaneBuilder.title);
  await expect(quickOptions(page).last()).toContainText(`See all results for “${arcaneBuilder.title}”`);
  const line = layer.locator('[role="option"]', { hasText: arcaneBuilder.title }).locator(".quick-search-meta").first();
  await expect(line).toHaveText(`${discoveries.singular} · Ultimate Magic`);
});

test("With Ultimate Magic inactive, its discoveries leave the class page and the list", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/books/manage");
  await openBookGroups(page);
  const toggle = page
    .locator(`.book-directory-item[data-book-id="${ultimateMagic}"]`)
    .getByRole("checkbox", { name: "Active Ultimate Magic", exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  const kept = discoveries.entries.filter((id) => ownerOf(id) !== ultimateMagic).length;
  const keptAll = allOptions.filter((stub) => ownerOf(stub.id) !== ultimateMagic).length;
  expect(kept).toBeLessThan(discoveries.entries.length);

  await page.goto(`#/entry/${wizard.id}`);
  const discoveryLink = groupLink(page, discoveries.label);
  await expect(discoveryLink.locator(".jump-count")).toHaveText(count(kept));
  await discoveryLink.click();
  await expect(page.locator(".list-count")).toContainText(
    `${count(kept)} of ${count(keptAll)} class options`,
  );
  // No row from the disabled book remains in the filtered group.
  const rows = await page.locator(".entry-list > li a").evaluateAll((links) =>
    links.map((link) => (link as HTMLAnchorElement).hash.replace(/^#\/entry\//, "")),
  );
  expect(rows.length).toBeGreaterThan(0);
  for (const id of rows) expect(ownerOf(decodeURIComponent(id))).not.toBe(ultimateMagic);
});

test("A starred class option refreshes its stored kind snapshot", async ({ page }) => {
  await installLibrary(page);
  await seedHome(page, {
    stars: [{ id: arcaneBuilder.id, title: arcaneBuilder.title, kind: "rule" }],
  });

  const storedKind = await page.evaluate(
    (id) =>
      new Promise<string | null>((resolve, reject) => {
        const open = indexedDB.open("cast-iron");
        open.onerror = () => reject(open.error);
        open.onsuccess = () => {
          const db = open.result;
          const get = db.transaction("bookmarks").objectStore("bookmarks").get(id);
          get.onsuccess = () => {
            db.close();
            resolve((get.result as { kind?: string } | undefined)?.kind ?? null);
          };
          get.onerror = () => {
            db.close();
            reject(get.error);
          };
        };
      }),
    arcaneBuilder.id,
  );
  expect(storedKind).toBe("class_option");

  const row = page.locator(".home-bookmarks .home-row").filter({ hasText: arcaneBuilder.title }).first();
  await expect(row).toBeVisible();
  await row.getByRole("link").click();
  await expect(page.getByRole("heading", { level: 1, name: arcaneBuilder.title })).toBeVisible();
});
