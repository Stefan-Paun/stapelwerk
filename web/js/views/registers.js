// Registers and flags as a definition list. Values are editable while the
// machine is paused; the value a step overwrote stays visible, struck through
// in red pencil, until the next step.
import { el, mount, setText } from '../dom.js';
import { icon } from '../icons.js';
import { FLAG_NAMES, REG_FLAGS, REG_NAMES, hex, signed } from '../machine.js';

/** Parse "42", "-7", "0x2A", "0b1010", "'A'" into a u32, or null. */
export function parseValue(text) {
  const s = String(text).trim().replace(/_/g, '');
  let v = null;
  if (/^-?\d+$/.test(s)) v = Number.parseInt(s, 10);
  else if (/^0x[0-9a-f]+$/i.test(s)) v = Number.parseInt(s.slice(2), 16);
  else if (/^0b[01]+$/i.test(s)) v = Number.parseInt(s.slice(2), 2);
  else if (/^'.'$/.test(s)) v = s.charCodeAt(1);
  if (v === null || !Number.isFinite(v) || v < -(2 ** 31) || v > 2 ** 32 - 1) return null;
  return v >>> 0;
}

const GROUPS = [
  { key: 'regs.callerSaved', regs: [0, 1, 2, 3] },
  { key: 'regs.calleeSaved', regs: [4, 5, 6, 7] },
  { key: 'regs.pointers', regs: [8, 9] },
];

export function createRegisters({ t, onSetReg, onBackToChange }) {
  const note = el('span', { class: 'pane-note mono' });
  const list = el('div', { class: 'regs' });
  const flagsBox = el('div', { class: 'flags', role: 'group', 'aria-label': t('regs.flags') });
  const node = el('section', { class: 'pane pane-regs', 'aria-labelledby': 'h-regs' },
    el('header', { class: 'pane-head' }, el('h2', { id: 'h-regs' }, t('regs.title')), note),
    list,
    el('div', { class: 'flags-row' }, el('h3', { class: 'sub-head' }, t('regs.flags')), flagsBox));

  let editing = null;
  let last = null;

  function startEdit(index, value) {
    editing = index;
    render(last);
    const input = list.querySelector(`input[data-reg="${index}"]`);
    if (input) { input.value = String(signed(value)); input.select(); input.focus(); }
  }

  function commit(index, text) {
    const v = parseValue(text);
    if (v === null) return false;
    editing = null;
    onSetReg(index, v);
    return true;
  }

  function regRow(i, view) {
    const value = view.regs[i] >>> 0;
    const changed = view.changedRegs.has(i);
    const before = view.prevRegs?.[i];
    const name = REG_NAMES[i];
    let valueNode;
    if (editing === i && view.editable) {
      const input = el('input', {
        class: 'reg-input mono',
        type: 'text',
        inputmode: 'text',
        dataset: { reg: i },
        'aria-label': t('regs.editLabel', { reg: name }),
        onkeydown: (ev) => {
          if (ev.key === 'Enter') { ev.preventDefault(); if (!commit(i, ev.target.value)) ev.target.setAttribute('aria-invalid', 'true'); }
          if (ev.key === 'Escape') { ev.preventDefault(); ev.stopPropagation(); editing = null; render(last); list.querySelector(`button[data-reg="${i}"]`)?.focus(); }
        },
        onblur: (ev) => { if (editing === i) { commit(i, ev.target.value); editing = null; render(last); } },
      });
      valueNode = input;
    } else {
      valueNode = el('button', {
        type: 'button',
        class: 'reg-val',
        dataset: { reg: i },
        'aria-disabled': view.editable ? null : 'true',
        'aria-label': t(changed ? 'regs.valueChanged' : 'regs.value', { reg: name, value: signed(value), hex: hex(value), before: before === undefined ? '' : signed(before) }),
        title: view.editable ? t('regs.editHint') : t('regs.pausedOnly'),
        onclick: () => { if (view.editable) startEdit(i, value); },
      },
      el('span', { class: 'hex mono' }, hex(value)),
      el('span', { class: 'dec mono' }, String(signed(value))));
    }
    return el('div', { class: ['reg', changed && 'is-changed', i === 9 && 'is-sp', i === 8 && 'is-fp'] },
      el('dt', { class: 'mono' }, name),
      el('dd', {},
        changed && before !== undefined ? el('s', { class: 'reg-old mono', 'aria-hidden': 'true' }, String(signed(before))) : null,
        valueNode,
        el('button', {
          type: 'button',
          class: 'icon-btn reg-back',
          'aria-label': t('regs.backToChange', { reg: name }),
          title: t('regs.backToChange', { reg: name }),
          disabled: view.canTravel ? null : true,
          onclick: () => onBackToChange(0, i),
        }, icon('rewindChange'))));
  }

  function render(view) {
    if (!view) return;
    last = view;
    setText(note, t('regs.pcNote', { pc: hex(view.pc, 4), line: view.line ?? '–' }));
    mount(list, GROUPS.map((g) => el('div', { class: 'reg-group' },
      el('p', { class: 'reg-group-head' }, t(g.key)),
      el('dl', {}, g.regs.map((i) => regRow(i, view))))));
    mount(flagsBox, FLAG_NAMES.map((name, bit) => {
      const on = (view.flags >> bit) & 1;
      const changed = ((view.flags ^ (view.prevFlags ?? view.flags)) >> bit) & 1;
      return el('button', {
        type: 'button',
        class: ['flag', on && 'is-on', changed && 'is-changed'],
        'aria-pressed': on ? 'true' : 'false',
        'aria-disabled': view.editable ? null : 'true',
        'aria-label': t('regs.flagLabel', { flag: name, meaning: t(`regs.flag.${name}`), value: on }),
        title: t(`regs.flag.${name}`),
        onclick: () => { if (view.editable) onSetReg(REG_FLAGS, (view.flags ^ (1 << bit)) & 15); },
      }, el('span', { class: 'flag-name mono' }, name), el('span', { class: 'flag-bit mono' }, String(on)));
    }));
  }

  return { node, update: render };
}
