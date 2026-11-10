import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import de from '../../web/js/texts/de.js';
import en from '../../web/js/texts/en.js';
import { createI18n, format, placeholders } from '../../web/js/i18n.js';
import { MANUAL_TOPICS } from '../../web/js/views/manual.js';

test('de and en have the same keys', () => {
  const a = Object.keys(de).sort();
  const b = Object.keys(en).sort();
  assert.deepEqual(a.filter((k) => !b.includes(k)), [], 'only in de');
  assert.deepEqual(b.filter((k) => !a.includes(k)), [], 'only in en');
});

test('de and en use the same placeholders per key', () => {
  const diff = [];
  for (const k of Object.keys(de)) if (placeholders(de[k]).join() !== placeholders(en[k] ?? '').join()) diff.push(k);
  assert.deepEqual(diff, []);
});

test('no empty texts', () => {
  for (const [k, v] of [...Object.entries(de), ...Object.entries(en)]) assert.ok(typeof v === 'string' && v.trim() !== '', k);
});

test('format fills placeholders and keeps unknown ones visible', () => {
  assert.equal(format('Schritt {step} / {total}', { step: 3, total: 9 }), 'Schritt 3 / 9');
  assert.equal(format('x {y}', {}), 'x {y}');
  const i = createI18n('de', { a: 'A {n}' });
  assert.equal(i.t('a', { n: 1 }), 'A 1');
  assert.equal(i.t('missing.key'), 'missing.key');
});

// Every key the code asks for must exist.
const js = fileURLToPath(new URL('../../web/js/', import.meta.url));
function sources(dir) {
  return readdirSync(dir).flatMap((n) => {
    const p = path.join(dir, n);
    if (statSync(p).isDirectory()) return n === 'texts' ? [] : sources(p);
    return n.endsWith('.js') ? [readFileSync(p, 'utf8')] : [];
  });
}

test('no unknown key in the code', () => {
  const code = sources(js).join('\n');
  const literal = new Set();
  for (const m of code.matchAll(/\bt\(\s*'([a-zA-Z0-9_.]+)'/g)) literal.add(m[1]);
  for (const m of code.matchAll(/\?\s*'([a-z]+\.[a-zA-Z0-9_.]+)'\s*:\s*'([a-z]+\.[a-zA-Z0-9_.]+)'/g)) { literal.add(m[1]); literal.add(m[2]); }
  const missing = [...literal].filter((k) => !(k in de));
  assert.deepEqual(missing, []);
});

test('key families built at run time exist', () => {
  const families = {
    'chapter.': [0, 1, 2, 3, 4, 5, 6, 7],
    'status.': ['paused', 'halted', 'fault', 'breakpoint', 'waiting', 'budget', 'not_assembled', 'running'],
    'seg.': ['text', 'data', 'input', 'heap', 'stack', 'screen'],
    'seg.rights.': ['text', 'data', 'input', 'heap', 'stack', 'screen'],
    'hex.': ['byte', 'word'],
    'screen.mode.': ['ist', 'soll', 'diff'],
    'heap.state.': ['free', 'live', 'freed', 'red'],
    'heap.col.': ['addr', 'size', 'status', 'line'],
    'ws.tab.': ['task', 'code', 'regs', 'stack', 'screen', 'console', 'mem'],
    'tutorial.': ['step', 'breakpoint', 'run', 'back'],
    'regs.flag.': ['Z', 'N', 'C', 'V'],
    'result.badge.': ['compact', 'fast', 'compact.rule', 'fast.rule'],
    'plan.state.': ['solved', 'open', 'locked'],
    'help.': ['step', 'back', 'runPause', 'restart', 'breakpoint', 'tests', 'help'],
    'nav.': ['home', 'sandbox', 'manual', 'about'],
    'about.': ['what.title', 'privacy.title', 'progress.title', 'keys.title', 'licenses.title', 'ai.title', 'engine.title'],
    'sandbox.ex.': ['count', 'echo', 'hello', 'list', 'flood', 'count.note', 'echo.note', 'hello.note', 'list.note', 'flood.note'],
    'access.': ['r', 'w'],
    'kind.': ['tutorial', 'debug', 'final', 'bonus', 'screen', 'mobile'],
    'regs.': ['callerSaved', 'calleeSaved', 'pointers'],
  };
  const missing = [];
  for (const [prefix, names] of Object.entries(families)) for (const n of names) if (!(`${prefix}${n}` in de)) missing.push(`${prefix}${n}`);
  for (const topic of MANUAL_TOPICS) for (const part of ['title', 'body']) if (!(`manual.${topic}.${part}` in de)) missing.push(`manual.${topic}.${part}`);
  assert.deepEqual(missing, []);
});

test('every error key from the spec has a text', () => {
  const keys = [
    'fault.illegal_instruction', 'fault.zero_instruction', 'fault.exec_outside_code', 'fault.bad_jump_target', 'fault.bad_return_address',
    'fault.null_access', 'fault.out_of_range', 'fault.guard_page', 'fault.write_to_code', 'fault.misaligned', 'fault.stack_overflow',
    'fault.stack_underflow', 'fault.stack_misaligned', 'fault.div_zero', 'fault.input_end', 'fault.heap_unallocated', 'fault.heap_overflow',
    'fault.use_after_free', 'fault.invalid_free', 'fault.double_free', 'fault.unknown_syscall', 'fault.string_too_long', 'fault.output_limit',
    'run.budget_exceeded',
    'asm.unknown_mnemonic', 'asm.unknown_register', 'asm.operand_count', 'asm.operand_type', 'asm.imm_range', 'asm.undefined_symbol',
    'asm.duplicate_label', 'asm.reserved_name', 'asm.bad_number', 'asm.bad_string', 'asm.unexpected_char', 'asm.local_label_without_global',
    'asm.needs_constant', 'asm.expr_div_zero', 'asm.expr_overflow', 'asm.data_in_text', 'asm.code_in_data', 'asm.section_overflow',
    'asm.forbidden_instruction', 'asm.too_many_errors',
    'lvl.unknown_section', 'lvl.unknown_key', 'lvl.missing_key', 'lvl.bad_value', 'lvl.duplicate_section', 'lvl.no_tests', 'lvl.allow_and_forbid', 'lvl.bad_image',
    'check.out_mismatch', 'check.out_missing', 'check.out_extra', 'check.text_mismatch', 'check.mem_mismatch', 'check.reg_mismatch',
    'check.screen_mismatch', 'check.heap_not_empty', 'check.abi_clobbered', 'check.not_halted',
    'abi.bad_handle', 'abi.bad_arg', 'abi.not_assembled',
  ];
  assert.deepEqual(keys.filter((k) => !(k in de)), []);
});
