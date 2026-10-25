// E2E: ELIMUPRO Awamu 5 — mitihani + ulinzi mkali (proctoring).
// Huthibitisha: maswali, jibu hufungwa (haliwezi kufutwa), MCQ hupimwa,
// copy/paste/devtools/focus huzuiwa NA kuripotiwa, camera/mic huwashwa,
// muhtasari wa matokeo.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');

const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

const ARGS = [
  '--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu',
  '--ignore-certificate-errors', '--use-fake-device-for-media-stream',
  '--use-fake-ui-for-media-stream', '--autoplay-policy=no-user-gesture-required',
];

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ARGS });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo EX ${stamp}`, code: `E${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CIS ${stamp}`, code: 'CIS', years: 1, auto_levels: true });
  const lv = (await get('/elimupro/levels')).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Algoriti', code: 'ALG' });
  const subj = (await get(`/elimupro/subjects?level_id=${lv.id}`)).subjects[0];

  const ctx = await b.newContext({ ignoreHTTPSErrors: true, permissions: ['camera', 'microphone'] });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 180)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3500);
  await p.click('[data-testid="edu-tab-mitihani"]');
  await p.waitForSelector('[data-testid="exam-status"]', { timeout: 30000 });
  log('kichupo cha Mitihani kimefunguliwa: true');

  await p.fill('[data-testid="exam-title"]', `Mtihani wa Ulinzi ${stamp}`);
  await p.fill('[data-testid="exam-minutes"]', '30');
  await p.click('[data-testid="exam-create"]');
  await p.waitForTimeout(2500);
  const exam = (await get('/proctor/exams')).exams.find((e) => e.title.includes(`Mtihani wa Ulinzi ${stamp}`));
  log('mtihani umeundwa:', Boolean(exam));
  await p.click(`[data-testid="exam-item-${exam.id}"]`);
  await p.waitForSelector('[data-testid="exam-detail"]', { timeout: 20000 });

  // Swali la MCQ
  await p.selectOption('[data-testid="exam-q-kind"]', 'mcq');
  await p.fill('[data-testid="exam-q-options"]', 'O(log n), O(n), O(n^2), O(1)');
  await p.fill('[data-testid="exam-q-key"]', 'A');
  await p.fill('[data-testid="exam-q-marks"]', '4');
  await p.fill('[data-testid="exam-q-prompt"]', 'Ugumu wa binary search ni upi?');
  await p.click('[data-testid="exam-q-add"]');
  await p.waitForTimeout(1500);
  // Swali la maandishi
  await p.selectOption('[data-testid="exam-q-kind"]', 'text');
  await p.fill('[data-testid="exam-q-marks"]', '6');
  await p.fill('[data-testid="exam-q-prompt"]', 'Eleza jinsi loop inavyofanya kazi.');
  await p.click('[data-testid="exam-q-add"]');
  await p.waitForTimeout(1500);
  const qs = (await get(`/proctor/exam/${exam.id}/questions?with_key=true`)).questions;
  log('maswali 2 yameongezwa:', qs.length === 2);
  log('ufunguo wa jibu unaonekana kwa mwalimu:', qs[0].answer_key === 'A');

  await p.click('[data-testid="exam-publish"]');
  await p.waitForTimeout(1500);
  log('mtihani umechapishwa:', (await get(`/proctor/exam/${exam.id}`)).exam.status === 'published');

  // Mwanafunzi anaanza — ulinzi huwashwa
  await p.fill('[data-testid="exam-student"]', `st-${stamp}`);
  await p.click('[data-testid="exam-start"]');
  await p.waitForSelector('[data-testid="exam-sitting"]', { timeout: 25000 });
  log('ulinzi umeanza (proctor bar):', true);
  await p.waitForTimeout(3000);
  const cam = await p.innerText('[data-testid="proctor-bar"]');
  log('kamera/mic zimewashwa:', cam.includes('kamera ON') && cam.includes('mic ON'));

  // Jibu MCQ
  await p.fill('[data-testid="exam-answer"]', 'A');
  await p.click('[data-testid="exam-lock"]');
  await p.waitForTimeout(1200);
  const locked1 = await p.locator('[data-testid="exam-locked"]').count();
  log('jibu la MCQ limefungwa:', locked1 > 0);

  // Swali la maandishi: copy huzuiwa + huripotiwa
  await p.click('[data-testid="exam-q-2"]');
  await p.waitForSelector('[data-testid="exam-answer"]', { timeout: 10000 });
  await p.fill('[data-testid="exam-answer"]', 'Loop hurudia maelekezo mpaka sharti litimizwe, kisha hukoma.');
  const before = await p.innerText('[data-testid="proctor-violations"]');

  // Jaribu copy (huzuiwa + kuripotiwa)
  await p.evaluate(() => {
    const ev = new ClipboardEvent('copy', { bubbles: true, cancelable: true });
    document.dispatchEvent(ev);
  });
  // Jaribu paste
  await p.evaluate(() => {
    const ev = new ClipboardEvent('paste', { bubbles: true, cancelable: true });
    document.dispatchEvent(ev);
  });
  // Jaribu F12 (devtools)
  await p.keyboard.press('F12');
  await p.waitForTimeout(2500);
  const after = await p.innerText('[data-testid="proctor-violations"]');
  log(`ukiukaji umeongezeka (${before} -> ${after}):`, before !== after);

  await p.click('[data-testid="exam-lock"]');
  await p.waitForTimeout(1200);

  // Jaribu kufuta jibu lililofungwa (haipaswi kubadilika)
  const ansBefore = (await get(`/proctor/attempt/${(await get(`/proctor/exam/${exam.id}/attempts`)).attempts[0].id}`)).answers
    .find((a) => a.body.includes('Loop'));
  const aid = (await get(`/proctor/exam/${exam.id}/attempts`)).attempts[0].id;
  const overwrite = await post(`/proctor/attempt/${aid}/answer`, { question_id: ansBefore.question_id, body: 'jibu jipya' });
  log('jibu lililofungwa haliwezi kubadilishwa:', overwrite.ok === false && overwrite.locked === true);

  // Wasilisha
  await p.click('[data-testid="exam-submit"]');
  await p.waitForSelector('[data-testid="exam-result"]', { timeout: 30000 });
  const res = await p.innerText('[data-testid="exam-result"]');
  log('matokeo yameonekana:', res.includes('Matokeo'));
  const attempt = (await get(`/proctor/attempt/${aid}`)).attempt;
  const mcq = (await get(`/proctor/attempt/${aid}`)).answers.find((a) => a.body === 'A');
  log('MCQ imepimwa kiotomatiki (4/4):', mcq && mcq.auto_score === 4);
  log('jumla ya alama (MCQ 4 + maandishi yaliyopimwa):', attempt.score >= 4 && attempt.grade_mode === 'llm');

  // Matukio ya ulinzi yamerekodiwa
  const evs = (await get(`/proctor/attempt/${aid}/events`)).events;
  const kinds = [...new Set(evs.map((e) => e.kind))];
  log('matukio ya ulinzi:', JSON.stringify(kinds));
  log('copy/paste/devtools yameripotiwa:', kinds.includes('copy') && kinds.includes('paste') && kinds.includes('devtools'));
  log('camera ilipiga picha:', kinds.includes('photo'));

  await ctx.close();
  await ctx0.close();
  await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
