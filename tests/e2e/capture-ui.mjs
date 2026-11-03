// Capture and measure the actual dist build. Dev-only; never shipped with the PWA.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.resolve(process.argv[2] ?? '/tmp/cast-iron-ui-simplification');
fs.mkdirSync(out, { recursive: true });
const port = process.env.E2E_VISUAL_PORT ?? '4399';
const baseURL = `http://127.0.0.1:${port}/cast-iron/`;
const server = spawn(process.execPath, ['server.mjs'], {
  cwd: here, env: { ...process.env, E2E_PORT: port }, stdio: 'ignore',
});
let browser;
const results = [];
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
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    const capture = async (name, hash, readySelector) => {
      if (searchOnly && !name.startsWith('search-')) return;
      if (hash) await page.goto(hash);
      if (readySelector) await page.locator(readySelector).first().waitFor();
      await page.screenshot({ path:path.join(out, `${tag}-${name}.png`) });
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
    await page.goto('#/browse/kind/trait');
    const variants = page.locator('.entry-list .source-variants').filter({ has:page.locator('summary', { hasText:/^Anatomist / }) });
    await variants.locator('summary').click();
    await variants.scrollIntoViewIfNeeded();
    await capture('source-variants', null, '.source-variants[open]');
    await capture('books', '#/books', '.book-directory-row');
    const core = page.locator('[data-book-id="core-rulebook"]');
    await core.locator('summary').click();
    await capture('books-expanded', null, '.book-directory-chapters a');
    await core.getByRole('link', { name:'Open Core Rulebook contents' }).click();
    await capture('book-contents', null, '.toc');
    await capture('class', '#/entry/core-rulebook/classes/fighter', '.entry > .rich');
    await capture('race', '#/entry/core-rulebook/races/dwarves', '.entry > .rich');
    await capture('prestige', '#/entry/core-rulebook/classes/arcane-archer', '.rel-requirements');
    await capture('spell-list', '#/spells/Wizard/1', '.level-group');
    }
    await capture('search-all', '#/search?q=grapple', '.results .hit-link');
    await page.getByRole('button', { name:'Rules text', exact:true }).click();
    await page.waitForFunction(() => document.querySelector('.scope-choice[aria-pressed="true"]')?.textContent === 'Rules text');
    await page.waitForFunction(() => {
      const rows = [...document.querySelectorAll('.results .hit-meta')];
      return rows.length > 0 && rows.every(row => row.textContent.startsWith('Rule ·'));
    });
    await page.waitForFunction(() => document.querySelector('.results .hit-link')?.getAttribute('href')?.includes('?a='));
    await capture('search-rules', null, '.results .hit-link');
    results.push({ tag, errors });
    await context.close();
  }
  fs.writeFileSync(path.join(out, 'measurements.json'), JSON.stringify(results, null, 2));
  console.log(JSON.stringify({ out, screenshots:results.filter(r => r.name).length, overflows:results.filter(r => r.overflow > 0), smallTargets:results.filter(r => r.smallTargets > 0), errors:results.filter(r => r.errors?.length) }, null, 2));
} finally {
  if (browser) await browser.close();
  server.kill();
}
