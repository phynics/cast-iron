import { test, expect, loadCorpus } from "./fixtures";
import { corpusEntry, installLibrary, waitControlled } from "./helpers";

const corpus = loadCorpus();

test("A long kind opens one letter group, focuses it, and Back returns to it", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const fireball = corpusEntry("spell", "Fireball");

  await page.goto("#/browse/kind/spell?l=F");
  await expect(page.getByRole("navigation", { name: "Jump to letter" })).toBeVisible();
  const group = page.locator("#letter-F");
  await expect(group.locator(".letter-head")).toHaveText(/^F/);
  await expect(group).toBeInViewport();
  await expect(group.locator(".letter-head")).toBeFocused();
  await expect(page.locator('.letter-btn[aria-current="true"]')).toHaveText("F");
  // Only the selected group holds entry links; the other letters stay collapsed.
  await expect(page.locator(".letter-group .show-letter").first()).toBeVisible();

  await group.getByRole("link", { name: fireball.title, exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: fireball.title })).toBeVisible();
  await expect.poll(() => new URL(page.url()).hash).toContain(`#/entry/${fireball.id}`);

  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/spell?l=F");
  await expect(page.locator("#letter-F")).toBeInViewport();
  await expect(page.locator("#letter-F .letter-head")).toBeFocused();
  await expect(page.locator("#letter-F .entry-list")).toBeVisible();
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
