// The only JavaScript module that touches WebAssembly (docs/ABI.md, "web/js/vm.js contract").
//
//   const vm = await loadVm(urlOrBytes);     // URL/string (fetch) or bytes (ArrayBuffer, view, Module)
//   vm.segments()                            // [{name, start, end, r, w, x}]
//   const lv = vm.loadLevel(text);           // Level | null (diagnostics in vm.levelDiagnostics)
//   lv.info(), lv.section(name), lv.image(name) → Uint8Array(4096) | null, lv.free()
//   const s = vm.newSession(lv | null);      // null = sandbox
//   s.assemble(src) → {ok, size, diagnostics:[{key,file,line,col,len,args}]}
//   s.reset(test = -1) s.step(n = 1) s.run(maxCycles = 50000) s.back(n = 1) s.goto(step)
//   s.backToChange(kind, target) s.setReg(reg, value)   (reg 10 = flag word, 11 = pc)
//   s.setBreakpoint(addr, on) → 1 | 0   s.clearBreakpoints()
//   s.pushInt(v) s.pushText(str) s.closeInput() → status
//   s.state() → {status, pc, regs[10], flags, steps, cycles, faultCode, faultAddr, outLen,
//                inIntPos, inIntLen, inTextPos, inTextLen, testIndex, budget}
//   s.memory() s.shadow() → live Uint8Array views (re-fetch after every call)
//   s.output() → [{kind, value}]   s.fault() s.symbols() s.lineMap() s.lineCounts() s.frames()
//   s.heap() s.disasm(addr, n) s.check() s.runTests() s.referenceScreen(test) s.digest()
//   s.horizon() s.free()
//
// Status results are unsigned (0xFFFFFFFF = handle or argument error, STATUS.ERROR in machine.js).
// JSON errors from the module ({"error":"abi.…"}) are thrown as VmError with that key. If the
// module traps (a Rust panic), the instance is unusable: vm.js creates a new one, reloads every
// level and session (last assembled source, test, breakpoints; the run itself starts over),
// calls vm.onInternalError(error) and throws VmError('internal').
//
// Pure ES module without DOM access; runs in browsers and under Node.

export const ABI_VERSION = 1;
export const STATUS_ERROR = 0xffffffff;
const NOT_ASSEMBLED = 6;
const VIEW_WORDS = 24; // StateView: 96 bytes
const MEM_BYTES = 65536;
const SHADOW_BYTES = 4096;
const OUTPUT_LIMIT = 8192;
const SCREEN_BYTES = 4096;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** Error from the VM: `key` is an ABI error key (`abi.bad_handle`, …) or `internal`. */
export class VmError extends Error {
  constructor(key, message, options) {
    super(message ?? key, options);
    this.name = 'VmError';
    this.key = key;
  }
}

const isTrap = (err) => err instanceof WebAssembly.RuntimeError;

function isBytes(x) {
  return x instanceof ArrayBuffer || ArrayBuffer.isView(x);
}

