import { test, expect, loadCorpus } from "./fixtures";
import { bookOwners, bookTrees, CONTENTS_HIDDEN_KINDS, installLibrary, openBookGroups } from "./helpers";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const grouped = (n: number) => n.toLocaleString("en-US");

/** The tier table, restated independently of the Rust code: this is what the page promises. */
const tiers: { label: string; categories: string[]; open: boolean }[] = [
  { label: "Rulebooks", categories: ["RPG"], open: true },
  { label: "Player Companions", categories: ["Player Companion"], open: false },
  { label: "Campaign Setting", categories: ["Campaign Setting"], open: false },
  { label: "Adventures", categories: ["Adventure Path", "Module", "Society"], open: false },
  { label: "Other", categories: [], open: false },
];
const tierOf = (source: any) =>
  tiers.find((tier) => tier.categories.includes(source.category))?.label ?? "Other";
const inTier = (label: string) => sources.filter((source) => tierOf(source) === label);

/** `2009-08-01` as the page writes it: a fixed English month table, then the year. */
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const monthYear = (date: string | undefined) => {
  const match = /^(\d{4})(?:-(\d{2}))?/.exec(date ?? "");
  if (!match) return undefined;
  const month = match[2] ? Number(match[2]) : 0;
  return month >= 1 && month <= 12 ? `${MONTHS[month - 1]} ${match[1]}` : match[1];
};
/** Entries per book as Books counts them: catalog entries by their canonical source (the first book that places them). */
const owners = bookOwners();
const entriesOf = (source: any): number =>
  corpus.catalog.entries.filter((stub: any) => (owners.get(stub.id) ?? stub.source) === source.id).length;

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Books opens with a title, one count line and a link to Active books, with no lede or sort", async ({ page }) => {
  await page.goto("#/books");
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  const count = page.locator(".books-count");
  // Every book active is the Paizo Everything set.
  await expect(count).toHaveText(`${grouped(sources.length)} of ${grouped(sources.length)} books active · Paizo Everything`);
  await expect(count).toHaveAttribute("aria-live", "polite");
  await expect(page.getByRole("link", { name: "Active books", exact: true })).toHaveAttribute("href", "#/books/active");

  // Nothing between the count line and the groups explains itself.
  await expect(page.locator(".wide > p")).toHaveCount(1);
  await expect(page.locator(".hint, .count-scope, .measure")).toHaveCount(0);
  await expect(page.getByText(/Open a book to read|Active books appear in search|Disabling a book/)).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "Sort books" })).toHaveCount(0);
  await expect(page.locator(".book-sort, .sort-options")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /Activate all|Deactivate all|Only Rulebooks/ })).toHaveCount(0);
});

