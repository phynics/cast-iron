import { test, expect, loadCorpus } from "./fixtures";
import {
  bookOwners,
  bookReadingOrder,
  bookTrees,
  CONTENTS_HIDDEN_KINDS,
  installLibrary,
  listedChapterTitles,
  listedChapters,
  openBookGroups,
  searchFor,
} from "./helpers";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;
/** The corpus marks two books third party (#189); they are opt-in, so a settled profile starts with the rest. */
const paizo = sources.filter((source) => !source.third_party);
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const grouped = (n: number) => n.toLocaleString("en-US");

/** The tier table, restated independently of the Rust code: this is what the page promises. */
const tiers: { label: string; categories: string[]; open: boolean }[] = [
  { label: "Rulebooks", categories: ["RPG"], open: true },
  { label: "Player Companions", categories: ["Player Companion"], open: false },
  { label: "Campaign Setting", categories: ["Campaign Setting"], open: false },
  { label: "Adventures", categories: ["Adventure Path", "Module", "Society"], open: false },
  { label: "Other", categories: [], open: false },
  { label: "Third party", categories: [], open: false },
];
const tierOf = (source: any) => {
  if (source.third_party) return "Third party";
  const fromCategory = tiers.find((tier) => tier.categories.includes(source.category))?.label ?? "Other";
  // A recognized series kind with no reviewed category still belongs to Adventures (#159).
  return fromCategory === "Other" && source.series_kind ? "Adventures" : fromCategory;
};
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

test("Books opens with a title, one count line and a link to Manage books, with no lede or sort", async ({ page }) => {
  await page.goto("#/books");
  await expect(page.getByRole("heading", { level: 1, name: "Books", exact: true })).toBeVisible();
  const count = page.locator(".books-count");
  // Every Paizo book active is the Paizo Everything set; the marked third-party books stay off.
  await expect(count).toHaveText(`${grouped(paizo.length)} of ${grouped(sources.length)} books active · Paizo Everything`);
  await expect(count).toHaveAttribute("aria-live", "polite");
  await expect(page.getByRole("link", { name: "Manage books", exact: true })).toHaveAttribute("href", "#/books/manage");

  // Nothing between the count line and the groups explains itself.
  await expect(page.locator(".wide > p")).toHaveCount(1);
  await expect(page.locator(".hint, .count-scope, .measure")).toHaveCount(0);
  await expect(page.getByText(/Open a book to read|Manage books appear in search|Disabling a book/)).toHaveCount(0);
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
    // Adventures holds series group headings rather than one flat list (#159).
    const list = tier.label === "Adventures"
      ? group.locator(".adventure-series-groups")
      : group.locator("ul.book-directory");
    expect(await toggle.getAttribute("aria-controls")).toBe(await list.getAttribute("id"));
    // Every Paizo book starts active; the third-party group starts with none.
    const active = tier.label === "Third party" ? 0 : members.length;
    const tierCount = active === members.length
      ? `${grouped(members.length)} ${members.length === 1 ? "book" : "books"}`
      : `${grouped(active)} of ${grouped(members.length)} active`;
    await expect(group.locator(".tier-active")).toHaveText(tierCount);
    await expect(group.locator(".tier-chevron")).toHaveCount(1);
    expect((await toggle.boundingBox())!.height).toBeGreaterThanOrEqual(44);
    // A closed group builds no rows: its list is empty until the reader opens it. The open
    // group paints its rows at once.
    if (tier.open) {
      await expect(list.locator(".book-directory-item").first()).toBeVisible();
      const ids = await list.locator(".book-directory-item").evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
      expect([...ids].sort()).toEqual(members.map((source) => source.id).sort());
    } else {
      await expect(list.locator(".book-directory-item")).toHaveCount(0);
    }
  }
  // Only the group that starts open holds rows after the mount.
  await expect(page.locator(".book-directory-item")).toHaveCount(inTier("Rulebooks").length);

  // The group heading is a heading with one button; nothing else sits in it, and no group acts.
  await expect(page.locator(".book-group-head a, .book-group-head input, .book-group-head button")).toHaveCount(present.length);
  await expect(page.locator("details.tier-details, .tier-summary, .tier-actions, .books-preset")).toHaveCount(0);

  // Opening and closing flips aria-expanded and the rows.
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  const toggle = adventures.locator(".tier-toggle");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  await toggle.press("Enter");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(adventures.locator(".book-directory-item").first()).toBeHidden();
});

