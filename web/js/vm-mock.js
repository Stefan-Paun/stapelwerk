// Stand-in for vm.js while the Rust core is not built (or with ?mock=1).
//
// It mirrors the vm.js contract (SPEC §15) so every view can be built and
// looked at without WebAssembly:
//
//   const vm = await loadVm();               // real: loadVm(urlOrBytes)
//   vm.levelIndex()                          // mock only: built-in demo levels
//   const lv = vm.loadLevel(textOrId);       // real: .lvl text → handle; mock: id
//   lv.info(), lv.section(name), lv.free()
//   const s = vm.newSession(lv | null);      // null = sandbox
//   s.assemble(src) → {ok, size, diagnostics:[{key,file,line,col,len,args}]}
//   s.reset(test) s.step(n) s.run(maxCycles) s.back(n) s.goto(step)
//   s.backToChange(kind, target) s.setReg(reg, value)   (reg 10 = flag word)
//   s.setBreakpoint(addr, on) s.clearBreakpoints()
//   s.pushInt(v) s.pushText(str) s.closeInput()
//   s.state() → decoded StateView {status, pc, regs[10], flags, steps, cycles,
//               faultCode, faultAddr, outLen, inIntPos, inIntLen, inTextPos,
//               inTextLen, testIndex, budget}
//   s.memory() s.shadow() s.output() s.fault() s.symbols() s.lineMap()
//   s.lineCounts() s.frames() s.heap() s.disasm(addr, n) s.check()
//   s.runTests() s.digest() s.free()
//   s.referenceScreen(test) → Uint8Array(4096) | null   (Soll image)
//   s.horizon() → furthest step reached on the current timeline (UI helper)
//
// The machine is a faithful-enough subset: real encodings, flags, cycle costs,
// segment checks, heap with red zones, frames, time travel via snapshots.
// It is not the reference implementation; the Rust core is.

import { STATUS, SEGMENTS, readWord } from './machine.js';
import { MOCK_CHAPTERS, MOCK_LEVELS } from './mock-levels.js';

const REGS = { r0: 0, r1: 1, r2: 2, r3: 3, r4: 4, r5: 5, r6: 6, r7: 7, fp: 8, sp: 9 };
const ALU = ['add', 'sub', 'mul', 'div', 'divu', 'rem', 'remu', 'and', 'or', 'xor', 'shl', 'shr', 'sar', 'cmp', 'tst'];
const IMM_KIND = ['s', 's', 's', 's', 'u', 's', 'u', 'u', 'u', 'u', 'sh', 'sh', 'sh', 's', 'u'];
const COND = { jmp: 0, jz: 1, je: 1, jnz: 2, jne: 2, jlt: 3, jge: 4, jle: 5, jgt: 6, jb: 7, jae: 8, jbe: 9, ja: 10, js: 11, jns: 12, jo: 13, jno: 14 };
const COND_NAME = ['jmp', 'jz', 'jnz', 'jlt', 'jge', 'jle', 'jgt', 'jb', 'jae', 'jbe', 'ja', 'js', 'jns', 'jo', 'jno'];
const SYSCALLS = { READ_INT: 1, WRITE_INT: 2, READ_CHAR: 3, WRITE_CHAR: 4, WRITE_STR: 5, ALLOC: 6, FREE: 7, RAND: 8 };
const SYS_NAME = Object.fromEntries(Object.entries(SYSCALLS).map(([k, v]) => [v, k]));
const COLORS = ['BLACK', 'BLUE', 'GREEN', 'CYAN', 'RED', 'MAGENTA', 'BROWN', 'LIGHTGRAY', 'DARKGRAY', 'LIGHTBLUE', 'LIGHTGREEN', 'LIGHTCYAN', 'LIGHTRED', 'PINK', 'YELLOW', 'WHITE'];
const BUILTINS = new Map([
  ['SCREEN', 0xf000], ['SCREEN_W', 64], ['SCREEN_H', 64], ['INPUT', 0x6000], ['HEAP', 0x8000], ['STACK_TOP', 0xf000],
  ...COLORS.map((c, i) => [c, i]),
]);
const MNEMONICS = ['halt', 'nop', 'sys', 'ret', 'mov', 'movhi', 'li', 'lea', ...ALU, 'neg', 'not', 'ld', 'ldb', 'ldbs', 'st', 'stb', 'push', 'pop', 'call', ...Object.keys(COND)];
const FZ = 1, FN = 2, FC = 4, FV = 8;
const SHADOW_FREE = 0, SHADOW_LIVE = 1, SHADOW_FREED = 2, SHADOW_RED = 3;

// ---------------------------------------------------------------- assembler

// Damerau-Levenshtein (optimal string alignment), as in the core assembler.
function levenshtein(a, b) {
  const d = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)]);
  for (let j = 1; j <= b.length; j++) d[0][j] = j;
  for (let i = 1; i <= a.length; i++) for (let j = 1; j <= b.length; j++) {
    d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) d[i][j] = Math.min(d[i][j], d[i - 2][j - 2] + 1);
  }
  return d[a.length][b.length];
}

function stripComment(line) {
  let quote = null;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (quote) { if (c === '\\') i++; else if (c === quote) quote = null; continue; }
    if (c === '"' || c === "'") quote = c;
    else if (c === ';') return line.slice(0, i);
  }
  return line;
}

function splitOperands(text) {
  const out = [];
  let depth = 0, quote = null, cur = '', start = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) { cur += c; if (c === '\\') { cur += text[++i] ?? ''; } else if (c === quote) quote = null; continue; }
    if (c === '"' || c === "'") quote = c;
    if (c === '[') depth++;
    if (c === ']') depth--;
    if (c === ',' && depth === 0) { out.push({ text: cur.trim(), offset: start + cur.search(/\S|$/) }); cur = ''; start = i + 1; continue; }
    cur += c;
  }
  if (cur.trim() !== '' || out.length) out.push({ text: cur.trim(), offset: start + cur.search(/\S|$/) });
  return out;
}

class AsmError extends Error {
  constructor(key, args = {}, col = 0, len = 1) { super(key); this.key = key; this.args = args; this.col = col; this.len = len; }
}

function parseNumber(tok) {
  const t = tok.replace(/_/g, '');
  if (/^0x[0-9a-f]+$/i.test(t)) return Number.parseInt(t.slice(2), 16);
  if (/^0b[01]+$/i.test(t)) return Number.parseInt(t.slice(2), 2);
  if (/^[0-9]+$/.test(t)) return Number.parseInt(t, 10);
  return null;
}

const ESC = { n: 10, t: 9, 0: 0, '\\': 92, '"': 34, "'": 39 };
function unescape(body) {
  const bytes = [];
  const enc = new TextEncoder();
  for (let i = 0; i < body.length; i++) {
    const c = body[i];
    if (c === '\\') {
      const n = body[++i];
      if (n === 'x') { bytes.push(Number.parseInt(body.slice(i + 1, i + 3), 16)); i += 2; continue; }
      if (!(n in ESC)) throw new AsmError('asm.bad_string');
      bytes.push(ESC[n]);
    } else bytes.push(...enc.encode(c));
  }
  return bytes;
}

/** Expression evaluator with C precedence. lookup(name) → number | undefined. */
function evaluate(text, lookup) {
  const toks = text.match(/0x[0-9a-fA-F_]+|0b[01_]+|[0-9][0-9_]*|'(?:\\.|[^'])'|[A-Za-z_.][A-Za-z0-9_.]*|<<|>>|[-+*/%&|^~()]|\S/g) ?? [];
  let i = 0;
  const peek = () => toks[i];
  const next = () => toks[i++];
  function primary() {
    const t = next();
    if (t === undefined) throw new AsmError('asm.operand_type', { expected: 'expr' });
    if (t === '(') { const v = binary(0); if (next() !== ')') throw new AsmError('asm.unexpected_char', { ch: ')' }); return v; }
    if (t === '-') return -primary();
    if (t === '~') return ~primary();
    if (t[0] === "'") { const b = unescape(t.slice(1, -1)); return b[0] ?? 0; }
    const n = parseNumber(t);
    if (n !== null) return n;
    if (/^[A-Za-z_.]/.test(t)) {
      const v = lookup(t);
      if (v === undefined) throw new AsmError('asm.undefined_symbol', { name: t });
      return v;
    }
    throw new AsmError('asm.unexpected_char', { ch: t });
  }
  const PREC = [['|'], ['^'], ['&'], ['<<', '>>'], ['+', '-'], ['*', '/', '%']];
  function binary(level) {
    if (level >= PREC.length) return primary();
    let v = binary(level + 1);
    while (PREC[level].includes(peek())) {
      const op = next();
      const r = binary(level + 1);
      if ((op === '/' || op === '%') && r === 0) throw new AsmError('asm.expr_div_zero');
      v = op === '|' ? v | r : op === '^' ? v ^ r : op === '&' ? v & r : op === '<<' ? v * 2 ** r : op === '>>' ? Math.floor(v / 2 ** r)
        : op === '+' ? v + r : op === '-' ? v - r : op === '*' ? v * r : op === '/' ? Math.trunc(v / r) : v % r;
    }
    return v;
  }
  const v = binary(0);
  if (i < toks.length) throw new AsmError('asm.unexpected_char', { ch: toks[i] });
  return v;
}

