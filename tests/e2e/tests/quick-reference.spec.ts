import fs from "node:fs";
import path from "node:path";
import { installedTest as test, expect, loadCorpus, ROOT, type Page } from "./fixtures";
import { installLibrary, waitControlled } from "./helpers";

const corpus = loadCorpus();

interface Chip {
  group: string;
  label: string;
  id: string;
  anchor?: string;
}

/**
 * The chips as `crates/web/src/views/quick.rs` lists them, read from the source so that the
 * spec follows the constant instead of repeating it. A change to the shape of the constant
 * breaks the count below, and the spec with it.
 */
function quickChips(): Chip[] {
  const source = fs.readFileSync(path.join(ROOT, "crates/web/src/views/quick.rs"), "utf8");
  const constants = new Map<string, Omit<Chip, "group">[]>();
  const constant = /const (CONDITIONS|ACTIONS|COMBAT_MODIFIERS|EVERY_SESSION): &\[Chip\] = &\[([\s\S]*?)\n\];/g;
  for (const [, name, body] of source.matchAll(constant)) {
    const chips = [...body.matchAll(/chip(?:_at)?\(\s*"([^"]+)",\s*"([^"]+)"(?:,\s*"([^"]+)")?\s*,?\s*\)/g)].map(
      ([, label, id, anchor]) => ({ label, id, anchor }),
    );
    constants.set(name, chips);
  }
  const groups: [string, string][] = [
    ["Conditions", "CONDITIONS"],
    ["Actions", "ACTIONS"],
    ["Combat modifiers", "COMBAT_MODIFIERS"],
    ["Every session", "EVERY_SESSION"],
  ];
  return groups.flatMap(([group, name]) => (constants.get(name) ?? []).map((chip) => ({ group, ...chip })));
}

const chips = quickChips();
const groupNames = ["Conditions", "Actions", "Combat modifiers", "Every session"];
const groupSizes = [35, 7, 6, 9];

test.beforeEach(async ({ page, context }) => {
  await installLibrary(page);
  await waitControlled(page);
  await context.setOffline(true);
});

test("Quick reference lists its groups in order with chips from the source constant", async ({ page }) => {
  expect(chips.length).toBe(groupSizes.reduce((a, b) => a + b, 0));
  await page.goto("#/quick");
  await expect(page.getByRole("heading", { name: "Quick reference", level: 1 })).toBeVisible();
  await expect(page).toHaveTitle(/^Quick reference · Cast Iron$/);
  await expect(page.locator("main h2.sub-head")).toHaveText(groupNames);
  for (const [i, name] of groupNames.entries()) {
    const section = page.locator("main section", { has: page.getByRole("heading", { name, level: 2, exact: true }) });
    const list = section.locator("ul.jump-list");
    await expect(list.locator("a.jump")).toHaveCount(groupSizes[i]);
    await expect(list.locator("a.jump")).toHaveText(chips.filter((c) => c.group === name).map((c) => c.label));
  }
  // A page of links: it copies no rule text.
  expect(await page.locator("main").innerText()).not.toMatch(/penalty|DC \d/i);
});

test("Every chip opens its entry, and its anchor where it has one, offline", async ({ page }) => {
  test.setTimeout(240_000);
  for (const chip of chips) {
    const entry = corpus.entries.get(chip.id);
    expect(entry, `${chip.label}: ${chip.id} is not in the library`).toBeTruthy();
    await page.goto("#/quick");
    const link = page.locator("main .jump-list").getByRole("link", { name: chip.label, exact: true });
    await expect(link, chip.label).toHaveCount(1);
    const expected = chip.anchor ? `#/entry/${chip.id}?a=${chip.anchor}` : `#/entry/${chip.id}`;
    await expect(link).toHaveAttribute("href", expected);
    await link.click();
    await expect(page, chip.label).toHaveURL(new RegExp(`${expected.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`));
    await expect(page.getByRole("heading", { name: entry!.title, level: 1 }), chip.label).toBeVisible();
    if (chip.anchor) {
      const heading = page.locator(`#h-${chip.anchor}`);
      await expect(heading, `${chip.label}: no heading ${chip.anchor} in ${entry!.title}`).toBeVisible();
      await expect(heading, chip.label).toBeInViewport();
    }
  }
});

/**
 * The chips whose target is not the obvious entry, written out here and not read from
 * `quick.rs`: a wrong but valid id in the constant would otherwise pass the sweep above,
 * because the sweep takes its expectations from the same constant. `title` is the entry's
 * heading, `anchorText` the heading that the anchor must reach.
 */
const SPECIAL: { label: string; id: string; title: string; anchor?: string; anchorText?: string }[] = [
  { label: "Broken", id: "ref/rule/broken-and-fragile", title: "Broken and Fragile" },
  { label: "Dead", id: "ref/misc/dead", title: "Dead" },
  { label: "Grappled", id: "ref/rule/grapple-conditions", title: "Grapple Conditions" },
  { label: "Pinned", id: "ref/rule/grapple-conditions", title: "Grapple Conditions" },
  { label: "Ability damage and drain", id: "ref/rule/ability-damage-drain", title: "Ability Score Damage, Penalty, and Drain" },
  {
    label: "AC modifiers",
    id: "ref/rule/combat-modifiers",
    title: "Combat Modifiers",
    anchor: "sec-table-8-6-armor-class-modifiers",
    anchorText: "Table 8-6: Armor Class Modifiers",
  },
  {
    label: "What provokes",
    id: "ref/rule/attacks-of-opportunity",
    title: "Attacks of Opportunity",
    anchor: "sec-actions-that-provoke-attacks-of-opportunity",
    anchorText: "Actions That Provoke Attacks of Opportunity",
  },
  {
    label: "Two-weapon fighting",
    id: "ref/rule/two-weapon-fighting",
    title: "Two-Weapon Fighting",
    anchor: "sec-table-8-7-two-weapon-fighting-penalties",
    anchorText: "Table 8-7: Two-Weapon Fighting Penalties",
  },
];

test("The special chips open the entries and headings that the design names", async ({ page }) => {
  for (const want of SPECIAL) {
    const chip = chips.find((c) => c.label === want.label);
    expect(chip, `${want.label} is not in quick.rs`).toBeTruthy();
    expect(chip!.id, `${want.label}: id in quick.rs`).toBe(want.id);
    expect(chip!.anchor, `${want.label}: anchor in quick.rs`).toBe(want.anchor);
    expect(corpus.entries.get(want.id)?.title, `${want.id} in the library`).toBe(want.title);
    await page.goto("#/quick");
    await page.locator("main .jump-list").getByRole("link", { name: want.label, exact: true }).click();
    const url = want.anchor ? `#/entry/${want.id}?a=${want.anchor}` : `#/entry/${want.id}`;
    await expect(page, want.label).toHaveURL(new RegExp(`${url.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`));
    await expect(page.getByRole("heading", { name: want.title, level: 1 }), want.label).toBeVisible();
    if (want.anchor) {
      const heading = page.locator(`#h-${want.anchor}`);
      await expect(heading, want.label).toHaveText(want.anchorText!);
      await expect(heading, want.label).toBeInViewport();
    }
  }
});

test("The Table tab is the third primary tab and shows its state on the quick page", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("#/quick");
  const tabs = page.getByRole("navigation", { name: "Primary" }).getByRole("link");
  await expect(tabs).toHaveText(["Home", "Books", "Table", "Settings"]);
  await expect(tabs.nth(2)).toHaveAttribute("href", "#/quick");
  await expect(tabs.nth(2)).toHaveAttribute("aria-current", "page");
  for (const i of [0, 1, 3]) await expect(tabs.nth(i)).not.toHaveAttribute("aria-current", "page");
  for (const i of [0, 1, 2, 3]) await expect(tabs.nth(i).locator(".nav-icon svg")).toBeVisible();
  await tabs.nth(1).click();
  await expect(page).toHaveURL(/#\/books$/);
  await expect(tabs.nth(1)).toHaveAttribute("aria-current", "page");
  await expect(tabs.nth(2)).not.toHaveAttribute("aria-current", "page");
  await tabs.nth(2).click();
  await expect(page).toHaveURL(/#\/quick$/);
  await expect(tabs.nth(2)).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("heading", { name: "Quick reference", level: 1 })).toBeVisible();
});

test("At 390 px the bottom bar holds four equal targets of at least 44 px, Table third", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("#/quick");
  await expect(page.getByRole("heading", { name: "Quick reference", level: 1 })).toBeVisible();
  const boxes = await page.locator("header.app-header nav a.nav-link").evaluateAll((links) =>
    links.map((a) => {
      const r = a.getBoundingClientRect();
      return { text: (a.textContent ?? "").trim(), x: r.x, y: r.y, width: r.width, height: r.height };
    }),
  );
  expect(boxes.map((b) => b.text)).toEqual(["Home", "Books", "Table", "Settings"]);
  for (const box of boxes) {
    expect(box.width, box.text).toBeGreaterThanOrEqual(44);
    expect(box.height, box.text).toBeGreaterThanOrEqual(44);
    expect(box.y + box.height, `${box.text} is inside the window`).toBeLessThanOrEqual(844);
    expect(box.y, `${box.text} is at the bottom`).toBeGreaterThan(844 - 120);
    expect(Math.abs(box.width - boxes[0].width), `${box.text} has the width of the others`).toBeLessThanOrEqual(1);
  }
  for (let i = 1; i < boxes.length; i++) expect(boxes[i].x).toBeGreaterThanOrEqual(boxes[i - 1].x + boxes[i - 1].width - 1);
  await expect(page.locator("header.app-header nav a[aria-current=page]")).toHaveText("Table");
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  // The last chip of the page is not hidden behind the bar.
  const last = page.locator("main .jump-list a.jump").last();
  await last.scrollIntoViewIfNeeded();
  const lastBox = (await last.boundingBox())!;
  expect(lastBox.y + lastBox.height).toBeLessThanOrEqual(boxes[0].y);
});

