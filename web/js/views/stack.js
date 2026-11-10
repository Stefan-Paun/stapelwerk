// Stack as a real table (address, value, meaning), high addresses on top so
// the stack visibly grows downwards. Beside it, dimension chains measured
// from the live row positions: frame size between sp and fp, and the fixed
// distance from fp to the return address.
import { el, mount, setText, svg } from '../dom.js';
import { hex, signed } from '../machine.js';
import { frameDimensions, stackRows } from '../stackmodel.js';

export function createStack({ t }) {
  const note = el('span', { class: 'pane-note mono' });
  const dims = svg('svg', { class: 'dims', 'aria-hidden': 'true' });
  const tbody = el('tbody');
  const empty = el('p', { class: 'empty' });
  const table = el('table', { class: 'stack' },
    el('caption', { class: 'sr-only' }, t('stack.caption')),
    el('thead', {}, el('tr', {},
      el('th', { scope: 'col', class: 'c-ptr' }, el('span', { class: 'sr-only' }, t('stack.pointer'))),
      el('th', { scope: 'col', class: 'c-addr' }, t('stack.addr')),
      el('th', { scope: 'col', class: 'c-val' }, t('stack.value')),
      el('th', { scope: 'col', class: 'c-mean' }, t('stack.meaning')))),
    tbody);
  const scroller = el('div', { class: 'stack-scroll' }, el('div', { class: 'stack-wrap' }, dims, table), empty);
  const node = el('section', { class: 'pane pane-stack', 'aria-labelledby': 'h-stack' },
    el('header', { class: 'pane-head' }, el('h2', { id: 'h-stack' }, t('stack.title')), note),
    scroller);

  let lastDims = [];
  let resizeObserver = null;

  function meaning(row, frames, view) {
    const f = frames[row.frame];
    switch (row.kind) {
      case 'oldfp': return [t('stack.oldFp'), row.value === 0 ? el('span', { class: 'muted' }, ` · ${t('stack.chainEnd')}`) : null];
      case 'ret': {
        // Visible as the instruction name "ret" (short enough for the column), spoken in full.
        const label = [el('span', { class: 'mono', 'aria-hidden': 'true' }, 'ret'), el('span', { class: 'sr-only' }, t('stack.ret'))];
        if (view.trapAddr !== null && row.value === view.trapAddr) return [...label, el('span', { class: 'muted' }, ` → ${t('stack.checker')}`)];
        const line = view.lineOf(row.value);
        return [...label, el('span', { class: 'muted' }, ` → ${line ? t('stack.line', { line }) : hex(row.value, 4)}`)];
      }
      case 'local': return [t('stack.local'), el('span', { class: 'muted mono' }, ` [fp−${-row.offset}]`)];
      default: {
        if (f && row.offset > 4) return [t('stack.arg'), el('span', { class: 'muted mono' }, ` [fp+${row.offset}]`)];
        if (view.trapAddr !== null && row.value === view.trapAddr) return [t('stack.trap')];
        if (view.isReturnAddr?.(row.value)) {
          // A return address outside the frame chain, e.g. below a value that was never popped.
          const line = view.lineOf(row.value);
          return [el('span', { class: 'mono', 'aria-hidden': 'true' }, 'ret'), el('span', { class: 'sr-only' }, t('stack.ret')),
            el('span', { class: 'muted' }, ` → ${t('stack.line', { line })}`)];
        }
        return [t('stack.pushed')];
      }
    }
  }

  function drawDims() {
    const rowsByAddr = new Map();
    for (const tr of tbody.querySelectorAll('tr[data-addr]')) rowsByAddr.set(Number(tr.dataset.addr), tr);
    const height = table.offsetHeight;
    dims.setAttribute('height', String(height));
    dims.setAttribute('viewBox', `0 0 56 ${height}`);
    const children = [];
    const x = 44;
    for (const d of lastDims) {
      const a = rowsByAddr.get(d.from);
      const b = rowsByAddr.get(d.to);
      if (!a || !b) continue;
      // Rows are drawn high address first: "to" (higher) sits above "from".
      const yTop = b.offsetTop + b.offsetHeight / 2;
      const yBot = a.offsetTop + a.offsetHeight / 2;
      const cls = `dim dim-${d.kind}${d.frame === 0 ? ' dim-current' : ''}`;
      children.push(
        svg('line', { class: `${cls} dim-ext`, x1: x - 4, y1: yTop, x2: 56, y2: yTop }),
        svg('line', { class: `${cls} dim-ext`, x1: x - 4, y1: yBot, x2: 56, y2: yBot }),
        svg('line', { class: `${cls} dim-line`, x1: x, y1: yTop, x2: x, y2: yBot }),
        svg('line', { class: `${cls} dim-tick`, x1: x - 3.5, y1: yTop + 3.5, x2: x + 3.5, y2: yTop - 3.5 }),
        svg('line', { class: `${cls} dim-tick`, x1: x - 3.5, y1: yBot + 3.5, x2: x + 3.5, y2: yBot - 3.5 }),
        svg('text', { class: `${cls} dim-label`, x: x - 6, y: (yTop + yBot) / 2, 'text-anchor': 'end', 'dominant-baseline': 'middle' },
          d.kind === 'ret' ? 'fp+4' : `fp−${d.bytes}`),
      );
    }
    mount(dims, children);
  }

  function update(view) {
    const frames = view.frames;
    const { rows, truncated } = stackRows(view.mem, { sp: view.regs[9], fp: view.regs[8] }, frames, { changed: view.changedBytes });
    setText(note, t('stack.note', { sp: hex(view.regs[9], 4), frames: frames.length }));
    if (!view.assembled || rows.length === 0) {
      table.hidden = true;
      dims.hidden = true;
      empty.hidden = false;
      setText(empty, view.assembled ? t('stack.empty') : t('stack.notAssembled'));
      return;
    }
    table.hidden = false;
    dims.hidden = false;
    empty.hidden = true;
    const out = [];
    const ordered = [...rows].reverse();
    let currentFrame = null;
    for (const row of ordered) {
      if (row.frame !== currentFrame && row.frame >= 0 && frames[row.frame] && row.kind === 'ret') {
        currentFrame = row.frame;
        const f = frames[row.frame];
        out.push(el('tr', { class: 'frame-head' }, el('th', { colspan: 4, scope: 'rowgroup' },
          el('span', { class: 'frame-name mono' }, f.func ?? '?'),
          el('span', { class: 'muted' }, ` · ${row.frame === 0 ? t('stack.frameCurrent') : t('stack.frameN', { n: row.frame })}`))));
      }
      const looseRet = row.kind !== 'ret' && row.kind !== 'oldfp' && row.kind !== 'local' && view.isReturnAddr?.(row.value);
      out.push(el('tr', {
        class: [`k-${row.kind}`, looseRet && 'k-ret', row.isSp && 'is-sp', row.isFp && 'is-fp', row.changed && (view.quiet ? 'is-touched' : 'is-changed')],
        dataset: { addr: row.addr },
      },
      el('td', { class: 'c-ptr mono' }, [row.isFp ? 'fp' : '', row.isFp && row.isSp ? ' ' : '', row.isSp ? 'sp' : ''].join('')),
      el('th', { scope: 'row', class: 'c-addr mono' }, hex(row.addr, 4)),
      el('td', { class: 'c-val mono' }, hex(row.value), el('span', { class: 'dec' }, ` ${signed(row.value)}`),
        row.changed ? el('span', { class: 'sr-only' }, ` (${t('common.changed')})`) : null),
      (() => { const m = el('td', { class: 'c-mean' }, meaning(row, frames, view)); m.title = m.textContent; return m; })()));
    }
    if (truncated) out.unshift(el('tr', { class: 'more' }, el('td', { colspan: 4 }, t('stack.truncated'))));
    mount(tbody, out);
    lastDims = frameDimensions({ sp: view.regs[9] }, frames);
    drawDims();
    // Keep sp in view: it is the bottom row. Then back off to a whole row, so no half-hidden
    // row peeks out under the sticky head.
    scroller.scrollTop = scroller.scrollHeight;
    if (scroller.scrollTop > 0) {
      const visTop = scroller.getBoundingClientRect().top + (table.tHead?.offsetHeight ?? 0);
      for (const tr of tbody.rows) {
        const r = tr.getBoundingClientRect();
        if (r.bottom > visTop + 0.5) { if (r.top < visTop - 0.5) scroller.scrollTop -= visTop - r.top; break; }
      }
    }
  }

  if ('ResizeObserver' in globalThis) {
    resizeObserver = new ResizeObserver(() => drawDims());
    resizeObserver.observe(table);
  }

  return { node, update, destroy: () => resizeObserver?.disconnect() };
}