function encR(op, a = 0, b = 0, c = 0) { return ((op << 24) | (a << 20) | (b << 16) | (c << 12)) >>> 0; }
function encI(op, a, b, imm) { return ((op << 24) | (a << 20) | (b << 16) | (imm & 0xffff)) >>> 0; }
function encJ(op, a, imm) { return ((op << 24) | (a << 20) | (imm & 0xfffff)) >>> 0; }

/**
 * Assemble player source (file 0) and optional library (file 1).
 * Returns {ok, size, diagnostics, program}.
 */
export function assemble(source, library = '', options = {}) {
  const diagnostics = [];
  const symbols = new Map(); // name → {value, kind, file, line}
  const consts = new Map(BUILTINS);
  const items = []; // parsed statements
  const files = [String(source ?? ''), String(library ?? '')];
  let textAddr = 0x100;
  let dataAddr = 0x4000;
  const sectionEnd = [0, 0];

  // Pass 1: parse and collect labels.
  const parsed = [];
  for (let file = 0; file < 2; file++) {
    let section = 'text';
    let global = null;
    files[file].replace(/\r\n?/g, '\n').split('\n').forEach((raw, idx) => {
      const lineNo = idx + 1;
      const err = (e) => diagnostics.push({ key: e.key, file, line: lineNo, col: (e.col ?? 0) + 1, len: e.len ?? 1, args: e.args ?? {} });
      try {
        let line = stripComment(raw);
        let offset = 0;
        const lab = /^\s*([A-Za-z_.][A-Za-z0-9_]*)\s*:/.exec(line);
        if (lab) {
          let name = lab[1];
          const col = line.indexOf(name);
          if (name.startsWith('.')) {
            if (!global) throw new AsmError('asm.local_label_without_global', {}, col, name.length);
            name = `${global}.${name.slice(1)}`;
          } else {
            if (BUILTINS.has(name) || MNEMONICS.includes(name.toLowerCase()) || name.toLowerCase() in REGS) throw new AsmError('asm.reserved_name', { name }, col, name.length);
            global = name;
          }
          if (symbols.has(name)) throw new AsmError('asm.duplicate_label', { name, first_line: symbols.get(name).line }, col, name.length);
          parsed.push({ kind: 'label', name, file, line: lineNo, section, col });
          offset = lab[0].length;
          line = line.slice(offset);
          symbols.set(name, { value: null, kind: section === 'text' ? 'code' : 'data', file, line: lineNo });
        }
        const m = /^(\s*)(\.?[A-Za-z_][A-Za-z0-9_]*)(.*)$/.exec(line);
        if (!m) {
          if (line.trim() !== '') throw new AsmError('asm.unexpected_char', { ch: line.trim()[0] }, offset + line.search(/\S/));
          return;
        }
        const mnemonic = m[2].toLowerCase();
        const mcol = offset + m[1].length;
        const operands = splitOperands(m[3]).map((o) => ({ text: o.text, col: offset + m[1].length + m[2].length + o.offset }));
        if (mnemonic === '.text' || mnemonic === '.data') { section = mnemonic.slice(1); return; }
        if (mnemonic === '.equ') {
          const name = operands[0]?.text;
          if (!name || operands.length !== 2) throw new AsmError('asm.operand_count', { expected: 2, got: operands.length }, mcol, 4);
          consts.set(name, evaluate(operands[1].text, (n) => consts.get(n)));
          return;
        }
        if (mnemonic.startsWith('.')) {
          if (section !== 'data') throw new AsmError('asm.data_in_text', {}, mcol, mnemonic.length);
          parsed.push({ kind: 'data', directive: mnemonic, operands, file, line: lineNo, col: mcol });
          return;
        }
        if (!MNEMONICS.includes(mnemonic)) {
          let suggestion = null;
          let best = [3];
          for (const cand of MNEMONICS) {
            const rank = [levenshtein(mnemonic, cand), cand[0] === mnemonic[0] ? 0 : 1, cand.length === mnemonic.length ? 0 : 1];
            if (rank[0] < mnemonic.length && (rank[0] < best[0] || (rank[0] === best[0] && (rank[1] < best[1] || (rank[1] === best[1] && rank[2] < best[2]))))) { best = rank; suggestion = cand; }
          }
          throw new AsmError('asm.unknown_mnemonic', suggestion ? { name: m[2], suggestion } : { name: m[2] }, mcol, m[2].length);
        }
        if (section !== 'text') throw new AsmError('asm.code_in_data', {}, mcol, mnemonic.length);
        if (file === 0 && options.forbid?.includes(mnemonic)) throw new AsmError('asm.forbidden_instruction', { mnemonic }, mcol, mnemonic.length);
        if (file === 0 && options.allow && !options.allow.includes(mnemonic)) throw new AsmError('asm.forbidden_instruction', { mnemonic }, mcol, mnemonic.length);
        parsed.push({ kind: 'instr', mnemonic, operands, file, line: lineNo, col: mcol, words: mnemonic === 'li' ? 2 : 1 });
      } catch (e) {
        if (e instanceof AsmError) err(e); else throw e;
      }
    });
    if (diagnostics.length > 50) break;
  }

  // Layout: player text, library text, trap; player data, library data.
  const dataBytes = [];
  for (const file of [0, 1]) {
    for (const st of parsed.filter((p) => p.file === file)) {
      if (st.kind === 'label') {
        if (st.section === 'text') symbols.get(st.name).value = textAddr; // patched below for data labels
        else symbols.get(st.name).value = dataAddr;
      } else if (st.kind === 'instr') { st.addr = textAddr; textAddr += 4 * st.words; }
      else if (st.kind === 'data') {
        try {
          const align = st.directive === '.word' ? 4 : st.directive === '.align' ? Math.max(1, evaluate(st.operands[0]?.text ?? '1', (n) => BUILTINS.get(n))) : 1;
          dataAddr = Math.ceil(dataAddr / align) * align;
          st.addr = dataAddr;
          st.size = dataSize(st);
        } catch (e) {
          st.addr = dataAddr;
          st.size = 0;
          if (e instanceof AsmError) diagnostics.push({ key: e.key, file: st.file, line: st.line, col: st.col + 1, len: 1, args: e.args });
        }
        dataAddr += st.size;
      }
    }
    sectionEnd[file] = textAddr;
  }
  const trap = textAddr;
  textAddr += 4;
  if (textAddr > 0x4000) diagnostics.push({ key: 'asm.section_overflow', file: 0, line: 1, col: 1, len: 1, args: { section: 'text', size: textAddr - 0x100, max: 0x3f00 } });
  // Fix data labels that precede data statements in the same line order.
  for (const st of parsed) {
    if (st.kind === 'label' && st.section === 'data') {
      const nextData = parsed.slice(parsed.indexOf(st) + 1).find((p) => p.kind === 'data' && p.file === st.file);
      if (nextData) symbols.get(st.name).value = nextData.addr;
    }
  }

  const lookup = (name) => {
    if (symbols.has(name)) return symbols.get(name).value;
    if (consts.has(name)) return consts.get(name);
    return undefined;
  };
  // Pass 2: encode.
  const instrs = new Map();
  const lineMap = [];
  const mem = new Map(); // addr → byte, for text + data
  const putWord = (addr, w) => { for (let k = 0; k < 4; k++) mem.set(addr + k, (w >>> (8 * k)) & 0xff); };
  let currentGlobal = [null, null];
  for (const st of parsed) {
    if (st.kind === 'label') { if (!st.name.includes('.')) currentGlobal[st.file] = st.name; continue; }
    const resolve = (name) => {
      if (name.startsWith('.') && currentGlobal[st.file]) return lookup(`${currentGlobal[st.file]}.${name.slice(1)}`);
      return lookup(name);
    };
    try {
      if (st.kind === 'data') { encodeData(st, resolve, mem); continue; }
      const list = encodeInstr(st, resolve, trap);
      list.forEach((ins, k) => {
        ins.addr = st.addr + 4 * k;
        ins.file = st.file;
        ins.line = st.line;
        instrs.set(ins.addr, ins);
        putWord(ins.addr, ins.word);
        lineMap.push({ a: ins.addr, f: st.file, l: st.line });
      });
    } catch (e) {
      if (e instanceof AsmError) diagnostics.push({ key: e.key, file: st.file, line: st.line, col: (e.col || st.col) + 1, len: e.len, args: e.args });
      else throw e;
    }
  }
  const trapIns = { m: 'halt', word: 0x01000000, text: 'halt', addr: trap, file: 1, line: 0, trap: true };
  instrs.set(trap, trapIns);
  putWord(trap, trapIns.word);

  diagnostics.sort((a, b) => a.file - b.file || a.line - b.line || a.col - b.col);
  if (diagnostics.length > 50) diagnostics.splice(50, Infinity, { key: 'asm.too_many_errors', file: 0, line: diagnostics[49].line, col: 1, len: 1, args: {} });
  const size = parsed.filter((p) => p.kind === 'instr' && p.file === 0).reduce((n, p) => n + p.words, 0);
  const symbolList = [...symbols.entries()].map(([name, s]) => ({ name, kind: s.kind, value: s.value, file: s.file, line: s.line }));
  symbolList.push({ name: '__trap', kind: 'code', value: trap, file: 1, line: 0 });
  return {
    ok: diagnostics.length === 0,
    size,
    diagnostics,
    program: diagnostics.length ? null : { instrs, mem, lineMap, symbols: symbolList, trap, textEnd: textAddr, codeEnd: sectionEnd[1] },
  };
}

