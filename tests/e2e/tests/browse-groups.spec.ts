import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { corpusEntry, count, installLibrary, letterId, waitControlled } from "./helpers";

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
