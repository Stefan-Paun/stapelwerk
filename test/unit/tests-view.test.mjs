// Test table texts: the budget message names the hottest line, whatever form the status has.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { describeMismatch } from '../../web/js/views/tests.js';
import { createI18n } from '../../web/js/i18n.js';
import de from '../../web/js/texts/de.js';

const { t } = createI18n('de', de);

test('budget exhaustion names cycles, line and count (core reports the status by name)', () => {
  const m = { key: 'check.not_halted', args: { status: 'BUDGET_EXCEEDED', cycles: 10000, hottest_line: 8, hottest_count: 1666 } };
  assert.equal(describeMismatch(t, m), 'Zyklenbudget aufgebraucht (10000 Zyklen): Zeile 8 lief 1666-mal. Endlosschleife?');
});

test('not halted without a budget stays the plain message', () => {
  assert.equal(describeMismatch(t, { key: 'check.not_halted', args: { status: 'FAULT' } }), de['check.not_halted']);
});
