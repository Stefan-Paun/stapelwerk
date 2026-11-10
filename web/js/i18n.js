// Flat-key translations with {placeholder} substitution.
// Pure module: works in the browser and under node --test.

const PLACEHOLDER = /\{([a-zA-Z0-9_]+)\}/g;

/** List the placeholder names used in a template, sorted and unique. */
export function placeholders(template) {
  const names = new Set();
  for (const m of String(template).matchAll(PLACEHOLDER)) names.add(m[1]);
  return [...names].sort();
}

/** Fill {name} placeholders; unknown ones stay visible so mistakes are noticed. */
export function format(template, params) {
  if (!params) return template;
  return String(template).replace(PLACEHOLDER, (all, name) =>
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : all);
}

/**
 * Create a translator. `dict` is a flat object of key → template.
 * Missing keys return the key itself so they show up in screenshots and tests.
 */
export function createI18n(lang, dict, fallback = null) {
  const numberFormat = new Intl.NumberFormat(lang === 'en' ? 'en-GB' : 'de-AT');
  function has(key) {
    return Object.prototype.hasOwnProperty.call(dict, key);
  }
  function t(key, params) {
    let template = has(key) ? dict[key] : fallback && Object.prototype.hasOwnProperty.call(fallback, key) ? fallback[key] : key;
    if (params && typeof params.count === 'number') {
      const pluralKey = `${key}.${params.count === 1 ? 'one' : 'other'}`;
      if (has(pluralKey)) template = dict[pluralKey];
    }
    return format(template, params);
  }
  return {
    lang,
    t,
    has,
    num: (n) => numberFormat.format(n),
  };
}

/** Pick 'de' or 'en' text from an object like {de: '…', en: '…'}. */
export function pick(lang, value) {
  if (value === null || value === undefined) return '';
  if (typeof value === 'string') return value;
  return value[lang] ?? value.de ?? value.en ?? '';
}