test("A closed group holds no rows in the document, and opening it renders them", async ({ page }) => {
  await page.goto("#/books");
  await expect(page.locator(".book-tier").first()).toBeVisible();
  for (const tier of tiers.filter((tier) => !tier.open)) {
    const group = page.locator(`.book-tier[data-tier="${tier.label}"]`);
    if (!(await group.count())) continue;
    const members = inTier(tier.label);
    const list = group.locator("ul.book-directory");
    const toggle = group.locator(".tier-toggle");
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    // Nothing to build and nothing to hide: the rows do not exist yet.
    await expect(list.locator(".book-directory-item")).toHaveCount(0);
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    if (tier.label === "Adventures") {
      await expect(group.locator(".adventure-group").first()).toBeVisible();
    }
    await expect(list.locator(".book-directory-item")).toHaveCount(members.length);
    const ids = await list.locator(".book-directory-item").evaluateAll((items) => items.map((item) => item.getAttribute("data-book-id")));
    expect([...ids].sort()).toEqual(members.map((source) => source.id).sort());
    // Closing hides them; the rows stay built, so a second open costs nothing.
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await expect(list.locator(".book-directory-item").first()).toBeHidden();
    await expect(list.locator(".book-directory-item")).toHaveCount(members.length);
  }
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
    // Adventures no longer has one dated list: each series group has its own order (#159).
    if (tier.label === "Adventures") continue;
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

  // Deactivate it on Manage books: the row on Books ends with a muted Inactive label.
  await page.goto("#/books/manage");
  await openBookGroups(page);
  await page.locator(`.book-directory-item[data-book-id="${source.id}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).uncheck();
  await page.goto("#/books");
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
    `${grouped(paizo.length - 1)} of ${grouped(sources.length)} books active · Custom`,
  );
  await expect(page.locator('.book-tier[data-tier="Rulebooks"] .tier-active')).toHaveText(
    `${grouped(inTier("Rulebooks").length - 1)} of ${grouped(inTier("Rulebooks").length)} active`,
  );
  // Every other row is untouched: only the row this test turned off joins the third-party rows
  // that are inactive by default.
  const thirdPartyCount = sources.filter((source) => source.third_party).length;
  await expect(page.locator(".book-directory-item .tag")).toHaveCount(1 + thirdPartyCount);
});

test("The groups the reader opened stay open on Back, before the scroll position returns", async ({ page }) => {
  await page.goto("#/books");
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  await adventures.locator(".tier-toggle").click();
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
  await expect(adventures.locator(".tier-toggle")).toHaveAttribute("aria-expanded", "true");
  await expect(adventures.locator(".book-directory-item").last()).toBeVisible();
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before), { timeout: 5000 })
    .toBeLessThanOrEqual(2);

  // A group the reader closed stays closed: Rulebooks was open by default.
  await page.locator('.book-tier[data-tier="Rulebooks"] .tier-toggle').click();
  await expect(page.locator('.book-tier[data-tier="Rulebooks"] .tier-toggle')).toHaveAttribute("aria-expanded", "false");
  await page.goto("#/settings");
  await page.goto("#/books");
  await expect(page.locator('.book-tier[data-tier="Rulebooks"] .tier-toggle')).toHaveAttribute("aria-expanded", "false");
  await expect(adventures.locator(".tier-toggle")).toHaveAttribute("aria-expanded", "true");
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

test("A book page opens with its title, a meta line and the contents, with no breadcrumb or toggle", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  const source = sources.find((row) => row.id === book.source)!;
  await page.goto(`#/browse/book/${book.source}`);
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(main.locator("nav.crumbs, .crumbs")).toHaveCount(0);
  await expect(main.getByRole("navigation", { name: "Breadcrumb" })).toHaveCount(0);
  // An active book shows no off line and no control that writes the book selection.
  await expect(page.locator(".book-off-line, .book-active-row")).toHaveCount(0);
  await expect(main.getByRole("checkbox", { name: "Active", exact: true })).toHaveCount(0);

  // Title, then meta, then In this book, then Contents, in that order.
  const entries = entriesOf(source);
  const meta = page.getByTestId("source-count");
  await expect(meta).toHaveText(`${monthYear(source.publication_date)} · ${grouped(entries)} ${entries === 1 ? "entry" : "entries"}`);
  const y = async (locator: ReturnType<typeof page.locator>) => (await locator.boundingBox())!.y;
  const order = [
    await y(main.getByRole("heading", { level: 1 })),
    await y(meta),
    await y(main.getByRole("heading", { level: 2, name: "In this book", exact: true })),
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

test("A Contents chapter opens its own page with a Books › Book › Chapter breadcrumb, and Back restores the position", async ({ page }) => {
  const core = sources.find((row) => row.id === "core-rulebook")!;
  const book = bookTrees().find((row) => row.source === core.id)!;
  await page.goto(`#/browse/book/${core.id}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(core.title)}`) });
  await expect(tree).toBeVisible();
  const chapters = listedChapters(book);
  const last = chapters[chapters.length - 1];
  const row = tree.locator(".toc-top > li").last();
  const link = row.locator(":scope > .toc-row a, :scope > a").first();
  await expect(link).toHaveAccessibleName(last.title);
  await expect(link).toHaveAttribute("href", new RegExp(`^#/browse/book/${escape(core.id)}/[^/]+$`));

  await row.scrollIntoViewIfNeeded();
  await page.evaluate(() => window.scrollBy(0, 40));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
  const chapterUrl = new URL(page.url()).hash;
  const href = (await link.getAttribute("href"))!;
  // Programmatic activation keeps the measured offset: Playwright would first scroll the link
  // into view again, and that extra scroll is what Back would then restore.
  await link.evaluate((element) => (element as HTMLAnchorElement).click());
  await expect(page).toHaveURL(new RegExp(`${escape(href)}$`));
  const heading = page.getByTestId("chapter-title");
  await expect(heading).toHaveText(last.title);
  const crumbs = page.getByRole("navigation", { name: "Breadcrumb" });
  await expect(crumbs.getByRole("link", { name: "Books", exact: true })).toHaveAttribute("href", "#/books");
  await expect(crumbs.getByRole("link", { name: core.title, exact: true })).toHaveAttribute(
    "href",
    `#/browse/book/${core.id}`,
  );
  await expect(crumbs.locator('[aria-current="page"]')).toHaveText(last.title);

  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`${escape(chapterUrl)}$`));
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(core.title)}`) })).toBeVisible();
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => Math.round(window.scrollY))) - before), { timeout: 5000 })
    .toBeLessThanOrEqual(2);
});

test("A chapter page opens an entry, and Next continues the book before Back returns to the chapter", async ({ page }) => {
  const core = sources.find((row) => row.id === "core-rulebook")!;
  const book = bookTrees().find((row) => row.source === core.id)!;
  // A hub chapter now opens its entry page instead of the chapter page, so this #346 test
  // keeps the first chapter that still opens a chapter page.
  const first = listedChapters(book).find((row) => !row.node.lists_children && row.node.entry)!;
  expect(first, "the book has a chapter page with its own entry").toBeTruthy();
  const order = bookReadingOrder(book);
  const entryId = order.find((id, at) => {
    if (at >= order.length - 1) return false;
    const path = pathOfEntry(book, id);
    return path !== null && first.path.every((part, depth) => path[depth] === part);
  });
  expect(entryId, "the first chapter has an entry with a next entry").toBeTruthy();
  const entry = corpus.entries.get(entryId!)!;

  await page.goto(`#/browse/book/${core.id}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(core.title)}`) });
  const row = tree.locator(".toc-top > li").filter({ has: page.getByText(first.title, { exact: true }) }).first();
  await row.locator(":scope > .toc-row a, :scope > a").first().click();
  await expect(page.getByTestId("chapter-title")).toBeVisible();
  await expect(page.getByTestId("chapter-title")).toHaveText(first.title);
  const chapterUrl = new URL(page.url()).hash;

  const entryLink = page.locator(`main a[href^="#/entry/${entryId}"]`).first();
  await expect(entryLink).toBeVisible();
  await entryLink.click();
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
  await expect(page.getByTestId("book-bar")).toBeVisible();
  // Back from the entry returns to the chapter page it came from.
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`${escape(chapterUrl)}$`));
  await expect(page.getByTestId("chapter-title")).toBeVisible();

  // Open the same entry again and step once; two Backs walk the entry and the chapter back.
  await expect(entryLink).toBeVisible();
  await entryLink.click();
  const next = page.locator('.book-steps a[rel="next"]');
  await expect(next).toBeVisible();
  // The book row keeps the printed title; the entry page shows the catalog title, so read the
  // target id from the link instead of comparing the two labels.
  const nextHref = (await next.getAttribute("href"))!;
  const nextId = decodeURIComponent(nextHref.split("?")[0].slice("#/entry/".length));
  const nextEntry = corpus.entries.get(nextId)!;
  await next.click();
  await expect(page.getByRole("heading", { level: 1, name: nextEntry.title, exact: true })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
  await page.goBack();
  await expect(page).toHaveURL(new RegExp(`${escape(chapterUrl)}$`));
  await expect(page.getByTestId("chapter-title")).toBeVisible();
});

