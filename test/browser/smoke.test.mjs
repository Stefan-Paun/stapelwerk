// Loading: no console errors, no CSP violation, no request leaving 127.0.0.1; the English
// shell and the language switch that keeps the hash.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { openApp, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser();
const T = { skip, timeout: 60000 };

async function assertClean(app) {
  assert.deepEqual(app.errors, [], 'console errors');
  assert.deepEqual(await app.csp(), [], 'CSP violations');
  assert.deepEqual(app.foreignRequests(), [], 'requests outside the test server');
}

test('German start page loads cleanly with the chapter plan and the real core', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/');
  try {
    await app.until(`document.querySelectorAll('.station').length === 30`, '30 stations');
    assert.equal(await app.evaluate('document.documentElement.lang'), 'de');
    assert.equal(await app.evaluate(`document.querySelector('.mainnav a[data-route="sandbox"]').textContent`), 'Sandkasten');
    // Fonts, styles and the wasm module all came from the server.
    await app.until(`document.fonts.status === 'loaded'`, 'fonts');
    assert.ok(app.requests.some((u) => u.endsWith('/wasm/stapelwerk.wasm')));
    assert.ok(app.requests.some((u) => u.endsWith('/levels/index.json')));
    await app.goto('/stapelwerk/#/level/2-2-bis-zur-null');
    await app.until(`!!document.querySelector('.ws .ed-text')`, 'workspace');
    await app.goto('/stapelwerk/#/manual');
    await app.goto('/stapelwerk/#/about');
    await app.goto('/stapelwerk/#/sandbox');
    await app.until(`!!document.querySelector('.sandbox-panel')`, 'sandbox');
    await assertClean(app);
  } finally {
    await app.close();
  }
});

test('/en/: lang=en, English texts, language switch keeps the hash both ways', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/en/#/level/2-2-bis-zur-null');
  try {
    await app.until(`!!document.querySelector('.ws-title')`, 'workspace');
    assert.equal(await app.evaluate('document.documentElement.lang'), 'en');
    assert.equal(await app.evaluate(`document.querySelector('.ws-title').textContent.trim()`), '2.2Until Zero');
    assert.equal(await app.evaluate(`document.querySelector('.mainnav a[data-route="manual"]').textContent`), 'Manual');
    assert.match(await app.evaluate(`document.querySelector('.dbg-primary').textContent`), /Assemble/);
    assert.match(await app.evaluate('document.title'), /Until Zero · Stapelwerk/);
    const link = await app.evaluate(`(() => { const a = document.querySelector('.lang-link'); return { href: a.href, hreflang: a.hreflang, text: a.textContent }; })()`);
    assert.equal(link.href, app.url('/stapelwerk/#/level/2-2-bis-zur-null'));
    assert.equal(link.hreflang, 'de');
    // Hash changes update the link.
    await app.evaluate(`location.hash = '#/manual'`);
    await app.until(`document.querySelector('.lang-link').href.endsWith('/stapelwerk/#/manual')`, 'link follows the hash');
    await app.evaluate(`location.hash = '#/level/2-2-bis-zur-null'`);
    await app.until(`document.querySelector('.lang-link').href.endsWith('#/level/2-2-bis-zur-null')`, 'link follows the hash');
    await app.click('.lang-link');
    await app.until(`document.documentElement.lang === 'de' && location.pathname === '/stapelwerk/' && !!document.querySelector('.ws-title')`, 'German page');
    assert.equal(await app.evaluate('location.hash'), '#/level/2-2-bis-zur-null');
    assert.equal(await app.evaluate(`document.querySelector('.ws-title').textContent.trim()`), '2.2Bis zur Null');
    assert.equal(await app.evaluate(`document.querySelector('.lang-link').href`), app.url('/stapelwerk/en/#/level/2-2-bis-zur-null'));
    await app.click('.lang-link');
    await app.until(`document.documentElement.lang === 'en' && location.pathname === '/stapelwerk/en/' && !!document.querySelector('.ws-title')`, 'English page again');
    assert.equal(await app.evaluate('location.hash'), '#/level/2-2-bis-zur-null');
    await assertClean(app);
  } finally {
    await app.close();
  }
});
