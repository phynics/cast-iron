import { installedTest as test, expect, loadCorpus, type Page } from "./fixtures";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;

/** A compiled record, by id, with the checks the pin must satisfy before the page can link. */
function storedEntry(id: string, kind: string) {
  const entry = corpus.entries.get(id);
  if (!entry) throw new Error(`The compiled corpus has no ${id}.`);
  if (entry.kind !== kind) throw new Error(`${id} is a ${entry.kind}, not a ${kind}.`);
  return entry;
}

const POWER_ATTACK = storedEntry("ref/feat/power-attack", "feat");
const FIREBALL = storedEntry("ref/spell/fireball", "spell");
const GOBLIN = storedEntry("ref/monster/goblin", "creature");
const DODGE = storedEntry("ref/feat/dodge", "feat");
const CASTING_CONDUIT = storedEntry("ref/feat/casting-conduit", "feat");
const CORE_RULEBOOK = sources.find((row) => row.id === "core-rulebook");
const ARTIFACTS_AND_LEGENDS = sources.find((row) => row.id === "artifacts-and-legends");

/**
 * Nothing but the link's own anchor may name the reference site. A `link` hint (preconnect,
 * dns-prefetch, prefetch), an `img`, a `script`, or an `iframe` would either make a request or
 * tell the browser to open a connection before the tap; neither is visible in the request log
 * for every kind of hint, so this checks the markup directly.
 */
async function noAonResources(page: Page) {
  const named = await page.evaluate(() =>
    [...document.querySelectorAll("link, img, script, iframe")].flatMap((element) =>
      [...element.attributes]
        .filter((attribute) => /aonprd\.com/i.test(attribute.value))
        .map(
          (attribute) => `${element.tagName.toLowerCase()}[${attribute.name}]=${attribute.value}`,
        ),
    ),
  );
  expect(named, "a preconnect hint, image, script or frame names aonprd.com").toEqual([]);
}

/**
 * The compact line keeps a text box shorter than the 44 px target. Scan outward from that box
 * with `elementFromPoint`, as a tap would, to measure the box the anchor really reaches; the
 * extra height comes from `.meta-link::after`, so the layout does not move.
 */
async function hitHeight(page: Page, selector: string) {
  return page.evaluate((sel) => {
    const link = document.querySelector(sel) as HTMLElement;
    link.scrollIntoView({ block: "center" });
    const rect = link.getBoundingClientRect();
    const x = rect.x + rect.width / 2;
    const hits = (y: number) => {
      const element = document.elementFromPoint(x, y);
      return element !== null && (element === link || link.contains(element));
    };
    let top = Math.round(rect.top);
    let bottom = Math.round(rect.bottom);
    while (top > 0 && hits(top - 1)) top -= 1;
    while (bottom < window.innerHeight - 1 && hits(bottom + 1)) bottom += 1;
    return { box: Math.round(rect.height), hit: bottom - top + 1 };
  }, selector);
}

test("The pin carries the harvested paths these tests need", () => {
  for (const entry of [POWER_ATTACK, FIREBALL, GOBLIN]) {
    expect(entry.aon, `${entry.id} has an AoN path`).toBeTruthy();
  }
  expect(CASTING_CONDUIT.aon, "Casting Conduit has no AoN path").toBeFalsy();
  expect(CORE_RULEBOOK?.aon, "Core Rulebook has a source page").toBeTruthy();
  expect(ARTIFACTS_AND_LEGENDS?.aon, "Artifacts and Legends has no source page").toBeFalsy();
});

