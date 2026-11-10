// Offline: with ?sw=1 the service worker installs and controls the page; with the server
// stopped and the network switched off, a reload still shows the page and a level works.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { WS_STATE, openApp, referenceOf, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser();

test('without ?sw=1 no service worker is registered locally', { skip, timeout: 30000 }, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/');
  try {
    await app.until(`document.querySelectorAll('.station').length === 30`, 'plan');
    assert.equal(await app.evaluate(`navigator.serviceWorker.getRegistrations().then((r) => r.length)`), 0);
  } finally {
    await app.close();
  }
});

test('service worker: precache, offline reload, level playable offline', { skip, timeout: 90000 }, async () => {
  const app = await openApp(ctx, '/stapelwerk/?sw=1#/');
  try {
    await app.until(`!!navigator.serviceWorker.controller && navigator.serviceWorker.controller.state === 'activated'`, 'controlling service worker', 30000);
    const caches = await app.evaluate(`caches.keys()`);
    assert.equal(caches.length, 1);
    assert.match(caches[0], /^stapelwerk-[0-9a-f]{12}$/);
    const cached = await app.evaluate(`caches.open(${JSON.stringify(caches[0])}).then((c) => c.keys()).then((k) => k.map((r) => new URL(r.url).pathname))`);
    for (const p of ['/stapelwerk/index.html', '/stapelwerk/en/index.html', '/stapelwerk/wasm/stapelwerk.wasm', '/stapelwerk/levels/2-2-bis-zur-null.lvl']) {
      assert.ok(cached.includes(p), `${p} cached`);
    }
    // A foreign cache of the same origin survives (only stapelwerk-… caches are touched).
    await app.evaluate(`caches.open('fremd-1').then((c) => c.put('/x', new Response('x')))`);

    // Now offline: stop the server and switch the network off.
    await ctx.server.stop();
    await app.send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
    assert.equal(await app.evaluate(`fetch('/stapelwerk/levels/index.json?nocache=' + Math.random(), { cache: 'no-store' }).then(() => 'online', () => 'offline')`), 'online',
      'the service worker answers even with a query string');
    await app.reload();
    await app.until(`document.querySelectorAll('.station').length === 30`, 'plan offline');
    await app.goto('/stapelwerk/?sw=1#/level/2-2-bis-zur-null');
    await app.setCode(referenceOf('2-2-bis-zur-null.lvl'));
    await app.click('.dbg-primary');
    await app.until(`(${WS_STATE}).result`, 'solved offline');
    // The English page is cached too.
    await app.goto('/stapelwerk/en/#/level/2-2-bis-zur-null');
    await app.until(`document.documentElement.lang === 'en' && document.querySelector('.ws-title')?.textContent.includes('Until Zero')`, 'English page offline');
    assert.ok((await app.evaluate(`caches.keys()`)).includes('fremd-1'));
    assert.deepEqual(app.errors, []);
    assert.deepEqual(await app.csp(), []);
  } finally {
    await app.close();
  }
});
