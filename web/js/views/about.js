// About: what this is, privacy, progress export/import/delete, licences,
// how it was built, keyboard shortcuts.
import { el, mount } from '../dom.js';
import { icon } from '../icons.js';
import { renderMarkup } from '../markup.js';
import { SHORTCUTS } from '../keys.js';

export function shortcutTable(t) {
  return el('table', { class: 'keys-table' },
    el('caption', { class: 'sr-only' }, t('help.title')),
    el('thead', {}, el('tr', {}, el('th', { scope: 'col' }, t('help.key')), el('th', { scope: 'col' }, t('help.action')))),
    el('tbody', {},
      SHORTCUTS.map((s) => el('tr', {}, el('td', {}, el('kbd', {}, s.key.toUpperCase())), el('td', {}, t(`help.${s.action}`)))),
      el('tr', {}, el('td', {}, el('kbd', {}, t('dbg.modEnter'))), el('td', {}, t('help.assembleTest'))),
      el('tr', {}, el('td', {}, el('kbd', {}, 'Esc')), el('td', {}, t('help.leaveEditor'))),
      el('tr', {}, el('td', {}, el('kbd', {}, 'Tab')), el('td', {}, t('help.tab')))));
}

export function renderAbout(app) {
  const { t, progress } = app;
  const status = el('p', { class: 'muted', role: 'status' });
  const fileInput = el('input', {
    type: 'file', accept: 'application/json,.json', class: 'sr-only', id: 'import-file',
    onchange: async (ev) => {
      const file = ev.target.files?.[0];
      if (!file) return;
      if (file.size > 2 * 1024 * 1024) { mount(status, t('about.importTooBig')); return; }
      const res = progress.importJson(await file.text());
      mount(status, res.ok ? t('about.imported') : t('about.importFailed', { error: res.error }));
      ev.target.value = '';
    },
  });
  const confirmDelete = el('dialog', { class: 'sheet', 'aria-labelledby': 'del-title' },
    el('form', { method: 'dialog' },
      el('h2', { id: 'del-title' }, t('about.deleteTitle')),
      el('p', {}, t('about.deleteText')),
      el('div', { class: 'sheet-actions' },
        el('button', { class: 'btn', value: 'cancel', autofocus: true }, t('common.cancel')),
        el('button', { class: 'btn btn-danger', value: 'delete' }, t('about.deleteConfirm')))));
  confirmDelete.addEventListener('close', () => {
    if (confirmDelete.returnValue === 'delete') { progress.clear(); mount(status, t('about.deleted')); }
  });

  const section = (id, ...children) => el('section', { class: 'about-section', 'aria-labelledby': `about-${id}` }, el('h2', { id: `about-${id}` }, t(`about.${id}.title`)), ...children);

  return el('div', { class: 'page page-about prose' },
    el('h1', { id: 'page-title', tabindex: '-1' }, t('about.title')),
    el('p', { class: 'lede' }, t('about.lede')),
    section('what', renderMarkup(t('about.what.body'))),
    section('privacy', renderMarkup(t('about.privacy.body'))),
    section('progress',
      renderMarkup(t('about.progress.body')),
      el('div', { class: 'button-row' },
        el('button', {
          type: 'button', class: 'btn',
          onclick: () => {
            const blob = new Blob([progress.exportJson()], { type: 'application/json' });
            const a = el('a', { href: URL.createObjectURL(blob), download: 'stapelwerk-fortschritt.json' });
            document.body.append(a); a.click(); a.remove();
            setTimeout(() => URL.revokeObjectURL(a.href), 1000);
            mount(status, t('about.exported'));
          },
        }, icon('download'), t('about.export')),
        el('label', { class: 'btn', for: 'import-file' }, icon('upload'), t('about.import')), fileInput,
        el('button', { type: 'button', class: 'btn btn-danger-quiet', onclick: () => confirmDelete.showModal() }, icon('trash'), t('about.delete'))),
      status, confirmDelete),
    section('keys', shortcutTable(t)),
    section('licenses', renderMarkup(t('about.licenses.body')),
      el('ul', {},
        el('li', {}, el('a', { href: new URL('fonts/OFL-SofiaSans.txt', app.base).href }, t('about.fontSofia'))),
        el('li', {}, el('a', { href: new URL('fonts/OFL-IBMPlexMono.txt', app.base).href }, t('about.fontPlex'))))),
    section('ai', renderMarkup(t('about.ai.body'))),
    section('engine', el('p', {}, app.vm.mock ? t('about.engineMock') : t('about.engineReal'))));
}
