import { useCallback, useEffect, useState } from "react";
import {
  educationService,
  type EduExamDetail,
  type EduExamMeta,
} from "#/api/education-service/education-service.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
  EduSpinner,
} from "./edu-ui";

const LEVELS = ["o-level", "a-level", "college"];

export function ExamsTab() {
  const [subject, setSubject] = useState("");
  const [level, setLevel] = useState("o-level");
  const [topics, setTopics] = useState("");
  const [numQuestions, setNumQuestions] = useState(10);
  const [duration, setDuration] = useState(180);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState("");

  const [exams, setExams] = useState<EduExamMeta[]>([]);
  const [detail, setDetail] = useState<EduExamDetail | null>(null);
  const [showAnswers, setShowAnswers] = useState(false);

  const loadExams = useCallback(async () => {
    try {
      const r = await educationService.exams();
      setExams(r.exams ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void loadExams();
  }, [loadExams]);

  const generate = async () => {
    if (!subject.trim()) {
      setError("Weka somo.");
      return;
    }
    setGenerating(true);
    setError("");
    setDetail(null);
    try {
      const r = await educationService.generateExam({
        subject: subject.trim(),
        level,
        topics: topics.trim() || undefined,
        num_questions: numQuestions,
        duration_min: duration,
        language: "Kiswahili",
      });
      if (!r.ok) throw new Error(r.error ?? "kizazi kilishindwa");
      setDetail({
        ok: true,
        exam: {
          id: r.id,
          title: r.title,
          subject: subject.trim(),
          level,
          num_questions: r.num_questions,
          duration_min: r.duration_min,
          total_marks: r.total_marks,
          created_at: "",
        },
        body: r.exam,
        answers: r.answers,
      });
      await loadExams();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setGenerating(false);
    }
  };

  const openExam = async (id: string) => {
    setError("");
    setShowAnswers(false);
    try {
      setDetail(await educationService.exam(id));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Tengeneza mtihani kamili (maswali + majibu + maelezo)
        </h3>
        <div className="grid gap-3 sm:grid-cols-5">
          <EduField label="Somo">
            <EduInput
              testId="edu-exam-subject"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="mf. Biology"
            />
          </EduField>
          <EduField label="Ngazi">
            <EduSelect
              testId="edu-exam-level"
              value={level}
              onChange={(e) => setLevel(e.target.value)}
            >
              {LEVELS.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Mada (kwa mkato)">
            <EduInput
              testId="edu-exam-topics"
              value={topics}
              onChange={(e) => setTopics(e.target.value)}
              placeholder="Seli, Jenetikia"
            />
          </EduField>
          <EduField label="Maswali">
            <EduInput
              testId="edu-exam-num"
              type="number"
              min={1}
              max={40}
              value={numQuestions}
              onChange={(e) => setNumQuestions(Number(e.target.value))}
            />
          </EduField>
          <EduField label="Dakika">
            <EduInput
              testId="edu-exam-duration"
              type="number"
              min={30}
              max={360}
              value={duration}
              onChange={(e) => setDuration(Number(e.target.value))}
            />
          </EduField>
        </div>
        <div className="mt-3">
          <EduButton
            testId="edu-exam-generate"
            onClick={generate}
            disabled={generating}
          >
            {generating ? "Inatengeneza… (dakika 2-6)" : "Tengeneza mtihani"}
          </EduButton>
        </div>
        {generating ? (
          <div className="mt-3">
            <EduSpinner label="AI inatengeneza mtihani kamili — tafadhali subiri…" />
          </div>
        ) : null}
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {detail ? (
        <EduCard testId="edu-exam-detail">
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div>
              <h3 className="text-base font-semibold text-content">
                {detail.body?.title ?? detail.exam.title}
              </h3>
              <div className="mt-1 flex flex-wrap gap-1">
                <EduBadge>{detail.exam.subject}</EduBadge>
                <EduBadge>{detail.exam.level}</EduBadge>
                <EduBadge>{detail.exam.num_questions} maswali</EduBadge>
                <EduBadge>{detail.exam.total_marks} alama</EduBadge>
                <EduBadge>{detail.exam.duration_min} dakika</EduBadge>
              </div>
            </div>
            <EduButton
              variant="ghost"
              testId="edu-exam-toggle-answers"
              onClick={() => setShowAnswers((v) => !v)}
            >
              {showAnswers ? "Ficha majibu" : "Onyesha majibu"}
            </EduButton>
          </div>

          {detail.body?.instructions ? (
            <p className="mb-3 rounded-md bg-surface-raised p-3 text-sm text-muted">
              {detail.body.instructions}
            </p>
          ) : null}

          {(detail.body?.sections ?? []).map((sec, si) => (
            <div key={si} className="mb-4">
              <h4 className="mb-2 text-sm font-semibold text-content">
                {sec.name}
              </h4>
              <ol className="flex flex-col gap-2">
                {(sec.questions ?? []).map((q) => (
                  <li key={q.number} className="text-sm">
                    <div className="flex items-start gap-2">
                      <span className="font-medium text-muted">
                        {q.number}.
                      </span>
                      <span className="flex-1 text-content">{q.question}</span>
                      <EduBadge>{q.marks} alama</EduBadge>
                    </div>
                    {showAnswers ? (
                      <ExamAnswer
                        number={q.number}
                        answers={(detail.answers?.sections ?? []).flatMap(
                          (s) => s.answers ?? [],
                        )}
                      />
                    ) : null}
                  </li>
                ))}
              </ol>
            </div>
          ))}
        </EduCard>
      ) : null}

      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Mitihani iliyohifadhiwa
        </h3>
        {exams.length === 0 ? (
          <EduEmpty>Hakuna mitihani bado.</EduEmpty>
        ) : (
          <ul className="flex flex-col divide-y divide-border">
            {exams.map((ex) => (
              <li key={ex.id}>
                <button
                  type="button"
                  onClick={() => openExam(ex.id)}
                  className="flex w-full items-center gap-2 py-2 text-left text-sm hover:bg-surface-raised"
                >
                  <span className="flex-1 text-content">{ex.title}</span>
                  <EduBadge>{ex.subject}</EduBadge>
                  <EduBadge>{ex.num_questions}Q</EduBadge>
                </button>
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}

function ExamAnswer({
  number,
  answers,
}: {
  number: number | string;
  answers: { number: number | string; answer?: string; explanation?: string }[];
}) {
  const found = answers.find((a) => String(a.number) === String(number));
  if (!found) return null;
  return (
    <div className="mt-1 ml-5 border-l border-border pl-3">
      <p className="text-sm">
        <span className="font-medium text-content">Jibu: </span>
        {found.answer}
      </p>
      {found.explanation ? (
        <p className="mt-0.5 text-xs whitespace-pre-wrap text-muted">
          {found.explanation}
        </p>
      ) : null}
    </div>
  );
}
