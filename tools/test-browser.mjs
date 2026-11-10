// npm run test:browser: runs test/browser/*.test.mjs (own CDP client, no npm packages).
// Needs a built dist/ (npm run build). Chrome comes from CHROME_PATH or Chrome for Testing in
// the puppeteer cache; without one the tests are skipped clearly, unless
// STAPELWERK_REQUIRE_BROWSER=1. One Chrome serves all test files (its port is handed over in
// STAPELWERK_CHROME_PORT) and is closed at the end, whatever happens.
import { existsSync } from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { ROOT, findFiles } from './lib.mjs';
import { findChrome, startChrome } from '../test/browser/cdp.mjs';

const files = findFiles(path.join(ROOT, 'test', 'browser'), '.test.mjs');
if (files.length === 0) {
  console.log('test:browser: no browser tests, skipped');
  process.exit(0);
}
if (!existsSync(path.join(ROOT, 'dist', 'index.html'))) {
  console.error('test:browser: dist/ is not built, run `npm run build` first');
  process.exit(1);
}

const chromePath = findChrome();
let chrome = null;
if (chromePath && typeof WebSocket === 'function') {
  chrome = await startChrome(chromePath);
  console.log(`test:browser: Chrome ${path.basename(chromePath)} on port ${chrome.port}`);
} else if (process.env.STAPELWERK_REQUIRE_BROWSER !== '1') {
  console.log('test:browser: no Chrome found (set CHROME_PATH), browser tests will be skipped');
}

const env = { ...process.env };
if (chrome) env.STAPELWERK_CHROME_PORT = String(chrome.port);
const child = spawn(process.execPath, ['--test', '--test-concurrency=1', ...files], { cwd: ROOT, env, stdio: 'inherit' });
const stop = () => child.kill('SIGTERM');
process.on('SIGINT', stop);
process.on('SIGTERM', stop);
const code = await new Promise((resolve) => child.on('exit', (c, signal) => resolve(c ?? (signal ? 1 : 0))));
await chrome?.close();
process.exit(code);
