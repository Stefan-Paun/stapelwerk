// Playing levels: solve 2.2 and keep the progress over a reload, solve the screen level 1.2,
// the budget message of 2.4's start code, and level 0.2 with the keyboard only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { WS_STATE, openApp, referenceOf, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser();
const T = { skip, timeout: 90000 };
const state = (app) => app.evaluate(WS_STATE);

async function assertClean(app) {
  assert.deepEqual(app.errors, [], 'console errors');
  assert.deepEqual(await app.csp(), [], 'CSP violations');
  assert.deepEqual(app.foreignRequests(), [], 'requests outside the test server');
}

test('level 2.2: reference solution solves it, result shows, progress survives a reload', T, async () => {
  const ref = referenceOf('2-2-bis-zur-null.lvl');
  const app = await openApp(ctx, '/stapelwerk/#/level/2-2-bis-zur-null');
  try {
    await app.setCode(ref);
    await app.click('.dbg-primary');
    await app.until(`(${WS_STATE}).result`, 'result view');
    const st = await state(app);
    assert.match(st.notice, /Gelöst/);
    assert.match(await app.evaluate(`document.querySelector('.status-chip').textContent`), /Tests 15\/15/);
    assert.equal(await app.evaluate(`document.activeElement.id`), 'h-result');
    assert.match(await app.evaluate(`document.querySelector('.result').textContent`), /Was du gerade gelernt hast|gelernt/);
    await app.until(`JSON.parse(localStorage.getItem('stapelwerk:progress')).levels['2-2-bis-zur-null']?.code === ${JSON.stringify(ref)}`, 'code saved');
    await app.reload();
    await app.until(`document.querySelector('.ed-text')?.value === ${JSON.stringify(ref)}`, 'saved code after reload');
    await app.goto('/stapelwerk/#/');
    await app.until(`document.querySelectorAll('.station').length === 30`, 'plan');
    // 2.2 is still locked (chapters 0 and 1 are open), but it is marked as solved.
    assert.equal(await app.evaluate(`[...document.querySelectorAll('.station')].find((s) => s.querySelector('.st-num').textContent === '2.2').classList.contains('is-solved')`), true);
    assert.match(await app.evaluate(`document.querySelector('.plan-count').textContent`), /^1\b/);
    await assertClean(app);
  } finally {
    await app.close();
  }
});

test('screen level 1.2: reference solution passes, screen shows the pixel', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/level/1-2-erster-pixel');
  try {
    await app.setCode(referenceOf('1-2-erster-pixel.lvl'));
    await app.click('.dbg-primary');
    await app.until(`(${WS_STATE}).result`, 'result view');
    // Run the first test (input 2 1 14) in the debugger: pixel (2,1) turns colour 14.
    await app.press('Escape');
    await app.press('l');
    await app.until(`(${WS_STATE}).kind === 'halted'`, 'halted');
    const lit = await app.evaluate(`(() => {
      const c = document.querySelector('.screen-canvas');
      const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
      const px = (x, y) => { const sx = Math.floor((x + 0.5) * c.width / 64), sy = Math.floor((y + 0.5) * c.height / 64), i = (sy * c.width + sx) * 4; return [d[i], d[i + 1], d[i + 2]].join(','); };
      return { target: px(2, 1), other: px(5, 5) };
    })()`);
    assert.notEqual(lit.target, lit.other, 'pixel (2,1) differs from the background');
    await assertClean(app);
  } finally {
    await app.close();
  }
});

test('level 2.4 start code: budget message names a line, the hot line is marked', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/level/2-4-endlosschleife');
  try {
    await app.until(`document.querySelector('.ed-text')?.value.includes('jnz  schleife')`, 'start code');
    await app.click('.dbg-primary');
    const why = await app.until(`(() => {
      const e = [...document.querySelectorAll('.test-why')].find((n) => /Zyklenbudget aufgebraucht/.test(n.textContent));
      if (!e) return null;
      const r = e.getBoundingClientRect();
      return r.width > 0 && r.height > 0 && !e.closest('[hidden]') ? e.textContent : null;
    })()`, 'visible budget message');
    const line = Number(why.match(/Zeile (\d+) lief \d+-mal/)?.[1]);
    assert.ok(line >= 8 && line <= 10, `hot line ${line} inside the loop (${why})`);
    assert.equal(await app.evaluate(`document.querySelector('.ed-gutter .gl.is-hot')?.dataset.line`), String(line));
    assert.match((await state(app)).notice, /Tests nicht bestanden/);
    assert.equal((await state(app)).result, false);
    await assertClean(app);
  } finally {
    await app.close();
  }
});

