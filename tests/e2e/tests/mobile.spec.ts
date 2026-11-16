import type { Locator, Page } from "@playwright/test";
import { test, installedTest, expect, loadCorpus } from "./fixtures";
import { differingPixels } from "./png-diff";
import { bookTrees, firstEntry, hasBlock, headingAnchors, installLibrary, openBookGroups, openQuickSearch, quickField, quickGroup, searchIcon, seedHome, waitResults } from "./helpers";

// Runs in the mobile-chromium project (390 px wide). Layout rules live in docs/decisions.md.
// Every screen names a target the compiled corpus actually holds; the two libraries differ
// in ids, sources, and counts, so none of these may be a literal entry id.
const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const statSpell = firstEntry("a spell with a stat block", (entry) => entry.kind === "spell" && hasBlock(entry, "stat_block"));
const tableEntry = firstEntry("an entry with a table", (entry) => hasBlock(entry, "table"));
const headingEntry = firstEntry("an entry with a heading", (entry) => headingAnchors(entry).length > 0);
const creature = firstEntry("a creature with a stat block", (entry) => entry.kind === "creature" && hasBlock(entry, "stat_block"));
const source = bookTrees()[0].source;
const classLevels = new Map<string, number>();
for (const entry of entries) {
  for (const level of entry.fields?.levels ?? []) classLevels.set(level.class, (classLevels.get(level.class) ?? 0) + 1);
}
const [listClass, listCount] = [...classLevels.entries()].sort((a, b) => b[1] - a[1])[0];
const listSpell = entries.find((entry) => entry.kind === "spell" &&
  entry.fields?.levels?.some((level: any) => level.class === listClass))!;
const listLevel = listSpell.fields.levels.find((level: any) => level.class === listClass).level;

const screens: [string, string][] = [
  ["#/", "#home-title"],
  ["#/search?q=fire", ".results .hit"],
  ["#/browse", "a.cat"],
  ["#/browse/kind/spell", ".entry-list li"],
  ["#/books", ".book-directory-row"],
  ["#/books/manage", ".preset-row"],
  [`#/browse/source/${encodeURIComponent(source)}`, ".toc"],
  [`#/spells/${encodeURIComponent(listClass)}/${listLevel}`, ".level-group"],
  ["#/feats/chains", ".chain-head"],
  [`#/entry/${statSpell.id}`, ".stat-block"],
  [`#/entry/${tableEntry.id}`, ".rules-table"],
  [`#/entry/${headingEntry.id}`, ".rich h2"],
  [`#/entry/${creature.id}`, ".stat-block"],
  ["#/bookmarks", "#home-title"],
  ["#/settings", "[data-testid=library-facts]"],
  ["#/legal", ".legal-text"],
];

installedTest("Mobile viewport shows no horizontal page scroll on key screens", async ({ page }) => {
  expect(page.viewportSize()?.width).toBe(390);
  await installLibrary(page);
  const wide: string[] = [];
  for (const [hash, ready] of screens) {
    await page.goto(hash);
    if (hash.startsWith("#/search")) await waitResults(page, "fire");
    await expect(page.locator(ready).first(), hash).toBeVisible();
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    if (overflow > 0) wide.push(`${hash} overflows by ${overflow}px`);
  }
  expect(wide).toEqual([]);
});

installedTest("A creature breadcrumb is one line of working links at 390 px", async ({ page }) => {
  await installLibrary(page);
  // The deepest printed taxonomy: Monsters > Dragon > Chromatic > Red.
  const nested = firstEntry("a creature with a deep section", (entry) =>
    entry.kind === "creature" && (entry.section?.length ?? 0) >= 4);
  await page.goto(`#/entry/${nested.id}`);
  await expect(page.locator(".stat-block").first()).toBeVisible();
  const crumbs = page.locator(".crumbs");
  await expect(crumbs).toBeVisible();
  // Printed taxonomy has no page, so it is not rendered as a segment.
  await expect(crumbs.locator("li > span")).toHaveCount(0);
  const labels = await crumbs.getByRole("link").allInnerTexts();
  expect(labels[0]).toBe("Home");
  expect(labels).toContain("Creatures");
  // One line: every segment shares the same top edge.
  const tops = await crumbs.locator("li").evaluateAll((items) =>
    items.map((item) => Math.round(item.getBoundingClientRect().top)));
  expect(new Set(tops).size).toBe(1);
  expect((await crumbs.boundingBox())!.height).toBeLessThanOrEqual(60);
});

installedTest("Book rows, the book page contents tree and the groups fit the mobile viewport", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await openBookGroups(page);
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0));
  if (!book) throw new Error("The compiled corpus has no book with contents.");
  const overflow = () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const link = row.locator("a.book-directory-row");
  // The row is a link and nothing else: no checkbox on Books.
  await expect(row.locator("input")).toHaveCount(0);
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.height).toBeGreaterThanOrEqual(56);
  expect(linkBox.x + linkBox.width).toBeLessThanOrEqual(390);
  expect(await overflow()).toBeLessThanOrEqual(0);

  await link.click();
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of `) });
  await expect(tree).toBeVisible();
  // The contents are expanded, with no disclosure to open, and every row is a 44 px target.
  await expect(tree.locator("details, summary, [aria-expanded]")).toHaveCount(0);
  const tocRow = tree.locator(".toc-top > li > .toc-row, .toc-top > li.toc-leaf > a").first();
  await expect(tocRow).toBeVisible();
  expect((await tocRow.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  expect(await overflow()).toBeLessThanOrEqual(0);

  // The book page has no breadcrumb, its first screen holds the title, the meta line,
  // In this book and the Contents heading, and nothing overflows.
  await expect(page.locator("main .crumbs")).toHaveCount(0);
  await expect(page.getByTestId("source-count")).toBeVisible();
  await expect(page.getByRole("heading", { level: 2, name: "In this book", exact: true })).toBeVisible();
  const heading = page.getByRole("heading", { level: 2, name: "Contents", exact: true });
  await expect(heading).toBeVisible();
  expect((await heading.boundingBox())!.y).toBeLessThan(844);
  expect(await overflow()).toBeLessThanOrEqual(0);
});

installedTest("Books and Manage books fit 390 px with full-size touch targets", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  const groups = page.locator(".book-tier");
  await expect(groups.first()).toBeVisible();
  await expect(groups.first().getByRole("heading", { level: 2 })).toContainText("Rulebooks");
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  await expect(adventures.getByRole("button")).toHaveAttribute("aria-expanded", "false");
  // Every group heading is a 44 px target that stays inside the screen.
  for (const toggle of await page.locator(".tier-toggle").all()) {
    const box = (await toggle.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  const overflow = async () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

  // Books has no group controls and no Only Rulebooks; the link to Manage books is a 44 px target.
  await expect(page.getByRole("button", { name: /Activate all|Deactivate all|Only Rulebooks/ })).toHaveCount(0);
  const link = page.getByRole("link", { name: "Manage books", exact: true });
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.height).toBeGreaterThanOrEqual(44);
  expect(linkBox.x + linkBox.width).toBeLessThanOrEqual(390);
  expect(await overflow()).toBeLessThanOrEqual(0);

  await link.click();
  await expect(page).toHaveURL(/#\/books\/manage$/);
  await expect(page.getByRole("heading", { level: 1, name: "Manage books", exact: true })).toBeVisible();
  const targets = [
    page.getByRole("button", { name: "Done", exact: true }),
    ...(await page.locator(".preset-row").all()),
  ];
  expect(targets.length).toBeGreaterThan(1);
  for (const target of targets) {
    const box = (await target.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  // A checkbox row is a whole-row label at least 56 px tall, and the box itself is not tiny.
  const row = page.locator(".toggle-row").first();
  expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(56);
  const box = (await row.getByRole("checkbox").boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(20);
  expect(await overflow()).toBeLessThanOrEqual(0);
  await adventures.getByRole("button").click();
  await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  expect(await overflow()).toBeLessThanOrEqual(0);
});

test("Mobile header Install button is icon-only, a full touch target, and crowds nothing", async ({ page, context }) => {
  await context.addInitScript({
    content: `document.addEventListener("DOMContentLoaded", () => {
      const event = new Event("beforeinstallprompt", { cancelable: true });
      event.prompt = async () => {};
      event.userChoice = Promise.resolve({ outcome: "dismissed" });
      window.dispatchEvent(event);
    });`,
  });
  await installLibrary(page);
  const button = page.locator("header.app-header").getByRole("button", { name: "Install Cast Iron as an app" });
  await expect(button).toBeVisible();
  await expect(button.locator(".header-action-label")).toBeHidden();
  const box = (await button.boundingBox())!;
  // Device pixel ratios round a 44px target slightly below 44 in the layout viewport.
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
  const search = (await searchIcon(page).boundingBox())!;
  expect(search.x + search.width).toBeLessThanOrEqual(box.x);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Mobile entry header is compact: one metadata line and an icon star", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/entry/${statSpell.id}`);
  await expect(page.locator(".stat-block").first()).toBeVisible();
  const top = await page.locator(".stat-block").first().evaluate((el) => el.getBoundingClientRect().top);
  // Title, a short metadata line and the stat block fit in the first 300 px of an 844 px screen.
  // (The old header, with pills and a full-row Star button, put the block near 380 px.) The bound
  // leaves room for a two-line title and a wrapped metadata line.
  expect(top).toBeLessThan(300);
  const star = (await page.locator(".bookmark-toggle").boundingBox())!;
  expect(Math.round(star.width)).toBe(44);
  expect(Math.round(star.height)).toBe(44);
  // A long book title may wrap once; wrapped pills used to take three rows.
  const meta = page.locator(".entry-meta");
  const lineHeight = await meta.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await meta.boundingBox())!.height).toBeLessThan(lineHeight * 2.6);
});

