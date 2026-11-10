// Sandbox: an endless loop can be paused and the page stays usable; HTML in program output,
// comments and labels stays text; a share link opens the code without running it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { WS_STATE, openApp, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser();
const T = { skip, timeout: 90000 };
const frames = (n) => `new Promise((r) => { let i = 0; const f = () => (++i > ${n} ? r(true) : requestAnimationFrame(f)); f(); })`;

test('endless loop: the pause button stops it, the page stays responsive', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/sandbox');
  try {
    await app.setCode('; endlos\nloop:\n        add r1, 1\n        jmp loop\n');
    await app.click('.dbg-primary');
    await app.until(`/Assembliert/.test((${WS_STATE}).notice)`, 'assembled');
    await app.click('[data-action="runPause"]');
    await app.until(`(${WS_STATE}).running && (${WS_STATE}).steps > 100000`, 'running');
    // The page answers while the machine runs (time slices per frame).
    const t0 = Date.now();
    await app.evaluate(frames(2));
    assert.ok(Date.now() - t0 < 2000, 'frames keep coming while running');
    await app.click('[data-action="runPause"]');
    await app.until(`!(${WS_STATE}).running && (${WS_STATE}).kind === 'paused'`, 'paused');
    const steps = (await app.evaluate(WS_STATE)).steps;
    await app.evaluate(frames(10));
    assert.equal((await app.evaluate(WS_STATE)).steps, steps, 'no more steps after pause');
    // Still usable: a step works and navigation works.
    await app.press('s');
    await app.until(`(${WS_STATE}).steps === ${steps + 1}`, 'one more step');
    await app.click('.mainnav a[data-route="manual"]');
    await app.until(`location.hash === '#/manual' && !!document.querySelector('#main h1')`, 'manual opened');
    assert.deepEqual(app.errors, []);
  } finally {
    await app.close();
  }
});

test('XSS probe: HTML in output, comments and labels stays text', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/sandbox');
  const injected = `[...document.querySelectorAll('#main img, #main script, #main iframe, #main svg [onload]')].length`;
  try {
    await app.setCode([
      '; <img src=x onerror="window.__xss=1"><script>window.__xss=2</script>',
      '        .data',
      's:      .string "<img src=x onerror=window.__xss=3><b>fett</b>"',
      '        .text',
      '        li r0, s',
      '        sys WRITE_STR',
      '        halt',
      '',
    ].join('\n'));
    await app.click('.dbg-primary');
    await app.until(`/Assembliert/.test((${WS_STATE}).notice)`, 'assembled');
    await app.press('Escape');
    await app.press('l');
    await app.until(`(${WS_STATE}).kind === 'halted'`, 'halted');
    assert.equal(await app.evaluate(`document.querySelector('.tape-out').textContent`), '<img src=x onerror=window.__xss=3><b>fett</b>');
    assert.match(await app.evaluate(`document.querySelector('.ed-back').textContent`), /<script>window.__xss=2<\/script>/);
    assert.equal(await app.evaluate(injected), 0);
    assert.equal(await app.evaluate(`document.querySelectorAll('#main b').length`), 0);

    // A label made of HTML is an assembler error whose message shows the text as text.
    await app.setCode('<img src=x onerror=window.__xss=4>:\n        halt\n<svg onload=window.__xss=5>\n');
    await app.click('.dbg-primary');
    await app.until(`document.querySelectorAll('.ed-errors li').length > 0`, 'assembler errors');
    assert.equal(await app.evaluate(injected), 0);
    assert.equal(await app.evaluate('window.__xss'), undefined);
    assert.deepEqual(await app.csp(), []);
    assert.deepEqual(app.errors, []);
  } finally {
    await app.close();
  }
});

test('share link: opens the code in the sandbox and never runs it by itself', T, async () => {
  const code = '; geteilt\n        mov r0, 4711\n        sys WRITE_INT\n        halt\n';
  const app = await openApp(ctx, '/stapelwerk/#/sandbox');
  let link;
  try {
    await app.setCode(code);
    await app.click('.sandbox-panel > button.btn');
    link = await app.until(`(() => { const o = document.querySelector('.share-out'); return o && !o.hidden && o.value; })()`, 'share link');
    assert.ok(link.startsWith(app.url('/stapelwerk/#/sandbox?code=v1:')), link);
    assert.deepEqual(app.errors, []);
  } finally {
    await app.close();
  }
  const other = await openApp(ctx, new URL(link).pathname + new URL(link).hash);
  try {
    await other.until(`document.querySelector('.ed-text')?.value === ${JSON.stringify(code)}`, 'shared code in the editor');
    await other.evaluate(frames(20));
    const st = await other.evaluate(WS_STATE);
    assert.equal(st.kind, 'idle', 'not assembled, not running');
    assert.equal(st.steps, 0);
    assert.equal(st.running, false);
    assert.equal(await other.evaluate(`document.querySelectorAll('.tape-out .o-int').length`), 0, 'no output');
    // Running it is the user's choice.
    await other.click('.dbg-primary');
    await other.press('Escape');
    await other.press('l');
    await other.until(`(${WS_STATE}).kind === 'halted'`, 'halted');
    assert.equal(await other.evaluate(`document.querySelector('.tape-out .o-int').textContent`), '4711');
    // A damaged link is reported and falls back to the normal sandbox.
    await other.goto('/stapelwerk/#/sandbox?code=v1:%%%kaputt');
    await other.until(`/beschädigt/.test(document.getElementById('live').textContent)`, 'bad link announced');
    assert.deepEqual(other.errors, []);
  } finally {
    await other.close();
  }
});
