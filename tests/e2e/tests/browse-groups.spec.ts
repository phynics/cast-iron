import type { Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { corpusEntry, count, featLeadsTo, groupLetter, installLibrary, letterId, num, waitControlled } from "./helpers";

const corpus = loadCorpus();

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
  const feats = corpus.catalog.entries.filter((stub) => stub.kind === "feat");
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

// ---- Feats: the filter menu, filtering in place, Feat chains and the chain marker (#193 PR 2) ----

const featsInCombat = [...corpus.entries.values()].filter((entry) => entry.kind === "feat" && (entry.fields?.feat_types ?? []).includes("Combat")).length;
const featCount = corpus.catalog.entries.filter((stub) => stub.kind === "feat").length;
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
  await expect(main.getByRole("link", { name: "Choose books" })).toBeVisible();
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
  const leads = featLeadsTo(5);
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
  await expect(main.getByRole("link", { name: "Choose books" })).toBeVisible();
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

test("Classes are listed under their group headings, in the order of the books, without pills or a menu", async ({ page }) => {
  await installLibrary(page);
  const classes = [...corpus.entries.values()].filter((entry) => entry.kind === "class");
  const order = ["core", "base", "hybrid", "occult", "unchained", "alternate", "npc"];
  const labels: Record<string, string> = { core: "Core", base: "Base", hybrid: "Hybrid", occult: "Occult", unchained: "Unchained", alternate: "Alternate", npc: "NPC" };
  const expected = order.map((type) => ({ label: labels[type], titles: classes.filter((entry) => entry.fields?.class_type === type).map((entry) => entry.title).sort() }));
  expect(expected.every((group) => group.titles.length > 0)).toBe(true);

  await page.goto("#/browse/kind/class");
  const main = page.locator("main");
  await expect(main.locator(".list-count")).toHaveText(`${count(classes.length)} classes`);
  await expect(main.locator("section.letter-group")).toHaveCount(expected.length);
  await expect(main.locator(".lens-chip, .lens-more, .lens-page, .lens-pages, .filter-row, .filter-btn")).toHaveCount(0);
  await expect(main.getByText("Core, base, and NPC")).toHaveCount(0);
  await expect(main.getByRole("navigation", { name: "Jump to letter" })).toHaveCount(0);
  const heads = await main.locator("h2.letter-head").allInnerTexts();
  expect(heads.map((head) => head.replace(/\s*[\d,]+$/, "").trim())).toEqual(expected.map((group) => group.label));
  expect(heads.map((head) => num(head.match(/([\d,]+)$/)?.[1]))).toEqual(expected.map((group) => group.titles.length));
  // Every class is under its heading, and a heading names its list.
  for (const [index, group] of expected.entries()) {
    const section = main.locator("section.letter-group").nth(index);
    await expect(section).toHaveAttribute("aria-labelledby", /.+/);
    const titles = (await section.locator(".entry-list a").allInnerTexts()).map((text) => text.trim()).sort();
    expect(titles).toEqual(group.titles);
  }

  // A class opens, and Back returns to the grouped list.
  const first = main.locator("section.letter-group").first().locator(".entry-list a").first();
  const title = (await first.innerText()).trim();
  await first.click();
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/class");
  await expect(main.locator("section.letter-group")).toHaveCount(expected.length);

  // An old link with a group in it shows the same grouped list instead of a broken page.
  await page.goto("#/browse/kind/class?facet=class_type:core");
  await expect(main.locator("section.letter-group")).toHaveCount(expected.length);
  await expect(main.locator(".list-count")).toHaveText(`${count(classes.length)} classes`);
});
