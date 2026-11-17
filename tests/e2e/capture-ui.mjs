// Capture and measure the actual dist build. Dev-only; never shipped with the PWA.
//
// Writes one PNG per state, `measurements.json`, and `manifest.json` with the provenance of
// every capture (commit, shell and content version, corpus tag, viewport, theme, settled state,
// masthead probe). The conditional views (storage failure, pending updates, iOS install guide)
// are in `capture-conditional.mjs`.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { buildProvenance, dist, here, pageFacts, shoot, viewportName } from './capture-lib.mjs';

const out = path.resolve(process.argv[2] ?? '/tmp/cast-iron-ui-simplification');
fs.mkdirSync(out, { recursive: true });
const port = process.env.E2E_VISUAL_PORT ?? '4399';
const baseURL = `http://127.0.0.1:${port}/cast-iron/`;
const server = spawn(process.execPath, ['server.mjs'], {
  cwd: here, env: { ...process.env, E2E_PORT: port }, stdio: 'ignore',
});
let browser;
const results = [];
const captures = [];
const build = buildProvenance();

/**
 * Titles that the library publishes under several books, from the compiled catalog, so the
 * fixture follows the corpus instead of naming a title that can disappear. Kind lists are paged
 * by first letter, so each candidate carries its letter. A title can be a revision that the list
 * merges into one row (the Elephant revisions), so the capture takes the first candidate whose
 * disclosure the browse list really renders.
 */
function sourceVariantCandidates() {
  const catalog = JSON.parse(fs.readFileSync(path.join(dist, 'content', build.content_version, 'catalog.json'), 'utf8'));
  const candidates = [];
  for (const kind of ['creature', 'item', 'spell', 'trait', 'feat']) {
    const groups = new Map();
    for (const stub of catalog.entries) if (stub.kind === kind) groups.set(stub.title, [...(groups.get(stub.title) ?? []), stub]);
    for (const title of [...groups.keys()].sort()) {
      if (new Set(groups.get(title).map(stub => stub.source)).size < 2) continue;
      const first = [...title].find(c => /[\p{L}\p{N}]/u.test(c))?.toUpperCase();
      candidates.push({ kind, title, letter: first && /^[A-Z]$/.test(first) ? first : '#', count: groups.get(title).length });
    }
  }
  if (!candidates.length) throw new Error('the compiled catalog has no title published under two books');
  return candidates;
}
const escapeRegExp = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const variantGroup = (page, title) => page.locator('.entry-list .source-variants')
  .filter({ has:page.locator('summary', { hasText:new RegExp(`^${escapeRegExp(title)} `) }) });
