// Local progress: solved levels, best metrics, badges, current code and up to
// three solution slots per level. Pure: the storage object (localStorage-like:
// getItem/setItem/removeItem) is passed in, so tests run under node.

export const STORAGE_KEY = 'stapelwerk:progress';
export const SCHEMA_VERSION = 1;
export const SLOT_COUNT = 3;
const MAX_CODE = 65536;
const ID = /^[a-z0-9][a-z0-9-]{0,63}$/;
const TUTORIAL_ITEMS = ['step', 'breakpoint', 'run', 'back'];

const empty = () => ({ v: SCHEMA_VERSION, unlockAll: false, levels: {} });

const isCount = (n) => Number.isInteger(n) && n >= 0 && n <= 0xffffffff;
const cleanCode = (c) => (typeof c === 'string' && c.length <= MAX_CODE ? c : null);

function cleanLevel(raw) {
  if (!raw || typeof raw !== 'object') return null;
  const best = raw.best && isCount(raw.best.cycles) && isCount(raw.best.size)
    ? { cycles: raw.best.cycles, size: raw.best.size } : null;
  const slots = Array.from({ length: SLOT_COUNT }, (_, i) => cleanCode(Array.isArray(raw.slots) ? raw.slots[i] : null));
  const tutorial = {};
  if (raw.tutorial && typeof raw.tutorial === 'object') {
    for (const item of TUTORIAL_ITEMS) if (raw.tutorial[item] === true) tutorial[item] = true;
  }
  return {
    solved: raw.solved === true,
    best,
    badges: { compact: raw.badges?.compact === true, fast: raw.badges?.fast === true },
    code: cleanCode(raw.code),
    slots,
    tutorial,
  };
}

/** Validate untrusted data (storage or import). Returns {ok, data, error}. */
export function validate(raw) {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return { ok: false, error: 'not_object' };
  if (raw.v !== SCHEMA_VERSION) return { ok: false, error: 'version' };
  if (!raw.levels || typeof raw.levels !== 'object' || Array.isArray(raw.levels)) return { ok: false, error: 'levels' };
  const data = empty();
  data.unlockAll = raw.unlockAll === true;
  for (const [id, entry] of Object.entries(raw.levels)) {
    if (!ID.test(id)) continue;
    const clean = cleanLevel(entry);
    if (clean) data.levels[id] = clean;
  }
  return { ok: true, data };
}

function blankLevel() {
  return { solved: false, best: null, badges: { compact: false, fast: false }, code: null, slots: [null, null, null], tutorial: {} };
}

export function createProgress(storage) {
  let data = empty();
  let damaged = false;

  function load() {
    damaged = false;
    let text = null;
    try { text = storage?.getItem(STORAGE_KEY) ?? null; } catch { text = null; }
    if (text === null) { data = empty(); return data; }
    try {
      const result = validate(JSON.parse(text));
      if (result.ok) data = result.data;
      else { data = empty(); damaged = true; }
    } catch {
      data = empty();
      damaged = true;
    }
    return data;
  }

  function save() {
    try { storage?.setItem(STORAGE_KEY, JSON.stringify(data)); return true; } catch { return false; }
  }

  function level(id) {
    return data.levels[id] ?? blankLevel();
  }

  function update(id, fn) {
    if (!ID.test(id)) return level(id);
    const rec = structuredClone(level(id));
    fn(rec);
    data.levels[id] = rec;
    save();
    return rec;
  }

  return {
    load,
    save,
    get data() { return data; },
    get damaged() { return damaged; },
    level,
    saveCode: (id, code) => update(id, (r) => { r.code = cleanCode(code); }),
    saveSlot(id, index, code) {
      if (!Number.isInteger(index) || index < 0 || index >= SLOT_COUNT) return level(id);
      return update(id, (r) => { r.slots[index] = cleanCode(code); });
    },
    markTutorial(id, item) {
      if (!TUTORIAL_ITEMS.includes(item)) return level(id);
      if (level(id).tutorial[item]) return level(id);
      return update(id, (r) => { r.tutorial[item] = true; });
    },
    tutorialDone: (id) => TUTORIAL_ITEMS.every((item) => level(id).tutorial[item] === true),
    /**
     * Record a passing run. Keeps the best cycles and size separately and
     * computes the badges against the reference (null reference → no badges).
     */
    markSolved(id, metrics, reference) {
      return update(id, (r) => {
        r.solved = true;
        const cycles = metrics.cycles_avg;
        const size = metrics.size;
        r.best = r.best
          ? { cycles: Math.min(r.best.cycles, cycles), size: Math.min(r.best.size, size) }
          : { cycles, size };
        if (reference) {
          r.badges.compact = r.badges.compact || size <= reference.size;
          r.badges.fast = r.badges.fast || cycles <= reference.cycles;
        }
      });
    },
    setUnlockAll(on) { data.unlockAll = Boolean(on); save(); },
    /** levels: [{id, chapter, order, kind}] in play order. */
    chapterComplete(levels, chapter) {
      const required = levels.filter((l) => l.chapter === chapter && l.kind !== 'bonus');
      return required.length > 0 && required.every((l) => level(l.id).solved);
    },
    isUnlocked(levels, id) {
      if (data.unlockAll) return true;
      const target = levels.find((l) => l.id === id);
      if (!target) return false;
      for (let c = 0; c < target.chapter; c++) {
        const has = levels.some((l) => l.chapter === c);
        if (has && !this.chapterComplete(levels, c)) return false;
      }
      const inChapter = levels.filter((l) => l.chapter === target.chapter).sort((a, b) => a.order - b.order);
      if (inChapter[0].id === id) return true;
      return level(inChapter[0].id).solved;
    },
    exportJson() {
      return JSON.stringify({ ...data, exported: 'stapelwerk-progress' }, null, 2);
    },
    importJson(text) {
      let parsed;
      try { parsed = JSON.parse(text); } catch { return { ok: false, error: 'json' }; }
      const result = validate(parsed);
      if (!result.ok) return result;
      data = result.data;
      save();
      return { ok: true };
    },
    clear() {
      data = empty();
      try { storage?.removeItem(STORAGE_KEY); } catch { /* storage blocked */ }
    },
  };
}
