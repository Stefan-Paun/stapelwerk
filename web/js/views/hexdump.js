// Hexdump of one segment at a time, bytes or words, with labels, the pc and
// sp marked and the bytes the last action changed hatched in red pencil.
// Rows are windowed so 16 KiB of heap stays cheap.
import { el, mount } from '../dom.js';
import { SEGMENTS, hex } from '../machine.js';
import { ROW_BYTES, hexRows } from '../hexdump.js';

const ROW_H = 20;
const SHOWN = ['text', 'data', 'input', 'heap', 'stack', 'screen'];

export function createHexdump({ t }) {
  let segment = 'data';
  let mode = 'byte';
  let view = null;
  let autoFollow = true;

  const segButtons = SHOWN.map((name) => el('button', {
    type: 'button', class: ['chip', `seg-${name}`], dataset: { seg: name }, 'aria-pressed': name === segment ? 'true' : 'false',
    onclick: () => { segment = name; autoFollow = false; for (const b of segButtons) b.setAttribute('aria-pressed', b.dataset.seg === name ? 'true' : 'false'); scroller.scrollTop = 0; draw(); },
  }, el('i', { class: 'seg-swatch', 'aria-hidden': 'true' }), t(`seg.${name}`)));
  const modeButtons = ['byte', 'word'].map((m) => el('button', {
    type: 'button', class: 'seg-btn', dataset: { mode: m }, 'aria-pressed': m === mode ? 'true' : 'false',
    onclick: () => { mode = m; for (const b of modeButtons) b.setAttribute('aria-pressed', b.dataset.mode === m ? 'true' : 'false'); draw(); },
  }, t(`hex.${m}`)));
  const range = el('p', { class: 'hex-range mono' });
  const rows = el('div', { class: 'hex-rows', role: 'table', 'aria-label': t('hex.title') });
  const spacer = el('div', { class: 'hex-spacer' }, rows);
  const scroller = el('div', { class: 'hex-scroll', tabindex: '0', 'aria-label': t('hex.scrollLabel') }, spacer);
  const node = el('div', { class: 'hexdump' },
    el('div', { class: 'hex-tools' },
      el('div', { class: 'chips', role: 'group', 'aria-label': t('hex.segments') }, segButtons),
      el('div', { class: 'seg', role: 'group', 'aria-label': t('hex.unit') }, modeButtons)),
    range, scroller);

  function draw() {
    if (!view) return;
    const seg = SEGMENTS.find((s) => s.name === segment);
    range.textContent = `${t(`seg.${segment}`)} · ${hex(seg.from, 4)}–${hex(seg.to - 1, 4)} · ${t(`seg.rights.${segment}`)}`;
    const total = (seg.to - seg.from) / ROW_BYTES;
    spacer.style.height = `${total * ROW_H}px`;
    const first = Math.max(0, Math.floor(scroller.scrollTop / ROW_H) - 4);
    const count = Math.ceil((scroller.clientHeight || 300) / ROW_H) + 8;
    const from = seg.from + first * ROW_BYTES;
    const to = Math.min(seg.to, from + count * ROW_BYTES);
    const model = hexRows(view.mem, { from, to, mode, changed: view.changedBytes, labels: view.labels, pc: view.pc, sp: view.sp });
    rows.style.transform = `translateY(${first * ROW_H}px)`;
    mount(rows, model.map((r) => el('div', { class: 'hex-row', role: 'row' },
      el('span', { class: 'hex-addr', role: 'rowheader' }, hex(r.addr, 4)),
      el('span', { class: ['hex-cells', `is-${mode}`], role: 'cell' }, r.cells.map((c) => el('span', {
        class: ['hx', c.changed && (view.quiet ? 'is-touched' : 'is-changed'), c.pc && 'is-pc', c.sp && 'is-sp', c.value === 0 && 'is-zero'],
      }, hex(c.value, mode === 'word' ? 8 : 2)))),
      mode === 'byte' ? el('span', { class: 'hex-ascii', role: 'cell' }, r.cells.map((c) => (c.value >= 0x20 && c.value < 0x7f ? String.fromCharCode(c.value) : '·')).join('')) : el('span', { class: 'hex-ascii', role: 'cell' }),
      el('span', { class: 'hex-label', role: 'cell' }, r.label ?? ''))));
  }

  scroller.addEventListener('scroll', () => requestAnimationFrame(draw));

  return {
    node,
    update(next) {
      view = next;
      // Follow the action: a fresh change jumps to its segment once.
      if (autoFollow && next.changedBytes?.size && !next.quiet) {
        const a = Math.min(...next.changedBytes);
        const seg = SEGMENTS.find((s) => a >= s.from && a < s.to);
        if (seg && SHOWN.includes(seg.name) && seg.name !== segment) {
          segment = seg.name;
          for (const b of segButtons) b.setAttribute('aria-pressed', b.dataset.seg === segment ? 'true' : 'false');
        }
      }
      draw();
    },
  };
}
