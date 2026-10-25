const { chromium } = require('playwright-core');
const URL = process.env.EDU_URL || 'http://127.0.0.1:12000/ide/elimu';

(async () => {
  const b = await chromium.launch({
    executablePath: '/usr/bin/chromium',
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--ignore-certificate-errors'],
  });
  const ctx = await b.newContext({ viewport: { width: 1400, height: 1000 }, ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => {
    localStorage.setItem("mbilinyi-onboarded", "1");
    localStorage.setItem("mbilinyi-onboarding-started", "1");
  });
  const page = await ctx.newPage();
  page.on('console', (m) => { if (m.type() === 'error') console.log('CONSOLE-ERR:', m.text().slice(0, 200)); });
  page.on('pageerror', (e) => console.log('PAGE-ERR:', String(e).slice(0, 250)));
  page.on('response', (r) => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url().slice(-80)); });
  await page.goto(URL, { waitUntil: 'networkidle', timeout: 90000 });
  await page.waitForTimeout(7000);
  console.log('body (90):', (await page.innerText('body')).replace(/\s+/g, ' ').slice(0, 90));
  console.log('html len:', (await page.content()).length);

  const tabs = await page.$$eval('[data-testid^="edu-tab-"]', (els) =>
    els.map((e) => e.getAttribute('data-testid')));
  console.log('tabs:', tabs.join(', '));

  const hasLesson = await page.$('[data-testid="edu-tab-lesson"]');
  console.log('AI Mwalimu tab ipo:', !!hasLesson);

  if (hasLesson) {
    const active = await page.$eval('[data-testid="edu-tab-lesson"]', (e) =>
      e.className.includes('border-primary'));
    console.log('kichupo cha AI Mwalimu kimechaguliwa:', active);
    console.log('file input ipo:', !!(await page.$('[data-testid="edu-lesson-file"]')));
    console.log('tengeneza button ipo:', !!(await page.$('[data-testid="edu-lesson-build"]')));
    console.log('kichwa input ipo:', !!(await page.$('[data-testid="edu-lesson-title"]')));
    console.log('kiwango select ipo:', !!(await page.$('[data-testid="edu-lesson-level"]')));

    // E2E: upload notes halisi → somo + sauti
    if (process.env.EDU_E2E === '1') {
      await page.setInputFiles('[data-testid="edu-lesson-file"]', '/tmp/notes_maji.txt');
      await page.fill('[data-testid="edu-lesson-title"]', 'Mzunguko wa Maji (E2E)');
      await page.fill('[data-testid="edu-lesson-subject"]', 'Jiografia');
      await page.click('[data-testid="edu-lesson-build"]');
      console.log('upload imeanzishwa — nasubiri somo…');
      await page.waitForSelector('[data-testid="edu-lesson-view"]', { timeout: 300000 });
      console.log('somo limeonekana:', true);
      const sections = await page.$$eval('[data-testid="edu-lesson-view"] h4', (els) =>
        els.map((e) => e.innerText.trim()).filter(Boolean));
      console.log('vichwa vya sehemu:', sections.join(' | '));
      const audioBtns = await page.$$eval(
        '[data-testid="edu-lesson-view"] button',
        (els) => els.map((e) => e.innerText.trim()).filter((t) => t.includes('Sikiliza')));
      console.log('vitufe vya sauti:', audioBtns.length, '→', audioBtns.join(' | '));
      await page.fill('[data-testid="edu-lesson-ask-input"]', 'Nieleze kuhusu mvua kwa ufupi.');
      await page.click('[data-testid="edu-lesson-ask"]');
      await page.waitForSelector('[data-testid="edu-lesson-answer"]', { timeout: 240000 });
      const ans = await page.$eval('[data-testid="edu-lesson-answer"]', (e) => e.innerText);
      console.log('jibu la AI (100):', ans.replace(/\s+/g, ' ').slice(0, 100));

      // Sauti inacheza kwa kweli?
      await page.click('[data-testid="edu-lesson-view"] button:has-text("Sikiliza utangulizi")');
      await page.waitForTimeout(4000);
      const a = await page.evaluate(() => {
        const el = document.querySelector('[data-testid="edu-lesson-view"] audio');
        return el ? { src: el.currentSrc.slice(-40), t: el.currentTime, dur: el.duration } : null;
      });
      console.log('audio inacheza:', JSON.stringify(a));
      await page.screenshot({ path: '/tmp/edu_lesson_e2e.png', fullPage: true });
    }
  }

  await page.screenshot({ path: '/tmp/edu_lesson_tab.png', fullPage: false });
  await b.close();
  console.log('DONE');
})();
