// Local acceptance measurements for one built library: install, storage, search, memory.
// Dev and test only; it is not part of the product.
//
//   cargo xtask build-site                        (canonical default)
//   node tests/e2e/measure-library.mjs
//
// For the PSRD carry-over library:
//   cargo xtask content --psrd && cargo xtask build-site --psrd
//   CAST_IRON_CONTENT=generated/psrd node tests/e2e/measure-library.mjs
//
// It serves dist/ on its own port, installs the library in a fresh profile per browser,
// and prints one JSON object plus a short table. Record the output in docs/status.md.

import { chromium, webkit } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { startServer } from "./server.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const dist = path.join(root, "dist");
const generated = path.resolve(root, process.env.CAST_IRON_CONTENT ?? "generated/content");
const port = Number(process.env.E2E_PORT ?? 4399);
const base = `http://127.0.0.1:${port}/cast-iron/`;

const QUERIES = [
  "fire", "cure light wounds", "grapple", "dragon", "sword", "armor class", "turn undead",
  "magic missile", "wizard", "bless", "invisibility", "potion", "goblin", "attack of opportunity",
  "ranger", "stealth", "charm", "weapon focus", "fly", "summon", "acid", "angel", "animal",
  "antipaladin", "arcane", "armor", "arrow", "bard", "bear", "blade", "blind", "blood", "bolt",
  "bow", "burn", "chaos", "cleric", "cold", "combat", "cure", "darkness", "death", "demon",
  "detect", "divine", "dwarf", "eagle", "earth", "elemental", "energy",
];

function firstEntry() {
  const manifest = JSON.parse(fs.readFileSync(path.join(generated, "manifest.json"), "utf8"));
  const catalog = JSON.parse(fs.readFileSync(path.join(generated, manifest.content_version, "catalog.json"), "utf8"));
  // A rule entry is present in every library and gives the reader real body content.
  const stub = catalog.entries.find((entry) => entry.kind === "rule");
  const shard = JSON.parse(
    fs.readFileSync(path.join(generated, manifest.content_version, stub.shard), "utf8"),
  );
  return { id: stub.id, title: shard.find((entry) => entry.id === stub.id).title };
}

const pct = (values, p) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(p * sorted.length) - 1)];
};

async function waitReady(page) {
  await page.getByRole("navigation", { name: "Primary" }).waitFor({ state: "visible", timeout: 600_000 });
}

