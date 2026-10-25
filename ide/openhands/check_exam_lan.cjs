// E2E: mwalimu hufungua chumba → QR → mwanafunzi hujiunga (tab nyingine) →
// mwalimu anaona live → mwanafunzi anawasilisha → matokeo.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');

const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);

(async () => {
  const b = await chromium.launch({
    executablePath: '/usr/bin/chromium',
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--ignore-certificate-errors'],
  });
  const ctx = await b.newContext({ viewport: { width: 1400, height: 1000 }, ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => {
    localStorage.setItem('mbilinyi-onboarded', '1');
    localStorage.setItem('mbilinyi-onboarding-started', '1');
  });

  // ---- MWALIMU: fungua chumba ----
  const teacher = await ctx.newPage();
  teacher.on('pageerror', (e) => log('TEACHER-PAGE-ERR:', String(e).slice(0, 200)));
  await teacher.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await teacher.waitForTimeout(4000);
  await teacher.click('[data-testid="edu-tab-lan"]');
  await teacher.waitForTimeout(1500);
  log('kichupo cha LAN kimefunguliwa:', true);

  const examOpts = await teacher.$$eval('[data-testid="exam-room-exam"] option', (o) => o.length);
  log('mitihani inapatikana:', examOpts);
  await teacher.fill('[data-testid="exam-room-duration"]', '45');
  await teacher.click('[data-testid="exam-room-create"]');
  await teacher.waitForSelector('[data-testid="exam-room-code"]', { timeout: 60000 });
  const code = (await teacher.innerText('[data-testid="exam-room-code"]')).trim();
  const qr = await teacher.$('[data-testid="exam-room-qr"] svg');
  const joinUrl = await teacher.innerText('[data-testid="exam-room-live"] .font-mono.text-xs');
  log('code:', code, '| QR:', !!qr, '| join_url:', joinUrl.trim());

  // ---- MWANAFUNZI: jiunge kwa QR (tab nyingine) ----
  const student = await ctx.newPage();
  student.on('pageerror', (e) => log('STUDENT-PAGE-ERR:', String(e).slice(0, 200)));
  const sUrl = joinUrl.trim().replace(/^http:\/\/[^/]+/, BASE);
  await student.goto(sUrl, { waitUntil: 'networkidle', timeout: 90000 });
  await student.waitForSelector('[data-testid="exam-join-name"]', { timeout: 30000 });
  log('ukurasa wa mwanafunzi umefunguka (QR URL):', sUrl);
  await student.fill('[data-testid="exam-join-name"]', 'Neema Mushi');
  await student.fill('[data-testid="exam-join-no"]', 'S100');
  await student.click('[data-testid="exam-join-start"]');
  await student.waitForSelector('[data-testid="exam-student-running"]', { timeout: 30000 });
  const qText = (await student.innerText('[data-testid="exam-student-answer"] ~ *, [data-testid="exam-student-running"]')).slice(0, 0);
  const question = await student.$eval('[data-testid="exam-student-running"] .whitespace-pre-wrap', (e) => e.innerText).catch(() => '');
  log('swali la kwanza (70):', question.replace(/\s+/g, ' ').slice(0, 70));
  await student.fill('[data-testid="exam-student-answer"]', 'Mole ni kiasi cha dutu yenye chembe 6.022x10^23');
  await student.waitForTimeout(1500);

  // ---- MWALIMU: anaona live ----
  await teacher.waitForTimeout(5000);
  const liveTxt = await teacher.innerText('[data-testid="exam-room-live"]');
  log('mwalimu anaona Neema:', liveTxt.includes('Neema Mushi'));
  const progress = (liveTxt.match(/1\/\d+/) || ['?'])[0];
  log('maendeleo yanayoonekana:', progress);

  // ---- MWANAFUNZI: wasilisha ----
  await student.click('[data-testid="exam-student-submit"]');
  await student.waitForSelector('[data-testid="exam-student-result"]', { timeout: 30000 });
  const result = await student.innerText('[data-testid="exam-student-result"]');
  log('matokeo ya mwanafunzi (60):', result.replace(/\s+/g, ' ').slice(0, 60));

  // ---- MWALIMU: matokeo ----
  await teacher.waitForTimeout(4000);
  const liveTxt2 = await teacher.innerText('[data-testid="exam-room-live"]');
  log('mwalimu anaona amewasilisha:', liveTxt2.includes('amewasilisha'));
  await teacher.click('[data-testid="exam-room-live"] button:has-text("Matokeo")');
  await teacher.waitForSelector('[data-testid="exam-room-results"]', { timeout: 30000 });
  const res = await teacher.innerText('[data-testid="exam-room-results"]');
  log('ripoti ya mwalimu (80):', res.replace(/\s+/g, ' ').slice(0, 80));

  await teacher.screenshot({ path: '/tmp/exam_teacher.png', fullPage: true });
  await student.screenshot({ path: '/tmp/exam_student.png', fullPage: true });
  await b.close();
  log('DONE');
})().catch((e) => {
  console.error('FAIL:', e.message);
  process.exit(1);
});
