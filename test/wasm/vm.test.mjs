// web/js/vm.js against the built module (docs/ABI.md, "web/js/vm.js contract").
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import { STATUS_ERROR, VmError, loadVm } from '../../web/js/vm.js';
import path from 'node:path';
import { PACKED_LEVEL_FILE, ROOT, levelText, lvlSection, referenceSource, wasmPath } from './helpers.mjs';

const S = { PAUSED: 0, HALTED: 1, FAULT: 2, BREAKPOINT: 3, WAITING: 4, BUDGET: 5, NOT_ASSEMBLED: 6 };
const bytes = readFileSync(wasmPath());
const vm = await loadVm(bytes);

function withSession(level, fn) {
  const s = vm.newSession(level);
  try {
    return fn(s);
  } finally {
    s.free();
  }
}

function withLevel(text, fn) {
  const lv = vm.loadLevel(text);
  assert.ok(lv, JSON.stringify(vm.levelDiagnostics));
  try {
    return fn(lv);
  } finally {
    lv.free();
  }
}

/** A module whose abi_version() returns 2 (hand-assembled). */
function moduleWithVersion2() {
  const name = (s) => [s.length, ...new TextEncoder().encode(s)];
  const exports = [2, ...name('abi_version'), 0, 0, ...name('memory'), 2, 0];
  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type: [] -> [i32]
    0x03, 0x02, 0x01, 0x00, // function 0 has type 0
    0x05, 0x03, 0x01, 0x00, 0x01, // memory: min 1 page
    0x07, exports.length, ...exports,
    0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x02, 0x0b, // body: i32.const 2
  ]);
}

test('loadVm checks the ABI version', async () => {
  assert.equal(vm.abiVersion, 1);
  assert.equal(vm.mock, false);
  await assert.rejects(loadVm(moduleWithVersion2()), (err) => err instanceof VmError && err.key === 'abi.version');
  const fromModule = await loadVm(new WebAssembly.Module(bytes));
  assert.equal(fromModule.segments().length, 8);
});

test('segments', () => {
  const segs = vm.segments();
  assert.deepEqual(segs[0], { name: 'null', start: 0, end: 256, r: false, w: false, x: false });
  assert.deepEqual(segs.map((s) => s.name), ['null', 'text', 'data', 'input', 'heap', 'guard', 'stack', 'screen']);
});

test('level 2.2: info, sections, diagnostics of a broken level', () => {
  assert.equal(vm.loadLevel('[level]\nid = x\n'), null);
  assert.ok(vm.levelDiagnostics.length > 0);
  assert.match(vm.levelDiagnostics[0].key, /^lvl\./);
  assert.equal(vm.levelDiagnostics[0].file, 2);
  withLevel(levelText(), (lv) => {
    assert.deepEqual(vm.levelDiagnostics, []);
    const info = lv.info();
    assert.equal(info.id, '2-2-bis-zur-null');
    assert.deepEqual(info.title, { de: 'Bis zur Null', en: 'Until Zero' });
    assert.deepEqual(info.tests.map((t) => t.index), [0, 1, 2, 3]);
    assert.equal(info.tests[0].name, 'grundfall');
    assert.equal(info.hidden_tests, 11);
    assert.equal(info.ref_cycles, null);
    assert.ok(info.sections.includes('start'));
    assert.ok(!info.sections.includes('reference'));
    assert.match(lv.section('start'), /halt/);
    assert.match(lv.section('hint.de'), /\n---\n/);
    assert.equal(lv.section('reference'), null);
    assert.equal(lv.section('nope'), null);
  });
});

