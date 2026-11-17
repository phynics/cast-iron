import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { count, installLibrary, isMember, waitHomeIdle, waitReady, waitResults, writeMeta } from "./helpers";

// The pinned corpus credits Monster Summoner's Handbook as an expansion of Summon Monster and
// Summon Monster I to IX (`expanded_in`, #459): the handbook expands these spells on page 18
// without printing them, so the credit is never a membership. The spec reads the credited
// entries from the compiled corpus and proves what a credit does and does not do.

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;
const HANDBOOK = sources.find((row) => row.title === "Monster Summoner's Handbook")?.id as string;
const HANDBOOK_TITLE = "Monster Summoner's Handbook";
const credited = [...corpus.entries.values()].filter((entry) =>
  ((entry as any).expanded_in ?? []).some((row: any) => row.publication === HANDBOOK),
);
const SUMMON_TITLES = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"].map((n) => `Summon Monster ${n}`);
const SUMMON_III = credited.find((entry) => entry.title === "Summon Monster III")!;
const stubOf = (id: string) => corpus.catalog.entries.find((stub) => stub.id === id)!;
const creditedIds = credited.map((entry) => entry.id);

test("Monster Summoner's Handbook credit: Summon Monster I to IX are hidden and not counted under it", async ({ page }) => {
  // The first install of the library takes longer than the default budget.
  test.setTimeout(300_000);
  expect(HANDBOOK, "the compiled library has Monster Summoner's Handbook").toBeTruthy();
  for (const title of SUMMON_TITLES) {
    expect(credited.map((entry) => entry.title), `${title} carries the handbook credit`).toContain(title);
  }
  for (const entry of credited) {
    expect(isMember(stubOf(entry.id), HANDBOOK), `${entry.title} is credited, not a member`).toBe(false);
    expect(entry.source_ids, `${entry.title} is printed elsewhere`).not.toContain(HANDBOOK);
  }
  const members = corpus.catalog.entries.filter((stub) => isMember(stub, HANDBOOK)).length;

  await installLibrary(page);
  const searchHits = async (query: string) => {
    await page.goto(`#/search?q=${encodeURIComponent(query)}&kind=spell`);
    await waitResults(page, query);
    return page.locator(".results .hit-link").evaluateAll((links) =>
      links.map((link) => (link as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]),
    );
  };

  await test.step("With every Paizo book active, Summon Monster III is visible and shows its credit", async () => {
    expect(await searchHits("Summon Monster III")).toContain(SUMMON_III.id);
    await page.goto(`#/entry/${SUMMON_III.id}`);
    await expect(page.getByTestId("entry-title")).toBeVisible();
    await expect(page.getByTestId("expanded-line")).toHaveCount(1);
    await expect(page.getByTestId("expanded-line")).toContainText(`Expanded in: ${HANDBOOK_TITLE} p. 18`);
    await expect(page.getByTestId("expanded-line")).not.toContainText("Inactive");
    // The credit is not a membership: the entry keeps its own book and the handbook's count stays.
    await page.goto(`#/browse/book/${HANDBOOK}`);
    await expect(page.getByTestId("source-count")).toContainText(`${count(members)} ${members === 1 ? "entry" : "entries"}`);
  });

  await test.step("With only the handbook active, no credited spell is listed, found or counted", async () => {
    await page.goto("#/");
    await waitHomeIdle(page);
    await writeMeta(page, "disabled_books", sources.map((row) => row.id).filter((id) => id !== HANDBOOK));
    await page.reload();
    await waitReady(page);
    await waitHomeIdle(page);

    // The book's count is its members: the credited spells add nothing to it.
    await page.goto(`#/browse/book/${HANDBOOK}`);
    await expect(page.getByRole("heading", { level: 1, name: HANDBOOK_TITLE, exact: true })).toBeVisible();
    await expect(page.getByTestId("source-count")).toContainText(`${count(members)} ${members === 1 ? "entry" : "entries"}`);

    // Search: the spell is gone, whether found by its title or by the kind filter alone.
    for (const query of ["Summon Monster III", "Summon Monster"]) {
      const hits = await searchHits(query);
      for (const id of creditedIds) expect(hits, `${id} in a search for ${query}`).not.toContain(id);
    }

    // Browse: the S group of spells lists none of them.
    await page.goto("#/browse/kind/spell?l=S");
    await expect(page.locator(".entry-list")).toBeVisible();
    for (const id of creditedIds) {
      await expect(page.locator(`.entry-list a[href="#/entry/${id}"]`)).toHaveCount(0);
    }
  });
});