test("A feat and a spell link to their stored Archives of Nethys page", async ({ page, guard }) => {
  for (const entry of [POWER_ATTACK, FIREBALL]) {
    await page.goto(`#/entry/${entry.id}`);
    await expect(page.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    await noAonResources(page);

    const link = page.getByTestId("aon-link");
    await expect(link).toHaveCount(1);
    await expect(link).toContainText("View on Archives of Nethys");
    // The stored path is already percent-encoded: the URL is the fixed base plus that path.
    await expect(link).toHaveAttribute("href", `https://aonprd.com/${entry.aon}`);
    await expect(link).toHaveAttribute("target", "_blank");
    await expect(link).toHaveAttribute("rel", "noopener noreferrer");
    await expect(link).toHaveAccessibleName(/View on Archives of Nethys \(opens in a new tab\)/);
  }

  // The link closes the entry's own text: directly after the typed Sources line, before the
  // related panels.
  await page.goto(`#/entry/${POWER_ATTACK.id}`);
  await expect(page.locator(".entry > .sources-line + .aon-line + #entry-related")).toHaveCount(1);

  // The line reads as secondary: the same weight as the citation line it follows.
  const weights = await page.evaluate(() => ({
    sources: getComputedStyle(document.querySelector(".entry > .sources-line")!).fontWeight,
    aon: getComputedStyle(document.querySelector(".entry > .aon-line a")!).fontWeight,
  }));
  expect(weights.aon, "the trail link is no heavier than the citation line").toBe(weights.sources);

  // The line stays compact and still reaches the 44 px target at a phone width.
  await page.setViewportSize({ width: 390, height: 844 });
  const target = await hitHeight(page, ".entry > .aon-line a");
  expect(target.box, "the visible text box stays compact").toBeLessThan(44);
  expect(target.hit, "the hit area reaches the 44 px target").toBeGreaterThanOrEqual(44);

  // Browsing the pages made no request to the reference site; the link waits for the tap.
  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

test("A creature links and an entry without a stored path shows no link", async ({ page, guard }) => {
  await page.goto(`#/entry/${GOBLIN.id}`);
  await expect(page.getByRole("heading", { level: 1, name: GOBLIN.title })).toBeVisible();
  await noAonResources(page);
  await expect(page.getByTestId("aon-link")).toHaveAttribute(
    "href",
    `https://aonprd.com/${GOBLIN.aon}`,
  );

  await page.goto(`#/entry/${CASTING_CONDUIT.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CASTING_CONDUIT.title })).toBeVisible();
  await noAonResources(page);
  await expect(page.getByTestId("aon-link")).toHaveCount(0);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

test("The AoN target leaves its neighbours their own edges at a phone width", async ({ page, guard }) => {
  // Dodge's citation wraps to two lines at 390 px, so the last book link sits directly above the
  // AoN line: the tightest case for the 44 px target's reach.
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`#/entry/${DODGE.id}`);
  await expect(page.getByRole("heading", { level: 1, name: DODGE.title })).toBeVisible();
  await noAonResources(page);

  const owns = await page.evaluate(() => {
    const paragraph = document.querySelector(".entry > .sources-line") as HTMLElement;
    const links = [...paragraph.querySelectorAll("a")] as HTMLElement[];
    const lastCitation = links[links.length - 1];
    const aon = document.querySelector(".entry > .aon-line a") as HTMLElement;
    const related = document.querySelector(".entry > #entry-related") as HTMLElement;
    aon.scrollIntoView({ block: "center" });
    const citation = lastCitation.getBoundingClientRect();
    const block = paragraph.getBoundingClientRect();
    const below = related.getBoundingClientRect();
    const ownedBy = (element: Element, x: number, y: number) => {
      const hit = document.elementFromPoint(x, y);
      return hit !== null && (hit === element || element.contains(hit));
    };
    return {
      // A point 1 px inside the citation link's own bottom edge.
      citationEdge: ownedBy(
        lastCitation,
        citation.x + citation.width / 2,
        citation.bottom - 1,
      ),
      // A point 1 px inside the citation block's bottom edge: the target may not reach into it,
      // even in the space below the link's own box.
      citationBlockEdge: ownedBy(paragraph, block.x + block.width / 2, block.bottom - 1),
      // The related panel's top edge stays its own.
      relatedEdge: ownedBy(related, below.x + 20, below.top),
    };
  });
  expect(owns.citationEdge, "the last citation link owns a point inside its bottom edge").toBe(true);
  expect(owns.citationBlockEdge, "the citation block owns a point inside its bottom edge").toBe(true);
  expect(owns.relatedEdge, "the related panel owns its top edge").toBe(true);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

test("A book with a source page links and one without shows no link", async ({ page, guard }) => {
  await page.goto(`#/browse/book/${CORE_RULEBOOK.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CORE_RULEBOOK.title })).toBeVisible();
  await noAonResources(page);

  const link = page.getByTestId("aon-link");
  await expect(link).toHaveCount(1);
  await expect(link).toContainText("View on Archives of Nethys");
  await expect(link).toHaveAttribute("href", `https://aonprd.com/${CORE_RULEBOOK.aon}`);
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", "noopener noreferrer");
  await expect(link).toHaveAccessibleName(/View on Archives of Nethys \(opens in a new tab\)/);

  // The colophon keeps the same compact line and the same 44 px target.
  await page.setViewportSize({ width: 390, height: 844 });
  const target = await hitHeight(page, ".colophon .aon-line a");
  expect(target.box, "the visible text box stays compact").toBeLessThan(44);
  expect(target.hit, "the hit area reaches the 44 px target").toBeGreaterThanOrEqual(44);

  await page.goto(`#/browse/book/${ARTIFACTS_AND_LEGENDS.id}`);
  await expect(
    page.getByRole("heading", { level: 1, name: ARTIFACTS_AND_LEGENDS.title }),
  ).toBeVisible();
  await noAonResources(page);
  await expect(page.getByTestId("aon-link")).toHaveCount(0);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

/**
 * Tapping the link must leave Cast Iron's own window on the same entry (#514). Where the platform
 * puts the new page is not the app's choice (see
 * docs/decisions/2026-11-19-aon-link-opens-where-the-platform-decides.md), but the app must never
 * navigate its own window to the reference site, and no app code may intercept the tap. The tap
 * is the only moment a request to the reference site is allowed, so this test answers it locally.
 */
test("Tapping the link opens a separate page and leaves the app on its entry", async ({
  page,
  context,
  guard,
}) => {
  const tapped: string[] = [];
  await context.route("https://aonprd.com/**", (route) => {
    tapped.push(route.request().url());
    return route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<!doctype html><title>Archives of Nethys stand-in</title>",
    });
  });

  await page.goto(`#/entry/${POWER_ATTACK.id}`);
  await expect(page.getByRole("heading", { level: 1, name: POWER_ATTACK.title })).toBeVisible();
  await noAonResources(page);
  const appUrl = page.url();
  expect(tapped, "no request to the reference site before the tap").toEqual([]);

  const popup = context.waitForEvent("page", { timeout: 15_000 });
  await page.getByTestId("aon-link").click();
  const opened = await popup;
  await opened.waitForLoadState("domcontentloaded");

  expect(opened, "the link opened a separate page").not.toBe(page);
  expect(opened.url()).toBe(`https://aonprd.com/${POWER_ATTACK.aon}`);
  expect(await opened.evaluate(() => window.opener), "noopener: no handle back to the app").toBeNull();

  // The app's own window did not navigate, and the entry is still on screen.
  expect(page.url(), "the app window keeps its address").toBe(appUrl);
  await expect(page.getByRole("heading", { level: 1, name: POWER_ATTACK.title })).toBeVisible();
  expect(tapped, "exactly the tapped request reached the reference site").toEqual([
    `https://aonprd.com/${POWER_ATTACK.aon}`,
  ]);

  // The tap is the one request the console guard may not count against the test.
  guard.external.length = 0;
  await opened.close();
});

/**
 * Neither Android (a Custom Tab) nor iOS (Safari View Controller) lets a page choose the browser
 * (docs/decisions/2026-11-21-aon-link-copy-action.md), so `Copy link` puts the same URL on the
 * clipboard for any browser. It makes no request, and it does not move the app.
 */
test("Copy link puts the stored URL on the clipboard and makes no request", async ({
  page,
  context,
  guard,
  browserName,
}) => {
  // Only Chromium lets a test grant the clipboard. Other browsers still run the click and status.
  if (browserName === "chromium") {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  }
  const tapped: string[] = [];
  await context.route("https://aonprd.com/**", (route) => {
    tapped.push(route.request().url());
    return route.abort();
  });

  await page.goto(`#/entry/${POWER_ATTACK.id}`);
  await expect(page.getByRole("heading", { level: 1, name: POWER_ATTACK.title })).toBeVisible();
  const appUrl = page.url();
  const copy = page.getByRole("button", { name: "Copy link", exact: true });
  await expect(copy).toHaveCount(1);
  await expect(copy).toHaveAttribute("data-testid", "aon-copy");
  const status = page.getByTestId("aon-copy-status");
  await expect(status).toHaveText("");

  const outcome = browserName === "chromium" ? "Link copied." : /^(Link copied\.|Could not copy\.)/;
  await copy.click();
  await expect(status).toHaveText(outcome);
  if (browserName === "chromium") {
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
      `https://aonprd.com/${POWER_ATTACK.aon}`,
    );
  }
  // A second tap clears the line and sets it again.
  await copy.click();
  await expect(status).toHaveText(outcome);
  expect(page.url(), "the app window keeps its address").toBe(appUrl);
  expect(tapped, "copying makes no request to the reference site").toEqual([]);
  await noAonResources(page);

  // At a phone width the button keeps the 44 px target.
  await page.setViewportSize({ width: 390, height: 844 });
  const target = await hitHeight(page, ".entry > .aon-line .aon-copy");
  expect(target.hit, "the hit area reaches the 44 px target").toBeGreaterThanOrEqual(44);

  // An entry without a stored path has neither the link nor the button.
  await page.goto(`#/entry/${CASTING_CONDUIT.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CASTING_CONDUIT.title })).toBeVisible();
  await expect(page.getByTestId("aon-copy")).toHaveCount(0);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

/**
 * The link and `Copy link` each expand their hit area past their text (`.meta-link::after`), and
 * sit on one line. At a phone width each must still own a point 1 px inside its own edge, and the
 * blocks above and below the line keep theirs.
 */
test("The AoN link and Copy link keep their own edges at a phone width", async ({ page, guard }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`#/entry/${DODGE.id}`);
  await expect(page.getByRole("heading", { level: 1, name: DODGE.title })).toBeVisible();

  const owns = await page.evaluate(() => {
    const link = document.querySelector(".entry > .aon-line a") as HTMLElement;
    const button = document.querySelector(".entry > .aon-line .aon-copy") as HTMLElement;
    const paragraph = document.querySelector(".entry > .sources-line") as HTMLElement;
    const related = document.querySelector(".entry > #entry-related") as HTMLElement;
    link.scrollIntoView({ block: "center" });
    const ownedBy = (element: Element, x: number, y: number) => {
      const hit = document.elementFromPoint(x, y);
      return hit !== null && (hit === element || element.contains(hit));
    };
    const a = link.getBoundingClientRect();
    const b = button.getBoundingClientRect();
    const block = paragraph.getBoundingClientRect();
    const below = related.getBoundingClientRect();
    const midA = a.y + a.height / 2;
    const midB = b.y + b.height / 2;
    return {
      sameLine: Math.abs(midA - midB) < a.height,
      linkRight: ownedBy(link, a.right - 1, midA),
      buttonLeft: ownedBy(button, b.left + 1, midB),
      buttonRight: ownedBy(button, b.right - 1, midB),
      citationBlockEdge: ownedBy(paragraph, block.x + block.width / 2, block.bottom - 1),
      relatedEdge: ownedBy(related, below.x + 20, below.top),
    };
  });
  expect(owns.sameLine, "the link and the button share a line at 390 px").toBe(true);
  expect(owns.linkRight, "the link owns a point 1 px inside its right edge").toBe(true);
  expect(owns.buttonLeft, "the button owns a point 1 px inside its left edge").toBe(true);
  expect(owns.buttonRight, "the button owns a point 1 px inside its right edge").toBe(true);
  expect(owns.citationBlockEdge, "the citation block owns its bottom edge").toBe(true);
  expect(owns.relatedEdge, "the related panel owns its top edge").toBe(true);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});
