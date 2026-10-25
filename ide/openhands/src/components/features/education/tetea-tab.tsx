import { useCallback, useEffect, useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import {
  educationService,
  type EduTeteaPaper,
  type EduTeteaStatus,
  type EduTeteaSubject,
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

const EXAMS = [
  { value: "", label: "Mitihani yote" },
  { value: "csee", label: "CSEE (Kidato cha 4)" },
  { value: "acsee", label: "ACSEE (Kidato cha 6)" },
  { value: "dsee", label: "DSEE" },
  { value: "form_ii", label: "Kidato cha 2" },
  { value: "gatce", label: "GATCE" },
  { value: "psle", label: "PSLE (Darasa la 7)" },
  { value: "std4", label: "Darasa la 4" },
];

const KINDS = [
  { value: "paper", label: "Karatasi za maswali" },
  { value: "solutions", label: "Majibu (solutions)" },
  { value: "book", label: "Vitabu" },
];

/**
 * TETEA — karatasi halisi za NECTA kutoka maktaba.tetea.org.
 *
 * Hupakua PDF, huchimba maandishi (pdftotext, kisha OCR kwa skana), na
 * huingiza maswali kwenye benki. OCR hutumika pale ambapo mpangilio wa
 * maandishi ni mbovu (safu mbili au skana).
 */
export function TeteaTab() {
  const [status, setStatus] = useState<EduTeteaStatus | null>(null);
  const [subjects, setSubjects] = useState<EduTeteaSubject[]>([]);
  const [papers, setPapers] = useState<EduTeteaPaper[]>([]);
  const [exam, setExam] = useState("");
  const [subject, setSubject] = useState("");
  const [kind, setKind] = useState("paper");
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [preview, setPreview] = useState<{
    rel: string;
    chars: number;
    ocr: boolean;
    detected: number;
    text: string;
  } | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const [st, sub, pp] = await Promise.all([
        educationService.teteaStatus(),
        educationService.teteaSubjects(exam || undefined),
        educationService.teteaPapers({
          exam: exam || undefined,
          subject: subject || undefined,
          kind: kind || undefined,
          limit: 60,
        }),
      ]);
      setStatus(st);
      setSubjects(sub.subjects ?? []);
      setPapers(pp.papers ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [exam, subject, kind]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return papers;
    return papers.filter(
      (p) =>
        p.rel.toLowerCase().includes(q) ||
        p.label.toLowerCase().includes(q) ||
        String(p.year ?? "").includes(q),
    );
  }, [papers, search]);

  const buildIndex = async () => {
    setBusy("index");
    setError("");
    setNotice("");
    try {
      const r = await educationService.teteaBuildIndex();
      setNotice(`Faharasa imejengwa: karatasi ${r.total ?? 0}.`);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy("");
    }
  };

  const importBatch = async () => {
    setBusy("import");
    setError("");
    setNotice("");
    try {
      const r = await educationService.teteaImport({
        exam: exam || undefined,
        subject: subject || undefined,
        kind,
        limit: 25,
        ocr: 1,
        max_pages: 12,
      });
      setNotice(
        `Karatasi ${r.papers}: maswali ${r.questions_added} yameongezwa, ` +
          `${r.skipped_duplicates} yalikuwa yameingizwa, ` +
          `OCR ${r.ocr_papers}, kushindwa ${r.failures}.`,
      );
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy("");
    }
  };

  const previewPaper = async (id: string) => {
    setBusy(id);
    setError("");
    try {
      const r = await educationService.teteaExtract(id, {
        ocr: 1,
        max_pages: 12,
        limit: 12,
      });
      if (!r.ok) {
        setError(r.error || "Uchimbaji umeshindwa.");
        return;
      }
      setPreview({
        rel: r.rel,
        chars: r.chars,
        ocr: r.ocr_used,
        detected: r.questions_detected,
        text: r.text ?? "",
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy("");
    }
  };

  const examLabel = (e: string) => EXAMS.find((x) => x.value === e)?.label ?? e;

  return (
    <div className="flex flex-col gap-4">
      {error ? <EduError message={error} /> : null}
      {notice ? (
        <div
          data-testid="tetea-notice"
          className="rounded-md border border-border bg-surface-raised px-3 py-2 text-sm text-content"
        >
          {notice}
        </div>
      ) : null}

      {status ? (
        <EduCard testId="tetea-status">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <h2 className="text-sm font-semibold text-content">
                Maktaba ya NECTA — TETEA
              </h2>
              <p className="text-xs text-muted">{status.root}</p>
            </div>
            <EduButton
              variant="ghost"
              onClick={() => void buildIndex()}
              disabled={busy === "index"}
              testId="tetea-build-index"
            >
              <RefreshCw className="size-4" aria-hidden />
              {busy === "index" ? "Inajenga…" : "Jenga faharasa"}
            </EduButton>
          </div>
          <div className="mt-3 flex flex-wrap gap-1">
            <EduBadge>{status.indexed} karatasi</EduBadge>
            <EduBadge>{status.papers_imported} zilizoingizwa</EduBadge>
            <EduBadge>{status.questions_from_tetea} maswali</EduBadge>
            {status.by_exam.slice(0, 5).map((e) => (
              <EduBadge key={e.exam}>
                {e.exam}: {e.count}
              </EduBadge>
            ))}
            <EduBadge>OCR: {status.tools.tesseract ? "ipo" : "haipo"}</EduBadge>
          </div>
        </EduCard>
      ) : null}

      <EduCard>
        <div className="grid gap-3 sm:grid-cols-4">
          <EduField label="Mtihani">
            <EduSelect
              value={exam}
              onChange={(e) => {
                setExam(e.target.value);
                setSubject("");
              }}
              testId="tetea-exam"
            >
              {EXAMS.map((e) => (
                <option key={e.value} value={e.value}>
                  {e.label}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Somo">
            <EduSelect
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              testId="tetea-subject"
            >
              <option value="">Masomo yote</option>
              {subjects.map((s) => (
                <option key={`${s.exam}-${s.subject}`} value={s.subject}>
                  {s.label} ({s.count})
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Aina">
            <EduSelect
              value={kind}
              onChange={(e) => setKind(e.target.value)}
              testId="tetea-kind"
            >
              {KINDS.map((k) => (
                <option key={k.value} value={k.value}>
                  {k.label}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Tafuta" hint="Jina la faili au mwaka">
            <div className="relative">
              <Search
                className="pointer-events-none absolute top-2 left-2 size-4 text-muted"
                aria-hidden
              />
              <EduInput
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="mf. Agriculture 2023"
                className="w-full pl-7"
                testId="tetea-search"
              />
            </div>
          </EduField>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <EduButton
            onClick={() => void importBatch()}
            disabled={busy === "import"}
            testId="tetea-import"
          >
            {busy === "import" ? "Inaingiza…" : "Ingiza kundi (25)"}
          </EduButton>
          <EduButton
            variant="ghost"
            onClick={() => void refresh()}
            disabled={loading}
            testId="tetea-refresh"
          >
            {loading ? "Inapakia…" : "Onyesha upya"}
          </EduButton>
          {loading ? <EduSpinner /> : null}
        </div>
      </EduCard>

      {preview ? (
        <EduCard testId="tetea-preview">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <h3 className="text-sm font-semibold text-content">
                Maandishi yaliyochimbwa
              </h3>
              <p className="text-xs text-muted">{preview.rel}</p>
            </div>
            <div className="flex flex-wrap gap-1">
              <EduBadge>{preview.chars} herufi</EduBadge>
              <EduBadge>{preview.detected} maswali</EduBadge>
              <EduBadge>{preview.ocr ? "OCR" : "pdftotext"}</EduBadge>
              <EduButton
                variant="ghost"
                onClick={() => setPreview(null)}
                testId="tetea-preview-close"
              >
                Funga
              </EduButton>
            </div>
          </div>
          <pre className="mt-3 max-h-72 overflow-auto rounded-md bg-surface-raised p-3 text-xs whitespace-pre-wrap text-content">
            {preview.text}
          </pre>
        </EduCard>
      ) : null}

      <EduCard testId="tetea-papers">
        <h3 className="mb-2 text-sm font-semibold text-content">
          Karatasi ({filtered.length})
        </h3>
        {filtered.length === 0 ? (
          <EduEmpty>
            Hakuna karatasi zilizolingana. Jenga faharasa kwanza.
          </EduEmpty>
        ) : (
          <ul className="divide-y divide-border">
            {filtered.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap items-center justify-between gap-2 py-2"
              >
                <div className="min-w-0">
                  <p className="truncate text-sm text-content">
                    {p.label}
                    {p.year ? ` — ${p.year}` : ""}
                  </p>
                  <p className="truncate text-xs text-muted">
                    {examLabel(p.exam)} · {p.kind} · {p.rel}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  {p.imported > 0 ? (
                    <EduBadge>{p.imported} maswali</EduBadge>
                  ) : null}
                  <EduButton
                    variant="ghost"
                    onClick={() => void previewPaper(p.id)}
                    disabled={busy === p.id}
                    testId={`tetea-open-${p.id}`}
                  >
                    {busy === p.id ? "Inachimba…" : "Angalia"}
                  </EduButton>
                </div>
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}
