import { useCallback, useEffect, useState } from "react";
import { BadgeCheck, FileCheck, RefreshCw, ShieldCheck, Stamp } from "lucide-react";
import {
  clearanceService,
  type Clearance,
  type ClearanceStatus,
  type ClearanceTemplate,
} from "#/api/education-service/clearance.api";
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
 * ELIMUPRO — Awamu 14: fomu ya kusafisha (moduli 14).
 * Departments husaini sehemu zao; clearance hukamilika sehemu zote
 * zinazozuia zikisainiwa.
 */
export function ClearanceTab() {
  const [status, setStatus] = useState<ClearanceStatus | null>(null);
  const [templates, setTemplates] = useState<ClearanceTemplate[]>([]);
  const [list, setList] = useState<Clearance[]>([]);
  const [sel, setSel] = useState<Clearance | null>(null);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [tplName, setTplName] = useState("");
  const [sealText, setSealText] = useState("");
  const [studentId, setStudentId] = useState("");
  const [tplId, setTplId] = useState("");
  const [year, setYear] = useState("");
  const [dept, setDept] = useState("library");
  const [signedBy, setSignedBy] = useState("");
  const [comment, setComment] = useState("");
  const [filterDept, setFilterDept] = useState("");

  const refresh = useCallback(async () => {
    try {
      setStatus(await clearanceService.status());
      const t = (await clearanceService.templates()).templates;
      setTemplates(t);
      if (!tplId && t.length > 0) setTplId(t[0].id);
      const l = await clearanceService.list({ department: filterDept });
      setList(l.clearances);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [tplId, filterDept]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const addTemplate = async () => {
    setBusy(true);
    try {
      const r = await clearanceService.addTemplate({ name: tplName, seal_text: sealText });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Template imeundwa na idara nne za msingi.");
        setTplName("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doRequest = async () => {
    if (!studentId.trim() || !tplId) {
      setErr("Weka mwanafunzi na template");
      return;
    }
    setBusy(true);
    try {
      const r = await clearanceService.request({
        student_id: studentId,
        template_id: tplId,
        academic_year: year,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Clearance imeombwa (${r.sections} sehemu).`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const openOne = async (id: string) => {
    try {
      const r = await clearanceService.get(id);
      setSel(r.clearance);
    } catch (e) {
      setErr(String(e));
    }
  };

  const sign = async (force = false) => {
    if (!sel) return;
    setBusy(true);
    try {
      const r = await clearanceService.sign({
        clearance_id: sel.id,
        department: dept,
        signed_by: signedBy,
        comment,
        force,
      });
      if (!r.ok) setErr(r.blocked ? `Kimezuiwa: ${r.error}` : (r.error ?? "imeshindikana"));
      else {
        setMsg(`Idara ${dept} imesaini. Hali: ${r.clearance_status}`);
        await openOne(sel.id);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const reject = async () => {
    if (!sel) return;
    setBusy(true);
    try {
      const r = await clearanceService.reject({
        clearance_id: sel.id,
        department: dept,
        signed_by: signedBy,
        comment,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Sehemu imekataliwa.");
        await openOne(sel.id);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="clearance" className="flex flex-col gap-4">
      <EduCard testId="clr-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="flex items-center gap-2 text-sm font-semibold text-content">
            <FileCheck className="size-5 text-primary" aria-hidden />
            Clearance (Awamu 14)
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="clr-templates">{status?.templates ?? 0} template</EduBadge>
            <EduBadge testId="clr-pending">{status?.pending ?? 0} zinasubiri</EduBadge>
            <EduBadge testId="clr-cleared">{status?.cleared ?? 0} zimekamilika</EduBadge>
            <EduBadge testId="clr-rejected">{status?.rejected ?? 0} zimekataliwa</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          <span className="text-xs text-muted">Idara za msingi:</span>
          {(status?.core_departments ?? []).map((d) => (
            <EduBadge key={d.code} testId={`clr-core-${d.code}`}>
              {d.name}
            </EduBadge>
          ))}
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <div className="grid gap-4 md:grid-cols-2">
        <EduCard testId="clr-template">
          <span className="text-sm font-semibold text-content">Template mpya</span>
          <div className="mt-2 flex flex-col gap-2">
            <EduField label="Jina">
              <EduInput testId="clr-tpl-name" value={tplName} onChange={(e) => setTplName(e.target.value)} placeholder="Clearance 2026" />
            </EduField>
            <EduField label="Maandishi ya mhuri">
              <EduInput testId="clr-seal" value={sealText} onChange={(e) => setSealText(e.target.value)} placeholder="CHUO CHA MTECHOS" />
            </EduField>
            <div>
              <EduButton testId="clr-tpl-add" disabled={busy} onClick={() => void addTemplate()}>
                Unda template
              </EduButton>
            </div>
          </div>
        </EduCard>

        <EduCard testId="clr-request">
          <span className="text-sm font-semibold text-content">Omba clearance</span>
          <div className="mt-2 flex flex-col gap-2">
            <EduField label="Mwanafunzi (ID)">
              <EduInput testId="clr-student" value={studentId} onChange={(e) => setStudentId(e.target.value)} placeholder="ID ya mwanafunzi" />
            </EduField>
            <EduField label="Template">
              <EduSelect testId="clr-tpl-select" value={tplId} onChange={(e) => setTplId(e.target.value)}>
                {templates.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </EduSelect>
            </EduField>
            <EduField label="Mwaka wa masomo (hiari)">
              <EduInput testId="clr-year" value={year} onChange={(e) => setYear(e.target.value)} placeholder="2026" />
            </EduField>
            <div>
              <EduButton testId="clr-request-btn" disabled={busy} onClick={() => void doRequest()}>
                Omba
              </EduButton>
            </div>
          </div>
        </EduCard>
      </div>

      <EduCard testId="clr-list">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">Clearance ({list.length})</span>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted">Chuja kwa idara:</span>
            <EduSelect testId="clr-filter-dept" value={filterDept} onChange={(e) => setFilterDept(e.target.value)}>
              <option value="">Zote</option>
              <option value="library">Maktaba</option>
              <option value="finance">Fedha</option>
              <option value="hostel">Hosteli</option>
              <option value="dean">Mkuu wa Kitivo</option>
            </EduSelect>
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {list.length === 0 ? (
            <EduEmpty>Hakuna clearance bado.</EduEmpty>
          ) : (
            list.slice(0, 10).map((c) => (
              <div
                key={c.id}
                data-testid={`clr-item-${c.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {c.student_id} <span className="text-xs text-muted">· {c.academic_year}</span>
                </span>
                <div className="flex items-center gap-1">
                  {c.sections.map((s) => (
                    <EduBadge key={s.department_code} testId={`clr-sec-${c.id}-${s.department_code}`}>
                      {s.department_code}: {s.status}
                    </EduBadge>
                  ))}
                  <EduBadge testId={`clr-state-${c.id}`}>{c.status}</EduBadge>
                  <EduButton variant="ghost" testId={`clr-open-${c.id}`} onClick={() => void openOne(c.id)}>
                    Fungua
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      {sel ? (
        <EduCard testId="clr-detail">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="flex items-center gap-2 text-sm font-semibold text-content">
              <ShieldCheck className="size-4 text-primary" aria-hidden />
              {sel.student_id} · {sel.academic_year}
            </span>
            <EduBadge testId="clr-detail-state">{sel.status}</EduBadge>
          </div>
          <div className="mt-2 flex flex-col gap-1">
            {sel.sections.map((s) => (
              <div
                key={s.department_code}
                data-testid={`clr-detail-sec-${s.department_code}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {s.department_code}
                  {s.signed_by ? (
                    <span className="text-xs text-muted"> · {s.signed_by}</span>
                  ) : null}
                </span>
                <EduBadge testId={`clr-detail-sec-status-${s.department_code}`}>{s.status}</EduBadge>
              </div>
            ))}
          </div>

          <div className="mt-3 grid gap-2 md:grid-cols-4">
            <EduField label="Idara yako">
              <EduSelect testId="clr-dept" value={dept} onChange={(e) => setDept(e.target.value)}>
                <option value="library">Maktaba</option>
                <option value="finance">Fedha</option>
                <option value="hostel">Hosteli</option>
                <option value="dean">Mkuu wa Kitivo</option>
              </EduSelect>
            </EduField>
            <EduField label="Jina la msaini">
              <EduInput testId="clr-signed-by" value={signedBy} onChange={(e) => setSignedBy(e.target.value)} placeholder="Bw. Mkuu" />
            </EduField>
            <EduField label="Maoni">
              <EduInput testId="clr-comment" value={comment} onChange={(e) => setComment(e.target.value)} placeholder="Safi" />
            </EduField>
            <div className="flex items-end gap-1">
              <EduButton testId="clr-sign" disabled={busy} onClick={() => void sign(false)}>
                <Stamp className="size-4" aria-hidden /> Saini
              </EduButton>
              <EduButton variant="ghost" testId="clr-reject" disabled={busy} onClick={() => void reject()}>
                Kataa
              </EduButton>
            </div>
          </div>
          {sel.status === "cleared" ? (
            <p className="mt-2 flex items-center gap-2 text-xs text-success">
              <BadgeCheck className="size-4" aria-hidden /> Clearance imekamilika — mhuri: {sel.seal_text || "—"}
            </p>
          ) : null}
        </EduCard>
      ) : null}
    </div>
  );
}