async function measure(browserName, browserType) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `cast-iron-measure-${browserName}-`));
  const context = await browserType.launchPersistentContext(dir, { serviceWorkers: "allow", baseURL: base });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  page.on("pageerror", (error) => errors.push(error.message));
  const entry = firstEntry();

  const started = Date.now();
  await page.goto(base);
  await page.getByRole("button", { name: "Download" }).waitFor({ timeout: 120_000 });
  const offer = Date.now();
  await page.getByRole("button", { name: "Download" }).click();
  await waitReady(page);
  const ready = Date.now();
  await page.waitForTimeout(1000);

  const storage = await page.evaluate(async () => {
    const estimate = await navigator.storage.estimate();
    return { usage: estimate.usage, quota: estimate.quota };
  });

  const entryStart = Date.now();
  await page.goto(`#/entry/${entry.id}`);
  await page.locator(".entry > .rich").waitFor({ state: "visible", timeout: 120_000 });
  const firstEntryMs = Date.now() - entryStart;

  // Facet chips: the time from opening Search until every rendered chip has a count, the
  // time from choosing a kind until its School and Spell level chips appear, and the time
  // from toggling one facet value until the chip counts reflect that selection. These are
  // the user-visible costs of facet counting on the full canonical library.
  const openStart = Date.now();
  await page.goto("#/search");
  await page.locator(".filters .chip-count").first().waitFor({ state: "visible", timeout: 180_000 });
  const openFirstChipMs = Date.now() - openStart;
  await page.waitForFunction(
    () => {
      const counts = [...document.querySelectorAll(".filters .chip-count")];
      return counts.length > 0 && counts.every((el) => /^\d+$/.test(el.textContent ?? ""));
    },
    undefined,
    { timeout: 180_000 },
  );
  const openCountsMs = Date.now() - openStart;

  const kindStart = Date.now();
  await page.locator(".filters .chip", { hasText: /^Spells/ }).click();
  const school = page.locator('.filter-group:has(legend:text-is("School"))');
  await school.locator(".chip-count").first().waitFor({ state: "visible", timeout: 180_000 });
  const kindsFacetMs = Date.now() - kindStart;

  const toggleStart = Date.now();
  const countsBefore = await page.evaluate(() =>
    [...document.querySelectorAll(".filters .chip-count")].map((el) => el.textContent),
  );
  await school.locator(".chip", { hasText: /^evocation\d*$/i }).first().click();
  await page.waitForFunction(
    (before) => {
      const now = [...document.querySelectorAll(".filters .chip-count")].map((el) => el.textContent);
      return now.length > 0 && JSON.stringify(now) !== JSON.stringify(before);
    },
    countsBefore,
    { timeout: 180_000 },
  );
  const facetToggleMs = Date.now() - toggleStart;

  // Warm the index once so the first query does not carry the index load.
  await page.goto("#/search");
  await page.locator("#search-input").fill("warm up");
  await page.waitForFunction(() => document.querySelector(".results-region")?.getAttribute("data-query") === "warm up", undefined, { timeout: 120_000 });

  const times = [];
  const wall = [];
  for (const query of QUERIES) {
    const start = Date.now();
    await page.locator("#search-input").fill(query);
    await page.waitForFunction((q) => document.querySelector(".results-region")?.getAttribute("data-query") === q, query, { timeout: 60_000 });
    const measured = Number(await page.locator(".results-region").getAttribute("data-search-ms"));
    times.push(Number.isFinite(measured) ? measured : Date.now() - start);
    wall.push(Date.now() - start);
  }

  // The JS heap with the search index and one entry page in use. WebAssembly memory is a
  // separate allocation, so this is a floor, not the whole process.
  let memory = null;
  if (browserName === "chromium") {
    const session = await context.newCDPSession(page);
    await session.send("Performance.enable");
    const metrics = await session.send("Performance.getMetrics");
    memory = Object.fromEntries(metrics.metrics.map((metric) => [metric.name, metric.value]));
  }

  await context.close();
  fs.rmSync(dir, { recursive: true, force: true });

  return {
    browser: browserName,
    manifest: JSON.parse(fs.readFileSync(path.join(dist, "content/manifest.json"), "utf8")).content_version,
    entries: JSON.parse(fs.readFileSync(path.join(dist, "content/manifest.json"), "utf8")).entry_count,
    offerMs: offer - started,
    installMs: ready - offer,
    readyMs: ready - started,
    storageUsageBytes: storage.usage,
    storageQuotaBytes: storage.quota,
    firstEntryId: entry.id,
    firstEntryMs,
    facets: {
      openFirstChipMs,
      openCountsMs,
      kindsFacetMs,
      facetToggleMs,
    },
    search: {
      queries: QUERIES.length,
      engineP50: pct(times, 0.5),
      engineP95: pct(times, 0.95),
      engineMax: Math.max(...times),
      wallP50: pct(wall, 0.5),
      wallP95: pct(wall, 0.95),
    },
    memory: memory && {
      jsHeapUsedBytes: memory.JSHeapUsedSize,
      jsHeapTotalBytes: memory.JSHeapTotalSize,
      documents: memory.Documents,
      nodes: memory.Nodes,
    },
    consoleErrors: errors,
  };
}

if (!fs.existsSync(path.join(dist, "content/manifest.json"))) {
  console.error("No built site at dist/. Run cargo xtask build-site first.");
  process.exit(1);
}
const server = await startServer({ dist, port });
const results = [];
try {
  for (const [name, type] of [["chromium", chromium], ["webkit", webkit]]) {
    results.push(await measure(name, type));
  }
} finally {
  server.server.close();
}
console.log(JSON.stringify(results, null, 2));
console.table(results.map((result) => ({
  browser: result.browser,
  "install s": (result.installMs / 1000).toFixed(1),
  "storage MB": (result.storageUsageBytes / 1e6).toFixed(1),
  "first entry ms": result.firstEntryMs,
  "open chips ms": result.facets.openFirstChipMs,
  "open counts ms": result.facets.openCountsMs,
  "kind facets ms": result.facets.kindsFacetMs,
  "facet toggle ms": result.facets.facetToggleMs,
  "engine p50": result.search.engineP50,
  "engine p95": result.search.engineP95,
  "wall p50": result.search.wallP50,
  "wall p95": result.search.wallP95,
  "heap MB": result.memory ? (result.memory.jsHeapUsedBytes / 1e6).toFixed(0) : "n/a",
  errors: result.consoleErrors.length,
})));
