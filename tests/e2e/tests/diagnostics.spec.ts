import { test, installedTest, expect, distManifest, loadCorpus, type Page } from "./fixtures";
import { count, installLibrary, searchFor, waitControlled, waitHomeIdle, waitReady } from "./helpers";

/**
 * Settings, Diagnostics (#423): the numbers that the app records about itself, for the iPhone
 * measurement session (`docs/research/iphone-measurement.md`). The page shows plain text and
 * Copy puts that text on the clipboard. Nothing is sent anywhere (rule 1).
 */

const corpus = loadCorpus();
const manifest = distManifest();
const spell = corpus.catalog.entries.find((row) => row.kind === "spell" && row.title === "Fireball")!;

const SECTIONS = [
  "Device and library",
  "Storage and memory",
  "Install (the last one on this device)",
  "This launch (times count from the start of the page)",
  "Searches in this launch",
  "Pages opened in this launch",
  "Earlier launches (newest first)",
];

async function openDiagnostics(page: Page) {
  await page.goto("#/settings/diagnostics");
  await expect(page.getByRole("heading", { level: 1, name: "Diagnostics" })).toBeVisible();
  await expect(page.getByText("Collecting...")).toHaveCount(0);
  await expect(page.locator(".diagnostics-section").first()).toBeVisible();
}

/** The sections as the page shows them, and the text that Copy must put on the clipboard. */
async function shown(page: Page) {
  const sections = await page.locator(".diagnostics-section").evaluateAll((nodes) =>
    nodes.map((node) => ({
      title: node.querySelector("h2")!.textContent!,
      body: node.querySelector("pre")!.textContent!,
    })),
  );
  const copied =
    "Cast Iron diagnostics\n" +
    sections.map(({ title, body }) => `\n${title}\n` + body.split("\n").map((line) => `  ${line}\n`).join("")).join("");
  return { sections, copied, text: sections.map((s) => s.body).join("\n") };
}