test("An entry opened from search outside the first active book shows no previous or next", async ({ page }) => {
  // The default selection keeps every book active, so the first book in book order is the
  // fallback context. An entry that only another book places has no reading footer. Stay in the
  // Rulebooks tier, which search opens by default, so the hit is visible.
  const first = bookTrees()[0];
  const otherBook = sources.find(
    (source) => source.category === "RPG" && source.id !== first.source && !source.third_party,
  )!;
  const placed = bookOwners();
  const other = [...corpus.entries.values()]
    .find((entry) => placed.get(entry.id) === otherBook.id && entry.title.trim().length > 3)!;
  expect(other, "the compiled corpus has an entry outside the first book").toBeTruthy();
  await searchFor(page, other.title);
  const result = page.locator(`.results a[href^="#/entry/${other.id}"]`).first();
  await expect(result).toBeVisible();
  await result.click();
  await expect(page.getByRole("heading", { level: 1, name: other.title, exact: true })).toBeVisible();
  await expect(page.locator(".book-bar")).toHaveCount(0);
  await expect(page.locator(".book-steps a")).toHaveCount(0);
});

function pathOfEntry(book: ReturnType<typeof bookTrees>[number], id: string): number[] | null {
  const walk = (nodes: any[], base: number[]): number[] | null => {
    for (const [index, node] of nodes.entries()) {
      const path = [...base, index];
      if (node.entry === id) return path;
      const found = walk(node.children ?? [], path);
      if (found) return found;
    }
    return null;
  };
  return walk(book.chapters, []);
}

