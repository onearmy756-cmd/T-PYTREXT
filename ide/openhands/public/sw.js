/*
 * MTECHOS Service Worker (PWA halisi) — Mbilinyi Tech.
 *
 * Mikakati:
 *   - App shell (index.html): network-first, fallback kwa cache (hufanya kazi bila mtandao).
 *   - Vitu tuli (/assets/, picha, font): cache-first + kuhuisha kwa chinichini.
 *   - /api/: network-only KAMWE — data ni ya moja kwa moja (CVE, habari, labs).
 *   - WebSocket: hurukwa kabisa.
 *
 * Scope hutolewa kutoka `self.location` ili base path (/ide) ifanye kazi bila kubadilisha.
 */

const VERSION = "mtechos-v1";
const SHELL_CACHE = `${VERSION}-shell`;
const ASSET_CACHE = `${VERSION}-assets`;

const SCOPE = new URL(self.registration.scope);
const BASE = SCOPE.pathname.replace(/\/$/, ""); // "" au "/ide"
const SHELL_URL = `${BASE}/index.html`;
const PRECACHE = [`${BASE}/`, SHELL_URL, `${BASE}/manifest.webmanifest`, `${BASE}/favicon.svg`];

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(SHELL_CACHE);
      // `addAll` hushindwa yote ikiwa moja haipatikani — weka kila moja peke yake.
      await Promise.all(
        PRECACHE.map((url) =>
          cache.add(new Request(url, { cache: "reload" })).catch(() => undefined),
        ),
      );
      await self.skipWaiting();
    })(),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(
        keys.filter((k) => !k.startsWith(VERSION)).map((k) => caches.delete(k)),
      );
      await self.clients.claim();
    })(),
  );
});

function isApi(url) {
  return url.pathname.startsWith("/api/") || url.pathname.includes("/api/");
}

function isAsset(url) {
  return (
    url.pathname.includes("/assets/") ||
    /\.(?:js|css|woff2?|ttf|png|jpe?g|svg|ico|webp|gif|map)$/i.test(url.pathname)
  );
}

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return; // origin nyingine — acha
  if (isApi(url)) return; // data halisi — usiingilie
  if (req.headers.get("upgrade") === "websocket") return;

  // Urambazaji (navigation) → app shell.
  if (req.mode === "navigate") {
    event.respondWith(
      (async () => {
        try {
          const fresh = await fetch(req);
          const cache = await caches.open(SHELL_CACHE);
          cache.put(SHELL_URL, fresh.clone());
          return fresh;
        } catch {
          const cache = await caches.open(SHELL_CACHE);
          return (
            (await cache.match(SHELL_URL)) ||
            (await cache.match(`${BASE}/`)) ||
            new Response("MTECHOS haipatikani bila mtandao.", {
              status: 503,
              headers: { "Content-Type": "text/plain; charset=utf-8" },
            })
          );
        }
      })(),
    );
    return;
  }

  // Vitu tuli → cache-first + kuhuisha chinichini.
  if (isAsset(url)) {
    event.respondWith(
      (async () => {
        const cache = await caches.open(ASSET_CACHE);
        const hit = await cache.match(req);
        if (hit) {
          event.waitUntil(
            fetch(req)
              .then((res) => (res && res.ok ? cache.put(req, res.clone()) : undefined))
              .catch(() => undefined),
          );
          return hit;
        }
        const res = await fetch(req);
        if (res && res.ok && res.type === "basic") cache.put(req, res.clone());
        return res;
      })(),
    );
  }
});
