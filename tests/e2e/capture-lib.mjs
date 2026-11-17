// Shared pieces of the screenshot capture scripts (`capture-ui.mjs`, `capture-conditional.mjs`).
// Dev-only; never shipped with the PWA.
//
// Every capture gets the same three things: a settle step that waits until the page has stopped
// changing, a masthead probe that records what the browser says about the header before the
// shot, and a frame probe that reads the saved PNG back and says whether the masthead is in it.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const here = path.dirname(fileURLToPath(import.meta.url));
export const root = path.resolve(here, '../..');
export const dist = process.env.E2E_DIST ? path.resolve(process.env.E2E_DIST) : path.join(root, 'dist');

export const VIEWPORTS = {
  phone: { width: 390, height: 844 },
  desktop: { width: 1280, height: 900 },
};
export const THEMES = ['light', 'dark'];

export const IOS_SAFARI_UA =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1';

const git = (...args) => {
  try { return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim(); } catch { return null; }
};

/** What the build under test is, read from the commit and from `dist/`. */
export function buildProvenance() {
  const manifest = JSON.parse(fs.readFileSync(path.join(dist, 'content/manifest.json'), 'utf8'));
  const sw = fs.readFileSync(path.join(dist, 'sw.js'), 'utf8');
  const shell = /const SHELL_VERSION = "([^"]+)";/.exec(sw)?.[1] ?? null;
  // The app is built from these paths. A change under tests/ or docs/ does not alter dist/.
  const dirty = git('status', '--porcelain', '--', 'crates', 'web', 'sources', 'Cargo.lock', 'Cargo.toml');
  return {
    commit: git('rev-parse', 'HEAD'),
    source_dirty: Boolean(dirty),
    shell_version: shell,
    content_version: manifest.content_version,
    schema_version: manifest.schema_version,
    corpus_tag: manifest.corpus_tag ?? null,
    entry_count: manifest.entry_count,
    dist: path.relative(root, dist) || 'dist',
  };
}

/**
 * Wait until the page has stopped changing: the ready selector is present, fonts have loaded, two
 * frames have painted, and no finite animation or transition is still running. Returns the
 * evidence, which goes into the manifest as the "settled state".
 */
export async function settle(page, readySelector) {
  if (readySelector) await page.locator(readySelector).first().waitFor({ timeout: 60000 });
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
  await page.waitForFunction(() => document.getAnimations()
    .every(a => a.playState !== 'running' || a.effect?.getTiming().iterations === Infinity));
  return page.evaluate(selector => ({
    ready_selector: selector ?? null,
    fonts: document.fonts.status,
    running_animations: document.getAnimations().filter(a => a.playState === 'running').length,
    url_hash: location.hash,
    title: document.querySelector('h1')?.textContent?.trim() ?? null,
  }), readySelector ?? null);
}

/** What the browser says about the masthead, scroll and focus, measured right before the shot. */
export function probeMasthead(page) {
  return page.evaluate(() => {
    const describe = el => el
      ? `${el.tagName.toLowerCase()}${el.id ? '#' + el.id : ''}${typeof el.className === 'string' && el.className ? '.' + el.className.trim().split(/\s+/).join('.') : ''}`
      : null;
    const header = document.querySelector('header.app-header');
    const rect = header?.getBoundingClientRect();
    const style = header && getComputedStyle(header);
    const brand = header?.querySelector('.brand')?.getBoundingClientRect();
    const hit = rect ? document.elementFromPoint(rect.x + rect.width / 2, Math.max(1, rect.y + rect.height / 2)) : null;
    const scroller = document.scrollingElement;
    return {
      scroll_y: Math.round(scrollY * 100) / 100,
      scrolling_element: describe(scroller),
      document_height: scroller.scrollHeight,
      inner_height: innerHeight,
      visual_viewport_offset_top: visualViewport?.offsetTop ?? null,
      focus: describe(document.activeElement),
      focus_in_header: Boolean(header?.contains(document.activeElement)),
      header: rect ? { x: rect.x, y: rect.y, width: rect.width, height: rect.height } : null,
      header_style: style ? {
        position: style.position, top: style.top, z_index: style.zIndex, opacity: style.opacity,
        visibility: style.visibility, display: style.display, transform: style.transform,
        background: style.backgroundColor, border_bottom_colour: style.borderBottomColor,
      } : null,
      brand: brand ? { x: brand.x, y: brand.y, width: brand.width, height: brand.height } : null,
      hit_at_header_centre: describe(hit),
      hit_is_header: Boolean(hit && header?.contains(hit)),
    };
  });
}

/**
 * Read the saved PNG back in a blank page and ask whether it shows the masthead. The check
 * looks at the frame, not at the DOM: the header band must be dominated by the header's own
 * background, its bottom border must run unbroken across the width, and the brand must have
 * ink on it. A frame without a masthead fails at least one of these. `offsetY` is where the
 * masthead should be in the frame: 0 for a viewport shot; a full-page shot taken while scrolled
 * places a sticky header at the scroll offset, not at the top of the image.
 */
