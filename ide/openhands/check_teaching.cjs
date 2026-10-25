// E2E: ELIMUPRO Awamu 2 — notes → kazi + AI-check → vikundi + chat (live) →
// matangazo. Hupitia UI halisi (kichupo "Ufundishaji").
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');

const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({
    executablePath: '/usr/bin/chromium',
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--ignore-certificate-errors'],
  });
  const ctx = await b.newContext({ viewport: { width: 1400, height: 1200 }, ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => {
    localStorage.setItem('mbilinyi-onboarded', '1');
    localStorage.setItem('mbilinyi-onboarding-started', '1');
  });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 200)));

  // ---- 0. Maandalizi: taasisi → course → level → somo (Awamu 1 endpoints) ----
  const post = async (path, body) =>
    (await ctx.request.post(`${BASE}/api${path}`, { data: body })).json();
  const get = async (path) => (await ctx.request.get(`${BASE}/api${path}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo TP ${stamp}`, code: `T${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CIS ${stamp}`, code: 'CIS', years: 2, auto_levels: true });
  const lv = (await get('/elimupro/levels')).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Programming', code: 'PRG' });
  const subj = (await get(`/elimupro/subjects?level_id=${lv.id}`)).subjects[0];
  log('maandalizi: level+subject', Boolean(lv && subj));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(4000);
  await p.click('[data-testid="edu-tab-ufundishaji"]');
  await p.waitForSelector('[data-testid="teaching-status"]', { timeout: 30000 });
  log('kichupo cha Ufundishaji kimefunguliwa: true');

  // ---- 1. NOTES ----
  await p.waitForSelector('[data-testid="tp-note-title"]', { timeout: 20000 });
  await p.selectOption('[data-testid="tp-note-level"]', { label: lv.name }).catch(() => {});
  await p.fill('[data-testid="tp-note-title"]', `Notes za C ${stamp}`);
  await p.fill('[data-testid="tp-note-body"]', 'Algoriti ni hatua mfululizo za kutatua tatizo.');
  await p.click('[data-testid="tp-note-save"]');
  await p.waitForTimeout(2500);
  const notesText = await p.innerText('[data-testid="teaching"]');
  log('notes zimehifadhiwa:', notesText.includes(`Notes za C ${stamp}`));

  // ---- 2. KAZI + AI-CHECK ----
  await p.click('[data-testid="teaching-section-assignments"]');
  await p.waitForSelector('[data-testid="tp-asg-title"]', { timeout: 20000 });
  await p.fill('[data-testid="tp-asg-title"]', `Zoezi la TP ${stamp}`);
  await p.fill('[data-testid="tp-asg-instructions"]', 'Eleza maana ya algoriti kwa Kiswahili.');
  await p.click('[data-testid="tp-asg-save"]');
  await p.waitForTimeout(2500);
  const asgText = await p.innerText('[data-testid="teaching"]');
  log('kazi imehifadhiwa:', asgText.includes(`Zoezi la TP ${stamp}`));

  // Mwanafunzi huwasilisha (jibu fupi → AI-check halisi au pending).
  const stu = await post('/elimupro/student/admit', {
    full_name: `Neema TP ${stamp}`,
    course_id: (await get('/elimupro/courses')).courses[0].id,
    level_id: lv.id,
    gender: 'F',
    allow_unverified: true,
  });
  const asgList = (await get('/teaching/assignments')).assignments;
  const aid = asgList.find((a) => a.title.includes(`Zoezi la TP ${stamp}`)).id;
  const sub = await post(`/teaching/assignment/${aid}/submit`, {
    student_id: stu.id,
    content: 'Algoriti ni mfululizo wa hatua zinazotatua tatizo hatua kwa hatua.',
  });
  log('jibu + AI-check:', sub.ok, '· mode', sub.ai_mode, '· alama AI', sub.ai_score);

  // Kichupo huonyesha majibu yenye alama ya AI.
  await p.click(`[data-testid="tp-asg-item-${aid}"]`);
  await p.waitForSelector('[data-testid="tp-asg-subs"]', { timeout: 20000 });
  await p.waitForTimeout(1500);
  const subsText = await p.innerText('[data-testid="tp-asg-subs"]');
  log('majibu yanaonekana kwenye UI:', subsText.includes(`Neema TP ${stamp}`));

  // ---- 3. VIKUNDI + CHAT (live) ----
  await p.click('[data-testid="teaching-section-groups"]');
  await p.waitForSelector('[data-testid="tp-grp-name"]', { timeout: 20000 });
  await p.fill('[data-testid="tp-grp-name"]', `Kikundi TP ${stamp}`);
  await p.click('[data-testid="tp-grp-save"]');
  await p.waitForTimeout(2500);
  const gid = (await get('/teaching/groups')).groups.find((g) => g.name.includes(`Kikundi TP ${stamp}`)).id;
  await p.click(`[data-testid="tp-grp-item-${gid}"]`);
  await p.waitForSelector('[data-testid="tp-chat"]', { timeout: 20000 });
  await p.waitForTimeout(1500);

  // Ujumbe hutumwa kwa HTTP (kama mwalimu mwingine) → LAZIMA uonekane kwa WS.
  await post(`/teaching/group/${gid}/message`, {
    author: 'Mwalimu Mwingine',
    author_type: 'teacher',
    body: `Habari za TP ${stamp}`,
  });
  await p.waitForTimeout(2500);
  const chatText = await p.innerText('[data-testid="tp-chat-messages"]');
  log('chat ya live (WS) imepokea ujumbe:', chatText.includes(`Habari za TP ${stamp}`));

  // Mwalimu hujibu kutoka UI.
  await p.fill('[data-testid="tp-chat-input"]', `Salaam ${stamp}`);
  await p.click('[data-testid="tp-chat-send"]');
  await p.waitForTimeout(2000);
  const chat2 = await p.innerText('[data-testid="tp-chat-messages"]');
  log('ujumbe kutoka UI umetumwa:', chat2.includes(`Salaam ${stamp}`));

  // ---- 4. MATANGAZO ----
  await p.click('[data-testid="teaching-section-announcements"]');
  await p.waitForSelector('[data-testid="tp-ann-title"]', { timeout: 20000 });
  await p.fill('[data-testid="tp-ann-title"]', `Tangazo la TP ${stamp}`);
  await p.fill('[data-testid="tp-ann-body"]', 'Karibuni kwenye muhula mpya.');
  await p.click('[data-testid="tp-ann-save"]');
  await p.waitForTimeout(2500);
  const annText = await p.innerText('[data-testid="teaching"]');
  log('tangazo limetumwa:', annText.includes(`Tangazo la TP ${stamp}`));

  await b.close();
  log('DONE');
})().catch((e) => {
  console.error('E2E FAILED:', e.message);
  process.exit(1);
});
