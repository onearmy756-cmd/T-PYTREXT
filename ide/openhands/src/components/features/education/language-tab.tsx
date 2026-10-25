import { useCallback, useEffect, useState } from "react";
import {
  educationService,
  type EduLanguageLesson,
  type EduLessonMeta,
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

const LEVELS = ["beginner", "intermediate", "advanced"];

const COMMON_LANGUAGES = [
  "Kiingereza (English)",
  "Kifaransa (French)",
  "Kiarabu (Arabic)",
  "Kihispania (Spanish)",
  "Kijerumani (German)",
  "Kichina (Mandarin)",
  "Kijapani (Japanese)",
  "Kireno (Portuguese)",
  "Kihindi (Hindi)",
  "Kirusi (Russian)",
];

export function LanguageTab() {
  const [language, setLanguage] = useState("");
  const [level, setLevel] = useState("beginner");
  const [topic, setTopic] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [lesson, setLesson] = useState<EduLanguageLesson | null>(null);
  const [history, setHistory] = useState<EduLessonMeta[]>([]);
  const [prefsUser, setPrefsUser] = useState<string | null>(null);
  const [savedNotice, setSavedNotice] = useState("");

  const loadHistory = useCallback(async () => {
    try {
      const r = await educationService.languageLessons();
      setHistory(r.lessons ?? []);
    } catch {
      /* historia ni ya ziada */
    }
  }, []);

  // Mapendeleo ya lugha ya mtumiaji hupakiwa mara moja, ili kila mtumiaji
  // aanze na lugha yake mwenyewe badala ya chaguo-msingi.
  useEffect(() => {
    void loadHistory();
    void (async () => {
      try {
        const p = await educationService.prefs();
        setPrefsUser(p.username);
        if (p.learn_language) setLanguage(p.learn_language);
        if (p.level) setLevel(p.level);
      } catch {
        /* bila login, mapendeleo hayapatikani */
      }
    })();
  }, [loadHistory]);

  const rememberPrefs = async () => {
    setSavedNotice("");
    try {
      const p = await educationService.savePrefs({
        learn_language: language.trim(),
        level,
      });
      if (p.saved) setSavedNotice("Mapendeleo yamehifadhiwa.");
      else setSavedNotice("Hifadhi inahitaji login.");
    } catch (e) {
      setSavedNotice(e instanceof Error ? e.message : String(e));
    }
  };

  const generate = async () => {
    if (!language.trim()) {
      setError("Weka lugha unayotaka kujifunza.");
      return;
    }
    setBusy(true);
    setError("");
    setLesson(null);
    try {
      const r = await educationService.languageLesson({
        language: language.trim(),
        level,
        topic: topic.trim() || undefined,
        native: "Kiswahili",
      });
      if (!r.ok) throw new Error(r.error ?? "somo lilishindwa");
      setLesson(r.lesson);
      await loadHistory();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Jifunze lugha yoyote — somo kamili kwa AI
        </h3>
        <div className="grid gap-3 sm:grid-cols-3">
          <EduField label="Lugha">
            <EduInput
              testId="edu-lang-language"
              list="edu-lang-options"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              placeholder="mf. Kifaransa (French)"
            />
            <datalist id="edu-lang-options">
              {COMMON_LANGUAGES.map((l) => (
                <option key={l} value={l} />
              ))}
            </datalist>
          </EduField>
          <EduField label="Ngazi">
            <EduSelect
              testId="edu-lang-level"
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
          <EduField label="Mada">
            <EduInput
              testId="edu-lang-topic"
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
              placeholder="mf. Salamu na kujitambulisha"
            />
          </EduField>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <EduButton
            testId="edu-lang-generate"
            onClick={generate}
            disabled={busy}
          >
            {busy ? "Inatengeneza somo… (dakika 2-5)" : "Tengeneza somo"}
          </EduButton>
          <EduButton
            variant="ghost"
            testId="edu-lang-save-prefs"
            onClick={rememberPrefs}
            disabled={!prefsUser}
          >
            Hifadhi lugha yangu
          </EduButton>
          {savedNotice ? (
            <span className="text-xs text-muted">{savedNotice}</span>
          ) : null}
        </div>
        {busy ? (
          <div className="mt-3">
            <EduSpinner label="AI inatengeneza somo kamili…" />
          </div>
        ) : null}
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {lesson ? <LessonView lesson={lesson} /> : null}

      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Masomo yaliyohifadhiwa
        </h3>
        {history.length === 0 ? (
          <EduEmpty>Hakuna masomo bado.</EduEmpty>
        ) : (
          <ul className="flex flex-col divide-y divide-border">
            {history.map((h) => (
              <li key={h.id} className="flex items-center gap-2 py-2 text-sm">
                <span className="flex-1 text-content">{h.language}</span>
                <EduBadge>{h.level}</EduBadge>
                <span className="text-muted">{h.topic}</span>
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}

function LessonView({ lesson }: { lesson: EduLanguageLesson }) {
  if (lesson.raw) {
    return (
      <EduCard testId="edu-lang-lesson">
        <pre className="overflow-auto text-xs whitespace-pre-wrap text-muted">
          {lesson.raw}
        </pre>
      </EduCard>
    );
  }
  return (
    <EduCard testId="edu-lang-lesson">
      <h3 className="mb-3 text-base font-semibold text-content">
        {lesson.title}
      </h3>

      <Section title="Malengo" items={lesson.objectives} />

      {lesson.vocabulary && lesson.vocabulary.length > 0 ? (
        <section className="mt-4">
          <h4 className="mb-2 text-xs font-semibold tracking-wide text-muted uppercase">
            Msamiati
          </h4>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-muted">
                  <th className="py-1 pr-3">Neno</th>
                  <th className="py-1 pr-3">Matamshi</th>
                  <th className="py-1 pr-3">Maana</th>
                  <th className="py-1">Mfano</th>
                </tr>
              </thead>
              <tbody>
                {lesson.vocabulary.map((v, i) => (
                  <tr key={i} className="border-t border-border">
                    <td className="py-1.5 pr-3 font-medium text-content">
                      {v.term}
                    </td>
                    <td className="py-1.5 pr-3 text-muted">
                      {v.pronunciation}
                    </td>
                    <td className="py-1.5 pr-3 text-content">{v.meaning}</td>
                    <td className="py-1.5 text-muted">{v.example}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}

      {lesson.grammar && lesson.grammar.length > 0 ? (
        <section className="mt-4">
          <h4 className="mb-2 text-xs font-semibold tracking-wide text-muted uppercase">
            Sarufi
          </h4>
          <ul className="flex flex-col gap-2">
            {lesson.grammar.map((g, i) => (
              <li key={i} className="text-sm">
                <p className="font-medium text-content">{g.rule}</p>
                {g.explanation ? (
                  <p className="text-muted">{g.explanation}</p>
                ) : null}
                {g.examples && g.examples.length > 0 ? (
                  <ul className="mt-0.5 list-disc pl-5 text-muted">
                    {g.examples.map((ex, j) => (
                      <li key={j}>{ex}</li>
                    ))}
                  </ul>
                ) : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {lesson.dialogue && lesson.dialogue.length > 0 ? (
        <section className="mt-4">
          <h4 className="mb-2 text-xs font-semibold tracking-wide text-muted uppercase">
            Mazungumzo
          </h4>
          <ul className="flex flex-col gap-1.5 text-sm">
            {lesson.dialogue.map((d, i) => (
              <li key={i}>
                <span className="font-medium text-content">{d.speaker}: </span>
                <span className="text-content">{d.line}</span>
                {d.translation ? (
                  <span className="block text-xs text-muted">
                    {d.translation}
                  </span>
                ) : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {lesson.exercises && lesson.exercises.length > 0 ? (
        <section className="mt-4">
          <h4 className="mb-2 text-xs font-semibold tracking-wide text-muted uppercase">
            Mazoezi
          </h4>
          <ol className="flex list-decimal flex-col gap-1.5 pl-5 text-sm">
            {lesson.exercises.map((e, i) => (
              <li key={i}>
                <span className="text-content">{e.question}</span>
                {e.answer ? (
                  <span className="block text-xs text-muted">
                    Jibu: {e.answer}
                  </span>
                ) : null}
              </li>
            ))}
          </ol>
        </section>
      ) : null}

      <Section title="Vidokezo vya matamshi" items={lesson.pronunciation_tips} />

      {lesson.cultural_note ? (
        <section className="mt-4 rounded-md bg-surface-raised p-3">
          <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
            Utamaduni
          </h4>
          <p className="text-sm text-content">{lesson.cultural_note}</p>
        </section>
      ) : null}
    </EduCard>
  );
}

function Section({ title, items }: { title: string; items?: string[] }) {
  if (!items || items.length === 0) return null;
  return (
    <section className="mt-4">
      <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
        {title}
      </h4>
      <ul className="flex list-disc flex-col gap-1 pl-5 text-sm text-content">
        {items.map((it, i) => (
          <li key={i}>{it}</li>
        ))}
      </ul>
    </section>
  );
}
