// E2E: ELIMUPRO Awamu 9 — maktaba (vitabu, kukopa, kurudisha, faini).
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');
const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo LB ${stamp}`, code: `L${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CS ${stamp}`, code: 'CS', years: 1, auto_levels: true });
  const course = (await get('/elimupro/courses')).courses.find((c) => c.name === `CS ${stamp}`).id;
  const lv = (await get(`/elimupro/levels?course_id=${course}`)).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Fizikia', code: 'PHY' });
  const inst = (await get('/elimupro/institution')).institution?.id || '';
  const mk = async (n) => post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: n, gender: 'F', allow_unverified: true,
  });
  const st1 = await mk(`Amina Lb ${stamp}`);
  const st2 = await mk(`Baraka Lb ${stamp}`);
  log('wanafunzi 2 wamesajiliwa:', Boolean(st1.id && st2.id));

  const ctx = await b.newContext({ ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 150)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3000);
  await p.click('[data-testid="edu-tab-maktaba"]');
  await p.waitForSelector('[data-testid="lib-status"]', { timeout: 30000 });
  log('kichupo cha Maktaba kimefunguliwa: true');

  // Chagua level (kwa kukopesha) kwanza, ili wanafunzi wapakiwe.
  await p.selectOption('[data-testid="lib-level"]', lv.id);
  await p.waitForTimeout(1500);

  // Ongeza kitabu cha nakala 2 kwa UI
  await p.fill('[data-testid="lib-title"]', `Fizikia Kidato ${stamp}`);
  await p.fill('[data-testid="lib-author"]', 'Mwl Juma');
  await p.fill('[data-testid="lib-copies"]', '3');
  await p.click('[data-testid="lib-add-book"]');
  await p.waitForTimeout(1800);

  const books = (await get(`/library/books?search=Fizikia%20Kidato%20${stamp}`)).books;
  log('kitabu kimeongezwa:', books.length === 1);
  const book = books[0];
  log('nakala 3 zimetengenezwa:', book.copies === 3 && book.available === 3);

  const detail = await get(`/library/book/${book.id}`);
  log('barcode za kipekee:', detail.items.length === 3 && new Set(detail.items.map((i) => i.barcode)).size === 3);
  log('barcode mfano:', detail.items[0].barcode);

  // Kopesha kwa UI (chagua mwanafunzi kutoka dropdown)
  await p.selectOption(`[data-testid="lib-borrow-student-${book.id}"]`, st1.id);
  await p.waitForTimeout(1800);
  const availAfter = (await get(`/library/book/${book.id}`)).items.filter((i) => i.status === 'available').length;
  log('nakala moja imekopwa (2/3 zinapatikana):', availAfter === 2);

  // Kikomo cha mikopo: kopa hadi 3 (max) kwa mwanafunzi yule yule.
  const fresh = await get(`/library/book/${book.id}`);
  const free = fresh.items.filter((i) => i.status === 'available').map((i) => i.barcode);
  for (const code of free) {
    const r = await post('/library/borrow', { student_id: st1.id, barcode: code });
    log(`kukopa ${code}:`, r.ok === true || r.error);
  }
  // Jaribio la 4 hukataliwa kwa SABABU ya kikomo (kitabu kingine chenye nakala).
  const other = await post('/library/book/add', { title: `Kemia Kidato ${stamp}`, copies: 1 });
  const over = await post('/library/borrow', { student_id: st1.id, book_id: other.id });
  log('kikomo cha mikopo huzuiwa:', over.ok === false && over.max === 3);

  // Kurudisha kwa kuchelewa → faini
  const loans = (await get(`/library/loans?student_id=${st1.id}&active=true`)).loans;
  log('mikopo inayoendelea = 3:', loans.length === 3);
  const l1 = loans[0];
  const late = await post('/library/return', { loan_id: l1.id, today: '2027-01-01' });
  log('kurudisha kwa kuchelewa huzalisha faini:', late.ok === true && late.fine_due === true && late.fine_cents > 0);
  log('faini ya siku nyingi:', late.fine);
  log('faini = siku × 200:', late.fine_cents === late.late_days * 20000);

  // Faini isiyolipwa huzuia kukopa
  const blocked = await post('/library/borrow', { student_id: st1.id, book_id: book.id });
  log('faini isiyolipwa huzuia kukopa:', blocked.ok === false && blocked.error === 'faini isiyolipwa');

  // Malipo ya faini
  const payFine = await post('/library/fine/pay', { loan_id: l1.id });
  log('faini inaweza kulipwa:', payFine.ok === true);
  const payTwice = await post('/library/fine/pay', { loan_id: l1.id });
  log('faini hailipwi mara mbili:', payTwice.ok === false);

  // Sasa anaweza kukopa tena (ana mikopo 2)
  const again = await post('/library/borrow', { student_id: st1.id, book_id: book.id });
  log('baada ya kulipa faini anaweza kukopa:', again.ok === true);

  // Kurudisha kwa wakati hakuna faini
  const loans2 = (await get(`/library/loans?student_id=${st1.id}&active=true`)).loans;
  const onTime = await post('/library/return', { loan_id: loans2[0].id });
  log('kurudisha kwa wakati hakuna faini:', onTime.ok === true && onTime.fine_due === false);

  // Orodha ya walichelewa
  const od = await get('/library/overdue');
  log('orodha ya walichelewa inafanya kazi:', od.ok === true && typeof od.count === 'number');

  // Mwanafunzi wa pili: nakala iliyokopwa haiwezi kukopwa tena
  const nowDetail = await get(`/library/book/${book.id}`);
  const taken = nowDetail.items.find((i) => i.status === 'loaned');
  if (taken) {
    const dup = await post('/library/borrow', { student_id: st2.id, barcode: taken.barcode });
    log('nakala iliyokopwa haiwezi kukopwa tena:', dup.ok === false);
  } else {
    log('nakala iliyokopwa haiwezi kukopwa tena: (hakuna nakala iliyokopwa)', false);
  }

  // Kufuta kitabu kilicho na mikopo hukataliwa
  const del = await post('/library/book/delete', { id: book.id });
  log('kufuta kitabu chenye mikopo hukataliwa:', del.ok === false);

  await ctx.close(); await ctx0.close(); await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
