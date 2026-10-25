import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { GraduationCap, Send, Clock, CheckCircle2, LogIn } from "lucide-react";
import {
  educationService,
  type ExamJoinResult,
  type ExamStudentQuestion,
  type ExamSubmitResult,
} from "#/api/education-service/education-service.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduError,
  EduField,
  EduInput,
  EduSpinner,
  EduTextarea,
} from "#/components/features/education/edu-ui";

/**
 * Ukurasa wa MWANAFUNZI — hufunguliwa kwa QR code au code ya chumba.
 * Inafanya kazi kwenye simu/tablet/laptop, bila intaneti (LAN pekee).
 */
export function ExamStudentPage({ code }: { code: string }) {
  const [room, setRoom] = useState<{
    title: string;
    needs_passcode: boolean;
    status: string;
    duration_min: number;
  } | null>(null);
  const [loadErr, setLoadErr] = useState("");

  const [name, setName] = useState("");
  const [studentNo, setStudentNo] = useState("");
  const [passcode, setPasscode] = useState("");
  const [joins, setJoins] = useState<ExamJoinResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [idx, setIdx] = useState(0);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [submitted, setSubmitted] = useState<ExamSubmitResult | null>(null);
  const [secondsLeft, setSecondsLeft] = useState<number | null>(null);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    educationService
      .examRoomGet(code)
      .then((r) => {
        if (!r.ok) throw new Error(r.error ?? "chumba haipo");
        setRoom({
          title: r.title,
          needs_passcode: r.needs_passcode,
          status: r.status,
          duration_min: r.duration_min,
        });
      })
      .catch((e) => setLoadErr(e instanceof Error ? e.message : String(e)));
  }, [code]);

  useEffect(() => {
    if (joins && !submitted && room) setSecondsLeft(room.duration_min * 60);
  }, [joins, submitted, room]);

  const questions: ExamStudentQuestion[] = useMemo(
    () => joins?.questions ?? [],
    [joins],
  );

  const save = useCallback(
    async (index: number, answer: string, current: number) => {
      if (!joins?.token) return;
      try {
        await educationService.examRoomSave(code, {
          token: joins.token,
          index,
          answer,
          current,
        });
      } catch {
        /* jaribio linalofuata litahifadhi */
      }
    },
    [code, joins?.token],
  );

  const setAnswer = (index: number, value: string) => {
    setDrafts((d) => ({ ...d, [index]: value }));
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => void save(index, value, index + 1), 800);
  };

  const join = async () => {
    if (!name.trim()) {
      setError("Weka jina lako kamili.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      const r = await educationService.examRoomJoin(code, {
        name: name.trim(),
        student_no: studentNo.trim() || undefined,
        passcode: passcode.trim() || undefined,
      });
      if (!r.ok) throw new Error(r.error ?? "kujiunga kulishindwa");
      setJoins(r);
      setDrafts((r.answers as Record<string, string>) ?? {});
      setIdx(r.current ?? 0);
      if (r.status === "submitted") {
        setSubmitted({ ok: true, score: r.score, total: r.total });
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const submit = async () => {
    if (!joins?.token) return;
    setBusy(true);
    setError("");
    try {
      const r = await educationService.examRoomSubmit(code, {
        token: joins.token,
        answers: drafts,
      });
      if (!r.ok) throw new Error("kuwasilisha kulishindwa");
      setSubmitted(r);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  // Timer ya kuhesabu chini; muda unapoisha, hufanya submit kiotomatiki.
  useEffect(() => {
    if (!joins || submitted) return;
    const started = Date.now();
    const limitMs = (room?.duration_min ?? 60) * 60 * 1000;
    const t = setInterval(() => {
      const left = Math.max(0, Math.round((limitMs - (Date.now() - started)) / 1000));
      setSecondsLeft(left);
      if (left <= 0) {
        clearInterval(t);
        void submit();
      }
    }, 1000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [joins?.token, submitted, room?.duration_min]);

  if (loadErr) {
    return (
      <div className="mx-auto max-w-lg p-6">
        <EduError message={loadErr} />
      </div>
    );
  }
  if (!room) {
    return (
      <div className="mx-auto max-w-lg p-6">
        <EduSpinner label="Inapakia chumba cha mtihani…" />
      </div>
    );
  }

  if (submitted) {
    const pct = submitted.percent ?? 0;
    return (
      <div data-testid="exam-student-result" className="mx-auto max-w-lg p-6">
        <EduCard>
          <div className="mb-3 flex items-center gap-2">
            <CheckCircle2 className="size-6 text-feedback-success" />
            <h2 className="text-lg font-semibold text-content">Umewasilisha!</h2>
          </div>
          <div className="mb-4">
            <div className="text-4xl font-bold text-primary">{pct}%</div>
            <div className="text-sm text-muted">
              Alama: {submitted.score ?? 0}/{submitted.total ?? 0}
              {submitted.needs_review
                ? ` · maswali ${submitted.needs_review} yanahitaji mwalimu`
                : ""}
            </div>
          </div>
          {submitted.wrong && submitted.wrong.length > 0 ? (
            <div>
              <h3 className="mb-2 text-sm font-semibold text-content">
                Maswali ya kurejea
              </h3>
              <ul className="flex flex-col gap-2 text-xs">
                {submitted.wrong.map((w, i) => (
                  <li key={i} className="rounded-md border border-border bg-surface-raised p-2">
                    <div className="text-content">{w.question}</div>
                    <div className="mt-1 text-feedback-error">
                      jibu lako: {w.answer || "—"}
                    </div>
                    <div className="text-feedback-success">sahihi: {w.expected}</div>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </EduCard>
      </div>
    );
  }

  if (!joins) {
    return (
      <div className="mx-auto max-w-lg p-6">
        <EduCard>
          <div className="mb-3 flex items-center gap-2">
            <GraduationCap className="size-6 text-primary" />
            <div>
              <h2 className="text-lg font-semibold text-content">{room.title}</h2>
              <div className="text-xs text-muted">Chumba: {code}</div>
            </div>
          </div>
          <div className="grid gap-3">
            <EduField label="Jina lako kamili">
              <EduInput
                testId="exam-join-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="mf. Asha Juma"
              />
            </EduField>
            <EduField label="Namba ya usajili (hiari)">
              <EduInput
                testId="exam-join-no"
                value={studentNo}
                onChange={(e) => setStudentNo(e.target.value)}
                placeholder="mf. S001"
              />
            </EduField>
            {room.needs_passcode ? (
              <EduField label="Code ya siri">
                <EduInput
                  testId="exam-join-passcode"
                  value={passcode}
                  onChange={(e) => setPasscode(e.target.value)}
                />
              </EduField>
            ) : null}
          </div>
          {error ? (
            <p className="mt-2 text-sm text-feedback-error">{error}</p>
          ) : null}
          <EduButton
            testId="exam-join-start"
            className="mt-3"
            onClick={join}
            disabled={busy}
          >
            <LogIn className="size-4" />
            {busy ? "Inaingia…" : "Anza Mtihani"}
          </EduButton>
        </EduCard>
      </div>
    );
  }

  const q = questions[idx];
  const answered = Object.keys(drafts).filter((k) => drafts[k]?.trim()).length;
  return (
    <div data-testid="exam-student-running" className="mx-auto max-w-2xl p-4">
      <div className="mb-3 flex items-center justify-between">
        <div className="text-sm font-semibold text-content">{room.title}</div>
        <div className="flex items-center gap-2">
          {secondsLeft !== null ? (
            <span className="flex items-center gap-1 text-sm text-content">
              <Clock className="size-4" />
              {Math.floor((secondsLeft ?? 0) / 60)}:
              {String((secondsLeft ?? 0) % 60).padStart(2, "0")}
            </span>
          ) : null}
          <EduBadge>
            {answered}/{questions.length}
          </EduBadge>
        </div>
      </div>

      <EduCard>
        {q ? (
          <>
            <div className="mb-1 text-xs text-muted">
              {q.section} · swali {idx + 1} kati ya {questions.length}
            </div>
            <div className="mb-3 whitespace-pre-wrap text-content">{q.question}</div>
            {q.marks ? (
              <div className="mb-2">
                <EduBadge>alama {q.marks}</EduBadge>
              </div>
            ) : null}
            <EduTextarea
              testId="exam-student-answer"
              value={drafts[idx] ?? ""}
              onChange={(e) => setAnswer(idx, e.target.value)}
              placeholder="Andika jibu lako hapa…"
            />
          </>
        ) : (
          <div className="text-sm text-muted">Hakuna maswali.</div>
        )}
      </EduCard>

      <div className="mt-3 flex items-center justify-between gap-2">
        <EduButton
          variant="ghost"
          onClick={() => setIdx((i) => Math.max(0, i - 1))}
          disabled={idx === 0}
        >
          Nyuma
        </EduButton>
        <div className="flex gap-2">
          <EduButton
            variant="ghost"
            onClick={() => setIdx((i) => Math.min(questions.length - 1, i + 1))}
            disabled={idx >= questions.length - 1}
          >
            Mbele
          </EduButton>
          <EduButton testId="exam-student-submit" onClick={submit} disabled={busy}>
            <Send className="size-4" />
            Wasilisha
          </EduButton>
        </div>
      </div>
      {error ? <p className="mt-2 text-sm text-feedback-error">{error}</p> : null}
    </div>
  );
}