// The first result row may start no lower than 280 px at 390 px (330 px with the hint of an
// inactive book). The header, the field, the scope row and one status line already take about
// 190 px, so the issue's "about 180 px" is the floor before any row. The numbers are in
// docs/decisions.md. Book sections fold into one row above the entries (#387), so the two are
// measured separately: the fold row and the first entry hit. The first entry hit starts no lower
// than 330 px, or 380 px with the hint.
const pageTop = (locator: Locator) => locator.first().evaluate((el) => el.getBoundingClientRect().top + window.scrollY);
const firstSectionTop = (page: Page) => pageTop(page.locator(".section-fold > summary"));
const firstHitTop = (page: Page) => pageTop(page.locator(".results > .hit"));
const pageOverflow = (page: Page) => page.evaluate(
  () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
);

installedTest("Search results at 390 px: one scope row with a Filters button, and results start near the top", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search?q=fireball");
  await waitResults(page, "fireball");

  // No Search button: results are live. The field only has its clear control.
  await expect(page.locator(".search-form button:not(.search-clear)")).toHaveCount(0);

  // The quick kinds are one row that scrolls sideways and never wraps; every pill is a 44 px target.
  const scopes = page.locator(".search-scopes");
  const pills = scopes.locator(".scope-choice");
  await expect(pills).toHaveCount(4);
  const boxes = await pills.evaluateAll((els) => els.map((el) => {
    const r = el.getBoundingClientRect();
    return { top: r.top, height: r.height, radius: getComputedStyle(el).borderRadius };
  }));
  for (const box of boxes) {
    expect(Math.round(box.top)).toBe(Math.round(boxes[0].top));
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.radius).toBe("999px");
  }
  // The row never wraps; whether it overflows depends on the pill sizes, so that is not asserted.
  expect(await scopes.evaluate((el) => getComputedStyle(el).flexWrap)).toBe("nowrap");
  await expect(page.getByRole("button", { name: "All entries", exact: true })).toHaveAttribute("aria-pressed", "true");

  // The Filters button sits at the end of the same row: compact, 44 px, never a pill.
  const filters = page.getByRole("button", { name: "Filters", exact: true });
  await expect(filters).toBeVisible();
  const row = (await page.locator(".search-controls").boundingBox())!;
  const button = (await filters.boundingBox())!;
  expect(button.height).toBeGreaterThanOrEqual(44);
  // Inside the row, at its end, on the same line as the pills, and narrower than the pills' row.
  expect(button.x).toBeGreaterThanOrEqual(row.x - 1);
  expect(button.x + button.width).toBeLessThanOrEqual(row.x + row.width + 1);
  expect(Math.round(button.y)).toBe(Math.round(boxes[0].top));
  expect(button.width).toBeLessThan(row.width / 2);
  expect(button.x).toBeGreaterThan((await scopes.boundingBox())!.x);
  expect(await filters.evaluate((el) => getComputedStyle(el).borderRadius)).not.toBe("999px");

  // The status line sits directly under the row, and the first result row starts near the top.
  const status = (await page.locator(".results-status").boundingBox())!;
  expect(status.y).toBeGreaterThanOrEqual(row.y + row.height);
  expect(status.y - (row.y + row.height)).toBeLessThan(row.height / 2);
  expect(await page.locator(".result-count").evaluate((el) => parseFloat(getComputedStyle(el).fontSize))).toBeLessThan(
    await page.locator(".hit-title").first().evaluate((el) => parseFloat(getComputedStyle(el).fontSize)));
  // `fireball` has no hint and one folded Book sections row leads: pin the row, then the first
  // entry hit, which must start no lower than 330 px.
  await expect(page.locator(".inactive-books-hint")).toHaveCount(0);
  const sectionTop = await firstSectionTop(page);
  expect(sectionTop).toBeLessThan(280);
  expect(sectionTop).toBeGreaterThan(status.y);
  const hitTop = await firstHitTop(page);
  console.log(`390 px: section fold row ${Math.round(sectionTop)} px, first entry hit ${Math.round(hitTop)} px`);
  expect(hitTop).toBeGreaterThan(sectionTop);
  expect(hitTop).toBeLessThanOrEqual(330);
  expect(await pageOverflow(page)).toBeLessThanOrEqual(0);

  // Choosing a kind in the row keeps its pressed pill inside the row.
  const feats = page.getByRole("button", { name: "Feats", exact: true });
  await feats.click();
  await expect(feats).toHaveAttribute("aria-pressed", "true");
  // The scroll into view runs a tick after the click, so the geometry is read until it settles.
  await expect.poll(async () => {
    const rowBox = (await scopes.boundingBox())!;
    const pressed = (await feats.boundingBox())!;
    return pressed.x >= rowBox.x - 1 && pressed.x + pressed.width <= rowBox.x + rowBox.width + 1;
  }).toBe(true);
  await expect(page.getByRole("button", { name: /^Filters/ })).toHaveText("Filters · 1");
});

