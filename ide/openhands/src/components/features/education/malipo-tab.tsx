import { useCallback, useEffect, useState } from "react";
import { RefreshCw, Smartphone, Wallet } from "lucide-react";
import {
  malipoService,
  type Intent,
  type MalipoStatus,
  type Provider,
} from "#/api/education-service/malipo.api";
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

/**
 * ELIMUPRO — Awamu 19: malipo ya simu (moduli 19).
 * M-Pesa, Tigo Pesa (Mixx), Airtel Money — muamala huthibitishwa mara moja.
 */
export function MalipoTab() {
  const [status, setStatus] = useState<MalipoStatus | null>(null);
  const [providers, setProviders] = useState<Provider[]>([]);
  const [intents, setIntents] = useState<Intent[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [invoiceId, setInvoiceId] = useState("");
  const [provider, setProvider] = useState("mpesa");
  const [phone, setPhone] = useState("");
  const [amount, setAmount] = useState("");
  const [providerRef, setProviderRef] = useState("");
  const [receivedBy, setReceivedBy] = useState("");
  const [filter, setFilter] = useState("");
  const [recon, setRecon] = useState<{
    balanced: boolean;
    difference_cents: number;
    note: string;
  } | null>(null);

  const refresh = useCallback(async () => {
    try {
      const st = await malipoService.status();
      setStatus(st);
      setProviders(st.providers);
      setIntents((await malipoService.intents({ status: filter })).intents);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [filter]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const createIntent = async () => {
    if (!invoiceId.trim()) {
      setErr("Weka namba ya ankara");
      return;
    }
    setBusy(true);
    try {
      const r = await malipoService.createIntent({
        invoice_id: invoiceId,
        provider,
        phone,
        amount,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`${r.push} Kumbukumbu: ${r.tx_ref}`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const confirm = async (id: string) => {
    setBusy(true);
    try {
      const r = await malipoService.confirm({
        intent_id: id,
        provider_ref: providerRef,
        received_by: receivedBy,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Imethibitishwa. Risiti ${r.receipt_no} · ankara: ${r.invoice_status}`);
        setProviderRef("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const fail = async (id: string, cancelled: boolean) => {
    setBusy(true);
    try {
      const r = await malipoService.fail({ intent_id: id, cancelled });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Muamala: ${r.status}`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doReconcile = async () => {
    setBusy(true);
    try {
      setRecon(await malipoService.reconcile());
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="malipo" className="flex flex-col gap-4">
      <EduCard testId="malipo-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="flex items-center gap-2 text-sm font-semibold text-content">
            <Smartphone className="size-5 text-primary" aria-hidden />
            Malipo ya simu (Awamu 19)
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="malipo-pending">{status?.pending ?? 0} zinasubiri</EduBadge>
            <EduBadge testId="malipo-confirmed">{status?.confirmed ?? 0} zilizothibitishwa</EduBadge>
            <EduBadge testId="malipo-failed">{status?.failed ?? 0} zilizoshindwa</EduBadge>
            <EduBadge testId="malipo-collected">{status?.collected ?? "0"}</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          {providers.map((p) => (
            <EduBadge key={p.code} testId={`malipo-provider-${p.code}`}>
              {p.name}
            </EduBadge>
          ))}
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="malipo-intent">
        <span className="text-sm font-semibold text-content">Anzisha malipo</span>
        <div className="mt-2 grid gap-2 md:grid-cols-5">
          <EduField label="Ankara (ID)">
            <EduInput testId="malipo-invoice" value={invoiceId} onChange={(e) => setInvoiceId(e.target.value)} placeholder="ID ya ankara" />
          </EduField>
          <EduField label="Mtoa huduma">
            <EduSelect testId="malipo-provider-select" value={provider} onChange={(e) => setProvider(e.target.value)}>
              {providers.map((p) => (
                <option key={p.code} value={p.code}>
                  {p.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Namba ya simu">
            <EduInput testId="malipo-phone" value={phone} onChange={(e) => setPhone(e.target.value)} placeholder="0712345678" />
          </EduField>
          <EduField label="Kiasi">
            <EduInput testId="malipo-amount" value={amount} onChange={(e) => setAmount(e.target.value)} placeholder="50000" />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="malipo-intent-btn" disabled={busy} onClick={() => void createIntent()}>
              Anzisha
            </EduButton>
          </div>
        </div>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Kumbukumbu ya mtoa (hiari)">
            <EduInput testId="malipo-provider-ref" value={providerRef} onChange={(e) => setProviderRef(e.target.value)} placeholder="MPESA-REF-123" />
          </EduField>
          <EduField label="Aliyepokea">
            <EduInput testId="malipo-received-by" value={receivedBy} onChange={(e) => setReceivedBy(e.target.value)} placeholder="Bw. Fedha" />
          </EduField>
          <div className="flex items-end gap-1">
            <EduButton testId="malipo-reconcile" disabled={busy} onClick={() => void doReconcile()}>
              <Wallet className="size-4" aria-hidden /> Upatanisho
            </EduButton>
          </div>
        </div>
        {recon ? (
          <p data-testid="malipo-recon" className="mt-2 text-xs text-muted">
            {recon.balanced ? "Zinalingana" : `Tofauti: ${recon.difference_cents}`} — {recon.note}
          </p>
        ) : null}
      </EduCard>

      <EduCard testId="malipo-intents">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">Miamala ({intents.length})</span>
          <EduSelect testId="malipo-filter" value={filter} onChange={(e) => setFilter(e.target.value)}>
            <option value="">Zote</option>
            <option value="pending">Zinasubiri</option>
            <option value="confirmed">Zilizothibitishwa</option>
            <option value="failed">Zilizoshindwa</option>
            <option value="cancelled">Zilizofutwa</option>
          </EduSelect>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {intents.length === 0 ? (
            <EduEmpty>Hakuna muamala.</EduEmpty>
          ) : (
            intents.slice(0, 12).map((it) => (
              <div
                key={it.id}
                data-testid={`malipo-intent-${it.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {it.tx_ref}{" "}
                  <span className="text-xs text-muted">
                    · {it.provider} · {it.phone} · {it.amount}
                  </span>
                </span>
                <div className="flex items-center gap-1">
                  <EduBadge testId={`malipo-intent-status-${it.id}`}>{it.status}</EduBadge>
                  {it.status === "pending" ? (
                    <>
                      <EduButton testId={`malipo-confirm-${it.id}`} disabled={busy} onClick={() => void confirm(it.id)}>
                        Thibitisha
                      </EduButton>
                      <EduButton variant="ghost" testId={`malipo-fail-${it.id}`} disabled={busy} onClick={() => void fail(it.id, false)}>
                        Shindwa
                      </EduButton>
                      <EduButton variant="ghost" testId={`malipo-cancel-${it.id}`} disabled={busy} onClick={() => void fail(it.id, true)}>
                        Futa
                      </EduButton>
                    </>
                  ) : null}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