export async function probeFrame(probePage, png, masthead, { offsetY = 0, scan = false } = {}) {
  if (!masthead.header) return { masthead_in_frame: false, reason: 'no header element' };
  const { header, brand, header_style: style } = masthead;
  return probePage.evaluate(async ({ b64, header, brand, background, borderColour, offsetY, scan }) => {
    const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
    const bitmap = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    ctx.drawImage(bitmap, 0, 0);
    const scale = bitmap.width / header.width > 1.5 ? Math.round(bitmap.width / header.width) : 1;
    // Resolve a CSS colour through a canvas, so any colour syntax gives an RGBA triple.
    const resolve = css => {
      const swatch = new OffscreenCanvas(1, 1).getContext('2d', { willReadFrequently: true });
      swatch.fillStyle = css; swatch.fillRect(0, 0, 1, 1);
      return swatch.getImageData(0, 0, 1, 1).data;
    };
    const bg = resolve(background);
    const border = resolve(borderColour);
    const near = (p, q, tolerance = 3) => Math.abs(p[0] - q[0]) <= tolerance && Math.abs(p[1] - q[1]) <= tolerance && Math.abs(p[2] - q[2]) <= tolerance;
    const measure = at => {
      const pixel = (x, y) => ctx.getImageData(Math.round(x * scale), Math.round((y + at) * scale), 1, 1).data;
      const top = Math.floor(header.y);
      const bottom = Math.floor(header.y + header.height) - 1;
      let bandBg = 0, bandTotal = 0;
      for (let y = top; y < bottom - 1; y += 3) for (let x = 0; x < header.width; x += 7) {
        bandTotal++; if (near(pixel(x, y), bg)) bandBg++;
      }
      const colours = new Map();
      let borderTotal = 0;
      for (let x = 0; x < header.width; x += 5) {
        const p = pixel(x, bottom); const key = `${p[0]},${p[1]},${p[2]}`;
        colours.set(key, (colours.get(key) ?? 0) + 1); borderTotal++;
      }
      const [modal, modalCount] = [...colours.entries()].sort((p, q) => q[1] - p[1])[0];
      let ink = 0, inkTotal = 0;
      if (brand) for (let y = brand.y; y < brand.y + brand.height; y += 2) for (let x = brand.x; x < brand.x + brand.width; x += 2) {
        inkTotal++; if (!near(pixel(x, y), bg, 24)) ink++;
      }
      const result = {
        header_background_share: Math.round((bandBg / bandTotal) * 1000) / 1000,
        bottom_border_uniform_share: Math.round((modalCount / borderTotal) * 1000) / 1000,
        bottom_border_colour: modal,
        brand_ink_share: inkTotal ? Math.round((ink / inkTotal) * 1000) / 1000 : null,
      };
      result.masthead_in_frame = result.header_background_share >= 0.5
        && result.bottom_border_uniform_share >= 0.98 && (result.brand_ink_share ?? 0) > 0.02;
      return result;
    };
    const result = { frame: { width: bitmap.width, height: bitmap.height }, looked_at_y: offsetY, ...measure(offsetY) };
    if (scan && !result.masthead_in_frame) {
      // Where is it, if not where it should be? A cheap test first: a header's bottom border is a
      // full-width row of one colour, so only rows of the border colour get the full measurement.
      const height = Math.round(header.height);
      result.found_at_y = null;
      for (let at = 0; at + height <= bitmap.height / scale; at++) {
        const row = ctx.getImageData(0, Math.round((at + height - 1) * scale), bitmap.width, 1).data;
        let same = 0, total = 0;
        for (let x = 0; x < bitmap.width; x += 5) { total++; if (near(row.subarray(x * 4, x * 4 + 3), border)) same++; }
        if (same / total < 0.98) continue;
        if (measure(at).masthead_in_frame) { result.found_at_y = at; break; }
      }
    }
    return result;
  }, { b64: png.toString('base64'), header, brand, background: style.background, borderColour: style.border_bottom_colour, offsetY, scan });
}

/**
 * A copy of `source` for one context. A browser profile with the installed library is hundreds of
 * megabytes, so reflinks (btrfs, xfs, APFS) make the copy nearly free where the file system has
 * them. Same approach as `copyProfile` in `tests/fixtures.ts`.
 */
export function copyProfile(source) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'cast-iron-capture-profile-'));
  const target = path.join(base, 'profile');
  try {
    if (process.platform === 'linux') execFileSync('cp', ['-a', '--reflink=auto', source, target], { stdio: 'ignore' });
    else if (process.platform === 'darwin') execFileSync('cp', ['-c', '-R', source, target], { stdio: 'ignore' });
    else fs.cpSync(source, target, { recursive: true });
  } catch {
    fs.rmSync(target, { recursive: true, force: true });
    fs.cpSync(source, target, { recursive: true, verbatimSymlinks: true });
  }
  return { dir: target, cleanup: () => fs.rmSync(base, { recursive: true, force: true }) };
}

/** Click Download on the installer and wait for Home. Installs the library into the context. */
export async function installLibrary(page) {
  await page.goto('./');
  await page.getByRole('button', { name: 'Download', exact: true }).click();
  await page.locator('#home-title').waitFor({ timeout: 180000 });
}