function dataSize(st) {
  const ops = st.operands;
  switch (st.directive) {
    case '.word': return 4 * ops.length;
    case '.byte': return ops.length;
    case '.string': case '.ascii': {
      const s = ops[0]?.text ?? '';
      if (!/^".*"$/.test(s)) throw new AsmError('asm.bad_string');
      return unescape(s.slice(1, -1)).length + (st.directive === '.string' ? 1 : 0);
    }
    case '.space': return Math.max(0, evaluate(ops[0]?.text ?? '0', (n) => BUILTINS.get(n)));
    case '.align': return 0;
    default: throw new AsmError('asm.unknown_mnemonic', { name: st.directive });
  }
}

function encodeData(st, resolve, mem) {
  let a = st.addr;
  if (st.directive === '.word') for (const op of st.operands) { const v = evaluate(op.text, resolve) >>> 0; for (let k = 0; k < 4; k++) mem.set(a++, (v >>> (8 * k)) & 0xff); }
  else if (st.directive === '.byte') for (const op of st.operands) mem.set(a++, evaluate(op.text, resolve) & 0xff);
  else if (st.directive === '.string' || st.directive === '.ascii') {
    for (const b of unescape(st.operands[0].text.slice(1, -1))) mem.set(a++, b);
    if (st.directive === '.string') mem.set(a++, 0);
  }
}

function encodeInstr(st, resolve, trap) {
  const { mnemonic: mn, operands: ops } = st;
  const need = (...counts) => {
    if (!counts.includes(ops.length)) throw new AsmError('asm.operand_count', { expected: counts[0], got: ops.length }, st.col, mn.length);
  };
  const reg = (op) => {
    const r = REGS[op.text.toLowerCase()];
    if (r === undefined) {
      if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(op.text) && /^r\d+$/i.test(op.text)) throw new AsmError('asm.unknown_register', { name: op.text }, op.col, op.text.length);
      throw new AsmError('asm.operand_type', { expected: 'register' }, op.col, op.text.length || 1);
    }
    return r;
  };
  const isReg = (op) => op && REGS[op.text.toLowerCase()] !== undefined;
  const evalOp = (op, text = op.text) => {
    try { return evaluate(text, resolve); } catch (e) {
      if (e instanceof AsmError && !e.col) { e.col = op.col; e.len = Math.max(1, op.text.length); }
      throw e;
    }
  };
  const imm = (op, kind) => {
    if (op.text.startsWith('[')) throw new AsmError('asm.operand_type', { expected: 'immediate' }, op.col, op.text.length);
    const v = evalOp(op);
    const [min, max] = kind === 's' ? [-32768, 32767] : kind === 'u' ? [0, 65535] : kind === 'sh' ? [0, 31] : [-(2 ** 31), 2 ** 32 - 1];
    if (v < min || v > max) throw new AsmError('asm.imm_range', { value: v, min, max }, op.col, op.text.length);
    return v;
  };
  const memop = (op) => {
    const m = /^\[\s*(.*?)\s*\]$/.exec(op.text);
    if (!m) throw new AsmError('asm.operand_type', { expected: 'memory' }, op.col, op.text.length);
    const inner = m[1];
    const rm = /^([A-Za-z0-9]+)\s*(?:([+-])\s*(.+))?$/.exec(inner);
    if (rm && REGS[rm[1].toLowerCase()] !== undefined) {
      const off = rm[2] ? (rm[2] === '-' ? -1 : 1) * evalOp(op, rm[3]) : 0;
      if (off < -32768 || off > 32767) throw new AsmError('asm.imm_range', { value: off, min: -32768, max: 32767 }, op.col, op.text.length);
      return { base: REGS[rm[1].toLowerCase()], off };
    }
    const abs = evalOp(op, inner);
    if (abs < 0 || abs > 0xffff) throw new AsmError('asm.imm_range', { value: abs, min: 0, max: 65535 }, op.col, op.text.length);
    return { base: 15, off: abs };
  };
  const rn = (r) => Object.keys(REGS)[r];
  const memText = ({ base, off }) => (base === 15 ? `[0x${off.toString(16)}]` : off === 0 ? `[${rn(base)}]` : `[${rn(base)} ${off < 0 ? '-' : '+'} ${Math.abs(off)}]`);
  switch (mn) {
    case 'halt': need(0); return [{ m: 'halt', word: encR(0x01), text: 'halt' }];
    case 'nop': need(0); return [{ m: 'nop', word: encR(0x02), text: 'nop' }];
    case 'ret': need(0); return [{ m: 'ret', word: encR(0x08), text: 'ret' }];
    case 'sys': {
      need(1);
      const name = ops[0].text.toUpperCase();
      const n = SYSCALLS[name] ?? imm(ops[0], 'u');
      return [{ m: 'sys', n, word: encI(0x04, 0, 0, n), text: `sys ${SYS_NAME[n] ?? n}` }];
    }
    case 'mov': {
      need(2);
      const rd = reg(ops[0]);
      if (isReg(ops[1])) { const rs = reg(ops[1]); return [{ m: 'mov', rd, rs, word: encR(0x10, rd, rs), text: `mov ${rn(rd)}, ${rn(rs)}` }]; }
      const v = evalOp(ops[1]);
      if (v < -32768 || v > 32767) throw new AsmError('asm.imm_range', { value: v, min: -32768, max: 32767, hint: 'li' }, ops[1].col, ops[1].text.length);
      return [{ m: 'movi', rd, imm: v, word: encI(0x11, rd, 0, v), text: `mov ${rn(rd)}, ${v}` }];
    }
    case 'movhi': { need(2); const rd = reg(ops[0]); const v = imm(ops[1], 'u'); return [{ m: 'movhi', rd, imm: v, word: encI(0x12, rd, 0, v), text: `movhi ${rn(rd)}, 0x${v.toString(16)}` }]; }
    case 'li': {
      need(2);
      const rd = reg(ops[0]);
      const v = imm(ops[1], 'li') >>> 0;
      const lo = v & 0xffff, hi = v >>> 16;
      const slo = lo >= 0x8000 ? lo - 0x10000 : lo;
      return [
        { m: 'movi', rd, imm: slo, word: encI(0x11, rd, 0, slo), text: `mov ${rn(rd)}, ${slo}` },
        { m: 'movhi', rd, imm: hi, word: encI(0x12, rd, 0, hi), text: `movhi ${rn(rd)}, 0x${hi.toString(16)}` },
      ];
    }
    case 'lea': { need(2); const rd = reg(ops[0]); const mo = memop(ops[1]); return [{ m: 'lea', rd, ...mo, word: encI(0x13, rd, mo.base, mo.off), text: `lea ${rn(rd)}, ${memText(mo)}` }]; }
    case 'neg': case 'not': {
      need(1, 2);
      const rd = reg(ops[0]);
      const rs = ops.length === 2 ? reg(ops[1]) : rd;
      return [{ m: mn, rd, rs, word: encR(mn === 'neg' ? 0x40 : 0x41, rd, rs), text: `${mn} ${rn(rd)}, ${rn(rs)}` }];
    }
    case 'ld': case 'ldb': case 'ldbs': case 'st': case 'stb': {
      need(2);
      const r = reg(ops[0]);
      const mo = memop(ops[1]);
      const op = { ld: 0x50, ldb: 0x51, ldbs: 0x52, st: 0x54, stb: 0x55 }[mn];
      return [{ m: mn, rd: r, ...mo, word: encI(op, r, mo.base, mo.off), text: `${mn} ${rn(r)}, ${memText(mo)}` }];
    }
    case 'push': case 'pop': { need(1); const r = reg(ops[0]); return [{ m: mn, rd: r, word: encR(mn === 'push' ? 0x58 : 0x59, r), text: `${mn} ${rn(r)}` }]; }
    case 'call': {
      need(1);
      if (isReg(ops[0])) { const r = reg(ops[0]); return [{ m: 'callr', rd: r, word: encR(0x63, r), text: `call ${rn(r)}` }]; }
      const target = evalOp(ops[0]);
      return [{ m: 'call', target, label: ops[0].text, word: encJ(0x61, 0, (target - st.addr) / 4), text: `call ${ops[0].text}` }];
    }
    default: break;
  }
  if (mn in COND) {
    need(1);
    if (mn === 'jmp' && isReg(ops[0])) { const r = reg(ops[0]); return [{ m: 'jmpr', rd: r, word: encR(0x62, r), text: `jmp ${rn(r)}` }]; }
    const cond = COND[mn];
    const target = evalOp(ops[0]);
    return [{ m: 'j', cond, target, label: ops[0].text, word: encJ(0x60, cond, (target - st.addr) / 4), text: `${COND_NAME[cond]} ${ops[0].text}` }];
  }
  const n = ALU.indexOf(mn);
  if (n >= 0) {
    let rd, rs1, src;
    if (mn === 'cmp' || mn === 'tst') { need(2); rd = 0; rs1 = reg(ops[0]); src = ops[1]; }
    else if (ops.length === 3) { rd = reg(ops[0]); rs1 = reg(ops[1]); src = ops[2]; }
    else { need(2, 3); rd = reg(ops[0]); rs1 = rd; src = ops[1]; }
    const head = mn === 'cmp' || mn === 'tst' ? `${mn} ${rn(rs1)}` : `${mn} ${rn(rd)}, ${rn(rs1)}`;
    if (isReg(src)) { const rs2 = reg(src); return [{ m: 'alu', op: n, rd, rs1, rs2, word: encR(0x20 + n, rd, rs1, rs2), text: `${head}, ${rn(rs2)}` }]; }
    const v = imm(src, IMM_KIND[n]);
    return [{ m: 'alui', op: n, rd, rs1, imm: v, word: encI(0x30 + n, rd, rs1, v), text: `${head}, ${v}` }];
  }
  throw new AsmError('asm.unknown_mnemonic', { name: mn }, st.col, mn.length);
}

