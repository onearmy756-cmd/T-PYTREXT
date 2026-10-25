// E2E: ELIMUPRO Awamu 1 — taasisi → course → level → subject → mwalimu →
// usajili (NECTA/RITA manual) → enrollment → ID card. Hupitia UI halisi.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');

const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({
    executablePath: '/usr/bin/chromium',
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--ignore-certificate-errors'],
  });
  const ctx = await b.newContext({ viewport: { width: 1400, height: 1100 }, ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => {
    localStorage.setItem('mbilinyi-onboarded', '1');
    localStorage.setItem('mbilinyi-onboarding-started', '1');
  });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 200)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(4000);
  await p.click('[data-testid="edu-tab-elimupro"]');
  await p.waitForSelector('[data-testid="elimupro-status"]', { timeout: 30000 });
  log('kichupo cha ELIMUPRO kimefunguliwa:', true);

  // ---- 1. TAASISI ----
  await p.fill('[data-testid="ep-inst-name"]', `Chuo cha E2E ${stamp}`);
  await p.fill('[data-testid="ep-inst-code"]', `E${stamp.slice(-3)}`);
  await p.click('[data-testid="ep-inst-save"]');
  await p.waitForSelector('[data-testid="ep-inst-msg"]', { timeout: 30000 });
  log('taasisi:', (await p.innerText('[data-testid="ep-inst-msg"]')).trim());

  // ---- 2. COURSE + LEVEL + SUBJECT ----
  await p.click('[data-testid="elimupro-section-courses"]');
  await p.waitForSelector('[data-testid="ep-course-name"]', { timeout: 20000 });
  await p.fill('[data-testid="ep-course-name"]', `Diploma ya E2E ${stamp}`);
  await p.fill('[data-testid="ep-course-code"]', 'DCS');
  await p.click('[data-testid="ep-course-add"]');
  await p.waitForSelector('[data-testid="ep-levels"]', { timeout: 30000 });
  const levels = await p.$$eval('[data-testid="ep-levels"] button', (n) => n.length);
  log('levels zilizotengenezwa automatically:', levels);

  await p.fill('[data-testid="ep-subject-name"]', `Programming E2E ${stamp}`);
  await p.click('[data-testid="ep-subject-add"]');
  await p.waitForTimeout(1500);
  const subs = await p.innerText('[data-testid="ep-subjects"]');
  log('somo limeongezwa:', subs.includes('Programming E2E'));

  // ---- 3. MWALIMU + KUGAWA SOMO ----
  await p.click('[data-testid="elimupro-section-teachers"]');
  await p.waitForSelector('[data-testid="ep-teacher-name"]', { timeout: 20000 });
  await p.fill('[data-testid="ep-teacher-name"]', `Mwalimu E2E ${stamp}`);
  await p.click('[data-testid="ep-teacher-add"]');
  await p.waitForSelector('[data-testid="ep-teacher-created"]', { timeout: 30000 });
  log('mwalimu:', (await p.innerText('[data-testid="ep-teacher-created"]')).replace(/\s+/g, ' ').slice(0, 80));

  await p.click('[data-testid="ep-teachers-list"] button');
  await p.waitForSelector('[data-testid="ep-teacher-detail"]', { timeout: 20000 });
  await p.selectOption('[data-testid="ep-assign-subject"]', { index: 1 });
  await p.click('[data-testid="ep-assign-do"]');
  await p.waitForTimeout(1800);
  const tdet = await p.innerText('[data-testid="ep-teacher-detail"]');
  log('somo limegawiwa:', tdet.includes('Programming E2E'));

  // ---- 4. USAJILI WA MWANAFUNZI ----
  await p.click('[data-testid="elimupro-section-students"]');
  await p.waitForSelector('[data-testid="ep-student-name"]', { timeout: 20000 });
  await p.fill('[data-testid="ep-student-name"]', `Neema E2E ${stamp}`);
  await p.fill('[data-testid="ep-student-form4"]', `S9999/${stamp}`);
  await p.fill('[data-testid="ep-student-birth"]', `1990-${stamp}`);

  // Kwanza: thibitisha NECTA (hapana API → manual)
  await p.click('[data-testid="ep-verify-necta"]');
  await p.waitForSelector('[data-testid="ep-necta-result"]', { timeout: 20000 });
  log('NECTA:', (await p.innerText('[data-testid="ep-necta-result"]')).replace(/\s+/g, ' ').slice(0, 70));

  // Chagua manual, kisha sajili
  await p.selectOption('[data-testid="ep-necta-mode"]', 'manual');
  await p.click('[data-testid="ep-student-admit"]');
  await p.waitForSelector('[data-testid="ep-admitted"]', { timeout: 30000 });
  const adm = (await p.innerText('[data-testid="ep-admitted"]')).replace(/\s+/g, ' ');
  log('usajili:', adm.slice(0, 90));
  const regNo = (adm.match(/([A-Z0-9]+\/\d{4}\/[A-Z]+\/\d{4})/) || [''])[0];
  log('reg_no iliyopatikana:', regNo);

  // ID card image actually loads?
  const imgOk = await p.$eval('[data-testid="ep-idcard-img"]', (img) =>
    img.complete && img.naturalWidth > 0);
  log('ID card SVG imepakiwa:', imgOk);

  // ---- 5. ENROLLMENT ----
  await p.click(`[data-testid="ep-student-${regNo}"]`);
  await p.waitForSelector('[data-testid="ep-student-detail"]', { timeout: 20000 });
  const sdet = await p.innerText('[data-testid="ep-student-detail"]');
  log('enrollment inaonyesha somo+mwalimu:', sdet.includes('Programming E2E') && sdet.includes('Mwalimu E2E'));

  await p.screenshot({ path: '/tmp/elimupro_browser.png', fullPage: true });
  await b.close();
  log('DONE');
})().catch((e) => {
  console.error('FAIL:', e.message);
  process.exit(1);
});
