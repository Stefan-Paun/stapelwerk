// Console: the input tape with its read position, the output tape (numbers
// and bytes, control characters made visible) and, where it applies, the
// expected output next to it. In the sandbox it takes interactive input.
import { el, mount, setText } from '../dom.js';
import { visibleChar } from '../machine.js';

const MAX_SHOWN = 2000;

export function createConsole({ t, interactive = false, onInt, onText, onClose }) {
  const note = el('span', { class: 'pane-note' });
  const inTape = el('div', { class: 'tape tape-in mono', 'aria-label': t('console.input') });
  const out = el('div', { class: 'tape tape-out mono', role: 'log', 'aria-label': t('console.output') });
  const expected = el('div', { class: 'tape tape-exp mono', 'aria-label': t('console.expected') });
  const expWrap = el('div', { class: 'console-col' }, el('h3', { class: 'sub-head' }, t('console.expected')), expected);
  let form = null;
  let field = null;
  let modeSel = null;
  if (interactive) {
    field = el('input', { class: 'mono', type: 'text', name: 'value', autocomplete: 'off', 'aria-label': t('console.inputField') });
    modeSel = el('select', { 'aria-label': t('console.inputKind') },
      el('option', { value: 'int' }, t('console.kind.int')),
      el('option', { value: 'text' }, t('console.kind.text')));
    form = el('form', {
      class: 'console-form',
      onsubmit: (ev) => {
        ev.preventDefault();
        const v = field.value;
        if (modeSel.value === 'int') {
          const nums = v.trim().split(/\s+/).filter(Boolean).map(Number);
          if (!nums.length || nums.some((n) => !Number.isInteger(n))) { field.setAttribute('aria-invalid', 'true'); return; }
          field.removeAttribute('aria-invalid');
          for (const n of nums) onInt(n);
        } else onText(`${v}\n`);
        field.value = '';
      },
    },
    modeSel, field,
    el('button', { type: 'submit', class: 'btn' }, t('console.send')),
    el('button', { type: 'button', class: 'btn btn-quiet', onclick: () => onClose() }, t('console.close')));
  }
  const node = el('section', { class: 'pane pane-console', 'aria-labelledby': 'h-console' },
    el('header', { class: 'pane-head' }, el('h2', { id: 'h-console' }, t('console.title')), note),
    el('div', { class: 'console-in' }, el('h3', { class: 'sub-head' }, t('console.input')), inTape),
    el('div', { class: 'console-grid' },
      el('div', { class: 'console-col' }, el('h3', { class: 'sub-head' }, t('console.output')), out),
      expWrap),
    form);

  function renderOutput(entries) {
    const shown = entries.length > MAX_SHOWN ? entries.slice(-MAX_SHOWN) : entries;
    const nodes = [];
    if (entries.length > MAX_SHOWN) nodes.push(el('span', { class: 'muted' }, t('console.cut', { count: entries.length - MAX_SHOWN })), '\n');
    let text = '';
    const flush = () => { if (text) { nodes.push(el('span', { class: 'o-chr' }, text)); text = ''; } };
    for (const e of shown) {
      if (e.kind === 0) { flush(); nodes.push(el('span', { class: 'o-int' }, String(e.value))); }
      else if (e.value === 10) { text += '↵'; flush(); nodes.push('\n'); }
      else text += visibleChar(e.value);
    }
    flush();
    if (!entries.length) nodes.push(el('span', { class: 'muted' }, t('console.empty')));
    mount(out, nodes);
    out.scrollTop = out.scrollHeight;
  }

  function update(view) {
    const ins = view.input?.ints ?? [];
    const pos = view.state.inIntPos;
    if (ins.length || view.input?.text) {
      mount(inTape, ins.map((v, i) => el('span', { class: ['i-int', i < pos && 'is-read', i === pos && 'is-next'] }, String(v))),
        view.input?.text ? el('span', { class: 'i-text' }, [...view.input.text].map(visibleChar).join('')) : null);
    } else mount(inTape, el('span', { class: 'muted' }, interactive ? t('console.inputInteractive') : t('console.noInput')));
    renderOutput(view.output);
    if (view.expectedOut) {
      expWrap.hidden = false;
      mount(expected, view.expectedOut.map((v, i) => {
        const got = view.outInts[i];
        return el('span', { class: ['o-int', got !== undefined && got !== v && 'is-wrong', got === v && 'is-right'] }, String(v));
      }));
    } else expWrap.hidden = true;
    setText(note, view.state.status === 4 ? t('console.waiting') : t('console.note', { read: pos, total: ins.length, out: view.output.length }));
    if (form) {
      form.classList.toggle('is-waiting', view.state.status === 4);
      if (view.state.status === 4 && document.activeElement?.closest?.('.pane-console') == null && view.focusInput) field.focus();
    }
  }

  return { node, update };
}
