// Row model for the stack view: pure, no DOM.
// Each row is one word between sp and the top of the stack (or a window of it),
// annotated with its role in a call frame (SPEC §9).
import { STACK_TOP, readWord } from './machine.js';

export const MAX_ROWS = 96;

/**
 * @param {Uint8Array} mem
 * @param {{sp:number, fp:number}} regs
 * @param {Array<{fp:number, ret:number, func:string|null, size:number}>} frames innermost first
 * @param {{changed?:Set<number>, symbols?:Map<number,string>}} opt
 */
export function stackRows(mem, regs, frames, opt = {}) {
  const rows = [];
  const sp = regs.sp >>> 0;
  if (sp < 0xd000 || sp > STACK_TOP || sp % 4 !== 0) return { rows, truncated: false };
  const roles = new Map();
  frames.forEach((f, index) => {
    roles.set(f.fp, { kind: 'oldfp', frame: index });
    roles.set(f.fp + 4, { kind: 'ret', frame: index });
  });
  // Locals: between the frame's lowest word and its fp. Args: above the return address
  // up to the caller's frame.
  frames.forEach((f, index) => {
    const low = index === 0 ? sp : frames[index - 1].fp + 8;
    for (let a = f.fp - 4; a >= low && a >= f.fp - Math.max(0, f.size); a -= 4) {
      if (!roles.has(a)) roles.set(a, { kind: 'local', frame: index, offset: a - f.fp });
    }
  });
  let truncated = false;
  for (let a = sp; a < STACK_TOP; a += 4) {
    if (rows.length >= MAX_ROWS) { truncated = true; break; }
    const value = readWord(mem, a);
    const role = roles.get(a) ?? { kind: 'saved', frame: frameIndexFor(frames, a) };
    let changed = false;
    for (let k = 0; k < 4; k++) if (opt.changed?.has(a + k)) changed = true;
    rows.push({
      addr: a,
      value,
      kind: role.kind,
      frame: role.frame,
      offset: role.offset ?? (role.frame !== undefined && role.frame >= 0 && frames[role.frame] ? a - frames[role.frame].fp : null),
      target: role.kind === 'ret' ? opt.symbols?.get(value) ?? null : null,
      isSp: a === sp,
      isFp: a === (regs.fp >>> 0),
      changed,
    });
  }
  return { rows, truncated };
}

function frameIndexFor(frames, addr) {
  for (let i = 0; i < frames.length; i++) if (addr <= frames[i].fp + 4) return i;
  return -1;
}

/**
 * Dimension chains for a frame: the distances the user should learn to see.
 * Returns [{from, to, kind}] address pairs (inclusive word starts).
 */
export function frameDimensions(regs, frames) {
  const dims = [];
  const sp = regs.sp >>> 0;
  frames.forEach((f, index) => {
    const low = index === 0 ? sp : frames[index - 1].fp + 8;
    if (f.fp > low) dims.push({ from: low, to: f.fp, kind: 'locals', bytes: f.fp - low, frame: index });
    dims.push({ from: f.fp, to: f.fp + 4, kind: 'ret', bytes: 4, frame: index });
  });
  return dims;
}
