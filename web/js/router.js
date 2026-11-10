// Hash routes, language neutral:
//   #/                 level select
//   #/level/<id>       workspace
//   #/manual[/<topic>] manual
//   #/sandbox[?code=…] sandbox
//   #/about            about, privacy, progress
// Pure parsing and formatting; startRouter() is the only DOM-touching part.

const ID = /^[a-z0-9][a-z0-9-]{0,63}$/;

function splitQuery(rest) {
  const q = rest.indexOf('?');
  if (q < 0) return [rest, new URLSearchParams()];
  return [rest.slice(0, q), new URLSearchParams(rest.slice(q + 1))];
}

/** Parse a location hash into {name, params, query}. */
export function parseHash(hash) {
  let rest = String(hash ?? '');
  if (rest.startsWith('#')) rest = rest.slice(1);
  if (rest === '' || rest === '/') return { name: 'home', params: {}, query: new URLSearchParams() };
  const [path, query] = splitQuery(rest);
  const parts = path.split('/').filter(Boolean).map((p) => {
    try { return decodeURIComponent(p); } catch { return '\u0000'; }
  });
  const [head, arg, extra] = parts;
  if (head === 'level' && arg && ID.test(arg) && extra === undefined) return { name: 'level', params: { id: arg }, query };
  if (head === 'manual' && extra === undefined) {
    if (!arg) return { name: 'manual', params: { topic: null }, query };
    if (ID.test(arg)) return { name: 'manual', params: { topic: arg }, query };
  }
  if (head === 'sandbox' && arg === undefined) return { name: 'sandbox', params: {}, query };
  if (head === 'about' && arg === undefined) return { name: 'about', params: {}, query };
  return { name: 'notfound', params: { path }, query };
}

/** Build a hash for a route. */
export function formatRoute(name, params = {}, query = null) {
  let path;
  switch (name) {
    case 'home': path = '#/'; break;
    case 'level': path = `#/level/${encodeURIComponent(params.id)}`; break;
    case 'manual': path = params.topic ? `#/manual/${encodeURIComponent(params.topic)}` : '#/manual'; break;
    case 'sandbox': path = '#/sandbox'; break;
    case 'about': path = '#/about'; break;
    default: throw new Error(`unknown route ${name}`);
  }
  const qs = query ? new URLSearchParams(query).toString() : '';
  return qs ? `${path}?${qs}` : path;
}

/** Listen for hash changes; calls onRoute(route) now and on every change. */
export function startRouter(onRoute, win = globalThis.window) {
  const handler = () => onRoute(parseHash(win.location.hash));
  win.addEventListener('hashchange', handler);
  handler();
  return () => win.removeEventListener('hashchange', handler);
}
