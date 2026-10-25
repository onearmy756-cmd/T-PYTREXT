// E2E: ELIMUPRO Awamu 7 — ratiba (timetable) + mahudhurio.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');
const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo AT ${stamp}`, code: `A${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CS ${stamp}`, code: 'CS', years: 1, auto_levels: true });
  const course = (await get('/elimupro/courses')).courses.find((c) => c.name === `CS ${stamp}`).id;
  const lv = (await get(`/elimupro/levels?course_id=${course}`)).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Fizikia', code: 'PHY' });
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Kemia', code: 'CHE' });
  const subs = (await get(`/elimupro/subjects?level_id=${lv.id}`)).subjects;
  const phy = subs.find((s) => s.code === 'PHY');
  const che = subs.find((s) => s.code === 'CHE');

  const inst = (await get('/elimupro/institution')).institution?.id || '';
  const st1 = await post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: `Asha At ${stamp}`, gender: 'F', allow_unverified: true,
  });
  const st2 = await post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: `Bakari At ${stamp}`, gender: 'M', allow_unverified: true,
  });
  log('wanafunzi 2 wamesajiliwa:', Boolean(st1.id && st2.id));

  const ctx = await b.newContext({ ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 150)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3000);
  await p.click('[data-testid="edu-tab-mahudhurio"]');
  await p.waitForSelector('[data-testid="att-status"]', { timeout: 30000 });
  log('kichupo cha Ratiba+Mahudhurio kimefunguliwa: true');

  await p.selectOption('[data-testid="att-level"]', lv.id);
  await p.selectOption('[data-testid="att-subject"]', phy.id);
  await p.selectOption('[data-testid="att-day"]', '1');
  await p.fill('[data-testid="att-start"]', '08:00');
  await p.fill('[data-testid="att-end"]', '10:00');
  await p.fill('[data-testid="att-room"]', `A${stamp.slice(-3)}`);
  await p.click('[data-testid="att-add-period"]');
  await p.waitForTimeout(1500);
  const per1 = (await get(`/attendance/timetable?level_id=${lv.id}`)).periods;
  log('kipindi kimeongezwa (08:00-10:00):', per1.length === 1 && per1[0].start === '08:00' && per1[0].end === '10:00');
  log('muda wa kumaliza ni sahihi (10:00):', per1[0].end_min === 600);

  // Mgongano wa level: somo lingine muda ule ule → hukataliwa.
  const clash = await post('/attendance/timetable/add', {
    level_id: lv.id, subject_id: che.id, day: 1, start: '09:00', end: '11:00', room: `C${stamp.slice(-3)}`,
  });
  log('mgongano wa level huzuiwa:', clash.ok === false && clash.kind === 'level');
  // Muda unaofuata (hakuna mgongano) → hukubaliwa.
  const ok2 = await post('/attendance/timetable/add', {
    level_id: lv.id, subject_id: che.id, day: 1, start: '10:00', end: '12:00', room: `C${stamp.slice(-3)}`,
  });
  log('muda usiogongana hukubaliwa:', ok2.ok === true);
  // Mgongano wa chumba: siku 2, level tupu (kuepuka mgongano wa level),
  // chumba kilekile, muda unaogongana → lazima uzuiwe.
  const room2 = `R${stamp.slice(-3)}`;
  const r1 = await post('/attendance/timetable/add', {
    level_id: lv.id, subject_id: phy.id, day: 2, start: '08:00', end: '09:00', room: room2,
  });
  const rClash = await post('/attendance/timetable/add', {
    subject_id: che.id, day: 2, start: '08:30', end: '09:30', room: room2,
  });
  log('mgongano wa chumba huzuiwa:', r1.ok === true && rClash.ok === false && rClash.kind === 'room');
  // Muda mbovu hukataliwa.
  const bad = await post('/attendance/timetable/add', {
    level_id: lv.id, subject_id: che.id, day: 1, start: '12:00', end: '11:00',
  });
  log('muda mbovu hukataliwa:', bad.ok === false);

  // Fungua kikao kwa kipindi cha kwanza
  const perId = per1[0].id;
  await p.click(`[data-testid="att-open-${perId}"]`);
  await p.waitForSelector('[data-testid="att-session"]', { timeout: 20000 });
  const code = await p.innerText('[data-testid="att-code"]');
  log('kikao kimefunguliwa na kodi:', code.includes('kodi'));
  const codeVal = code.replace('kodi ', '').trim();

  const sess = (await get(`/attendance/sessions?level_id=${lv.id}`)).sessions[0];
  log('wanafunzi wote wamejazwa:', sess.total === 2);
  log('mwanzo wote ni absent:', sess.present === 0);

  // Weka mahudhurio kwa UI
  await p.click(`[data-testid="att-set-${st1.id}-present"]`);
  await p.waitForTimeout(800);
  await p.click(`[data-testid="att-set-${st2.id}-late"]`);
  await p.waitForTimeout(1200);
  const pct = await p.innerText('[data-testid="att-percent"]');
  // 1 present + 1 late = (1+0.5)/2 = 75%
  log('asilimia (present+late=75%):', pct.includes('75'));

  // Mwanafunzi hujisajili kwa kodi (check-in)
  await post('/attendance/mark', { session_id: sess.id, student_id: st1.id, status: 'absent' });
  const ci = await post('/attendance/checkin', { code: codeVal, student_id: st1.id });
  log('kujisajili kwa kodi hufanya kazi:', ci.ok === true && ci.status === 'present');
  const ciBad = await post('/attendance/checkin', { code: 'ZZZZZZ', student_id: st1.id });
  log('kodi mbovu hukataliwa:', ciBad.ok === false);

  // Kufunga kikao huzuia mabadiliko
  await post('/attendance/session/close', { id: sess.id });
  const locked = await post('/attendance/mark', { session_id: sess.id, student_id: st1.id, status: 'absent' });
  log('kikao kilichofungwa hakibadiliki:', locked.ok === false);

  // Muhtasari wa mwanafunzi
  const sum = await get(`/attendance/summary/student/${st2.id}`);
  log('muhtasari wa mwanafunzi (late=50%):', sum.ok && sum.overall === 50);

  // Muhtasari wa level
  const lvlSum = await get(`/attendance/summary/level?level_id=${lv.id}`);
  log('muhtasari wa level una wanafunzi 2:', lvlSum.students.length === 2);

  await ctx.close(); await ctx0.close(); await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
