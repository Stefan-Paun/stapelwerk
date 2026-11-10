// Shared set-up for the browser tests: finds Chrome, starts tools/serve.mjs on a free port
// and Chrome headless once per test file (both stopped in after()), and opens pages that
// record errors, CSP violations and every request. Waits always poll a condition.
import { after, before } from 'node:test';
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { findChrome, openPage, startChrome, waitFor } from './cdp.mjs';

export { waitFor };
export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
export const CHROME = findChrome();
export const skip = typeof WebSocket !== 'function' ? 'Node without a built-in WebSocket (needs Node >= 22)'
  : !CHROME ? 'no Chrome found (set CHROME_PATH or install Chrome for Testing via puppeteer)' : false;
// In CI a missing Chrome must fail instead of being skipped quietly.
if (skip && process.env.STAPELWERK_REQUIRE_BROWSER === '1') throw new Error(`browser tests required, but: ${skip}`);

/** Starts tools/serve.mjs on a free port of 127.0.0.1; resolves to {origin, url, stop}. */
export async function startServer({ base = '/stapelwerk/' } = {}) {
  const proc = spawn(process.execPath, [path.join(ROOT, 'tools', 'serve.mjs')], {
    cwd: ROOT,
    env: { ...process.env, PORT: '0', HOST: '127.0.0.1', BASE: base },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  let exited = false;
  proc.on('exit', () => { exited = true; });
  let out = '';
  proc.stdout.setEncoding('utf8');
  proc.stdout.on('data', (d) => { out += d; });
  const url = await waitFor(() => {
    if (exited) throw new Error('serve.mjs exited (is dist/ built? run npm run build)');
    return out.match(/serve: (http:\/\/127\.0\.0\.1:\d+\/\S*)/)?.[1];
  }, { timeout: 10000, what: 'server start' });
  const stop = async () => {
    if (exited) return;
    proc.kill('SIGTERM');
    await waitFor(() => exited, { timeout: 5000, what: 'server exit' }).catch(() => proc.kill('SIGKILL'));
  };
  return { origin: new URL(url).origin, url, stop, get running() { return !exited; } };
}

/** Registers before/after hooks for one server and one Chrome; returns the live context. */
export function useBrowser({ base } = {}) {
  const ctx = { server: null, chrome: null };
  before(async () => {
    if (skip) return;
    ctx.server = await startServer({ base });
    // tools/test-browser.mjs starts one Chrome for all files; a single file run starts its own.
    const shared = Number(process.env.STAPELWERK_CHROME_PORT);
    ctx.chrome = shared ? { port: shared, close: async () => {} } : await startChrome(CHROME);
  });
  after(async () => {
    await ctx.chrome?.close();
    await ctx.server?.stop();
  });
  return ctx;
}

const KEYS = {
  Enter: { code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' },
  Tab: { code: 'Tab', windowsVirtualKeyCode: 9 },
  Escape: { code: 'Escape', windowsVirtualKeyCode: 27 },
  ' ': { code: 'Space', windowsVirtualKeyCode: 32, text: ' ' },
  ArrowDown: { code: 'ArrowDown', windowsVirtualKeyCode: 40 },
  ArrowUp: { code: 'ArrowUp', windowsVirtualKeyCode: 38 },
  End: { code: 'End', windowsVirtualKeyCode: 35 },
  Home: { code: 'Home', windowsVirtualKeyCode: 36 },
  '?': { code: 'Slash', windowsVirtualKeyCode: 191, text: '?' },
};
for (const c of 'abcdefghijklmnopqrstuvwxyz') KEYS[c] = { code: `Key${c.toUpperCase()}`, windowsVirtualKeyCode: c.toUpperCase().charCodeAt(0), text: c };
for (const c of '0123456789') KEYS[c] = { code: `Digit${c}`, windowsVirtualKeyCode: c.charCodeAt(0), text: c };
const MOD = { alt: 1, ctrl: 2, meta: 4, shift: 8 };

/**
 * Opens `pathname` (relative to the server origin) in a fresh browser context.
 * Options: viewport {width, height}, media [{name, value}], storage {key: value} (written
 * before the app starts), offline handled by the caller.
 */
export async function openApp(ctx, pathname, { viewport = { width: 1280, height: 800 }, media = null, storage = null } = {}) {
  const page = await openPage(ctx.chrome.port);
  const errors = [];
  const requests = [];
  page.on('Runtime.exceptionThrown', (p) => errors.push(`exception: ${p.exceptionDetails.exception?.description ?? p.exceptionDetails.text}`));
  page.on('Runtime.consoleAPICalled', (p) => {
    if (p.type === 'error' || p.type === 'assert') errors.push(`console.${p.type}: ${p.args.map((a) => a.value ?? a.description).join(' ')}`);
  });
  page.on('Log.entryAdded', (p) => {
    if (p.entry.level === 'error' || p.entry.source === 'security') errors.push(`log(${p.entry.source}): ${p.entry.text} ${p.entry.url ?? ''}`);
  });
  page.on('Network.requestWillBeSent', (p) => requests.push(p.request.url));
  await page.send('Runtime.enable');
  await page.send('Log.enable');
  await page.send('Network.enable');
  await page.send('Page.enable');
  await page.send('Emulation.setDeviceMetricsOverride', { width: viewport.width, height: viewport.height, deviceScaleFactor: 1, mobile: viewport.width < 600 });
  if (media) await page.send('Emulation.setEmulatedMedia', { features: media });
  // Records CSP violations from inside the page (the listener is installed by CDP, not by the page).
  await page.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `window.__csp = []; document.addEventListener('securitypolicyviolation', (e) => window.__csp.push(e.violatedDirective + ' ' + e.blockedURI));`,
  });
  if (storage) {
    const entries = JSON.stringify(Object.entries(storage));
    await page.send('Page.addScriptToEvaluateOnNewDocument', {
      source: `if (!sessionStorage.getItem('__seeded')) { sessionStorage.setItem('__seeded', '1'); for (const [k, v] of ${entries}) localStorage.setItem(k, v); }`,
    });
  }

  const evaluate = async (expression) => {
    const r = await page.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true, userGesture: true });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
    return r.result.value;
  };
  const until = (expression, what = expression, timeout = 15000) => waitFor(() => evaluate(expression), { what, timeout });
  const ready = () => until(`document.readyState === 'complete' && !!document.querySelector('#main > *:not(noscript)')`, 'app rendered', 20000);

  const app = {
    ...page,
    errors,
    requests,
    evaluate,
    until,
    url: (p) => new URL(p, ctx.server.origin).href,
    async goto(p) {
      const url = new URL(p, ctx.server.origin).href;
      // A hash-only change re-renders in place: mark the old view and wait for a new one.
      const same = await evaluate(`location.href === ${JSON.stringify(url)}`).catch(() => true);
      if (!same) await evaluate(`document.querySelector('#main > *') && (document.querySelector('#main > *').dataset.stale = '1')`).catch(() => {});
      await page.send('Page.navigate', { url });
      await until(`location.href === ${JSON.stringify(url)}`, `navigation to ${url}`);
      await until(`!!document.querySelector('#main > *:not(noscript):not([data-stale])')`, `view for ${url}`, 20000);
      await ready();
    },
    async reload() {
      await evaluate(`document.documentElement.dataset.reloadMark = '1'`);
      await page.send('Page.reload', { ignoreCache: false });
      await until(`!document.documentElement.dataset.reloadMark`, 'page unloaded');
      await ready();
    },
    async press(key, modifiers = []) {
      const k = KEYS[key];
      if (!k) throw new Error(`unknown key ${key}`);
      const mods = modifiers.reduce((m, name) => m | MOD[name], 0);
      const text = mods & (MOD.ctrl | MOD.meta | MOD.alt) ? undefined : k.text;
      await page.send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', key, code: k.code, windowsVirtualKeyCode: k.windowsVirtualKeyCode, modifiers: mods, text, unmodifiedText: text });
      await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code: k.code, windowsVirtualKeyCode: k.windowsVirtualKeyCode, modifiers: mods });
    },
    insertText: (text) => page.send('Input.insertText', { text }),
    /** A real mouse click in the middle of the first element matching `selector`. */
    async click(selector) {
      const box = await until(`(() => {
        const e = document.querySelector(${JSON.stringify(selector)});
        if (!e) return null;
        e.scrollIntoView({ block: 'center', inline: 'center' });
        const r = e.getBoundingClientRect();
        return r.width > 0 && r.height > 0 ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : null;
      })()`, `clickable ${selector}`);
      for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
        await page.send('Input.dispatchMouseEvent', { type, x: box.x, y: box.y, button: 'left', clickCount: type === 'mouseMoved' ? 0 : 1 });
      }
    },
    /** Replaces the editor text by typing it (select all, insert), like a user pasting. */
    async setCode(code) {
      await until(`!!document.querySelector('.ed-text')`, 'editor');
      await evaluate(`(() => { const a = document.querySelector('.ed-text'); a.focus(); a.select(); })()`);
      await page.send('Input.insertText', { text: code });
      const now = await evaluate(`document.querySelector('.ed-text').value`);
      if (now !== code) throw new Error('editor text differs after typing');
    },
    csp: () => evaluate('window.__csp'),
    /** Requests that did not go to the test server (data: and blob: URLs are local by nature). */
    foreignRequests: () => requests.filter((u) => !u.startsWith(`${ctx.server.origin}/`) && !/^(data|blob):/.test(u)),
  };
  await app.goto(pathname);
  return app;
}

