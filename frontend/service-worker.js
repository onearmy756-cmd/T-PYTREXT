const CACHE_NAME = 'pytrex-v1';
const CORE = [
  '/',
  'index.html',
  'manifest.webmanifest',
  '../icons/icon.ico'
];

self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(CACHE_NAME).then(cache => cache.addAll(CORE))
  );
  self.skipWaiting();
});

self.addEventListener('activate', event => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', event => {
  event.respondWith(
    caches.match(event.request).then(resp => {
      return resp || fetch(event.request).then(fetchResp => {
        return caches.open(CACHE_NAME).then(cache => {
          try { cache.put(event.request, fetchResp.clone()); } catch (e) { /* opaque responses may fail */ }
          return fetchResp;
        });
      });
    }).catch(() => caches.match('index.html'))
  );
});
