// Contrast of the text tokens in both themes, computed from web/css/tokens.css (WCAG 2.x, AA).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const css = readFileSync(new URL('../../web/css/tokens.css', import.meta.url), 'utf8');

function block(selectorStart) {
  const i = css.indexOf(selectorStart);
  assert.ok(i >= 0, `missing ${selectorStart}`);
  const open = css.indexOf('{', i);
  const close = css.indexOf('}', open);
  const vars = {};
  for (const m of css.slice(open + 1, close).matchAll(/--([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\s*;/g)) vars[m[1]] = m[2];
  return vars;
}

const light = block(':root {');
const dark = { ...light, ...block(":root[data-theme='dark'] {") };

function lum(hex) {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}
export function contrast(a, b) {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

// [foreground, background] pairs that carry text.
const PAIRS = [
  ['ink', 'paper'], ['ink-2', 'paper'], ['ink-3', 'paper'],
  ['ink', 'paper-2'], ['ink-2', 'paper-2'], ['ink-3', 'paper-2'],
  ['ink', 'paper-3'], ['ink-2', 'paper-3'], ['ink-3', 'paper-3'],
  ['blue', 'paper'], ['blue', 'paper-2'], ['blue', 'paper-3'], ['blue-on', 'blue'],
  ['pencil', 'paper'], ['pencil', 'paper-3'], ['pencil', 'pencil-wash'], ['ink', 'pencil-wash'],
  ['stamp', 'paper'], ['stamp', 'paper-3'], ['ink', 'stamp-wash'], ['stamp', 'stamp-wash'],
  ['ochre', 'paper'], ['ochre', 'ochre-wash'], ['ink', 'ochre-wash'],
  ['ink', 'blue-wash'],
  ['head-ink', 'head-bg'], ['head-ink-2', 'head-bg'],
];

for (const [name, vars] of [['light', light], ['dark', dark]]) {
  test(`${name} theme: text tokens reach AA 4.5:1`, () => {
    const failures = [];
    for (const [fg, bg] of PAIRS) {
      assert.ok(vars[fg] && vars[bg], `${name}: token ${fg} or ${bg} missing`);
      const c = contrast(vars[fg], vars[bg]);
      if (c < 4.5) failures.push(`${fg} on ${bg}: ${c.toFixed(2)}`);
    }
    assert.deepEqual(failures, []);
  });
}
