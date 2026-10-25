// E2E: ELIMUPRO Awamu 11 — matokeo, transkripti, vyeti.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');
const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo RS ${stamp}`, code: `R${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CS ${stamp}`, code: 'CS', years: 1, auto_levels: true });
  const course = (await get('/elimupro/courses')).courses.find((c) => c.name === `CS ${stamp}`).id;
  const lv = (await get(`/elimupro/levels?course_id=${course}`)).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Fizikia', code: 'PHY', credits: 3 });
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Kemia', code: 'CHE', credits: 1 });
  const subs = (await get(`/elimupro/subjects?level_id=${lv.id}`)).subjects;
  const phy = subs.find((x) => x.code === 'PHY');
  const che = subs.find((x) => x.code === 'CHE');
  const inst = (await get('/elimupro/institution')).institution?.id || '';
  const st = await post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: `Neema Rs ${stamp}`, gender: 'F', allow_unverified: true,
  });
  log('mwanafunzi amesajiliwa:', Boolean(st.id));

  // Nguzo mbili za alama (kwa somo moja) + moja ya somo la pili.
  const c1 = await post('/marks/column/add', { level_id: lv.id, subject_id: phy.id, kind: 't1', label: 'Zoezi 1', max_marks: 100, weight: 1 });
  const c2 = await post('/marks/column/add', { level_id: lv.id, subject_id: phy.id, kind: 'final', label: 'Mtihani', max_marks: 100, weight: 3 });
  const c3 = await post('/marks/column/add', { level_id: lv.id, subject_id: che.id, kind: 'final', label: 'Mtihani', max_marks: 100, weight: 1 });
  log('nguzo 3 zimeongezwa:', Boolean(c1.id && c2.id && c3.id));

  await post('/marks/set', { column_id: c1.id, student_id: st.id, score: 80 });
  await post('/marks/set', { column_id: c2.id, student_id: st.id, score: 90 });
  await post('/marks/set', { column_id: c3.id, student_id: st.id, score: 65 });
  log('alama zimewekwa: true');

  const ctx = await b.newContext({ ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 150)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3000);
  await p.click('[data-testid="edu-tab-matokeo"]');
  await p.waitForSelector('[data-testid="res-status"]', { timeout: 30000 });
  log('kichupo cha Matokeo kimefunguliwa: true');

  // Chagua level → orodha ya matokeo
  await p.selectOption('[data-testid="res-level"]', lv.id);
  await p.waitForTimeout(2000);
  const ranked = await get(`/results/level?level_id=${lv.id}`);
  log('orodha ya matokeo ina mwanafunzi 1:', ranked.count === 1);
  const row = ranked.results[0];
  log('nafasi ni 1:', row.rank === 1);
  // Fizikia: (80*1 + 90*3)/(1+3) = 350/4 = 87.5 → A(5); Kemia 65 → C(3)
  // GPA = (5*3 + 3*1)/4 = 4.5
  log('GPA = 4.5:', row.gpa === 4.5);
  log('daraja = First Class:', row.classification === 'First Class');
  log('wastani = 76.25:', row.average === 76.25);

  // Onyesha matokeo ya mwanafunzi kwa UI
  await p.click(`[data-testid="res-view-${st.id}"]`);
  await p.waitForSelector('[data-testid="res-student"]', { timeout: 20000 });
  log('matokeo ya mwanafunzi yanaonekana: true');
  const detail = await get(`/results/student?student_id=${st.id}`);
  const phyRow = detail.subjects.find((s) => s.code === 'PHY');
  const cheRow = detail.subjects.find((s) => s.code === 'CHE');
  log('Fizikia = 87.5 (A):', phyRow.score === 87.5 && phyRow.letter === 'A');
  log('Kemia = 65 (C):', cheRow.score === 65 && cheRow.letter === 'C');
  log('masomo 2 yenye alama:', detail.subjects_entered === 2);

  // Toa transkripti kwa UI
  await p.click('[data-testid="res-issue-transcript"]');
  await p.waitForTimeout(2000);
  const tr = await get(`/results/transcript?student_id=${st.id}`);
  log('transkripti imetolewa:', Boolean(tr.issued_transcript));
  log('GPA ya transkripti = 4.5:', tr.issued_transcript?.gpa === 4.5);

  // Toa cheti kwa UI
  await p.click('[data-testid="res-issue-cert"]');
  await p.waitForTimeout(2000);
  const certs = (await get(`/results/certificates?student_id=${st.id}`)).certificates;
  log('cheti kimetolewa:', certs.length === 1);
  const cert = certs[0];
  log('serial ya cheti:', cert.serial);
  log('kodi ya cheti:', cert.verify_code);
  log('umbizo la serial (CERT/YYYY/NNNNNN):', /^CERT\/\d{4}\/\d{6}$/.test(cert.serial));

  // Thibitisha kwa UI
  await p.fill('[data-testid="res-verify-serial"]', cert.serial);
  await p.fill('[data-testid="res-verify-code"]', cert.verify_code);
  await p.click('[data-testid="res-verify-btn"]');
  await p.waitForTimeout(1500);
  const vOk = await p.textContent('[data-testid="res-verify-result"]');
  log('uthibitisho wa UI ni halali:', vOk.includes('Halali'));

  // Kodi mbovu haithibitishi
  const bad = await get(`/results/verify?serial=${cert.serial}&code=XXXXXXXXXX`);
  log('kodi mbovu haithibitishi:', bad.valid === false);

  // Cheti kimoja kwa serial
  const c2nd = await post('/results/certificate/issue', { student_id: st.id, kind: 'award' });
  log('vyeti viwili vina serial tofauti:', c2nd.serial !== cert.serial && c2nd.verify_code !== cert.verify_code);

  // Mwanafunzi hayupo
  const missing = await get('/results/student?student_id=hakuna');
  log('mwanafunzi hayupo hurejesha kosa:', missing.ok === false);

  await ctx.close(); await ctx0.close(); await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
