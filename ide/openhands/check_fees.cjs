// E2E: ELIMUPRO Awamu 8 — ada, ankara na malipo.
const { chromium } = require('/workspace/project/engines/mbilinyi/node_modules/playwright-core');
const BASE = process.env.BASE || 'http://127.0.0.1:12000';
const log = (...a) => console.log(...a);
const stamp = Date.now().toString().slice(-5);

(async () => {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx0 = await b.newContext({ ignoreHTTPSErrors: true });
  const post = async (p, body) => (await ctx0.request.post(`${BASE}/api${p}`, { data: body })).json();
  const get = async (p) => (await ctx0.request.get(`${BASE}/api${p}`)).json();

  await post('/elimupro/institution/save', { name: `Chuo FE ${stamp}`, code: `F${stamp.slice(-3)}` });
  await post('/elimupro/course/add', { name: `CS ${stamp}`, code: 'CS', years: 1, auto_levels: true });
  const course = (await get('/elimupro/courses')).courses.find((c) => c.name === `CS ${stamp}`).id;
  const lv = (await get(`/elimupro/levels?course_id=${course}`)).levels[0];
  await post('/elimupro/subject/add', { level_id: lv.id, name: 'Fizikia', code: 'PHY' });
  const inst = (await get('/elimupro/institution')).institution?.id || '';
  const st = await post('/elimupro/student/admit', {
    institution_id: inst, course_id: course, level_id: lv.id,
    full_name: `Neema Fe ${stamp}`, gender: 'F', allow_unverified: true,
  });
  log('mwanafunzi amesajiliwa:', Boolean(st.id));

  const ctx = await b.newContext({ ignoreHTTPSErrors: true });
  await ctx.addInitScript(() => { localStorage.setItem('mbilinyi-onboarded', '1'); });
  const p = await ctx.newPage();
  p.on('pageerror', (e) => log('PAGE-ERR:', String(e).slice(0, 150)));

  await p.goto(`${BASE}/ide/elimu`, { waitUntil: 'networkidle', timeout: 90000 });
  await p.waitForTimeout(3000);
  await p.click('[data-testid="edu-tab-ada"]');
  await p.waitForSelector('[data-testid="fee-status"]', { timeout: 30000 });
  log('kichupo cha Ada kimefunguliwa: true');

  await p.selectOption('[data-testid="fee-level"]', lv.id);
  await p.waitForTimeout(1200);
  await p.fill('[data-testid="fee-item-name"]', 'Ada ya Muhula');
  await p.fill('[data-testid="fee-item-amount"]', '1500');
  await p.click('[data-testid="fee-item-add"]');
  await p.waitForTimeout(1500);
  await p.fill('[data-testid="fee-item-name"]', 'Bima');
  await p.fill('[data-testid="fee-item-amount"]', '50.50');
  await p.click('[data-testid="fee-item-add"]');
  await p.waitForTimeout(1500);

  const items = (await get(`/fees/items?level_id=${lv.id}`)).items;
  log('ada 2 zimeongezwa:', items.length === 2);
  log('kiasi kimehifadhiwa kwa senti (150000):', items.find((i) => i.name === 'Ada ya Muhula').amount_cents === 150000);
  log('kiasi cha desimali (5050):', items.find((i) => i.name === 'Bima').amount_cents === 5050);

  // Tengeneza ankara kwa UI
  await p.click(`[data-testid="fee-invoice-create-${st.id}"]`);
  await p.waitForTimeout(2000);
  const invs = (await get(`/fees/invoices?level_id=${lv.id}`)).invoices;
  log('ankara imetengenezwa:', invs.length === 1);
  const inv = invs[0];
  log('jumla = 1550.50 (150000+5050):', inv.total_cents === 155050);
  log('hali ni unpaid:', inv.status === 'unpaid');

  // Ankara hiyo hiyo haitengenezwi mara mbili
  const dup = await post('/fees/invoice/create', { student_id: st.id, term: '2026/T1' });
  log('ankara hairudiwi (existing):', dup.existing === true);

  // Malipo ya kwanza: 1000
  await p.click(`[data-testid="fee-pay-${inv.id}"]`);
  await p.waitForSelector('[data-testid="fee-pay-dialog"]', { timeout: 20000 });
  await p.fill('[data-testid="fee-pay-amount"]', '1000');
  await p.click('[data-testid="fee-pay-submit"]');
  await p.waitForTimeout(2000);
  const after1 = (await get(`/fees/invoice/${inv.id}`)).invoice;
  log('malipo ya kwanza (partial):', after1.status === 'partial' && after1.paid_cents === 100000);
  log('salio = 550.50:', after1.balance_cents === 55050);
  log('risiti imetolewa:', after1.payments.length === 1 && /^RCP\/\d{4}\/\d{6}$/.test(after1.payments[0].receipt_no));
  log('risiti namba:', after1.payments[0].receipt_no);

  // Malipo yanayozidi salio hukataliwa
  const over = await post('/fees/pay', { invoice_id: inv.id, amount: '9999' });
  log('malipo yanayozidi salio hukataliwa:', over.ok === false);

  // Malipo ya mwisho: 550.50 → paid
  const last = await post('/fees/pay', { invoice_id: inv.id, amount: '550.50', method: 'mobile' });
  log('malipo ya mwisho hufanya paid:', last.ok === true && last.status === 'paid' && last.balance_cents === 0);
  log('risiti ya pili:', last.receipt_no);

  // Muhtasari
  const sum = await get(`/fees/summary?level_id=${lv.id}`);
  log('makusanyo yote (100%):', sum.collection_rate === 100 && sum.collected_cents === 155050);
  log('hakuna salio:', sum.outstanding_cents === 0);

  // Risiti ya malipo
  const rcp = await get(`/fees/receipt/${last.payment_id}`);
  log('risiti inaonyeshwa:', rcp.ok && rcp.receipt.receipt_no === last.receipt_no);

  // Filtra kwa hali
  const unpaid = await get(`/fees/invoices?level_id=${lv.id}&status=unpaid`);
  log('filtra ya unpaid ni tupu:', unpaid.invoices.length === 0);

  await ctx.close(); await ctx0.close(); await b.close();
  log('DONE');
})().catch((e) => { console.error('E2E FAILED:', e.message); process.exit(1); });
