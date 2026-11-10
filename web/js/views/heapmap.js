// Heap map: all 4096 granules of 4 bytes as a sheet of cells, each state with
// its own pattern (never colour alone), plus the block list with source lines.
import { el, mount } from '../dom.js';
import { HEAP, hex } from '../machine.js';

const COLS = 128;
const ROWS = 32;
const CELL = 5;
const STATES = ['free', 'live', 'freed', 'red'];

export function createHeapmap({ t }) {
  const canvas = el('canvas', { class: 'heap-canvas', width: COLS * CELL, height: ROWS * CELL, role: 'img', 'aria-label': t('heap.title') });
  const legend = el('ul', { class: 'legend' }, STATES.map((s) => el('li', {}, el('i', { class: `lg lg-${s}`, 'aria-hidden': 'true' }), t(`heap.state.${s}`))));
  const tbody = el('tbody');
  const table = el('table', { class: 'heap-table' },
    el('caption', { class: 'sr-only' }, t('heap.blocks')),
    el('thead', {}, el('tr', {}, ['addr', 'size', 'status', 'line'].map((k) => el('th', { scope: 'col' }, t(`heap.col.${k}`))))),
    tbody);
  const empty = el('p', { class: 'empty' }, t('heap.empty'));
  const node = el('div', { class: 'heapmap' }, el('div', { class: 'heap-sheet' }, canvas), legend, empty, table);
  const ctx = canvas.getContext('2d');

  function css(name) { return getComputedStyle(node).getPropertyValue(name).trim(); }

  function update(view) {
    const shadow = view.shadow;
    const ink = css('--ink') || '#16222E';
    const blue = css('--blue') || '#1D4F91';
    const red = css('--pencil') || '#C6372F';
    const line = css('--rule-faint') || '#D5DCE3';
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (let i = 0; i < COLS * ROWS; i++) {
      const x = (i % COLS) * CELL, y = Math.floor(i / COLS) * CELL;
      const s = shadow[i];
      if (s === 1) { ctx.fillStyle = blue; ctx.fillRect(x, y, CELL - 1, CELL - 1); }
      else if (s === 2) { ctx.strokeStyle = red; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(x, y + CELL - 1); ctx.lineTo(x + CELL - 1, y); ctx.stroke(); }
      else if (s === 3) { ctx.fillStyle = ink; ctx.fillRect(x + 1, y + 1, CELL - 3, CELL - 3); }
      else { ctx.fillStyle = line; ctx.fillRect(x + 2, y + 2, 1, 1); }
    }
    const live = view.heap.filter((b) => b.live).length;
    canvas.setAttribute('aria-label', t('heap.aria', { live, total: view.heap.length }));
    empty.hidden = view.heap.length > 0;
    table.hidden = view.heap.length === 0;
    mount(tbody, view.heap.map((b) => el('tr', { class: b.live ? 'is-live' : 'is-freed' },
      el('td', { class: 'mono' }, hex(b.addr, 4)),
      el('td', { class: 'mono' }, `${b.size} B`),
      el('td', {}, b.live ? t('heap.live') : t('heap.freed')),
      el('td', { class: 'mono' }, b.line ? String(b.line) : '–'))));
  }

  return { node, update, HEAP };
}
