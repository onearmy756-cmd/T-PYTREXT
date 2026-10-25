import { useCallback, useEffect, useRef, useState } from "react";
import {
  Play,
  Pause,
  Upload,
  Sparkles,
  MessageCircleQuestion,
  Volume2,
  BookOpen,
} from "lucide-react";
import {
  educationService,
  type EduAudioClip,
  type EduLessonAudio,
  type EduLessonMwalimu,
  type EduLessonSection,
  type EduLessonSummary,
  type EduLessonTools,
} from "#/api/education-service/education-service.api";
import { getAgentServerBaseUrl } from "#/api/agent-server-config";
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

const LEVELS = [
  { value: "msingi", label: "Msingi (Shule ya Msingi)" },
  { value: "sekondari", label: "Sekondari" },
  { value: "chuo", label: "Chuo / Chuo Kikuu" },
];

const LANGS = ["Kiswahili", "English"];

/**
 * AI MWALIMU — mwalimu ana-upload notes (PDF/Word/picha/maandishi), AI
 * inatengeneza somo kamili (maelezo + mifano + uchambuzi) na sauti ya kila
 * sehemu. Mwanafunzi husoma, kusikiliza, na kuuliza maswali.
 */
export function LessonTab() {
  const [file, setFile] = useState<File | null>(null);
  const [title, setTitle] = useState("");
  const [subject, setSubject] = useState("");
  const [language, setLanguage] = useState("Kiswahili");
  const [level, setLevel] = useState("sekondari");
  const [minutes, setMinutes] = useState(10);
  const [withAudio, setWithAudio] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [lesson, setLesson] = useState<EduLessonMwalimu | null>(null);
  const [audio, setAudio] = useState<EduLessonAudio | null>(null);
  const [lessonId, setLessonId] = useState("");
  const [history, setHistory] = useState<EduLessonSummary[]>([]);
  const [tools, setTools] = useState<EduLessonTools | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const loadHistory = useCallback(async () => {
    try {
      const r = await educationService.lessonList();
      setHistory(r.lessons ?? []);
      setTools(r.tools ?? null);
    } catch {
      /* historia si muhimu kwa kazi kuu */
    }
  }, []);

  useEffect(() => {
    void loadHistory();
  }, [loadHistory]);

  const build = async () => {
    if (!file) {
      setError("Chagua faili la notes kwanza (PDF, Word, picha, au maandishi).");
      return;
    }
    setBusy(true);
    setError("");
    setLesson(null);
    setAudio(null);
    try {
      const r = await educationService.lessonUpload(file, {
        name: file.name,
        title: title.trim() || undefined,
        subject: subject.trim() || undefined,
        language,
        level,
        minutes,
        audio: withAudio ? 1 : 0,
      });
      if (!r.ok) throw new Error(r.error ?? "kutengeneza somo kulishindwa");
      setLesson(r.lesson ?? null);
      setAudio(r.audio ?? null);
      setLessonId(r.id ?? "");
      await loadHistory();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const openLesson = async (id: string) => {
    setBusy(true);
    setError("");
    try {
      const r = await educationService.lessonGet(id);
      setLesson(r.lesson);
      setAudio(r.audio);
      setLessonId(id);
      setTitle(r.title);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-1 text-sm font-semibold text-content">
          AI Mwalimu — upload notes, AI ifundishe
        </h3>
        <p className="mb-3 text-xs text-muted">
          Mwalimu ana-upload notes (PDF, Word, picha, au maandishi). AI
          inatengeneza somo kamili: maelezo, mifano halisi, uchambuzi, na sauti
          ya Kiswahili.
        </p>

        <div className="grid gap-3 sm:grid-cols-2">
          <EduField label="Faili la notes" hint="PDF · DOCX · TXT · PNG · JPG">
            <div className="flex items-center gap-2">
              <input
                ref={inputRef}
                data-testid="edu-lesson-file"
                type="file"
                accept=".pdf,.docx,.txt,.md,.png,.jpg,.jpeg,.webp,.bmp,.tif,.tiff"
                onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                className="block w-full text-xs text-muted file:mr-2 file:rounded-md file:border-0 file:bg-surface-raised file:px-2.5 file:py-1.5 file:text-xs file:text-content"
              />
            </div>
          </EduField>
          <EduField label="Kichwa (hiari)">
            <EduInput
              testId="edu-lesson-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="mf. Mzunguko wa Maji"
            />
          </EduField>
          <EduField label="Somo (hiari)">
            <EduInput
              testId="edu-lesson-subject"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="mf. Jiografia"
            />
          </EduField>
          <EduField label="Lugha">
            <EduSelect
              testId="edu-lesson-language"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
            >
              {LANGS.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kiwango">
            <EduSelect
              testId="edu-lesson-level"
              value={level}
              onChange={(e) => setLevel(e.target.value)}
            >
              {LEVELS.map((l) => (
                <option key={l.value} value={l.value}>
                  {l.label}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Muda (dakika)">
            <EduInput
              testId="edu-lesson-minutes"
              type="number"
              min={3}
              max={60}
              value={minutes}
              onChange={(e) => setMinutes(Number(e.target.value) || 10)}
            />
          </EduField>
        </div>

        <label className="mt-3 flex items-center gap-2 text-sm text-content">
          <input
            type="checkbox"
            checked={withAudio}
            onChange={(e) => setWithAudio(e.target.checked)}
          />
          Tengeneza sauti (TTS) ya kila sehemu
        </label>

        <div className="mt-3 flex items-center gap-2">
          <EduButton
            testId="edu-lesson-build"
            onClick={build}
            disabled={busy || !file}
          >
            <Upload className="size-4" />
            {busy ? "Inatengeneza somo…" : "Tengeneza Somo"}
          </EduButton>
          {tools && !tools.espeak ? (
            <span className="text-xs text-feedback-warning">
              TTS haipo — sakinisha: {tools.install.tts}
            </span>
          ) : null}
        </div>

        {busy ? (
          <div className="mt-3">
            <EduSpinner label="AI inasoma notes na kutengeneza somo…" />
          </div>
        ) : null}
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {lesson ? (
        <LessonView
          lesson={lesson}
          audio={audio}
          lessonId={lessonId}
          onAudio={setAudio}
        />
      ) : null}

      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Masomo yaliyotengenezwa
        </h3>
        {history.length === 0 ? (
          <EduEmpty>Hakuna somo bado. Upload notes kuanza.</EduEmpty>
        ) : (
          <ul className="flex flex-col divide-y divide-border">
            {history.map((h) => (
              <li key={h.id} className="flex items-center gap-2 py-2 text-sm">
                <button
                  type="button"
                  onClick={() => void openLesson(h.id)}
                  className="flex-1 truncate text-left text-content hover:text-primary"
                >
                  {h.title}
                </button>
                <EduBadge>{h.source_type}</EduBadge>
                <EduBadge>{h.level}</EduBadge>
                <EduBadge>{h.minutes} dk</EduBadge>
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}

function LessonView({
  lesson,
  audio,
  lessonId,
  onAudio,
}: {
  lesson: EduLessonMwalimu;
  audio: EduLessonAudio | null;
  lessonId: string;
  onAudio: (a: EduLessonAudio) => void;
}) {
  if (lesson.raw) {
    return (
      <EduCard testId="edu-lesson-view">
        <h3 className="mb-2 text-sm font-semibold text-content">Somo</h3>
        <pre className="overflow-auto text-xs whitespace-pre-wrap text-muted">
          {lesson.raw}
        </pre>
      </EduCard>
    );
  }
  return (
    <div data-testid="edu-lesson-view" className="flex flex-col gap-4">
      <EduCard>
        <div className="mb-2 flex flex-wrap items-center gap-2">
          <BookOpen className="size-4 text-primary" />
          <h3 className="text-base font-semibold text-content">
            {lesson.mada ?? "Somo"}
          </h3>
        </div>
        {lesson.utangulizi ? (
          <p className="text-sm text-content">{lesson.utangulizi}</p>
        ) : null}
        {audio?.utangulizi ? (
          <ClipPlayer clip={audio.utangulizi} label="Sikiliza utangulizi" />
        ) : null}
      </EduCard>

      {(lesson.sehemu ?? []).map((sec, i) => (
        <SectionView
          key={i}
          index={i}
          section={sec}
          clip={audio?.sehemu?.[String(i)] ?? null}
        />
      ))}

      {lesson.hitimisho ? (
        <EduCard>
          <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
            Hitimisho
          </h4>
          <p className="text-sm text-content">{lesson.hitimisho}</p>
          {audio?.hitimisho ? (
            <ClipPlayer clip={audio.hitimisho} label="Sikiliza hitimisho" />
          ) : null}
        </EduCard>
      ) : null}

      {lesson.maswali_ya_mazoezi && lesson.maswali_ya_mazoezi.length > 0 ? (
        <EduCard>
          <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
            Maswali ya mazoezi
          </h4>
          <ol className="flex list-decimal flex-col gap-1 pl-5 text-sm text-content">
            {lesson.maswali_ya_mazoezi.map((q, i) => (
              <li key={i}>{q}</li>
            ))}
          </ol>
        </EduCard>
      ) : null}

      {lesson.msamiati && lesson.msamiati.length > 0 ? (
        <EduCard>
          <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
            Msamiati
          </h4>
          <dl className="flex flex-col gap-1.5 text-sm">
            {lesson.msamiati.map((g, i) => (
              <div key={i} className="flex gap-2">
                <dt className="min-w-32 font-medium text-content">{g.neno}</dt>
                <dd className="text-muted">{g.maana}</dd>
              </div>
            ))}
          </dl>
        </EduCard>
      ) : null}

      {!audio?.available ? (
        <EduCard>
          <p className="text-xs text-feedback-warning">
            {audio?.error ?? "Sauti haikuzalishwa. Angalia espeak-ng."}
          </p>
          <EduButton
            className="mt-2"
            onClick={async () => {
              const r = await educationService.lessonAudio(lessonId);
              if (r.audio) onAudio(r.audio);
            }}
          >
            <Volume2 className="size-4" />
            Zalisha sauti sasa
          </EduButton>
        </EduCard>
      ) : null}

      <AskBox lessonId={lessonId} />
    </div>
  );
}

function SectionView({
  index,
  section,
  clip,
}: {
  index: number;
  section: EduLessonSection;
  clip: EduAudioClip | null;
}) {
  return (
    <EduCard>
      <div className="mb-2 flex items-center gap-2">
        <span className="flex size-6 items-center justify-center rounded-full bg-primary text-xs font-semibold text-on-primary">
          {index + 1}
        </span>
        <h4 className="text-sm font-semibold text-content">
          {section.kichwa ?? `Sehemu ${index + 1}`}
        </h4>
        {clip ? <EduBadge>sauti</EduBadge> : null}
      </div>

      {section.maelezo ? (
        <p className="mb-3 text-sm text-content">{section.maelezo}</p>
      ) : null}

      {section.mfano ? (
        <div className="mb-3 rounded-md border border-border bg-surface-raised p-3">
          <div className="mb-1 flex items-center gap-1.5 text-xs font-semibold text-primary">
            <Sparkles className="size-3.5" />
            MFANO
          </div>
          <p className="text-sm text-content">{section.mfano}</p>
        </div>
      ) : null}

      {section.uchambuzi ? (
        <div className="rounded-md border border-border bg-surface-raised p-3">
          <div className="mb-1 text-xs font-semibold text-muted uppercase">
            Uchambuzi
          </div>
          <p className="text-sm text-muted">{section.uchambuzi}</p>
        </div>
      ) : null}

      {clip ? <ClipPlayer clip={clip} label="Sikiliza sehemu hii" /> : null}
    </EduCard>
  );
}

function ClipPlayer({ clip, label }: { clip: EduAudioClip; label: string }) {
  const [playing, setPlaying] = useState(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const toggle = () => {
    const el = audioRef.current;
    if (!el) return;
    if (playing) {
      el.pause();
      setPlaying(false);
    } else {
      const base = (getAgentServerBaseUrl() ?? "").replace(/\/+$/, "");
      el.src = `${base}${clip.url}`;
      void el.play().then(() => setPlaying(true)).catch(() => setPlaying(false));
    }
  };

  return (
    <div className="mt-3 flex items-center gap-2">
      <EduButton variant="ghost" onClick={toggle}>
        {playing ? <Pause className="size-4" /> : <Play className="size-4" />}
        {playing ? "Simamisha" : label}
      </EduButton>
      <audio ref={audioRef} onEnded={() => setPlaying(false)} className="hidden" />
    </div>
  );
}

function AskBox({ lessonId }: { lessonId: string }) {
  const [q, setQ] = useState("");
  const [busy, setBusy] = useState(false);
  const [answer, setAnswer] = useState("");
  const [error, setError] = useState("");

  const ask = async () => {
    if (!q.trim()) return;
    setBusy(true);
    setError("");
    setAnswer("");
    try {
      const r = await educationService.lessonAsk(lessonId, q.trim());
      if (!r.ok) throw new Error(r.error ?? "hakuna jibu");
      setAnswer(r.answer ?? "");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <EduCard>
      <h4 className="mb-2 flex items-center gap-1.5 text-sm font-semibold text-content">
        <MessageCircleQuestion className="size-4 text-primary" />
        Uliza mwalimu wa AI
      </h4>
      <div className="flex gap-2">
        <EduInput
          testId="edu-lesson-ask-input"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="mf. Nieleze tena kuhusu mvua…"
        />
        <EduButton testId="edu-lesson-ask" onClick={ask} disabled={busy}>
          {busy ? "…" : "Uliza"}
        </EduButton>
      </div>
      {error ? (
        <p className="mt-2 text-sm text-feedback-error">{error}</p>
      ) : null}
      {answer ? (
        <p
          data-testid="edu-lesson-answer"
          className="mt-3 rounded-md border border-border bg-surface-raised p-3 text-sm text-content whitespace-pre-wrap"
        >
          {answer}
        </p>
      ) : null}
    </EduCard>
  );
}
