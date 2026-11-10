// Start-up: language, theme, engine (vm.js or the mock), router, keyboard,
// live region, optional service worker.
import { createI18n } from './i18n.js';
import { startRouter } from './router.js';
import { createStore } from './state.js';
import { createProgress } from './progress.js';
import { createLevelClient } from './lvlclient.js';
import { actionForKey } from './keys.js';
import { el, mount } from './dom.js';
import { icon, logo } from './icons.js';
import { renderLevelSelect } from './views/levelselect.js';
import { renderWorkspace } from './views/workspace.js';
import { renderManualPage } from './views/manual.js';
import { renderAbout, shortcutTable } from './views/about.js';
import { decodeSource } from './share.js';

const html = document.documentElement;
const lang = html.lang === 'en' ? 'en' : 'de';
const base = new URL('../', import.meta.url);
const params = new URLSearchParams(location.search);
const THEME_KEY = 'stapelwerk:theme';

function safeStorage() {
  try {
    const s = globalThis.localStorage;
    const probe = 'stapelwerk:probe';
    s.setItem(probe, '1');
    s.removeItem(probe);
    return s;
  } catch {
    return null;
  }
}

// The real core (vm.js + WebAssembly) always; the JavaScript mock only with ?mock=1.
async function loadEngine() {
  if (params.has('mock')) {
    const mock = await import('./vm-mock.js');
    return mock.loadVm();
  }
  const mod = await import('./vm.js');
  const vm = await mod.loadVm(new URL('wasm/stapelwerk.wasm', base));
  return vm;
}

function applyTheme(theme) {
  if (theme === 'light' || theme === 'dark') html.dataset.theme = theme;
  else delete html.dataset.theme;
}