test('level 2.2: the reference solution passes every test, the empty program does not', (t) => {
  withLevel(levelText(), (lv) => withSession(lv, (s) => {
    const t0 = performance.now();
    const asm = s.assemble(referenceSource());
    const report = s.runTests();
    const ms = performance.now() - t0;
    t.diagnostic(`assemble reference + run all ${report.tests.length} tests (incl. reference run): ${ms.toFixed(2)} ms`);
    assert.deepEqual(asm, { ok: true, size: 9, diagnostics: [] });
    assert.equal(report.passed, true);
    assert.equal(report.tests.length, 15);
    assert.ok(report.tests.every((r) => r.passed && r.status === S.HALTED && r.fault === null && r.mismatch === null));
    assert.deepEqual(report.tests[0].input, { in: [3, 4, 5, 0], in_text: '', out: [12], out_text: null });
    assert.equal(report.tests[14].hidden, true);
    assert.equal('input' in report.tests[14], false, 'hidden and passed: no input');
    assert.deepEqual(report.reference, { cycles: report.metrics.cycles_avg, size: 9 });
    assert.equal(report.metrics.size, 9);

    const t1 = performance.now();
    s.assemble(referenceSource());
    s.runTests();
    t.diagnostic(`second run (reference cached): ${(performance.now() - t1).toFixed(2)} ms`);

    assert.equal(s.assemble('').ok, true);
    const empty = s.runTests();
    assert.equal(empty.passed, false);
    const failed = empty.tests.find((r) => r.hidden && !r.passed);
    assert.ok(failed.input, 'failed hidden test carries its input');
    assert.equal(empty.tests[0].passed, false);
  }));
});

test('packed level 2.2 (dist/levels) passes with the reference solution', { skip: !existsSync(PACKED_LEVEL_FILE) && 'run npm run build first' }, (t) => {
  withLevel(readFileSync(PACKED_LEVEL_FILE, 'utf8'), (lv) => withSession(lv, (s) => {
    const info = lv.info();
    assert.equal(typeof info.ref_cycles, 'number');
    assert.equal(info.ref_size, 9);
    const t0 = performance.now();
    s.assemble(referenceSource());
    const report = s.runTests();
    t.diagnostic(`packed: assemble reference + run all tests: ${(performance.now() - t0).toFixed(2)} ms`);
    assert.equal(report.passed, true);
    assert.deepEqual(report.reference, { cycles: info.ref_cycles, size: info.ref_size });
    assert.equal(s.referenceScreen(0), null, 'no reference in packed levels');
  }));
});

test('assembler diagnostics arrive with line and column', () => {
  withSession(null, (s) => {
    const res = s.assemble('        mov r1, 0\n        ad r1, 2\n  mov r9, 1\n');
    assert.equal(res.ok, false);
    assert.equal(res.size, 0);
    const [d1, d2] = res.diagnostics;
    assert.deepEqual({ key: d1.key, file: d1.file, line: d1.line, col: d1.col, len: d1.len }, { key: 'asm.unknown_mnemonic', file: 0, line: 2, col: 9, len: 2 });
    assert.equal(d1.args.suggestion, 'add');
    assert.equal(d2.line, 3);
    assert.equal(s.state().status, S.NOT_ASSEMBLED);
    assert.equal(s.step(1), S.NOT_ASSEMBLED);
    assert.equal(s.check(), null);
    assert.deepEqual(s.runTests(), { passed: false, tests: [], metrics: null, reference: null });
  });
});

