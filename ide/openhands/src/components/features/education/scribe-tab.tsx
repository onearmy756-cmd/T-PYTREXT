import { useCallback, useEffect, useState } from "react";
import {
  educationService,
  type EduScribe,
  type EduScribeAnalysis,
} from "#/api/education-service/education-service.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSpinner,
  EduTextarea,
} from "./edu-ui";

export function ScribeTab() {
  const [title, setTitle] = useState("");
  const [transcript, setTranscript] = useState("");
  const [subject, setSubject] = useState("");
  const [language, setLanguage] = useState("Kiswahili");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [analysis, setAnalysis] = useState<EduScribeAnalysis | null>(null);
  const [history, setHistory] = useState<EduScribe[]>([]);

  const loadHistory = useCallback(async () => {
    try {
      const r = await educationService.scribeList();
      setHistory(r.scribe ?? []);
    } catch {
      /* historia si muhimu kwa kazi kuu */
    }
  }, []);

  useEffect(() => {
    void loadHistory();
  }, [loadHistory]);

  const analyze = async () => {
    if (!transcript.trim()) {
      setError("Weka maandishi (au tumia sauti kwenye kivinjari).");
      return;
    }
    setBusy(true);
    setError("");
    setAnalysis(null);
    try {
      const r = await educationService.scribeAnalyze({
        title: title.trim() || undefined,
        transcript: transcript.trim(),
        language,
        subject: subject.trim() || undefined,
      });
      if (!r.ok) throw new Error(r.error ?? "uchambuzi ulishindwa");
      setAnalysis(r.analysis);
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
          AI Scribe — uchambuzi wa mihadhari na maandishi
        </h3>
        <div className="grid gap-3 sm:grid-cols-3">
          <EduField label="Kichwa">
            <EduInput
              testId="edu-scribe-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="mf. Mhadhara wa Fotosintesis"
            />
          </EduField>
          <EduField label="Somo">
            <EduInput
              testId="edu-scribe-subject"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="mf. Biology"
            />
          </EduField>
          <EduField label="Lugha">
            <EduInput
              testId="edu-scribe-language"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
            />
          </EduField>
        </div>
        <div className="mt-3">
          <EduField
            label="Maandishi / manukuu ya mhadhara"
            hint="Unaweza kunakili maandishi ya sauti (transcript) hapa."
          >
            <EduTextarea
              testId="edu-scribe-transcript"
              value={transcript}
              onChange={(e) => setTranscript(e.target.value)}
              placeholder="Bandika maandishi ya mhadhara hapa…"
              className="min-h-40"
            />
          </EduField>
        </div>
        <div className="mt-3">
          <EduButton
            testId="edu-scribe-analyze"
            onClick={analyze}
            disabled={busy}
          >
            {busy ? "Inachambua…" : "Chambua kwa AI"}
          </EduButton>
        </div>
        {busy ? (
          <div className="mt-3">
            <EduSpinner label="AI inachambua maandishi…" />
          </div>
        ) : null}
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {analysis ? <AnalysisView analysis={analysis} /> : null}

      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Kumbukumbu za Scribe
        </h3>
        {history.length === 0 ? (
          <EduEmpty>Hakuna kumbukumbu bado.</EduEmpty>
        ) : (
          <ul className="flex flex-col divide-y divide-border">
            {history.map((h) => (
              <li
                key={h.id}
                className="flex items-center gap-2 py-2 text-sm"
              >
                <span className="flex-1 text-content">{h.title}</span>
                <EduBadge>{h.source_type}</EduBadge>
                <EduBadge>{h.language}</EduBadge>
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}

function AnalysisView({ analysis }: { analysis: EduScribeAnalysis }) {
  if (analysis.raw) {
    return (
      <EduCard testId="edu-scribe-analysis">
        <h3 className="mb-2 text-sm font-semibold text-content">Uchambuzi</h3>
        <pre className="overflow-auto text-xs whitespace-pre-wrap text-muted">
          {analysis.raw}
        </pre>
      </EduCard>
    );
  }
  return (
    <EduCard testId="edu-scribe-analysis">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold text-content">Uchambuzi</h3>
        {analysis.difficulty ? (
          <EduBadge>kiwango: {analysis.difficulty}</EduBadge>
        ) : null}
        {analysis.word_count ? (
          <EduBadge>{analysis.word_count} maneno</EduBadge>
        ) : null}
      </div>

      {analysis.summary ? (
        <section className="mb-4">
          <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
            Muhtasari
          </h4>
          <p className="text-sm text-content">{analysis.summary}</p>
        </section>
      ) : null}

      <AnalysisList title="Hoja kuu" items={analysis.key_points} />
      <AnalysisList title="Mada" items={analysis.topics} />
      <AnalysisList title="Maswali ya kujipima" items={analysis.questions} />
      <AnalysisList title="Kazi zinazofuata" items={analysis.action_items} />

      {analysis.glossary && analysis.glossary.length > 0 ? (
        <section className="mt-4">
          <h4 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">
            Msamiati
          </h4>
          <dl className="flex flex-col gap-1.5 text-sm">
            {analysis.glossary.map((g, i) => (
              <div key={i} className="flex gap-2">
                <dt className="min-w-32 font-medium text-content">{g.term}</dt>
                <dd className="text-muted">{g.meaning}</dd>
              </div>
            ))}
          </dl>
        </section>
      ) : null}
    </EduCard>
  );
}

function AnalysisList({
  title,
  items,
}: {
  title: string;
  items?: string[];
}) {
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