function currentTheme() {
  if (html.dataset.theme) return html.dataset.theme;
  return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

async function boot() {
  const storage = safeStorage();
  try { applyTheme(storage?.getItem(THEME_KEY)); } catch { /* ignore */ }
  const dict = (await import(`./texts/${lang}.js`)).default;
  const i18n = createI18n(lang, dict);
  const { t } = i18n;
  const main = document.getElementById('main');
  const live = document.getElementById('live');

  let vm;
  try {
    vm = await loadEngine();
  } catch (err) {
    console.error(err);
    mount(main, el('div', { class: 'page' }, el('h1', { id: 'page-title', tabindex: '-1' }, t('boot.failed')), el('p', {}, t('boot.failedText'))));
    return;
  }
  vm.onInternalError = () => app.announce(t('boot.internal'));

  const progress = createProgress(storage);
  progress.load();
  const store = createStore({ route: null });
  let announceTimer = 0;
  const app = {
    t, lang, i18n, vm, base, progress, store,
    levels: createLevelClient(vm, { base }),
    announce(text) {
      clearTimeout(announceTimer);
      live.textContent = '';
      announceTimer = setTimeout(() => { live.textContent = text; }, 30);
    },
    rerender: () => { if (store.get().route) render(store.get().route, false); },
  };

  // Header texts come from i18n so the English shell needs no extra markup.
  const brand = document.querySelector('.brand');
  if (brand) mount(brand, logo(), el('span', { class: 'brand-name' }, 'Stapelwerk'));
  for (const a of document.querySelectorAll('.mainnav a[data-route]')) a.textContent = t(`nav.${a.dataset.route}`);
  document.querySelector('.mainnav')?.setAttribute('aria-label', t('nav.main'));
  document.querySelector('.skip')?.replaceChildren(t('nav.skip'));
  const langLink = document.querySelector('.lang-link');
  const themeBtn = document.querySelector('.theme-toggle');
  const helpBtn = document.querySelector('.help-toggle');

  function syncTheme() {
    const dark = currentTheme() === 'dark';
    mount(themeBtn, icon(dark ? 'sun' : 'moon'));
    themeBtn.setAttribute('aria-label', dark ? t('theme.toLight') : t('theme.toDark'));
    themeBtn.title = themeBtn.getAttribute('aria-label');
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', dark ? '#0B2340' : '#1D4F91');
  }
  themeBtn?.addEventListener('click', () => {
    const next = currentTheme() === 'dark' ? 'light' : 'dark';
    applyTheme(next);
    try { storage?.setItem(THEME_KEY, next); } catch { /* ignore */ }
    syncTheme();
    // Canvases read colours from CSS; redraw the current view.
    app.rerender();
  });
  syncTheme();

  // Help dialog with the shortcut table.
  const help = el('dialog', { class: 'sheet', id: 'help', 'aria-labelledby': 'help-title' },
    el('h2', { id: 'help-title' }, t('help.title')),
    shortcutTable(t),
    el('p', { class: 'muted' }, t('help.note')),
    el('form', { method: 'dialog', class: 'sheet-actions' }, el('button', { class: 'btn', autofocus: true }, t('common.close'))));
  document.body.append(help);
  if (helpBtn) {
    mount(helpBtn, icon('keyboard'));
    helpBtn.setAttribute('aria-label', t('help.title'));
    helpBtn.title = t('help.title');
    helpBtn.addEventListener('click', () => help.showModal());
  }

  function syncLangLink() {
    if (!langLink) return;
    const target = lang === 'de' ? 'en/' : '../';
    langLink.setAttribute('href', `${target}${location.hash}`);
  }

  if (progress.damaged) app.announce(t('boot.progressReset'));

  let current = null;
  let renderSeq = 0;
  async function render(route, focus = true) {
    const seq = ++renderSeq;
    current?.destroy?.();
    current = null;
    syncLangLink();
    for (const a of document.querySelectorAll('.mainnav a[data-route]')) {
      const on = a.dataset.route === route.name || (route.name === 'level' && a.dataset.route === 'home');
      if (on) a.setAttribute('aria-current', 'page'); else a.removeAttribute('aria-current');
    }
    html.dataset.view = route.name;
    let view = null;
    let title = t('nav.home');
    try {
      if (route.name === 'home') { view = { node: await renderLevelSelect(app) }; title = t('plan.title'); }
      else if (route.name === 'level') {
        const ws = await renderWorkspace(app, { levelId: route.params.id });
        if (ws) { view = ws; title = ws.node.querySelector('.ws-title')?.textContent ?? title; }
      } else if (route.name === 'sandbox') {
        const code = route.query.get('code');
        const initialCode = code ? await decodeSource(code) : null;
        view = await renderWorkspace(app, { sandbox: true, initialCode });
        title = t('sandbox.title');
        if (code && initialCode === null) app.announce(t('sandbox.badLink'));
      } else if (route.name === 'manual') { view = { node: renderManualPage(app, route.params.topic) }; title = t('nav.manual'); }
      else if (route.name === 'about') { view = { node: renderAbout(app) }; title = t('about.title'); }
    } catch (err) {
      console.error(err);
      view = { node: el('div', { class: 'page' }, el('h1', { id: 'page-title', tabindex: '-1' }, t('boot.viewFailed')), el('p', {}, String(err?.message ?? err))) };
    }
    if (seq !== renderSeq) { view?.destroy?.(); return; }
    if (!view) {
      view = { node: el('div', { class: 'page page-notfound' }, el('h1', { id: 'page-title', tabindex: '-1' }, t('notfound.title')), el('p', {}, t('notfound.text')), el('a', { class: 'btn btn-primary', href: '#/' }, t('nav.plan'))) };
      title = t('notfound.title');
    }
    current = view;
    mount(main, view.node);
    document.title = route.name === 'home' ? `Stapelwerk · ${t('meta.tagline')}` : `${title} · Stapelwerk`;
    if (focus) (view.focusTarget ?? view.node.querySelector('#page-title, h1'))?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
    store.set({ route });
  }

  document.addEventListener('keydown', (ev) => {
    if (ev.defaultPrevented || document.querySelector('dialog[open]')) return;
    const target = ev.target;
    const typing = target instanceof HTMLElement && (target.matches('input:not([type=range]):not([type=checkbox]):not([type=radio]), textarea, select') || target.isContentEditable);
    const action = actionForKey(ev, { typing });
    if (!action) return;
    if (action === 'help') { ev.preventDefault(); help.showModal(); return; }
    if (current?.onAction?.(action)) ev.preventDefault();
  });

  let first = true;
  startRouter((route) => { render(route, !first); first = false; });

  const wantSw = params.get('sw') === '1' || !['localhost', '127.0.0.1', '[::1]'].includes(location.hostname);
  if (wantSw && 'serviceWorker' in navigator) {
    navigator.serviceWorker.register(new URL('sw.js', base), { scope: base.pathname }).catch(() => { /* offline support optional */ });
  }
}

boot();
