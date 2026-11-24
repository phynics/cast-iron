import { installedTest as test, expect, goOffline, loadCorpus } from "./fixtures";
import {
  installLibrary,
  waitControlled,
  bodyHeadings,
  bodyLinks,
  bookSources,
  bookTrees,
  corpusEntry,
  bookReadingOrder,
  firstEntry,
  letterId,
  openBookEntry,
  pathToEntry,
  searchIcon,
  waitHomeIdle,
  waitReady,
  writeMeta,
} from "./helpers";

/**
 * A placed rule article long enough for the keyboard walk: more than ten headings, at
 * least one typed cross-reference, and a following entry in its book. The ids and chapter
 * structure change with the corpus, so the test asks the compiled library for the article
 * instead of naming one. Take the shortest such article: the walk visits every section, and
 * the longest subsystem took several minutes under WebKit.
 */
function readingTarget() {
  const entries = [...loadCorpus().entries.values()];
  const books = bookTrees();
  const place = new Map<string, { book: (typeof books)[number]; order: string[] }>();
  for (const book of books) {
    const order = bookReadingOrder(book);
    for (const id of order) if (!place.has(id)) place.set(id, { book, order });
  }
  const candidates = entries
    .filter((entry) =>
      entry.kind === "rule" &&
      bodyHeadings(entry).filter((heading) => heading.text.trim()).length > 10 &&
      bodyLinks(entry.body).length > 0 &&
      place.has(entry.id) &&
      place.get(entry.id)!.order.indexOf(entry.id) < place.get(entry.id)!.order.length - 1)
    .sort((a, b) => bodyHeadings(a).length - bodyHeadings(b).length);
  const target = candidates[0];
  if (!target) throw new Error("The compiled corpus has no placed rule article for the keyboard reading test.");
  return { target, book: place.get(target.id)!.book };
}

test("Keyboard reading traverses a complete subsystem offline and preserves book history", async ({ page, context, server }) => {
  // The walk visits every contents stop, then goes forward and back in the book. On the
  // canonical library it takes about 100-115 s on a quiet machine and up to about 200 s under
  // load; the shortest candidate still walks more than ten sections, and the ceiling keeps
  // headroom for the chapter page that now precedes the article.
  // The per-step assertions still fail fast; only the whole-walk ceiling moves.
  test.setTimeout(300_000);
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
  // run the next evaluation before the toggle, which returns no links at all. This assertion checks
  // that the disclosure opened. `openBookEntry` waits out the route's pending focus frame, so the
  // Enter is not lost to <main> (#121).
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
    await searchIcon(page).focus();
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

test("Keyboard focus stays clear of the fixed bars and small links keep a 44 px target at phone width (#12)", async ({ page, browserName }) => {
  await installLibrary(page);
  await waitControlled(page);
  const tabKey = browserName === "webkit" ? "Alt+Tab" : "Tab";
  await page.setViewportSize({ width: 390, height: 844 });
  const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve(null)))));

  // H1: walking a book's Contents, a focused control never sits under the bottom tab bar.
  await page.goto("#/browse/book/core-rulebook");
  await expect(page.locator("h1")).toBeVisible();
  await settle();
  await searchIcon(page).focus();
  const nav = page.getByRole("navigation", { name: "Primary" });
  await expect(nav).toBeVisible();
  let scrolled = 0;
  for (let step = 0; step < 40; step++) {
    await page.keyboard.press(tabKey);
    const state = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement;
      const bar = document.querySelector('nav[aria-label="Primary"]')!.getBoundingClientRect();
      return {
        inBar: !!el.closest('nav[aria-label="Primary"]'),
        bottom: el.getBoundingClientRect().bottom,
        barTop: bar.top,
        scrollY: window.scrollY,
        name: (el.textContent ?? "").trim().slice(0, 40),
      };
    });
    if (state.inBar) continue;
    expect(state.bottom, `focus step ${step} ("${state.name}") ends above the bottom tab bar`).toBeLessThanOrEqual(state.barTop + 0.5);
    scrolled = Math.max(scrolled, state.scrollY);
  }
  // The walk must have reached past the first screen, or it proved nothing.
  expect(scrolled).toBeGreaterThan(0);

  // M2: a focused scope pill is fully visible, and the Filters button does not cover it.
  await page.goto("#/search?q=fire");
  const spells = page.locator(".search-scopes .scope-choice", { hasText: /^Spells$/ });
  await expect(spells).toBeVisible();
  await page.locator(".search-scopes .scope-choice").first().focus();
  for (let step = 0; step < 6; step++) {
    if (await spells.evaluate((el) => el === document.activeElement)) break;
    await page.keyboard.press(tabKey);
  }
  await expect(spells).toBeFocused();
  // Scrolling into view runs on the next frame.
  await settle();
  await expect.poll(async () => {
    const pill = (await spells.boundingBox())!;
    const filters = (await page.locator(".search-controls .filters-toggle").boundingBox())!;
    const scroller = (await page.locator(".search-scopes").boundingBox())!;
    return {
      clearOfFilters: pill.x + pill.width <= filters.x + 0.5,
      insideScroller: pill.x >= scroller.x - 0.5 && pill.x + pill.width <= scroller.x + scroller.width + 0.5,
    };
  }).toEqual({ clearOfFilters: true, insideScroller: true });

  // L7: the diagnostics way back and the book page's legal line keep a 44 px target. The hit
  // area is the link's box including its ::after, so probe the point 20 px below the text's
  // centre: it must still land on the link.
  const reaches = async (link: ReturnType<typeof page.locator>) => {
    await link.scrollIntoViewIfNeeded();
    const box = (await link.boundingBox())!;
    const x = box.x + box.width / 2;
    const cy = box.y + box.height / 2;
    return page.evaluate(([px, py]) => {
      const hit = (dy: number) => !!document.elementFromPoint(px, py + dy)?.closest("a");
      return { above: hit(-20), below: hit(20) };
    }, [x, cy]);
  };
  await page.goto("#/settings/diagnostics");
  await expect(page.getByRole("link", { name: "Back to Settings" })).toBeVisible();
  expect(await reaches(page.getByRole("link", { name: "Back to Settings" }))).toEqual({ above: true, below: true });
  await page.goto("#/browse/book/core-rulebook");
  const legal = page.locator("footer.colophon").getByRole("link", { name: "Legal and attribution" });
  await expect(legal).toBeVisible();
  expect(await reaches(legal)).toEqual({ above: true, below: true });
});

