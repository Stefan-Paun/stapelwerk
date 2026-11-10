// Service worker template. tools/build.mjs fills in the two placeholders below:
//   VERSION   → build version string (a hash over the precached files)
//   PRECACHE  → JSON array of paths relative to this file (every file of dist/ but sw.js)
// Cache-first from a versioned cache that is filled completely on install. Navigations
// ("…/", "…/en/", with or without a query) are answered with the matching index.html, so
// the page and every level work offline. Only caches named "stapelwerk-…" are touched.
const VERSION = '__STAPELWERK_VERSION__';
const PRECACHE = __STAPELWERK_PRECACHE__;
const PREFIX = 'stapelwerk-';
const CACHE = `${PREFIX}${VERSION}`;
const SCOPE = new URL('./', self.location.href);

self.addEventListener('install', (event) => {
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    // cache: 'reload' skips the HTTP cache so a new version never stores stale files.
    await cache.addAll(PRECACHE.map((p) => new Request(new URL(p, SCOPE).href, { cache: 'reload' })));
    await self.skipWaiting();
  })());
});

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    const keys = await caches.keys();
    await Promise.all(keys.filter((k) => k.startsWith(PREFIX) && k !== CACHE).map((k) => caches.delete(k)));
    await self.clients.claim();
  })());
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  const url = new URL(req.url);
  if (req.method !== 'GET' || url.origin !== SCOPE.origin || !url.pathname.startsWith(SCOPE.pathname)) return;
  event.respondWith(respond(req, url));
});

async function respond(req, url) {
  const cache = await caches.open(CACHE);
  const navigate = req.mode === 'navigate';
  const key = navigate && url.pathname.endsWith('/') ? new URL('index.html', url).href : req;
  const hit = await cache.match(key, { ignoreSearch: true });
  if (hit) return hit;
  try {
    return await fetch(req);
  } catch (err) {
    if (navigate) {
      const shell = await cache.match(new URL('index.html', SCOPE).href);
      if (shell) return shell;
    }
    throw err;
  }
}
