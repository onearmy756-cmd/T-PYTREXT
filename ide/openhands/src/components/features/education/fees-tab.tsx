import { useCallback, useEffect, useState } from "react";
import { Banknote, FileText, Plus, Receipt, RefreshCw, TrendingUp, Wallet } from "lucide-react";
import {
  feesService,
  type FeeInvoice,
  type FeeItem,
  type FeeStatus,
  type FeeSummary,
} from "#/api/education-service/fees.api";
import { elimuproService, type EpLevel, type EpStudent } from "#/api/education-service/elimupro.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
} from "./edu-ui";

function statusBadge(status: string) {
  const map: Record<string, string> = {
    paid: "bg-success/10 text-success border-success/30",
    partial: "bg-warning/10 text-warning border-warning/30",
    unpaid: "bg-danger/10 text-danger border-danger/30",
  };
  return map[status] ?? "border-border text-muted";
}

/**
 * ELIMUPRO — Awamu 8: ada, ankara na malipo.
 * Ankara huchukua ada za lazima za level ya mwanafunzi; malipo hupata risiti
 * ya kipekee; salio hupungua hadi `paid`.
 */
export function FeesTab() {
  const [status, setStatus] = useState<FeeStatus | null>(null);
  const [summary, setSummary] = useState<FeeSummary | null>(null);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [students, setStudents] = useState<EpStudent[]>([]);
  const [items, setItems] = useState<FeeItem[]>([]);
  const [invoices, setInvoices] = useState<FeeInvoice[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  // Fomu
  const [itemName, setItemName] = useState("");
  const [itemAmount, setItemAmount] = useState("");
  const [term, setTerm] = useState("2026/T1");
  const [payFor, setPayFor] = useState<FeeInvoice | null>(null);
  const [payAmount, setPayAmount] = useState("");
  const [payMethod, setPayMethod] = useState("cash");

  const refresh = useCallback(async () => {
    try {
      setStatus(await feesService.status());
      setItems((await feesService.items(levelId)).items);
      setInvoices((await feesService.invoices({ level_id: levelId })).invoices);
      if (levelId) {
        setSummary(await feesService.summary({ level_id: levelId }));
        setStudents((await elimuproService.students(levelId)).students);
      }
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [levelId]);

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.levels("");
        setLevels(r.levels);
        if (r.levels[0]) setLevelId(r.levels[0].id);
      } catch (e) {
        setErr(String(e));
      }
    })();
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const addItem = async () => {
    setBusy(true);
    try {
      const r = await feesService.addItem({ level_id: levelId, name: itemName, amount: itemAmount });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Ada "${itemName}" imeongezwa.`);
        setItemName("");
        setItemAmount("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const createInvoice = async (studentId: string) => {
    setBusy(true);
    try {
      const r = await feesService.createInvoice({ student_id: studentId, term });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(r.existing ? "Ankara ilikuwepo tayari." : `Ankara imetengenezwa (${r.lines} mistari).`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const submitPay = async () => {
    if (!payFor) return;
    setBusy(true);
    try {
      const r = await feesService.pay({
        invoice_id: payFor.id,
        amount: payAmount,
        method: payMethod,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Malipo yamepokelewa. Risiti: ${r.receipt_no} (${r.status}).`);
        setPayFor(null);
        setPayAmount("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="fees" className="flex flex-col gap-4">
      <EduCard testId="fee-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Wallet className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">Ada + Malipo (Awamu 8)</span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="fee-billed">billed {status ? (status.billed / 100).toLocaleString() : 0}</EduBadge>
            <EduBadge testId="fee-collected">collected {status ? (status.collected / 100).toLocaleString() : 0}</EduBadge>
            <EduBadge testId="fee-outstanding">outstanding {status ? (status.outstanding / 100).toLocaleString() : 0}</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard>
        <EduField label="Level">
          <EduSelect testId="fee-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
            {levels.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
      </EduCard>

      <EduCard testId="fee-items">
        <div className="flex items-center gap-2">
          <Banknote className="size-4 text-primary" aria-hidden />
          <span className="text-sm font-semibold text-content">Aina za ada ({items.length})</span>
        </div>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Jina">
            <EduInput testId="fee-item-name" value={itemName} onChange={(e) => setItemName(e.target.value)} placeholder="Ada ya Muhula" />
          </EduField>
          <EduField label="Kiasi">
            <EduInput testId="fee-item-amount" value={itemAmount} onChange={(e) => setItemAmount(e.target.value)} placeholder="1500" />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="fee-item-add" disabled={busy || !itemName || !itemAmount} onClick={() => void addItem()}>
              <Plus className="size-4" aria-hidden /> Ongeza
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {items.length === 0 ? (
            <EduEmpty>Hakuna aina za ada bado.</EduEmpty>
          ) : (
            items.map((it) => (
              <div key={it.id} data-testid={`fee-item-${it.id}`} className="flex items-center justify-between rounded-md border border-border p-2">
                <span className="text-sm text-content">{it.name}</span>
                <div className="flex gap-1">
                  <EduBadge>{it.amount}</EduBadge>
                  <EduBadge>{it.mandatory ? "lazima" : "hiari"}</EduBadge>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="fee-invoices">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <FileText className="size-4 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">Ankara ({invoices.length})</span>
          </div>
          <div className="flex items-center gap-2">
            <EduInput testId="fee-term" value={term} onChange={(e) => setTerm(e.target.value)} />
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {invoices.length === 0 ? (
            <EduEmpty>Hakuna ankara bado.</EduEmpty>
          ) : (
            invoices.map((inv) => (
              <div key={inv.id} data-testid={`fee-invoice-${inv.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {inv.student} <span className="text-xs text-muted">({inv.reg_no}) — {inv.term}</span>
                </span>
                <div className="flex flex-wrap items-center gap-1">
                  <EduBadge>{inv.total}</EduBadge>
                  <EduBadge>imelipwa {inv.paid}</EduBadge>
                  <span className={`rounded-md border px-2 py-0.5 text-xs ${statusBadge(inv.status)}`} data-testid={`fee-status-${inv.id}`}>
                    {inv.status}
                  </span>
                  <EduButton variant="ghost" testId={`fee-pay-${inv.id}`} onClick={() => setPayFor(inv)}>
                    <Receipt className="size-3.5" aria-hidden /> Lipa
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="fee-students">
        <span className="text-sm font-semibold text-content">Tengeneza ankara kwa mwanafunzi</span>
        <div className="mt-2 flex flex-col gap-1">
          {students.length === 0 ? (
            <EduEmpty>Hakuna wanafunzi kwenye level hii.</EduEmpty>
          ) : (
            students.map((st) => (
              <div key={st.id} className="flex items-center justify-between rounded-md border border-border p-2">
                <span className="text-sm text-content">{st.full_name}</span>
                <EduButton variant="ghost" testId={`fee-invoice-create-${st.id}`} disabled={busy} onClick={() => void createInvoice(st.id)}>
                  Tengeneza ankara
                </EduButton>
              </div>
            ))
          )}
        </div>
      </EduCard>

      {summary ? (
        <EduCard testId="fee-summary">
          <div className="flex items-center gap-2">
            <TrendingUp className="size-4 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">Muhtasari wa makusanyo</span>
            <EduBadge testId="fee-rate">{summary.collection_rate}%</EduBadge>
          </div>
          <div className="mt-2 flex flex-wrap gap-2 text-xs text-muted">
            <span>Billed: {summary.billed}</span>
            <span>Collected: {summary.collected}</span>
            <span>Outstanding: {summary.outstanding}</span>
          </div>
        </EduCard>
      ) : null}

      {payFor ? (
        <EduCard testId="fee-pay-dialog">
          <span className="text-sm font-semibold text-content">
            Lipa ankara ya {payFor.student} (salio {payFor.balance})
          </span>
          <div className="mt-2 grid gap-2 md:grid-cols-3">
            <EduField label="Kiasi">
              <EduInput testId="fee-pay-amount" value={payAmount} onChange={(e) => setPayAmount(e.target.value)} placeholder={payFor.balance} />
            </EduField>
            <EduField label="Njia">
              <EduSelect testId="fee-pay-method" value={payMethod} onChange={(e) => setPayMethod(e.target.value)}>
                <option value="cash">Fedha taslimu</option>
                <option value="bank">Benki</option>
                <option value="mobile">Simu (M-Pesa)</option>
                <option value="cheque">Hundi</option>
              </EduSelect>
            </EduField>
            <div className="flex items-end gap-1">
              <EduButton testId="fee-pay-submit" disabled={busy || !payAmount} onClick={() => void submitPay()}>
                Pokea malipo
              </EduButton>
              <EduButton variant="ghost" onClick={() => setPayFor(null)}>
                Ghairi
              </EduButton>
            </div>
          </div>
        </EduCard>
      ) : null}
    </div>
  );
}