async function instantiate(source) {
  if (source instanceof WebAssembly.Module) {
    return { module: source, instance: await WebAssembly.instantiate(source, {}) };
  }
  if (isBytes(source)) return WebAssembly.instantiate(source, {});
  const url = source instanceof URL ? source : new URL(String(source), globalThis.location?.href);
  if (url.protocol === 'file:') {
    // Node: fetch() does not support file: URLs.
    const { readFile } = await import('node:fs/promises');
    return WebAssembly.instantiate(await readFile(url), {});
  }
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${url.pathname}: HTTP ${res.status}`);
  if (typeof WebAssembly.instantiateStreaming === 'function') {
    try {
      return await WebAssembly.instantiateStreaming(res.clone(), {});
    } catch {
      // Wrong MIME type or no streaming support: fall back to the bytes.
    }
  }
  return WebAssembly.instantiate(await res.arrayBuffer(), {});
}

/** Loads the module and returns the VM. Refuses an ABI version other than 1. */
export async function loadVm(urlOrBytes) {
  const { module, instance } = await instantiate(urlOrBytes);
  const version = instance.exports.abi_version?.() >>> 0;
  if (version !== ABI_VERSION) throw new VmError('abi.version', `stapelwerk.wasm has ABI version ${version}, expected ${ABI_VERSION}`);
  return new Vm(module, instance);
}

function decodeImage(rows) {
  const out = new Uint8Array(SCREEN_BYTES);
  rows.split('\n').slice(0, 64).forEach((row, y) => {
    for (let x = 0; x < Math.min(64, row.length); x++) {
      const v = Number.parseInt(row[x], 16);
      out[y * 64 + x] = Number.isNaN(v) ? 0 : v;
    }
  });
  return out;
}

class Vm {
  constructor(module, instance) {
    this.mock = false;
    this.abiVersion = ABI_VERSION;
    this.module = module;
    /** Diagnostics of the last failed loadLevel (`[{key, file, line, col, len, args}]`). */
    this.levelDiagnostics = [];
    /** Called with the original error after a trap was recovered from. */
    this.onInternalError = null;
    this.levels = new Set();
    this.sessions = new Set();
    this.recovering = false;
    this.attach(instance);
  }

  attach(instance) {
    this.instance = instance;
    this.x = instance.exports;
    this.mem = this.x.memory;
    this.buffer = null;
    this.epoch = 0;
    this.sync();
  }

  /** Recreates the byte view when memory.grow replaced the ArrayBuffer. */
  sync() {
    if (this.mem.buffer !== this.buffer) {
      this.buffer = this.mem.buffer;
      this.u8 = new Uint8Array(this.buffer);
      this.epoch++;
    }
  }

  /** Calls an export; returns its raw result. Handles traps (see the module comment). */
  call(name, ...args) {
    let r;
    try {
      r = this.x[name](...args);
    } catch (err) {
      if (!isTrap(err) || this.recovering) throw err;
      this.recover();
      this.onInternalError?.(err);
      throw new VmError('internal', `internal error in ${name}`, { cause: err });
    }
    this.sync();
    return r;
  }

  recover() {
    this.recovering = true;
    try {
      for (let attempt = 0; attempt < 4; attempt++) {
        this.attach(new WebAssembly.Instance(this.module, {}));
        try {
          for (const obj of [...this.levels, ...this.sessions]) {
            try {
              obj.reload();
            } catch (err) {
              if (isTrap(err)) throw err;
            }
          }
          return;
        } catch {
          // The object that trapped again has dropped its state; start over with a new instance.
        }
      }
    } finally {
      this.recovering = false;
    }
  }

  /** Copies `bytes` into a fresh input buffer and calls `fn(ptr, len)`. */
  withBytes(bytes, fn) {
    if (bytes.length === 0) return fn(0, 0);
    const ptr = this.call('buf_alloc', bytes.length) >>> 0;
    if (ptr === 0) throw new VmError('internal', 'buf_alloc failed');
    this.u8.set(bytes, ptr);
    const instance = this.instance;
    try {
      return fn(ptr, bytes.length);
    } finally {
      if (this.instance === instance) this.call('buf_free', ptr, bytes.length);
    }
  }

  withText(text, fn) {
    return this.withBytes(encoder.encode(String(text)), fn);
  }

  /** Length of the last result in the output buffer. */
  outLen() {
    return this.x.out_len() >>> 0;
  }

  /** The output buffer as text (`len` from the export's result). */
  outText(len) {
    const ptr = this.x.out_ptr() >>> 0;
    this.sync();
    return decoder.decode(this.u8.subarray(ptr, ptr + (len >>> 0)));
  }

  /** The output buffer as JSON; `{"error": key}` is thrown as VmError. */
  outJson(len) {
    const v = JSON.parse(this.outText(len));
    if (v && typeof v === 'object' && !Array.isArray(v) && typeof v.error === 'string' && Object.keys(v).length === 1) {
      throw new VmError(v.error);
    }
    return v;
  }

  segments() {
    return this.outJson(this.call('segments'));
  }

  /** Parses a `.lvl` text. Returns a Level, or null (diagnostics in `levelDiagnostics`). */
  loadLevel(text) {
    const level = new Level(this, String(text));
    if (!level.h) return null;
    return level;
  }

  newSession(level) {
    return new Session(this, level ?? null);
  }
}

class Level {
  constructor(vm, text) {
    this.vm = vm;
    this.text = text;
    this.h = 0;
    this.infoCache = null;
    this.h = this.load(text);
    if (this.h) {
      vm.levels.add(this);
      vm.levelDiagnostics = [];
    } else {
      const res = JSON.parse(vm.outText(vm.outLen()));
      vm.levelDiagnostics = res.diagnostics ?? [{ key: res.error ?? 'internal', file: 2, line: 1, col: 1, len: 0, args: {} }];
    }
  }

  load(text) {
    const vm = this.vm;
    return vm.withText(text, (p, l) => vm.call('level_load', p, l)) >>> 0;
  }

  /** After a trap: parse the text again (a text that traps again is dropped). */
  reload() {
    const text = this.text;
    this.h = 0;
    this.text = null;
    if (text === null) return;
    this.h = this.load(text);
    this.text = text;
  }

  handle() {
    if (!this.h) throw new VmError('abi.bad_handle', 'level was freed');
    return this.h;
  }

  /** The `level_info` object (docs/ABI.md). */
  info() {
    this.infoCache ??= this.vm.outJson(this.vm.call('level_info', this.handle()));
    return structuredClone(this.infoCache);
  }

  /** Raw section text, or null if the level has no such section. */
  section(name) {
    const vm = this.vm;
    const n = vm.withText(name, (p, l) => vm.call('level_section', this.handle(), p, l));
    return n < 0 ? null : vm.outText(n);
  }

  /** Image `name` (`[image.NAME]`) as Uint8Array(4096) of colour indices, or null. */
  image(name) {
    const rows = this.section(`image.${name}`);
    return rows === null ? null : decodeImage(rows);
  }

  free() {
    if (this.h) this.vm.call('level_free', this.h);
    this.h = 0;
    this.vm.levels.delete(this);
  }
}

class Session {
  constructor(vm, level) {
    this.vm = vm;
    this.level = level;
    this.h = 0;
    this.source = null; // last successfully assembled source
    this.testIndex = -1;
    this.breakpoints = new Set();
    this.maxStep = 0;
    this.views = null;
    this.open();
    vm.sessions.add(this);
  }

  open() {
    const lv = this.level ? this.level.handle() : 0;
    const h = this.vm.call('sess_new', lv) >>> 0;
    if (h === STATUS_ERROR) throw new VmError('abi.bad_handle', 'cannot create a session');
    this.h = h;
    this.ptrs = null;
    this.views = null;
  }

  /** After a trap: new handle, reassemble the last source, restore test and breakpoints. */
  reload() {
    const { source, testIndex, breakpoints } = this;
    this.h = 0;
    this.source = null;
    this.maxStep = 0;
    this.open();
    if (source === null) return;
    const vm = this.vm;
    const errors = vm.withText(source, (p, l) => vm.call('sess_assemble', this.h, p, l)) >>> 0;
    if (errors !== 0) return;
    this.source = source;
    for (const a of breakpoints) vm.call('sess_bp', this.h, a, 1);
    if (testIndex !== -1) vm.call('sess_reset', this.h, testIndex);
  }

  handle() {
    if (!this.h) throw new VmError('abi.bad_handle', 'session was freed');
    return this.h;
  }

  /** Stable pointers (fetched once per handle). */
  pointers() {
    if (!this.ptrs) {
      const vm = this.vm;
      const h = this.handle();
      this.ptrs = {
        view: vm.call('sess_view', h) >>> 0,
        mem: vm.call('sess_mem', h) >>> 0,
        shadow: vm.call('sess_shadow', h) >>> 0,
        kinds: vm.call('sess_out_kinds', h) >>> 0,
        vals: vm.call('sess_out_vals', h) >>> 0,
      };
      if (Object.values(this.ptrs).includes(STATUS_ERROR)) {
        this.ptrs = null;
        throw new VmError('abi.bad_handle');
      }
    }
    return this.ptrs;
  }

  /** Typed-array views, recreated whenever memory.buffer changed. */
  typed() {
    const vm = this.vm;
    vm.sync();
    if (!this.views || this.views.epoch !== vm.epoch || this.views.h !== this.h) {
      const p = this.pointers();
      const buf = vm.buffer;
      this.views = {
        epoch: vm.epoch,
        h: this.h,
        view: new Uint32Array(buf, p.view, VIEW_WORDS),
        mem: new Uint8Array(buf, p.mem, MEM_BYTES),
        shadow: new Uint8Array(buf, p.shadow, SHADOW_BYTES),
        kinds: new Uint8Array(buf, p.kinds, OUTPUT_LIMIT),
        vals: new Int32Array(buf, p.vals, OUTPUT_LIMIT),
      };
    }
    return this.views;
  }

  status(name, ...args) {
    const r = this.vm.call(name, this.handle(), ...args) >>> 0;
    this.track();
    return r;
  }

  track() {
    const steps = this.typed().view[13];
    if (steps > this.maxStep) this.maxStep = steps;
  }

  json(name, ...args) {
    return this.vm.outJson(this.vm.call(name, this.handle(), ...args));
  }

  assemble(source) {
    const vm = this.vm;
    const src = String(source);
    const errors = vm.withText(src, (p, l) => vm.call('sess_assemble', this.handle(), p, l)) >>> 0;
    const res = vm.outJson(vm.outLen());
    this.source = errors === 0 ? src : null;
    this.testIndex = -1;
    this.breakpoints.clear();
    this.maxStep = 0;
    return { ok: res.ok, size: res.size, diagnostics: res.diagnostics };
  }

  reset(test = -1) {
    const t = Math.trunc(test) | 0;
    const r = this.vm.call('sess_reset', this.handle(), t) >>> 0;
    if (r !== STATUS_ERROR && r !== NOT_ASSEMBLED) {
      this.testIndex = t;
      this.maxStep = 0;
    }
    return r;
  }

  step(n = 1) { return this.status('sess_step', clampU32(n)); }
  run(maxCycles = 50000) { return this.status('sess_run', clampU32(maxCycles)); }
  back(n = 1) { return this.status('sess_back', clampU32(n)); }
  goto(step) { return this.status('sess_goto', clampU32(step)); }
  backToChange(kind, target) { return this.status('sess_back_to_change', kind >>> 0, target >>> 0); }

  setReg(reg, value) {
    const r = this.vm.call('sess_set_reg', this.handle(), reg >>> 0, value >>> 0) >>> 0;
    if (r !== STATUS_ERROR && r !== NOT_ASSEMBLED) this.maxStep = this.typed().view[13];
    return r;
  }

  setBreakpoint(addr, on) {
    const r = this.vm.call('sess_bp', this.handle(), addr >>> 0, on ? 1 : 0) >>> 0;
    if (r === 1) {
      if (on) this.breakpoints.add(addr >>> 0);
      else this.breakpoints.delete(addr >>> 0);
    }
    return r;
  }

  clearBreakpoints() {
    this.vm.call('sess_bp_clear', this.handle());
    this.breakpoints.clear();
  }

  pushInt(v) { return this.vm.call('sess_in_int', this.handle(), v | 0) >>> 0; }

  pushText(str) {
    const vm = this.vm;
    return vm.withText(str, (p, l) => vm.call('sess_in_text', this.handle(), p, l)) >>> 0;
  }

  closeInput() { return this.vm.call('sess_in_close', this.handle()) >>> 0; }

  /** The decoded StateView. */
  state() {
    this.handle();
    const v = this.typed().view;
    return {
      status: v[0], pc: v[1], regs: Array.from(v.subarray(2, 12)), flags: v[12], steps: v[13], cycles: v[14],
      faultCode: v[15], faultAddr: v[16], outLen: v[17], inIntPos: v[18], inIntLen: v[19], inTextPos: v[20],
      inTextLen: v[21], testIndex: v[22] | 0, budget: v[23],
    };
  }

  /** Furthest step reached on the current timeline (UI helper for the time slider). */
  horizon() { return Math.max(this.maxStep, this.typed().view[13]); }

  /** Live view of the 65536 memory bytes. */
  memory() { this.handle(); return this.typed().mem; }

  /** Live view of the 4096-byte heap shadow. */
  shadow() { this.handle(); return this.typed().shadow; }

  /** Visible output: [{kind, value}] (kind 0 number, 1 byte). */
  output() {
    const { view, kinds, vals } = this.typed();
    const n = Math.min(view[17], OUTPUT_LIMIT);
    const out = new Array(n);
    for (let i = 0; i < n; i++) out[i] = { kind: kinds[i], value: vals[i] };
    return out;
  }

  fault() { return this.json('sess_fault'); }
  symbols() { return this.json('sess_symbols'); }
  lineMap() { return this.json('sess_linemap'); }
  lineCounts() { return this.json('sess_line_counts'); }
  frames() { return this.json('sess_frames'); }
  heap() { return this.json('sess_heap'); }
  disasm(addr, count) { return this.json('sess_disasm', addr >>> 0, count >>> 0); }

  /** The current test evaluated from a fresh run (one run_tests entry); null without a test. */
  check() {
    try {
      return this.json('sess_check');
    } catch (err) {
      if (err instanceof VmError && err.key === 'abi.not_assembled') return null;
      throw err;
    }
  }

  runTests() {
    try {
      return this.json('sess_run_tests');
    } catch (err) {
      if (err instanceof VmError && err.key === 'abi.not_assembled') return { passed: false, tests: [], metrics: null, reference: null };
      throw err;
    }
  }

  /**
   * Soll screen of test `test` (Uint8Array(4096)) or null. For a visible test it follows its
   * `expect_screen` from `info()`: an image name (in packed levels also `__ref_NAME`) gives that
   * image, `reference` the reference solution's screen (unpacked levels only), none gives null.
   * Hidden tests: the reference screen if the level has a reference.
   */
  referenceScreen(test) {
    if (!this.level) return null;
    const index = test | 0;
    const t = this.level.info().tests.find((x) => x.index === index);
    if (t && t.expect_screen === null) return null;
    if (t && t.expect_screen !== 'reference') return this.level.image(t.expect_screen);
    const ptr = this.vm.call('sess_reference_screen', this.handle(), index) >>> 0;
    if (ptr === STATUS_ERROR || ptr === 0) return null;
    return this.vm.u8.slice(ptr, ptr + SCREEN_BYTES);
  }

  digest() {
    const vm = this.vm;
    const n = vm.call('sess_digest', this.handle());
    const text = vm.outText(n);
    if (text.startsWith('{')) vm.outJson(n);
    return text;
  }

  free() {
    if (this.h) this.vm.call('sess_free', this.h);
    this.h = 0;
    this.ptrs = null;
    this.views = null;
    this.vm.sessions.delete(this);
  }
}

function clampU32(n) {
  const v = Math.trunc(Number(n));
  if (!(v > 0)) return 0;
  return v > 0xffffffff ? 0xffffffff : v;
}
