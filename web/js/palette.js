// The 16 screen colours. Names and indices are fixed by the ISA (SPEC §6);
// the RGB values are a design decision and live only here, never in the core.
//
// How the values were chosen
// - Each colour was set in OKLCH, keeping the hue its name promises
//   (BROWN looks brown, CYAN looks cyan) and then tuning lightness and chroma
//   with a small search that maximised the smallest pairwise OKLab distance
//   under normal vision and under simulated protanopia, deuteranopia and
//   tritanopia (Machado et al. 2009, full severity).
// - Result: smallest distance 0.067 (OKLab ΔE) across all four simulations,
//   e.g. LIGHTGRAY/PINK under protanopia. The classic CGA set drops to 0.006
//   (BLUE/MAGENTA under protanopia), i.e. indistinguishable.
// - The trick is lightness: almost every colour sits on its own lightness step
//   between 0.16 (BLACK) and 1.0 (WHITE), so even with hue information lost the
//   pixels stay apart. Hue then separates along the blue–yellow axis, which
//   red–green colour vision keeps.
// - The palette is the same in the light and the dark theme: pixels are data,
//   and a test image must look identical everywhere. The canvas always sits on
//   index 0 (BLACK); every other colour has at least 2:1 contrast to it, and
//   the canvas frame separates the screen from the page in both themes.
// - Colour is never the only carrier: the screen inspector always names the
//   colour under the pointer, and the Soll/Ist diff marks mismatches with a
//   pattern.

export const PALETTE = Object.freeze([
  { index: 0, name: 'BLACK', hex: '#080E13' },
  { index: 1, name: 'BLUE', hex: '#2450BC' },
  { index: 2, name: 'GREEN', hex: '#09B94B' },
  { index: 3, name: 'CYAN', hex: '#088697' },
  { index: 4, name: 'RED', hex: '#CA3929' },
  { index: 5, name: 'MAGENTA', hex: '#823188' },
  { index: 6, name: 'BROWN', hex: '#6D3903' },
  { index: 7, name: 'LIGHTGRAY', hex: '#ABABAB' },
  { index: 8, name: 'DARKGRAY', hex: '#57595C' },
  { index: 9, name: 'LIGHTBLUE', hex: '#75BCFE' },
  { index: 10, name: 'LIGHTGREEN', hex: '#B5F8C2' },
  { index: 11, name: 'LIGHTCYAN', hex: '#6AF3FA' },
  { index: 12, name: 'LIGHTRED', hex: '#ED9276' },
  { index: 13, name: 'PINK', hex: '#E8B4D0' },
  { index: 14, name: 'YELLOW', hex: '#F3E157' },
  { index: 15, name: 'WHITE', hex: '#FFFFFF' },
].map(Object.freeze));

/** [r, g, b] bytes for each colour index. */
export const RGB = Object.freeze(PALETTE.map((c) => Object.freeze([1, 3, 5].map((i) => parseInt(c.hex.slice(i, i + 2), 16)))));

/** Colour index for a screen byte (upper bits are stored but ignored). */
export function colorIndex(byte) {
  return byte & 0x0f;
}

export function colorName(byte) {
  return PALETTE[colorIndex(byte)].name;
}
