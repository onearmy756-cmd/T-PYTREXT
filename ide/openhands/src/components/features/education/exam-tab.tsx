import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Camera,
  CheckCircle2,
  ClipboardList,
  Eye,
  FileText,
  Plus,
  RefreshCw,
  Send,
  ShieldCheck,
  SquarePlay,
} from "lucide-react";
import {
  proctorService,
  type PcAttempt,
  type PcExam,
  type PcQuestion,
  type PcStatus,
} from "#/api/education-service/proctor.api";
import { elimuproService, type EpLevel, type EpSubject } from "#/api/education-service/elimupro.api";
import { aiStyleSuspect, useProctor } from "./use-proctor";
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
 * ELIMUPRO — Awamu 5: mitihani + ulinzi mkali (proctoring).
 * Mwalimu: kuunda mtihani, maswali, kuchapisha, matokeo + ukiukaji.
 * Mwanafunzi: sehemu mbili (maswali kushoto, majibu kulia), jibu hufungwa,
 * camera/mic huwashwa, copy/paste/devtools huzuiwa na kuripotiwa.
 */
export function ExamTab() {
  const [status, setStatus] = useState<PcStatus | null>(null);
  const [exams, setExams] = useState<PcExam[]>([]);
  const [open, setOpen] = useState<PcExam | null>(null);
  const [sitting, setSitting] = useState<{ exam: PcExam; attemptId: string } | null>(null);
  const [err, setErr] = useState("");

  const refresh = useCallback(async () => {
    try {
      setStatus(await proctorService.status());
      setExams((await proctorService.exams()).exams);
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (sitting) {
    return (
      <Sitting
        exam={sitting.exam}
        attemptId={sitting.attemptId}
        onDone={() => {
          setSitting(null);
          void refresh();
        }}
      />
    );
  }
  if (open) {
    return (
      <ExamDetail
        exam={open}
        onBack={() => {
          setOpen(null);
          void refresh();
        }}
        onSit={(attemptId) => setSitting({ exam: open, attemptId })}
      />
    );
  }

  return (
    <div data-testid="exams" className="flex flex-col gap-4">
      <EduCard testId="exam-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <ShieldCheck className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">
              Mitihani + ulinzi (Awamu 5)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge>{status?.exams ?? 0} mitihani</EduBadge>
            <EduBadge>{status?.published ?? 0} tayari</EduBadge>
            <EduBadge>{status?.attempts ?? 0} wafanya</EduBadge>
            <EduBadge>{status?.active ?? 0} wanaendelea</EduBadge>
            <EduBadge>{status?.proctor_events ?? 0} matukio</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      <CreateExam onCreated={() => void refresh()} />
      {err ? <EduError message={err} /> : null}

      <EduCard>
        <div className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-content">Mitihani ({exams.length})</span>
          {exams.length === 0 ? (
            <EduEmpty>Hakuna mitihani bado.</EduEmpty>
          ) : (
            exams.map((e) => (
              <button
                key={e.id}
                type="button"
                data-testid={`exam-item-${e.id}`}
                onClick={() => setOpen(e)}
                className="rounded-md border border-border p-2 text-left transition hover:border-primary/60"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium text-content">{e.title}</span>
                  <div className="flex gap-1">
                    <EduBadge>{e.status}</EduBadge>
                    <EduBadge>{e.questions} maswali</EduBadge>
                    <EduBadge>{e.minutes} dk</EduBadge>
                    <EduBadge>{e.attempts} wafanya</EduBadge>
                  </div>
                </div>
              </button>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}

// ------------------------------------------------------------
// Kuunda mtihani
// ------------------------------------------------------------

function CreateExam({ onCreated }: { onCreated: () => void }) {
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [subjectId, setSubjectId] = useState("");
  const [title, setTitle] = useState("");
  const [minutes, setMinutes] = useState("60");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.levels("");
        setLevels(r.levels);
        if (r.levels[0]) setLevelId(r.levels[0].id);
      } catch {
        /* hakuna levels */
      }
    })();
  }, []);

  useEffect(() => {
    if (!levelId) return;
    void (async () => {
      try {
        const r = await elimuproService.subjects(levelId);
        setSubjects(r.subjects);
        setSubjectId(r.subjects[0]?.id ?? "");
      } catch {
        setSubjects([]);
      }
    })();
  }, [levelId]);

  const create = async () => {
    if (!title.trim()) {
      setErr("Kichwa cha mtihani kinahitajika.");
      return;
    }
    setBusy(true);
    setErr("");
    try {
      await proctorService.createExam({
        title: title.trim(),
        subject_id: subjectId,
        level_id: levelId,
        minutes: Number(minutes) || 60,
        proctor: true,
      });
      setMsg("Mtihani umeundwa. Ongeza maswali kisha chapisha.");
      setTitle("");
      onCreated();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <EduCard>
      <div className="grid gap-3 md:grid-cols-2">
        <EduField label="Level">
          <EduSelect testId="exam-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
            {levels.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
        <EduField label="Somo">
          <EduSelect testId="exam-subject" value={subjectId} onChange={(e) => setSubjectId(e.target.value)}>
            {subjects.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
        <EduField label="Kichwa">
          <EduInput
            testId="exam-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Mtihani wa kati ya muhula"
          />
        </EduField>
        <EduField label="Dakika">
          <EduInput
            testId="exam-minutes"
            value={minutes}
            onChange={(e) => setMinutes(e.target.value)}
            inputMode="numeric"
          />
        </EduField>
        <div className="md:col-span-2 flex flex-col gap-2">
          {err ? <EduError message={err} /> : null}
          {msg ? <p className="text-xs text-primary">{msg}</p> : null}
          <EduButton testId="exam-create" onClick={() => void create()} disabled={busy}>
            <Plus className="size-4" aria-hidden />
            {busy ? "Inaunda..." : "Unda mtihani"}
          </EduButton>
        </div>
      </div>
    </EduCard>
  );
}

// ------------------------------------------------------------
// Maelezo ya mtihani (mwalimu)
// ------------------------------------------------------------

function ExamDetail({
  exam,
  onBack,
  onSit,
}: {
  exam: PcExam;
  onBack: () => void;
  onSit: (attemptId: string) => void;
}) {
  const [questions, setQuestions] = useState<PcQuestion[]>([]);
  const [attempts, setAttempts] = useState<PcAttempt[]>([]);
  const [detail, setDetail] = useState<{ status: string; minutes: number } | null>(null);
  const [kind, setKind] = useState<"mcq" | "text" | "code">("mcq");
  const [prompt, setPrompt] = useState("");
  const [options, setOptions] = useState("A, B, C, D");
  const [answerKey, setAnswerKey] = useState("A");
  const [marks, setMarks] = useState("2");
  const [studentId, setStudentId] = useState("");
  const [err, setErr] = useState("");

  const load = useCallback(async () => {
    try {
      const d = await proctorService.exam(exam.id);
      setDetail({ status: d.exam.status, minutes: d.exam.minutes });
      setQuestions((await proctorService.questions(exam.id, true)).questions);
      setAttempts((await proctorService.attempts(exam.id)).attempts);
    } catch (e) {
      setErr(String(e));
    }
  }, [exam.id]);

  useEffect(() => {
    void load();
  }, [load]);

  const addQ = async () => {
    if (!prompt.trim()) {
      setErr("Swali linahitajika.");
      return;
    }
    try {
      await proctorService.addQuestion(exam.id, {
        kind,
        prompt: prompt.trim(),
        options: kind === "mcq" ? options.split(",").map((o) => o.trim()).filter(Boolean) : [],
        answer_key: kind === "mcq" ? answerKey.trim() : "",
        marks: Number(marks) || 1,
      });
      setPrompt("");
      await load();
    } catch (e) {
      setErr(String(e));
    }
  };

  const publish = async () => {
    try {
      await proctorService.publish(exam.id);
      await load();
    } catch (e) {
      setErr(String(e));
    }
  };

  const sit = async () => {
    if (!studentId.trim()) {
      setErr("Weka namba ya mwanafunzi.");
      return;
    }
    try {
      const r = await proctorService.start(exam.id, studentId.trim());
      if (!r.ok) {
        setErr("Imeshindwa kuanza mtihani.");
        return;
      }
      onSit(r.attempt_id);
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div data-testid="exam-detail" className="flex flex-col gap-4">
      <EduCard>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <ClipboardList className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">{exam.title}</span>
            <EduBadge>{detail?.status ?? exam.status}</EduBadge>
          </div>
          <div className="flex gap-1">
            <EduButton variant="ghost" testId="exam-back" onClick={onBack}>
              Rudi
            </EduButton>
            {detail?.status !== "published" ? (
              <EduButton testId="exam-publish" onClick={() => void publish()}>
                Chapisha
              </EduButton>
            ) : null}
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      <EduCard testId="exam-add-q">
        <div className="grid gap-3 md:grid-cols-2">
          <EduField label="Aina ya swali">
            <EduSelect testId="exam-q-kind" value={kind} onChange={(e) => setKind(e.target.value as "mcq" | "text" | "code")}>
              <option value="mcq">Chaguo (MCQ)</option>
              <option value="text">Maandishi</option>
              <option value="code">Code (IT)</option>
            </EduSelect>
          </EduField>
          <EduField label="Alama">
            <EduInput testId="exam-q-marks" value={marks} onChange={(e) => setMarks(e.target.value)} inputMode="numeric" />
          </EduField>
          <div className="md:col-span-2">
            <EduField label="Swali">
              <EduTextarea testId="exam-q-prompt" value={prompt} onChange={(e) => setPrompt(e.target.value)} rows={3} />
            </EduField>
          </div>
          {kind === "mcq" ? (
            <>
              <EduField label="Chaguo (tenganisha kwa koma)">
                <EduInput testId="exam-q-options" value={options} onChange={(e) => setOptions(e.target.value)} />
              </EduField>
              <EduField label="Jibu sahihi (herufi)">
                <EduInput testId="exam-q-key" value={answerKey} onChange={(e) => setAnswerKey(e.target.value)} />
              </EduField>
            </>
          ) : null}
          <div className="md:col-span-2">
            <EduButton testId="exam-q-add" onClick={() => void addQ()}>
              <Plus className="size-4" aria-hidden /> Ongeza swali
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard testId="exam-questions">
        <span className="text-sm font-semibold text-content">Maswali ({questions.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {questions.length === 0 ? (
            <EduEmpty>Hakuna maswali bado.</EduEmpty>
          ) : (
            questions.map((q) => (
              <div key={q.id} className="rounded-md border border-border p-2">
                <div className="flex items-start justify-between gap-2">
                  <span className="text-sm text-content">
                    {q.n}. {q.prompt}
                  </span>
                  <div className="flex shrink-0 gap-1">
                    <EduBadge>{q.kind}</EduBadge>
                    <EduBadge>{q.marks} alama</EduBadge>
                  </div>
                </div>
                {q.options.length > 0 ? (
                  <div className="mt-1 flex flex-wrap gap-1">
                    {q.options.map((o, i) => (
                      <EduBadge key={`${q.id}-${i}`}>
                        {String.fromCharCode(65 + i)}. {o}
                      </EduBadge>
                    ))}
                  </div>
                ) : null}
                {q.answer_key ? <p className="mt-1 text-xs text-primary">Jibu: {q.answer_key}</p> : null}
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="exam-sit">
        <div className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-content">Kufanya mtihani</span>
          <EduInput
            testId="exam-student"
            value={studentId}
            onChange={(e) => setStudentId(e.target.value)}
            placeholder="Namba ya mwanafunzi (student_id)"
          />
          <EduButton testId="exam-start" onClick={() => void sit()}>
            <SquarePlay className="size-4" aria-hidden /> Anza mtihani (ulinzi huwashwa)
          </EduButton>
        </div>
      </EduCard>

      <EduCard testId="exam-attempts">
        <span className="text-sm font-semibold text-content">Wafanya ({attempts.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {attempts.map((a) => (
            <div key={a.id} className="flex items-center justify-between rounded-md border border-border p-2">
              <span className="text-sm text-content">{a.student}</span>
              <div className="flex gap-1">
                <EduBadge>{a.status}</EduBadge>
                <EduBadge>alama {a.score < 0 ? "—" : a.score}</EduBadge>
                <EduBadge>
                  <AlertTriangle className="size-3" aria-hidden /> {a.violations}
                </EduBadge>
                {a.grade_mode ? <EduBadge>{a.grade_mode}</EduBadge> : null}
              </div>
            </div>
          ))}
        </div>
      </EduCard>
    </div>
  );
}

// ------------------------------------------------------------
// Kufanya mtihani (mwanafunzi) — sehemu mbili
// ------------------------------------------------------------

function Sitting({
  exam,
  attemptId,
  onDone,
}: {
  exam: PcExam;
  attemptId: string;
  onDone: () => void;
}) {
  const [questions, setQuestions] = useState<PcQuestion[]>([]);
  const [answers, setAnswers] = useState<Record<string, { body: string; locked: boolean }>>({});
  const [draft, setDraft] = useState<Record<string, string>>({});
  const [current, setCurrent] = useState(0);
  const [result, setResult] = useState<{ score: number; max: number; mode: string } | null>(null);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  const proctor = useProctor(attemptId, !result, {
    photoEveryMs: 20000,
    onFlag: (kind) => {
      if (kind === "ai_suspect") setErr("Onyo: mtindo wa jibu unaonekana kama wa AI.");
    },
  });

  useEffect(() => {
    void (async () => {
      try {
        const r = await proctorService.questions(exam.id);
        setQuestions(r.questions);
        const a = await proctorService.attempt(attemptId);
        const map: Record<string, { body: string; locked: boolean }> = {};
        a.answers.forEach((x) => {
          map[x.question_id] = { body: x.body, locked: x.locked };
        });
        setAnswers(map);
      } catch (e) {
        setErr(String(e));
      }
    })();
  }, [exam.id, attemptId]);

  const q = questions[current] ?? null;

  const lockAnswer = async () => {
    if (!q) return;
    const body = (draft[q.id] ?? "").trim();
    if (!body) {
      setErr("Andika jibu kabla ya kufunga.");
      return;
    }
    if (q.kind === "text" && aiStyleSuspect(body)) {
      proctor.report("ai_suspect", "maandishi yenye mtindo wa AI");
    }
    try {
      const r = await proctorService.saveAnswer(attemptId, q.id, body);
      if (!r.ok) {
        setErr(r.error ?? "Imeshindwa kuhifadhi");
        return;
      }
      setAnswers((prev) => ({ ...prev, [q.id]: { body, locked: true } }));
      setDraft((prev) => ({ ...prev, [q.id]: "" }));
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  };

  const runCode = async () => {
    if (!q) return;
    setBusy(true);
    try {
      const body = (draft[q.id] ?? answers[q.id]?.body ?? "").trim();
      const r = await proctorService.runCode(attemptId, q.id, "python", body);
      setErr(`Matokeo ya kuendesha: ${JSON.stringify(r.run).slice(0, 300)}`);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const submit = async () => {
    setBusy(true);
    try {
      const r = await proctorService.submit(attemptId);
      setResult({ score: r.score, max: r.max_score, mode: r.grade_mode });
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const answered = useMemo(() => Object.values(answers).filter((a) => a.locked).length, [answers]);

  return (
    <div data-testid="exam-sitting" className="flex flex-col gap-4">
      <EduCard testId="proctor-bar">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <ShieldCheck className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">{exam.title}</span>
            <EduBadge>
              <Camera className="size-3" aria-hidden /> {proctor.cameraOn ? "kamera ON" : "kamera OFF"}
            </EduBadge>
            <EduBadge>{proctor.micOn ? "mic ON" : "mic OFF"}</EduBadge>
            <EduBadge testId="proctor-violations">
              <AlertTriangle className="size-3" aria-hidden /> ukiukaji {proctor.violations}
            </EduBadge>
            <EduBadge>sauti {(proctor.audioLevel * 100).toFixed(0)}%</EduBadge>
          </div>
          <EduButton testId="exam-submit" onClick={() => void submit()} disabled={busy || Boolean(result)}>
            <Send className="size-4" aria-hidden /> Wasilisha
          </EduButton>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      {result ? (
        <EduCard testId="exam-result">
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="size-5 text-primary" aria-hidden />
              <span className="text-sm font-semibold text-content">
                Matokeo: {result.score} / {result.max}
              </span>
              <EduBadge>{result.mode}</EduBadge>
            </div>
            {result.mode === "pending" ? (
              <p className="text-xs text-muted">
                Baadhi ya majibu yanahitaji mwalimu (au LLM) kuyasahihisha.
              </p>
            ) : null}
            <EduButton testId="exam-done" onClick={onDone}>
              Funga
            </EduButton>
          </div>
        </EduCard>
      ) : null}

      {!result && q ? (
        <div className="grid gap-4 lg:grid-cols-2">
          {/* Kushoto: maswali yote */}
          <EduCard testId="exam-pane-left">
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-2">
                <FileText className="size-4 text-primary" aria-hidden />
                <span className="text-sm font-semibold text-content">
                  Maswali ({answered}/{questions.length} yamejibiwa)
                </span>
              </div>
              {questions.map((x, i) => (
                <button
                  key={x.id}
                  type="button"
                  data-testid={`exam-q-${i + 1}`}
                  onClick={() => setCurrent(i)}
                  className={
                    "rounded-md border p-2 text-left text-sm transition " +
                    (i === current ? "border-primary" : "border-border hover:border-primary/50")
                  }
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-content">
                      {x.n}. {x.prompt.slice(0, 90)}
                    </span>
                    {answers[x.id]?.locked ? (
                      <EduBadge>
                        <CheckCircle2 className="size-3" aria-hidden /> imefungwa
                      </EduBadge>
                    ) : (
                      <EduBadge>wazi</EduBadge>
                    )}
                  </div>
                </button>
              ))}
            </div>
          </EduCard>

          {/* Kulia: jibu la swali la sasa */}
          <EduCard testId="exam-pane-right">
            <div className="flex flex-col gap-2">
              <span className="text-sm font-semibold text-content">
                Swali {q.n} ({q.kind}) — alama {q.marks}
              </span>
              <p className="text-sm text-content">{q.prompt}</p>
              {q.options.length > 0 ? (
                <div className="flex flex-wrap gap-1">
                  {q.options.map((o, i) => (
                    <EduBadge key={`${q.id}-opt-${i}`}>
                      {String.fromCharCode(65 + i)}. {o}
                    </EduBadge>
                  ))}
                </div>
              ) : null}

              {answers[q.id]?.locked ? (
                <div data-testid="exam-locked" className="rounded-md border border-primary/40 bg-surface-secondary p-3">
                  <div className="flex items-center gap-2">
                    <Eye className="size-4 text-primary" aria-hidden />
                    <span className="text-xs text-primary">Jibu limefungwa — haliwezi kufutwa wala kukatwa</span>
                  </div>
                  <p className="mt-2 whitespace-pre-wrap text-sm text-content">{answers[q.id].body}</p>
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  {q.kind === "mcq" ? (
                    <EduInput
                      testId="exam-answer"
                      value={draft[q.id] ?? ""}
                      onChange={(e) => setDraft((p) => ({ ...p, [q.id]: e.target.value }))}
                      placeholder="Andika herufi ya jibu (mfano B)"
                    />
                  ) : (
                    <EduTextarea
                      testId="exam-answer"
                      value={draft[q.id] ?? ""}
                      onChange={(e) => setDraft((p) => ({ ...p, [q.id]: e.target.value }))}
                      rows={q.kind === "code" ? 10 : 6}
                      placeholder={q.kind === "code" ? "print('habari')" : "Andika jibu lako..."}
                    />
                  )}
                  <div className="flex flex-wrap gap-1">
                    <EduButton testId="exam-lock" onClick={() => void lockAnswer()}>
                      <CheckCircle2 className="size-4" aria-hidden /> Hifadhi & funga jibu
                    </EduButton>
                    {q.kind === "code" ? (
                      <EduButton variant="ghost" testId="exam-run" onClick={() => void runCode()} disabled={busy}>
                        Endesha
                      </EduButton>
                    ) : null}
                  </div>
                  <p className="text-xs text-muted">
                    Ukishafunga jibu, haliwezi kubadilishwa. Hii inazuia kufuta/kukata.
                  </p>
                </div>
              )}
            </div>
          </EduCard>
        </div>
      ) : null}
    </div>
  );
}
