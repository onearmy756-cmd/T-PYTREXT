import { useCallback, useEffect, useState } from "react";
import { Brain, Flag, RefreshCw, Sparkles } from "lucide-react";
import {
  aiEngineService,
  type AiFlag,
  type AiStatus,
  type AntiCheatResult,
} from "#/api/education-service/aiengine.api";
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
 * ELIMUPRO — Awamu 17: injini ya AI (moduli 17).
 * AI Scribe, Lesson Gen, Anti-Cheat, Tutor — AI husaidia, haichukui nafasi.
 */
export function AiEngineTab() {
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [flags, setFlags] = useState<AiFlag[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);
  const [mode, setMode] = useState("");

  const [scribeText, setScribeText] = useState("");
  const [subject, setSubject] = useState("");
  const [lessonNotes, setLessonNotes] = useState("");
  const [level, setLevel] = useState("Chuo / Chuo Kikuu");
  const [cheatText, setCheatText] = useState("");
  const [cheatKind, setCheatKind] = useState("assignment");
  const [cheat, setCheat] = useState<AntiCheatResult | null>(null);
  const [question, setQuestion] = useState("");
  const [tutorNotes, setTutorNotes] = useState("");
  const [out, setOut] = useState("");
  const [reviewer, setReviewer] = useState("");

  const refresh = useCallback(async () => {
    try {
      setStatus(await aiEngineService.status());
      setFlags((await aiEngineService.flags()).flags);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runScribe = async () => {
    setBusy(true);
    try {
      const r = await aiEngineService.scribe({ transcript: scribeText, subject });
      setMode(r.mode);
      setOut(typeof r.result === "string" ? r.result : JSON.stringify(r.result, null, 2));
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runLesson = async () => {
    setBusy(true);
    try {
      const r = await aiEngineService.lesson({ notes: lessonNotes, subject, level });
      setMode(r.mode);
      setOut(typeof r.result === "string" ? r.result : JSON.stringify(r.result, null, 2));
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runCheat = async () => {
    setBusy(true);
    try {
      const r = await aiEngineService.anticheat({ text: cheatText, kind: cheatKind });
      setCheat(r);
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runTutor = async () => {
    setBusy(true);
    try {
      const r = await aiEngineService.tutor({ question, notes: tutorNotes, subject });
      setMode(r.mode);
      setOut(typeof r.result === "string" ? r.result : JSON.stringify(r.result, null, 2));
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="aiengine" className="flex flex-col gap-4">
      <EduCard testId="ai-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="flex items-center gap-2 text-sm font-semibold text-content">
            <Brain className="size-5 text-primary" aria-hidden />
            Injini ya AI (Awamu 17)
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="ai-jobs">{status?.jobs ?? 0} kazi</EduBadge>
            <EduBadge testId="ai-jobs-llm">{status?.jobs_llm ?? 0} LLM</EduBadge>
            <EduBadge testId="ai-jobs-offline">{status?.jobs_offline ?? 0} offline</EduBadge>
            <EduBadge testId="ai-flags">{status?.flags ?? 0} flag</EduBadge>
            <EduBadge testId="ai-flags-unrev">{status?.flags_unreviewed ?? 0} hazijakaguliwa</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          {Object.entries(status?.capabilities ?? {}).map(([k, v]) => (
            <EduBadge key={k} testId={`ai-cap-${k}`}>
              {k}: {v ? "ndiyo" : "hapana"}
            </EduBadge>
          ))}
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="ai-scribe">
        <span className="flex items-center gap-2 text-sm font-semibold text-content">
          <Sparkles className="size-4" aria-hidden /> AI Scribe
        </span>
        <div className="mt-2 flex flex-col gap-2">
          <EduTextarea testId="ai-scribe-text" value={scribeText} rows={4} onChange={(e) => setScribeText(e.target.value)} placeholder="Weka maandishi ya mhadhara..." />
          <div className="flex flex-wrap items-end gap-2">
            <EduField label="Somo (hiari)">
              <EduInput testId="ai-subject" value={subject} onChange={(e) => setSubject(e.target.value)} placeholder="Fizikia" />
            </EduField>
            <EduButton testId="ai-scribe-run" disabled={busy} onClick={() => void runScribe()}>
              Changanua
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard testId="ai-lesson">
        <span className="text-sm font-semibold text-content">AI Lesson Gen</span>
        <div className="mt-2 flex flex-col gap-2">
          <EduTextarea testId="ai-lesson-notes" value={lessonNotes} rows={4} onChange={(e) => setLessonNotes(e.target.value)} placeholder="Weka notes (maandishi)..." />
          <div className="flex flex-wrap items-end gap-2">
            <EduField label="Kiwango">
              <EduSelect testId="ai-level" value={level} onChange={(e) => setLevel(e.target.value)}>
                <option value="Msingi (Shule ya Msingi)">Msingi</option>
                <option value="Sekondari">Sekondari</option>
                <option value="Chuo / Chuo Kikuu">Chuo</option>
              </EduSelect>
            </EduField>
            <EduButton testId="ai-lesson-run" disabled={busy} onClick={() => void runLesson()}>
              Tengeneza somo
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard testId="ai-anticheat">
        <span className="flex items-center gap-2 text-sm font-semibold text-content">
          <Flag className="size-4" aria-hidden /> AI Anti-Cheat
        </span>
        <div className="mt-2 flex flex-col gap-2">
          <EduTextarea testId="ai-cheat-text" value={cheatText} rows={4} onChange={(e) => setCheatText(e.target.value)} placeholder="Weka kazi ya mwanafunzi..." />
          <div className="flex flex-wrap items-end gap-2">
            <EduField label="Aina">
              <EduSelect testId="ai-cheat-kind" value={cheatKind} onChange={(e) => setCheatKind(e.target.value)}>
                <option value="assignment">Assignment</option>
                <option value="exam">Mtihani</option>
              </EduSelect>
            </EduField>
            <EduButton testId="ai-cheat-run" disabled={busy} onClick={() => void runCheat()}>
              Chunguza
            </EduButton>
          </div>
          {cheat ? (
            <div data-testid="ai-cheat-res" className="rounded-md border border-border p-2">
              <div className="flex flex-wrap items-center gap-1">
                <EduBadge testId="ai-cheat-score">alama {cheat.score}</EduBadge>
                <EduBadge testId="ai-cheat-risk">hatari: {cheat.risk}</EduBadge>
                <EduBadge testId="ai-cheat-flagged">{cheat.flagged ? "imeashiriwa" : "safi"}</EduBadge>
              </div>
              {cheat.reasons.length > 0 ? (
                <ul className="mt-1 list-inside list-disc text-xs text-muted">
                  {cheat.reasons.map((r, i) => (
                    <li key={i} data-testid={`ai-cheat-reason-${i}`}>
                      {r}
                    </li>
                  ))}
                </ul>
              ) : null}
              <p className="mt-1 text-xs text-muted">{cheat.note}</p>
            </div>
          ) : null}
        </div>
      </EduCard>

      <EduCard testId="ai-tutor">
        <span className="text-sm font-semibold text-content">AI Tutor</span>
        <div className="mt-2 flex flex-col gap-2">
          <EduField label="Swali">
            <EduInput testId="ai-question" value={question} onChange={(e) => setQuestion(e.target.value)} placeholder="Kwa nini maji huchemka?" />
          </EduField>
          <EduTextarea testId="ai-tutor-notes" value={tutorNotes} rows={3} onChange={(e) => setTutorNotes(e.target.value)} placeholder="Notes za kumbukumbu (hiari)..." />
          <div>
            <EduButton testId="ai-tutor-run" disabled={busy} onClick={() => void runTutor()}>
              Uliza
            </EduButton>
          </div>
        </div>
      </EduCard>

      {out ? (
        <EduCard testId="ai-out">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold text-content">Matokeo</span>
            <EduBadge testId="ai-out-mode">mode: {mode || "—"}</EduBadge>
          </div>
          <pre data-testid="ai-out-text" className="mt-2 max-h-60 overflow-auto rounded-md bg-surface-secondary p-2 text-xs text-content">
            {out}
          </pre>
        </EduCard>
      ) : null}

      <EduCard testId="ai-flags-list">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">Flag za uchunguzi ({flags.length})</span>
          <EduField label="Mkaguzi">
            <EduInput testId="ai-reviewer" value={reviewer} onChange={(e) => setReviewer(e.target.value)} placeholder="jina la mwalimu" />
          </EduField>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {flags.length === 0 ? (
            <EduEmpty>Hakuna flag.</EduEmpty>
          ) : (
            flags.slice(0, 10).map((f) => (
              <div
                key={f.id}
                data-testid={`ai-flag-${f.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {f.kind} · {f.ref_id || "—"}{" "}
                  <span className="text-xs text-muted">({f.reasons.join("; ")})</span>
                </span>
                <div className="flex items-center gap-1">
                  <EduBadge testId={`ai-flag-score-${f.id}`}>{f.score}</EduBadge>
                  <EduBadge testId={`ai-flag-risk-${f.id}`}>{f.risk}</EduBadge>
                  {f.reviewed ? (
                    <EduBadge testId={`ai-flag-rev-${f.id}`}>imekaguliwa</EduBadge>
                  ) : (
                    <EduButton
                      variant="ghost"
                      testId={`ai-flag-review-${f.id}`}
                      onClick={() => void aiEngineService.reviewFlag(f.id, reviewer).then(refresh)}
                    >
                      Kagua
                    </EduButton>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
