import type { Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { corpusEntry, isMember, memberBooks, count, featLeadsTo, groupLetter, installLibrary, letterId, num, openBookGroups, openQuickSearch, quickField, waitControlled, waitResults } from "./helpers";

const corpus = loadCorpus();
/** The books the corpus marks third party (#189); a settled profile starts without them. */
const thirdParty = new Set<string>(
  corpus.catalog.sources.filter((row: any) => row.third_party).map((row: any) => row.id as string),
);
/** The catalog rows the default active set shows: every entry with a member book that is not third party. */
const activeStubs = corpus.catalog.entries.filter(
  (stub: any) => memberBooks(stub).some((book) => !thirdParty.has(book)),
);
const activeIds = new Set(activeStubs.map((stub: any) => stub.id as string));

test("A long kind opens one letter group, focuses it, and Back returns to it", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const fireball = corpusEntry("spell", "Fireball");

  await page.goto("#/browse/kind/spell?l=F");
  await expect(page.getByRole("navigation", { name: "Jump to letter" })).toBeVisible();
  const group = page.locator(`#${letterId("F")}`);
  const heading = group.locator(".letter-head");
  await expect(heading).toHaveText(/^F/);
  await expect(heading).toBeInViewport();
  await expect(heading).toBeFocused();
  await expect(page.locator('.letter-btn[aria-current="true"]')).toHaveText("F");
  // Only the selected group holds entry links; the other letters stay collapsed.
  await expect(page.locator(".letter-group .letter-head-link").first()).toBeVisible();

  // A duplicated title sits inside a SourceVariants disclosure, so follow the entry href.
  const link = group.locator(`a[href="#/entry/${fireball.id}"]`);
  await expect(link).toHaveCount(1);
  const details = link.locator("xpath=ancestor::details");
  if ((await details.count()) > 0 && !(await details.evaluate((element: HTMLDetailsElement) => element.open))) {
    await details.locator("summary").click();
  }
  await link.click();
  await expect(page.getByRole("heading", { level: 1, name: fireball.title })).toBeVisible();
  await expect.poll(() => new URL(page.url()).hash).toContain(`#/entry/${fireball.id}`);

  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/spell?l=F");
  await expect(page.locator(`#${letterId("F")} .letter-head`)).toBeInViewport();
  await expect(page.locator(`#${letterId("F")} .letter-head`)).toBeFocused();
  await expect(page.locator(`#${letterId("F")} .entry-list`)).toBeVisible();
  await expect(page.locator('.letter-btn[aria-current="true"]')).toHaveText("F");
});

test("A short kind stays one flat list without a jump bar", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const classes = corpus.catalog.entries.filter((stub) => stub.kind === "class");
  expect(classes.length).toBeLessThanOrEqual(200);
  await page.goto("#/browse/kind/class");
  await expect(page.getByRole("navigation", { name: "Jump to letter" })).toHaveCount(0);
  await expect(page.locator('.entry-list a[href^="#/entry/"]')).toHaveCount(classes.length);
});

test("A kind's list page is a title, a count line, the filters, a letter bar and rows, with no breadcrumb, lede or search link", async ({ page }) => {
  await installLibrary(page);
  const feats = activeStubs.filter((stub) => stub.kind === "feat");
  expect(feats.length).toBeGreaterThan(200);
  await page.goto("#/browse/kind/feat");
  const main = page.locator("main");
  const title = main.getByRole("heading", { level: 1, name: "Feats" });
  await expect(title).toBeVisible();
  // The count line names the kind and is a live region.
  const line = main.locator(".list-count");
  await expect(line).toHaveText(`${count(feats.length)} feats`);
  await expect(line).toHaveAttribute("aria-live", "polite");

  // What the template removed stays removed.
  await expect(main.locator("nav.crumbs")).toHaveCount(0);
  await expect(main.getByRole("navigation", { name: "Breadcrumb" })).toHaveCount(0);
  await expect(main.locator(".lede-line")).toHaveCount(0);
  await expect(main.getByText("Feats by type")).toHaveCount(0);
  await expect(main.getByRole("link", { name: /^Search feats$/ })).toHaveCount(0);
  await expect(main.getByText("in your active books")).toHaveCount(0);

  // The order, top to bottom: title, count, filters, letter bar, rows.
  const nav = main.getByRole("navigation", { name: "Jump to letter" });
  await expect(nav).toBeVisible();
  const top = async (locator: ReturnType<typeof main.locator>) => (await locator.first().boundingBox())!.y;
  const ys = [await top(title), await top(line), await top(main.locator(".lenses")), await top(nav), await top(main.locator(".letter-group"))];
  expect([...ys].sort((a, b) => a - b)).toEqual(ys);
  expect(new Set(ys).size).toBe(ys.length);
  expect(await nav.evaluate((el) => getComputedStyle(el).position)).toBe("sticky");

  // The ways out of the removed pieces still exist: the tab bar leads home, and the header
  // icon opens a search scoped to this list.
  await expect(page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Home", exact: true })).toBeVisible();
  await expect(page.locator("header.app-header").getByRole("button", { name: "Search", exact: true })).toBeVisible();
});

test("A short kind has a count line but no letter bar", async ({ page }) => {
  await installLibrary(page);
  const classes = corpus.catalog.entries.filter((stub) => stub.kind === "class");
  await page.goto("#/browse/kind/class");
  await expect(page.locator("main .list-count")).toHaveText(`${count(classes.length)} ${classes.length === 1 ? "class" : "classes"}`);
  await expect(page.locator("main nav.crumbs")).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "Jump to letter" })).toHaveCount(0);
});

test("Feat chains and the spell pages start their crumbs below a tab destination", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/feats/chains");
  await expect(page.locator("main .crumbs li")).toHaveText(["Feats"]);
  await page.goto("#/spells");
  await expect(page.locator("main .crumbs li")).toHaveText(["Spells"]);
  const crumbs = page.locator("main .crumbs");
  await expect(crumbs.getByRole("link", { name: "Home" })).toHaveCount(0);
});

// ---- Races: the printed group filter (#196) ----

const RACE_GROUP_LABELS: Record<string, string> = {
  core_race: "Core races (Advanced Race Guide)",
  featured_race: "Featured races",
  uncommon_race: "Uncommon races",
  other_race: "Other races",
};
const typedRaces = [...corpus.entries.values()].filter(
  (entry) => entry.kind === "race" && entry.id.startsWith("ref/race/") && activeIds.has(entry.id),
);
const raceGroupCounts = typedRaces.reduce((counts, entry) => {
  const group = entry.race_group;
  if (group) counts.set(group, (counts.get(group) ?? 0) + 1);
  return counts;
}, new Map<string, number>());
const raceCount = activeStubs.filter((stub) => stub.kind === "race").length;