test("Keyboard and touch readers get a 44 px original-text disclosure on a revision in force (#12)", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("#/browse/book/core-rulebook");
  const all = bookSources();
  await page.goto("#/");
  await waitHomeIdle(page);
  await writeMeta(page, "disabled_books", all.filter((id) => id !== "core-rulebook" && id !== "the-elephant-in-the-room-feat-taxes-in-pathfinder"));
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await page.goto("#/entry/ref/feat/dodge");
  const summary = page.locator("details.rel-original > summary");
  await expect(summary).toContainText("Original text");
  expect((await summary.boundingBox())!.height).toBeGreaterThanOrEqual(43.5);
});

test("Keyboard and touch readers keep the tab bar and stat values usable at 200 % text (#12 L6)", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve(null)))));
  const setRoot = async (percent: number) => {
    await page.evaluate((value) => { document.documentElement.style.fontSize = `${value}%`; }, percent);
    await settle();
  };
  const overflow = () => page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  // The widest offenders, so a failure names the element that pushes the page sideways.
  const offenders = () => page.evaluate(() => Array.from(document.body.querySelectorAll("*"))
    .map((el) => ({ el, right: el.getBoundingClientRect().right }))
    .filter((hit) => hit.right > innerWidth + 0.5 && !hit.el.closest(".table-wrap"))
    .slice(0, 8)
    .map((hit) => `${hit.el.tagName.toLowerCase()}.${hit.el.className} right=${Math.round(hit.right)}`));
  // Each tab of the bottom bar is inside the viewport, at least 44 px each way, and keeps its
  // accessible name and the current marker whether its label shows or not.
  const tabs = async () => {
    const nav = page.getByRole("navigation", { name: "Primary" });
    for (const name of ["Home", "Books", "Table", "Settings"]) {
      const link = nav.getByRole("link", { name, exact: true });
      const box = (await link.boundingBox())!;
      const width = await page.evaluate(() => innerWidth);
      expect(box, `${name} tab`).toBeTruthy();
      expect(box.x, `${name} tab left edge`).toBeGreaterThanOrEqual(-0.5);
      expect(box.x + box.width, `${name} tab right edge`).toBeLessThanOrEqual(width + 0.5);
      expect(box.width, `${name} tab width`).toBeGreaterThanOrEqual(43.5);
      expect(box.height, `${name} tab height`).toBeGreaterThanOrEqual(43.5);
    }
    await expect(nav.locator('a[aria-current="page"]')).toHaveCount(1);
  };
  const labelWidth = () => page.locator('nav[aria-label="Primary"] .nav-label').first().evaluate((el) => el.getBoundingClientRect().width);

  // A word is split only when a client rect of it sits on two lines. Hyphens and slashes are
  // legitimate break points, so a token is the run between whitespace, hyphens and slashes.
  const splitWords = (selector: string) => page.evaluate((sel) => {
    const split: string[] = [];
    for (const root of Array.from(document.querySelectorAll(sel))) {
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode() as Text | null; node; node = walker.nextNode() as Text | null) {
        for (const match of node.data.matchAll(/[^\s\-\u2010-\u2015/]+/g)) {
          const range = document.createRange();
          range.setStart(node, match.index!);
          range.setEnd(node, match.index! + match[0].length);
          const lines = new Set(Array.from(range.getClientRects()).filter((r) => r.width > 0).map((r) => Math.round(r.top)));
          // Superscript markers sit a few pixels higher than the line; compare line boxes only.
          if (lines.size > 1 && !node.parentElement!.closest("sup")) split.push(match[0]);
        }
      }
    }
    return split;
  }, selector);

  const viewports = [{ width: 390, height: 844 }, { width: 320, height: 700 }];
  const pages = [
    { name: "Fireball", route: `#/entry/${corpusEntry("spell", "Fireball").id}` },
    { name: "Goblin", route: `#/entry/${corpusEntry("creature", "Goblin").id}` },
    { name: "Wizard", route: "#/entry/ref/class/wizard" },
  ];
  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await page.goto("#/");
    await waitHomeIdle(page);
    await setRoot(100);
    await tabs();
    expect(await labelWidth(), `${viewport.width} px at 100 %: the labels show`).toBeGreaterThan(10);
    for (const percent of [125, 150, 175, 200]) {
      await setRoot(percent);
      await tabs();
      expect(await overflow(), `${viewport.width} px at ${percent} %: home scrolls sideways: ${await offenders()}`).toBeLessThanOrEqual(0);
    }
    expect(await labelWidth(), `${viewport.width} px at 200 %: the labels are hidden, the icons stay`).toBeLessThanOrEqual(1);
    for (const { name, route } of pages) {
      await page.goto(route);
      await expect(page.locator("h1")).toBeVisible();
      await setRoot(200);
      const blocks = page.locator(".stat-block");
      expect(await blocks.count(), `${name} has a stat block`).toBeGreaterThan(0);
      expect(await overflow(), `${viewport.width} px at 200 %: ${name} scrolls sideways: ${await offenders()}`).toBeLessThanOrEqual(0);
      expect(await splitWords(".stat-block"), `${viewport.width} px at 200 %: ${name} splits words`).toEqual([]);
      await tabs();
    }
    await setRoot(100);
  }
});