// ---------------------------------------------------------------- machine

function freshState(budget) {
  const regs = new Uint32Array(10);
  regs[9] = 0xf000;
  return {
    regs, flags: 0, pc: 0x100, steps: 0, cycles: 0, status: STATUS.PAUSED, fault: null,
    mem: new Uint8Array(65536), shadow: new Uint8Array(4096), blocks: [], rng: 0x9e3779b9,
    outLen: 0, inIntPos: 0, inTextPos: 0, lineCounts: new Map(), budget,
  };
}

function cloneState(s) {
  return {
    ...s,
    regs: s.regs.slice(),
    mem: s.mem.slice(),
    shadow: s.shadow.slice(),
    blocks: s.blocks.map((b) => ({ ...b })),
    lineCounts: new Map(s.lineCounts),
    fault: s.fault ? { ...s.fault, args: { ...s.fault.args } } : null,
  };
}

function copyInto(target, src) {
  // Restore into the existing buffers so views keep stable references.
  target.mem.set(src.mem);
  target.shadow.set(src.shadow);
  target.regs.set(src.regs);
  for (const k of ['flags', 'pc', 'steps', 'cycles', 'status', 'outLen', 'inIntPos', 'inTextPos', 'rng', 'budget']) target[k] = src[k];
  target.fault = src.fault ? { ...src.fault, args: { ...src.fault.args } } : null;
  target.blocks = src.blocks.map((b) => ({ ...b }));
  target.lineCounts = new Map(src.lineCounts);
}

const s32 = (v) => v | 0;
const FAULT_CODE = ['illegal_instruction', 'zero_instruction', 'exec_outside_code', 'bad_jump_target', 'bad_return_address', 'null_access', 'out_of_range', 'guard_page', 'write_to_code', 'misaligned', 'stack_overflow', 'stack_underflow', 'stack_misaligned', 'div_zero', 'input_end', 'heap_unallocated', 'heap_overflow', 'use_after_free', 'invalid_free', 'double_free', 'unknown_syscall', 'string_too_long', 'output_limit'];

class Fault extends Error {
  constructor(key, args = {}) { super(key); this.key = key; this.args = args; }
}

class Machine {
  constructor(program, cfg) {
    this.program = program;
    this.cfg = cfg; // {ints, text, interactive, heapCheck, budget, screen}
    this.outKinds = [];
    this.outVals = [];
    this.s = freshState(cfg.budget);
    for (const [a, b] of program.mem) this.s.mem[a] = b;
    if (cfg.screen) this.s.mem.set(cfg.screen, 0xf000);
  }

  check(addr, size, write) {
    const s = this.s;
    if (addr > 0xffff) throw new Fault('fault.out_of_range', { addr, access: write ? 'w' : 'r' });
    if (size === 4 && addr % 4) throw new Fault('fault.misaligned', { addr, size });
    if (addr < 0x100) throw new Fault('fault.null_access', { addr, access: write ? 'w' : 'r' });
    if (addr < 0x4000 && write) throw new Fault('fault.write_to_code', { addr });
    if (addr >= 0xc000 && addr < 0xd000) throw new Fault('fault.guard_page', { addr, access: write ? 'w' : 'r' });
    if (addr >= 0x8000 && addr < 0xc000 && this.cfg.heapCheck !== 'off') {
      for (let a = addr; a < addr + size; a += 4) {
        const st = s.shadow[(a - 0x8000) >> 2];
        if (st === SHADOW_FREE) throw new Fault('fault.heap_unallocated', { addr: a });
        if (st === SHADOW_FREED) throw new Fault('fault.use_after_free', { addr: a });
        if (st === SHADOW_RED) throw new Fault('fault.heap_overflow', { addr: a });
      }
    }
  }

  load(addr, size) {
    this.check(addr, size, false);
    const m = this.s.mem;
    return size === 4 ? readWord(m, addr) : m[addr];
  }

  store(addr, size, value) {
    this.check(addr, size, true);
    const m = this.s.mem;
    for (let k = 0; k < size; k++) m[addr + k] = (value >>> (8 * k)) & 0xff;
  }

  pushWord(v, s) {
    const sp = (s.regs[9] - 4) >>> 0;
    if (sp % 4) throw new Fault('fault.stack_misaligned', { sp });
    if (sp < 0xd000) throw new Fault('fault.stack_overflow', { sp });
    if (sp >= 0xf000) throw new Fault('fault.stack_underflow', { sp });
    s.regs[9] = sp;
    const m = s.mem;
    for (let k = 0; k < 4; k++) m[sp + k] = (v >>> (8 * k)) & 0xff;
  }

  popWord(s) {
    const sp = s.regs[9];
    if (sp % 4) throw new Fault('fault.stack_misaligned', { sp });
    if (sp >= 0xf000) throw new Fault('fault.stack_underflow', { sp });
    if (sp < 0xd000) throw new Fault('fault.stack_overflow', { sp });
    const v = readWord(s.mem, sp);
    s.regs[9] = sp + 4;
    return v;
  }