test("An old ?sort= link opens Books, and the address says so", async ({ page }) => {
  for (const sort of ["category", "date", "library", "title"]) {
    await page.goto(`#/books?sort=${sort}`);
    await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
    await expect(page).toHaveURL(/#\/books$/);
    await expect(page.locator(".book-tier").first()).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Sort books" })).toHaveCount(0);
  }
  // The redirect replaces the entry: one Back leaves Books instead of landing on the old URL.
  await page.goto("#/settings");
  await page.goto("#/books?sort=date");
  await expect(page).toHaveURL(/#\/books$/);
  await page.goBack();
  await expect(page).toHaveURL(/#\/settings$/);
});

test("Groups are h2 disclosure buttons in tier order: Rulebooks open, the rest closed, with a muted count", async ({ page }) => {
  await page.goto("#/books");
  const present = tiers.filter((tier) => inTier(tier.label).length > 0);
  const groups = page.locator(".book-tier");
  await expect(groups).toHaveCount(present.length);
  expect(present[0].label).toBe("Rulebooks");
  for (const [index, tier] of present.entries()) {
    const group = groups.nth(index);
    const members = inTier(tier.label);
    const heading = group.getByRole("heading", { level: 2 });
    await expect(heading).toContainText(tier.label);
    const toggle = heading.getByRole("button");
    await expect(toggle).toHaveAttribute("aria-expanded", tier.open ? "true" : "false");
    const list = group.locator("ul.book-directory");
    expect(await toggle.getAttribute("aria-controls")).toBe(await list.getAttribute("id"));
    // Every book starts active, so the count is the plain number of books.
    await expect(group.locator(".tier-active")).toHaveText(`${grouped(members.length)} ${members.length === 1 ? "book" : "books"}`);
    await expect(group.locator(".tier-chevron")).toHaveCount(1);
    expect((await toggle.boundingBox())!.height).toBeGreaterThanOrEqual(44);
    // A closed group keeps its rows out of reach until it opens.
    if (tier.open) await expect(list.locator(".book-directory-item").first()).toBeVisible();
    else await expect(list.locator(".book-directory-item").first()).toBeHidden();
    const ids = await list.locator(".book-directory-item").evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
    expect([...ids].sort()).toEqual(members.map((source) => source.id).sort());
  }
  await expect(page.locator(".book-directory-item")).toHaveCount(sources.length);

  // The group heading is a heading with one button; nothing else sits in it, and no group acts.
  await expect(page.locator(".book-group-head a, .book-group-head input, .book-group-head button")).toHaveCount(present.length);
  await expect(page.locator("details.tier-details, .tier-summary, .tier-actions, .books-preset")).toHaveCount(0);

  // Opening and closing flips aria-expanded and the rows.
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  const toggle = adventures.getByRole("button");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  await toggle.press("Enter");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(adventures.locator(".book-directory-item").first()).toBeHidden();
});

test("A group heading looks different from a book row", async ({ page }) => {
  await page.goto("#/books");
  const heading = page.locator(".book-group-head").first();
  const title = page.locator(".book-directory-row .cat-name").first();
  const style = (locator: typeof heading) =>
    locator.evaluate((el) => {
      const css = getComputedStyle(el);
      return { family: css.fontFamily, weight: css.fontWeight, size: parseFloat(css.fontSize), background: css.backgroundColor, border: css.borderTopWidth };
    });
  const group = await style(heading);
  const row = await style(title);
  // Serif 700 above sans 600, larger, and with no fill or box.
  expect(group.family).not.toBe(row.family);
  expect(Number(group.weight)).toBe(700);
  expect(Number(row.weight)).toBe(600);
  expect(group.size).toBeGreaterThan(row.size);
  expect(group.background).toBe("rgba(0, 0, 0, 0)");
  expect(group.border).toBe("0px");
  const toggle = heading.getByRole("button");
  expect(await toggle.evaluate((el) => getComputedStyle(el).backgroundColor)).toBe("rgba(0, 0, 0, 0)");
  // The count is muted and not bold.
  const countStyle = await page.locator(".book-tier .tier-active").first().evaluate((el) => {
    const css = getComputedStyle(el);
    return { weight: css.fontWeight, color: css.color, ink: getComputedStyle(document.body).color };
  });
  expect(Number(countStyle.weight)).toBe(400);
  expect(countStyle.color).not.toBe(countStyle.ink);
  // The row is a link with a chevron and a hairline under it.
  const rowItem = page.locator(".book-directory-item").first();
  expect(await rowItem.evaluate((el) => getComputedStyle(el).borderBottomWidth)).toBe("1px");
  await expect(rowItem.locator("svg.book-chevron")).toHaveCount(1);
});

test("Inside each tier the oldest book comes first and undated books come last", async ({ page }) => {
  await page.goto("#/books");
  await openBookGroups(page);
  for (const tier of tiers) {
    const group = page.locator(`.book-tier[data-tier="${tier.label}"]`);
    if (!(await group.count())) continue;
    const ids: string[] = await group
      .locator(".book-directory-item")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id") as string));
    const dated = ids
      .map((id) => sources.find((source) => source.id === id)?.publication_date as string | undefined)
      .map((date) => (/^\d{4}/.test(date ?? "") ? (date as string) : null));
    const known = dated.filter((date): date is string => date !== null);
    expect([...known].sort(), tier.label).toEqual(known);
    // An undated book never comes before a dated one.
    const firstUndated = dated.indexOf(null);
    if (firstUndated >= 0) expect(dated.slice(firstUndated).every((date) => date === null), tier.label).toBe(true);
  }
});

test("A book row shows a month-year and an entry count, and the Inactive tag only on an inactive book", async ({ page }) => {
  const source = inTier("Rulebooks").find((candidate) => /^\d{4}-\d{2}/.test(candidate.publication_date ?? ""))!;
  await page.goto("#/books");
  const row = page.locator(`.book-directory-item[data-book-id="${source.id}"]`);
  const entries = entriesOf(source);
  await expect(row.locator(".cat-blurb")).toHaveText(`${monthYear(source.publication_date)} · ${grouped(entries)} ${entries === 1 ? "entry" : "entries"}`);
  await expect(row.locator(".tag")).toHaveCount(0);
  await expect(page.getByText("Disabled")).toHaveCount(0);

  // Deactivate it on its own page: the row on Books ends with a muted Inactive label.
  await row.locator("a.book-directory-row").click();
  await page.locator(".book-active-row").getByRole("checkbox", { name: "Active", exact: true }).uncheck();
  await page.goBack();
  await expect(page.locator(".book-tier").first()).toBeVisible();
  const inactive = row.locator(".tag");
  await expect(inactive).toHaveText("Inactive");
  await expect(row.locator(".cat-blurb")).toContainText(`${grouped(entries)}`);
  // A label has no border and no fill (the row around it is the link).
  expect(await inactive.evaluate((el) => {
    const css = getComputedStyle(el);
    return [css.borderTopWidth, css.backgroundColor];
  })).toEqual(["0px", "rgba(0, 0, 0, 0)"]);
  await expect(page.locator(".books-count")).toHaveText(
    `${grouped(sources.length - 1)} of ${grouped(sources.length)} books active · Custom`,
  );
  await expect(page.locator('.book-tier[data-tier="Rulebooks"] .tier-active')).toHaveText(
    `${grouped(inTier("Rulebooks").length - 1)} of ${grouped(inTier("Rulebooks").length)} active`,
  );
  // Every other row is untouched.
  await expect(page.locator(".book-directory-item .tag")).toHaveCount(1);
});

test("The groups the reader opened stay open on Back, before the scroll position returns", async ({ page }) => {
  await page.goto("#/books");
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  await adventures.getByRole("button").click();
  const items = adventures.locator(".book-directory-item");
  const count = await items.count();
  expect(count).toBeGreaterThan(8);
  const target = items.nth(count - 1);
  const id = (await target.getAttribute("data-book-id"))!;
  await target.scrollIntoViewIfNeeded();
  await page.evaluate(() => window.scrollBy(0, 40));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);
  await target.locator("a.book-directory-row").click();
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(id)}`));
  await page.goBack();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.locator(".book-tier").first()).toBeVisible();
  // The group is open again and the position is where it was.
  await expect(adventures.getByRole("button")).toHaveAttribute("aria-expanded", "true");
  await expect(adventures.locator(".book-directory-item").last()).toBeVisible();
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before), { timeout: 5000 })
    .toBeLessThanOrEqual(2);

  // A group the reader closed stays closed: Rulebooks was open by default.
  await page.locator('.book-tier[data-tier="Rulebooks"]').getByRole("button").click();
  await expect(page.locator('.book-tier[data-tier="Rulebooks"]').getByRole("button")).toHaveAttribute("aria-expanded", "false");
  await page.goto("#/settings");
  await page.goto("#/books");
  await expect(page.locator('.book-tier[data-tier="Rulebooks"]').getByRole("button")).toHaveAttribute("aria-expanded", "false");
  await expect(adventures.getByRole("button")).toHaveAttribute("aria-expanded", "true");
});

test("A book row is one link to its book page, with no checkbox, no inline tree and no Read link", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  await page.goto("#/books");
  await openBookGroups(page);
  const item = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const link = item.locator("a.book-directory-row");
  await expect(link).toHaveAttribute("href", `#/browse/book/${book.source}`);
  await expect(link).toContainText(book.title);

  // Nothing on the list expands, and nothing is called "Read ...".
  await expect(page.locator(".book-directory-item details")).toHaveCount(0);
  await expect(page.locator(".book-tree, [data-testid=book-tree], .book-tree-root")).toHaveCount(0);
  await expect(page.locator(".book-directory-item").getByRole("link", { name: /^Read / })).toHaveCount(0);

  // A row is a link and nothing else: no checkbox, no switch, a 56 px target for two lines.
  await expect(item.locator("input")).toHaveCount(0);
  await expect(page.locator(".book-directory-item input")).toHaveCount(0);
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.height).toBeGreaterThanOrEqual(56);

  await link.click();
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(book.source)}`));
  await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) })).toBeVisible();
});

test("A book page opens with its title, a meta line and the Active row, with no breadcrumb", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  const source = sources.find((row) => row.id === book.source)!;
  await page.goto(`#/browse/book/${book.source}`);
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(main.locator("nav.crumbs, .crumbs")).toHaveCount(0);
  await expect(main.getByRole("navigation", { name: "Breadcrumb" })).toHaveCount(0);

  // Title, then meta, then the Active row, then Contents, in that order.
  const entries = entriesOf(source);
  const meta = page.getByTestId("source-count");
  await expect(meta).toHaveText(`${monthYear(source.publication_date)} · ${grouped(entries)} ${entries === 1 ? "entry" : "entries"}`);
  const y = async (locator: ReturnType<typeof page.locator>) => (await locator.boundingBox())!.y;
  const order = [
    await y(main.getByRole("heading", { level: 1 })),
    await y(meta),
    await y(page.locator(".book-active-row")),
    await y(main.getByRole("heading", { level: 2, name: "Contents", exact: true })),
  ];
  expect([...order].sort((a, b) => a - b)).toEqual(order);

  // The old explanations are gone.
  await expect(page.locator(".order-note, .toc-flag, .toc-note")).toHaveCount(0);
  await expect(page.getByText(/Book contents still need review|How this list was made|Position unverified|check order|New entries \(page order\)/)).toHaveCount(0);
  // One quiet line under Contents says the order is a draft.
  const review = page.locator(".review-line");
  await expect(review).toHaveText("Chapter order not yet checked against the printed book");
  expect(await review.evaluate((el) => getComputedStyle(el).fontWeight)).toBe("400");
});

