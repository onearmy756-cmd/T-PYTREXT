/**
 * Usajili wa service worker wa MTECHOS (PWA).
 *
 * Hutumika kwa uzalishaji pekee: kwenye dev, Vite HMR huhitaji mtandao wa
 * moja kwa moja, na SW iliyohifadhiwa huficha mabadiliko. Hupuuza ikiwa
 * kivinjari hakina `serviceWorker` (Safari ya zamani, mazingira ya jaribio).
 */
export function registerServiceWorker(): void {
  if (import.meta.env.DEV) return;
  if (typeof navigator === "undefined" || !("serviceWorker" in navigator)) return;

  const register = () => {
    const base = import.meta.env.VITE_BASE_PATH?.trim().replace(/\/+$/, "") ?? "";
    const url = `${base}/sw.js`;
    navigator.serviceWorker.register(url, { scope: `${base}/` }).catch(() => {
      // Kukosa SW si kosa la kuzuia app — inaendelea bila offline cache.
    });
  };

  if (document.readyState === "complete") register();
  else window.addEventListener("load", register, { once: true });
}
