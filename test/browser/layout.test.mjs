// Layout and robustness: five viewports without horizontal page scrolling and with the
// debugger bar fully visible, reduced motion, damaged local storage.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { openApp, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser();
const T = { skip, timeout: 120000 };

const VIEWPORTS = [[390, 844], [768, 1024], [1024, 768], [1280, 720], [1366, 768], [1920, 1080]];

const NO_HSCROLL = `document.scrollingElement.scrollWidth <= window.innerWidth`;
// Elements that must be completely inside the viewport (at the top of the page).
const CUT = `(() => {
  const out = [];
  for (const e of document.querySelectorAll('.dbg .dbg-primary, .dbg .dbg-btn, .appbar .brand, .appbar .lang-link, .appbar .icon-btn')) {
    const r = e.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) { out.push(e.className + ': not rendered'); continue; }
    if (r.left < -0.5 || r.right > innerWidth + 0.5 || r.top < -0.5 || r.bottom > innerHeight + 0.5) out.push(e.className + ': ' + [r.left, r.top, r.right, r.bottom].map(Math.round).join(','));
    const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    if (!hit || !(e === hit || e.contains(hit))) out.push(e.className + ': covered by ' + (hit?.className || hit?.tagName));
  }
  return out;
})()`;
// Nothing in the head, the toolbar or the panes may stick out on the right: the workspace
// clips sideways overflow, so it would be cut off silently instead of scrolling.
const CLIPPED = `(() => {
  const out = [];
  for (const e of document.querySelectorAll('.ws-head *, .dbg *, .ws-grid .pane, .ws-grid .pane-head')) {
    const r = e.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) continue;
    if (r.right > innerWidth + 0.5) out.push((e.className || e.tagName) + ' right=' + Math.round(r.right));
  }
  return out.slice(0, 8);
})()`;
const WIDEST = `(() => {
  let worst = null;
  for (const e of document.querySelectorAll('body *')) {
    const r = e.getBoundingClientRect();
    if (r.right > innerWidth + 0.5 && (!worst || r.right > worst.right)) worst = { cls: e.className || e.tagName, right: Math.round(r.right) };
  }
  return worst;
})()`;

for (const [width, height] of VIEWPORTS) {
  test(`${width}×${height}: no horizontal scrolling, debugger bar visible`, T, async () => {
    const app = await openApp(ctx, '/stapelwerk/#/', { viewport: { width, height } });
    try {
      for (const hash of ['#/', '#/level/2-2-bis-zur-null', '#/level/1-2-erster-pixel', '#/sandbox', '#/manual', '#/about']) {
        await app.goto(`/stapelwerk/${hash}`);
        await app.until(`document.fonts.status === 'loaded'`, 'fonts');
        assert.ok(await app.evaluate(NO_HSCROLL), `${hash}: page scrolls sideways (${JSON.stringify(await app.evaluate(WIDEST))})`);
        if (hash.startsWith('#/level') || hash === '#/sandbox') {
          await app.evaluate('window.scrollTo(0, 0)');
          assert.deepEqual(await app.evaluate(CUT), [], `${hash}: cut or covered controls`);
          assert.deepEqual(await app.evaluate(CLIPPED), [], `${hash}: clipped on the right`);
          // After solving, the result is still reachable without sideways scrolling.
        }
      }
      // The workspace after assembling (errors, result) must not grow sideways either.
      await app.goto('/stapelwerk/#/level/2-2-bis-zur-null');
      await app.setCode('        mov r1, 0\n        bogus r9, 1\n        halt\n');
      await app.click('.dbg-primary');
      await app.until(`document.querySelectorAll('.ed-errors li').length > 0`, 'error list');
      assert.ok(await app.evaluate(NO_HSCROLL), `errors: page scrolls sideways (${JSON.stringify(await app.evaluate(WIDEST))})`);
      assert.deepEqual(app.errors, []);
    } finally {
      await app.close();
    }
  });
}

