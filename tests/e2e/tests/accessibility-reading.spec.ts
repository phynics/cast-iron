import { test, expect, goOffline, loadCorpus } from "./fixtures";
import {
  installLibrary,
  waitControlled,
  bodyHeadings,
  bodyLinks,
  bookTrees,
  bookEntryOrder,
  firstEntry,
  letterId,
  openBookEntry,
  pathToEntry,
} from "./helpers";

/**
 * A placed rule article long enough for the keyboard walk: more than ten headings, at
 * least one typed cross-reference, and a following entry in its book. The canonical and
 * PSRD libraries differ in ids and chapter structure, so the test asks the compiled corpus
 * for the article instead of naming one.
 */
function readingTarget() {
  const entries = [...loadCorpus().entries.values()];
  const books = bookTrees();
  const place = new Map<string, { book: (typeof books)[number]; order: string[] }>();
  for (const book of books) {
    const order = bookEntryOrder(book);
    for (const id of order) if (!place.has(id)) place.set(id, { book, order });
  }
  const candidates = entries
    .filter((entry) =>
      entry.kind === "rule" &&
      bodyHeadings(entry).filter((heading) => heading.text.trim()).length > 10 &&
      bodyLinks(entry.body).length > 0 &&
      place.has(entry.id) &&
      place.get(entry.id)!.order.indexOf(entry.id) < place.get(entry.id)!.order.length - 1)
    .sort((a, b) => bodyHeadings(b).length - bodyHeadings(a).length);
  const target = candidates[0];
  if (!target) throw new Error("The compiled corpus has no placed rule article for the keyboard reading test.");
  return { target, book: place.get(target.id)!.book };
}

test("Keyboard reading traverses a complete subsystem offline and preserves book history", async ({ page, context, server }) => {
  await installLibrary(page); await waitControlled(page); await goOffline(context, server);
  const { target, book } = readingTarget();
  expect(pathToEntry(book.chapters, target.id)).not.toBeNull();

  // Follow the visible book tree, rather than inventing a category rules route.
  await openBookEntry(page, book, target.id, { keys: true });
  await expect(page.locator("h1")).toHaveText(target.title);
  const start = page.url();
  const contents = page.locator(".article-contents");
  await contents.locator("summary").press("Enter");
  // The disclosure opens on the key event; read its links only once they are exposed. WebKit can
  // run the next evaluation before the toggle, which returns no links at all.
  await expect(contents).toHaveJSProperty("open", true);
  await expect(contents.getByRole("link").first()).toBeVisible();
  // The reader derives a heading id even when the compiler stored no explicit anchor.
  const ids = await page.locator(".rich :is(h2,h3,h4,h5,h6)[id]").evaluateAll((els) => els.map((el) => el.id));
  const links = await contents.getByRole("link").evaluateAll((els) => els.map((a) => ({ href: a.getAttribute("href")!, text: a.textContent! })));
  expect(ids.length).toBeGreaterThan(10);
  // The reader namespaces every rendered id, and derives one for an unanchored heading.
  const anchors = links.map((link) => new URLSearchParams(link.href.split("?")[1]).get("a")!);
  expect(anchors).toEqual(ids.map((id) => (id.startsWith("h-") ? id.slice(2) : id)));
  // Every contents stop must reach a uniquely rendered section of this subsystem.
  for (const [index] of links.entries()) {
    const disclosure = page.locator(".article-contents");
    if (!(await disclosure.evaluate((e: HTMLDetailsElement) => e.open))) await disclosure.locator("summary").press("Enter");
    await disclosure.locator(`a[href="${links[index].href}"]`).press("Enter");
    await expect.poll(() => page.evaluate((anchor) => {
      const el = document.getElementById(`h-${anchor}`);
      if (!el) return false;
      const top = el.getBoundingClientRect().top;
      return top >= 0 && top < innerHeight;
    }, anchors[index])).toBe(true);
  }
  await page.keyboard.press("ControlOrMeta+End");
  const next = page.locator('.book-steps a[rel="next"]');
  await expect(next).toBeVisible();
  const nextTitle = (await next.locator(".step-title").innerText()).trim();
  await next.press("Enter");
  await expect(page.locator("h1")).not.toHaveText(target.title);
  await expect(page.locator("h1")).toHaveText(nextTitle);
  // Same focus move as below: wait for it, or the Enter on "previous" lands on <main>.
  await expect(page.locator("main#main")).toBeFocused();
  await page.locator('.book-steps a[rel="prev"]').press("Enter");
  await expect(page.locator("h1")).toHaveText(target.title);
  // The route change moves focus to <main> on the next frame. Wait for it before the next key,
  // or the Enter can land on <main> instead of the cross-reference link.
  await expect(page.locator("main#main")).toBeFocused();
  // Follow an actual typed cross-reference from this article.
  const cross = page.locator('.entry > .rich a[href^="#/entry/"]').first();
  await expect(cross).toBeAttached();
  await cross.press("Enter");
  const other = page.url();
  expect(other).not.toBe(start);
  await page.goBack(); await expect(page.locator("h1")).toHaveText(target.title);
  await page.goForward(); await expect.poll(() => page.url()).toBe(other);
});

test("Keyboard controls expose focus and native disclosure state at phone and desktop widths", async ({ page, browserName }) => {
  await installLibrary(page);
  // A letter the compiled spell list has, so the jump control exists in both libraries.
  const letter = firstEntry("a spell", (entry) => entry.kind === "spell").title[0].toUpperCase();
  // WebKit on macOS uses Option-Tab to include buttons in sequential navigation.
  const tabKey = browserName === "webkit" ? "Alt+Tab" : "Tab";
  for (const width of [390, 1280]) {
    await page.setViewportSize({ width, height: 844 });
    await page.goto("#/");
    // Route changes focus the reading area on the next frame. Test controls after that settles.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(null)))));
    await page.getByRole("searchbox", { name: "Search the rules", exact: true }).focus();
    await page.keyboard.press(tabKey);
    const focused = page.locator(":focus");
    await expect(focused).toHaveCount(1);
    expect(await focused.evaluate(el => {
      const s = getComputedStyle(el);
      return s.outlineStyle !== "none" && parseFloat(s.outlineWidth) >= 2;
    })).toBe(true);
    await expect(page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Home", exact: true })).toHaveAttribute("aria-current", "page");
    await page.goto("#/browse/kind/spell");
    const jump = page.getByRole("link", { name: `Jump to ${letter}`, exact: true });
    // Measure the control before activation: the route change remounts the jump bar.
    const rect = await jump.boundingBox();
    expect(rect!.width).toBeGreaterThanOrEqual(44); expect(rect!.height).toBeGreaterThanOrEqual(44);
    await jump.focus(); await page.keyboard.press("Enter");
    const group = page.locator(`#${letterId(letter)}`);
    await expect.poll(() => group.evaluate(el => el.getBoundingClientRect().top)).toBeGreaterThanOrEqual(0);
    await expect(group.locator(".letter-head")).toBeFocused();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});
