// E2E: ELIMUPRO Awamu 6 — alama za mtindo wa Excel.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');
const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo MK ${stamp}`, code: `M${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CS ${stamp}`, code: 'CS', years: 1, auto_levels: true });
  const lv = (await get('/elimupro/levels')).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Hisabati', code: 'MATH' });
  const subj = (await get(`/elimupro/subjects?level_id=${lv.id}`)).subjects[0];
  const inst = (await get('/elimupro/institution')).institution?.id || '';
  const course = (await get('/elimupro/courses')).courses[0].id;
  const st = await post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: `Juma Mk ${stamp}`, gender: 'M', allow_unverified: true,
  });
  const sid = st.id;
  log('mwanafunzi amesajiliwa:', Boolean(sid));
  log('masomo yamesajiliwa kiotomatiki:', st.enrolled_subjects >= 1);

  const ctx = await b.newContext({ ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 150)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3000);
  await p.click('[data-testid="edu-tab-alama"]');
  await p.waitForSelector('[data-testid="marks-status"]', { timeout: 30000 });
  log('kichupo cha Alama kimefunguliwa: true');

  await p.selectOption('[data-testid="marks-level"]', lv.id);
  await p.selectOption('[data-testid="marks-subject"]', subj.id);
  await p.waitForTimeout(1500);

  // Ongeza nguzo T1 na Final
  await p.selectOption('[data-testid="marks-kind"]', 't1');
  await p.fill('[data-testid="marks-max"]', '100');
  await p.fill('[data-testid="marks-weight"]', '1');
  await p.click('[data-testid="marks-add-column"]');
  await p.waitForTimeout(1200);
  await p.selectOption('[data-testid="marks-kind"]', 'final');
  await p.fill('[data-testid="marks-weight"]', '2');
  await p.click('[data-testid="marks-add-column"]');
  await p.waitForTimeout(1500);
  const cols = (await get(`/marks/columns?level_id=${lv.id}&subject_id=${subj.id}`)).columns;
  log('nguzo 2 zimeongezwa (T1, Final):', cols.length === 2 && cols[0].label === 'T1' && cols[1].label === 'Final');
  log('uzito wa Final = 2:', cols[1].weight === 2);

  const t1 = cols.find((c) => c.kind === 't1');
  const fin = cols.find((c) => c.kind === 'final');
  await p.waitForSelector(`[data-testid="marks-cell-${sid}-${t1.id}"]`, { timeout: 20000 });
  await p.fill(`[data-testid="marks-cell-${sid}-${t1.id}"]`, '60');
  await p.fill(`[data-testid="marks-cell-${sid}-${fin.id}"]`, '90');
  await p.click('[data-testid="marks-save"]');
  await p.waitForTimeout(2500);

  const gb = await get(`/marks/gradebook?level_id=${lv.id}&subject_id=${subj.id}`);
  const row = gb.rows.find((r) => r.student_id === sid);
  log('alama zimehifadhiwa:', row && row.marks.length === 2);
  log('wastani kwa uzito (60+90*2)/3 = 80%:', row && Math.abs(row.percent - 80) < 0.01);
  log('daraja = A:', row && row.grade === 'A');

  // Alama isiyozidi max
  const over = await post('/marks/set', { column_id: t1.id, student_id: sid, score: 999 });
  log('alama hukatwa kwenye max (100):', over.score === 100);
  await post('/marks/set', { column_id: t1.id, student_id: sid, score: 60 });

  // Export CSV
  const csv = await (await ctx0.request.get(`${BASE}/api/marks/export.csv?level_id=${lv.id}&subject_id=${subj.id}`)).text();
  const lines = csv.trim().split('\n');
  log('CSV ina kichwa + safu:', lines.length >= 2 && lines[0].includes('Reg No'));
  log('CSV ina nguzo T1 na Final:', lines[0].includes('T1') && lines[0].includes('Final'));
  log('CSV ina daraja:', lines[1].includes('A'));
  log('CSV header:', JSON.stringify(lines[0]));

  // Ripoti ya mwanafunzi
  const rep = await get(`/marks/student/${sid}`);
  log('ripoti ya mwanafunzi ina somo:', rep.ok && rep.subjects.length === 1 && rep.subjects[0].grade === 'A');

  await ctx.close(); await ctx0.close(); await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
