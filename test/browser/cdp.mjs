// Tiny Chrome DevTools Protocol client for the browser tests. No dependencies: it starts
// Chrome headless and talks CDP over the WebSocket built into Node >= 22.
// Pattern: agent-board's test/cdp.mjs (start retries, error collector, required switch).
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const CFT = {
  darwin: [
    ['chrome-mac-arm64', 'Google Chrome for Testing.app', 'Contents', 'MacOS', 'Google Chrome for Testing'],
    ['chrome-mac-x64', 'Google Chrome for Testing.app', 'Contents', 'MacOS', 'Google Chrome for Testing'],
  ],
  linux: [['chrome-linux64', 'chrome']],
  win32: [['chrome-win64', 'chrome.exe']],
};
const SYSTEM = {
  linux: ['/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/usr/bin/chromium', '/usr/bin/chromium-browser'],
};

/** CHROME_PATH, else the newest Chrome for Testing in the puppeteer cache, else a system Chrome on Linux; null if none. */
export function findChrome() {
  const env = process.env.CHROME_PATH;
  if (env) return existsSync(env) ? env : null;
  const cache = path.join(os.homedir(), '.cache', 'puppeteer', 'chrome');
  if (existsSync(cache)) {
    const versions = readdirSync(cache).sort((a, b) => b.localeCompare(a, 'en', { numeric: true }));
    for (const v of versions) {
      for (const parts of CFT[process.platform] ?? []) {
        const p = path.join(cache, v, ...parts);
        if (existsSync(p)) return p;
      }
    }
  }
  return (SYSTEM[process.platform] ?? []).find((p) => existsSync(p)) ?? null;
}

/** Polls fn() until it returns something truthy; throws after `timeout` ms. Never a fixed sleep. */
export async function waitFor(fn, { timeout = 15000, interval = 50, what = 'condition' } = {}) {
  const end = Date.now() + timeout;
  let last;
  for (;;) {
    try {
      last = await fn();
      if (last) return last;
    } catch (err) {
      last = err;
    }
    if (Date.now() > end) throw new Error(`timed out waiting for ${what} (last: ${last instanceof Error ? last.message : JSON.stringify(last)})`);
    await sleep(interval);
  }
}

/**
 * On a Mac only one Chrome for Testing may run at a time (shared machine, 8 GB):
 * wait until no other one is running. `pgrep -x` matches the exact process name only.
 */
export async function waitForFreeChrome(chromePath, { timeout = 10 * 60 * 1000 } = {}) {
  if (process.platform !== 'darwin' || process.env.CI) return;
  const name = path.basename(chromePath);
  const busy = () => spawnSync('pgrep', ['-x', name]).status === 0;
  if (!busy()) return;
  console.log(`# waiting for another "${name}" to finish`);
  await waitFor(() => !busy(), { timeout, interval: 1000, what: `no other ${name}` });
}

/** Starts Chrome headless with its own temporary profile; port 0 lets Chrome pick and report it. */
export async function launchChrome(chromePath) {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'stapelwerk-chrome-'));
  const args = [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${dir}`, '--no-first-run',
    '--no-default-browser-check', '--disable-extensions', '--disable-background-networking',
    '--disable-component-update', '--disable-sync', '--mute-audio', '--hide-scrollbars',
    '--window-size=1280,800', 'about:blank',
  ];
  // GitHub's Ubuntu runners block Chrome's sandbox via AppArmor; only there is it switched off.
  if (process.platform === 'linux' && process.env.CI) args.unshift('--no-sandbox');
  const proc = spawn(chromePath, args, { stdio: 'ignore' });
  let exited = false;
  proc.on('exit', () => { exited = true; });
  const close = async () => {
    if (!exited) {
      proc.kill('SIGTERM');
      await waitFor(() => exited, { timeout: 5000, what: 'Chrome to exit' }).catch(() => {
        proc.kill('SIGKILL');
        return waitFor(() => exited, { timeout: 5000, what: 'Chrome to exit after SIGKILL' }).catch(() => {});
      });
    }
    for (let i = 0; i < 10; i++) {
      try { rmSync(dir, { recursive: true, force: true }); break; } catch { await sleep(100); }
    }
  };
  try {
    const port = await waitFor(() => {
      if (exited) throw new Error('Chrome exited right away');
      const f = path.join(dir, 'DevToolsActivePort');
      return existsSync(f) && Number(readFileSync(f, 'utf8').split('\n')[0]);
    }, { timeout: 20000, what: 'Chrome debugging port' });
    return { port, close, pid: proc.pid };
  } catch (err) {
    await close();
    throw err;
  }
}

/** Starts Chrome with retries: slow CI runners sometimes report the port late. */
export async function startChrome(chromePath, attempts = 3) {
  await waitForFreeChrome(chromePath);
  for (let i = 1; ; i++) {
    try {
      return await launchChrome(chromePath);
    } catch (err) {
      if (i >= attempts) throw err;
      console.warn(`# Chrome start ${i}/${attempts} failed (${err.message}), retrying`);
    }
  }
}

function connect(url) {
  const ws = new WebSocket(url);
  let nextId = 0;
  const pending = new Map();
  const listeners = [];
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && pending.has(msg.id)) {
      const { ok, fail, method } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) fail(new Error(`CDP ${method}: ${msg.error.message}`));
      else ok(msg.result);
    } else if (msg.method) {
      for (const [name, fn] of listeners) if (name === msg.method) fn(msg.params, msg.sessionId);
    }
  };
  const opened = new Promise((ok, fail) => {
    ws.onopen = ok;
    ws.onerror = () => fail(new Error('CDP connection failed'));
  });
  const send = (method, params = {}, sessionId) => new Promise((ok, fail) => {
    const id = ++nextId;
    pending.set(id, { ok, fail, method });
    ws.send(JSON.stringify(sessionId ? { id, method, params, sessionId } : { id, method, params }));
    setTimeout(() => { if (pending.delete(id)) fail(new Error(`CDP ${method}: no answer`)); }, 30000).unref();
  });
  const on = (name, fn) => listeners.push([name, fn]);
  const close = () => { try { ws.close(); } catch { /* already closed */ } };
  return { opened, send, on, close };
}

/**
 * Opens a tab in a fresh browser context (own storage, caches and service workers),
 * so every test starts from a clean slate. Returns {send, on, close, targetId}.
 */
export async function openPage(port) {
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const browser = connect(version.webSocketDebuggerUrl);
  await browser.opened;
  const { browserContextId } = await browser.send('Target.createBrowserContext', { disposeOnDetach: true });
  const { targetId } = await browser.send('Target.createTarget', { url: 'about:blank', browserContextId });
  const page = connect(`ws://127.0.0.1:${port}/devtools/page/${targetId}`);
  await page.opened;
  const close = async () => {
    page.close();
    await browser.send('Target.closeTarget', { targetId }).catch(() => {});
    await browser.send('Target.disposeBrowserContext', { browserContextId }).catch(() => {});
    browser.close();
  };
  return { send: page.send, on: page.on, close, targetId, browser };
}
