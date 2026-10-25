import { useCallback, useEffect, useState } from "react";
import { Award, FileCheck2, RefreshCw, Trophy } from "lucide-react";
import {
  resultsService,
  type Certificate,
  type LevelResult,
  type ResultsStatus,
  type StudentResults,
} from "#/api/education-service/results.api";
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

/**
 * ELIMUPRO — Awamu 11: matokeo, transkripti na vyeti.
 * Matokeo huchukua alama za Awamu 6 (kwa uzito), huzalisha GPA + daraja,
 * na vyeti hupata serial + kodi ya kuthibitisha.
 */
export function ResultsTab() {
  const [status, setStatus] = useState<ResultsStatus | null>(null);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [students, setStudents] = useState<EpStudent[]>([]);
  const [ranked, setRanked] = useState<LevelResult[]>([]);
  const [current, setCurrent] = useState<StudentResults | null>(null);
  const [certs, setCerts] = useState<Certificate[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [vSerial, setVSerial] = useState("");
  const [vCode, setVCode] = useState("");
  const [verifyResult, setVerifyResult] = useState<{ valid: boolean; text: string } | null>(null);

  const refresh = useCallback(async () => {
    try {
      setStatus(await resultsService.status());
      if (levelId) {
        setRanked((await resultsService.level(levelId)).results);
        setStudents((await elimuproService.students(levelId)).students);
      }
      setCerts((await resultsService.certificates({})).certificates);
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

  const loadStudent = async (id: string) => {
    try {
      setCurrent(await resultsService.transcript(id));
    } catch (e) {
      setErr(String(e));
    }
  };

  const issueTranscript = async (studentId: string) => {
    setBusy(true);
    try {
      const r = await resultsService.issueTranscript({ student_id: studentId });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Transkripti imetolewa (GPA ${r.gpa}, ${r.classification}).`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const issueCertificate = async (studentId: string, kind: string) => {
    setBusy(true);
    try {
      const r = await resultsService.issueCertificate({ student_id: studentId, kind });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Cheti ${r.serial} imetolewa. Kodi: ${r.verify_code}`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doVerify = async () => {
    try {
      const r = await resultsService.verify(vSerial, vCode);
      setVerifyResult({
        valid: r.valid,
        text: r.valid
          ? `Halali — ${r.certificate?.student} (${r.certificate?.title})`
          : r.error ?? "Cheti hakipatikani",
      });
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div data-testid="results" className="flex flex-col gap-4">
      <EduCard testId="res-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Trophy className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">Matokeo + Vyeti (Awamu 11)</span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="res-marks">{status?.marks ?? 0} alama</EduBadge>
            <EduBadge>{status?.students_with_marks ?? 0} wanafunzi</EduBadge>
            <EduBadge testId="res-certs">{status?.certificates ?? 0} vyeti</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        {status ? (
          <div className="mt-2 flex flex-wrap gap-1">
            {status.grade_scale.map((g) => (
              <EduBadge key={g.letter}>
                {g.letter} ≥{g.min} ({g.points.toFixed(1)})
              </EduBadge>
            ))}
          </div>
        ) : null}
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard>
        <EduField label="Level">
          <EduSelect testId="res-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
            {levels.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
      </EduCard>

      <EduCard testId="res-ranked">
        <span className="text-sm font-semibold text-content">Orodha ya matokeo ({ranked.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {ranked.length === 0 ? (
            <EduEmpty>Hakuna matokeo bado.</EduEmpty>
          ) : (
            ranked.map((r) => (
              <div key={r.student_id} data-testid={`res-row-${r.student_id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  #{r.rank} {r.student} <span className="text-xs text-muted">({r.reg_no})</span>
                </span>
                <div className="flex flex-wrap items-center gap-1">
                  <EduBadge testId={`res-avg-${r.student_id}`}>wastani {r.average}</EduBadge>
                  <EduBadge testId={`res-gpa-${r.student_id}`}>GPA {r.gpa}</EduBadge>
                  <EduBadge testId={`res-class-${r.student_id}`}>{r.classification}</EduBadge>
                  <EduButton variant="ghost" testId={`res-view-${r.student_id}`} onClick={() => void loadStudent(r.student_id)}>
                    Onyesha
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      {current ? (
        <EduCard testId="res-student">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="text-sm font-semibold text-content">
              {current.student.name} ({current.student.reg_no})
            </span>
            <div className="flex flex-wrap items-center gap-1">
              <EduBadge testId="res-student-gpa">GPA {current.gpa}</EduBadge>
              <EduBadge testId="res-student-class">{current.classification}</EduBadge>
              <EduButton variant="ghost" testId="res-issue-transcript" disabled={busy} onClick={() => void issueTranscript(current.student.id)}>
                <FileCheck2 className="size-3.5" aria-hidden /> Toa transkripti
              </EduButton>
              <EduButton variant="ghost" testId="res-issue-cert" disabled={busy} onClick={() => void issueCertificate(current.student.id, 'completion')}>
                <Award className="size-3.5" aria-hidden /> Toa cheti
              </EduButton>
            </div>
          </div>
          <div className="mt-2 flex flex-col gap-1">
            {current.subjects.map((su) => (
              <div key={su.subject_id} data-testid={`res-sub-${su.subject_id}`} className="flex items-center justify-between rounded-md border border-border p-2">
                <span className="text-sm text-content">{su.subject}</span>
                <div className="flex gap-1">
                  <EduBadge>{su.score === null ? "—" : su.score}</EduBadge>
                  <EduBadge>{su.letter}</EduBadge>
                  <EduBadge>{su.points.toFixed(1)} pts</EduBadge>
                </div>
              </div>
            ))}
          </div>
        </EduCard>
      ) : null}

      <EduCard testId="res-certs">
        <span className="text-sm font-semibold text-content">Vyeti ({certs.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {certs.length === 0 ? (
            <EduEmpty>Hakuna vyeti bado.</EduEmpty>
          ) : (
            certs.slice(0, 12).map((c) => (
              <div key={c.id} data-testid={`res-cert-${c.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {c.student} — {c.title}
                </span>
                <div className="flex flex-wrap items-center gap-1">
                  <EduBadge>{c.kind}</EduBadge>
                  <EduBadge testId={`res-cert-serial-${c.id}`}>{c.serial}</EduBadge>
                  <EduBadge testId={`res-cert-code-${c.id}`}>{c.verify_code}</EduBadge>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="res-verify">
        <span className="text-sm font-semibold text-content">Thibitisha cheti</span>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Serial">
            <EduInput testId="res-verify-serial" value={vSerial} onChange={(e) => setVSerial(e.target.value)} />
          </EduField>
          <EduField label="Kodi">
            <EduInput testId="res-verify-code" value={vCode} onChange={(e) => setVCode(e.target.value)} />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="res-verify-btn" disabled={!vSerial || !vCode} onClick={() => void doVerify()}>
              Thibitisha
            </EduButton>
          </div>
        </div>
        {verifyResult ? (
          <p data-testid="res-verify-result" className={`mt-2 text-xs ${verifyResult.valid ? "text-success" : "text-danger"}`}>
            {verifyResult.valid ? "✓ " : "✗ "}
            {verifyResult.text}
          </p>
        ) : null}
      </EduCard>

      <EduCard>
        <span className="text-sm font-semibold text-content">Wanafunzi</span>
        <div className="mt-2 flex flex-col gap-1">
          {students.length === 0 ? (
            <EduEmpty>Hakuna wanafunzi kwenye level hii.</EduEmpty>
          ) : (
            students.map((st) => (
              <div key={st.id} className="flex items-center justify-between rounded-md border border-border p-2">
                <span className="text-sm text-content">{st.full_name}</span>
                <EduButton variant="ghost" testId={`res-load-${st.id}`} onClick={() => void loadStudent(st.id)}>
                  Matokeo
                </EduButton>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