test("A book page links each kind it holds to the kind list filtered to the book", async ({ page }) => {
  const core = sources.find((row) => row.id === "core-rulebook")!;
  // Home's category order, which is what the section promises.
  const kindOrder: [string, string][] = [
    ["spell", "Spells"],
    ["feat", "Feats"],
    ["class", "Classes"],
    ["prestige_class", "Prestige classes"],
    ["race", "Races"],
    ["trait", "Traits"],
    ["item", "Items"],
    ["creature", "Creatures"],
    ["archetype", "Archetypes"],
    ["racial_trait", "Racial traits"],
    ["skill", "Skills"],
    ["affliction", "Afflictions"],
    ["trap", "Traps"],
  ];
  const expected = kindOrder
    .map(([kind, label]) => {
      const n = corpus.catalog.entries.filter(
        (stub: any) => stub.kind === kind && (owners.get(stub.id) ?? stub.source) === core.id,
      ).length;
      return { kind, label, n };
    })
    .filter((row) => row.n > 0);
  expect(expected.length).toBeGreaterThan(1);

  await page.goto(`#/browse/book/${core.id}`);
  const section = page.locator(".book-in-this-book");
  await expect(section.getByRole("heading", { level: 2, name: "In this book", exact: true })).toBeVisible();
  const links = section.locator(".jump");
  await expect(links).toHaveCount(expected.length);
  for (const [index, row] of expected.entries()) {
    const link = links.nth(index);
    await expect(link.locator(".book-kind-label")).toHaveText(row.label);
    await expect(link.locator(".book-kind-count")).toHaveText(grouped(row.n));
    await expect(link).toHaveAttribute("href", `#/browse/kind/${row.kind}?source=${core.id}`);
  }
  // Rules has no list page, so it has no link.
  await expect(section.locator(".jump", { hasText: "Rules" })).toHaveCount(0);

  // The first link opens that kind filtered to the book, with the book pill on top.
  await links.first().click();
  const first = expected[0];
  await expect(page).toHaveURL(new RegExp(`#/browse/kind/${first.kind}\\?source=${escape(core.id)}$`));
  await expect(page.getByRole("heading", { level: 1, name: first.label })).toBeVisible();
  await expect(page.getByRole("button", { name: `Remove filter: ${core.title}`, exact: true })).toBeVisible();
});

