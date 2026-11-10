import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PALETTE, RGB, colorIndex, colorName } from '../../web/js/palette.js';

const NAMES = ['BLACK', 'BLUE', 'GREEN', 'CYAN', 'RED', 'MAGENTA', 'BROWN', 'LIGHTGRAY', 'DARKGRAY', 'LIGHTBLUE', 'LIGHTGREEN', 'LIGHTCYAN', 'LIGHTRED', 'PINK', 'YELLOW', 'WHITE'];

test('16 valid entries with the names fixed by the ISA', () => {
  assert.equal(PALETTE.length, 16);
  PALETTE.forEach((c, i) => {
    assert.equal(c.index, i);
    assert.equal(c.name, NAMES[i]);
    assert.match(c.hex, /^#[0-9A-F]{6}$/);
    assert.equal(RGB[i].length, 3);
    assert.ok(RGB[i].every((v) => Number.isInteger(v) && v >= 0 && v <= 255));
  });
  assert.equal(new Set(PALETTE.map((c) => c.hex)).size, 16);
});

test('upper bits are ignored', () => {
  assert.equal(colorIndex(0xfe), 14);
  assert.equal(colorName(0x1f), 'WHITE');
});

// Machado et al. 2009 (severity 1.0) and OKLab distance: the palette must stay
// apart for the three common colour vision deficiencies.
const M = {
  normal: null,
  protan: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deutan: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.01182, 0.04294, 0.968881]],
  tritan: [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.3039]],
};
const lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
function oklab([r, g, b]) {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s];
}

test('colours stay distinguishable under simulated colour vision deficiency', () => {
  const linear = RGB.map((c) => c.map((v) => lin(v / 255)));
  for (const [kind, mat] of Object.entries(M)) {
    const labs = linear.map((c) => oklab(mat ? mat.map((row) => Math.min(1, Math.max(0, row[0] * c[0] + row[1] * c[1] + row[2] * c[2]))) : c));
    let min = Infinity;
    let pair = '';
    for (let i = 0; i < 16; i++) for (let j = i + 1; j < 16; j++) {
      const d = Math.hypot(labs[i][0] - labs[j][0], labs[i][1] - labs[j][1], labs[i][2] - labs[j][2]);
      if (d < min) { min = d; pair = `${NAMES[i]}/${NAMES[j]}`; }
    }
    assert.ok(min >= 0.06, `${kind}: ${pair} only ${min.toFixed(3)} apart`);
  }
});
