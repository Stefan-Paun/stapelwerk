// Code editor: a plain <textarea> over a backdrop that draws the current line,
// error marks and column rules, plus a gutter with breakpoints, line numbers
// and execution counters. No library; scroll positions are kept in sync.
import { el, mount } from '../dom.js';
import { breakpointMark } from '../icons.js';

const PALETTE_TOKENS = ['mov', 'add', 'sub', 'cmp', 'jz', 'jnz', 'jmp', 'sys ', 'push', 'pop', 'call', 'ret', 'halt', 'r0', 'r1', 'r2', ', ', '[', ']', ':', '; '];

export function createEditor({ t, value = '', label, onInput, onToggleBreakpoint, onEscape }) {
  const gutterInner = el('div', { class: 'ed-gutter-inner' });
  const gutter = el('div', { class: 'ed-gutter', 'aria-hidden': 'true' }, gutterInner);
  const backInner = el('div', { class: 'ed-back-inner' });
  const back = el('div', { class: 'ed-back', 'aria-hidden': 'true' }, backInner);
  const errorsId = `ed-errors-${Math.random().toString(36).slice(2, 8)}`;
  const area = el('textarea', {
    class: 'ed-text',
    spellcheck: 'false',
    autocapitalize: 'off',
    autocomplete: 'off',
    autocorrect: 'off',
    wrap: 'off',
    'aria-label': label,
    'aria-describedby': errorsId,
    'aria-keyshortcuts': 'Control+Enter Escape',
  });
  area.value = value;
  const errorsList = el('ul', { class: 'ed-errors', id: errorsId, 'aria-live': 'polite' });
  const body = el('div', { class: 'ed-body' }, back, area);
  const palette = el('div', { class: 'ed-palette', role: 'group', 'aria-label': t('editor.palette') },
    PALETTE_TOKENS.map((tok) => el('button', {
      type: 'button',
      class: 'chip mono',
      onclick: () => insert(tok.trim() === tok && /^[a-z]+$/.test(tok) && tok.length > 2 ? `${tok} ` : tok),
    }, tok.trim() || '␣')));
  const root = el('div', { class: 'editor' }, el('div', { class: 'ed-frame' }, gutter, body), palette, errorsList);

  let state = { current: null, hot: null, errors: [], counts: new Map(), breakpoints: new Set(), codeLines: new Set() };
  let lines = [];

  function insert(text) {
    const { selectionStart: a, selectionEnd: b } = area;
    area.setRangeText(text, a, b, 'end');
    area.focus();
    area.dispatchEvent(new Event('input'));
  }

  function renderLines() {
    lines = area.value.split('\n');
    const errorsByLine = new Map();
    for (const e of state.errors) {
      if (e.file !== 0) continue;
      if (!errorsByLine.has(e.line)) errorsByLine.set(e.line, []);
      errorsByLine.get(e.line).push(e);
    }
    const gutterRows = [];
    const backRows = [];
    lines.forEach((text, i) => {
      const n = i + 1;
      const isCurrent = state.current === n;
      const errs = errorsByLine.get(n);
      const count = state.counts.get(n) ?? 0;
      const bp = state.breakpoints.has(n);
      gutterRows.push(el('div', {
        class: ['gl', isCurrent && 'is-current', errs && 'has-error', state.hot === n && 'is-hot', bp && 'has-bp', state.codeLines.has(n) && 'is-code'],
        dataset: { line: n },
        title: state.codeLines.has(n) ? t('editor.toggleBp') : null,
      },
      el('span', { class: 'gl-count' }, count ? `${count > 9999 ? '9k+' : count}` : ''),
      el('span', { class: 'gl-bp' }, bp ? breakpointMark() : null),
      el('span', { class: 'gl-no' }, String(n)),
      el('span', { class: 'gl-pc' })));
      const row = el('div', { class: ['bl', isCurrent && 'is-current', errs && 'has-error', state.hot === n && !isCurrent && 'is-hot'] });
      if (errs) {
        // Rebuild the text with the error ranges wrapped, columns are UTF-16 units.
        let pos = 0;
        const sorted = [...errs].sort((a, b) => a.col - b.col);
        for (const e of sorted) {
          const start = Math.max(pos, Math.min(text.length, e.col - 1));
          const end = Math.max(start + 1, Math.min(text.length, start + Math.max(1, e.len)));
          if (start > pos) row.append(text.slice(pos, start));
          row.append(el('span', { class: 'err' }, text.slice(start, end) || ' '));
          pos = end;
        }
        if (pos < text.length) row.append(text.slice(pos));
      } else row.append(text || ' ');
      backRows.push(row);
    });
    mount(gutterInner, gutterRows);
    mount(backInner, backRows);
    syncScroll();
  }

  function syncScroll() {
    gutterInner.style.transform = `translateY(${-area.scrollTop}px)`;
    backInner.style.transform = `translate(${-area.scrollLeft}px, ${-area.scrollTop}px)`;
  }

  function renderErrors(messages) {
    mount(errorsList, messages.map((m) => el('li', {}, el('button', {
      type: 'button',
      class: 'ed-error',
      onclick: () => goToLine(m.line, m.col),
    }, el('span', { class: 'ed-error-pos mono' }, t('editor.linecol', { line: m.line, col: m.col })), ' ', m.text))));
    errorsList.hidden = messages.length === 0;
  }

  function goToLine(line, col = 1) {
    let offset = 0;
    for (let i = 0; i < line - 1 && i < lines.length; i++) offset += lines[i].length + 1;
    offset += Math.max(0, col - 1);
    area.focus();
    area.setSelectionRange(offset, offset);
    scrollToLine(line);
  }

  function scrollToLine(line) {
    const lh = parseFloat(getComputedStyle(area).lineHeight) || 20;
    const top = (line - 1) * lh;
    if (top < area.scrollTop + lh || top > area.scrollTop + area.clientHeight - 2 * lh) {
      area.scrollTop = Math.max(0, top - area.clientHeight / 3);
      syncScroll();
    }
  }

  area.addEventListener('scroll', syncScroll);
  area.addEventListener('input', () => { renderLines(); onInput?.(area.value); });
  area.addEventListener('keydown', (ev) => {
    if (ev.key === 'Tab' && !ev.shiftKey && !ev.ctrlKey && !ev.metaKey && !ev.altKey) {
      ev.preventDefault();
      insert('\t');
    } else if (ev.key === 'Escape') {
      ev.preventDefault();
      onEscape?.();
    }
  });
  gutter.addEventListener('click', (ev) => {
    const row = ev.target.closest('.gl');
    if (row) onToggleBreakpoint?.(Number(row.dataset.line));
  });
  renderLines();
  errorsList.hidden = true;

  return {
    node: root,
    textarea: area,
    get value() { return area.value; },
    set value(v) { area.value = v; renderLines(); },
    caretLine() { return area.value.slice(0, area.selectionStart).split('\n').length; },
    focus() { area.focus(); },
    goToLine,
    update(next) {
      const prevCurrent = state.current;
      state = { ...state, ...next };
      renderLines();
      if (next.current && next.current !== prevCurrent && document.activeElement !== area) scrollToLine(next.current);
    },
    setErrors(messages) { renderErrors(messages); },
    setReadonly(on) { area.readOnly = Boolean(on); },
  };
}
