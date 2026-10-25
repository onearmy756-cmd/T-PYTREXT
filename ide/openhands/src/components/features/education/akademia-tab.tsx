import { useCallback, useEffect, useState } from "react";
import { GraduationCap, ShieldCheck, Plus, Trash2, Newspaper } from "lucide-react";
import {
  akademiaService,
  type AkStatus,
  type AskResult,
  type DocRow,
} from "#/api/education-service/akademia.api";
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
  EduTextarea,
} from "./edu-ui";

/**
 * MTECHOS — Sehemu 2 & 4: Akademia ya AI (moduli 23).
 * Sehemu 2: AI ya usalama (kufundisha, muhtasari wa CVE/habari).
 * Sehemu 4: Teknolojia, Afya, Kilimo, Fedha, Hali ya hewa — RAG kwa kila nyanja.
 */
export function AkademiaTab() {
  const [status, setStatus] = useState<AkStatus | null>(null);
  const [docs, setDocs] = useState<DocRow[]>([]);
  const [domain, setDomain] = useState("security");
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState<AskResult | null>(null);
  const [brief, setBrief] = useState<{ news: { title: string; url: string }[]; cves: { id: string; desc: string }[] } | null>(null);
  const [lesson, setLesson] = useState<{ utangulizi?: string; hatua?: string[]; mifano?: string[]; maswali?: string[] } | null>(null);

  const [newTitle, setNewTitle] = useState("");
  const [newBody, setNewBody] = useState("");
  const [newSource, setNewSource] = useState("");

  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  const refresh = useCallback(async () => {
    try {
      setStatus(await akademiaService.status());
      setDocs((await akademiaService.docs(domain)).docs);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [domain]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const doAsk = async () => {
    if (!question.trim()) return;
    setBusy(true);
    try {
      setAnswer(await akademiaService.ask({ question: question.trim(), domain, web: true }));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doTeach = async () => {
    if (!question.trim()) return;
    setBusy(true);
    try {
      const r = await akademiaService.teach({ topic: question.trim(), domain });
      setLesson(r.lesson);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doBrief = async () => {
    setBusy(true);
    try {
      const r = await akademiaService.securityBrief({});
      setBrief({ news: r.news, cves: r.cves });
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addDoc = async () => {
    if (!newTitle.trim() || !newBody.trim()) return;
    setBusy(true);
    try {
      await akademiaService.docAdd({
        domain,
        title: newTitle.trim(),
        body: newBody.trim(),
        source: newSource.trim(),
      });
      setNewTitle("");
      setNewBody("");
      setNewSource("");
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3" data-testid="akademia-tab">
      <EduCard testId="akademia-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">
            Akademia ya AI — usalama, teknolojia, afya, kilimo, fedha, hali ya hewa
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="akademia-docs">{status?.docs_total ?? 0} hati</EduBadge>
            <EduBadge testId="akademia-asks">{status?.asks ?? 0} maswali</EduBadge>
            <EduBadge>{status?.llm ? "LLM ipo" : "RAG pekee"}</EduBadge>
          </div>
        </div>
        <p className="mt-1 text-xs text-muted">
          RAG halisi (cosine kwenye vector.rs) + web halisi (DuckDuckGo) + NVD kwa CVE.
        </p>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      <EduCard testId="akademia-domain">
        <div className="grid gap-2 md:grid-cols-3">
          <EduField label="Nyanja">
            <EduSelect testId="akademia-domain-select" value={domain} onChange={(e) => setDomain(e.target.value)}>
              {(status?.domains ?? []).map((d) => (
                <option key={d.id} value={d.id}>
                  {d.label} ({d.docs})
                </option>
              ))}
            </EduSelect>
          </EduField>
          <div className="md:col-span-2">
            <EduField label="Swali au mada">
              <EduInput
                testId="akademia-question"
                value={question}
                onChange={(e) => setQuestion(e.target.value)}
                placeholder="mf. Nifanyeje nikipata barua pepe ya ulaghai?"
                onKeyDown={(e) => e.key === "Enter" && void doAsk()}
              />
            </EduField>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <EduButton testId="akademia-ask" disabled={busy} onClick={() => void doAsk()}>
            <ShieldCheck className="size-4" aria-hidden /> Uliza
          </EduButton>
          <EduButton testId="akademia-teach" disabled={busy} onClick={() => void doTeach()}>
            <GraduationCap className="size-4" aria-hidden /> Nifundishe
          </EduButton>
          <EduButton testId="akademia-brief" disabled={busy} onClick={() => void doBrief()}>
            <Newspaper className="size-4" aria-hidden /> Muhtasari wa usalama
          </EduButton>
          {busy ? <EduSpinner label="Inafanya kazi…" /> : null}
        </div>
      </EduCard>

      {answer ? (
        <EduCard testId="akademia-answer">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="text-sm font-semibold text-content">Jibu</span>
            <EduBadge testId="akademia-engine">{answer.engine}</EduBadge>
          </div>
          <p className="mt-2 whitespace-pre-wrap text-sm text-content">{answer.answer}</p>
          {answer.doc_sources.length ? (
            <div className="mt-2">
              <span className="text-xs font-semibold uppercase text-muted">Vyanzo (hati zako)</span>
              <ul className="ml-4 list-disc text-xs text-content">
                {answer.doc_sources.map((s, i) => (
                  <li key={`doc-${i}`}>
                    {s.title} <span className="text-muted">({s.score})</span>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
          {answer.web_sources.length ? (
            <div className="mt-2">
              <span className="text-xs font-semibold uppercase text-muted">Vyanzo (mtandaoni)</span>
              <ul className="ml-4 list-disc text-xs">
                {answer.web_sources.map((s, i) => (
                  <li key={`web-${i}`}>
                    <a href={s.url} target="_blank" rel="noreferrer" className="text-primary underline">
                      {s.title}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
          <p className="mt-2 text-xs text-muted">{answer.note_sw}</p>
        </EduCard>
      ) : null}

      {lesson ? (
        <EduCard testId="akademia-lesson">
          <span className="text-sm font-semibold text-content">Somo</span>
          {lesson.utangulizi ? <p className="mt-1 text-sm text-content">{lesson.utangulizi}</p> : null}
          {lesson.hatua?.length ? (
            <ol className="ml-4 mt-2 list-decimal text-sm text-content">
              {lesson.hatua.map((h, i) => (
                <li key={`lh-${i}`}>{h}</li>
              ))}
            </ol>
          ) : null}
          {lesson.mifano?.length ? (
            <div className="mt-2">
              <span className="text-xs font-semibold uppercase text-muted">Mifano</span>
              <ul className="ml-4 list-disc text-sm text-content">
                {lesson.mifano.map((m, i) => (
                  <li key={`lm-${i}`}>{m}</li>
                ))}
              </ul>
            </div>
          ) : null}
          {lesson.maswali?.length ? (
            <div className="mt-2">
              <span className="text-xs font-semibold uppercase text-muted">Maswali ya mazoezi</span>
              <ul className="ml-4 list-disc text-sm text-content">
                {lesson.maswali.map((q, i) => (
                  <li key={`lq-${i}`}>{q}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </EduCard>
      ) : null}

      {brief ? (
        <EduCard testId="akademia-brief-out">
          <span className="text-sm font-semibold text-content">Muhtasari wa usalama (halisi)</span>
          {brief.cves.length ? (
            <div className="mt-2">
              <span className="text-xs font-semibold uppercase text-muted">CVE za hivi karibuni (NVD)</span>
              <div className="mt-1 flex flex-col gap-1">
                {brief.cves.map((c) => (
                  <div key={c.id} className="rounded border border-border p-2 text-xs">
                    <div className="font-semibold text-content">{c.id}</div>
                    <div className="text-muted">{c.desc}</div>
                  </div>
                ))}
              </div>
            </div>
          ) : null}
          {brief.news.length ? (
            <div className="mt-2">
              <span className="text-xs font-semibold uppercase text-muted">Habari</span>
              <ul className="ml-4 list-disc text-xs">
                {brief.news.map((n, i) => (
                  <li key={`n-${i}`}>
                    <a href={n.url} target="_blank" rel="noreferrer" className="text-primary underline">
                      {n.title}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </EduCard>
      ) : null}

      <EduCard testId="akademia-add">
        <span className="text-sm font-semibold text-content">Ongeza maudhui ya mwalimu ({domain})</span>
        <div className="mt-2 grid gap-2 md:grid-cols-2">
          <EduField label="Kichwa">
            <EduInput testId="akademia-new-title" value={newTitle} onChange={(e) => setNewTitle(e.target.value)} />
          </EduField>
          <EduField label="Chanzo (hiari)">
            <EduInput testId="akademia-new-source" value={newSource} onChange={(e) => setNewSource(e.target.value)} />
          </EduField>
        </div>
        <div className="mt-2">
          <EduField label="Maudhui">
            <EduTextarea
              testId="akademia-new-body"
              value={newBody}
              onChange={(e) => setNewBody(e.target.value)}
              className="min-h-24"
            />
          </EduField>
        </div>
        <div className="mt-2">
          <EduButton testId="akademia-add-btn" disabled={busy} onClick={() => void addDoc()}>
            <Plus className="size-4" aria-hidden /> Ongeza
          </EduButton>
        </div>
      </EduCard>

      <EduCard testId="akademia-docs-list">
        <span className="text-sm font-semibold text-content">Hati za nyanja ({docs.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {docs.length === 0 ? (
            <EduEmpty>Hakuna hati kwenye nyanja hii bado.</EduEmpty>
          ) : (
            docs.map((d) => (
              <div
                key={d.id}
                data-testid={`akademia-doc-${d.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <div className="min-w-0">
                  <div className="text-sm text-content">{d.title}</div>
                  <div className="truncate text-xs text-muted">{d.preview}</div>
                </div>
                <EduButton
                  testId={`akademia-doc-del-${d.id}`}
                  onClick={() => void akademiaService.docRemove(d.id).then(refresh)}
                >
                  <Trash2 className="size-4" aria-hidden />
                </EduButton>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