test('debugger: step, back, goto, backToChange, setReg, breakpoints', () => {
  withLevel(levelText(), (lv) => withSession(lv, (s) => {
    assert.equal(s.assemble(referenceSource()).ok, true);
    const lines = new Map(s.lineMap().filter((e) => e.f === 0).map((e) => [e.l, e.a]));
    assert.equal(s.reset(0), S.PAUSED);
    assert.equal(s.state().testIndex, 0);
    assert.equal(s.step(1), S.PAUSED);
    assert.equal(s.state().steps, 1);
    assert.equal(s.run(), S.HALTED);
    const end = s.state();
    assert.deepEqual(s.output(), [{ kind: 0, value: 12 }]);
    assert.equal(s.horizon(), end.steps);
    const finalDigest = s.digest();
    assert.match(finalDigest, /^[0-9a-f]{16}$/);

    assert.equal(s.back(2), S.PAUSED); // before `sys WRITE_INT`
    assert.equal(s.state().steps, end.steps - 2);
    assert.equal(s.state().outLen, 0, 'output truncated when going back');
    assert.equal(s.horizon(), end.steps);
    assert.equal(s.goto(5), S.PAUSED);
    assert.equal(s.state().steps, 5);
    const d5 = s.digest();
    assert.equal(s.goto(end.steps), S.HALTED);
    assert.equal(s.digest(), finalDigest);
    s.reset(0);
    s.step(5);
    assert.equal(s.digest(), d5, 'goto matches a fresh run');

    // r1 holds the sum; its last change is the final `add r1, r0` (7 → 12).
    s.run();
    assert.equal(s.backToChange(0, 1), S.PAUSED);
    const before = s.state();
    assert.equal(before.regs[1], 7);
    assert.match(s.disasm(before.pc, 1)[0].text, /^add r1, /);
    assert.equal(s.backToChange(3, 0), STATUS_ERROR);

    // Editing a register discards the future and is replayed by goto.
    s.reset(0);
    s.step(1);
    assert.equal(s.setReg(1, 100), S.PAUSED);
    assert.equal(s.horizon(), 1);
    assert.equal(s.run(), S.HALTED);
    assert.deepEqual(s.output(), [{ kind: 0, value: 112 }]);
    s.goto(0);
    s.run();
    assert.deepEqual(s.output(), [{ kind: 0, value: 112 }]);
    assert.equal(s.setReg(1, 0), STATUS_ERROR, 'not while halted');
    s.goto(1);
    assert.equal(s.setReg(12, 0), STATUS_ERROR, 'bad register');

    // Breakpoint on `add r1, r0` (line 6 of the reference).
    const addAddr = lines.get(6);
    assert.equal(s.setBreakpoint(addAddr, true), 1);
    assert.equal(s.setBreakpoint(0x4000, true), 0, 'not an instruction');
    s.reset(0);
    assert.equal(s.run(), S.BREAKPOINT);
    assert.equal(s.state().pc, addAddr);
    assert.equal(s.state().regs[0], 3);
    assert.equal(s.run(), S.BREAKPOINT);
    assert.equal(s.state().regs[0], 4);
    assert.equal(s.step(1), S.PAUSED, 'step ignores breakpoints');
    s.clearBreakpoints();
    assert.equal(s.run(), S.HALTED);
    assert.deepEqual(s.lineCounts().find(([line]) => line === 6), [6, 3]);
    assert.equal(s.check().passed, true);
  }));
});

test('sandbox: WAITING_INPUT, pushInt, pushText, closeInput', () => {
  withSession(null, (s) => {
    assert.equal(s.assemble(referenceSource()).ok, true);
    assert.equal(s.state().testIndex, -1);
    assert.equal(s.run(), S.WAITING);
    assert.equal(s.pushInt(5), S.WAITING);
    assert.equal(s.run(), S.WAITING);
    assert.equal(s.state().inIntPos, 1);
    s.pushInt(-2);
    s.pushInt(0);
    assert.equal(s.state().inIntLen, 3);
    assert.equal(s.run(), S.HALTED);
    assert.deepEqual(s.output(), [{ kind: 0, value: 3 }]);
    assert.equal(s.check(), null, 'no test in the sandbox');
    assert.equal(s.referenceScreen(0), null);

    const echo = 'loop:\n        sys READ_CHAR\n        cmp r0, -1\n        jz done\n        sys WRITE_CHAR\n        jmp loop\ndone:\n        halt\n';
    assert.equal(s.assemble(echo).ok, true);
    assert.equal(s.run(), S.WAITING);
    assert.equal(s.pushText('hé'), S.WAITING);
    assert.equal(s.state().inTextLen, 3, 'UTF-8 bytes');
    assert.equal(s.run(), S.WAITING);
    assert.equal(s.closeInput(), S.WAITING);
    assert.equal(s.run(), S.HALTED);
    assert.deepEqual(s.output().map((o) => o.value), [0x68, 0xc3, 0xa9]);
    assert.ok(s.output().every((o) => o.kind === 1));
  });
});