test("The copyright is printed once, in the colophon at the bottom, each distinct notice once", async ({ page }) => {
  // Gods and Magic carries the same notice twice, once as `Copyright 2008` with the title in
  // front and once as `© 2008`. (The Advanced Player's Guide was the example until the corpus
  // replaced its repeated notices with the printed one.)
  const source = sources.find((row) => row.id === "gods-and-magic")!;
  expect(source.copyright.split("Sean K Reynolds").length - 1, "the corpus still repeats it").toBeGreaterThan(1);
  await page.goto(`#/browse/book/${source.id}`);
  await expect(page.getByRole("heading", { level: 1, name: source.title, exact: true })).toBeVisible();
  await expect(page.locator(".copyright")).toHaveCount(1);
  const text = await page.locator(".copyright").innerText();
  expect(text.split("2008").length - 1).toBe(1);
  expect(text).toContain("Sean K Reynolds");
  // Nothing of it sits under the title: it is the last block, in a footer, before the page ends.
  const colophon = page.locator("footer.colophon");
  await expect(colophon).toHaveCount(1);
  await expect(colophon.locator(".copyright")).toHaveCount(1);
  await expect(colophon.getByRole("link", { name: "Legal and attribution" })).toHaveAttribute("href", "#/legal");
  const colophonY = (await colophon.boundingBox())!.y;
  expect(colophonY).toBeGreaterThan((await page.locator(".book-active-row").boundingBox())!.y);
  expect(colophonY).toBeGreaterThan((await page.locator("nav.toc, .review-line").first().boundingBox())!.y);
  await expect(page.locator(".book > .copyright")).toHaveCount(0);

  // A notice with another year is a different notice: both stay. (The Core Rulebook was the
  // example until the corpus replaced its notices with the single printed one.)
  const other = sources.find((row) => row.id === "pathfinder-29-mother-of-flies")!;
  await page.goto(`#/browse/book/${other.id}`);
  await expect(page.getByRole("heading", { level: 1, name: other.title, exact: true })).toBeVisible();
  const lines = await page.locator("footer.colophon .copyright").allInnerTexts();
  expect(lines.length).toBeGreaterThan(1);
  expect(new Set(lines).size).toBe(lines.length);
  expect(lines.join(" ")).toContain("2009");
  expect(lines.join(" ")).toContain("2010");
});

