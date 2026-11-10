// Authored icons: one 24-unit grid, one 1.6 stroke, square caps like a
// technical pen. Built with createElementNS; no markup strings.
import { svg } from './dom.js';

const P = {
  step: ['M5 5l8 7-8 7', 'M17 5v14'],
  back: ['M19 5l-8 7 8 7', 'M7 5v14'],
  run: ['M7 4.5l12 7.5-12 7.5z'],
  pause: ['M8 5v14', 'M16 5v14'],
  restart: ['M5 12a7 7 0 1 0 2.05-4.95', 'M5 4v4h4'],
  check: ['M4.5 12.5l5 5 10-11'],
  assemble: ['M8 6l-5 6 5 6', 'M16 6l5 6-5 6', 'M13.5 4l-3 16'],
  cross: ['M6 6l12 12', 'M18 6L6 18'],
  test: ['M4 12.5l4.5 4.5L20 5.5', 'M4 20h16'],
  lock: ['M6 11h12v9H6z', 'M8.5 11V8a3.5 3.5 0 0 1 7 0v3'],
  chevron: ['M9 5l7 7-7 7'],
  chevronDown: ['M5 9l7 7 7-7'],
  sun: ['M12 7.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9z', 'M12 2v2.5', 'M12 19.5V22', 'M2 12h2.5', 'M19.5 12H22', 'M4.9 4.9l1.8 1.8', 'M17.3 17.3l1.8 1.8', 'M4.9 19.1l1.8-1.8', 'M17.3 6.7l1.8-1.8'],
  moon: ['M19 14.5A8 8 0 0 1 9.5 5a8 8 0 1 0 9.5 9.5z'],
  download: ['M12 4v11', 'M7 10.5l5 5 5-5', 'M5 20h14'],
  upload: ['M12 20V9', 'M7 13.5l5-5 5 5', 'M5 4h14'],
  trash: ['M5 7h14', 'M9 7V4.5h6V7', 'M7 7l1 13h8l1-13'],
  link: ['M10 14l4-4', 'M8.5 11.5L6 14a3 3 0 0 0 4 4l2.5-2.5', 'M15.5 12.5L18 10a3 3 0 0 0-4-4l-2.5 2.5'],
  keyboard: ['M3 6h18v12H3z', 'M6.5 9.5h1', 'M10.5 9.5h1', 'M14.5 9.5h1', 'M17.5 9.5h.5', 'M7 14.5h10'],
  pencil: ['M4 20l1-4L16 5l3 3L8 19z', 'M14 7l3 3'],
  rewindChange: ['M12 5v14', 'M5 12l-2.5-2.5', 'M5 12l-2.5 2.5', 'M3 12h6', 'M15 9h6v6h-6z'],
};

/** Inline SVG icon; decorative (aria-hidden) unless a label is given. */
export function icon(name, label = null) {
  const node = svg('svg', {
    class: `icon icon-${name}`,
    viewBox: '0 0 24 24',
    width: 20,
    height: 20,
    'aria-hidden': label ? null : 'true',
    role: label ? 'img' : null,
    'aria-label': label,
    focusable: 'false',
  });
  for (const d of P[name] ?? []) node.append(svg('path', { d }));
  return node;
}

/** Breakpoint marker: a filled disc, the one universal debugger sign. */
export function breakpointMark() {
  return svg('svg', { class: 'bp-mark', viewBox: '0 0 12 12', width: 10, height: 10, 'aria-hidden': 'true' },
    svg('circle', { cx: 6, cy: 6, r: 5 }));
}

/** Word mark: three stacked plates with a dimension line beside them. */
export function logo() {
  return svg('svg', { class: 'logo', viewBox: '0 0 32 32', width: 28, height: 28, 'aria-hidden': 'true' },
    svg('path', { class: 'logo-dim', d: 'M4 6v20M2 8l4-4M2 28l4-4M4 6h3M4 26h3' }),
    svg('rect', { x: 11, y: 6, width: 18, height: 5 }),
    svg('rect', { x: 11, y: 13.5, width: 18, height: 5 }),
    svg('rect', { class: 'logo-top', x: 11, y: 21, width: 18, height: 5 }));
}