test('time slices: run(50000) repeatedly until HALTED', () => {
  withSession(null, (s) => {
    const src = '        li r1, 200000\nloop:\n        sub r1, 1\n        jnz loop\n        mov r0, r1\n        sys WRITE_INT\n        halt\n';
    assert.equal(s.assemble(src).ok, true);
    let slices = 0;
    let status;
    let lastCycles = 0;
    do {
      status = s.run(50000);
      slices++;
      const st = s.state();
      if (status === S.PAUSED) assert.ok(st.cycles - lastCycles >= 50000, 'a slice uses its cycles');
      lastCycles = st.cycles;
    } while (status === S.PAUSED && slices < 1000);
    assert.equal(status, S.HALTED);
    assert.ok(slices > 5, `${slices} slices`);
    const liWords = (s.lineMap().find((e) => e.l === 3).a - 256) / 4;
    assert.equal(s.state().steps, liWords + 2 * 200000 + 3);
    assert.deepEqual(s.output(), [{ kind: 0, value: 0 }]);
    // Time travel across many snapshots.
    assert.equal(s.goto(liWords + 2 * 61728), S.PAUSED);
    assert.equal(s.state().regs[1], 200000 - 61728);
  });
});

test('JSON queries: fault, symbols, frames, heap, disasm', () => {
  withSession(null, (s) => {
    const src = [
      'main:', '        li r0, 12', '        sys ALLOC', '        call f', '        mov r1, 0', '        div r0, r1', '        halt',
      'f:', '        push fp', '        mov fp, sp', '        pop fp', '        ret',
    ].join('\n');
    assert.equal(s.assemble(src).ok, true);
    const lines = new Map(s.lineMap().map((e) => [e.l, e.a]));
    assert.deepEqual(s.lineMap()[0], { a: 256, f: 0, l: 2 });
    assert.deepEqual(s.symbols().find((x) => x.name === 'f'), { name: 'f', kind: 'code', value: lines.get(9), file: 0, line: 8 });
    assert.equal(s.fault(), null);
    s.setBreakpoint(lines.get(11), true); // `pop fp` inside f
    assert.equal(s.run(), S.BREAKPOINT);
    const frames = s.frames();
    assert.equal(frames[0].func, 'f');
    assert.deepEqual(Object.keys(frames[0]), ['fp', 'ret', 'func', 'size']);
    s.clearBreakpoints();
    assert.equal(s.run(), S.FAULT);
    const fault = s.fault();
    assert.equal(fault.key, 'fault.div_zero');
    assert.deepEqual(Object.keys(fault), ['key', 'pc', 'step', 'args']);
    const st = s.state();
    assert.equal(st.faultCode, 14);
    assert.equal(fault.pc, st.pc);
    assert.equal(fault.pc, lines.get(6));
    const heap = s.heap();
    assert.equal(heap.length, 1);
    assert.deepEqual(heap[0], { addr: 0x8000, size: 12, live: true, line: 3 });
    assert.equal(s.shadow()[0], 1, 'allocated granule');
    const [d] = s.disasm(256, 1);
    assert.deepEqual(Object.keys(d), ['addr', 'word', 'text', 'label']);
    assert.equal(d.label, 'main');
    assert.equal(d.word, (s.memory()[256] | (s.memory()[257] << 8) | (s.memory()[258] << 16) | (s.memory()[259] << 24)) >>> 0);
    assert.throws(() => s.disasm(0x10000, 1), (err) => err instanceof VmError && err.key === 'abi.bad_arg');
  });
});

test('typed-array views stay valid after memory.grow', () => {
  withSession(null, (s) => {
    s.assemble('        li r0, 0x1234\n        st r0, [0x4000]\n        halt\n');
    s.run();
    const before = s.memory();
    const oldBuffer = vm.mem.buffer;
    const grown = [];
    try {
      while (vm.mem.buffer === oldBuffer && grown.length < 500) {
        const x = vm.newSession(null);
        x.assemble('        halt\n');
        grown.push(x);
      }
      assert.notEqual(vm.mem.buffer, oldBuffer, 'memory grew');
      assert.equal(before.byteLength, 0, 'the old view is detached');
      const mem = s.memory();
      assert.equal(mem.length, 65536);
      assert.equal(mem[0x4000] | (mem[0x4001] << 8), 0x1234);
      assert.equal(s.shadow().length, 4096);
      assert.equal(s.state().status, S.HALTED);
      assert.equal(s.memory(), mem, 'same view while the buffer is unchanged');
    } finally {
      for (const x of grown) x.free();
    }
  });
});

