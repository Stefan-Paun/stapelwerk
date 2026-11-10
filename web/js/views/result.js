// After solving: a test stamp, the metrics against the reference, the two
// badges, "what you just learned", solution slots and the way on.
import { el } from '../dom.js';
import { icon } from '../icons.js';
import { renderMarkup } from '../markup.js';

export function renderResult({ t, lang, level, result, record, next, onSaveSlot, tutorialMissing }) {
  const m = result.metrics;
  const ref = result.reference;
  const compact = ref ? m.size <= ref.size : false;
  const fast = ref ? m.cycles_avg <= ref.cycles : false;
  const row = (label, value, refValue, ok, unit) => el('tr', {},
    el('th', { scope: 'row' }, label),
    el('td', { class: 'mono num' }, `${value}`),
    el('td', { class: 'mono num muted' }, refValue === null ? '–' : `${refValue}`),
    el('td', { class: ['delta', ok ? 'ok' : 'over'] }, refValue === null ? '' : ok ? t('result.within') : t('result.over', { n: value - refValue, unit })));
  const badge = (key, earned) => el('li', { class: ['badge', earned && 'is-earned'] },
    el('span', { class: 'badge-mark' }, earned ? icon('check') : el('span', { class: 'badge-empty' })),
    el('span', {}, el('strong', {}, t(`result.badge.${key}`)), el('span', { class: 'muted' }, ` ${t(`result.badge.${key}.rule`)}`)),
    el('span', { class: 'sr-only' }, earned ? t('result.earned') : t('result.notEarned')));
  return el('section', { class: 'result', 'aria-labelledby': 'h-result' },
    el('div', { class: 'result-head' },
      el('p', { class: 'stamp', 'aria-hidden': 'true' }, el('span', {}, t('result.stamp'))),
      el('div', {},
        el('h2', { id: 'h-result', tabindex: '-1' }, t('result.title')),
        el('p', { class: 'muted' }, tutorialMissing ? t('result.tutorialMissing') : t('result.sub', { tests: result.tests.length })))),
    el('table', { class: 'metrics' },
      el('caption', { class: 'sr-only' }, t('result.metrics')),
      el('thead', {}, el('tr', {}, el('th', { scope: 'col' }, t('result.col.metric')), el('th', { scope: 'col' }, t('result.col.yours')), el('th', { scope: 'col' }, t('result.col.ref')), el('th', { scope: 'col' }, el('span', { class: 'sr-only' }, t('result.col.delta'))))),
      el('tbody', {},
        row(t('result.cycles'), m.cycles_avg, ref?.cycles ?? null, fast, t('result.unit.cycles')),
        row(t('result.size'), m.size, ref?.size ?? null, compact, t('result.unit.words')))),
    el('ul', { class: 'badges' }, badge('compact', compact || record.badges.compact), badge('fast', fast || record.badges.fast)),
    el('div', { class: 'learned prose' }, el('h3', {}, t('result.learned')), renderMarkup(level.learned(lang), { headingLevel: 4 })),
    el('div', { class: 'slots' },
      el('p', { class: 'muted' }, t('result.slots')),
      el('div', { class: 'slot-row' }, [0, 1, 2].map((i) => el('button', {
        type: 'button', class: 'btn btn-quiet btn-small', onclick: (ev) => { onSaveSlot(i); ev.currentTarget.textContent = t('result.slotSaved', { n: i + 1 }); },
      }, record.slots[i] ? t('result.slotReplace', { n: i + 1 }) : t('result.slotSave', { n: i + 1 }))))),
    next ? el('a', { class: 'btn btn-primary btn-next', href: `#/level/${next.id}` }, t('result.next', { num: `${next.chapter}.${next.order}`, title: next.title[lang] }), icon('chevron'))
      : el('a', { class: 'btn btn-primary btn-next', href: '#/' }, t('result.toPlan'), icon('chevron')));
}
