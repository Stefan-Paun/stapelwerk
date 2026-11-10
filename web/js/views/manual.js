// Manual: short reference topics. Used as a tab in the workspace and as the
// #/manual route (a reading page with the topic list beside it).
import { el, mount } from '../dom.js';
import { renderMarkup } from '../markup.js';

export const MANUAL_TOPICS = ['register', 'befehle', 'flags', 'zweierkomplement', 'speicher', 'stack', 'aufruf', 'heap', 'bildschirm', 'systemaufrufe', 'debugger'];

export function topicContent(t, topic, headingLevel = 2) {
  const id = MANUAL_TOPICS.includes(topic) ? topic : MANUAL_TOPICS[0];
  return el('article', { class: 'prose manual-article' },
    el(`h${headingLevel}`, {}, t(`manual.${id}.title`)),
    renderMarkup(t(`manual.${id}.body`), { headingLevel: headingLevel + 1 }));
}

/** Compact manual for the workspace tab: a topic picker and the article. */
export function createManualTab({ t }) {
  let topic = MANUAL_TOPICS[0];
  const select = el('select', { 'aria-label': t('manual.pick'), onchange: () => { topic = select.value; render(); } },
    MANUAL_TOPICS.map((id) => el('option', { value: id }, t(`manual.${id}.title`))));
  const body = el('div', { class: 'manual-body' });
  const node = el('div', { class: 'manual-tab' }, el('label', { class: 'field-inline' }, t('manual.topic'), select), body);
  function render() { mount(body, topicContent(t, topic, 3)); }
  render();
  return {
    node,
    show(id) { if (MANUAL_TOPICS.includes(id)) { topic = id; select.value = id; render(); } },
  };
}

/** #/manual[/<topic>] page. */
export function renderManualPage(app, topic) {
  const { t } = app;
  const current = MANUAL_TOPICS.includes(topic) ? topic : MANUAL_TOPICS[0];
  return el('div', { class: 'page page-manual' },
    el('nav', { class: 'toc', 'aria-label': t('manual.toc') },
      el('h1', { class: 'toc-title' }, t('nav.manual')),
      el('ol', {}, MANUAL_TOPICS.map((id) => el('li', {}, el('a', {
        href: `#/manual/${id}`,
        'aria-current': id === current ? 'page' : null,
      }, t(`manual.${id}.title`)))))),
    topicContent(t, current, 2));
}
