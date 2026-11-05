import fs from "node:fs";
import path from "node:path";
import { test, expect, GENERATED, loadCorpus } from "./fixtures";
import { corpusEntry, installLibrary } from "./helpers";

const { version, catalog } = loadCorpus();
const legal = JSON.parse(fs.readFileSync(path.join(GENERATED, version, "legal.json"), "utf8"));
const coverage = JSON.parse(fs.readFileSync(path.join(GENERATED, version, "coverage.json"), "utf8"));
const entry = corpusEntry("spell", "Fireball");
const stub = catalog.entries.find((candidate: any) => candidate.id === entry.id)!;

test("Licensing information is preserved and accessible", async ({ page }) => {
  await installLibrary(page);
  // Reachable from the settings screen and from every entry.
  await page.goto("#/settings");
  await page.getByRole("link", { name: "Legal and attribution, and library coverage" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Legal and attribution" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Contact the maintainer" }))
    .toHaveAttribute("href", "https://github.com/phynics/cast-iron/issues");

  const text = page.locator(".legal-text");
  await expect(text.filter({ hasText: "15. COPYRIGHT NOTICE" })).toHaveCount(1);
  await expect(text.filter({ hasText: "OPEN GAME LICENSE Version 1.0a" })).toHaveCount(1);
  expect(legal.licenses[0].text).toContain("15. COPYRIGHT NOTICE");
  await expect(page.getByRole("heading", { name: "Community Use notice" })).toBeVisible();
  await expect(page.locator("#community-use + .legal-text")).toHaveText(legal.community_use_notice);
  await expect(page.getByRole("heading", { name: "Product Identity declaration" })).toBeVisible();

  // Both dataset pins and their commits are shown, whichever library is installed.
  const datasets = page.locator("#dataset ~ .legal-list > li");
  await expect(datasets).toHaveCount(legal.datasets.length);
  for (const dataset of legal.datasets) {
    await expect(datasets.filter({ hasText: dataset.id })).toHaveCount(1);
    await expect(datasets.filter({ hasText: dataset.commit })).toHaveCount(1);
  }

  // Every Section 15 line the compiler collected is shown.
  const lines = page.locator("#section-15 ~ .legal-list > li");
  await expect(lines).toHaveCount(legal.section_15.length);
  await expect(lines.first()).toContainText(legal.section_15[0].title);
  await expect(lines.last()).toContainText(legal.section_15.at(-1).title);

  // Coverage tables: by publication, by category, excluded, links.
  const byPublication = page.locator("table", { has: page.locator("caption", { hasText: "By publication" }) });
  await expect(byPublication.locator("tbody tr")).toHaveCount(coverage.by_publication.length);
  await expect(byPublication).toContainText(String(coverage.by_publication[0].included_entries));
  const byKind = page.locator("table", { has: page.locator("caption", { hasText: "By category" }) });
  await expect(byKind.locator("tbody tr")).toHaveCount(Object.keys(coverage.by_kind).length);
  const excluded = page.locator("table", { has: page.locator("caption", { hasText: "Excluded from this library" }) });
  await expect(excluded.locator("tbody tr")).toHaveCount(coverage.excluded.length);
  await expect(page.locator("table", { has: page.locator("caption", { hasText: "Link coverage" }) })).toBeVisible();

  // An entry carries the Section 15 line of every publication it came from, and a link back here.
  const sourceIds: string[] = (entry as any).source_ids ?? [stub.source];
  const copyrights = [...new Set(sourceIds.map((id) => catalog.sources.find((candidate: any) => candidate.id === id)?.copyright).filter(Boolean))];
  await page.goto(`#/entry/${entry.id}`);
  await expect(page.locator(".entry-foot .copyright")).toHaveCount(copyrights.length);
  for (const copyright of copyrights) {
    await expect(page.locator(".entry-foot .copyright").filter({ hasText: copyright })).toHaveCount(1);
  }
  await page.locator(".entry-foot a[href='#/legal']").click();
  await expect(page.getByRole("heading", { level: 1, name: "Legal and attribution" })).toBeVisible();
});
