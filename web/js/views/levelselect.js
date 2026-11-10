// Level select as a chapter plan: each chapter is a dimension chain, each
// level a station on it. Solved stations are inked, open ones drawn, locked
// ones dashed. Badges and level kinds are spelled out, never colour-only.
import { el } from '../dom.js';
import { icon } from '../icons.js';

const KIND_KEYS = { tutorial: 'kind.tutorial', debug: 'kind.debug', final: 'kind.final', bonus: 'kind.bonus' };

export async function renderLevelSelect(app) {
  const { t, lang, progress } = app;
  const { levels } = await app.levels.index();
  const chapters = [...new Set(levels.map((l) => l.chapter))].sort((a, b) => a - b);
  const unlocked = (id) => progress.isUnlocked(levels, id);
  const nextUp = levels.find((l) => unlocked(l.id) && !progress.level(l.id).solved && l.kind !== 'bonus');
  const solvedCount = levels.filter((l) => progress.level(l.id).solved).length;

  const toggle = el('input', {
    type: 'checkbox', id: 'unlock-all', checked: progress.data.unlockAll,
    onchange: (ev) => { progress.setUnlockAll(ev.target.checked); app.rerender(); },
  });

  const station = (l) => {
    const rec = progress.level(l.id);
    const open = unlocked(l.id);
    const state = rec.solved ? 'solved' : open ? 'open' : 'locked';
    const tags = [];
    if (KIND_KEYS[l.kind]) tags.push(t(KIND_KEYS[l.kind]));
    if (l.tags?.includes('screen')) tags.push(t('kind.screen'));
    if (l.mobile) tags.push(t('kind.mobile'));
    const badges = [rec.badges.compact && t('result.badge.compact'), rec.badges.fast && t('result.badge.fast')].filter(Boolean);
    const inner = [
      el('span', { class: 'st-node', 'aria-hidden': 'true' }, state === 'solved' ? icon('check') : state === 'locked' ? icon('lock') : null),
      el('span', { class: 'st-num mono' }, `${l.chapter}.${l.order}`),
      el('span', { class: 'st-title' }, l.title[lang] ?? l.title.de),
      el('span', { class: 'st-card' }, l.card[lang] ?? l.card.de),
      tags.length ? el('span', { class: 'st-tags' }, tags.join(' · ')) : null,
      badges.length ? el('span', { class: 'st-badges' }, badges.map((b) => el('span', { class: 'mini-badge' }, b))) : null,
      el('span', { class: 'sr-only' }, ` (${t(`plan.state.${state}`)})`),
    ];
    return el('li', { class: ['station', `is-${state}`, l.kind === 'bonus' && 'is-bonus', nextUp?.id === l.id && 'is-next'] },
      open
        ? el('a', { class: 'st-link', href: `#/level/${l.id}`, 'aria-current': nextUp?.id === l.id ? 'step' : null }, inner)
        : el('span', { class: 'st-link', 'aria-disabled': 'true' }, inner));
  };

  const chapterNode = (c) => {
    const inChapter = levels.filter((l) => l.chapter === c);
    const required = inChapter.filter((l) => l.kind !== 'bonus');
    const done = required.filter((l) => progress.level(l.id).solved).length;
    const complete = progress.chapterComplete(levels, c);
    const open = inChapter.some((l) => unlocked(l.id));
    return el('li', { class: ['chapter', complete && 'is-complete', !open && 'is-locked'], 'aria-labelledby': `ch-${c}` },
      el('div', { class: 'chapter-head' },
        el('span', { class: 'chapter-no mono', 'aria-hidden': 'true' }, String(c)),
        el('h2', { id: `ch-${c}` }, el('span', { class: 'sr-only' }, `${t('plan.chapterShort')} ${c}: `), t(`chapter.${c}`)),
        el('span', { class: 'chapter-progress mono' }, t('plan.progress', { done, total: required.length }))),
      el('ol', { class: 'stations', dataset: { count: inChapter.length } }, inChapter.map(station)));
  };

  return el('div', { class: 'page page-plan' },
    el('header', { class: 'plan-head' },
      el('div', { class: 'plan-intro' },
        el('h1', { id: 'page-title', tabindex: '-1' }, t('plan.title')),
        el('p', { class: 'lede' }, t('plan.lede'))),
      el('div', { class: 'plan-side' },
        nextUp ? el('a', { class: 'btn btn-primary', href: `#/level/${nextUp.id}` },
          el('span', {}, t('plan.continue', { num: `${nextUp.chapter}.${nextUp.order}`, title: nextUp.title[lang] ?? nextUp.title.de })), icon('chevron')) : null,
        el('p', { class: 'plan-count mono' }, t('plan.solved', { done: solvedCount, total: levels.length })),
        el('label', { class: 'switch', for: 'unlock-all' }, toggle, el('span', { class: 'switch-track', 'aria-hidden': 'true' }), el('span', {}, t('plan.unlockAll'))),
        app.vm.mock ? el('p', { class: 'mock-note' }, t('plan.mockNote')) : null)),
    el('ol', { class: 'chapters' }, chapters.map(chapterNode)));
}