installedTest("Search results at 390 px: the hidden-matches hint sits in the status area and stays small", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const hint = page.locator(".results-status .inactive-books-hint");
  await expect(hint).toBeVisible();
  // The action keeps its full name for assistive technology and shows a short label.
  const action = hint.getByRole("button", { name: /^Turn on .+/ });
  await expect(action).toHaveText("Turn on");
  // Like "Show all": a line of text whose ::after pseudo-element is the 44 px hit area.
  const hit = await action.evaluate((el) => parseFloat(getComputedStyle(el, "::after").height));
  expect(Math.round(hit)).toBeGreaterThanOrEqual(44);
  const sectionTop = await firstSectionTop(page);
  expect(sectionTop).toBeLessThan(330);
  expect(sectionTop).toBeGreaterThan((await hint.boundingBox())!.y);
  const hitTop = await firstHitTop(page);
  console.log(`390 px with the hint: section fold row ${Math.round(sectionTop)} px, first entry hit ${Math.round(hitTop)} px`);
  expect(hitTop).toBeGreaterThan(sectionTop);
  expect(hitTop).toBeLessThanOrEqual(380);
  expect(await pageOverflow(page)).toBeLessThanOrEqual(0);
});

installedTest("Search results at 390 px: book sections fold into one row that opens the list", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search?q=fire");
  await waitResults(page, "fire");
  const fold = page.locator(".section-fold");
  const summary = fold.locator("> summary");
  // One 44 px Navigation row (a 6 px rectangle, never a pill) whose name carries the count.
  await expect(summary).toHaveAccessibleName(/^Book sections · [\d,]+ (match|matches)$/);
  const box = (await summary.boundingBox())!;
  expect(box.height).toBeGreaterThanOrEqual(44);
  expect(await summary.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("6px");
  // Folded: no section row is in the flow, and the entries are right under the row.
  await expect(fold).not.toHaveAttribute("open", /.*/);
  await expect(page.locator(".section-results li").first()).toBeHidden();
  const folded = await firstHitTop(page);
  expect(folded).toBeLessThanOrEqual(380);

  // One activation opens the list; focus stays on the row, and the entries move down. The toggle
  // replaces the history entry: the URL records the open row and Back has nothing extra to undo.
  const historyLength = await page.evaluate(() => history.length);
  await summary.click();
  await expect(fold).toHaveAttribute("open", /.*/);
  await expect(page).toHaveURL(/#\/search\?q=fire&sections=1$/);
  expect(await page.evaluate(() => history.length)).toBe(historyLength);
  await expect(summary).toBeFocused();
  const rows = page.locator(".section-results li");
  await expect(rows.first()).toBeVisible();
  for (const row of await rows.all()) {
    expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }
  expect(await firstHitTop(page)).toBeGreaterThan(folded);
  expect(await pageOverflow(page)).toBeLessThanOrEqual(0);

  // The open list ends inside the group's own bordered container, with space before the entries.
  expect(await fold.evaluate((el) => getComputedStyle(el).borderTopWidth)).toBe("1px");
  const group = (await fold.boundingBox())!;
  const lastRow = (await rows.last().boundingBox())!;
  expect(lastRow.y + lastRow.height).toBeLessThanOrEqual(group.y + group.height);
  expect((await page.locator(".results > .hit").first().boundingBox())!.y).toBeGreaterThanOrEqual(group.y + group.height + 8);

  // The keyboard folds the row again and the URL follows.
  await page.keyboard.press("Enter");
  await expect(fold).not.toHaveAttribute("open", /.*/);
  await expect(page).toHaveURL(/#\/search\?q=fire$/);
  await expect(summary).toBeFocused();

  // A new query folds an open row: the flag is not part of the next route.
  await summary.click();
  await expect(fold).toHaveAttribute("open", /.*/);
  await page.locator("#search-input").fill("fireball");
  await waitResults(page, "fireball");
  await expect(fold).not.toHaveAttribute("open", /.*/);
  await expect(page).not.toHaveURL(/sections=1/);
});

installedTest("Search results at 390 px: the filter panel opens in the page flow with focus management", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search?q=fireball");
  await waitResults(page, "fireball");
  const toggle = page.getByRole("button", { name: /^Filters/ });
  const panel = page.locator("#filters-panel");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(panel).toBeHidden();

  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(panel).toBeVisible();
  // Focus moves into the panel.
  await expect(panel).toBeFocused();
  // Non-modal and in flow: static, under the row, at most half the viewport, no dialog.
  expect(await panel.evaluate((el) => getComputedStyle(el).position)).toBe("static");
  const row = (await page.locator(".search-controls").boundingBox())!;
  const box = (await panel.boundingBox())!;
  expect(box.y).toBeGreaterThanOrEqual(row.y + row.height);
  const viewportHeight = page.viewportSize()!.height;
  expect(box.height).toBeLessThanOrEqual(viewportHeight / 2 + 1);
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(await pageOverflow(page)).toBeLessThanOrEqual(0);
  // The Kind chips inside are full-size targets.
  for (const chip of await panel.locator(".filter-group").first().locator(".chip").all()) {
    expect((await chip.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }

  // The count follows the active filters.
  await panel.locator(".chip", { hasText: /^Spells/ }).click();
  await expect(toggle).toHaveText("Filters · 1");
  await expect(panel).toBeVisible();

  // Escape closes the panel and returns focus to the button.
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(toggle).toBeFocused();

  // From the keyboard alone: Enter opens the panel and focus moves into it; Shift+Tab leaves it
  // for the button, and Enter there closes it. No mouse click touches the button, so a focus
  // that is on the button afterwards was not put there by the pointer.
  await toggle.focus();
  await page.keyboard.press("Enter");
  await expect(panel).toBeVisible();
  await expect(panel).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(toggle).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(panel).toBeHidden();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(toggle).toBeFocused();

  // After a resize to the desktop layout the button is hidden: Escape in the open panel closes
  // the state and does not try to give focus to the hidden button. focus() on a display:none
  // element is a silent no-op in Chromium, so the page looks the same either way; a spy on the
  // button (installed by the test, not by the app) records whether the app asked for focus.
  await toggle.click();
  await expect(panel).toBeFocused();
  await page.setViewportSize({ width: 1024, height: 800 });
  await expect(toggle).toBeHidden();
  await panel.locator(".chip input").first().focus();
  await page.evaluate(() => {
    const button = document.querySelector(".filters-toggle") as HTMLElement;
    (window as any).__toggleFocusCalls = 0;
    button.focus = () => { (window as any).__toggleFocusCalls++; };
  });
  await page.keyboard.press("Escape");
  // A role query skips hidden elements, so the hidden button is read by its class.
  await expect(page.locator(".filters-toggle")).toHaveAttribute("aria-expanded", "false");
  expect(await page.evaluate(() => (window as any).__toggleFocusCalls)).toBe(0);
  expect(await page.evaluate(() => document.activeElement === document.body)).toBe(false);
  await expect(panel.locator(".chip input").first()).toBeFocused();
});

installedTest("Publication groups fit 390 px with full-size targets", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");
  await page.getByRole("button", { name: /^Filters/ }).click();
  const group = page.locator('.filters .filter-group:has(legend:text-is("Publication"))');
  await expect(group.locator(".publication-group").first()).toBeVisible();
  await expect(group.locator(".publication-group").first()).toHaveAttribute("data-tier", "Rulebooks");
  const adventures = group.locator('details.publication-group[data-tier="Adventures"]');
  await expect(adventures).toHaveCount(1);
  await expect(adventures).not.toHaveAttribute("open", "");
  // Every tier that the budget skipped is a closed heading: full-size, inside the viewport.
  const headings = group.locator("details.publication-group > summary");
  expect(await headings.count()).toBeGreaterThanOrEqual(3);
  for (const heading of await headings.all()) {
    await expect(heading).toBeVisible();
    const box = (await heading.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  const summary = adventures.locator("summary");
  const summaryBox = (await summary.boundingBox())!;
  expect(summaryBox.height).toBeGreaterThanOrEqual(44);
  expect(summaryBox.x + summaryBox.width).toBeLessThanOrEqual(390);
  // Group headings and summaries are plain labels, never pills.
  const radii = await group.locator(".publication-group-title").evaluateAll((els) =>
    els.map((el) => getComputedStyle(el).borderRadius),
  );
  expect(radii.length).toBeGreaterThan(1);
  expect(radii.every((radius) => radius !== "999px")).toBe(true);
  const overflowClosed = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(overflowClosed).toBeLessThanOrEqual(0);
  await summary.click();
  await expect(adventures).toHaveAttribute("open", "");
  const chips = adventures.locator(".chip");
  expect(await chips.count()).toBeGreaterThan(0);
  for (const chip of await chips.all()) {
    const box = (await chip.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  const overflow = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Mobile Quick search is a full-screen layer with 44 px controls", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await expect(page.locator(".book-directory-row").first()).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 500));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);

  const icon = (await searchIcon(page).boundingBox())!;
  expect(Math.round(icon.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(icon.height)).toBeGreaterThanOrEqual(44);

  const layer = await openQuickSearch(page, { keyboard: true });
  // The layer animates in; measure its resting position, not a frame of the expand.
  await layer.evaluate((el) => Promise.all(el.getAnimations({ subtree: true }).map((a) => a.finished)));
  const box = (await layer.boundingBox())!;
  const viewport = page.viewportSize()!;
  // Sub-pixel offsets round to -0, which `toBe(0)` rejects under Object.is; the layer only
  // has to sit flush with the viewport.
  expect(Math.abs(box.x)).toBeLessThan(1);
  expect(Math.abs(box.y)).toBeLessThan(1);
  expect(Math.round(box.width)).toBe(viewport.width);
  expect(Math.round(box.height)).toBe(viewport.height);
  const field = (await quickField(page).boundingBox())!;
  const cancel = (await layer.getByRole("button", { name: "Cancel" }).boundingBox())!;
  expect(Math.round(field.height)).toBeGreaterThanOrEqual(44);
  expect(Math.round(cancel.height)).toBeGreaterThanOrEqual(44);
  expect(cancel.x + cancel.width).toBeLessThanOrEqual(viewport.width);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);

  await layer.getByRole("button", { name: "Cancel" }).click();
  await expect(layer).toBeHidden();
  expect(await page.evaluate(() => Math.round(window.scrollY))).toBe(before);
  await expect(searchIcon(page)).toBeFocused();
});

installedTest("Mobile Quick search rows are full-size touch targets", async ({ page }) => {
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  await quickField(page).fill("fireb");
  const options = layer.getByRole("option");
  await expect(options).not.toHaveCount(1);
  await expect(options.last()).toHaveText("See all results for “fireb”");
  const viewport = page.viewportSize()!;
  for (const option of await options.all()) {
    const box = (await option.boundingBox())!;
    expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  }
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  // The empty field lists recent searches with 44 px remove buttons.
  await options.last().click();
  await waitResults(page, "fireb");
  await page.goto("#/");
  await openQuickSearch(page);
  const remove = layer.getByRole("button", { name: /^Remove “fireb” from recent searches$/ });
  const box = (await remove.boundingBox())!;
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
});

installedTest("Spell lists at 390 px: Wizard is one tap from the hub, and rows show school, facts and rulebooks first", async ({ page }) => {
  await installLibrary(page);
  const overflow = () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

  // The hub opens with the class picker: Wizard sits inside the first screen, not below 30 cards.
  await page.goto("#/spells");
  const wizard = page.locator(".class-picker").first().getByRole("link", { name: /^Wizard spells,/ });
  await expect(wizard).toBeVisible();
  const box = (await wizard.boundingBox())!;
  expect(box.height).toBeGreaterThanOrEqual(44);
  expect(box.y + box.height, "Wizard is inside the first screen").toBeLessThanOrEqual(844);
  expect(await overflow()).toBeLessThanOrEqual(0);
  await wizard.click();
  await expect(page.getByRole("heading", { level: 1, name: "Wizard spells" })).toBeVisible();
  // The class page is a related-page link in the filter row, beside the school menu.
  const classPage = page.getByRole("link", { name: "Wizard class page" });
  await expect(classPage).toHaveClass(/page-link/);
  expect((await classPage.boundingBox())!.height).toBeGreaterThanOrEqual(44);

  // Level 5: two-line rows with the school, the facts and the book. Expectations come from the corpus.
  await page.goto("#/spells/Wizard/5");
  const rows = page.locator(".spell-rows .spell-row");
  await expect(rows.first()).toBeVisible();
  await expect(page.locator(".spell-list")).toHaveAttribute("data-density", "wide");
  const total = await rows.count();
  expect(total).toBeGreaterThan(10);
  for (const index of [0, 1, 2, 3, 4]) {
    const row = rows.nth(index);
    await expect(row.locator(".spell-school"), `row ${index} school`).not.toHaveText("");
    await expect(row.locator(".spell-row-meta"), `row ${index} facts`).not.toHaveText("");
  }
  const cone = rows.filter({ has: page.getByRole("link", { name: "Cone of Cold", exact: true }) }).first();
  await expect(cone.locator(".spell-school")).toHaveText("Evocation");
  await expect(cone.locator(".spell-row-meta")).toContainText("1 standard action");
  await expect(cone.locator(".spell-row-meta")).toContainText("Reflex half");
  await expect(cone.locator(".spell-row-meta")).toContainText("Core Rulebook");
  // The name never wraps and a row is at least 44 px tall.
  const heights = await rows.evaluateAll((nodes) => nodes.map((n) => n.getBoundingClientRect().height));
  expect(Math.min(...heights)).toBeGreaterThanOrEqual(44);
  const names = await page.locator(".spell-name").evaluateAll((nodes) =>
    nodes.map((n) => n.getClientRects().length === 1 && n.getBoundingClientRect().height < 30));
  expect(names.every(Boolean)).toBe(true);
  expect(await overflow()).toBeLessThanOrEqual(0);

  // A Core spell comes before a spell that only an adventure prints.
  const category = new Map<string, string>(corpus.catalog.sources.map((s: any) => [s.id, s.category]));
  const stubs = new Map(corpus.catalog.entries.map((stub) => [stub.id, stub]));
  const wizardFive = entries.filter((entry) => entry.kind === "spell" &&
    entry.fields?.levels?.some((level: any) => level.class.toLowerCase() === "wizard" && level.level === 5));
  const adventure = wizardFive.find((entry) =>
    ["Adventure Path", "Module"].includes(category.get(stubs.get(entry.id)!.source) ?? ""));
  expect(adventure, "an adventure-only Wizard 5 spell in the corpus").toBeTruthy();
  const order = await page.locator(".spell-rows a.spell-name").evaluateAll((nodes) => nodes.map((n) => (n as HTMLAnchorElement).hash));
  const at = (id: string) => order.indexOf(`#/entry/${id}`);
  expect(at("ref/spell/cone-of-cold")).toBeGreaterThanOrEqual(0);
  expect(at(adventure!.id)).toBeGreaterThan(at("ref/spell/cone-of-cold"));
  await expect(page.locator(".tier-head").first()).toHaveText("Rulebooks");

  // The school menu narrows the list in place; its panel opens in the page flow under the row.
  const menu = page.getByRole("button", { name: "School: All schools", exact: true });
  await menu.click();
  const panel = page.locator(`#${await menu.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  // The counts are this level's: together they add up to the rows on the page, and the chosen
  // school's count is the number of rows left. Every row is a 44 px target.
  const counts = (await panel.locator("input[type=checkbox] ~ .choice-count").allInnerTexts()).map(Number);
  expect(counts.reduce((a, b) => a + b, 0)).toBe(total);
  const rowHeights = await panel.locator(".choice-row").evaluateAll((els) => els.map((el) => el.getBoundingClientRect().height));
  expect(rowHeights.filter((height) => height < 43.5)).toEqual([]);
  const box390 = (await panel.boundingBox())!;
  expect(box390.x).toBeGreaterThanOrEqual(0);
  expect(box390.x + box390.width).toBeLessThanOrEqual(390);
  const evocation = panel.getByRole("checkbox", { name: /^Evocation, / });
  const evocationCount = Number((await evocation.getAttribute("aria-label"))!.match(/(\d+) spells?$/)![1]);
  await evocation.check();
  await expect(page.getByRole("button", { name: "School: Evocation", exact: true })).toBeVisible();
  await expect(rows.first()).toBeVisible();
  const schools = await page.locator(".spell-school").allInnerTexts();
  expect(schools.length).toBe(evocationCount);
  expect(new Set(schools)).toEqual(new Set(["Evocation"]));
  await page.getByRole("checkbox", { name: /^Conjuration, / }).check();
  await expect(page.getByRole("button", { name: "School: 2 chosen", exact: true })).toBeVisible();
  expect(new Set(await page.locator(".spell-school").allInnerTexts())).toEqual(new Set(["Evocation", "Conjuration"]));
  expect(await overflow()).toBeLessThanOrEqual(0);
});

installedTest("Spell list density at 390 px: Compact is two lines, toggles without a reload and survives one", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/spells/${encodeURIComponent(listClass)}/${listLevel}`);
  const list = page.locator(".spell-list");
  // Wide is the default: the gist sits under the facts on a phone, so the fact line has two rows.
  await expect(list).toHaveAttribute("data-density", "wide");
  const row = page.locator(".spell-row:has(.spell-gist)").first();
  const meta = row.locator(".spell-row-meta");
  await expect(meta).toBeVisible();
  const lineHeight = await meta.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await meta.boundingBox())!.height).toBeGreaterThan(lineHeight * 1.5);

  // Both choices are 44 px Filter-role targets in one named group.
  const toggle = page.getByRole("group", { name: "Spell row density" });
  await expect(toggle).toBeVisible();
  for (const choice of await toggle.getByRole("radio").all()) {
    const box = (await choice.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
  }
  // A marker that a reload would clear proves the toggle does not navigate.
  await page.evaluate(() => ((window as any).__densityMarker = "kept"));

  await page.getByRole("radio", { name: "Compact" }).check();
  await expect(list).toHaveAttribute("data-density", "compact");
  expect((await meta.boundingBox())!.height).toBeLessThan(lineHeight * 1.5);
  expect(await page.evaluate(() => (window as any).__densityMarker)).toBe("kept");

  // Rows stay at least 44 px tall and the name never wraps in either mode.
  const heights = await page.locator(".spell-rows .spell-row").evaluateAll((nodes) =>
    nodes.map((n) => n.getBoundingClientRect().height));
  expect(Math.min(...heights)).toBeGreaterThanOrEqual(44);
  const names = await page.locator(".spell-name").evaluateAll((nodes) =>
    nodes.map((n) => n.getClientRects().length === 1 && n.getBoundingClientRect().height < 30));
  expect(names.every(Boolean)).toBe(true);

  // The choice is stored, not just held in the page.
  await page.reload();
  await expect(list).toHaveAttribute("data-density", "compact", { timeout: 30000 });
  const metaAfter = page.locator(".spell-row:has(.spell-gist) .spell-row-meta").first();
  await expect(metaAfter).toBeVisible();
  const lineHeightAfter = await metaAfter.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await metaAfter.boundingBox())!.height).toBeLessThan(lineHeightAfter * 1.5);
});

const homeSpells = entries.filter((entry) => entry.kind === "spell")
  .filter((entry, i, all) => all.findIndex((c) => c.title === entry.title) === i)
  .map((entry) => ({ id: entry.id as string, title: entry.title as string }));

installedTest("Home at 390 px: Categories starts in the first viewport with no stars or recent history", async ({ page }) => {
  await installLibrary(page);
  const top = await page.locator("#by-kind").evaluate((el) => el.getBoundingClientRect().top);
  expect(top).toBeLessThan(page.viewportSize()!.height);
  await expect(page.getByRole("heading", { name: "Starred" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Recently viewed" })).toHaveCount(0);
});

installedTest("Quick search at 390 px: recently viewed rows are full-size touch targets", async ({ page }) => {
  await installLibrary(page);
  await seedHome(page, { recents: homeSpells.slice(0, 3) });
  await openQuickSearch(page);
  const viewed = quickGroup(page, "Recently viewed");
  await expect(viewed).toHaveCount(3);
  const viewport = page.viewportSize()!;
  for (const row of await viewed.all()) {
    const box = (await row.boundingBox())!;
    expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  }
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.keyboard.press("Escape");
});

installedTest("Quick search at 390 px: twelve recents take five rows and a Show more control", async ({ page }) => {
  await installLibrary(page);
  await seedHome(page, { recents: homeSpells.slice(0, 12) });
  await openQuickSearch(page);
  const viewed = quickGroup(page, "Recently viewed");
  await expect(viewed).toHaveCount(5);
  const more = page.getByRole("button", { name: /^Show more/ });
  await expect(more).toBeVisible();
  const last = (await viewed.last().boundingBox())!;
  const moreBox = (await more.boundingBox())!;
  expect(moreBox.y).toBeGreaterThan(last.y);
  for (const row of await viewed.all()) {
    expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Feats list at 390 px: title, count, filters and a sticky 48 px letter bar, with rows after them", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Feats" })).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText(/^[\d,]+ feats$/);
  await expect(main.locator("nav.crumbs")).toHaveCount(0);
  await expect(main.locator(".lede-line")).toHaveCount(0);
  const nav = main.getByRole("navigation", { name: "Jump to letter" });
  await expect(nav).toBeVisible();
  expect(Math.round((await nav.boundingBox())!.height)).toBe(48);
  // The bar sticks under the 60 px masthead once the page scrolls past it, and nothing else
  // below the masthead sticks.
  await page.evaluate(() => window.scrollTo(0, 1200));
  const header = (await page.locator("header.app-header").boundingBox())!;
  // (The masthead is 60 px plus its 1 px rule; the bar sits at 60 px, under the rule.)
  await expect.poll(async () => Math.abs((await nav.boundingBox())!.y - (header.y + header.height))).toBeLessThanOrEqual(1);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Feats at 390 px: the first feat row is in the first viewport, and the type menu works beside an open panel", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const main = page.locator("main");
  const button = main.getByRole("button", { name: /^Type: / });
  await expect(button).toBeEnabled();
  const viewport = page.viewportSize()!;
  const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
  const first = main.locator(".letter-group .entry-list a").first();
  await expect(first).toBeVisible();
  const y = (await first.boundingBox())!.y;
  console.log(`first feat row y at 390 px: ${Math.round(y)}`);
  expect(y).toBeLessThan(tabBar.y - 44);
  expect(y).toBeLessThan(viewport.height / 2);
  // The filter row is one row: the menu and the Feat chains link share a line, inside the width.
  const link = main.locator(".filter-row").getByRole("link", { name: "Feat chains" });
  const menu = (await button.boundingBox())!;
  const chains = (await link.boundingBox())!;
  expect(Math.abs(menu.y - chains.y)).toBeLessThan(8);
  expect(chains.x + chains.width).toBeLessThanOrEqual(viewport.width);
  expect(Math.round(menu.height)).toBeGreaterThanOrEqual(44);

  // The open panel is in the page, at most half the viewport, and its rows are 44 px.
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  const box = (await panel.boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(viewport.height * 0.5 + 1);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  const rows = await panel.locator(".choice-row").evaluateAll((els) => els.slice(0, 8).map((el) => el.getBoundingClientRect().height));
  expect(rows.filter((height) => height < 43.5)).toEqual([]);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  // Choosing a type closes it, filters, and leaves the button focused.
  await panel.getByRole("searchbox").fill("comb");
  await panel.getByRole("link", { name: /^Combat, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat?facet=feat_type:Combat");
  await expect(main.getByRole("button", { name: "Type: Combat" })).toBeFocused();
  await expect(main.getByRole("button", { name: "Type: Combat" })).toHaveAttribute("aria-expanded", "false");
});

installedTest("Scoped Quick search at 390 px: the scope pill is a 44 px target and fits, also for a long kind name", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  for (const [hash, name] of [["#/browse/kind/feat", "Feats"], ["#/browse/kind/prestige_class", "Prestige classes"]] as const) {
    await page.goto(hash);
    const layer = await openQuickSearch(page);
    const pill = layer.getByRole("button", { name: `Remove scope: ${name}`, exact: true });
    await expect(pill).toBeVisible();
    await expect(quickField(page)).toHaveAttribute("placeholder", `Search ${name.toLowerCase()}`);
    const box = (await pill.boundingBox())!;
    expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    await pill.click();
    await expect(pill).toHaveCount(0);
    await expect(quickField(page)).toBeFocused();
    await layer.getByRole("button", { name: "Cancel" }).click();
    await expect(layer).toBeHidden();
  }
});

installedTest("Every other kind page at 390 px shows its first row in the first viewport, with no horizontal scroll", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  const main = page.locator("main");
  const kinds: [string, string][] = [
    ["spell", "Spells"],
    ["trait", "Traits"],
    ["item", "Items"],
    ["archetype", "Archetypes"],
    ["class", "Classes"],
  ];
  const report: string[] = [];
  for (const [kind, title] of kinds) {
    await page.goto(`#/browse/kind/${kind}`);
    await expect(main.getByRole("heading", { level: 1, name: title })).toBeVisible();
    const menu = main.locator(".filter-btn");
    if (kind !== "class") await expect(menu).toBeEnabled();
    else await expect(menu).toHaveCount(0);
    const first = main.locator(".entry-list a").first();
    await expect(first).toBeVisible();
    const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
    const y = (await first.boundingBox())!.y;
    report.push(`${kind} ${Math.round(y)}`);
    expect(y, `${kind} first row`).toBeLessThan(tabBar.y - 44);
    expect(y, `${kind} first row`).toBeLessThan(viewport.height / 2);
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    expect(overflow, `${kind} overflow`).toBeLessThanOrEqual(0);
    // A menu is one 44 px button; the pages beside it share its line.
    if (kind !== "class") expect(Math.round((await menu.boundingBox())!.height)).toBeGreaterThanOrEqual(44);
  }
  console.log(`first row y at 390 px: ${report.join(", ")}`);
});

installedTest("Items at 390 px: the category menu opens in the page, filters, and Back and Escape return", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  await page.goto("#/browse/kind/item");
  const main = page.locator("main");
  const button = main.getByRole("button", { name: /^Category: / });
  await expect(button).toBeEnabled();
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  const box = (await panel.boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(viewport.height * 0.5 + 1);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  const rows = await panel.locator(".choice-row").evaluateAll((els) => els.slice(0, 8).map((el) => el.getBoundingClientRect().height));
  expect(rows.filter((height) => height < 43.5)).toEqual([]);
  // Escape closes it and returns focus to the button.
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(button).toBeFocused();
  expect(new URL(page.url()).hash).toBe("#/browse/kind/item");
  // A choice filters, and Back undoes it.
  await button.click();
  await panel.getByRole("searchbox").fill("rods");
  await panel.getByRole("link", { name: /^Rods, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/item?facet=item_category:Rods");
  await expect(main.getByRole("button", { name: "Category: Rods" })).toBeFocused();
  const first = main.locator(".entry-list a").first();
  await expect(first).toBeVisible();
  const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
  const y = (await first.boundingBox())!.y;
  console.log(`first item row y at 390 px, filtered to Rods: ${Math.round(y)}`);
  expect(y).toBeLessThan(tabBar.y - 44);
  await page.goBack();
  await expect(main.getByRole("button", { name: "Category: All categories" })).toBeVisible();
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Classes at 390 px: the group headings are one line and the last group is reachable by scrolling", async ({ page }) => {
  await installLibrary(page);
  const types = new Set(
    entries
      .filter((entry) => entry.kind === "class")
      .map((entry) => entry.fields?.class_type)
      .filter((type): type is string => Boolean(type)),
  );
  const ungrouped = entries.some((entry) => entry.kind === "class" && !entry.fields?.class_type);
  await page.goto("#/browse/kind/class");
  const main = page.locator("main");
  const heads = main.locator("h2.letter-head");
  await expect(heads).toHaveCount(types.size + (ungrouped ? 1 : 0));
  const heights = await heads.evaluateAll((els) => els.map((el) => el.getBoundingClientRect().height));
  expect(Math.max(...heights)).toBeLessThan(48);
  const rows = await main.locator(".entry-list li").evaluateAll((els) => els.map((el) => el.getBoundingClientRect().height));
  expect(Math.min(...rows)).toBeGreaterThanOrEqual(44);
  const last = heads.nth((await heads.count()) - 1);
  await last.scrollIntoViewIfNeeded();
  await expect(last).toBeInViewport();
});

// ---- Book-filtered kind lists (#158) --------------------------------------------------------

installedTest("Book kind links stay rectangles at 390 px and become plain 44 px rows at 320 px", async ({ page }) => {
  await installLibrary(page);
  const book = bookTrees().find((candidate) => candidate.source === "core-rulebook") ?? bookTrees()[0];
  await page.goto(`#/browse/book/${book.source}`);
  const links = page.locator(".book-in-this-book .jump");
  await expect(links.first()).toBeVisible();
  const style = (locator: ReturnType<typeof links.first>) =>
    locator.evaluate((element) => {
      const css = getComputedStyle(element);
      const box = element.getBoundingClientRect();
      return {
        radius: css.borderTopLeftRadius,
        background: css.backgroundColor,
        borderTop: css.borderTopWidth,
        borderBottom: css.borderBottomWidth,
        height: box.height,
        width: box.width,
      };
    });

  // 390 px: the navigation rectangles, with a surface fill and a card radius.
  const wide = await style(links.first());
  expect(wide.radius).toBe("6px");
  expect(wide.background).not.toBe("rgba(0, 0, 0, 0)");
  expect(wide.borderTop).toBe("1px");
  expect(wide.height).toBeGreaterThanOrEqual(44);

  // 320 px: plain rows, each with a hairline separator, the label left and the count right.
  await page.setViewportSize({ width: 320, height: 844 });
  await expect(links.first()).toBeVisible();
  const narrow = await style(links.first());
  expect(narrow.radius).toBe("0px");
  expect(narrow.background).toBe("rgba(0, 0, 0, 0)");
  expect(narrow.borderTop).toBe("0px");
  expect(narrow.borderBottom).toBe("1px");
  expect(narrow.height).toBeGreaterThanOrEqual(44);
  const label = (await links.first().locator(".book-kind-label").boundingBox())!;
  const tally = (await links.first().locator(".book-kind-count").boundingBox())!;
  expect(label.x).toBeLessThan(tally.x);
  expect(tally.x + tally.width).toBeLessThanOrEqual(320);
  expect(narrow.width).toBeLessThanOrEqual(320);
  const overflow = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Creatures at 390 px: two filter menus and the first row in the first viewport", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/creature");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Creatures" })).toBeVisible();
  const typeButton = main.getByRole("button", { name: /^Type: / });
  const crButton = main.getByRole("button", { name: /^CR: / });
  await expect(typeButton).toBeEnabled();
  await expect(crButton).toBeEnabled();
  // Both menus share the filter row and are full touch targets inside the screen.
  for (const button of [typeButton, crButton]) {
    const box = (await button.boundingBox())!;
    expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  expect(Math.abs((await typeButton.boundingBox())!.y - (await crButton.boundingBox())!.y)).toBeLessThan(8);

  const first = main.locator(".letter-group .entry-list > li").first();
  await expect(first).toBeVisible();
  const viewport = page.viewportSize()!;
  const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
  const y = (await first.boundingBox())!.y;
  console.log(`first creature row y at 390 px: ${Math.round(y)}`);
  expect(y).toBeLessThan(tabBar.y - 44);
  expect(y).toBeLessThan(viewport.height / 2);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);

  // The type panel opens in the page and its rows are 44 px targets.
  await typeButton.click();
  const panel = page.locator(`#${await typeButton.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  const box = (await panel.boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(viewport.height * 0.5 + 1);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  const rows = await panel.locator(".choice-row").evaluateAll((els) => els.slice(0, 8).map((el) => el.getBoundingClientRect().height));
  expect(rows.filter((height) => height < 43.5)).toEqual([]);
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(typeButton).toBeFocused();
});

/** A table that fits may differ in a few fringe pixels; a table that scrolls differs by thousands. */
const CUE_TOLERANCE_PIXELS = 50;

/**
 * Whether the scroll cues of a progression table paint anything. The wrap's own background carries
 * both cues (the pinned-edge line and shadow, and the right-edge shadow), so the check screenshots
 * the wrap, paints the same wrap with a plain surface background, and counts the pixels that
 * differ. A table that fits stays within `CUE_TOLERANCE_PIXELS`. A pseudo-element on a header
 * cell counts as a cue.
 */
async function cuePaints(page: Page, wrap: Locator): Promise<boolean> {
  // A cue drawn on a pinned cell itself (a pseudo-element) would show whether or not the table
  // scrolls, so no header cell may have one.
  const pseudo = await wrap.evaluate((el) =>
    [...el.querySelectorAll("th")].flatMap((th) => [getComputedStyle(th, "::before").content, getComputedStyle(th, "::after").content]));
  if (pseudo.some((content) => content !== "none" && content !== "normal")) return true;
  await wrap.scrollIntoViewIfNeeded();
  const top = (await wrap.boundingBox())!.y;
  await page.evaluate((delta) => window.scrollBy({ top: delta, behavior: "instant" }), top - 120);
  // Wait until the box has stopped moving before the screenshot.
  let box = (await wrap.boundingBox())!;
  for (let i = 0; i < 20; i++) {
    await page.waitForTimeout(50);
    const next = (await wrap.boundingBox())!;
    const still = next.x === box.x && next.y === box.y && next.width === box.width && next.height === box.height;
    box = next;
    if (still) break;
  }
  // Inset from the rounded corners, whose antialiasing differs between a layered and a plain background.
  const clip = { x: box.x + 8, y: box.y + 8, width: box.width - 16, height: Math.min(box.height, 400) - 8 };
  const cued = await page.screenshot({ clip });
  await wrap.evaluate((el) => el.style.setProperty("background", "var(--surface)", "important"));
  const plain = await page.screenshot({ clip });
  await wrap.evaluate((el) => el.style.removeProperty("background"));
  return differingPixels(cued, plain) > CUE_TOLERANCE_PIXELS;
}

installedTest("Class progression at 390 px: Level stays pinned, the core columns fit, and spells per day sits in a disclosure", async ({ page }) => {
  await installLibrary(page);
  const overflow = () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

  await page.goto("#/entry/ref/class/wizard");
  const table = page.locator(".progression-table").first();
  await expect(table).toBeVisible();
  const wrap = page.locator(".table-wrap:has(> .progression-table)").first();
  await wrap.scrollIntoViewIfNeeded();

  // Level, Base Attack Bonus, the three saves and Special all sit inside the viewport.
  const headers = table.locator("thead th");
  await expect(headers).toHaveText(["Level", "Base Attack Bonus", "Fort Save", "Ref Save", "Will Save", "Special"]);
  for (let i = 0; i < 6; i++) {
    const box = (await headers.nth(i).boundingBox())!;
    expect(box.x, `column ${i} starts on screen`).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, `column ${i} ends on screen`).toBeLessThanOrEqual(390);
  }
  expect(await overflow()).toBeLessThanOrEqual(0);
  expect(await wrap.evaluate((el) => el.scrollWidth - el.clientWidth), "the core table needs no sideways scroll").toBeLessThanOrEqual(0);
  expect(await cuePaints(page, wrap), "Wizard's core table fits and shows no cue").toBe(false);

  // The spell columns are not in this table; they sit in a closed disclosure with the slot table.
  const disclosure = page.locator("details.progression-spells").first();
  await expect(disclosure.locator("summary")).toHaveText("Spells per day");
  expect((await disclosure.locator("summary").boundingBox())!.height).toBeGreaterThanOrEqual(44);
  await expect(disclosure).not.toHaveAttribute("open", "");
  await disclosure.locator("summary").click();
  const slots = disclosure.locator(".progression-table");
  await expect(slots).toBeVisible();
  await expect(slots.locator("thead th")).toHaveText(["Level", "0", "1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th", "9th"]);
  await expect(slots.locator("tbody tr")).toHaveCount(20);
  expect(await overflow()).toBeLessThanOrEqual(0);
  expect(await cuePaints(page, disclosure.locator(".table-wrap")), "Wizard's slot table fits and shows no cue").toBe(false);

  // The spell columns are out of the core table, so a Wizard needs no sideways scroll. Monk has
  // four more columns, so its table scrolls. Level keeps its left edge and its opaque background
  // while the other columns move under it.
  await page.goto("#/entry/ref/class/monk");
  const monk = page.locator(".table-wrap:has(> .progression-table)").first();
  await expect(monk).toBeVisible();
  await monk.scrollIntoViewIfNeeded();
  expect(await monk.evaluate((el) => el.scrollWidth - el.clientWidth), "Monk's table scrolls").toBeGreaterThan(100);
  const pinned = monk.locator("tbody tr").nth(9).locator("th").first();
  await expect(pinned).toHaveText("10th");
  const before = (await pinned.boundingBox())!;
  const lastBefore = (await monk.locator("tbody tr").nth(9).locator("td").last().boundingBox())!;
  await monk.evaluate((el) => { el.scrollLeft = el.scrollWidth; });
  const after = (await pinned.boundingBox())!;
  const lastAfter = (await monk.locator("tbody tr").nth(9).locator("td").last().boundingBox())!;
  expect(lastAfter.x, "the other columns moved").toBeLessThan(lastBefore.x - 100);
  expect(Math.abs(after.x - before.x), "Level keeps its left edge").toBeLessThanOrEqual(1);
  const background = await pinned.evaluate((el) => getComputedStyle(el).backgroundColor);
  expect(background, "pinned cell is opaque").not.toMatch(/rgba|transparent|\//);
  // The pinned edge and its shadow show once the content has scrolled away from the start, and
  // not before: at the end of the scroll only the left cue paints.
  expect(await cuePaints(page, monk), "Monk's scrolled table shows its cues").toBe(true);
  await monk.evaluate((el) => { el.scrollLeft = 0; });
  expect(await cuePaints(page, monk), "Monk at the start shows the right-edge cue").toBe(true);
  await monk.evaluate((el) => { el.scrollLeft = el.scrollWidth; });
  const headBackground = await monk.locator("thead th").first().evaluate((el) => getComputedStyle(el).backgroundColor);
  expect(headBackground, "pinned header is opaque").not.toMatch(/rgba|transparent|\//);
  expect(await overflow()).toBeLessThanOrEqual(0);

  // A non-caster has no disclosure, and its core columns fit as well.
  await page.goto("#/entry/ref/class/fighter");
  await expect(page.locator(".progression-table").first()).toBeVisible();
  await expect(page.locator("details.progression-spells")).toHaveCount(0);
  expect(await overflow()).toBeLessThanOrEqual(0);
  // Special keeps at least 90 px, and the iterative attack bonus of level 20 takes one value per
  // line: four lines, all at one left edge, none broken inside a value.
  const fighter = page.locator(".table-wrap:has(> .progression-table)").first();
  expect((await fighter.locator("thead th").last().boundingBox())!.width, "Special width").toBeGreaterThanOrEqual(90);
  const parts = await fighter.locator("tbody tr").last().locator("td").first().locator(".atk").evaluateAll(
    (nodes) => nodes.map((n) => { const r = n.getBoundingClientRect(); return { text: n.textContent, x: r.x, y: r.y, h: r.height }; }));
  expect(parts.map((p) => p.text)).toEqual(["+20/", "+15/", "+10/", "+5"]);
  const xs = parts.map((p) => p.x);
  expect(Math.max(...xs) - Math.min(...xs), "one left edge").toBeLessThanOrEqual(1);
  for (let i = 1; i < parts.length; i++) {
    expect(Math.abs(parts[i].y - (parts[i - 1].y + parts[i - 1].h)), `value ${i} follows the previous line`).toBeLessThanOrEqual(1);
  }
  // A two-part bonus (level 6, +6/+1) stays on one line while it fits.
  const two = await fighter.locator("tbody tr").nth(5).locator("td").first().locator(".atk").evaluateAll(
    (nodes) => nodes.map((n) => ({ text: n.textContent, y: n.getBoundingClientRect().y })));
  expect(two.map((p) => p.text)).toEqual(["+6/", "+1"]);
  expect(Math.abs(two[0].y - two[1].y), "+6/+1 on one line").toBeLessThanOrEqual(1);
  // A table that fits shows no scroll cue, in either theme.
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    expect(await cuePaints(page, fighter), `Fighter ${scheme}: no cue on a table that fits`).toBe(false);
  }
});

installedTest("Class progression at 1280 px: the spells table has the core table's type size", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await installLibrary(page);
  await page.goto("#/entry/ref/class/wizard");
  const core = page.locator(".progression-table").first();
  await expect(core).toBeVisible();
  const disclosure = page.locator("details.progression-spells").first();
  await disclosure.locator("summary").click();
  const size = (table: Locator) => table.evaluate((el) => getComputedStyle(el).fontSize);
  expect(await size(disclosure.locator(".progression-table"))).toBe(await size(core));
  // Nothing scrolls at this width, so neither table shows a cue.
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    expect(await cuePaints(page, page.locator(".table-wrap:has(> .progression-table)").first()), `core ${scheme}`).toBe(false);
    expect(await cuePaints(page, disclosure.locator(".table-wrap")), `slots ${scheme}`).toBe(false);
  }
});

installedTest("Spell list at 390 px: the name field clears the filter row, and a long scope wraps in the Quick search pill", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  await page.goto("#/spells/Wizard/3");
  const gap = await page.evaluate(() => {
    const row = document.querySelector("main .filter-row")!.getBoundingClientRect();
    const field = document.querySelector("main .list-filter")!.getBoundingClientRect();
    return field.top - row.bottom;
  });
  expect(gap).toBeGreaterThanOrEqual(11.5);

  // Four schools: the scope is long. The pill shows all of it, on two lines or more, and the
  // cross at the right is still one 44 px target with the text.
  await page.goto("#/spells/Wizard/3?school=evocation&school=conjuration&school=illusion&school=necromancy");
  const layer = await openQuickSearch(page);
  const name = "Remove scope: Evocation or Conjuration or Illusion or Necromancy Wizard spells";
  const pill = layer.getByRole("button", { name, exact: true });
  await expect(pill).toBeVisible();
  const box = (await pill.boundingBox())!;
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  const text = pill.locator("span");
  const lineHeight = await text.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await text.boundingBox())!.height).toBeGreaterThan(lineHeight * 1.5);
  // Nothing is cut: the text fits its box and the cross sits inside the pill.
  expect(await text.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
  await expect(pill.locator("svg")).toBeVisible();
  await pill.click();
  await expect(pill).toHaveCount(0);
});