test("Contents leaves out spells, feats, items and class options now that In this book links them", async ({ page }) => {
  // The owner wants these kinds reached through `In this book` (#158). The links exist, so the
  // exclusion is on (`LEAVE_OUT_LINKED_KINDS`) and the book page must not list them twice.
  const kindOf = new Map<string, string>(corpus.catalog.entries.map((stub: any) => [stub.id, stub.kind]));
  const hidden = CONTENTS_HIDDEN_KINDS;
  const book = bookTrees().find((candidate) => candidate.source === "core-rulebook") ?? bookTrees()[0];
  await page.goto(`#/browse/book/${book.source}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) });
  await expect(tree).toBeVisible();

  // Open every chapter and section, then no row may lead to a hidden kind.
  for (let round = 0; round < 6; round++) {
    const closed = tree.locator("details.toc-details:not([open]) > summary");
    const n = await closed.count();
    if (!n) break;
    for (let i = n - 1; i >= 0; i--) await closed.nth(i).click().catch(() => undefined);
  }
  const hrefs: string[] = await tree.locator('a[href^="#/entry/"]').evaluateAll((links) => links.map((a) => a.getAttribute("href") as string));
  expect(hrefs.length).toBeGreaterThan(0);
  const kinds = new Set(hrefs.map((href) => kindOf.get(decodeURIComponent(href.slice("#/entry/".length).split("?")[0]))));
  for (const kind of hidden) expect(kinds.has(kind), `${kind} appears in Contents`).toBe(false);
  expect(kinds.has("spell"), "spells are reached by In this book, not Contents").toBe(false);
  expect(kinds.has("feat"), "feats are reached by In this book, not Contents").toBe(false);

  // A top-level chapter that held only left-out kinds is not listed.
  const entriesOf = (node: any): string[] => [...(node.entry ? [node.entry] : []), ...(node.children ?? []).flatMap(entriesOf)];
  const onlyHidden = book.chapters.filter((chapter: any) => {
    const ids = entriesOf(chapter);
    return ids.length > 0 && ids.every((id) => hidden.has(kindOf.get(id) as string));
  });
  const titles = (await tree.locator(".toc-top > li > .toc-details > summary .toc-title, .toc-top > li.toc-leaf > a").allInnerTexts()).map((t) => t.trim());
  for (const chapter of onlyHidden) expect(titles, chapter.title).not.toContain(chapter.title);
  expect(titles.length).toBe(book.chapters.length - onlyHidden.length);
});

test("The catch-all chapter is renamed and is not shown when nothing is left in it", async ({ page }) => {
  const kindOf = new Map<string, string>(corpus.catalog.entries.map((stub: any) => [stub.id, stub.kind]));
  const hidden = CONTENTS_HIDDEN_KINDS;
  const withCatchAll = bookTrees().filter((book) => book.chapters.some((chapter: any) => chapter.title === "New entries (page order)"));
  expect(withCatchAll.length).toBeGreaterThan(0);
  const leaves = (node: any): string[] => [...(node.entry ? [node.entry] : []), ...(node.children ?? []).flatMap(leaves)];
  for (const book of withCatchAll) {
    const catchAll = book.chapters.find((chapter: any) => chapter.title === "New entries (page order)");
    const left = leaves(catchAll).filter((id) => !hidden.has(kindOf.get(id) as string));
    await page.goto(`#/browse/book/${book.source}`);
    await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
    await expect(page.getByText("New entries (page order)")).toHaveCount(0);
    const rows = page.locator(".toc-top > li");
    await expect(rows.first()).toBeVisible();
    const more = rows.filter({ hasText: "More from this book" });
    const lone = book.chapters.filter((chapter: any) => leaves(chapter).some((id: string) => !hidden.has(kindOf.get(id) as string))).length === 1;
    if (left.length === 0) await expect(more).toHaveCount(0);
    else if (!lone) await expect(more).toHaveCount(1);
  }
});

