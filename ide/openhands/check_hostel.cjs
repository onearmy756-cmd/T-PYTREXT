// E2E: ELIMUPRO Awamu 10 — hosteli (malazi, ugawaji, jinsia).
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');
const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo HS ${stamp}`, code: `H${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CS ${stamp}`, code: 'CS', years: 1, auto_levels: true });
  const course = (await get('/elimupro/courses')).courses.find((c) => c.name === `CS ${stamp}`).id;
  const lv = (await get(`/elimupro/levels?course_id=${course}`)).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Fizikia', code: 'PHY' });
  const inst = (await get('/elimupro/institution')).institution?.id || '';
  const mk = async (n, g) => post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: n, gender: g, allow_unverified: true,
  });
  const m1 = await mk(`Juma Hs ${stamp}`, 'M');
  const f1 = await mk(`Neema Hs ${stamp}`, 'F');
  log('wanafunzi 2 (M,F) wamesajiliwa:', Boolean(m1.id && f1.id));

  const ctx = await b.newContext({ ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 150)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3000);
  await p.click('[data-testid="edu-tab-hosteli"]');
  await p.waitForSelector('[data-testid="hostel-status"]', { timeout: 30000 });
  log('kichupo cha Hosteli kimefunguliwa: true');

  // Ongeza hosteli ya kiume kwa UI
  await p.fill('[data-testid="hostel-name"]', `Kilimanjaro ${stamp}`);
  await p.selectOption('[data-testid="hostel-gender"]', 'male');
  await p.click('[data-testid="hostel-add-btn"]');
  await p.waitForTimeout(1800);
  const hs = (await get('/hostel/list')).hostels;
  const h = hs.find((x) => x.name === `Kilimanjaro ${stamp}`);
  log('hosteli ya kiume imeongezwa:', Boolean(h) && h.gender === 'male');

  // Chagua hosteli mpya kwenye dropdown (orodha hupangwa kwa jina).
  await p.selectOption('[data-testid="hostel-select"]', h.id);
  await p.waitForTimeout(1500);

  // Ongeza chumba cha vitanda 2 kwa UI
  await p.fill('[data-testid="hostel-room-no"]', `A${stamp.slice(-3)}`);
  await p.fill('[data-testid="hostel-room-beds"]', '2');
  await p.click('[data-testid="hostel-room-add"]');
  await p.waitForTimeout(1800);
  const hd = await get(`/hostel/${h.id}`);
  log('chumba cha vitanda 2 kimeongezwa:', hd.rooms.length === 1 && hd.rooms[0].beds === 2);
  const room = hd.rooms[0];
  log('vitanda ni huru:', room.beds_detail.every((b) => b.status === 'free'));

  // Jinsia: mwanafunzi wa kike HAKUBALIWI kwenye hosteli ya kiume
  const wrong = await post('/hostel/allocate', { student_id: f1.id, hostel_id: h.id });
  log('jinsia isiyolingana huzuiwa:', wrong.ok === false && wrong.error === 'jinsia haikubaliwi kwenye hosteli hii');

  // Mwanafunzi wa kiume hukubaliwa (kwa UI)
  await p.selectOption('[data-testid="hostel-level"]', lv.id);
  await p.waitForTimeout(1500);
  await p.click(`[data-testid="hostel-alloc-${m1.id}"]`);
  await p.waitForTimeout(1800);
  const hd2 = await get(`/hostel/${h.id}`);
  log('kitanda kimoja kimechukuliwa:', hd2.rooms[0].occupied === 1);
  log('mwanafunzi ameonekana kwenye kitanda:', hd2.rooms[0].beds_detail.some((b) => b.status === 'occupied'));

  // Ugawaji mara mbili hukataliwa
  const twice = await post('/hostel/allocate', { student_id: m1.id, hostel_id: h.id });
  log('ugawaji mara mbili hukataliwa:', twice.ok === false && twice.error === 'mwanafunzi ana nafasi tayari');

  // Kuondoa hufanya kitanda kiwe huru
  const allocs = (await get(`/hostel/allocations?hostel_id=${h.id}`)).allocations;
  log('ugawaji mmoja unaoendelea:', allocs.length === 1 && allocs[0].active === true);
  const vac = await post('/hostel/vacate', { allocation_id: allocs[0].id });
  log('kuondoa kunafanya kazi:', vac.ok === true);
  const hd3 = await get(`/hostel/${h.id}`);
  log('kitanda kimeachiliwa:', hd3.rooms[0].occupied === 0 && hd3.rooms[0].beds_detail.every((b) => b.status === 'free'));

  // Baada ya kuondoa, anaweza kugawiwa tena
  const again = await post('/hostel/allocate', { student_id: m1.id, hostel_id: h.id });
  log('baada ya kuondoa anaweza kugawiwa tena:', again.ok === true);

  // Chumba kimoja kwa hosteli (namba inarudiwa)
  const dup = await post('/hostel/room/add', { hostel_id: h.id, room_no: `A${stamp.slice(-3)}`, beds: 2 });
  log('chumba kilekile hukataliwa:', dup.ok === false);

  // Uwezo wa hosteli
  const occ = (await get('/hostel/occupancy')).occupancy;
  const mine = occ.find((o) => o.hostel_id === h.id);
  log('uwezo unaonyeshwa (1/2 = 50%):', mine && mine.beds === 2 && mine.occupied === 1 && mine.occupancy_pct === 50);

  // Jinsia huzuiwa hata kwa bed_id maalum (kitanda huru kipo)
  const freshHd = await get(`/hostel/${h.id}`);
  const freeBed = freshHd.rooms[0].beds_detail.find((b) => b.status === 'free');
  const full = await post('/hostel/allocate', { student_id: f1.id, bed_id: freeBed.id });
  log('jinsia huzuiwa hata kwa bed_id maalum:', full.ok === false && full.error === 'jinsia haikubaliwi kwenye hosteli hii');

  await ctx.close(); await ctx0.close(); await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
