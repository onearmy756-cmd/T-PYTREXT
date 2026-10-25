import { useCallback, useEffect, useState } from "react";
import { Database, FlaskConical, Play, RefreshCw, Shield, Terminal } from "lucide-react";
import {
  itToolsService,
  type DbQueryResult,
  type ItLab,
  type ItSession,
  type ItStatus,
  type ItSubmission,
} from "#/api/education-service/ittools.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
  EduTextarea,
} from "./edu-ui";

/**
 * ELIMUPRO — Awamu 16: zana za IT (moduli 16).
 * Code editor, terminal, cyber lab, database tools na ufuatiliaji wa mwalimu.
 */
export function ItToolsTab() {
  const [status, setStatus] = useState<ItStatus | null>(null);
  const [sessions, setSessions] = useState<ItSession[]>([]);
  const [subs, setSubs] = useState<ItSubmission[]>([]);
  const [labs, setLabs] = useState<ItLab[]>([]);
  const [schema, setSchema] = useState<{ table: string; columns: { name: string; type: string }[] }[]>([]);
  const [monitor, setMonitor] = useState<{
    live: { id: string; student_id: string; kind: string; language: string }[];
    recent: { student_id: string; language: string; feedback: string; ok: boolean; score: number }[];
  }>({ live: [], recent: [] });
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [studentId, setStudentId] = useState("");
  const [language, setLanguage] = useState("python");
  const [kind, setKind] = useState("editor");
  const [code, setCode] = useState('print("Habari MTECHOS")');
  const [expect, setExpect] = useState("");
  const [out, setOut] = useState("");
  const [sql, setSql] = useState("SELECT * FROM wanafunzi");
  const [dbRes, setDbRes] = useState<DbQueryResult | null>(null);
  const [labTitle, setLabTitle] = useState("");
  const [labExpected, setLabExpected] = useState("");
  const [labCat, setLabCat] = useState("cyber");
  const [labPoints, setLabPoints] = useState("10");
  const [labId, setLabId] = useState("");
  const [labAnswer, setLabAnswer] = useState("");
  const [sessionId, setSessionId] = useState("");

  const refresh = useCallback(async () => {
    try {
      setStatus(await itToolsService.status());
      setSessions((await itToolsService.sessions()).sessions);
      setSubs((await itToolsService.submissions()).submissions);
      const l = (await itToolsService.labs()).labs;
      setLabs(l);
      if (!labId && l.length > 0) setLabId(l[0].id);
      setSchema((await itToolsService.dbSchema()).tables);
      setMonitor(await itToolsService.monitor());
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [labId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const startSession = async () => {
    if (!studentId.trim()) {
      setErr("Weka kitambulisho cha mwanafunzi");
      return;
    }
    setBusy(true);
    try {
      const r = await itToolsService.startSession({ student_id: studentId, kind, language });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setSessionId(r.id ?? "");
        setMsg(`Kikao kimeanza (${r.kind}).`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runCode = async () => {
    setBusy(true);
    try {
      const r = await itToolsService.run({
        code,
        language,
        session_id: sessionId,
        student_id: studentId,
      });
      const res = r.result;
      setOut(
        `exit=${res.exit}\n${res.stdout ?? ""}${res.stderr ? `\n[stderr]\n${res.stderr}` : ""}`,
      );
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const submitCode = async () => {
    if (!studentId.trim()) {
      setErr("Weka kitambulisho cha mwanafunzi");
      return;
    }
    setBusy(true);
    try {
      const r = await itToolsService.submit({
        student_id: studentId,
        code,
        language,
        session_id: sessionId,
        expect_stdout: expect,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Kazi: ${r.passed ? "IMEFAULU" : "HAIJAPA"} · alama ${r.score} · ${r.feedback}`);
        setOut(`exit=${r.exit}\n${r.stdout ?? ""}`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runSql = async () => {
    setBusy(true);
    try {
      setDbRes(await itToolsService.dbQuery(sql));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addLab = async () => {
    setBusy(true);
    try {
      const r = await itToolsService.addLab({
        title: labTitle,
        category: labCat,
        expected: labExpected,
        points: Number(labPoints) || 10,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Lab imeundwa.");
        setLabTitle("");
        setLabExpected("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const attemptLab = async () => {
    if (!labId || !studentId.trim()) {
      setErr("Weka mwanafunzi na chagua lab");
      return;
    }
    setBusy(true);
    try {
      const r = await itToolsService.attemptLab({ lab_id: labId, student_id: studentId, answer: labAnswer });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(r.correct ? `Sahihi! +${r.points} pointi` : "Jibu si sahihi.");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="ittools" className="flex flex-col gap-4">
      <EduCard testId="it-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="flex items-center gap-2 text-sm font-semibold text-content">
            <Terminal className="size-5 text-primary" aria-hidden />
            Zana za IT (Awamu 16)
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="it-active">{status?.sessions_active ?? 0} vikao hai</EduBadge>
            <EduBadge testId="it-subs">{status?.submissions ?? 0} kazi</EduBadge>
            <EduBadge testId="it-subs-ok">{status?.submissions_ok ?? 0} zilizofaulu</EduBadge>
            <EduBadge testId="it-labs">{status?.labs ?? 0} lab</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          {(status?.languages ?? []).map((l) => (
            <EduBadge key={l} testId={`it-lang-${l}`}>
              {l}
            </EduBadge>
          ))}
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="it-session">
        <span className="text-sm font-semibold text-content">Kikao cha mwanafunzi</span>
        <div className="mt-2 grid gap-2 md:grid-cols-5">
          <EduField label="Mwanafunzi">
            <EduInput testId="it-student" value={studentId} onChange={(e) => setStudentId(e.target.value)} placeholder="ID ya mwanafunzi" />
          </EduField>
          <EduField label="Aina">
            <EduSelect testId="it-kind" value={kind} onChange={(e) => setKind(e.target.value)}>
              <option value="editor">Code editor</option>
              <option value="terminal">Terminal</option>
              <option value="db">Database</option>
              <option value="cyber">Cyber lab</option>
            </EduSelect>
          </EduField>
          <EduField label="Lugha">
            <EduSelect testId="it-language" value={language} onChange={(e) => setLanguage(e.target.value)}>
              {(status?.languages ?? ["python"]).map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <div className="flex items-end">
            <EduButton testId="it-start" disabled={busy} onClick={() => void startSession()}>
              Anza kikao
            </EduButton>
          </div>
          <div className="flex items-end">
            <EduBadge testId="it-session-id">{sessionId || "—"}</EduBadge>
          </div>
        </div>
      </EduCard>

      <EduCard testId="it-editor">
        <span className="flex items-center gap-2 text-sm font-semibold text-content">
          <Play className="size-4" aria-hidden /> Code editor + terminal
        </span>
        <div className="mt-2 flex flex-col gap-2">
          <EduTextarea
            testId="it-code"
            value={code}
            rows={6}
            onChange={(e) => setCode(e.target.value)}
            placeholder="Andika code hapa..."
          />
          <div className="flex flex-wrap items-end gap-2">
            <EduField label="Matokeo yanayotarajiwa (stdout)">
              <EduInput testId="it-expect" value={expect} onChange={(e) => setExpect(e.target.value)} placeholder="Habari MTECHOS" />
            </EduField>
            <EduButton testId="it-run" disabled={busy} onClick={() => void runCode()}>
              Kimiza
            </EduButton>
            <EduButton testId="it-submit" disabled={busy} onClick={() => void submitCode()}>
              Wasilisha
            </EduButton>
          </div>
          {out ? (
            <pre data-testid="it-out" className="max-h-40 overflow-auto rounded-md bg-surface-secondary p-2 text-xs text-content">
              {out}
            </pre>
          ) : null}
        </div>
      </EduCard>

      <div className="grid gap-4 md:grid-cols-2">
        <EduCard testId="it-db">
          <span className="flex items-center gap-2 text-sm font-semibold text-content">
            <Database className="size-4" aria-hidden /> Database tools (SELECT pekee)
          </span>
          <div className="mt-2 flex flex-col gap-2">
            <EduTextarea testId="it-sql" value={sql} rows={3} onChange={(e) => setSql(e.target.value)} />
            <div className="flex flex-wrap gap-1">
              <EduButton testId="it-sql-run" disabled={busy} onClick={() => void runSql()}>
                Tekeleza
              </EduButton>
              {schema.map((t) => (
                <EduBadge key={t.table} testId={`it-table-${t.table}`}>
                  {t.table}({t.columns.length})
                </EduBadge>
              ))}
            </div>
            {dbRes ? (
              dbRes.ok ? (
                <div data-testid="it-db-res" className="max-h-40 overflow-auto text-xs text-content">
                  <div className="text-muted">{dbRes.count} safu</div>
                  <table className="w-full">
                    <thead>
                      <tr>
                        {(dbRes.columns ?? []).map((c) => (
                          <th key={c} className="border-b border-border p-1 text-left">
                            {c}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {(dbRes.rows ?? []).map((row, i) => (
                        <tr key={i}>
                          {row.map((v, j) => (
                            <td key={j} className="border-b border-border p-1">
                              {String(v)}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p data-testid="it-db-err" className="text-xs text-warning">
                  {dbRes.error}
                </p>
              )
            ) : null}
          </div>
        </EduCard>

        <EduCard testId="it-cyber">
          <span className="flex items-center gap-2 text-sm font-semibold text-content">
            <Shield className="size-4" aria-hidden /> Cyber security lab
          </span>
          <div className="mt-2 grid gap-2 md:grid-cols-2">
            <EduField label="Kichwa">
              <EduInput testId="it-lab-title" value={labTitle} onChange={(e) => setLabTitle(e.target.value)} placeholder="SQL Injection" />
            </EduField>
            <EduField label="Kundi">
              <EduSelect testId="it-lab-cat" value={labCat} onChange={(e) => setLabCat(e.target.value)}>
                <option value="cyber">Cyber</option>
                <option value="network">Network</option>
                <option value="forensics">Forensics</option>
                <option value="crypto">Crypto</option>
              </EduSelect>
            </EduField>
            <EduField label="Jibu sahihi">
              <EduInput testId="it-lab-expected" value={labExpected} onChange={(e) => setLabExpected(e.target.value)} placeholder="flag{sqli}" />
            </EduField>
            <EduField label="Pointi">
              <EduInput testId="it-lab-points" value={labPoints} onChange={(e) => setLabPoints(e.target.value)} />
            </EduField>
          </div>
          <div className="mt-2">
            <EduButton testId="it-lab-add" disabled={busy} onClick={() => void addLab()}>
              <FlaskConical className="size-4" aria-hidden /> Unda lab
            </EduButton>
          </div>

          <div className="mt-3 grid gap-2 md:grid-cols-3">
            <EduField label="Lab">
              <EduSelect testId="it-lab-select" value={labId} onChange={(e) => setLabId(e.target.value)}>
                {labs.map((l) => (
                  <option key={l.id} value={l.id}>
                    {l.title} ({l.points})
                  </option>
                ))}
              </EduSelect>
            </EduField>
            <EduField label="Jibu lako">
              <EduInput testId="it-lab-answer" value={labAnswer} onChange={(e) => setLabAnswer(e.target.value)} placeholder="flag{...}" />
            </EduField>
            <div className="flex items-end">
              <EduButton testId="it-lab-attempt" disabled={busy} onClick={() => void attemptLab()}>
                Tuma jibu
              </EduButton>
            </div>
          </div>
        </EduCard>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <EduCard testId="it-sessions">
          <span className="text-sm font-semibold text-content">Vikao ({sessions.length})</span>
          <div className="mt-2 flex flex-col gap-1">
            {sessions.length === 0 ? (
              <EduEmpty>Hakuna kikao.</EduEmpty>
            ) : (
              sessions.slice(0, 8).map((s) => (
                <div
                  key={s.id}
                  data-testid={`it-session-${s.id}`}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
                >
                  <span className="text-sm text-content">
                    {s.student_id} <span className="text-xs text-muted">· {s.kind} · {s.language}</span>
                  </span>
                  <div className="flex items-center gap-1">
                    <EduBadge testId={`it-session-status-${s.id}`}>{s.status}</EduBadge>
                    {s.live ? (
                      <EduButton variant="ghost" testId={`it-session-end-${s.id}`} onClick={() => void itToolsService.endSession(s.id).then(refresh)}>
                        Funga
                      </EduButton>
                    ) : null}
                  </div>
                </div>
              ))
            )}
          </div>
        </EduCard>

        <EduCard testId="it-subs">
          <span className="text-sm font-semibold text-content">Kazi zilizowasilishwa ({subs.length})</span>
          <div className="mt-2 flex flex-col gap-1">
            {subs.length === 0 ? (
              <EduEmpty>Hakuna kazi.</EduEmpty>
            ) : (
              subs.slice(0, 8).map((s) => (
                <div
                  key={s.id}
                  data-testid={`it-sub-${s.id}`}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
                >
                  <span className="text-sm text-content">
                    {s.student_id} <span className="text-xs text-muted">· {s.feedback}</span>
                  </span>
                  <div className="flex items-center gap-1">
                    <EduBadge testId={`it-sub-score-${s.id}`}>{s.score}</EduBadge>
                    <EduBadge testId={`it-sub-ok-${s.id}`}>{s.ok ? "pass" : "fail"}</EduBadge>
                  </div>
                </div>
              ))
            )}
          </div>
        </EduCard>
      </div>

      <EduCard testId="it-monitor">
        <span className="text-sm font-semibold text-content">Ufuatiliaji wa mwalimu</span>
        <div className="mt-2 flex flex-wrap gap-1">
          {monitor.live.length === 0 ? (
            <EduEmpty>Hakuna kikao hai.</EduEmpty>
          ) : (
            monitor.live.map((l) => (
              <EduBadge key={l.id} testId={`it-live-${l.id}`}>
                {l.student_id} · {l.kind}
              </EduBadge>
            ))
          )}
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {monitor.recent.slice(0, 5).map((r, i) => (
            <span key={i} className="text-xs text-muted">
              {r.student_id} · {r.language} · {r.feedback} ({r.score})
            </span>
          ))}
        </div>
      </EduCard>
    </div>
  );
}