test("An inactive book shows one off line and no toggle; turning it on removes both", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  const source = sources.find((row) => row.id === book.source)!;
  await page.goto("#/books/manage");
  await openBookGroups(page);
  const toggle = page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true });
  await toggle.uncheck();
  await expect(toggle).not.toBeChecked();

  // The book page shows the line and no way to change the choice.
  await page.goto(`#/browse/book/${book.source}`);
  const off = page.getByTestId("book-off-line");
  await expect(off).toHaveText("This book is turned off. Its entries don't appear in search or lists.");
  await expect(page.locator(".book-active-row")).toHaveCount(0);
  await expect(page.getByRole("checkbox", { name: "Active", exact: true })).toHaveCount(0);

  // Turning it on happens on Manage books; the line goes away on the book page.
  await page.goto("#/books/manage");
  await openBookGroups(page);
  await page.locator(`.book-directory-item[data-book-id="${book.source}"]`)
    .getByRole("checkbox", { name: `Active ${source.title}`, exact: true }).check();
  await page.goto(`#/browse/book/${book.source}`);
  await expect(page.getByRole("heading", { level: 1, name: book.title, exact: true })).toBeVisible();
  await expect(page.getByTestId("book-off-line")).toHaveCount(0);
  await expect(page.locator(".book-active-row")).toHaveCount(0);
});