test('bad handles', () => {
  const s = vm.newSession(null);
  s.free();
  assert.throws(() => s.step(1), (err) => err instanceof VmError && err.key === 'abi.bad_handle');
  assert.throws(() => s.state(), VmError);
  const lv = vm.loadLevel(levelText());
  const handle = lv.h;
  lv.free();
  assert.throws(() => lv.info(), (err) => err.key === 'abi.bad_handle');
  assert.throws(() => vm.newSession(lv), (err) => err.key === 'abi.bad_handle');
  assert.equal(vm.x.sess_step(handle, 1) >>> 0, STATUS_ERROR);
  assert.throws(() => vm.outJson(vm.call('sess_fault', 0xdead)), (err) => err.key === 'abi.bad_handle');
  // A level freed after the session was created keeps working inside the session.
  const lv2 = vm.loadLevel(levelText());
  const s2 = vm.newSession(lv2);
  lv2.free();
  s2.assemble(referenceSource());
  assert.equal(s2.runTests().passed, true);
  s2.free();
});

test('trap: new instance, sources reloaded, error "internal"', () => {
  withLevel(levelText(), (lv) => withSession(lv, (s) => {
    s.assemble(referenceSource());
    s.reset(2);
    s.setBreakpoint(s.lineMap().find((e) => e.l === 6).a, true);
    const oldInstance = vm.instance;
    const reported = [];
    vm.onInternalError = (err) => reported.push(err);
    vm.x = { ...vm.x, sess_step() { throw new WebAssembly.RuntimeError('unreachable'); } };
    try {
      assert.throws(() => s.step(1), (err) => err instanceof VmError && err.key === 'internal');
    } finally {
      vm.onInternalError = null;
    }
    assert.equal(reported.length, 1);
    assert.notEqual(vm.instance, oldInstance);
    const st = s.state();
    assert.equal(st.status, S.PAUSED);
    assert.equal(st.testIndex, 2);
    assert.equal(st.steps, 0);
    assert.equal(s.run(), S.BREAKPOINT, 'breakpoints restored');
    s.clearBreakpoints();
    assert.equal(s.run(), S.HALTED);
    assert.deepEqual(s.output(), [{ kind: 0, value: 3 }]);
    assert.equal(lv.info().id, '2-2-bis-zur-null', 'level reloaded');
    assert.equal(s.runTests().passed, true);
  }));
});

const SCREEN_LEVEL = [
  '[level]', 'id = bild', 'title.de = Bild', 'title.en = Picture', 'card.de = x', 'card.en = x', '',
  '[reference]', '        li r0, 0xF000', '        mov r1, 5', '        stb r1, [r0 + 1]', '        halt', '',
  '[test]', 'name = ref', 'expect_screen = reference', '',
  '[test]', 'name = named', 'screen_init = start', 'expect_screen = ziel', '',
  '[test]', 'name = packed', 'expect_screen = __ref_ref', '',
  '[test]', 'name = none', 'in = 1 2', 'out = 3', '',
  '[test]', 'name = versteckt', 'hidden = yes', 'expect_screen = reference', '',
  '[image.start]', 'F', '',
  '[image.ziel]', '.', '.A1', '',
  '[image.__ref_ref]', '.5', '',
].join('\n');

test('level_info test details (ABI v1 extension)', () => {
  withLevel(levelText(), (lv) => {
    const [t0] = lv.info().tests;
    assert.deepEqual(t0, {
      index: 0, name: 'grundfall', in: [3, 4, 5, 0], in_text: '', out: [12], out_text: null, regs: [], set: [],
      call: null, expect_reg: [], expect: [], screen_init: null, expect_screen: null, heap_empty: false,
    });
  });
  withLevel(SCREEN_LEVEL, (lv) => {
    const tests = lv.info().tests;
    assert.deepEqual(tests.map((t) => [t.name, t.screen_init, t.expect_screen]), [
      ['ref', null, 'reference'], ['named', 'start', 'ziel'], ['packed', null, '__ref_ref'], ['none', null, null],
    ]);
    assert.equal(lv.info().hidden_tests, 1);
  });
});