  checkTarget(target, key) {
    if (target % 4 || target < 0x100 || target >= this.program.textEnd || !this.program.instrs.has(target)) throw new Fault(key, { target });
  }

  /** Execute one instruction. Returns nothing; updates status on halt/fault/wait. */
  stepOne() {
    const s = this.s;
    const ins = this.program.instrs.get(s.pc);
    if (!ins) {
      const key = s.pc >= 0x100 && s.pc < 0x4000 ? 'fault.zero_instruction' : 'fault.exec_outside_code';
      this.fail(new Fault(key, {}));
      return;
    }
    // Work on a scratch copy of registers and flags for precise faults.
    const regs = s.regs.slice();
    let flags = s.flags;
    let pc = s.pc + 4;
    let cycles = 1;
    const snap = { sp: s.regs[9], outLen: s.outLen, inIntPos: s.inIntPos, inTextPos: s.inTextPos };
    const setZN = (r) => { flags &= ~(FZ | FN); if ((r >>> 0) === 0) flags |= FZ; if (r & 0x80000000) flags |= FN; };
    const setCV = (c, v) => { flags &= ~(FC | FV); if (c) flags |= FC; if (v) flags |= FV; };
    const aluOp = (op, a, b) => {
      a >>>= 0; b >>>= 0;
      let r = 0;
      switch (ALU[op]) {
        case 'add': r = (a + b) >>> 0; setCV(a + b > 0xffffffff, ((~(a ^ b)) & (a ^ r)) >>> 31); break;
        case 'sub': case 'cmp': r = (a - b) >>> 0; setCV(a < b, ((a ^ b) & (a ^ r)) >>> 31); break;
        case 'mul': {
          r = Math.imul(a, b) >>> 0;
          const pu = BigInt(a) * BigInt(b);
          const ps = BigInt(s32(a)) * BigInt(s32(b));
          setCV(pu > 0xffffffffn, ps > 2147483647n || ps < -2147483648n);
          cycles = 3;
          break;
        }
        case 'div': case 'rem': {
          if (b === 0) throw new Fault('fault.div_zero');
          const sa = s32(a), sb = s32(b);
          if (sa === -2147483648 && sb === -1) { r = ALU[op] === 'div' ? 0x80000000 : 0; setCV(false, true); }
          else { r = (ALU[op] === 'div' ? Math.trunc(sa / sb) : sa % sb) >>> 0; setCV(false, false); }
          cycles = 8;
          break;
        }
        case 'divu': case 'remu':
          if (b === 0) throw new Fault('fault.div_zero');
          r = ALU[op] === 'divu' ? Math.floor(a / b) >>> 0 : (a % b) >>> 0; setCV(false, false); cycles = 8; break;
        case 'and': case 'tst': r = (a & b) >>> 0; setCV(false, false); break;
        case 'or': r = (a | b) >>> 0; setCV(false, false); break;
        case 'xor': r = (a ^ b) >>> 0; setCV(false, false); break;
        case 'shl': { const n = b & 31; r = (a << n) >>> 0; setCV(n ? (a >>> (32 - n)) & 1 : 0, false); break; }
        case 'shr': { const n = b & 31; r = a >>> n; setCV(n ? (a >>> (n - 1)) & 1 : 0, false); break; }
        case 'sar': { const n = b & 31; r = (s32(a) >> n) >>> 0; setCV(n ? (a >>> (n - 1)) & 1 : 0, false); break; }
        default: break;
      }
      setZN(r);
      return r;
    };
    const ea = (ins2) => (((ins2.base === 15 ? 0 : regs[ins2.base]) + ins2.off) >>> 0);
    const condTrue = (c) => {
      const z = flags & FZ, n = (flags & FN) !== 0, cf = flags & FC, v = (flags & FV) !== 0;
      return [true, z, !z, n !== v, n === v, z || n !== v, !z && n === v, cf, !cf, cf || z, !cf && !z, n, !n, v, !v][c];
    };
    // Memory writes are deferred until no fault can happen any more.
    const writes = [];
    const deferStore = (addr, size, value) => { this.check(addr, size, true); writes.push([addr, size, value]); };
    let output = null;
    let heapAction = null;
    try {
      switch (ins.m) {
        case 'halt': s.status = STATUS.HALTED; pc = s.pc; break;
        case 'nop': break;
        case 'movi': regs[ins.rd] = ins.imm >>> 0; break;
        case 'mov': regs[ins.rd] = regs[ins.rs]; break;
        case 'movhi': regs[ins.rd] = ((ins.imm << 16) | (regs[ins.rd] & 0xffff)) >>> 0; break;
        case 'lea': { const a = ea(ins); if (a > 0xffff) throw new Fault('fault.out_of_range', { addr: a, access: 'r' }); regs[ins.rd] = a; break; }
        case 'alu': { const r = aluOp(ins.op, regs[ins.rs1], regs[ins.rs2]); if (ALU[ins.op] !== 'cmp' && ALU[ins.op] !== 'tst') regs[ins.rd] = r; break; }
        case 'alui': { const r = aluOp(ins.op, regs[ins.rs1], ins.imm); if (ALU[ins.op] !== 'cmp' && ALU[ins.op] !== 'tst') regs[ins.rd] = r; break; }
        case 'neg': regs[ins.rd] = aluOp(1, 0, regs[ins.rs]); break;
        case 'not': { const r = (~regs[ins.rs]) >>> 0; setZN(r); setCV(false, false); regs[ins.rd] = r; break; }
        case 'ld': regs[ins.rd] = this.load(ea(ins), 4); cycles = 2; break;
        case 'ldb': regs[ins.rd] = this.load(ea(ins), 1); cycles = 2; break;
        case 'ldbs': { const b = this.load(ea(ins), 1); regs[ins.rd] = (b & 0x80 ? b | 0xffffff00 : b) >>> 0; cycles = 2; break; }
        case 'st': deferStore(ea(ins), 4, regs[ins.rd]); cycles = 2; break;
        case 'stb': deferStore(ea(ins), 1, regs[ins.rd] & 0xff); cycles = 2; break;
        case 'push': {
          const sp = (regs[9] - 4) >>> 0;
          if (sp % 4) throw new Fault('fault.stack_misaligned', { sp });
          if (sp < 0xd000) throw new Fault('fault.stack_overflow', { sp });
          writes.push([sp, 4, regs[ins.rd]]);
          regs[9] = sp; cycles = 2; break;
        }
        case 'pop': {
          const sp = regs[9];
          if (sp % 4) throw new Fault('fault.stack_misaligned', { sp });
          if (sp >= 0xf000) throw new Fault('fault.stack_underflow', { sp });
          const v = readWord(s.mem, sp);
          regs[9] = sp + 4; regs[ins.rd] = v; cycles = 2; break;
        }
        case 'j': if (condTrue(ins.cond)) { this.checkTarget(ins.target, 'fault.bad_jump_target'); pc = ins.target; } break;
        case 'jmpr': this.checkTarget(regs[ins.rd], 'fault.bad_jump_target'); pc = regs[ins.rd]; break;
        case 'call': case 'callr': {
          const target = ins.m === 'call' ? ins.target : regs[ins.rd];
          this.checkTarget(target, 'fault.bad_jump_target');
          const sp = (regs[9] - 4) >>> 0;
          if (sp < 0xd000) throw new Fault('fault.stack_overflow', { sp });
          writes.push([sp, 4, s.pc + 4]);
          regs[9] = sp; pc = target; cycles = 2; break;
        }
        case 'ret': {
          const sp = regs[9];
          if (sp >= 0xf000) throw new Fault('fault.stack_underflow', { sp });
          if (sp % 4) throw new Fault('fault.stack_misaligned', { sp });
          const target = readWord(s.mem, sp);
          this.checkTarget(target, 'fault.bad_return_address');
          regs[9] = sp + 4; pc = target; cycles = 2; break;
        }
        case 'sys': {
          cycles = 4;
          switch (ins.n) {
            case 1:
              if (s.inIntPos < this.cfg.ints.length) { regs[0] = this.cfg.ints[s.inIntPos] >>> 0; snap.inIntPos = s.inIntPos + 1; }
              else if (this.cfg.interactive && !this.cfg.closed) { s.status = STATUS.WAITING_INPUT; return; }
              else throw new Fault('fault.input_end');
              break;
            case 3:
              if (s.inTextPos < this.cfg.text.length) { regs[0] = this.cfg.text[s.inTextPos]; snap.inTextPos = s.inTextPos + 1; }
              else if (this.cfg.interactive && !this.cfg.closed) { s.status = STATUS.WAITING_INPUT; return; }
              else regs[0] = 0xffffffff;
              break;
            case 2: output = [[0, s32(regs[0])]]; break;
            case 4: output = [[1, regs[0] & 0xff]]; break;
            case 5: {
              const bytes = [];
              let a = regs[0];
              for (;;) {
                if (bytes.length >= 4096) throw new Fault('fault.string_too_long', { addr: regs[0] });
                const b = this.load(a, 1);
                if (b === 0) break;
                bytes.push([1, b]);
                a++;
              }
              output = bytes;
              break;
            }
            case 6: heapAction = ['alloc', regs[0]]; break;
            case 7: heapAction = ['free', regs[0]]; break;
            case 8: { let x = s.rng; x ^= x << 13; x >>>= 0; x ^= x >>> 17; x ^= x << 5; x >>>= 0; snap.rng = x; regs[0] = x; break; }
            default: throw new Fault('fault.unknown_syscall', { n: ins.n });
          }
          break;
        }
        default: throw new Fault('fault.illegal_instruction', { word: ins.word });
      }
      if (output && s.outLen + output.length > 8192) throw new Fault('fault.output_limit');
      if (heapAction) {
        const r = this.heapOp(heapAction[0], heapAction[1] >>> 0, ins.line);
        if (heapAction[0] === 'alloc') regs[0] = r;
      }
    } catch (e) {
      if (e instanceof Fault) { this.fail(e); return; }
      throw e;
    }
    // Commit.
    for (const [addr, size, value] of writes) for (let k = 0; k < size; k++) s.mem[addr + k] = (value >>> (8 * k)) & 0xff;
    if (output) {
      for (const [k, v] of output) { this.outKinds[s.outLen] = k; this.outVals[s.outLen] = v; s.outLen++; }
      this.outKinds.length = s.outLen; this.outVals.length = s.outLen;
    }
    s.regs.set(regs);
    s.flags = flags;
    if ('inIntPos' in snap) s.inIntPos = snap.inIntPos;
    if ('inTextPos' in snap) s.inTextPos = snap.inTextPos;
    if (snap.rng !== undefined) s.rng = snap.rng;
    s.pc = pc;
    s.steps++;
    s.cycles += cycles;
    if (ins.file === 0) s.lineCounts.set(ins.line, (s.lineCounts.get(ins.line) ?? 0) + 1);
    if (s.status === STATUS.PAUSED && s.cycles >= s.budget) s.status = STATUS.BUDGET_EXCEEDED;
  }