test('prefers-reduced-motion: no transitions or animations anywhere', T, async () => {
  const media = [{ name: 'prefers-reduced-motion', value: 'reduce' }];
  const moving = `(() => {
    const sec = (v) => Math.max(0, ...String(v).split(',').map((s) => parseFloat(s) * (s.trim().endsWith('ms') ? 0.001 : 1)).filter(Number.isFinite));
    const bad = [];
    for (const e of document.querySelectorAll('*')) {
      for (const pseudo of [null, '::before', '::after']) {
        const cs = getComputedStyle(e, pseudo);
        if (cs.transitionProperty !== 'none' && sec(cs.transitionDuration) > 0.01) bad.push((e.className || e.tagName) + (pseudo ?? '') + ' transition ' + cs.transitionDuration);
        if (cs.animationName !== 'none' && sec(cs.animationDuration) > 0.01) bad.push((e.className || e.tagName) + (pseudo ?? '') + ' animation ' + cs.animationName);
      }
    }
    if (getComputedStyle(document.documentElement).scrollBehavior === 'smooth') bad.push('html scroll-behavior smooth');
    return bad.slice(0, 12);
  })()`;
  const app = await openApp(ctx, '/stapelwerk/#/', { media });
  try {
    assert.equal(await app.evaluate(`matchMedia('(prefers-reduced-motion: reduce)').matches`), true);
    for (const hash of ['#/', '#/level/2-2-bis-zur-null', '#/sandbox']) {
      await app.goto(`/stapelwerk/${hash}`);
      assert.deepEqual(await app.evaluate(moving), [], hash);
    }
    // Also while something happens: a solved result with its stamp.
    await app.goto('/stapelwerk/#/level/0-2-ein-register-aendern');
    await app.setCode('        mov r0, 42\n        sys WRITE_INT\n        halt\n');
    await app.click('.dbg-primary');
    await app.until(`!!document.querySelector('.result')`, 'result');
    assert.deepEqual(await app.evaluate(moving), [], 'result view');
    assert.deepEqual(app.errors, []);
  } finally {
    await app.close();
  }
  // Sanity check: without the preference there is motion, so the check above can fail.
  const plain = await openApp(ctx, '/stapelwerk/#/level/2-2-bis-zur-null', { media: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
  try {
    assert.notDeepEqual(await plain.evaluate(moving), []);
  } finally {
    await plain.close();
  }
});

test('damaged localStorage: the page still starts and says so', T, async () => {
  const storage = {
    'stapelwerk:progress': '{"v":1,"levels":{"2-2-bis',
    'stapelwerk:theme': '"><img src=x onerror=alert(1)>',
    'stapelwerk:unknown': '\u0000￿',
  };
  const app = await openApp(ctx, '/stapelwerk/#/', { storage });
  try {
    await app.until(`document.querySelectorAll('.station').length === 30`, 'plan');
    await app.until(`/beschädigt/.test(document.getElementById('live').textContent)`, 'damage announced');
    assert.equal(await app.evaluate('document.documentElement.dataset.theme ?? null'), null, 'garbage theme ignored');
    await app.goto('/stapelwerk/#/level/2-2-bis-zur-null');
    await app.until(`document.querySelector('.ed-text')?.value.length > 0`, 'workspace with start code');
    assert.deepEqual(app.errors, []);
  } finally {
    await app.close();
  }
  // Valid JSON with wrong types inside is cleaned field by field.
  const typed = {
    'stapelwerk:progress': JSON.stringify({ v: 1, unlockAll: 'yes', levels: { '2-2-bis-zur-null': { solved: 'yes', code: 42, slots: 'x', best: { cycles: -1 } }, '../x': {} } }),
  };
  const app2 = await openApp(ctx, '/stapelwerk/#/level/2-2-bis-zur-null', { storage: typed });
  try {
    await app2.until(`document.querySelector('.ed-text')?.value.includes('Bis zur Null')`, 'start code instead of the bad code');
    await app2.goto('/stapelwerk/#/');
    await app2.until(`document.querySelectorAll('.station').length === 30`, 'plan');
    assert.equal(await app2.evaluate(`document.getElementById('unlock-all').checked`), false);
    assert.equal(await app2.evaluate(`document.querySelectorAll('.station.is-solved').length`), 0);
    assert.deepEqual(app2.errors, []);
  } finally {
    await app2.close();
  }
});
