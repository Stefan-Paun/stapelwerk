// Row model for the hexdump: pure, no DOM.
import { SEGMENTS, readWord, segmentOf } from './machine.js';

export const ROW_BYTES = 16;

/**
 * Build hexdump rows for [from, to).
 * @param {Uint8Array} mem 64 KiB
 * @param {{from:number,to:number, mode?:'byte'|'word', changed?:Set<number>, labels?:Map<number,string>, pc?:number, sp?:number}} opt
 */
export function hexRows(mem, opt) {
  const mode = opt.mode ?? 'byte';
  const rows = [];
  const from = Math.max(0, opt.from - (opt.from % ROW_BYTES));
  const to = Math.min(0x10000, opt.to);
  for (let addr = from; addr < to; addr += ROW_BYTES) {
    const cells = [];
    if (mode === 'word') {
      for (let a = addr; a < addr + ROW_BYTES; a += 4) {
        let changed = false;
        for (let k = 0; k < 4; k++) if (opt.changed?.has(a + k)) changed = true;
        cells.push({ addr: a, value: readWord(mem, a), changed, pc: a === opt.pc, sp: a === opt.sp });
      }
    } else {
      for (let a = addr; a < addr + ROW_BYTES; a++) {
        cells.push({ addr: a, value: mem[a], changed: Boolean(opt.changed?.has(a)), pc: opt.pc !== undefined && a >= opt.pc && a < opt.pc + 4, sp: a === opt.sp });
      }
    }
    let label = null;
    if (opt.labels) for (let a = addr; a < addr + ROW_BYTES; a++) if (opt.labels.has(a)) { label = opt.labels.get(a); break; }
    rows.push({ addr, segment: segmentOf(addr)?.name ?? null, cells, label });
  }
  return rows;
}

/** The byte addresses whose value differs between two memory images. */
export function diffBytes(before, after, from = 0, to = 0x10000) {
  const out = new Set();
  if (!before || !after) return out;
  for (let a = from; a < to; a++) if (before[a] !== after[a]) out.add(a);
  return out;
}

export function segmentRange(name) {
  return SEGMENTS.find((s) => s.name === name) ?? null;
}
