// Level list and level sections through the VM. Views only see the normalised
// Level object below, never the VM's level handles.
//
// Real build: levels/index.json (written by `stapelwerk pack-levels`) lists
// {id, chapter, order, kind, title, card, mobile}; each level is fetched as
// levels/<id>.lvl and parsed by the core (vm.loadLevel(text)).
// Mock: the VM carries demo data (vm.levelIndex(), vm.loadLevel(id)).

import { pick } from './i18n.js';

function splitHints(text) {
  if (!text) return [];
  return text.split(/\n---\n/).map((h) => h.trim()).filter(Boolean);
}

const regObject = (list) => (Array.isArray(list) && list.length ? Object.fromEntries(list.map((r) => [r.reg, r.value])) : null);

/**
 * Test list indexed by test number. The core reports visible tests with their
 * details and only counts the hidden ones; hidden slots become placeholders.
 */
function normaliseTests(info) {
  if (Array.isArray(info.testsAll)) return info.testsAll.map((t, i) => ({ index: i, ...t }));
  const visible = Array.isArray(info.tests) ? info.tests.filter((t) => t && typeof t === 'object') : [];
  const total = Math.max(visible.length + (info.hidden_tests ?? 0), ...visible.map((t) => t.index + 1));
  const tests = Array.from({ length: total }, (_, i) => ({ index: i, name: `#${i + 1}`, hidden: true }));
  for (const t of visible) {
    tests[t.index] = {
      index: t.index,
      name: t.name,
      hidden: false,
      in: t.in ?? [],
      in_text: t.in_text ?? '',
      out: t.out ?? null,
      out_text: t.out_text ?? null,
      call: t.call ?? null,
      regs: regObject(t.regs),
      expect_reg: regObject(t.expect_reg),
      screen: t.expect_screen != null,
      screen_init: t.screen_init ?? null,
      heap_empty: Boolean(t.heap_empty),
    };
  }
  return tests;
}

function normalise(handle, entry) {
  const info = handle.info();
  return {
    handle,
    id: info.id,
    chapter: info.chapter ?? entry?.chapter ?? 0,
    order: info.order ?? entry?.order ?? 1,
    kind: info.kind ?? 'normal',
    tags: info.tags ?? entry?.tags ?? [],
    mobile: Boolean(info.mobile ?? entry?.mobile),
    title: info.title ?? entry?.title ?? { de: info.id, en: info.id },
    card: info.card ?? entry?.card ?? { de: '', en: '' },
    budget: info.budget ?? 200000,
    tests: normaliseTests(info),
    reference: info.ref_cycles != null && info.ref_size != null ? { cycles: info.ref_cycles, size: info.ref_size } : null,
    text: (lang) => handle.section(`text.${lang}`) ?? '',
    hints: (lang) => splitHints(handle.section(`hint.${lang}`)),
    learned: (lang) => handle.section(`learned.${lang}`) ?? '',
    start: handle.section('start') ?? '',
    image: (name) => handle.image?.(name) ?? null,
  };
}

export function createLevelClient(vm, { base, fetchFn = globalThis.fetch } = {}) {
  let indexPromise = null;
  const cache = new Map();

  async function index() {
    if (!indexPromise) {
      indexPromise = (async () => {
        if (vm.mock) return vm.levelIndex();
        const res = await fetchFn(new URL('levels/index.json', base));
        if (!res.ok) throw new Error(`levels/index.json: ${res.status}`);
        const data = await res.json();
        const levels = Array.isArray(data) ? data : data.levels;
        return { levels };
      })();
    }
    const data = await indexPromise;
    const levels = [...data.levels].sort((a, b) => a.chapter - b.chapter || a.order - b.order);
    return { levels };
  }

  // One promise per id: two overlapping load(id) calls share a single level handle
  // instead of loading it twice and leaking one. A failed load is forgotten so it can be retried.
  function load(id) {
    if (!cache.has(id)) {
      const pending = loadUncached(id).then((level) => {
        if (!level) cache.delete(id);
        return level;
      }, (err) => {
        cache.delete(id);
        throw err;
      });
      cache.set(id, pending);
    }
    return cache.get(id);
  }

  async function loadUncached(id) {
    const { levels } = await index();
    const entry = levels.find((l) => l.id === id);
    if (!entry) return null;
    let handle;
    if (vm.mock) handle = vm.loadLevel(id);
    else {
      const res = await fetchFn(new URL(`levels/${encodeURIComponent(id)}.lvl`, base));
      if (!res.ok) return null;
      handle = vm.loadLevel(await res.text());
    }
    if (!handle) return null;
    return normalise(handle, entry);
  }

  return { index, load };
}

export const levelTitle = (lang, level) => pick(lang, level.title);
export const levelNumber = (level) => `${level.chapter}.${level.order}`;