/** Wait until a service worker controls the page, so the shell and library are offline-ready. */
export async function waitControlled(page) {
  await page.waitForFunction(() => Boolean(navigator.serviceWorker?.controller), null, { timeout: 60000 });
}

/** The viewport of a context under capture, named for the manifest. */
export const viewportName = size => (size.width <= 600 ? 'phone' : 'desktop');

/**
 * Scroll an element to the middle of the viewport and check that the reader can see it. A state
 * whose subject sits below the first screen is not shown by a shot from the top (#448), so the
 * scroll is part of the evidence and every check here throws rather than saving a frame that
 * does not show the subject.
 */
export async function scrollToTarget(page, { selector, text = [] }) {
  const target = page.locator(selector).first();
  await target.waitFor();
  await target.evaluate(el => el.scrollIntoView({ block: 'center', inline: 'nearest' }));
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const found = await target.evaluate((el, expected) => {
    const box = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    const header = document.querySelector('header.app-header')?.getBoundingClientRect();
    const hit = box.width > 0 && box.height > 0
      ? document.elementFromPoint(box.x + box.width / 2, box.y + Math.min(box.height / 2, 24))
      : null;
    const textContent = (el.innerText || el.textContent || '').replace(/\s+/g, ' ').trim();
    return {
      selector: expected.selector,
      bounds: { x: box.x, y: box.y, width: box.width, height: box.height },
      viewport: { width: innerWidth, height: innerHeight },
      scroll_y: Math.round(scrollY * 100) / 100,
      fully_in_viewport: box.top >= 0 && box.bottom <= innerHeight && box.left >= 0 && box.right <= innerWidth,
      clear_of_header: !header || box.top >= header.bottom - 1,
      visible_style: style.visibility === 'visible' && style.display !== 'none' && Number(style.opacity) > 0.99,
      hit_is_inside_target: Boolean(hit && el.contains(hit)),
      text: textContent,
      missing_text: expected.text.filter(part => !textContent.includes(part)),
    };
  }, { selector, text });
  if (!found.fully_in_viewport) throw new Error(`${selector} is not fully in the viewport after scrolling: ${JSON.stringify(found.bounds)} in ${JSON.stringify(found.viewport)}`);
  if (!found.clear_of_header) throw new Error(`${selector} is under the sticky header after scrolling`);
  if (!found.visible_style) throw new Error(`${selector} is not visibly rendered`);
  if (!found.hit_is_inside_target) throw new Error(`${selector} is covered at its top centre by another element`);
  if (found.missing_text.length) throw new Error(`${selector} is missing text: ${JSON.stringify(found.missing_text)}`);
  return found;
}

/**
 * Settle, probe, shoot, and read the frame back. `fullPage` shots scroll to the top first:
 * Chromium places a sticky header at the scroll offset of a full-page frame, so a shot taken while
 * scrolled has no masthead at the top of the image (see the capture evidence record, #448). A
 * viewport shot always carries the masthead and is the default.
 *
 * Returns the provenance record for the manifest. `masthead_in_frame` false on a viewport or
 * full-page shot is a warning the caller must show.
 *
 * `scrollTo` names the subject of a state that does not fit on the first screen: the element is
 * scrolled to the middle, checked by `scrollToTarget`, and its bounds and text go into the record.
 */
export async function shoot(page, probePage, { file, readySelector, fullPage = false, resetScroll = false, scrollTo = null }) {
  const settled = await settle(page, readySelector);
  const scrollBeforeReset = await page.evaluate(() => scrollY);
  if (fullPage || resetScroll) {
    await page.evaluate(() => scrollTo(0, 0));
    await settle(page, readySelector);
  }
  const scrolled = scrollTo ? await scrollToTarget(page, scrollTo) : null;
  if (scrolled) await settle(page, readySelector);
  const masthead = await probeMasthead(page);
  const png = await page.screenshot({ path: file, fullPage });
  const frame = await probeFrame(probePage, png, masthead, { offsetY: fullPage ? masthead.scroll_y : 0 });
  return {
    file: path.basename(file),
    capture_mode: fullPage ? 'full-page' : 'viewport',
    ...(fullPage || resetScroll ? { scroll_before_reset: Math.round(scrollBeforeReset) } : {}),
    ...(scrolled ? { scroll_to: scrolled } : {}),
    settled,
    masthead,
    frame,
    ...(frame.masthead_in_frame ? {} : { warning: 'the saved frame does not show the masthead' }),
  };
}

/** Facts about a context's page that a reviewer needs next to the image. */
export function pageFacts(page, browser, { engine, simulation = null }) {
  return page.evaluate(() => ({
    theme_applied: matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light',
    data_theme: document.documentElement.getAttribute('data-theme'),
    device_pixel_ratio: devicePixelRatio,
    inner_size: { width: innerWidth, height: innerHeight },
    user_agent: navigator.userAgent,
  })).then(facts => ({ engine, browser_version: browser.version(), ...(simulation ? { simulation } : {}), ...facts }));
}