test("A book page highlights the Books tab, also from an old publication link", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  await page.goto(`#/browse/book/${book.source}`);
  await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveCount(1);
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Books");

  // The old publication address is the book page: same tab, and the address becomes the book's.
  await page.goto("#/");
  await page.goto(`#/browse/source/${book.source}`);
  await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Books");
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(book.source)}$`));
  await expect(page.locator('a.nav-link[href="#/"]')).not.toHaveAttribute("aria-current", "page");
  await page.goBack();
  await expect(page).toHaveURL(/#\/$/);
});

test("Back from a book page returns to Books at the same scroll position", async ({ page }) => {
  await page.goto("#/books");
  await expect(page.locator(".book-tier").first()).toBeVisible();
  // Use the group that is open by default.
  const items = page.locator('.book-tier[data-tier="Rulebooks"] .book-directory-item');
  const count = await items.count();
  expect(count).toBeGreaterThan(3);
  const target = items.nth(count - 1);
  const id = (await target.getAttribute("data-book-id"))!;
  await target.scrollIntoViewIfNeeded();
  await page.evaluate(() => window.scrollBy(0, 40));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  await target.locator("a.book-directory-row").click();
  await expect(page).toHaveURL(new RegExp(`#/browse/book/${escape(id)}`));
  await page.goBack();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(page.locator(".book-tier").first()).toBeVisible();
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before), { timeout: 5000 })
    .toBeLessThanOrEqual(2);
});