test('budget end: status 5 with cycles and hottest line (ABI v1 extension)', () => {
  const text = readFileSync(path.join(ROOT, 'levels', '2-4-endlosschleife.lvl'), 'utf8');
  withLevel(text, (lv) => withSession(lv, (s) => {
    assert.equal(s.assemble(lvlSection(text, 'start')).ok, true);
    const report = s.runTests();
    const t = report.tests.find((r) => r.name === 'ungerade');
    assert.equal(t.passed, false);
    assert.equal(t.status, S.BUDGET);
    assert.equal(t.fault, null);
    assert.equal(t.cycles, 10000);
    assert.equal(t.mismatch.key, 'check.not_halted');
    const { status, cycles, hottest_line: line, hottest_count: count } = t.mismatch.args;
    assert.deepEqual([status, cycles, line], ['BUDGET_EXCEEDED', 10000, 8]);
    assert.ok(count > 1000, `line ${line} ran ${count} times`);
    // sess_check reports the same entry; the line counters agree.
    assert.equal(s.reset(t.index), S.PAUSED);
    const checked = s.check();
    assert.deepEqual(checked.mismatch, t.mismatch);
    assert.equal(s.run(1e9), S.BUDGET);
    const counts = new Map(s.lineCounts());
    assert.equal(counts.get(8), count);
    // A passing test keeps the plain form.
    assert.equal(report.tests.find((r) => r.name === 'gerade').mismatch, null);
  }));
});

test('level_info: built-in constants in addresses are resolved', () => {
  const text = '[level]\nid = a\n\n[test]\nname = t\nset INPUT + 4 = 1\nexpectb STACK_TOP - 1 = 2\nexpect werte + 8 = 3\n';
  withLevel(text, (lv) => {
    const [t] = lv.info().tests;
    assert.deepEqual(t.set, [{ addr: 0x6004, words: [1] }]);
    assert.deepEqual(t.expect, [{ addr: 0xEFFF, bytes: [2] }, { addr: 'werte + 8', words: [3] }]);
  });
});

test('Level.image and referenceScreen', () => {
  withLevel(SCREEN_LEVEL, (lv) => withSession(lv, (s) => {
    const ziel = lv.image('ziel');
    assert.equal(ziel.length, 4096);
    assert.deepEqual([ziel[0], ziel[64], ziel[65], ziel[66], ziel[67]], [0, 0, 10, 1, 0]);
    assert.equal(lv.image('start')[0], 15);
    assert.equal(lv.image('fehlt'), null);
    const ref = s.referenceScreen(0);
    assert.equal(ref.length, 4096);
    assert.equal(ref[1], 5, 'reference solution drew pixel 1');
    assert.deepEqual(s.referenceScreen(1), ziel);
    assert.equal(s.referenceScreen(2)[1], 5, 'packed reference image');
    assert.equal(s.referenceScreen(3), null, 'no screen expectation');
    assert.equal(s.referenceScreen(4)[1], 5, 'hidden test: reference run');
    assert.equal(s.referenceScreen(99), null);
    // The reference passes its own screen test; a black screen does not.
    s.assemble('        li r0, 0xF000\n        mov r1, 5\n        stb r1, [r0 + 1]\n        halt\n');
    const report = s.runTests();
    assert.equal(report.tests[0].passed, true);
    assert.equal(report.tests[1].mismatch.key, 'check.screen_mismatch');
  }));
  withLevel(levelText(), (lv) => withSession(lv, (s) => {
    assert.equal(s.referenceScreen(0), null, 'level 2.2 has no screen tests');
  }));
});

test('reset with a missing call label: start fault as in the test runner', () => {
  const text = readFileSync(path.join(ROOT, 'levels', '6-1-fakultaet.lvl'), 'utf8');
  const code = lvlSection(text, 'reference').replaceAll('fak:', 'fakultaet:').replaceAll('call fak ', 'call fakultaet ');
  withLevel(text, (lv) => withSession(lv, (s) => {
    assert.equal(s.assemble(code).ok, true);
    assert.equal(s.reset(0), S.FAULT);
    assert.equal(s.fault().key, 'fault.bad_jump_target');
    assert.equal(s.state().testIndex, 0);
    assert.equal(s.state().steps, 0);
    assert.equal(s.run(), S.FAULT, 'the run cannot continue');
    const checked = s.check();
    assert.equal(checked.status, S.FAULT);
    assert.equal(checked.fault.key, 'fault.bad_jump_target');
  }));
});
