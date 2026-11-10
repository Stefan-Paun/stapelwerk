// The site does not depend on living under /stapelwerk/: mounted at /a/b/c/ it loads,
// plays a level and switches language, and every request stays below that path.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { WS_STATE, openApp, referenceOf, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser({ base: '/a/b/c/' });

test('site works under /a/b/c/ (German and English)', { skip, timeout: 60000 }, async () => {
  const app = await openApp(ctx, '/a/b/c/#/');
  try {
    await app.until(`document.querySelectorAll('.station').length === 30`, 'plan');
    await app.goto('/a/b/c/#/level/2-2-bis-zur-null');
    await app.setCode(referenceOf('2-2-bis-zur-null.lvl'));
    await app.click('.dbg-primary');
    await app.until(`(${WS_STATE}).result`, 'solved');
    await app.click('.lang-link');
    await app.until(`location.pathname === '/a/b/c/en/' && document.documentElement.lang === 'en' && !!document.querySelector('.ws-title')`, 'English page');
    assert.equal(await app.evaluate('location.hash'), '#/level/2-2-bis-zur-null');
    const outside = app.requests.filter((u) => !u.startsWith(`${ctx.server.origin}/a/b/c/`) && !/^(data|blob):/.test(u));
    assert.deepEqual(outside, []);
    assert.deepEqual(app.errors, []);
    assert.deepEqual(await app.csp(), []);
  } finally {
    await app.close();
  }
});
