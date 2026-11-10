// Facts about the machine that the interface needs (SPEC §2, §5, §15).
// Shared by views, the mock and the pure row models.

export const STATUS = Object.freeze({
  PAUSED: 0,
  HALTED: 1,
  FAULT: 2,
  BREAKPOINT: 3,
  WAITING_INPUT: 4,
  BUDGET_EXCEEDED: 5,
  NOT_ASSEMBLED: 6,
  ERROR: 0xffffffff,
});

export const STATUS_NAME = Object.freeze({
  0: 'paused', 1: 'halted', 2: 'fault', 3: 'breakpoint', 4: 'waiting', 5: 'budget', 6: 'not_assembled',
});

export const REG_NAMES = Object.freeze(['r0', 'r1', 'r2', 'r3', 'r4', 'r5', 'r6', 'r7', 'fp', 'sp']);
/** Index used by setReg for the flag word (UI contract with vm.js). */
export const REG_FLAGS = 10;
export const FLAG_NAMES = Object.freeze(['Z', 'N', 'C', 'V']);

export const SEGMENTS = Object.freeze([
  { name: 'null', from: 0x0000, to: 0x0100 },
  { name: 'text', from: 0x0100, to: 0x4000 },
  { name: 'data', from: 0x4000, to: 0x6000 },
  { name: 'input', from: 0x6000, to: 0x8000 },
  { name: 'heap', from: 0x8000, to: 0xc000 },
  { name: 'guard', from: 0xc000, to: 0xd000 },
  { name: 'stack', from: 0xd000, to: 0xf000 },
  { name: 'screen', from: 0xf000, to: 0x10000 },
].map(Object.freeze));

export const SCREEN = 0xf000;
export const SCREEN_W = 64;
export const SCREEN_H = 64;
export const STACK_TOP = 0xf000;
export const HEAP = 0x8000;

export function segmentOf(addr) {
  return SEGMENTS.find((s) => addr >= s.from && addr < s.to) ?? null;
}

export const hex = (value, digits = 8) => (value >>> 0).toString(16).toUpperCase().padStart(digits, '0');
export const hex4 = (value) => hex(value, 4);
export const signed = (value) => value | 0;

/** Read a little-endian u32 from a byte array. */
export function readWord(mem, addr) {
  return (mem[addr] | (mem[addr + 1] << 8) | (mem[addr + 2] << 16) | (mem[addr + 3] << 24)) >>> 0;
}

/** Bytes [0x00..0x1F, 0x7F] as visible control pictures (␀ ␁ … ␡). */
export function visibleChar(byte) {
  const b = byte & 0xff;
  if (b < 0x20) return String.fromCharCode(0x2400 + b);
  if (b === 0x7f) return '␡';
  if (b >= 0x80) return '·';
  return String.fromCharCode(b);
}