  heapOp(kind, value, line) {
    const s = this.s;
    if (kind === 'alloc') {
      if (value === 0 || value > 0x4000) return 0;
      const size = (value + 7) & ~7;
      // First fit over the heap; blocks sorted by address. Each block owns size + 8 red zone.
      const used = s.blocks.filter((b) => b.live).sort((a, b) => a.addr - b.addr);
      let addr = 0x8000;
      for (const b of used) { if (addr + size + 8 <= b.addr) break; addr = Math.max(addr, b.addr + b.size + 8); }
      if (addr + size + 8 > 0xc000) return 0;
      s.blocks = s.blocks.filter((b) => b.live || b.addr + b.size + 8 <= addr || b.addr >= addr + size + 8);
      s.blocks.push({ addr, size, live: true, line });
      for (let a = addr; a < addr + size; a++) s.mem[a] = 0xcd;
      for (let g = addr; g < addr + size; g += 4) s.shadow[(g - 0x8000) >> 2] = SHADOW_LIVE;
      for (let g = addr + size; g < addr + size + 8; g += 4) s.shadow[(g - 0x8000) >> 2] = SHADOW_RED;
      return addr;
    }
    if (value === 0) return 0;
    const block = s.blocks.find((b) => b.addr === value);
    if (!block) throw new Fault('fault.invalid_free', { addr: value });
    if (!block.live) throw new Fault('fault.double_free', { addr: value });
    block.live = false;
    for (let a = block.addr; a < block.addr + block.size; a++) s.mem[a] = 0xdd;
    for (let g = block.addr; g < block.addr + block.size; g += 4) s.shadow[(g - 0x8000) >> 2] = SHADOW_FREED;
    return 0;
  }

  fail(e) {
    this.s.status = STATUS.FAULT;
    this.s.fault = { key: e.key, pc: this.s.pc, step: this.s.steps, args: e.args };
  }
}

// ---------------------------------------------------------------- session

const SNAPSHOT_LIMIT = 64;

export class Session {
  constructor(level) {
    this.level = level; // mock level object or null (sandbox)
    this.asm = null;
    this.machine = null;
    this.breakpoints = new Set();
    this.testIndex = -1;
    this.edits = [];
    this.snapshots = [];
    this.k = 1024;
    this.maxStep = 0;
    this.inputInts = [];
    this.inputText = [];
    this.closed = false;
    this.memView = new Uint8Array(65536);
    this.shadowView = new Uint8Array(4096);
  }

  assemble(source) {
    const lib = this.level?.library ?? '';
    const res = assemble(source, lib, { allow: this.level?.allow, forbid: this.level?.forbid });
    this.asm = res.ok ? res.program : null;
    this.size = res.size;
    this.machine = null;
    if (this.asm) this.reset(this.testIndex);
    return { ok: res.ok, size: res.size, diagnostics: res.diagnostics };
  }

  config(test) {
    const lv = this.level;
    if (test < 0 || !lv || !lv.tests[test]) {
      return { ints: this.inputInts, text: this.inputText, interactive: true, closed: this.closed, heapCheck: lv?.heapCheck ?? 'strict', budget: lv?.budget ?? 10000000 };
    }
    const t = lv.tests[test];
    return {
      ints: t.in ?? [],
      text: [...new TextEncoder().encode(t.in_text ?? '')],
      interactive: false,
      heapCheck: lv.heapCheck ?? 'strict',
      budget: t.budget ?? lv.budget ?? 200000,
      screen: t.screen_init ? imageBytes(lv.images[t.screen_init]) : null,
      call: t.call ?? null,
      regs: t.regs ?? null,
      seed: t.seed ?? lv.seed ?? 1,
    };
  }

  build(test) {
    const cfg = this.config(test);
    const m = new Machine(this.asm, cfg);
    if (cfg.call) {
      const sym = this.asm.symbols.find((x) => x.name === cfg.call);
      m.s.pc = sym ? sym.value : 0x100;
      if (cfg.regs) for (const [r, v] of Object.entries(cfg.regs)) m.s.regs[REGS[r]] = v >>> 0;
      let x = (cfg.seed >>> 0) || 0x9e3779b9;
      const rnd = () => { x ^= x << 13; x >>>= 0; x ^= x >>> 17; x ^= x << 5; x >>>= 0; return x; };
      for (let r = 4; r <= 8; r++) m.s.regs[r] = r === 8 ? (rnd() | 1) >>> 0 : rnd();
      m.abiBefore = [...m.s.regs.slice(4, 10)];
      m.pushWord(this.asm.trap, m.s);
      m.trapCall = true;
    }
    return m;
  }

  reset(test = -1) {
    if (!this.asm) return STATUS.NOT_ASSEMBLED;
    this.testIndex = test;
    this.machine = this.build(test);
    this.edits = [];
    this.snapshots = [cloneState(this.machine.s)];
    this.k = 1024;
    this.maxStep = 0;
    this.sync();
    return this.machine.s.status;
  }

  sync() {
    this.memView.set(this.machine.s.mem);
    this.shadowView.set(this.machine.s.shadow);
  }

  canRun() {
    return this.machine && (this.machine.s.status === STATUS.PAUSED || this.machine.s.status === STATUS.BREAKPOINT);
  }

  /** One instruction on the current timeline, with edits, trap detection and snapshots. */
  tick() {
    const m = this.machine;
    this.applyEdits(m.s.steps);
    m.stepOne();
    if (m.trapCall && m.s.pc === this.asm.trap && m.s.status === STATUS.PAUSED) m.s.status = STATUS.HALTED;
    if (m.s.steps % this.k === 0 && m.s.status === STATUS.PAUSED) this.snapshot();
  }