// Relative luminance and contrast ratio of two `rgb()` colours, as WCAG defines them.
function contrastRatio(a: string, b: string): number {
  const luminance = (css: string) => {
    const [r, g, b] = css.match(/[\d.]+/g)!.slice(0, 3).map((v) => {
      const c = Number(v) / 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

test("Keyboard users get a fixed star name, 3:1 control edges and an answer to the storage request (#12 L2, L4, L5)", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);

  // L4: the star keeps its name; aria-pressed alone carries the state.
  const spell = firstEntry("a spell", (entry) => entry.kind === "spell");
  await page.goto(`#/entry/${spell.id}`);
  const star = page.locator(".bookmark-toggle");
  await expect(star).toBeEnabled();
  await expect(star).toHaveAccessibleName(`Star ${spell.title}`);
  await expect(star).toHaveAttribute("aria-pressed", "false");
  await star.focus();
  await page.keyboard.press("Enter");
  await expect(star).toHaveAttribute("aria-pressed", "true");
  await expect(star).toHaveAccessibleName(`Star ${spell.title}`);
  await expect(star).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(star).toHaveAttribute("aria-pressed", "false");
  await expect(star).toHaveAccessibleName(`Star ${spell.title}`);

  // L5: the border of a control is at least 3:1 against the page, in both themes.
  const edge = (selector: string) => page.locator(selector).first().evaluate((el) => {
    const css = getComputedStyle(el);
    return { border: css.borderTopColor, width: css.borderTopWidth };
  });
  const pageGround = () => page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor);
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto("#/spells/Wizard/3");
    await expect(page.locator(".filter-btn").first()).toBeVisible();
    await expect(page.locator(".level-nav .lvl").first()).toBeVisible();
    const controls: Record<string, string> = { "filter button": ".filter-btn", "level tab": ".level-nav a.lvl:not([aria-current])" };
    for (const [name, selector] of Object.entries(controls)) {
      const { border, width } = await edge(selector);
      expect(width, `${name} has a border`).not.toBe("0px");
      const ratio = contrastRatio(border, await pageGround());
      expect(ratio, `${name} edge ${border} in ${scheme}: ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(3);
    }
    await page.goto("#/search?q=fire");
    const pill = page.locator(".search-scopes .scope-choice:not([aria-pressed=\"true\"])").first();
    await expect(pill).toBeVisible();
    const { border } = await edge(".search-scopes .scope-choice:not([aria-pressed=\"true\"])");
    const ratio = contrastRatio(border, await pageGround());
    expect(ratio, `scope pill edge ${border} in ${scheme}: ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(3);
  }
  await page.emulateMedia({ colorScheme: null });

  // L2: the storage request answers. Chromium refuses it in tests; the answer is a status line.
  await page.goto("#/settings");
  const request = page.getByRole("button", { name: "Request persistent storage" });
  await expect(request).toBeVisible();
  const status = page.getByTestId("storage-request-status");
  await expect(status).toHaveAttribute("role", "status");
  await expect(status).toHaveText("");
  await request.focus();
  await page.keyboard.press("Enter");
  await expect(status).toHaveText(/^(Granted\. The browser will not clear|Not granted\. On iPhone, add Cast Iron to the Home Screen)/);
  if (/^Not granted/.test(await status.innerText())) await expect(request).toBeFocused();
  else await expect(status).toBeFocused();
});
