// Debugger by keyboard: step (S), back (Z), breakpoint (Ctrl/⌘+B in the editor), run (L)
// up to the breakpoint, then edit a register while paused and run to the end.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { WS_STATE, openApp, referenceOf, skip, useBrowser } from './harness.mjs';

const ctx = useBrowser();

test('step, back, breakpoint and run by keyboard; register edit while paused', { skip, timeout: 90000 }, async () => {
  const ref = referenceOf('2-2-bis-zur-null.lvl');
  const app = await openApp(ctx, '/stapelwerk/#/level/2-2-bis-zur-null');
  const state = () => app.evaluate(WS_STATE);
  try {
    await app.setCode(ref);
    // Esc leaves the editor; the shortcuts rest while typing.
    await app.press('Escape');
    assert.equal(await app.evaluate(`document.activeElement.dataset.action`), 'step');
    await app.press('s');
    await app.until(`(${WS_STATE}).steps === 1`, 'one step');
    assert.equal((await state()).line, 3, 'after mov r1, 0 the machine stands on line 3');
    await app.until(`/Zeile 1/.test(document.getElementById('live').textContent)`, 'live region describes the step');
    await app.press('s');
    await app.until(`(${WS_STATE}).steps === 2 && (${WS_STATE}).line === 4`, 'second step');
    await app.press('z');
    await app.until(`(${WS_STATE}).steps === 1 && (${WS_STATE}).line === 3`, 'one step back');

    // Breakpoint on line 9 (mov r0, r1) with the caret there and Ctrl/⌘+B.
    await app.evaluate(`(() => {
      const a = document.querySelector('.ed-text');
      const lines = a.value.split('\\n');
      const at = lines.slice(0, 8).join('\\n').length + 1;
      a.focus();
      a.setSelectionRange(at, at);
    })()`);
    await app.press('b', [process.platform === 'darwin' ? 'meta' : 'ctrl']);
    await app.until(`document.querySelector('.ed-gutter .gl[data-line="9"]')?.classList.contains('has-bp')`, 'breakpoint mark on line 9');
    assert.equal(await app.evaluate(`document.querySelector('.ed-text').value`), ref, 'the shortcut did not change the text');
    await app.press('Escape');
    await app.press('l');
    await app.until(`(${WS_STATE}).kind === 'breakpoint' && (${WS_STATE}).line === 9`, 'run stops at the breakpoint');
    // Test 1 sums 3 + 4 + 5.
    assert.equal(await app.evaluate(`document.querySelector('.reg-val[data-reg="1"] .dec').textContent`), '12');

    // Edit r1 while paused: Enter on the value, type, Enter.
    await app.evaluate(`document.querySelector('.reg-val[data-reg="1"]').focus()`);
    await app.press('Enter');
    await app.until(`document.activeElement?.matches('input.reg-input[data-reg="1"]')`, 'register input');
    await app.insertText('100');
    await app.press('Enter');
    await app.until(`document.querySelector('.reg-val[data-reg="1"] .dec')?.textContent === '100'`, 'r1 = 100');
    assert.equal(await app.evaluate(`document.querySelector('.reg-val[data-reg="1"]').closest('.reg').classList.contains('is-changed')`), true, 'changed register is marked');
    // A wrong value is refused and marked.
    await app.evaluate(`document.querySelector('.reg-val[data-reg="2"]').focus()`);
    await app.press('Enter');
    await app.until(`document.activeElement?.matches('input.reg-input[data-reg="2"]')`, 'register input r2');
    await app.insertText('zwölf');
    await app.press('Enter');
    assert.equal(await app.evaluate(`document.activeElement.getAttribute('aria-invalid')`), 'true');
    await app.press('Escape');
    await app.until(`!document.querySelector('input.reg-input')`, 'edit cancelled');

    await app.press('l');
    await app.until(`(${WS_STATE}).kind === 'halted'`, 'halted');
    assert.match(await app.evaluate(`document.querySelector('.tape-out').textContent`), /^100/);
    // Registers can no longer be edited after the halt.
    assert.equal(await app.evaluate(`document.querySelector('.reg-val[data-reg="1"]').getAttribute('aria-disabled')`), 'true');
    assert.deepEqual(app.errors, []);
    assert.deepEqual(await app.csp(), []);
  } finally {
    await app.close();
  }
});