  advance(count, useBreakpoints, maxCycles = Infinity) {
    const m = this.machine;
    const startCycles = m.s.cycles;
    let first = true;
    for (let i = 0; i < count; i++) {
      if (m.s.status === STATUS.BREAKPOINT) m.s.status = STATUS.PAUSED;
      if (m.s.status !== STATUS.PAUSED) break;
      if (useBreakpoints && !first && this.breakpoints.has(m.s.pc)) { m.s.status = STATUS.BREAKPOINT; break; }
      first = false;
      this.tick();
      if (m.s.cycles - startCycles >= maxCycles) break;
    }
    if (m.s.steps > this.maxStep) this.maxStep = m.s.steps;
  }

  snapshot() {
    const s = this.machine.s;
    if (this.snapshots.some((x) => x.steps === s.steps)) return;
    this.snapshots.push(cloneState(s));
    if (this.snapshots.length > SNAPSHOT_LIMIT) {
      this.snapshots = this.snapshots.filter((_, i) => i % 2 === 0);
      this.k *= 2;
    }
  }

  applyEdits(step) {
    for (const e of this.edits) if (e.step === step) {
      if (e.reg === 10) this.machine.s.flags = e.value & 15; else this.machine.s.regs[e.reg] = e.value >>> 0;
    }
  }

  step(n = 1) {
    if (!this.machine) return STATUS.NOT_ASSEMBLED;
    if (this.canRun()) this.advance(n, false);
    this.sync();
    return this.machine.s.status;
  }

  run(maxCycles = 50000) {
    if (!this.machine) return STATUS.NOT_ASSEMBLED;
    if (this.canRun()) this.advance(Infinity, true, maxCycles);
    this.sync();
    return this.machine.s.status;
  }

  goto(target) {
    if (!this.machine) return STATUS.NOT_ASSEMBLED;
    const m = this.machine;
    const t = Math.max(0, Math.min(Math.floor(target), this.maxStep));
    if (t < m.s.steps) {
      let snap = this.snapshots[0];
      for (const x of this.snapshots) if (x.steps <= t && x.steps >= snap.steps) snap = x;
      copyInto(m.s, snap);
      // The output tape only grows on this timeline, so cutting it is enough.
      m.outKinds.length = snap.outLen;
      m.outVals.length = snap.outLen;
    }
    if (m.s.status === STATUS.BREAKPOINT) m.s.status = STATUS.PAUSED;
    while (m.s.steps < t && m.s.status === STATUS.PAUSED) this.tick();
    this.applyEdits(m.s.steps);
    this.sync();
    return m.s.status;
  }

  back(n = 1) {
    if (!this.machine) return STATUS.NOT_ASSEMBLED;
    return this.goto(Math.max(0, this.machine.s.steps - n));
  }

  /** kind 0 = register index, 1 = memory word address, 2 = memory byte address. */
  backToChange(kind, target) {
    if (!this.machine) return STATUS.NOT_ASSEMBLED;
    const now = this.machine.s.steps;
    const probe = this.build(this.testIndex);
    const read = (st) => (kind === 0 ? (target === 10 ? st.flags : st.regs[target]) : kind === 1 ? readWord(st.mem, target) : st.mem[target]);
    let last = -1;
    const saved = this.machine;
    this.machine = probe;
    while (probe.s.steps < now && probe.s.status === STATUS.PAUSED) {
      this.applyEdits(probe.s.steps);
      const before = read(probe.s);
      const step = probe.s.steps;
      probe.stepOne();
      if (read(probe.s) !== before) last = step;
    }
    this.machine = saved;
    return this.goto(last < 0 ? 0 : last);
  }

  setReg(reg, value) {
    if (!this.machine) return STATUS.NOT_ASSEMBLED;
    const s = this.machine.s;
    if (!(s.status === STATUS.PAUSED || s.status === STATUS.BREAKPOINT)) return STATUS.ERROR;
    if (!(reg >= 0 && reg <= 10)) return STATUS.ERROR;
    this.edits = this.edits.filter((e) => e.step < s.steps || (e.step === s.steps && e.reg !== reg));
    this.snapshots = this.snapshots.filter((x) => x.steps <= s.steps);
    this.edits.push({ step: s.steps, reg, value: value >>> 0 });
    this.applyEdits(s.steps);
    this.maxStep = s.steps;
    return s.status;
  }

  setBreakpoint(addr, on) {
    if (on) this.breakpoints.add(addr); else this.breakpoints.delete(addr);
    return on ? 1 : 0;
  }

  clearBreakpoints() { this.breakpoints.clear(); }

  pushInt(v) {
    this.inputInts.push(v | 0);
    if (this.machine?.s.status === STATUS.WAITING_INPUT) this.machine.s.status = STATUS.PAUSED;
  }

  pushText(str) {
    this.inputText.push(...new TextEncoder().encode(str));
    if (this.machine?.s.status === STATUS.WAITING_INPUT) this.machine.s.status = STATUS.PAUSED;
  }

  closeInput() {
    this.closed = true;
    if (this.machine) this.machine.cfg.closed = true;
    if (this.machine?.s.status === STATUS.WAITING_INPUT) this.machine.s.status = STATUS.PAUSED;
  }

