// Test table: every visible test with its input, expectation and result;
// hidden tests as a count, opened up only when one fails (input + expected).
import { el, mount } from '../dom.js';
import { icon } from '../icons.js';

export function describeMismatch(t, m, num = String) {
  if (!m) return '';
  const args = { ...m.args };
  // The core names the status ("BUDGET_EXCEEDED"); the JS mock used the number 5.
  const budget = args.status === 'BUDGET_EXCEEDED' || args.status === 5;
  if (m.key === 'check.not_halted' && budget && args.hottest_line !== undefined) {
    return t('check.not_halted_budget', { cycles: num(args.cycles), line: args.hottest_line, count: args.hottest_count });
  }
  if (typeof args.addr === 'number') args.addr = `0x${(args.addr >>> 0).toString(16).toUpperCase().padStart(4, '0')}`;
  return t(m.key, args);
}

export function createTests({ t, num = String, onOpen, faultText }) {
  const list = el('ol', { class: 'tests' });
  const summary = el('p', { class: 'tests-summary' });
  const node = el('div', { class: 'tests-wrap' }, summary, list);

  function update({ level, result, current }) {
    const rows = [];
    const visible = level.tests.map((test, i) => ({ test, i })).filter((x) => !x.test.hidden);
    const hidden = level.tests.filter((x) => x.hidden).length;
    for (const { test, i } of visible) {
      const r = result?.tests?.[i];
      const status = r ? (r.passed ? 'passed' : 'failed') : 'pending';
      const io = test.call
        ? `${test.call}(${Object.entries(test.regs ?? {}).map(([k, v]) => `${k}=${v}`).join(', ')})${test.expect_reg ? ` → ${Object.entries(test.expect_reg).map(([k, v]) => `${k}=${v}`).join(', ')}` : ''}`
        : test.screen ? `${(test.in ?? []).join(' ')} → ${t('tests.screen')}` : `${(test.in ?? []).join(' ') || '–'} → ${(test.out ?? []).join(' ') || '–'}`;
      rows.push(el('li', { class: ['test', `is-${status}`, current === i && 'is-current'] },
        el('span', { class: 'test-mark' }, status === 'passed' ? icon('check', t('tests.passed')) : status === 'failed' ? icon('cross', t('tests.failed')) : el('span', { class: 'sr-only' }, t('tests.pending'))),
        el('span', { class: 'test-name mono' }, test.name),
        el('span', { class: 'test-io mono' }, io),
        el('span', { class: 'test-cyc mono' }, r ? t('tests.cycles', { n: num(r.cycles) }) : ''),
        el('button', { type: 'button', class: 'btn btn-quiet btn-small', onclick: () => onOpen(i), 'aria-label': t('tests.openLabel', { name: test.name }) }, t('tests.open')),
        r && !r.passed ? el('p', { class: 'test-why' }, r.fault ? faultText(r.fault) : describeMismatch(t, r.mismatch, num)) : null));
    }
    const hiddenResults = result?.tests?.filter((r) => r.hidden) ?? [];
    const hiddenFailed = hiddenResults.filter((r) => !r.passed);
    if (hidden) {
      rows.push(el('li', { class: ['test', 'test-hidden', result && (hiddenFailed.length ? 'is-failed' : 'is-passed')] },
        el('span', { class: 'test-mark' }, result ? (hiddenFailed.length ? icon('cross') : icon('check')) : null),
        el('span', { class: 'test-name' }, t('tests.hidden', { count: hidden })),
        el('span', { class: 'test-io' }, result ? t('tests.hiddenResult', { passed: hiddenResults.length - hiddenFailed.length, total: hidden }) : t('tests.hiddenNote'))));
      for (const r of hiddenFailed) {
        rows.push(el('li', { class: 'test is-failed test-reveal' },
          el('span', { class: 'test-mark' }, icon('cross')),
          el('span', { class: 'test-name mono' }, r.name),
          el('span', { class: 'test-io mono' }, r.input ? `${(r.input.in ?? []).join(' ')} → ${(r.input.out ?? []).join(' ')}` : ''),
          el('span', { class: 'test-cyc mono' }, t('tests.cycles', { n: num(r.cycles) })),
          el('button', { type: 'button', class: 'btn btn-quiet btn-small', onclick: () => onOpen(r.index) }, t('tests.open')),
          el('p', { class: 'test-why' }, r.fault ? faultText(r.fault) : describeMismatch(t, r.mismatch, num))));
      }
    }
    mount(list, rows);
    if (!result) mount(summary, t('tests.notRun'));
    else {
      const passed = result.tests.filter((x) => x.passed).length;
      mount(summary, el('strong', { class: result.passed ? 'ok' : 'bad' }, result.passed ? t('tests.allPassed') : t('tests.someFailed', { passed, total: result.tests.length })));
    }
  }

  return { node, update };
}
