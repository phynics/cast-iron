import fs from "node:fs";
import path from "node:path";
import { installedTest as test, expect, GENERATED, loadCorpus } from "./fixtures";
import { corpusEntry, installLibrary } from "./helpers";

const { version, catalog } = loadCorpus();
const legal = JSON.parse(fs.readFileSync(path.join(GENERATED, version, "legal.json"), "utf8"));
const coverage = JSON.parse(fs.readFileSync(path.join(GENERATED, version, "coverage.json"), "utf8"));
const entry = corpusEntry("spell", "Fireball");
const stub = catalog.entries.find((candidate: any) => candidate.id === entry.id)!;

/** The group label the Legal page gives a coverage reason. */
function reasonGroup(reason: string) {
  for (const group of ["converter derived exclusion", "converter native exclusion"]) {
    if (reason.startsWith(group)) return group;
  }
  if (reason.startsWith("no publication with a Section 15 notice")) {
    return "no publication with a Section 15 notice or a notice-pending decision";
  }
  if (reason.startsWith("converter coverage [")) {
    const end = reason.indexOf("]");
    if (end >= 0) return reason.slice(0, end + 1);
  }
  if (reason.startsWith("rights basis `")) {
    const end = reason.indexOf("`", "rights basis `".length);
    if (end >= 0) return reason.slice(0, end + 1);
  }
  return reason;
}

async function tableTotals(page: import("@playwright/test").Page, caption: string) {
  const table = page.locator("table", { has: page.locator("caption", { hasText: caption }) });
  return table.locator("tbody tr").evaluateAll((rows) =>
    rows.map((row) => [...row.children].slice(1).map((cell) => Number(cell.textContent ?? "0"))),
  );
}

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

  // Publications shipped with a reviewed notice-pending decision are disclosed
  // separately, never as a fabricated Section 15 line.
  if (legal.notice_pending?.length) {
    await expect(page.getByRole("heading", { name: "Section 15 notice pending" })).toBeVisible();
    const pending = page.locator("#notice-pending ~ .legal-list > li");
    await expect(pending).toHaveCount(legal.notice_pending.length);
    for (const title of legal.notice_pending) {
      await expect(pending.filter({ hasText: title })).toHaveCount(1);
    }
    expect(legal.section_15.map((line: any) => line.title)).not.toContain(legal.notice_pending[0]);
  }

  // Coverage opens with counts, not with every upstream path. The initial page stays small
  // enough for a phone, and each full table is built only when its disclosure opens.
  await expect(page.locator("[data-coverage-summary]")).toBeVisible();
  const renderedText = await page.evaluate(() => document.body.innerText.length);
  expect(renderedText, "the initial legal page must not carry the full coverage dump").toBeLessThan(600_000);
  await expect(page.locator('[data-detail="excluded"] table')).toHaveCount(0);
  await expect(page.locator('[data-detail="publications"] table')).toHaveCount(0);

  const byKind = page.locator("table", { has: page.locator("caption", { hasText: "By category" }) });
  await expect(byKind.locator("tbody tr")).toHaveCount(Object.keys(coverage.by_kind).length);

  // The exclusion summary groups the coverage rows. The groups, paths, and records must add
  // back to the compiled report without dropping a row.
  const exclusionGroups = new Set(coverage.excluded.map((row: any) => reasonGroup(row.reason)));
  const exclusionSummary = page.locator("table", { has: page.locator("caption", { hasText: "Exclusions by reason" }) });
  await expect(exclusionSummary.locator("tbody tr")).toHaveCount(exclusionGroups.size);
  const exclusionTotals = await tableTotals(page, "Exclusions by reason");
  expect(exclusionTotals.reduce((sum, [paths]) => sum + paths, 0)).toBe(coverage.excluded.length);
  expect(exclusionTotals.reduce((sum, [, records]) => sum + records, 0)).toBe(
    coverage.excluded.reduce((sum: number, row: any) => sum + row.records, 0),
  );

  // The full tables remain available on demand.
  const publications = page.locator('[data-detail="publications"]');
  await publications.locator("summary").click();
  const byPublication = publications.locator("table", { has: page.locator("caption", { hasText: "By publication" }) });
  await expect(byPublication.locator("tbody tr")).toHaveCount(coverage.by_publication.length);
  await expect(byPublication).toContainText(String(coverage.by_publication[0].included_entries));

  const excluded = page.locator('[data-detail="excluded"]');
  await excluded.locator("summary").click();
  const excludedTable = excluded.locator("table", { has: page.locator("caption", { hasText: "Excluded from this library" }) });
  await expect(excludedTable.locator("tbody tr")).toHaveCount(coverage.excluded.length);
  await expect(excludedTable).toContainText(coverage.excluded[0].path);
  // The rows are built once: closing the disclosure hides them but keeps them mounted.
  await excluded.locator("summary").click();
  await expect(excluded).not.toHaveAttribute("open", "");
  await expect(excludedTable.locator("tbody tr")).toHaveCount(coverage.excluded.length);

  if (coverage.rejected.length) {
    const rejectedGroups = new Set(coverage.rejected.map((row: any) => reasonGroup(row.reason)));
    const rejectedSummary = page.locator("table", { has: page.locator("caption", { hasText: "Rejected records by reason" }) });
    await expect(rejectedSummary.locator("tbody tr")).toHaveCount(rejectedGroups.size);
    const rejected = page.locator('[data-detail="rejected"]');
    await rejected.locator("summary").click();
    await expect(rejected.locator("table").locator("tbody tr")).toHaveCount(coverage.rejected.length);
  }

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