test("A book page shows a hierarchical contents tree and builds closed branches lazily", async ({ page }) => {
  const kindOf = new Map<string, string>(corpus.catalog.entries.map((stub: any) => [stub.id, stub.kind]));
  const hidden = CONTENTS_HIDDEN_KINDS;
  const shown = (node: any): boolean => {
    const entries = (n: any): string[] => [...(n.entry ? [n.entry] : []), ...(n.children ?? []).flatMap(entries)];
    const all = entries(node);
    return all.length === 0 || all.some((id) => !hidden.has(kindOf.get(id) as string));
  };
  const book =
    bookTrees().find((candidate) =>
      candidate.chapters.some((chapter) =>
        (chapter.children ?? []).some((child) => (child.children ?? []).length > 0 && shown(child)) && shown(chapter),
      ),
    ) ?? bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0));
  if (!book) throw new Error("The compiled corpus has no book with nested contents.");

  await page.goto(`#/browse/book/${book.source}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) });
  await expect(tree).toBeVisible();

  // A chapter with at least one listed child: the first such chapter, counted among listed rows.
  const listedTop = book.chapters.filter((chapter: any) => shown(chapter));
  const chapter: any = listedTop.find((candidate: any) => (candidate.children ?? []).some(shown) && (candidate.children ?? []).length > 0);
  const chapterIndex = listedTop.indexOf(chapter);
  const chapterRow = tree.locator(".toc-top > li").nth(chapterIndex);
  const chapterSummary = chapterRow.locator(":scope > .toc-details > summary");
  await expect(chapterSummary).toContainText(chapter.title);

  // A closed chapter has no child rows in the document until it opens.
  await expect(chapterRow.locator(":scope > .toc-details > ul")).toHaveCount(0);
  await chapterSummary.click();
  await expect(chapterRow.locator(":scope > .toc-details > ul")).toBeVisible();

  // The disclosure is keyboard operable and reports its state through the native element.
  const details = chapterSummary.locator("xpath=..");
  const openBefore = await details.evaluate((element) => (element as HTMLDetailsElement).open);
  await chapterSummary.focus();
  await chapterSummary.press("Enter");
  await expect
    .poll(() => details.evaluate((element) => (element as HTMLDetailsElement).open))
    .toBe(!openBefore);
  await chapterSummary.press("Enter");
  await expect
    .poll(() => details.evaluate((element) => (element as HTMLDetailsElement).open))
    .toBe(openBefore);

  // A section within the chapter opens the same way and reveals its own children.
  const branch = chapterRow.locator(":scope > .toc-details > ul > li.toc-branch").first();
  if (await branch.count()) {
    const branchChildren = branch.locator(":scope > .toc-details > ul");
    await expect(branchChildren).toHaveCount(0);
    await branch.locator(":scope > .toc-details > summary").click();
    await expect(branchChildren).toBeVisible();
  }
  // No row on the page carries a flag or a note about its position.
  await expect(page.locator(".toc-flag, .toc-note")).toHaveCount(0);
});

test("A chapter toggle that Back overtakes does not rewrite the Books entry", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0))!;
  await page.goto("#/books");
  await openBookGroups(page);
  // The race is a matter of task order, so repeat it.
  for (let round = 0; round < 6; round++) {
    await page.locator(`.book-directory-item[data-book-id="${book.source}"] a.book-directory-row`).click();
    const summary = page.locator(".toc-top > li > .toc-details > summary").first();
    await expect(summary).toBeVisible();
    // The browser fires `toggle` in a task after the click. Ask for Back in the same task, so the
    // traversal can land before the event does: the late handler must not write the book's URL
    // over the Books entry.
    await summary.evaluate((element) => {
      element.click();
      history.back();
    });
    await expect(page.locator(".book-tier").first()).toBeVisible();
    // Let any deferred toggle event run, then check that Books is still what the entry shows.
    await page.evaluate(() => new Promise<void>((resolve) => setTimeout(resolve, 300)));
    await expect(page).toHaveURL(/#\/books$/);
    await expect(page.locator(".book-tier").first()).toBeVisible();
  }
});