let variantFixture = null;
/** The first candidate, in catalog order, that the browse list renders as a disclosure. */
async function openSourceVariants(page) {
  const list = variantFixture ? [variantFixture] : sourceVariantCandidates().slice(0, 200);
  for (const candidate of list) {
    await page.goto(`#/browse/kind/${candidate.kind}?l=${encodeURIComponent(candidate.letter)}`);
    await page.locator('.entry-list').first().waitFor();
    const group = variantGroup(page, candidate.title);
    if (await group.count() !== 1) continue;
    variantFixture = candidate;
    await group.locator('summary').click();
    await group.scrollIntoViewIfNeeded();
    return candidate;
  }
  throw new Error('no catalog title with two books renders as a source-variants disclosure');
}
const searchOnly = process.argv.includes('--search-only');
try {
  let ready = false;
  for (let i = 0; i < 50; i++) {
    try { ready = (await fetch(baseURL)).ok; } catch {}
    if (ready) break;
    if (server.exitCode !== null) throw new Error('Visual test server failed to start');
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  if (!ready) throw new Error('Visual test server did not become ready');
  browser = await chromium.launch();
  for (const mobile of [false, true]) for (const colorScheme of ['light', 'dark']) {
    const tag = `${mobile ? 'mobile' : 'desktop'}-${colorScheme}`;
    const context = await browser.newContext({ baseURL, viewport: mobile ? { width:390, height:844 } : { width:1280, height:900 }, colorScheme, isMobile:mobile, hasTouch:mobile });
    const page = await context.newPage();
    const probePage = await context.newPage();
    await probePage.goto('about:blank');
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    const capture = async (name, hash, readySelector) => {
      if (searchOnly && !name.startsWith('search-')) return;
      if (hash) await page.goto(hash);
      const record = await shoot(page, probePage, { file: path.join(out, `${tag}-${name}.png`), readySelector });
      if (record.warning) console.warn(`${tag}-${name}: ${record.warning}`);
      captures.push({ state: name, viewport: { name: viewportName(page.viewportSize()), ...page.viewportSize() }, theme: colorScheme, ...record, page: await pageFacts(page, browser, { engine: 'chromium' }) });
      const measurement = await page.evaluate(() => ({
        overflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - innerWidth,
        primaryTop: document.querySelector('.entry > .rich')?.getBoundingClientRect().top,
        relatedTop: document.querySelector('#entry-related')?.getBoundingClientRect().top,
        smallTargets: [...document.querySelectorAll('.letter-btn, .lvl')].filter(el => {
          const r = el.getBoundingClientRect(); return r.width > 0 && (r.width < 44 || r.height < 44);
        }).length,
        activeScopes: [...document.querySelectorAll('.scope-choice[aria-pressed="true"]')].map(el => ({ text:el.textContent, background:getComputedStyle(el).backgroundColor, foreground:getComputedStyle(el).color })),
      }));
      results.push({ tag, name, ...measurement });
    };
    await page.goto('./');
    await page.getByRole('button', { name:'Download', exact:true }).click();
    await page.locator('#home-title').waitFor({ timeout:90000 });
    if (!searchOnly) {
    await capture('home-empty', null, '#home-title');
    await page.goto('#/entry/core-rulebook/spells/fireball');
    await page.locator('.bookmark-toggle').click();
    await capture('home-stars', '#/', '.home-bookmarks a');
    // Publications of one title sit under one disclosure. Kind lists are paged by first letter.
    await openSourceVariants(page);
    await capture('source-variants', null, '.source-variants[open]');
    // The Books page lists priority tiers; only Rulebooks start open. Open every tier and series.
    await page.goto('#/books');
    await page.locator('.book-tier').first().waitFor();
    for (const toggle of ['.tier-toggle[aria-expanded="false"]', '.series-toggle[aria-expanded="false"]']) {
      for (let left = await page.locator(toggle).count(); left > 0; left--) await page.locator(toggle).first().click();
    }
    await capture('books', null, '.book-directory-row');
    // A book row opens the book page; its contents tree is the second screen of the audit.
    await page.locator('a.book-directory-row', { has:page.locator('.cat-name', { hasText:/^Core Rulebook$/ }) }).click();
    await capture('book-contents', null, '.toc');
    await capture('class', '#/entry/core-rulebook/classes/fighter', '.entry > .rich');
    await capture('race', '#/entry/core-rulebook/races/dwarves', '.entry > .rich');
    await capture('prestige', '#/entry/core-rulebook/classes/arcane-archer', '.rel-requirements');
    await capture('spell-list', '#/spells/Wizard/1', '.level-group');
    }
    await capture('search-all', '#/search?q=grapple', '.results .hit-link:visible');
    await page.getByRole('button', { name:'Rules text', exact:true }).click();
    await page.waitForFunction(() => document.querySelector('.scope-choice[aria-pressed="true"]')?.textContent === 'Rules text');
    await page.waitForFunction(() => {
      const rows = [...document.querySelectorAll('.results .hit-meta')];
      // The Rules scope also lists Stamina and Mythic add-on rows, so only the first row is a Rule.
      return location.hash.includes('kind=rule') && rows.length > 0 && rows[0].textContent.startsWith('Rule ·');
    });
    // The first row can be a group of publications and carry no anchor; some row must.
    await page.waitForFunction(() => [...document.querySelectorAll('.results .hit-link')].some(link => link.getAttribute('href')?.includes('?a=')));
    await capture('search-rules', null, '.results .hit-link:visible');
    results.push({ tag, errors });
    await context.close();
  }
  fs.writeFileSync(path.join(out, 'measurements.json'), JSON.stringify(results, null, 2));
  fs.writeFileSync(path.join(out, 'manifest.json'), JSON.stringify({
    script: 'tests/e2e/capture-ui.mjs',
    build,
    fixtures: { source_variants: variantFixture, search_query: 'grapple', book_page: 'Core Rulebook' },
    captures,
  }, null, 2));
  const missing = captures.filter(c => c.warning);
  if (missing.length) console.warn(`masthead missing from ${missing.length} saved frames: ${missing.map(c => c.file).join(', ')}`);
  console.log(JSON.stringify({ out, screenshots:results.filter(r => r.name).length, overflows:results.filter(r => r.overflow > 0), smallTargets:results.filter(r => r.smallTargets > 0), errors:results.filter(r => r.errors?.length) }, null, 2));
} finally {
  if (browser) await browser.close();
  server.kill();
}