test("A book page links each kind it holds to the kind list filtered to the book", async ({ page }) => {
  const core = sources.find((row) => row.id === "core-rulebook")!;
  // Home's category order, which is what the section promises.
  const kindOrder: [string, string][] = [
    ["spell", "Spells"],
    ["feat", "Feats"],
    ["class", "Classes"],
    ["prestige_class", "Prestige classes"],
    ["class_option", "Class options"],
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
  // The Council of Thieves Player's Guide carries the same notice twice, once as `© 2009` and
  // once as `Copyright 2009` with the title in front. The book set has no printed copy of it,
  // so the corpus keeps the import's joined notice. (Gods and Magic and the Advanced Player's
  // Guide were the examples until the corpus replaced their notices with the printed one.)
  const source = sources.find((row) => row.id === "council-of-thieves-players-guide")!;
  expect(source.copyright.split("Hank Woon").length - 1, "the corpus still repeats it").toBeGreaterThan(1);
  await page.goto(`#/browse/book/${source.id}`);
  await expect(page.getByRole("heading", { level: 1, name: source.title, exact: true })).toBeVisible();
  await expect(page.locator(".copyright")).toHaveCount(1);
  const text = await page.locator(".copyright").innerText();
  expect(text.split("2009").length - 1).toBe(1);
  expect(text).toContain("Hank Woon");
  // Nothing of it sits under the title: it is the last block, in a footer, before the page ends.
  const colophon = page.locator("footer.colophon");
  await expect(colophon).toHaveCount(1);
  await expect(colophon.locator(".copyright")).toHaveCount(1);
  await expect(colophon.getByRole("link", { name: "Legal and attribution" })).toHaveAttribute("href", "#/legal");
  const colophonY = (await colophon.boundingBox())!.y;
  expect(colophonY).toBeGreaterThan((await page.locator(".book-in-this-book").boundingBox())!.y);
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

test("Contents is one expanded list with no disclosure buttons and no kind that has a list page", async ({ page }) => {
  const kindOf = new Map<string, string>(corpus.catalog.entries.map((stub: any) => [stub.id, stub.kind]));
  const leaves = (node: any): string[] => [...(node.entry ? [node.entry] : []), ...(node.children ?? []).flatMap(leaves)];
  const book = bookTrees().find((candidate) =>
    candidate.chapters.some((chapter) => leaves(chapter).some((id) => kindOf.get(id) === "rule")),
  );
  if (!book) throw new Error("The compiled corpus has no book with a rules chapter.");
  await page.goto(`#/browse/book/${book.source}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) });
  await expect(tree).toBeVisible();

  // Every branch is expanded except a hub whose own page already lists its children: still no
  // disclosure, and at least one ordinary branch keeps its rows.
  await expect(tree.locator("button[aria-expanded], details, summary")).toHaveCount(0);
  await expect(tree.locator(".toc-branch:not(.toc-hub) > .toc-list").first()).toBeVisible();

  // Only rules entries remain; every kind with a list page is reached through In this book.
  const hrefs: string[] = await tree.locator('a[href^="#/entry/"]').evaluateAll((links) => links.map((a) => a.getAttribute("href") as string));
  const kinds = new Set(hrefs.map((href) => kindOf.get(decodeURIComponent(href.slice("#/entry/".length).split("?")[0]))));
  expect(kinds.has("rule"), "rules stay in Contents").toBe(true);
  for (const kind of CONTENTS_HIDDEN_KINDS) expect(kinds.has(kind), `${kind} appears in Contents`).toBe(false);
  expect(kinds.has("feat"), "feats are reached by In this book, not Contents").toBe(false);
  expect(kinds.has("item"), "items are reached by In this book, not Contents").toBe(false);

  // The top level is every chapter that still has a row, in book order.
  const titles = (await tree.locator(".toc-top > li > .toc-row .toc-title").allInnerTexts())
    .map((title) => title.trim());
  expect(titles).toEqual(listedChapterTitles(book));
});

test("A hub whose own page lists its children is one Contents row with its count", async ({ page }) => {
  const bestiary = sources.find((row) => row.id === "bestiary")!;
  const book = bookTrees().find((row) => row.source === bestiary.id)!;
  const titled = (nodes: any[], title: string): any => {
    for (const node of nodes) {
      if (node.title === title) return node;
      const found = titled(node.children ?? [], title);
      if (found) return found;
    }
    return undefined;
  };
  const hub = titled(book.chapters, "Universal Monster Rules");
  expect(hub, "the Bestiary contents carry Universal Monster Rules").toBeTruthy();
  const count = (node: any): number =>
    (node.entry ? 1 : 0) + (node.children ?? []).reduce((sum: number, child: any) => sum + count(child), 0);

  await page.goto(`#/browse/book/${bestiary.id}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(bestiary.title)}`) });
  await expect(tree).toBeVisible();
  const row = tree.locator(".toc-hub").filter({ hasText: "Universal Monster Rules" });
  await expect(row).toHaveCount(1);
  await expect(row.locator(".toc-count")).toHaveText(grouped(count(hub)));
  await expect(row.locator(".toc-list")).toHaveCount(0);

  // The row links to the hub entry, which already lists the rules.
  const link = row.locator(":scope > .toc-row a");
  await expect(link).toHaveAttribute("href", new RegExp(`^#/entry/${escape(hub.entry)}(\\?|$)`));
  await link.click();
  await expect(page.getByRole("heading", { level: 1, name: "Universal Monster Rules", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: "Ability Damage and Drain", exact: true }).first()).toBeVisible();
});

test("Top-level chapter rows show a dotted leader, hidden in a narrow contents container", async ({ page }) => {
  const core = sources.find((row) => row.id === "core-rulebook")!;
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(`#/browse/book/${core.id}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(core.title)}`) });
  await expect(tree).toBeVisible();

  // An ordinary top-level row opens its chapter page; the leader and the count join the link.
  // A collapsed hub row is covered by its own test.
  const row = tree.locator(".toc-top > li:not(.toc-hub) > .toc-row").filter({ has: page.locator("a") }).first();
  const link = row.locator("a");
  const title = (await link.locator(".toc-title").innerText()).trim();
  const leader = row.locator(".toc-leader");
  await expect(leader).toHaveCount(1);
  await expect(leader).toBeVisible();
  expect(await leader.evaluate((el) => getComputedStyle(el).borderBottomStyle)).toBe("dotted");
  expect((await leader.boundingBox())!.width).toBeGreaterThan(8);
  await expect(row.locator(".toc-count")).toHaveCount(1);
  // The leader is decorative: the link keeps the title as its whole accessible name.
  await expect(link).toHaveAccessibleName(title);

  // Only the top level draws a leader; a nested section keeps a plain gap.
  const nested = tree.locator(".toc-list .toc-list .toc-leader").first();
  await expect(nested).toHaveCount(1);
  expect(await nested.evaluate((el) => getComputedStyle(el).display)).toBe("none");

  // Below a 340 px container the leader is hidden and the row keeps its 44 px target and link.
  await page.setViewportSize({ width: 320, height: 844 });
  await expect(leader).toBeHidden();
  expect(await leader.evaluate((el) => getComputedStyle(el).display)).toBe("none");
  expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  await expect(link).toHaveAttribute("href", new RegExp(`^#/browse/book/${escape(core.id)}/[^/]+$`));
  await expect(link).toHaveAccessibleName(title);
});

test("The Core Rulebook contents have no feat or item rows, and Feats opens the filtered list", async ({ page }) => {
  const core = sources.find((row) => row.id === "core-rulebook")!;
  await page.goto(`#/browse/book/${core.id}`);
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(core.title)}`) });
  await expect(tree).toBeVisible();
  await expect(tree.locator("button[aria-expanded], details, summary")).toHaveCount(0);

  const kindOf = new Map<string, string>(corpus.catalog.entries.map((stub: any) => [stub.id, stub.kind]));
  const hrefs: string[] = await tree.locator('a[href^="#/entry/"]').evaluateAll((links) => links.map((a) => a.getAttribute("href") as string));
  const kinds = new Set(hrefs.map((href) => kindOf.get(decodeURIComponent(href.slice("#/entry/".length).split("?")[0]))));
  expect(kinds.has("feat")).toBe(false);
  expect(kinds.has("item")).toBe(false);
  expect(kinds.has("spell")).toBe(false);

  const feats = page.locator(".book-in-this-book .jump", { hasText: "Feats" });
  await expect(feats).toHaveAttribute("href", `#/browse/kind/feat?source=${core.id}`);
  await feats.click();
  await expect(page).toHaveURL(new RegExp(`#/browse/kind/feat\\?source=${escape(core.id)}$`));
  await expect(page.getByRole("heading", { level: 1, name: "Feats" })).toBeVisible();
  await expect(page.getByRole("button", { name: `Remove filter: ${core.title}`, exact: true })).toBeVisible();
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
    const more = page.locator(".toc-top > li", { hasText: "More from this book" });
    const oneListedChapter = book.chapters.filter((chapter: any) => leaves(chapter).some((id: string) => !hidden.has(kindOf.get(id) as string))).length === 1;
    if (left.length === 0) await expect(more).toHaveCount(0);
    else if (!oneListedChapter) await expect(more).toHaveCount(1);
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

test("A book page shows a nested contents list with every branch expanded", async ({ page }) => {
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

  const listedTop = book.chapters.filter((chapter: any) => shown(chapter));
  const chapter: any = listedTop.find((candidate: any) => (candidate.children ?? []).some(shown) && (candidate.children ?? []).length > 0);
  const chapterIndex = listedTop.indexOf(chapter);
  const chapterRow = tree.locator(".toc-top > li").nth(chapterIndex);
  await expect(chapterRow.locator(":scope > .toc-row .toc-title")).toContainText(chapter.title);

  // The children are in the document without a click, inside one nested list.
  const children = chapterRow.locator(":scope > .toc-list > li");
  await expect(children.first()).toBeVisible();
  await expect(children).toHaveCount((chapter.children ?? []).filter(shown).length);
  await expect(tree.locator("details, summary, [aria-expanded]")).toHaveCount(0);

  // A deeper section is expanded too.
  const nested = chapterRow.locator(":scope > .toc-list > li.toc-branch").first();
  if (await nested.count()) {
    await expect(nested.locator(":scope > .toc-list > li").first()).toBeVisible();
  }
  // No row on the page carries a flag or a note about its position.
  await expect(page.locator(".toc-flag, .toc-note")).toHaveCount(0);

  // A `?c=` address still marks the row it names, without a disclosure.
  const firstListed = listedTop[0];
  await page.goto(`#/browse/book/${book.source}?c=1`);
  await expect(page.locator("#toc-1")).toHaveAttribute("aria-current", "location");
  await expect(page.locator("#toc-1")).toContainText(firstListed.title === "New entries (page order)" ? "More from this book" : firstListed.title);
});
