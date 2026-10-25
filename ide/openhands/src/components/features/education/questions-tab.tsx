import { useCallback, useEffect, useState } from "react";
import {
  educationService,
  type EduQuestion,
  type EduSubject,
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

export function QuestionsTab() {
  const [subjects, setSubjects] = useState<EduSubject[]>([]);
  const [questions, setQuestions] = useState<EduQuestion[]>([]);
  const [subject, setSubject] = useState("");
  const [level, setLevel] = useState("");
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [open, setOpen] = useState<string | null>(null);

  // Kizazi cha AI
  const [genSubject, setGenSubject] = useState("");
  const [genLevel, setGenLevel] = useState("o-level");
  const [genTopic, setGenTopic] = useState("");
  const [genCount, setGenCount] = useState(5);
  const [generating, setGenerating] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const [s, q] = await Promise.all([
        educationService.subjects(),
        educationService.questions({
          subject: subject || undefined,
          level: level || undefined,
          q: search || undefined,
          limit: 100,
        }),
      ]);
      setSubjects(s.subjects ?? []);
      setQuestions(q.questions ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [subject, level, search]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runSeed = async () => {
    setLoading(true);
    setError("");
    try {
      await educationService.seed();
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const runGenerate = async () => {
    if (!genSubject.trim()) {
      setError("Weka somo kwa kizazi cha AI.");
      return;
    }
    setGenerating(true);
    setError("");
    try {
      const r = await educationService.generateQuestions({
        subject: genSubject.trim(),
        level: genLevel,
        topic: genTopic.trim() || undefined,
        count: genCount,
        language: "Kiswahili",
      });
      if (!r.ok) throw new Error(r.error ?? "kizazi kilishindwa");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setGenerating(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Tengeneza maswali kwa AI (mtindo wa NECTA)
        </h3>
        <div className="grid gap-3 sm:grid-cols-4">
          <EduField label="Somo">
            <EduInput
              testId="edu-gen-subject"
              value={genSubject}
              onChange={(e) => setGenSubject(e.target.value)}
              placeholder="mf. Physics"
            />
          </EduField>
          <EduField label="Ngazi">
            <EduSelect
              testId="edu-gen-level"
              value={genLevel}
              onChange={(e) => setGenLevel(e.target.value)}
            >
              {LEVELS.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Mada">
            <EduInput
              testId="edu-gen-topic"
              value={genTopic}
              onChange={(e) => setGenTopic(e.target.value)}
              placeholder="mf. Umeme"
            />
          </EduField>
          <EduField label="Idadi">
            <EduInput
              testId="edu-gen-count"
              type="number"
              min={1}
              max={25}
              value={genCount}
              onChange={(e) => setGenCount(Number(e.target.value))}
            />
          </EduField>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="edu-gen-run"
            onClick={runGenerate}
            disabled={generating}
          >
            {generating ? "Inatengeneza… (inaweza kuchukua dakika)" : "Tengeneza"}
          </EduButton>
          <EduButton variant="ghost" onClick={runSeed} disabled={loading}>
            Weka maswali ya mfano
          </EduButton>
        </div>
        {generating ? (
          <div className="mt-3">
            <EduSpinner label="AI inatengeneza maswali na maelezo…" />
          </div>
        ) : null}
      </EduCard>

      {error ? <EduError message={error} /> : null}

      <EduCard>
        <div className="mb-3 flex flex-wrap items-end gap-3">
          <EduField label="Somo">
            <EduSelect
              testId="edu-filter-subject"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
            >
              <option value="">Zote</option>
              {subjects.map((s) => (
                <option key={s.subject} value={s.subject}>
                  {s.subject}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Ngazi">
            <EduSelect
              testId="edu-filter-level"
              value={level}
              onChange={(e) => setLevel(e.target.value)}
            >
              <option value="">Zote</option>
              {LEVELS.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Tafuta">
            <EduInput
              testId="edu-filter-search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="neno kwenye swali/jibu"
            />
          </EduField>
          <EduButton variant="ghost" onClick={refresh} disabled={loading}>
            Chuja
          </EduButton>
        </div>

        {loading ? (
          <EduSpinner label="Inapakia…" />
        ) : questions.length === 0 ? (
          <EduEmpty>
            Hakuna maswali bado. Weka ya mfano au tumia AI kutengeneza.
          </EduEmpty>
        ) : (
          <ul className="flex flex-col divide-y divide-border">
            {questions.map((q) => (
              <li key={q.id} className="py-2">
                <button
                  type="button"
                  onClick={() => setOpen(open === q.id ? null : q.id)}
                  className="flex w-full items-start gap-2 text-left"
                >
                  <span className="mt-0.5 text-xs text-muted">
                    {open === q.id ? "▾" : "▸"}
                  </span>
                  <span className="flex-1 text-sm text-content">
                    {q.question}
                  </span>
                  <span className="flex shrink-0 gap-1">
                    <EduBadge>{q.subject}</EduBadge>
                    <EduBadge>{q.level}</EduBadge>
                  </span>
                </button>
                {open === q.id ? (
                  <div className="mt-2 ml-5 flex flex-col gap-1.5 border-l border-border pl-3 text-sm">
                    {q.topic ? (
                      <p className="text-xs text-muted">Mada: {q.topic}</p>
                    ) : null}
                    <p>
                      <span className="font-medium text-content">Jibu: </span>
                      <span className="text-content">{q.answer}</span>
                    </p>
                    <p className="text-muted">
                      <span className="font-medium">Maelezo: </span>
                      {q.explanation}
                    </p>
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}