/** The [reference] section of a level source file. */
export function referenceOf(file) {
  return section(file, 'reference');
}
/** The [start] section of a level source file. */
export function startOf(file) {
  return section(file, 'start');
}
function section(file, name) {
  const text = readFileSync(path.join(ROOT, 'levels', file), 'utf8');
  const lines = text.split('\n');
  const start = lines.findIndex((l) => l.trim() === `[${name}]`);
  if (start < 0) throw new Error(`${file}: no [${name}]`);
  const out = [];
  for (const l of lines.slice(start + 1)) {
    if (/^\[[a-z]/.test(l)) break;
    out.push(l);
  }
  return `${out.join('\n').replace(/\s+$/, '')}\n`;
}

/** Workspace state in one call: status chip kind, steps, current line, result shown. */
export const WS_STATE = `(() => {
  const chip = document.querySelector('.status-chip');
  const range = document.querySelector('#timeline');
  const cur = document.querySelector('.ed-gutter .gl.is-current');
  return {
    kind: chip?.dataset.kind ?? null,
    steps: range ? Number(range.value) : null,
    line: cur ? Number(cur.dataset.line) : null,
    running: document.querySelector('[data-action="runPause"]')?.getAttribute('aria-pressed') === 'true',
    result: !!document.querySelector('#h-result') && !document.querySelector('#h-result').closest('[hidden]'),
    notice: document.querySelector('.notice:not([hidden])')?.textContent ?? '',
  };
})()`;