async function expectHeaderFits(page: Page, width: number) {
  {
    await page.setViewportSize({ width, height: 800 });
    await page.goto("#/quick");
    await expect(page.getByRole("heading", { name: "Quick reference", level: 1 })).toBeVisible();
    const measure = () =>
      page.evaluate(() => {
        const header = document.querySelector("header.app-header")!;
        // The visually hidden status element has no box of its own to place.
        const items = [...header.children].filter((el) => !el.classList.contains("sr-only")).map((el) => {
          const r = el.getBoundingClientRect();
          return { cls: el.className || el.tagName, left: r.left, right: r.right, top: r.top, bottom: r.bottom };
        });
        const nav = header.querySelector("nav")!;
        const links = [...nav.querySelectorAll("a")].map((a) => {
          const r = a.getBoundingClientRect();
          return { text: (a.textContent ?? "").trim(), top: r.top, bottom: r.bottom, height: r.height };
        });
        const h1 = document.querySelector("main h1")!.getBoundingClientRect();
        return {
          items,
          links,
          headerHeight: header.getBoundingClientRect().height,
          h1Left: h1.left,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        };
      });
    const quick = await measure();
    expect(quick.overflow).toBeLessThanOrEqual(0);
    // One row: every header control is centred on the same line, and the header is as tall as
    // it is with one row of 44 px targets, not two.
    const centres = quick.items.map((i) => (i.top + i.bottom) / 2);
    expect(Math.max(...centres) - Math.min(...centres), JSON.stringify(quick.items)).toBeLessThanOrEqual(2);
    expect(quick.links.map((l) => l.text)).toEqual(["Home", "Books", "Table", "Settings"]);
    expect(Math.max(...quick.links.map((l) => l.top)) - Math.min(...quick.links.map((l) => l.top))).toBeLessThanOrEqual(1);
    expect(quick.headerHeight).toBeLessThan(80);
    // The controls do not overlap each other.
    for (let i = 1; i < quick.items.length; i++) expect(quick.items[i].left).toBeGreaterThanOrEqual(quick.items[i - 1].right - 1);

    // The header sits in the same place on every page: the brand at the page column's left
    // edge, and the last control at the same right edge as on Home.
    const brandLeft = quick.items[0].left;
    const lastRight = quick.items[quick.items.length - 1].right;
    expect(Math.abs(quick.h1Left - brandLeft), "the heading and the brand share a left edge").toBeLessThanOrEqual(1);
    await page.goto("#/");
    await expect(page.locator("section[aria-labelledby=by-kind] a.cat").first()).toBeVisible();
    const home = await measure();
    expect(Math.abs(home.items[0].left - brandLeft)).toBeLessThanOrEqual(1);
    expect(Math.abs(home.items[home.items.length - 1].right - lastRight)).toBeLessThanOrEqual(1);
    expect(home.h1Left, "Home's heading starts at the brand's edge").toBeCloseTo(brandLeft, 0);
    const categories = await page.locator("section[aria-labelledby=by-kind] ul.cats").evaluate((el) => el.getBoundingClientRect().right);
    expect(Math.abs(categories - lastRight), "the last header control ends at the category grid's right edge").toBeLessThanOrEqual(1);
  }
}

test("At 768 px the header stays on one row with the column alignment of #409", async ({ page }) => {
  await expectHeaderFits(page, 768);
});

test("At 1024 px the header stays on one row with the column alignment of #409", async ({ page }) => {
  await expectHeaderFits(page, 1024);
});

test("At 1280 px the header stays on one row with the column alignment of #409", async ({ page }) => {
  await expectHeaderFits(page, 1280);
});
