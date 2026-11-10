// The 64×64 screen: Ist (current memory), Soll (expected image) and the
// difference, marked with a red-pencil hatch per wrong pixel. The pointer
// readout names coordinate, address, value and colour.
import { el, mount, setText } from '../dom.js';
import { PALETTE, RGB, colorIndex } from '../palette.js';
import { SCREEN, hex } from '../machine.js';

const MODES = ['ist', 'soll', 'diff'];

/** Colour chip; the colour is set through the CSSOM (allowed by the CSP). */
export function swatch(index) {
  const node = el('i', { class: 'swatch', 'aria-hidden': 'true' });
  node.style.backgroundColor = PALETTE[index].hex;
  return node;
}
const SCALE = 6; // overlay resolution per pixel for the diff hatch

export function createScreen({ t }) {
  const canvas = el('canvas', { class: 'screen-canvas', width: 64, height: 64, role: 'img', 'aria-label': t('screen.title') });
  const overlay = el('canvas', { class: 'screen-overlay', width: 64 * SCALE, height: 64 * SCALE, 'aria-hidden': 'true' });
  const info = el('p', { class: 'screen-info mono' });
  const summary = el('p', { class: 'sr-only', 'aria-live': 'off' });
  const ticks = el('div', { class: 'screen-ticks', 'aria-hidden': 'true' },
    [0, 16, 32, 48, 64].map((v) => el('span', { class: 'tick-x', dataset: { v } }, String(v))),
    [16, 32, 48, 64].map((v) => el('span', { class: 'tick-y', dataset: { v } }, String(v))));
  const segButtons = MODES.map((m) => el('button', {
    type: 'button', class: 'seg-btn', dataset: { mode: m }, 'aria-pressed': m === 'ist' ? 'true' : 'false',
    onclick: () => setMode(m),
  }, t(`screen.mode.${m}`)));
  const seg = el('div', { class: 'seg', role: 'group', 'aria-label': t('screen.modeLabel') }, segButtons);
  const frame = el('div', { class: 'screen-frame' }, canvas, overlay, ticks);
  const node = el('section', { class: 'pane pane-screen', 'aria-labelledby': 'h-screen' },
    el('header', { class: 'pane-head' }, el('h2', { id: 'h-screen' }, t('screen.title')), seg),
    el('div', { class: 'screen-body' }, frame, el('div', { class: 'screen-side' }, info, summary)));

  const ctx = canvas.getContext('2d');
  const octx = overlay.getContext('2d');
  const image = ctx.createImageData(64, 64);
  let mode = 'ist';
  let view = null;
  let hover = null;

  function setMode(m) {
    mode = m;
    for (const b of segButtons) b.setAttribute('aria-pressed', b.dataset.mode === m ? 'true' : 'false');
    draw();
  }

  function pixels() {
    if (!view) return null;
    if (mode === 'soll') return view.expected;
    return view.mem.subarray(SCREEN, SCREEN + 4096);
  }

  function draw() {
    if (!view) return;
    const hasExpected = Boolean(view.expected);
    for (const b of segButtons) if (b.dataset.mode !== 'ist') b.disabled = !hasExpected;
    if (!hasExpected && mode !== 'ist') { mode = 'ist'; setMode('ist'); return; }
    const src = pixels();
    const d = image.data;
    let lit = 0;
    for (let i = 0; i < 4096; i++) {
      const c = colorIndex(src ? src[i] : 0);
      if (c) lit++;
      const [r, g, b] = RGB[c];
      d[i * 4] = r; d[i * 4 + 1] = g; d[i * 4 + 2] = b; d[i * 4 + 3] = 255;
    }
    ctx.putImageData(image, 0, 0);
    octx.clearRect(0, 0, overlay.width, overlay.height);
    let wrong = 0;
    if (mode === 'diff' && view.expected) {
      const cur = view.mem.subarray(SCREEN, SCREEN + 4096);
      const red = getComputedStyle(node).getPropertyValue('--pencil').trim() || '#C6372F';
      octx.strokeStyle = red;
      octx.lineWidth = 1.5;
      for (let i = 0; i < 4096; i++) {
        if (colorIndex(cur[i]) === colorIndex(view.expected[i])) continue;
        wrong++;
        const x = (i % 64) * SCALE, y = Math.floor(i / 64) * SCALE;
        octx.fillStyle = 'rgba(255,255,255,0.55)';
        octx.fillRect(x, y, SCALE, SCALE);
        octx.beginPath();
        octx.moveTo(x, y + SCALE); octx.lineTo(x + SCALE, y);
        octx.moveTo(x, y + SCALE / 2); octx.lineTo(x + SCALE / 2, y);
        octx.moveTo(x + SCALE / 2, y + SCALE); octx.lineTo(x + SCALE, y + SCALE / 2);
        octx.stroke();
      }
    }
    canvas.setAttribute('aria-label', mode === 'diff'
      ? t('screen.ariaDiff', { count: wrong })
      : t('screen.aria', { mode: t(`screen.mode.${mode}`), count: lit }));
    setText(summary, mode === 'diff' ? t('screen.ariaDiff', { count: wrong }) : '');
    renderInfo();
  }

  function renderInfo() {
    if (!hover || !view) {
      mount(info, el('span', { class: 'muted' }, t('screen.hoverHint')));
      return;
    }
    const { x, y } = hover;
    const addr = SCREEN + y * 64 + x;
    const value = view.mem[addr];
    const parts = [
      el('span', {}, el('b', {}, 'x '), String(x)),
      el('span', {}, el('b', {}, 'y '), String(y)),
      el('span', {}, hex(addr, 4)),
      el('span', {}, swatch(colorIndex(value)), `${hex(value, 2)} ${PALETTE[colorIndex(value)].name}`),
    ];
    if (view.expected) {
      const e = view.expected[y * 64 + x];
      parts.push(el('span', { class: colorIndex(e) !== colorIndex(value) ? 'is-wrong' : '' }, `${t('screen.mode.soll')} `,
        swatch(colorIndex(e)), PALETTE[colorIndex(e)].name));
    }
    mount(info, parts);
  }

  function pointer(ev) {
    const r = canvas.getBoundingClientRect();
    const x = Math.floor(((ev.clientX - r.left) / r.width) * 64);
    const y = Math.floor(((ev.clientY - r.top) / r.height) * 64);
    if (x < 0 || y < 0 || x > 63 || y > 63) { hover = null; renderInfo(); return; }
    hover = { x, y };
    renderInfo();
  }
  frame.addEventListener('pointermove', pointer);
  frame.addEventListener('pointerdown', pointer);
  frame.addEventListener('pointerleave', () => { hover = null; renderInfo(); });

  return {
    node,
    update(next) { view = next; draw(); },
    redraw: draw,
  };
}
