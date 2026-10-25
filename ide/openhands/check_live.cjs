// E2E: ELIMUPRO Awamu 3 & 4 — madarasa ya moja kwa moja + uwasilishaji.
// Huthibitisha WebRTC HALISI: washiriki wawili kwenye kikao kimoja, mesh
// connection inafunguka, sauti inapita, transcript/slides/alama zinafanya kazi.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');

const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

const ARGS = [
  '--no-sandbox',
  '--disable-dev-shm-usage',
  '--disable-gpu',
  '--ignore-certificate-errors',
  '--use-fake-device-for-media-stream',
  '--use-fake-ui-for-media-stream',
  '--autoplay-policy=no-user-gesture-required',
  '--disable-features=WebRtcHideLocalIpsWithMdns',
];

async function newPeer(browser, tag) {
  const ctx = await browser.newContext({
    viewport: { width: 1400, height: 1200 },
    ignoreHTTPSErrors: true,
    permissions: ['microphone'],
  });
  await ctx.addInitScript(() => {
    localStorage.setItem('mbilinyi-onboarded', '1');
    localStorage.setItem('mbilinyi-onboarding-started', '1');
  });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log(`PAGE-ERR[${tag}]:`, String(e).slice(0, 160)));
  return { ctx, p };
}

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ARGS });

  // ---- Maandalizi: taasisi → course → level → somo ----
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (path, body) =>
    (await ctx0.request.post(`${BASE}/api${path}`, { data: body })).json();
  const get = async (path) => (await ctx0.request.get(`${BASE}/api${path}`)).json();
  await post('/elimupro/institution/save', { name: `Chuo LV ${stamp}`, code: `L${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CIS ${stamp}`, code: 'CIS', years: 2, auto_levels: true });
  const lv = (await get('/elimupro/levels')).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Algoriti', code: 'ALG' });
  const subj = (await get(`/elimupro/subjects?level_id=${lv.id}`)).subjects[0];

  const A = await newPeer(b, 'A');
  const B = await newPeer(b, 'B');

  // ---- Kikao cha "live" ----
  await A.p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await A.p.waitForTimeout(3500);
  await A.p.click('[data-testid="edu-tab-live"]');
  await A.p.waitForSelector('[data-testid="live-status"]', { timeout: 30000 });
  log('kichupo cha Live kimefunguliwa: true');

  await A.p.selectOption('[data-testid="lv-kind"]', 'live');
  await A.p.fill('[data-testid="lv-title"]', `Somo la Live ${stamp}`);
  await A.p.click('[data-testid="lv-create"]');
  await A.p.waitForTimeout(2500);
  const sessions = (await get('/live/sessions?kind=live')).sessions;
  const sess = sessions.find((s) => s.title.includes(`Somo la Live ${stamp}`));
  log('kikao kimeundwa kwenye UI:', Boolean(sess));

  // A: ingia kama mwalimu
  await A.p.click(`[data-testid="live-item-${sess.id}"]`);
  await A.p.waitForSelector('[data-testid="live-join"]', { timeout: 20000 });
  await A.p.fill('[data-testid="live-name"]', `Mwalimu ${stamp}`);
  await A.p.click('[data-testid="live-join"]');
  await A.p.waitForSelector('[data-testid="live-peers"]', { timeout: 20000 });
  await A.p.click('[data-testid="live-start"]');
  await A.p.waitForTimeout(1200);

  // B: mshiriki wa pili (browser tofauti) — hujiunga kwa HTTP halisi
  await B.p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await B.p.waitForTimeout(3500);
  await B.p.click('[data-testid="edu-tab-live"]');
  await B.p.waitForSelector(`[data-testid="live-item-${sess.id}"]`, { timeout: 30000 });
  await B.p.click(`[data-testid="live-item-${sess.id}"]`);
  await B.p.waitForSelector('[data-testid="live-join"]', { timeout: 20000 });
  await B.p.fill('[data-testid="live-name"]', `Neema ${stamp}`);
  await B.p.click('[data-testid="live-join"]');
  await B.p.waitForSelector('[data-testid="live-peers"]', { timeout: 20000 });

  // Subiri mesh WebRTC iunganishe (ICE + offer/answer kwa WS)
  const connectedIn = async (page) => {
    for (let i = 0; i < 30; i += 1) {
      const txt = await page.innerText('[data-testid="live-peers"]').catch(() => '');
      if (txt.includes('imeunganishwa')) return true;
      await page.waitForTimeout(1000);
    }
    return false;
  };
  const aOk = await connectedIn(A.p);
  const bOk = await connectedIn(B.p);
  log('A: WebRTC mesh imeunganishwa:', aOk);
  log('B: WebRTC mesh imeunganishwa:', bOk);

  // Sauti halisi: kipengele cha <audio> chenye stream kutoka peer
  const audioA = await A.p.evaluate(() => {
    const els = Array.from(document.querySelectorAll('audio'));
    return els.map((e) => Boolean(e.srcObject)).filter(Boolean).length;
  });
  log('A: sauti ya peer imepokelewa (<audio srcObject>):', audioA > 0);

  // AI Scribe: transcript huingia kwa wote kwa WS
  await post(`/live/session/${sess.id}/transcript`, {
    speaker: 'Mwalimu',
    text: `Loop ni muundo wa kurudia ${stamp}`,
  });
  await A.p.waitForTimeout(2500);
  const trA = await A.p.innerText('[data-testid="live-transcript"]');
  log('transcript imefika A kwa WS:', trA.includes(`Loop ni muundo wa kurudia ${stamp}`));

  // Muhtasari wa AI (LLM halisi)
  await A.p.click('[data-testid="live-summary"]');
  await A.p.waitForTimeout(8000);
  const sumA = await A.p.innerText('[data-testid="live-summary-out"]').catch(() => '');
  log('muhtasari wa AI umezalishwa:', sumA.length > 10);

  // ---- Kikao cha uwasilishaji (Awamu 4) ----
  await A.p.click('[data-testid="live-back"]');
  await A.p.waitForTimeout(2000);
  await A.p.selectOption('[data-testid="lv-kind"]', 'presentation');
  await A.p.fill('[data-testid="lv-title"]', `Uwasilishaji ${stamp}`);
  await A.p.click('[data-testid="lv-create"]');
  await A.p.waitForTimeout(2500);
  const pres = (await get('/live/sessions?kind=presentation')).sessions.find((s) =>
    s.title.includes(`Uwasilishaji ${stamp}`),
  );
  await A.p.click(`[data-testid="live-item-${pres.id}"]`);
  await A.p.waitForSelector('[data-testid="live-join"]', { timeout: 20000 });
  await A.p.fill('[data-testid="live-name"]', `Mwana ${stamp}`);
  await A.p.click('[data-testid="live-join"]');
  await A.p.waitForSelector('[data-testid="live-slides"]', { timeout: 20000 });

  // Ongeza slides 2 (PNG ndogo halisi) na uendeshe
  await post(`/live/session/${pres.id}/slide`, {
    title: 'slide1',
    data: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
  });
  await post(`/live/session/${pres.id}/slide`, {
    title: 'slide2',
    data: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  });
  await A.p.reload({ waitUntil: 'networkidle' });
  await A.p.waitForTimeout(3000);
  await A.p.click('[data-testid="edu-tab-live"]');
  await A.p.click(`[data-testid="live-item-${pres.id}"]`);
  await A.p.waitForSelector('[data-testid="live-join"]', { timeout: 20000 });
  await A.p.fill('[data-testid="live-name"]', `Mwana ${stamp}`);
  await A.p.click('[data-testid="live-join"]');
  await A.p.waitForSelector('[data-testid="live-slide-img"]', { timeout: 25000 });
  log('slides zinaonekana:', true);
  await A.p.click('[data-testid="live-slide-2"]');
  await A.p.waitForTimeout(1500);
  const cur = (await get(`/live/session/${pres.id}`)).session.current_slide;
  log('slide ya sasa imebadilika:', cur === 2);

  // Alama za uwasilishaji
  await post(`/live/session/${pres.id}/mark`, {
    student_id: 'st-demo',
    score: 9,
    feedback: 'Uwasilishaji mzuri',
  });
  const marks = (await get(`/live/session/${pres.id}/marks`)).marks;
  log('alama za uwasilishaji:', marks.length === 1 && marks[0].score === 9);

  await ctx0.close();
  await b.close();
  log('DONE');
})().catch((e) => {
  console.error('E2E FAILED:', e.message);
  process.exit(1);
});