test('level 0.2 with the keyboard only (Tab, Space, Enter, shortcuts)', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/');
  const tabTo = async (expr, what, max = 80) => {
    for (let i = 0; i < max; i++) {
      if (await app.evaluate(`(() => { const a = document.activeElement; return !!a && (${expr}); })()`)) return;
      await app.press('Tab');
    }
    throw new Error(`Tab never reached ${what}`);
  };
  try {
    await app.until(`document.querySelectorAll('.station').length === 30`, 'plan');
    // 0.2 is locked until 0.1 is solved: switch on "unlock everything" with the keyboard.
    await tabTo(`a.id === 'unlock-all'`, 'the unlock switch');
    await app.press(' ');
    await app.until(`document.getElementById('unlock-all').checked && !!document.querySelector('a[href="#/level/0-2-ein-register-aendern"]')`, 'all unlocked');
    assert.equal(await app.evaluate('document.activeElement.id'), 'unlock-all', 'focus stays on the switch after the plan re-renders');
    await tabTo(`a.matches('a[href="#/level/0-2-ein-register-aendern"]')`, 'level 0.2');
    const outline = await app.evaluate(`(() => { const s = getComputedStyle(document.activeElement); return s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0 || s.boxShadow !== 'none'; })()`);
    assert.ok(outline, 'focused station has a visible focus mark');
    await app.press('Enter');
    await app.until(`document.querySelector('.ws-num')?.textContent === '0.2' && document.activeElement.id === 'ws-title'`, 'workspace 0.2 with focus on its title');
    await tabTo(`a.classList.contains('ed-text')`, 'the editor');
    // Select all and type the fix.
    await app.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: process.platform === 'darwin' ? 4 : 2, commands: ['selectAll'] });
    await app.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: process.platform === 'darwin' ? 4 : 2 });
    await app.insertText(referenceOf('0-2-ein-register-aendern.lvl'));
    assert.match(await app.evaluate(`document.querySelector('.ed-text').value`), /mov {2}r0, 42/);
    await app.press('Enter', ['ctrl']);
    await app.until(`(${WS_STATE}).result && document.activeElement.id === 'h-result'`, 'solved, focus on the result heading');
    assert.equal(await app.evaluate(`document.querySelector('.ed-text').value.includes('\\n\\n\\n\\n')`), false, 'Ctrl+Enter inserted no newline');
    // The way on is reachable by keyboard as well.
    await tabTo(`a.classList.contains('btn-next')`, 'the next-level button', 20);
    await app.press('Enter');
    await app.until(`document.querySelector('.ws-num')?.textContent === '1.1'`, 'next level 1.1');
    await assertClean(app);
  } finally {
    await app.close();
  }
});

test('level 6.1 with a misnamed function: the start of the call test is reported, not swallowed', T, async () => {
  const app = await openApp(ctx, '/stapelwerk/#/level/6-1-fakultaet');
  try {
    await app.setCode('fakultaet:\n        mov r0, 1\n        ret\n');
    await app.click('.dbg-primary');
    await app.until(`/Tests nicht bestanden|nicht starten/.test((${WS_STATE}).notice)`, 'tests fail');
    assert.equal((await app.evaluate(WS_STATE)).result, false);
    // Restarting the test in the debugger shows why it cannot start.
    await app.press('Escape');
    await app.press('r');
    const notice = await app.until(`(() => { const n = (${WS_STATE}).notice; return /Sprungziel|nicht starten/.test(n) && n; })()`, 'start fault or start error shown');
    assert.ok(notice.length > 10, notice);
    // Stepping does not pretend that anything runs.
    await app.press('s');
    assert.equal((await app.evaluate(WS_STATE)).steps, 0);
    await assertClean(app);
  } finally {
    await app.close();
  }
});