installedTest("Settings links to Diagnostics, which lists the launch, the library and the install", async ({ page, browserName }) => {
  await installLibrary(page);
  await page.goto("#/settings");
  await page.getByRole("link", { name: "Timings and storage" }).click();
  await expect(page).toHaveURL(/#\/settings\/diagnostics$/);
  await expect(page.getByRole("heading", { level: 1, name: "Diagnostics" })).toBeVisible();
  await expect(page.getByText("Collecting...")).toHaveCount(0);
  // Diagnostics belongs to Settings in the navigation.
  await expect(page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Settings" })).toHaveAttribute("aria-current", "page");

  const { sections, text } = await shown(page);
  expect(sections.map((s) => s.title)).toEqual(SECTIONS);
  expect(text).toContain(`library id: ${manifest.content_version}`);
  expect(text).toContain(`${count(manifest.entry_count)} entries, ${count(manifest.files.length)} files`);
  expect(text).toMatch(/launch: #\d+, shown as browser tab, online at start: yes/);
  expect(text).toMatch(/Home on screen, ready to use: [\d.]+ (ms|s)/);
  expect(text).toMatch(/library files checked: [\d.]+ (ms|s); catalog read: [\d.]+ (ms|s)/);
  expect(text).toMatch(/search index: read [\d.]+ (ms|s), decode [\d.]+ (ms|s), [\d.]+ MB; ready at [\d.]+ (ms|s)/);
  expect(text).toMatch(/storage used: [\d.]+ \w+ of [\d.]+ \w+/);
  expect(text).toMatch(/persistent storage: (granted|not granted)/);
  expect(text).toMatch(/WebAssembly memory \(peak\): [\d.]+ \w+/);
  // The installation that the profile copy carries: its phases and what the persist request got.
  expect(text).toMatch(/total: [\d.]+ (ms|s|min)/);
  expect(text).toMatch(/scan [\d.]+ \w+, download [\d.]+ \w+.*, verify [\d.]+ \w+.*, activate [\d.]+ \w+/);
  expect(text).toContain(`files: ${count(manifest.files.length)} (0 already there)`);
  expect(text).toMatch(/persistent storage asked for: (granted|not granted)/);
  // Chromium offers a JavaScript heap number; Safari does not, and the page says so.
  if (browserName === "chromium") expect(text).toMatch(/JavaScript heap: [\d.]+ \w+ used of/);
  else expect(text).toContain("JavaScript heap: not offered by this browser");
});

installedTest("A search and a page that were opened show up in the numbers, without the query", async ({ page }) => {
  await installLibrary(page);
  await searchFor(page, "grapple");
  await searchFor(page, "initiative");
  await page.goto(`#/entry/${spell.id}`);
  await expect(page.getByTestId("entry-title")).toHaveText(spell.title);
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await openDiagnostics(page);

  const { text } = await shown(page);
  expect(text).toMatch(/\d+ searches\. A search starts 80 ms after the last key\./);
  expect(text).toMatch(/first: 7 characters, [\d,]+ results: [\d.]+ (ms|s) \(engine [\d.]+ (ms|s)\) \(waited [\d.]+ (ms|s) for the index\)/);
  expect(text).toMatch(/last \d+:\n- \d+ characters, [\d,]+ results/);
  expect(text).toMatch(new RegExp(`${spell.title}: on screen after [\\d.]+ (ms|s), [\\d,]+ elements, [\\d,]+ characters`));
  // The report holds lengths and counts only.
  expect(text).not.toContain("grapple");
  expect(text).not.toContain("initiative");
});

installedTest("Copy puts the text on the clipboard, and no request leaves the page", async ({ page, context, server, browserName }) => {
  await installLibrary(page);
  await openDiagnostics(page);
  // Only Chromium lets a test grant the clipboard. WebKit still runs the click and the status.
  if (browserName === "chromium") await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await server.clearLog();

  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(page.getByText("Collecting...")).toHaveCount(0);
  await page.getByRole("button", { name: "Copy results", exact: true }).click();
  const status = page.getByTestId("diagnostics-status");
  if (browserName === "chromium") {
    await expect(status).toHaveText("Copied the results.");
    const { copied } = await shown(page);
    const onClipboard = await page.evaluate(() => navigator.clipboard.readText());
    expect(onClipboard).toBe(copied);
    expect(onClipboard.startsWith("Cast Iron diagnostics\n\nDevice and library\n  made: ")).toBe(true);
    expect(onClipboard).toContain(`  library id: ${manifest.content_version}\n`);
  } else {
    // A browser that refuses the clipboard says so, and the text stays on the page to select.
    await expect(status).toHaveText(/^(Copied the results\.|Could not copy: .*Press and hold the text below)/);
  }
  // Chromium asks for sw.js itself, whatever the page does; nothing else may be requested.
  expect((await server.requests()).filter((p) => !p.startsWith("sw.js"))).toEqual([]);
});

installedTest("Diagnostics fits the screen and its buttons are 44 px targets", async ({ page }) => {
  await installLibrary(page);
  await openDiagnostics(page);
  for (const name of ["Copy results", "Refresh"]) {
    const box = await page.getByRole("button", { name, exact: true }).boundingBox();
    expect(box!.height, name).toBeGreaterThanOrEqual(44);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
});

installedTest("Earlier launches are listed, newest first", async ({ page }) => {
  await installLibrary(page);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
  await openDiagnostics(page);
  const { text } = await shown(page);
  const number = Number(/launch: #(\d+),/.exec(text)![1]);
  expect(number).toBeGreaterThanOrEqual(2);
  expect(text).toMatch(new RegExp(`#${number - 1}: browser tab, online at start yes, Home [\\d.]+ (ms|s), 0 searches`));
});

test("A restart with no network is recorded as an offline launch that reached Home", async ({ openProfile, server }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);
  await context.close();

  await server.clearLog();
  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("./");
  await waitReady(page);
  await expect(page.locator("#home-title")).toBeVisible();
  await waitHomeIdle(page);
  await openDiagnostics(page);
  const { text } = await shown(page);
  expect(text).toMatch(/launch: #\d+, shown as browser tab, online at start: no/);
  expect(text).toMatch(/Home on screen, ready to use: [\d.]+ (ms|s)/);
  expect(text).toContain("online now: no");
  // The install record from the online launch is still there.
  expect(text).toMatch(/persistent storage asked for: (granted|not granted)/);
});
