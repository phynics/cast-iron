import { test, expect, loadCorpus } from "./fixtures";
import { buildLibraryWithStatSets, installLibrary, waitControlled } from "./helpers";

// A creature that prints several stat sets lists them on its entry page (#166). The pinned corpus
// has no converted stat sets yet, so the spec serves the compiled library with `stat_sets` and
// their `##` headings added to one creature. The derivation, the drift rule and the CR facet are
// the native tests' business (`cast_iron_schema::stat_sets`, the compiler's corpus tests).

const corpus = loadCorpus();
const stubs: any[] = corpus.catalog.entries;
const TARGET = stubs.find((stub) => stub.id === "ref/monster/air-elemental");
const CONTROL = stubs.find((stub) => stub.kind === "creature" && stub.id !== TARGET?.id);

test("a creature with several stat sets lists them with their rating and jumps to a set", async ({ page, server }) => {
  test.setTimeout(300_000);
  expect(TARGET, "the compiled library has Air Elemental").toBeTruthy();
  expect(CONTROL, "the compiled library has another creature").toBeTruthy();

  const variant = buildLibraryWithStatSets({
    [TARGET.id]: [
      { title: "Small Air Elemental", anchor: "stat-small", cr: { twenty_fourths: 24 }, size: "small" },
      { title: "Large Air Elemental", anchor: "stat-large", cr: { twenty_fourths: 120 }, size: "large" },
      { title: "Elder Air Elemental", anchor: "stat-elder", cr: { twenty_fourths: 264 }, size: "huge" },
    ],
  });
  try {
    await server.overlay(variant.dir);
    await installLibrary(page);
    await waitControlled(page);

    const list = page.getByRole("navigation", { name: "Stat sets" });

    await test.step("the sets are listed above the body, in order, with CR and size, named as one", async () => {
      await page.goto(`#/entry/${TARGET.id}`);
      await expect(page.getByTestId("entry-title")).toBeVisible();
      await expect(list).toHaveCount(1);
      const links = list.getByRole("link");
      await expect(links).toHaveCount(3);
      await expect(links.nth(0)).toHaveAccessibleName("Small Air Elemental, CR 1 · Small");
      await expect(links.nth(1)).toHaveAccessibleName("Large Air Elemental, CR 5 · Large");
      await expect(links.nth(2)).toHaveAccessibleName("Elder Air Elemental, CR 11 · Huge");
      await expect(links.nth(2)).toHaveAttribute("href", `#/entry/${TARGET.id}?a=stat-elder`);
      // The list comes before the body it points into.
      const listTop = (await list.boundingBox())!.y;
      const bodyTop = (await page.locator(".rich").first().boundingBox())!.y;
      expect(listTop).toBeLessThan(bodyTop);
    });

    await test.step("a link scrolls to its heading and the list keeps working after it", async () => {
      await list.getByRole("link", { name: /Elder Air Elemental/ }).click();
      await expect(page).toHaveURL(new RegExp(`#/entry/${TARGET.id}\\?a=stat-elder$`));
      await expect(page.locator("#h-stat-elder")).toBeInViewport();
      await list.getByRole("link", { name: /Large Air Elemental/ }).click();
      await expect(page.locator("#h-stat-large")).toBeInViewport();
    });

    await test.step("a creature with no stat sets shows no list", async () => {
      await page.goto(`#/entry/${CONTROL.id}`);
      await expect(page.getByTestId("entry-title")).toBeVisible();
      await expect(page.getByRole("navigation", { name: "Stat sets" })).toHaveCount(0);
    });
  } finally {
    variant.cleanup();
  }
});