test("The Races list shows every printed group with its count and filters in place", async ({ page }) => {
  // The pinned corpus carries all four printed race groups (#196). No skip: a corpus
  // without them must fail here.
  expect([...raceGroupCounts.keys()].sort()).toEqual([
    "core_race",
    "featured_race",
    "other_race",
    "uncommon_race",
  ]);
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/browse/kind/race");
  const main = page.locator("main");
  const lens = main.locator("section.lens").filter({
    has: page.getByRole("heading", { name: "By group" }),
  });
  await expect(lens).toBeVisible();
  const chips = lens.locator(".lens-chip");
  await expect(chips).toHaveCount(4);

  for (const [group, n] of raceGroupCounts) {
    const label = RACE_GROUP_LABELS[group];
    expect(label, `a label for ${group}`).toBeTruthy();
    const chip = chips.filter({ hasText: label });
    await expect(chip).toHaveCount(1);
    await expect(chip).toContainText(String(n));
    await expect(chip).toHaveAttribute("href", new RegExp(`facet=race_group:${group}`));
  }

  // A chip filters the list in place, and the count line compares the part with the whole.
  const core = raceGroupCounts.get("core_race")!;
  const coreChip = chips.filter({ hasText: RACE_GROUP_LABELS.core_race });
  await coreChip.click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/race?facet=race_group:core_race");
  await expect(main.locator(".list-count")).toHaveText(`${count(core)} of ${count(raceCount)} races · Show all`);
  await expect(main.locator(".entry-list a[href^='#/entry/']").first()).toBeVisible();
  await expect(chips.filter({ hasText: RACE_GROUP_LABELS.core_race })).toHaveAttribute("aria-current", "true");

  // Selection must be visible, including a check mark and readable count in both themes.
  for (const theme of ["light", "dark"]) {
    await page.evaluate((theme) => document.documentElement.setAttribute("data-theme", theme), theme);
    const selected = chips.filter({ hasText: RACE_GROUP_LABELS.core_race });
    const style = await selected.evaluate((el) => {
      const css = getComputedStyle(el);
      const check = getComputedStyle(el, "::before");
      const root = document.createElement("span");
      root.style.backgroundColor = "var(--accent)";
      el.append(root);
      const accent = getComputedStyle(root).backgroundColor;
      root.remove();
      return {
        background: css.backgroundColor,
        accent,
        color: css.color,
        countColor: getComputedStyle(el.querySelector("small")!).color,
        check: check.content,
        checkBorder: check.borderBottomWidth,
        height: el.getBoundingClientRect().height,
      };
    });
    expect(style.background).toBe(style.accent);
    expect(style.countColor).toBe(style.color);
    expect(style.check).not.toBe("none");
    expect(style.checkBorder).toBe("2px");
    expect(style.height).toBeGreaterThanOrEqual(44);
  }

  await main.getByRole("link", { name: "Show all races" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/race");
  await expect(main.locator(".list-count")).toHaveText(`${count(raceCount)} races`);
});

// ---- Feats: the filter menu, filtering in place, Feat chains and the chain marker (#193 PR 2) ----

const featsInCombat = [...corpus.entries.values()]
  .filter((entry) => activeIds.has(entry.id) && entry.kind === "feat" && (entry.fields?.feat_types ?? []).includes("Combat"))
  .length;
const featCount = activeStubs.filter((stub) => stub.kind === "feat").length;
const typeButton = (page: Page) => page.getByRole("button", { name: /^Type: /, exact: false });

test("The feat type menu opens, finds a type, filters the list in place, and Back returns to all types", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const main = page.locator("main");
  // The pill wall, its heading, "24 more" and the Feat chains card are gone.
  await expect(typeButton(page)).toHaveText("Type: All types");
  await expect(main.locator(".lens-chip, .lens-more, .lens-page, .lens-pages")).toHaveCount(0);
  await expect(main.getByText("By feat type")).toHaveCount(0);

  const button = typeButton(page);
  await expect(button).toBeEnabled();
  expect(await button.evaluate((el) => getComputedStyle(el).borderRadius)).toBe("6px");
  expect(Math.round((await button.boundingBox())!.height)).toBeGreaterThanOrEqual(44);
  await expect(button).toHaveAttribute("aria-expanded", "false");
  const panelId = (await button.getAttribute("aria-controls"))!;
  const panel = page.locator(`#${panelId}`);
  await expect(panel).toBeHidden();

  await button.click();
  await expect(button).toHaveAttribute("aria-expanded", "true");
  await expect(panel).toBeVisible();
  // Not a dialog, not fixed: it sits in the page flow and is at most half the viewport.
  expect(await panel.evaluate((el) => getComputedStyle(el).position)).toBe("static");
  expect(await panel.evaluate((el) => el.closest("dialog"))).toBeNull();
  const viewportHeight = page.viewportSize()!.height;
  expect((await panel.boundingBox())!.height).toBeLessThanOrEqual(viewportHeight * 0.5 + 1);
  // More than ten types: a find field, focused on open.
  const find = panel.getByRole("searchbox", { name: "Find a type" });
  await expect(find).toBeFocused();
  const list = panel.getByRole("list", { name: "Feat type" });
  await expect(list.getByRole("link", { name: /^All types/ })).toHaveAttribute("aria-current", "true");

  await find.fill("comb");
  await expect(panel.getByRole("status")).toHaveText(/^\d+ types?$/);
  const combat = list.getByRole("link", { name: `Combat, ${count(featsInCombat)}` });
  await expect(combat).toBeVisible();
  await expect(list.getByRole("link", { name: /^Achievement/ })).toHaveCount(0);
  await find.fill("zzzz");
  await expect(panel.getByRole("status")).toHaveText('No types match "zzzz".');
  await panel.getByRole("button", { name: "Clear" }).click();
  await find.fill("comb");

  const before = page.url();
  await combat.click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat?facet=feat_type:Combat");
  expect(page.url()).not.toBe(before);
  // Filtered in place: still the Feats page, A to Z, with the count of the part and the whole.
  await expect(main.getByRole("heading", { level: 1, name: "Feats" })).toBeVisible();
  const line = main.locator(".list-count");
  await expect(line).toHaveText(`${count(featsInCombat)} of ${count(featCount)} feats · Show all`);
  await expect(typeButton(page)).toHaveText("Type: Combat");
  await expect(typeButton(page)).toBeEnabled();
  // The selected value appears once, in the button.
  await expect(main.getByText("Combat", { exact: true }).filter({ visible: true })).toHaveCount(0);
  await expect(typeButton(page)).toBeFocused();
  await expect(main.locator(".letter-group .entry-list a").first()).toBeVisible();
  const heads = await main.locator(".letter-head").allInnerTexts();
  const total = heads.reduce((sum, head) => sum + num(head.match(/([\d,]+) entries/)?.[1]), 0);
  expect(total).toBe(featsInCombat);

  // A letter keeps the filter, and the filtered state is a history entry.
  await main.getByRole("link", { name: "Jump to C" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat?facet=feat_type:Combat&l=C");
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat?facet=feat_type:Combat");
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat");
  await expect(typeButton(page)).toHaveText("Type: All types");
  await expect(main.locator(".list-count")).toHaveText(`${count(featCount)} feats`);
  await page.goForward();
  await expect(typeButton(page)).toHaveText("Type: Combat");

  // Show all in the count line clears the filter.
  await main.getByRole("link", { name: "Show all feats" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat");
  await expect(main.locator(".list-count")).toHaveText(`${count(featCount)} feats`);
});

test("Escape closes the feat type menu and returns focus to its button without changing the route", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const button = typeButton(page);
  await expect(button).toBeEnabled();
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  await expect(panel.getByRole("searchbox")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(button).toHaveAttribute("aria-expanded", "false");
  await expect(button).toBeFocused();
  expect(new URL(page.url()).hash).toBe("#/browse/kind/feat");
  // Choosing the type that is already current closes the panel too.
  await page.goto("#/browse/kind/feat?facet=feat_type:combat");
  await expect(typeButton(page)).toHaveText("Type: Combat");
  await typeButton(page).click();
  await page.locator(".choice-row[aria-current='true']").click();
  await expect(panel).toBeHidden();
  await expect(typeButton(page)).toBeFocused();
});

test("A shared feat type link with another case still reads as that type, and an absent type shows an empty state", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat?facet=feat_type:COMBAT");
  const main = page.locator("main");
  await expect(typeButton(page)).toHaveText("Type: Combat");
  await expect(main.locator(".list-count")).toHaveText(new RegExp(`^${count(featsInCombat)} of ${count(featCount)} feats`));
  await page.goto("#/browse/kind/feat?facet=feat_type:Nonesuch");
  await expect(main.locator(".list-count")).toHaveText(`0 of ${count(featCount)} feats · Show all`);
  await expect(typeButton(page)).toHaveText("Type: All types");
  await expect(main.getByText("No Nonesuch feats in your active books.")).toBeVisible();
  // The way out is the page where the active set changes, not the read-only Books list.
  await expect(main.getByRole("link", { name: "Manage books", exact: true })).toHaveAttribute("href", "#/books/manage");
  await expect(main.getByRole("link", { name: "Choose books" })).toHaveCount(0);
  await main.locator("p.empty-actions").getByRole("link", { name: "Show all feats" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat");
});

test("Feat chains is a plain accent link in the filter row, not a card", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const link = page.locator("main .filter-row").getByRole("link", { name: "Feat chains" });
  await expect(link).toBeVisible();
  const style = await link.evaluate((el) => {
    const s = getComputedStyle(el);
    return { border: s.borderTopWidth, weight: s.fontWeight, color: s.color, accent: getComputedStyle(document.documentElement).getPropertyValue("--accent").trim(), svg: !!el.querySelector("svg[aria-hidden='true']") };
  });
  expect(style.border).toBe("0px");
  expect(Number(style.weight)).toBeGreaterThanOrEqual(600);
  expect(style.svg).toBe(true);
  expect(Math.round((await link.boundingBox())!.height)).toBeGreaterThanOrEqual(44);
  // The row is on the same line as the menu, so the link adds no height.
  const menu = (await typeButton(page).boundingBox())!;
  const box = (await link.boundingBox())!;
  expect(Math.abs(box.y + box.height / 2 - (menu.y + menu.height / 2))).toBeLessThan(8);
  await link.click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/feats/chains");
  await expect(page.getByRole("heading", { name: "Feat chains", level: 1 })).toBeVisible();
});

test("Feats that lead to more than five feats carry a chain marker, and it stays quiet", async ({ page }) => {
  await installLibrary(page);
  const leads = featLeadsTo(5, activeIds);
  const powerAttack = corpusEntry("feat", "Power Attack");
  const dodge = corpusEntry("feat", "Dodge");
  for (const feat of [powerAttack, dodge]) {
    const n = leads.get(feat.id);
    expect(n, `${feat.title} should lead to more than five feats`).toBeGreaterThan(5);
    await page.goto(`#/browse/kind/feat?l=${groupLetter(feat.title)}`);
    const row = page.locator(`.letter-group a[href="#/entry/${feat.id}"]`).first();
    await expect(row.locator("svg.chain-mark")).toHaveCount(1);
    expect(await row.locator("svg.chain-mark").getAttribute("aria-hidden")).toBe("true");
    await expect(row).toHaveAccessibleName(new RegExp(`^${feat.title} ?, leads to ${count(n!)} feats$`));
  }
  // A feat that leads nowhere has none, and its name is its title.
  const plain = corpusEntry("feat", "Aberrant Tumor");
  await page.goto("#/browse/kind/feat?l=A");
  await expect(page.locator(`.letter-group a[href="#/entry/${plain.id}"]`).first()).toHaveAccessibleName(plain.title);

  // Measure: across every letter group the markers match the relations, and they are few.
  const letters = await page.getByRole("navigation", { name: "Jump to letter" }).getByRole("link").evaluateAll((links) => links.map((link) => (link.textContent ?? "").trim()));
  let marked = 0;
  for (const letter of letters) {
    await page.goto(`#/browse/kind/feat?l=${encodeURIComponent(letter)}`);
    const expected = [...leads.keys()].filter((id) => groupLetter(corpus.entries.get(id)!.title) === letter).length;
    await expect(page.locator(".letter-group .chain-mark")).toHaveCount(expected);
    marked += expected;
  }
  console.log(`chain markers: ${marked} of ${featCount} feats (${((marked / featCount) * 100).toFixed(1)} %)`);
  expect(marked).toBe(leads.size);
  expect(marked / featCount).toBeLessThan(0.05);
});

// ---- The other kinds: items, traits, archetypes, spells and classes (#193 PR 3, #195) ----

const itemCount = corpus.catalog.entries.filter((stub) => stub.kind === "item").length;
const itemsIn = (category: string) => [...corpus.entries.values()].filter((entry) => entry.kind === "item" && entry.fields?.category === category).length;
const traitsIn = (category: string) => [...corpus.entries.values()].filter((entry) => entry.kind === "trait" && entry.fields?.category === category).length;
const categoryButton = (page: Page) => page.getByRole("button", { name: /^Category: / });

test("The item category menu finds one of many categories, filters in place, and Back and Escape behave", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/item");
  const main = page.locator("main");
  const rods = itemsIn("Rods");
  expect(rods).toBeGreaterThan(0);
  expect(itemCount).toBeGreaterThan(5000);

  // No pill wall, no "N more", no card; one menu.
  const button = categoryButton(page);
  await expect(button).toHaveText("Category: All categories");
  await expect(button).toBeEnabled();
  await expect(main.locator(".lens-chip, .lens-chips, .lens-more, .lens-page, .lens-pages")).toHaveCount(0);
  await expect(main.getByText(/By category|\d+ more/)).toHaveCount(0);
  await expect(main.locator(".list-count")).toHaveText(`${count(itemCount)} items`);
  await expect(main.getByRole("navigation", { name: "Jump to letter" })).toBeVisible();

  // Many categories: a find field, focused on open.
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  const find = panel.getByRole("searchbox", { name: "Find a category" });
  await expect(find).toBeFocused();
  expect(await panel.evaluate((el) => getComputedStyle(el).position)).toBe("static");
  const list = panel.getByRole("list", { name: "Item category" });
  expect(await list.getByRole("link").count()).toBeGreaterThan(40);
  await find.fill("zzzz");
  await expect(panel.getByRole("status")).toHaveText('No categories match "zzzz".');
  await panel.getByRole("button", { name: "Clear" }).click();
  await find.fill("rods");
  await expect(panel.getByRole("status")).toHaveText("1 category");

  // Escape closes the panel, keeps the route, and returns focus to the button.
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(button).toBeFocused();
  expect(new URL(page.url()).hash).toBe("#/browse/kind/item");
  await button.click();
  await panel.getByRole("searchbox").fill("rods");
  await list.getByRole("link", { name: `Rods, ${count(rods)}` }).click();

  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/item?facet=item_category:Rods");
  await expect(main.getByRole("heading", { level: 1, name: "Items" })).toBeVisible();
  const line = main.locator(".list-count");
  await expect(line).toHaveText(`${count(rods)} of ${count(itemCount)} items · Show all`);
  await expect(line).not.toContainText("Rods");
  await expect(categoryButton(page)).toHaveText("Category: Rods");
  await expect(categoryButton(page)).toBeFocused();
  await expect(main.locator(".letter-group .entry-list a").first()).toBeVisible();
  const heads = await main.locator(".letter-head").allInnerTexts();
  expect(heads.reduce((sum, head) => sum + num(head.match(/([\d,]+) (?:entries|entry)/)?.[1]), 0)).toBe(rods);

  // Back returns to all items, Forward to the category, and Show all clears it.
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/item");
  await expect(categoryButton(page)).toHaveText("Category: All categories");
  await expect(line).toHaveText(`${count(itemCount)} items`);
  await page.goForward();
  await expect(categoryButton(page)).toHaveText("Category: Rods");
  await main.getByRole("link", { name: "Show all items" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/item");

  // The header search keeps the category, without saying "items" twice.
  await page.goto("#/browse/kind/item?facet=item_category:Rods");
  await page.locator("header.app-header").getByRole("button", { name: "Search", exact: true }).click();
  await expect(page.getByRole("button", { name: "Remove scope: Rods", exact: true })).toBeVisible();
});

test("An item category that no active book holds shows the empty state with the way out", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/item?facet=item_category:Nonesuch");
  const main = page.locator("main");
  await expect(main.locator(".list-count")).toHaveText(`0 of ${count(itemCount)} items · Show all`);
  await expect(categoryButton(page)).toHaveText("Category: All categories");
  await expect(main.getByText("No Nonesuch in your active books.")).toBeVisible();
  // The way out is the page where the active set changes, not the read-only Books list.
  await expect(main.getByRole("link", { name: "Manage books", exact: true })).toHaveAttribute("href", "#/books/manage");
  await expect(main.getByRole("link", { name: "Choose books" })).toHaveCount(0);
  await main.locator("p.empty-actions").getByRole("link", { name: "Show all items" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/item");
});

test("Traits, archetypes and spells each have one filter menu in the filter row and no pill wall", async ({ page }) => {
  await installLibrary(page);
  const main = page.locator("main");
  const combat = traitsIn("Combat");
  const traitCount = corpus.catalog.entries.filter((stub) => stub.kind === "trait").length;
  expect(combat).toBeGreaterThan(0);

  await page.goto("#/browse/kind/trait");
  await expect(categoryButton(page)).toHaveText("Category: All categories");
  await expect(main.locator(".lens-chip, .lens-more, .lens-page, .lens-pages")).toHaveCount(0);
  await categoryButton(page).click();
  const traitPanel = page.locator(`#${await categoryButton(page).getAttribute("aria-controls")}`);
  await traitPanel.getByRole("list", { name: "Trait category" }).getByRole("link", { name: `Combat, ${count(combat)}` }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/trait?facet=trait_category:Combat");
  await expect(main.locator(".list-count")).toHaveText(`${count(combat)} of ${count(traitCount)} traits · Show all`);
  await expect(categoryButton(page)).toHaveText("Category: Combat");

  await page.goto("#/browse/kind/archetype");
  const classButton = main.getByRole("button", { name: /^Class: / });
  await expect(classButton).toHaveText("Class: All classes");
  await expect(main.locator(".lens-chip, .lens-more, .lens-page, .lens-pages")).toHaveCount(0);
  await classButton.click();
  const archetypePanel = page.locator(`#${await classButton.getAttribute("aria-controls")}`);
  await expect(archetypePanel.getByRole("searchbox", { name: "Find a class" })).toBeFocused();
  await archetypePanel.getByRole("searchbox").fill("wizard");
  await archetypePanel.getByRole("list", { name: "Archetype class" }).getByRole("link", { name: /^Wizard, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toMatch(/^#\/browse\/kind\/archetype\?facet=archetype_class:wizard/i);
  await expect(main.locator(".list-count")).toHaveText(/^[\d,]+ of [\d,]+ archetypes · Show all$/);
  await expect(main.getByRole("button", { name: "Class: Wizard" })).toBeVisible();

  // Spells: the school menu, and the spell lists as a plain link beside it, not a card.
  await page.goto("#/browse/kind/spell");
  const school = main.getByRole("button", { name: /^School: / });
  await expect(school).toHaveText("School: All schools");
  await expect(main.locator(".lens-chip, .lens-more, .lens-page, .lens-pages")).toHaveCount(0);
  const lists = main.locator(".filter-row").getByRole("link", { name: "Spell lists" });
  await expect(lists).toBeVisible();
  await school.click();
  const schoolPanel = page.locator(`#${await school.getAttribute("aria-controls")}`);
  await schoolPanel.getByRole("list", { name: "Spell school" }).getByRole("link", { name: /^Evocation, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/spell?facet=school:evocation");
  await expect(main.getByRole("button", { name: "School: Evocation" })).toBeFocused();
  await expect(main.locator(".list-count")).toHaveText(/^[\d,]+ of [\d,]+ spells · Show all$/);
  await main.locator(".filter-row").getByRole("link", { name: "Spell lists" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells");
});

test("Classes are listed under their data group headings, without pills or a menu", async ({ page }) => {
  await installLibrary(page);
  const classes = [...corpus.entries.values()].filter((entry) => entry.kind === "class");
  // Derive the groups from the built data, so a new class_type does not break the test.
  const types = new Set(
    classes
      .map((entry) => entry.fields?.class_type)
      .filter((type): type is string => Boolean(type)),
  );
  const ungrouped = classes.some((entry) => !entry.fields?.class_type);
  const groupCount = types.size + (ungrouped ? 1 : 0);

  await page.goto("#/browse/kind/class");
  const main = page.locator("main");
  await expect(main.locator(".list-count")).toHaveText(`${count(classes.length)} classes`);
  await expect(main.locator("section.letter-group")).toHaveCount(groupCount);
  await expect(main.locator(".lens-chip, .lens-more, .lens-page, .lens-pages, .filter-row, .filter-btn")).toHaveCount(0);
  await expect(main.getByText("Core, base, and NPC")).toHaveCount(0);
  await expect(main.getByRole("navigation", { name: "Jump to letter" })).toHaveCount(0);
  const heads = await main.locator("h2.letter-head").allInnerTexts();
  const labels = heads.map((head) => head.replace(/\s*[\d,]+$/, "").trim());
  expect(new Set(labels).size).toBe(labels.length);
  expect(labels.every((label) => label.length > 0)).toBe(true);
  const groupCounts = heads.map((head) => num(head.match(/([\d,]+)$/)?.[1]));
  expect(groupCounts.reduce((sum, value) => sum + value, 0)).toBe(classes.length);
  // Every class is under its heading, and a heading names its list.
  const seen: string[] = [];
  for (const [index] of labels.entries()) {
    const section = main.locator("section.letter-group").nth(index);
    await expect(section).toHaveAttribute("aria-labelledby", /.+/);
    const titles = (await section.locator(".entry-list a").allInnerTexts()).map((text) => text.trim());
    expect(titles).toHaveLength(groupCounts[index]);
    seen.push(...titles);
  }
  expect(seen.sort()).toEqual(classes.map((entry) => entry.title).sort());

  // A class opens, and Back returns to the grouped list.
  const first = main.locator("section.letter-group").first().locator(".entry-list a").first();
  const title = (await first.innerText()).trim();
  await first.click();
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/class");
  await expect(main.locator("section.letter-group")).toHaveCount(groupCount);

  // An old link with a group in it shows the same grouped list instead of a broken page.
  await page.goto("#/browse/kind/class?facet=class_type:core");
  await expect(main.locator("section.letter-group")).toHaveCount(groupCount);
  await expect(main.locator(".list-count")).toHaveText(`${count(classes.length)} classes`);
});

// ---- Book-filtered kind lists (#158) --------------------------------------------------------

const stubById = new Map(corpus.catalog.entries.map((stub: any) => [stub.id, stub]));
/** True when the entry belongs to the book: the book is one of its `source_ids` (#394). */
const inBookOf = (id: string, source: string) => isMember(stubById.get(id)!, source);
const inBook = (source: string, kind: string) =>
  corpus.catalog.entries.filter((stub: any) => stub.kind === kind && isMember(stub, source)).length;
const inBookTotal = (source: string) =>
  corpus.catalog.entries.filter((stub: any) => isMember(stub, source)).length;
const bookPill = (page: Page, title: string) =>
  page.getByRole("button", { name: `Remove filter: ${title}`, exact: true });

test("A book-filtered kind list keeps the Home tab, names the book in a pill, and counts the book", async ({ page }) => {
  await installLibrary(page);
  const core = corpus.catalog.sources.find((source: any) => source.id === "core-rulebook")!;
  const spells = inBook(core.id, "spell");
  expect(spells).toBeGreaterThan(0);

  await page.goto(`#/browse/kind/spell?source=${core.id}`);
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Spells" })).toBeVisible();
  await expect(bookPill(page, core.title)).toBeVisible();
  await expect(bookPill(page, core.title)).toHaveClass(/scope-pill/);
  // The line counts the book and never repeats its name.
  const line = main.locator(".list-count");
  await expect(line).toHaveText(`${count(spells)} spells`);
  await expect(line).not.toContainText(core.title);
  await expect(main.locator(".book-filter")).toHaveCount(1);

  // The tab stays Home, and the rows are the book's spells.
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Home");
  await expect(main.locator('.entry-list a[href^="#/entry/"]').first()).toBeVisible();
});

test("Removing the book pill keeps the letter and the facet, and moves focus to the first filter control", async ({ page }) => {
  await installLibrary(page);
  const core = corpus.catalog.sources.find((source: any) => source.id === "core-rulebook")!;
  const bookSpells = inBook(core.id, "spell");
  const evocation = [...corpus.entries.values()].filter(
    (entry: any) =>
      entry.kind === "spell" &&
      inBookOf(entry.id, core.id) &&
      // The index drops a bracketed qualifier, as the old `school:conjuration (calling)` links do.
      (entry.fields?.school ?? "").toLowerCase().split(/[[(]/)[0].trim() === "evocation",
  );
  expect(evocation.length).toBeGreaterThan(0);
  const letter = groupLetter([...evocation].sort((a, b) => a.title.localeCompare(b.title))[0].title);
  const hash = `#/browse/kind/spell?source=${core.id}&facet=school:evocation&l=${encodeURIComponent(letter)}`;
  await page.goto(hash);

  const main = page.locator("main");
  await expect(main.locator(".list-count")).toHaveText(
    `${count(evocation.length)} of ${count(bookSpells)} spells · Show all`,
  );
  await bookPill(page, core.title).click();
  // Only the book filter goes: the letter and the facet stay.
  await expect.poll(() => new URL(page.url()).hash).toBe(
    `#/browse/kind/spell?facet=school:evocation&l=${encodeURIComponent(letter)}`,
  );
  await expect(bookPill(page, core.title)).toHaveCount(0);
  await expect(main.getByRole("button", { name: "School: Evocation" })).toBeFocused();
});

test("A book with none of a kind says so and offers the whole list", async ({ page }) => {
  await installLibrary(page);
  const empty = corpus.catalog.sources.find(
    (source: any) => inBookTotal(source.id) > 0 && inBook(source.id, "spell") === 0,
  )!;
  expect(empty, "a book with entries but no spells").toBeTruthy();
  await page.goto(`#/browse/kind/spell?source=${empty.id}`);
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Spells" })).toBeVisible();
  await expect(bookPill(page, empty.title)).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText("0 spells");
  await expect(main.getByText(`No spells in ${empty.title}.`)).toBeVisible();
  const browse = main.getByRole("link", { name: "Browse all spells" });
  await expect(browse).toHaveAttribute("href", "#/browse/kind/spell");
  await browse.click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/spell");
  await expect(main.locator(".book-filter")).toHaveCount(0);
  await expect(main.locator(".list-count")).toHaveText(
    `${count(corpus.catalog.entries.filter((stub: any) => stub.kind === "spell").length)} spells`,
  );
});

test("An unknown book shows the whole kind list with a notice, not a missing page", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/spell?source=no-such-book");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Spells" })).toBeVisible();
  await expect(main.getByText("That book isn't in your library, so this list shows all spells.")).toBeVisible();
  await expect(main.locator(".book-filter, .scope-pill")).toHaveCount(0);
  await expect(main.locator(".list-count")).toHaveText(
    `${count(corpus.catalog.entries.filter((stub: any) => stub.kind === "spell").length)} spells`,
  );
  await expect(page.getByRole("heading", { name: "Page not found" })).toHaveCount(0);
  await expect(page.locator('a.nav-link[aria-current="page"]')).toHaveText("Home");
});

// ---- Creatures: type and challenge rating menus (#193 PR 4) ----

const creatures = [...corpus.entries.values()].filter((entry) => entry.kind === "creature");
const creatureCount = corpus.catalog.entries.filter((stub) => stub.kind === "creature").length;
const crOf = (entry: { fields?: Record<string, any> }): string | null => {
  const twentyFourths = entry.fields?.cr?.twenty_fourths;
  if (typeof twentyFourths !== "number") return null;
  return ({ 3: "1/8", 4: "1/6", 6: "1/4", 8: "1/3", 12: "1/2" } as Record<number, string>)[twentyFourths] ?? String(twentyFourths / 24);
};
const dragons = creatures.filter((entry) => entry.fields?.creature_type === "dragon");
const creaturesAtCr10 = creatures.filter((entry) => crOf(entry) === "10");
const dragonsAtCr10 = dragons.filter((entry) => crOf(entry) === "10");
const creatureTypeButton = (page: Page) => page.locator("main").getByRole("button", { name: /^Type: / });
const creatureCrButton = (page: Page) => page.locator("main").getByRole("button", { name: /^CR: / });

test("The creature type menu filters to Dragon, the CR menu narrows it further, and Back steps through both", async ({ page }) => {
  await installLibrary(page);
  expect(creatureCount).toBeGreaterThan(1000);
  expect(dragons.length).toBeGreaterThan(0);
  expect(dragonsAtCr10.length).toBeGreaterThan(0);
  await page.goto("#/browse/kind/creature");
  const main = page.locator("main");

  // The old pill lens is gone: the filter row holds the two menus.
  const typeButton = creatureTypeButton(page);
  const crButton = creatureCrButton(page);
  await expect(typeButton).toHaveText("Type: All types");
  await expect(crButton).toHaveText("CR: All CRs");
  await expect(typeButton).toBeEnabled();
  await expect(crButton).toBeEnabled();
  await expect(main.locator(".lens-chip, .lens-chips, .lens-more, .lens-page, .lens-pages")).toHaveCount(0);
  await expect(main.getByText("By type")).toHaveCount(0);
  await expect(main.locator(".list-count")).toHaveText(`${count(creatureCount)} creatures`);

  // The type menu has a find field (13 types) and every label reads as a title, not the index value.
  await typeButton.click();
  const typePanel = page.locator(`#${await typeButton.getAttribute("aria-controls")}`);
  const find = typePanel.getByRole("searchbox", { name: "Find a type" });
  await expect(find).toBeFocused();
  const typeList = typePanel.getByRole("list", { name: "Creature type" });
  const typeLabels = (await typeList.locator(".choice-label").allInnerTexts()).filter((label) => label !== "All types");
  expect(typeLabels.length).toBe(13);
  expect(typeLabels.every((label) => /^[A-Z]/.test(label))).toBe(true);
  expect(typeLabels).toEqual(expect.arrayContaining(["Aberration", "Dragon", "Magical Beast", "Monstrous Humanoid", "Ooze"]));
  await expect(typePanel.getByRole("status")).toHaveText("13 types");
  await find.fill("dragon");
  await typeList.getByRole("link", { name: `Dragon, ${count(dragons.length)}` }).click();

  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/creature?facet=creature_type:dragon");
  await expect(typeButton).toHaveText("Type: Dragon");
  await expect(crButton).toHaveText("CR: All CRs");
  await expect(main.locator(".list-count")).toHaveText(`${count(dragons.length)} of ${count(creatureCount)} creatures · Show all`);
  await expect(typeButton).toBeFocused();

  // The CR menu lists the ratings numerically, with a find field, and narrows the same list.
  await crButton.click();
  const crPanel = page.locator(`#${await crButton.getAttribute("aria-controls")}`);
  const crList = crPanel.getByRole("list", { name: "Challenge rating" });
  await expect(crPanel.getByRole("searchbox", { name: "Find a CR" })).toBeFocused();
  const crLabels = (await crList.locator(".choice-label").allInnerTexts()).filter((label) => label !== "All CRs");
  expect(crLabels.slice(0, 6)).toEqual(["1/8", "1/6", "1/4", "1/3", "1/2", "1"]);
  expect(crLabels.at(-1)).toBe("30");
  await expect(crPanel.getByRole("status")).toHaveText("35 ratings");
  await crPanel.getByRole("searchbox").fill("10");
  // A menu counts its whole group, not the other menu's selection: CR 10 is the whole library's.
  await crList.getByRole("link", { name: `10, ${count(creaturesAtCr10.length)}` }).click();

  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/creature?facet=creature_type:dragon&facet=cr:10");
  await expect(typeButton).toHaveText("Type: Dragon");
  await expect(crButton).toHaveText("CR: 10");
  await expect(main.locator(".list-count")).toHaveText(`${count(dragonsAtCr10.length)} of ${count(creatureCount)} creatures · Show all`);
  await expect(crButton).toBeFocused();
  await expect(main.locator(".entry-list a").first()).toBeVisible();

  // Back undoes one menu at a time, and Forward replays them.
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/creature?facet=creature_type:dragon");
  await expect(crButton).toHaveText("CR: All CRs");
  await expect(main.locator(".list-count")).toHaveText(`${count(dragons.length)} of ${count(creatureCount)} creatures · Show all`);
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/creature");
  await expect(typeButton).toHaveText("Type: All types");
  await expect(main.locator(".list-count")).toHaveText(`${count(creatureCount)} creatures`);
  await page.goForward();
  await expect(typeButton).toHaveText("Type: Dragon");
  await page.goForward();
  await expect(crButton).toHaveText("CR: 10");

  // Show all clears both fields at once.
  await main.getByRole("link", { name: "Show all creatures" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/creature");
  await expect(typeButton).toHaveText("Type: All types");
  await expect(crButton).toHaveText("CR: All CRs");
});

test("Escape closes the creature CR menu and returns focus to its button without changing the route", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/creature");
  const button = creatureCrButton(page);
  await expect(button).toBeEnabled();
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  await expect(panel.getByRole("searchbox", { name: "Find a CR" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(button).toHaveAttribute("aria-expanded", "false");
  await expect(button).toBeFocused();
  expect(new URL(page.url()).hash).toBe("#/browse/kind/creature");
});

test("A creature combination no active book holds shows the empty state and both values stay in their buttons", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/creature?facet=creature_type:dragon&facet=cr:1/8");
  const main = page.locator("main");
  await expect(main.locator(".list-count")).toHaveText(`0 of ${count(creatureCount)} creatures · Show all`);
  await expect(creatureTypeButton(page)).toHaveText("Type: Dragon");
  await expect(creatureCrButton(page)).toHaveText("CR: 1/8");
  await expect(main.getByText("No CR 1/8 Dragon creatures in your active books.")).toBeVisible();
  await expect(main.getByRole("link", { name: "Manage books", exact: true })).toHaveAttribute("href", "#/books/manage");
  await main.locator("p.empty-actions").getByRole("link", { name: "Show all creatures" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/creature");
  await expect(main.locator(".list-count")).toHaveText(`${count(creatureCount)} creatures`);
});

test("Removing the book pill on a class list focuses the H1", async ({ page }) => {
  await installLibrary(page);
  const core = corpus.catalog.sources.find((source: any) => source.id === "core-rulebook")!;
  await page.goto(`#/browse/kind/class?source=${core.id}`);
  const heading = page.getByRole("heading", { level: 1, name: "Classes" });
  await expect(heading).toBeVisible();
  await bookPill(page, core.title).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/class");
  await expect(bookPill(page, core.title)).toHaveCount(0);
  await expect(heading).toBeFocused();
});

test("A book-filtered creature list keeps the book when the Type menu narrows it", async ({ page }) => {
  await installLibrary(page);
  const bestiary = "bestiary";
  const bookCreatures = inBook(bestiary, "creature");
  const bookDragons = [...corpus.entries.values()].filter(
    (entry: any) =>
      entry.kind === "creature" &&
      inBookOf(entry.id, bestiary) &&
      entry.fields?.creature_type === "dragon",
  ).length;
  expect(bookCreatures).toBeGreaterThan(0);
  expect(bookDragons).toBeGreaterThan(0);

  await page.goto(`#/browse/kind/creature?source=${bestiary}`);
  const main = page.locator("main");
  await expect(main.locator(".list-count")).toHaveText(`${count(bookCreatures)} creatures`);
  await expect(bookPill(page, corpus.catalog.sources.find((source: any) => source.id === bestiary)!.title)).toBeVisible();

  const typeButton = main.getByRole("button", { name: /^Type: / });
  await typeButton.click();
  const panel = page.locator(`#${await typeButton.getAttribute("aria-controls")}`);
  await panel.getByRole("link", { name: /^Dragon, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe(
    `#/browse/kind/creature?source=${bestiary}&facet=creature_type:dragon`,
  );
  await expect(bookPill(page, corpus.catalog.sources.find((source: any) => source.id === bestiary)!.title)).toBeVisible();
  await expect(typeButton).toHaveText("Type: Dragon");
  await expect(main.locator(".list-count")).toHaveText(
    `${count(bookDragons)} of ${count(bookCreatures)} creatures · Show all`,
  );
  await expect(main.locator('.entry-list a[href^="#/entry/"]').first()).toBeVisible();
});

test("An inactive book's list still shows its entries with the pill", async ({ page }) => {
  await installLibrary(page);
  const core = corpus.catalog.sources.find((source: any) => source.id === "core-rulebook")!;
  const spells = inBook(core.id, "spell");
  await page.goto("#/books/manage");
  await openBookGroups(page);
  const active = page.locator(`.book-directory-item[data-book-id="${core.id}"]`)
    .getByRole("checkbox", { name: `Active ${core.title}`, exact: true });
  await active.uncheck();
  await expect(active).not.toBeChecked();

  await page.goto(`#/browse/kind/spell?source=${core.id}`);
  const main = page.locator("main");
  await expect(bookPill(page, core.title)).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText(`${count(spells)} spells`);
  await expect(main.locator('.entry-list a[href^="#/entry/"]').first()).toBeVisible();
});

// ---- Spell lists: the hub, a class list, and the school multi-select (#193 PR 5) ----

/** The active spells of a class (any case) at a level, with the school word the index holds. */
function wizardSpells(level: number, klass = "wizard") {
  return [...corpus.entries.values()]
    .filter((entry: any) => entry.kind === "spell" && activeIds.has(entry.id))
    .filter((entry: any) => entry.fields?.levels?.some((l: any) => l.class.toLowerCase() === klass && l.level === level))
    .map((entry: any) => ({
      title: entry.title as string,
      school: ((entry.fields?.school ?? "") as string).toLowerCase().split(/[[(]/)[0].trim(),
    }));
}

test("The spell hub is a list page: a count of spell lists, the class picker, and no school pill wall", async ({ page }) => {
  await installLibrary(page);
  const classes = new Set<string>();
  for (const entry of corpus.entries.values() as Iterable<any>) {
    if (entry.kind !== "spell" || !activeIds.has(entry.id)) continue;
    for (const l of entry.fields?.levels ?? []) classes.add(l.class);
  }
  await page.goto("#/spells");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Spell lists" })).toBeVisible();
  const line = main.locator(".list-count");
  await expect(line).toHaveText(`${count(classes.size)} spell lists`);
  await expect(line).toHaveAttribute("aria-live", "polite");
  await expect(main.locator("nav.crumbs li")).toHaveText(["Spells"]);
  await expect(main.locator(".class-picker").first().getByRole("link", { name: /^Wizard spells,/ })).toBeVisible();
  // The wall of school pills that linked into search, and the extra link back, are gone.
  await expect(main.locator(".lens-chip, .lens-chips, .lens")).toHaveCount(0);
  await expect(main.getByText("By school")).toHaveCount(0);
  await expect(main.getByRole("link", { name: "All spells, A to Z" })).toHaveCount(0);

  // The A to Z list of every spell is one tap away: the `Spells` crumb.
  await main.locator("nav.crumbs").getByRole("link", { name: "Spells", exact: true }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/spell");
  await page.goBack();
  await expect(main.getByRole("heading", { level: 1, name: "Spell lists" })).toBeVisible();

  // The header search on the hub searches spells, and says so.
  await openQuickSearch(page);
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search spells");
  await expect(page.getByRole("button", { name: "Remove scope: Spells", exact: true })).toBeVisible();
});

test("A class list opens with the list-page head: title, count line, then the filter row with the school menu and the class page", async ({ page }) => {
  await installLibrary(page);
  const here = wizardSpells(3);
  expect(here.length).toBeGreaterThan(20);
  await page.goto("#/spells/Wizard/3");
  const main = page.locator("main");
  const title = main.getByRole("heading", { level: 1, name: "Wizard spells, level 3" });
  await expect(title).toBeVisible();
  const line = main.locator(".list-count");
  await expect(line).toHaveText(`${count(here.length)} spells`);
  const row = main.locator(".filter-row");
  const school = row.getByRole("button", { name: "School: All schools", exact: true });
  await expect(school).toBeVisible();
  expect((await school.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  const classPage = row.getByRole("link", { name: "Wizard class page" });
  await expect(classPage).toHaveClass(/page-link/);
  // The name field has the block gap under the filter row, not a shared edge (12 px at least).
  await expect(main.locator(".list-filter")).toBeVisible();
  const gap = await page.evaluate(() => {
    const row = document.querySelector("main .filter-row")!.getBoundingClientRect();
    const field = document.querySelector("main .list-filter")!.getBoundingClientRect();
    return field.top - row.bottom;
  });
  expect(gap).toBeGreaterThanOrEqual(11.5);
  // The old disclosure of pills and the bordered class rectangle are gone.
  await expect(main.locator("details.school-filter, .class-page, .chip-count")).toHaveCount(0);
  // The order from the top: title, count line, filter row, level bar, rows.
  await expect(main.locator(".level-nav")).toBeVisible();
  await expect(main.locator(".spell-rows").first()).toBeVisible();
  const tops = await page.evaluate(() => {
    const top = (selector: string) => document.querySelector(selector)!.getBoundingClientRect().top;
    return [top("main h1"), top("main .list-count"), top("main .filter-row"), top("main .level-nav"), top("main .spell-rows")];
  });
  expect([...tops].sort((a, b) => a - b)).toEqual(tops);
  // Cantrips name themselves.
  await page.goto("#/spells/Wizard/0");
  await expect(main.locator(".list-count")).toHaveText(`${count(wizardSpells(0).length)} cantrips`);
});

test("The school menu on a list is a multi-select: the route holds the schools, the list narrows in place, and Back walks the choices", async ({ page }) => {
  await installLibrary(page);
  const here = wizardSpells(3);
  const of = (...schools: string[]) => here.filter((spell) => schools.includes(spell.school)).length;
  expect(of("evocation")).toBeGreaterThan(0);
  expect(of("conjuration")).toBeGreaterThan(0);
  const main = page.locator("main");
  await page.goto("#/spells/Wizard/3");
  const button = (name: string) => main.getByRole("button", { name, exact: true });
  await button("School: All schools").click();
  const panel = page.locator(`#${await button("School: All schools").getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  // Opening focuses the All row while nothing is chosen. The box never covers the page.
  await expect(panel.getByRole("link", { name: "All schools" })).toBeFocused();
  expect((await panel.boundingBox())!.height).toBeLessThanOrEqual(page.viewportSize()!.height * 0.5 + 1);
  // A marker that a rebuild would clear proves a choice does not rebuild the page.
  await page.evaluate(() => ((window as any).__kept = "yes"));

  const evocation = panel.getByRole("checkbox", { name: /^Evocation, / });
  await evocation.check();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/3?school=evocation");
  await expect(button("School: Evocation")).toHaveClass(/filter-btn-on/);
  await expect(main.locator(".list-count")).toHaveText(`${count(of("evocation"))} of ${count(here.length)} spells · Show all`);
  // The panel stays open and the tick stays under the reader's finger.
  await expect(panel).toBeVisible();
  await expect(evocation).toBeChecked();
  await expect(evocation).toBeFocused();
  const shown = await main.locator(".spell-school").allInnerTexts();
  expect(shown).toHaveLength(of("evocation"));
  expect(new Set(shown)).toEqual(new Set(["Evocation"]));

  const conjuration = panel.getByRole("checkbox", { name: /^Conjuration, / });
  await conjuration.check();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/3?school=evocation&school=conjuration");
  await expect(button("School: 2 chosen")).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText(`${count(of("evocation", "conjuration"))} of ${count(here.length)} spells · Show all`);
  expect(new Set(await main.locator(".spell-school").allInnerTexts())).toEqual(new Set(["Evocation", "Conjuration"]));
  expect(await page.evaluate(() => (window as any).__kept)).toBe("yes");

  // Back undoes the last choice, then the first; Forward replays it.
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/3?school=evocation");
  await expect(conjuration).not.toBeChecked();
  await expect(evocation).toBeChecked();
  await expect(button("School: Evocation")).toBeVisible();
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/3");
  await expect(button("School: All schools")).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText(`${count(here.length)} spells`);
  await page.goForward();
  await expect(button("School: Evocation")).toBeVisible();
  expect(await page.evaluate(() => (window as any).__kept)).toBe("yes");

  // A shared link opens the same state, closed; the level links keep the schools; Show all clears them.
  await page.goto("#/spells/Wizard/3?school=evocation&school=conjuration");
  await page.reload();
  await expect(button("School: 2 chosen")).toHaveAttribute("aria-expanded", "false");
  await expect(main.locator(".list-count")).toHaveText(`${count(of("evocation", "conjuration"))} of ${count(here.length)} spells · Show all`);
  await main.locator(".level-nav").getByRole("link", { name: /^Level 4, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/4?school=evocation&school=conjuration");
  await expect(button("School: 2 chosen")).toBeVisible();
  await main.getByRole("link", { name: "Show all spells" }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/4");
  await expect(button("School: All schools")).toBeVisible();

  // Escape closes the panel and returns focus to the button.
  await button("School: All schools").click();
  await page.keyboard.press("Escape");
  await expect(button("School: All schools")).toBeFocused();
  await expect(button("School: All schools")).toHaveAttribute("aria-expanded", "false");
});

test("Quick search on a spell list names the class and the chosen schools, and one tap removes the scope", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/spells/Wizard/3");
  await openQuickSearch(page);
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search Wizard spells");
  await page.getByRole("button", { name: "Remove scope: Wizard spells", exact: true }).click();
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search spells, feats, rules…");
  await page.keyboard.press("Escape");

  await page.goto("#/spells/Wizard/3?school=evocation&school=conjuration");
  await openQuickSearch(page);
  await expect(quickField(page)).toHaveAttribute("placeholder", "Search Evocation or Conjuration Wizard spells");
  const pill = page.getByRole("button", { name: "Remove scope: Evocation or Conjuration Wizard spells", exact: true });
  await expect(pill).toBeVisible();
  // Enter opens the results with the class and both schools applied; every hit is a spell.
  await quickField(page).fill("fire");
  await page.keyboard.press("Enter");
  await waitResults(page, "fire");
  const hash = new URL(page.url()).hash;
  expect(hash).toContain("kind=spell");
  expect(hash).toContain("facet=school:evocation");
  expect(hash).toContain("facet=school:conjuration");
  expect(hash.toLowerCase()).toContain("facet=class:wizard");
  for (const meta of await page.locator(".results .hit-meta").allInnerTexts()) expect(meta).toMatch(/^Spell/);
  await expect(page.getByRole("button", { name: "Remove scope: Evocation or Conjuration Wizard spells", exact: true })).toBeVisible();
});

test("In the school menu All schools only closes the panel while nothing is chosen, and clears the choices otherwise", async ({ page }) => {
  await installLibrary(page);
  const main = page.locator("main");
  await page.goto("#/spells/Wizard/3");
  const button = main.getByRole("button", { name: "School: All schools", exact: true });
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  const all = panel.getByRole("link", { name: "All schools" });
  // The current row shows its state with the same box as the school rows, ticked.
  await expect(all).toHaveAttribute("aria-current", "true");
  await expect(all.locator(".choice-box .choice-check")).toBeVisible();
  const history = await page.evaluate(() => window.history.length);
  await all.click();
  await expect(panel).toBeHidden();
  await expect(button).toBeFocused();
  await expect(button).toHaveAttribute("aria-expanded", "false");
  // Nothing navigated: no new history entry, the same address.
  expect(await page.evaluate(() => window.history.length)).toBe(history);
  expect(new URL(page.url()).hash).toBe("#/spells/Wizard/3");

  // With a school chosen the row is not current, its box is empty, and it clears the choices.
  await button.click();
  await panel.getByRole("checkbox", { name: /^Evocation, / }).check();
  await expect(all).not.toHaveAttribute("aria-current", "true");
  await expect(all.locator(".choice-check")).toHaveCount(0);
  await all.click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/3");
  await expect(main.getByRole("button", { name: "School: All schools", exact: true })).toBeVisible();
  await expect(panel.getByRole("checkbox", { name: /^Evocation, / })).not.toBeChecked();
  await expect(all).toHaveAttribute("aria-current", "true");
});

test("A school checkbox is named with its label and its spell count", async ({ page }) => {
  await installLibrary(page);
  const main = page.locator("main");
  await page.goto("#/spells/Wizard/3");
  await main.getByRole("button", { name: "School: All schools", exact: true }).click();
  const evocation = main.getByRole("checkbox", { name: /^Evocation, / });
  const n = wizardSpells(3).filter((spell) => spell.school === "evocation").length;
  await expect(evocation).toHaveAccessibleName(`Evocation, ${count(n)} ${n === 1 ? "spell" : "spells"}`);
});

test("A level heading in the whole-list view keeps the chosen schools", async ({ page }) => {
  await installLibrary(page);
  const main = page.locator("main");
  // The whole-list view: a level heading is a link to that level, and keeps the schools.
  await page.goto("#/spells/Wizard?school=evocation&school=conjuration");
  const heading = main.locator(".level-head a", { hasText: "Level 3" });
  await expect(heading).toBeVisible();
  expect(await heading.getAttribute("href")).toBe("#/spells/Wizard/3?school=evocation&school=conjuration");
  await heading.click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/spells/Wizard/3?school=evocation&school=conjuration");
  await expect(main.getByRole("button", { name: "School: 2 chosen", exact: true })).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText(/^[\d,]+ of [\d,]+ spells · Show all$/);
});