  state() {
    if (!this.machine) {
      return { status: STATUS.NOT_ASSEMBLED, pc: 0x100, regs: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0xf000], flags: 0, steps: 0, cycles: 0, faultCode: 0, faultAddr: 0, outLen: 0, inIntPos: 0, inIntLen: 0, inTextPos: 0, inTextLen: 0, testIndex: this.testIndex, budget: 0 };
    }
    const s = this.machine.s;
    const cfg = this.machine.cfg;
    return {
      status: s.status, pc: s.pc, regs: [...s.regs], flags: s.flags, steps: s.steps, cycles: s.cycles,
      faultCode: s.fault ? FAULT_CODE.indexOf(s.fault.key.slice(6)) + 1 : 0,
      faultAddr: s.fault?.args.addr ?? 0,
      outLen: s.outLen, inIntPos: s.inIntPos, inIntLen: cfg.ints.length, inTextPos: s.inTextPos, inTextLen: cfg.text.length,
      testIndex: this.testIndex, budget: s.budget,
    };
  }

  /** Furthest step on the current timeline (for the time slider). */
  horizon() { return this.maxStep; }
  memory() { return this.memView; }
  shadow() { return this.shadowView; }
  output() {
    if (!this.machine) return [];
    const m = this.machine;
    return Array.from({ length: m.s.outLen }, (_, i) => ({ kind: m.outKinds[i], value: m.outVals[i] }));
  }
  fault() { return this.machine?.s.fault ?? null; }
  symbols() { return this.asm?.symbols ?? []; }
  lineMap() { return this.asm?.lineMap ?? []; }
  lineCounts() { return this.machine ? [...this.machine.s.lineCounts.entries()].sort((a, b) => a[0] - b[0]) : []; }

  frames() {
    if (!this.machine) return [];
    const s = this.machine.s;
    const out = [];
    let fp = s.regs[8];
    let last = 0;
    const funcFor = (addr) => {
      let best = null;
      for (const sym of this.asm.symbols) if (sym.kind === 'code' && !sym.name.includes('.') && !sym.name.startsWith('__') && sym.value <= addr && (!best || sym.value > best.value)) best = sym;
      return best?.name ?? null;
    };
    let pcFunc = funcFor(s.pc);
    let lowest = s.regs[9];
    while (fp !== 0 && fp >= 0xd000 && fp < 0xf000 && fp > last && out.length < 64) {
      const ret = readWord(s.mem, fp + 4);
      out.push({ fp, ret, func: pcFunc, size: fp - lowest });
      pcFunc = funcFor(ret - 4);
      lowest = fp + 8;
      last = fp;
      fp = readWord(s.mem, fp);
    }
    return out;
  }

  heap() { return this.machine ? this.machine.s.blocks.map((b) => ({ addr: b.addr, size: b.size, live: b.live, line: b.line })) : []; }

  disasm(addr, count) {
    const out = [];
    if (!this.asm) return out;
    const labels = new Map(this.asm.symbols.filter((x) => x.kind === 'code').map((x) => [x.value, x.name]));
    for (let a = addr, i = 0; i < count; a += 4, i++) {
      const ins = this.asm.instrs.get(a);
      out.push({ addr: a, word: ins ? ins.word : readWord(this.memView, a), text: ins ? ins.text : '.word', label: labels.get(a) ?? null });
    }
    return out;
  }

  evaluate(m, index) {
    const lv = this.level;
    const t = lv.tests[index];
    const s = m.s;
    const result = { index, name: t.name, hidden: Boolean(t.hidden), passed: false, status: s.status, cycles: s.cycles, steps: s.steps, fault: s.fault, mismatch: null };
    const fail = (key, args) => { result.mismatch = { key, args }; return result; };
    if (s.status === STATUS.FAULT) return result;
    if (s.status !== STATUS.HALTED) return fail('check.not_halted', { status: s.status });
    if (m.trapCall) {
      const names = ['r4', 'r5', 'r6', 'r7', 'fp', 'sp'];
      for (let i = 0; i < 6; i++) if ((s.regs[4 + i] >>> 0) !== (m.abiBefore[i] >>> 0)) return fail('check.abi_clobbered', { reg: names[i], before: m.abiBefore[i], after: s.regs[4 + i] });
    }
    if (t.out) {
      const outs = [];
      for (let i = 0; i < s.outLen; i++) if (m.outKinds[i] === 0) outs.push(m.outVals[i]);
      for (let i = 0; i < Math.max(outs.length, t.out.length); i++) {
        if (i >= outs.length) return fail('check.out_missing', { index: i, expected: t.out[i] });
        if (i >= t.out.length) return fail('check.out_extra', { index: i, actual: outs[i] });
        if (outs[i] !== t.out[i]) return fail('check.out_mismatch', { index: i, expected: t.out[i], actual: outs[i] });
      }
    }
    if (t.expect_reg) for (const [r, v] of Object.entries(t.expect_reg)) {
      if ((s.regs[REGS[r]] | 0) !== (v | 0)) return fail('check.reg_mismatch', { reg: r, expected: v, actual: s.regs[REGS[r]] | 0 });
    }
    if (t.expect_screen) {
      const want = this.referenceScreen(index);
      let count = 0, first = null;
      for (let i = 0; i < 4096; i++) if ((s.mem[0xf000 + i] & 15) !== (want[i] & 15)) { count++; if (!first) first = { x: i % 64, y: i >> 6, expected: want[i] & 15, actual: s.mem[0xf000 + i] & 15 }; }
      if (count) return fail('check.screen_mismatch', { ...first, count });
    }
    if (t.expect_heap_empty) {
      const live = s.blocks.filter((b) => b.live);
      if (live.length) return fail('check.heap_not_empty', { count: live.length, alloc_line: live[0].line });
    }
    result.passed = true;
    return result;
  }

  check() {
    if (!this.machine || !this.level || this.testIndex < 0) return null;
    return this.evaluate(this.machine, this.testIndex);
  }

  runTests(withReference = true) {
    if (!this.asm || !this.level) return { passed: false, tests: [], metrics: null, reference: null };
    const tests = this.level.tests.map((t, i) => {
      const m = this.build(i);
      const saved = this.machine;
      this.machine = m;
      while (m.s.status === STATUS.PAUSED) {
        m.stepOne();
        if (m.trapCall && m.s.pc === this.asm.trap && m.s.status === STATUS.PAUSED) m.s.status = STATUS.HALTED;
      }
      this.machine = saved;
      const r = this.evaluate(m, i);
      if (r.hidden && !r.passed) r.input = { in: t.in ?? [], out: t.out ?? null };
      return r;
    });
    const cyc = tests.map((t) => t.cycles);
    const steps = tests.map((t) => t.steps);
    return {
      passed: tests.every((t) => t.passed),
      tests,
      metrics: { cycles_avg: Math.ceil(cyc.reduce((a, b) => a + b, 0) / tests.length), cycles_max: Math.max(...cyc), steps_avg: Math.ceil(steps.reduce((a, b) => a + b, 0) / tests.length), size: this.size },
      reference: withReference && this.level.ref ? referenceMetrics(this.level) : null,
    };
  }

  /** Soll image for a screen test: the reference solution's screen after the same test. */
  referenceScreen(index) {
    const lv = this.level;
    if (!lv || index < 0 || !lv.tests[index]?.expect_screen) return null;
    lv.refScreens ??= new Map();
    if (lv.refScreens.has(index)) return lv.refScreens.get(index);
    const t = lv.tests[index];
    let img;
    if (t.expect_screen !== 'reference') img = imageBytes(lv.images[t.expect_screen]);
    else {
      const ref = new Session(lv);
      ref.assemble(lv.ref);
      ref.reset(index);
      ref.machine.s.budget = 1e9;
      ref.advance(Infinity, false);
      img = ref.machine.s.mem.slice(0xf000, 0x10000);
    }
    lv.refScreens.set(index, img);
    return img;
  }

  digest() {
    if (!this.machine) return '0000000000000000';
    let h = 0xcbf29ce484222325n;
    const feed = (b) => { h ^= BigInt(b & 0xff); h = (h * 0x100000001b3n) & 0xffffffffffffffffn; };
    const s = this.machine.s;
    for (const r of s.regs) for (let k = 0; k < 4; k++) feed(r >>> (8 * k));
    feed(s.flags); for (let k = 0; k < 4; k++) feed(s.pc >>> (8 * k));
    for (let i = 0; i < s.mem.length; i += 61) feed(s.mem[i]);
    return h.toString(16).padStart(16, '0');
  }

  free() { this.machine = null; this.asm = null; }
}

function imageBytes(rows) {
  const out = new Uint8Array(4096);
  (rows ?? []).forEach((row, y) => {
    for (let x = 0; x < Math.min(64, row.length); x++) {
      const c = row[x];
      out[y * 64 + x] = c === '.' ? 0 : Number.parseInt(c, 16) || 0;
    }
  });
  return out;
}

function referenceMetrics(lv) {
  if (lv.refMetrics) return lv.refMetrics;
  const s = new Session(lv);
  s.assemble(lv.ref);
  const r = s.runTests(false);
  lv.refMetrics = r.metrics ? { cycles: r.metrics.cycles_avg, size: r.metrics.size } : null;
  return lv.refMetrics;
}

// ---------------------------------------------------------------- vm facade

class MockLevelHandle {
  constructor(level) { this.level = level; }
  info() {
    const lv = this.level;
    return {
      id: lv.id, chapter: lv.chapter, order: lv.order, kind: lv.kind, tags: lv.tags, mobile: lv.mobile,
      title: lv.title, card: lv.card, budget: lv.budget ?? 200000,
      tests: lv.tests.filter((t) => !t.hidden).map((t) => t.name),
      testsAll: lv.tests.map((t) => ({ name: t.name, hidden: Boolean(t.hidden), in: t.hidden ? null : t.in ?? null, in_text: t.hidden ? null : t.in_text ?? null, out: t.hidden ? null : t.out ?? null, screen: Boolean(t.expect_screen), call: t.call ?? null, regs: t.hidden ? null : t.regs ?? null, expect_reg: t.hidden ? null : t.expect_reg ?? null })),
      allow: lv.allow ?? null, forbid: lv.forbid ?? null,
      ref_cycles: lv.ref ? referenceMetrics(lv)?.cycles ?? null : null,
      ref_size: lv.ref ? referenceMetrics(lv)?.size ?? null : null,
      full: Boolean(lv.text),
    };
  }
  section(name) {
    const lv = this.level;
    const [kind, lang] = name.split('.');
    if (kind === 'start') return lv.start ?? null;
    const value = lv[kind];
    if (!value) return null;
    if (kind === 'hint') return value[lang]?.join('\n---\n') ?? null;
    return value[lang] ?? null;
  }
  free() {}
}

class MockVm {
  constructor() { this.mock = true; this.abiVersion = 1; }
  segments() { return SEGMENTS; }
  levelIndex() {
    return {
      chapters: MOCK_CHAPTERS,
      levels: MOCK_LEVELS.map((l) => ({ id: l.id, chapter: l.chapter, order: l.order, kind: l.kind, tags: l.tags, mobile: l.mobile, title: l.title, card: l.card })),
    };
  }
  loadLevel(id) {
    const lv = MOCK_LEVELS.find((l) => l.id === id);
    return lv ? new MockLevelHandle(lv) : null;
  }
  newSession(handle) { return new Session(handle ? handle.level : null); }
}

export async function loadVm() {
  return new MockVm();
}
