import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  BookOpen,
  ClipboardList,
  Megaphone,
  MessagesSquare,
  Plus,
  RefreshCw,
  Send,
  Sparkles,
} from "lucide-react";
import {
  teachingService,
  groupWsUrl,
  type TpAnnouncement,
  type TpAssignment,
  type TpGroup,
  type TpMessage,
  type TpNote,
  type TpStatus,
  type TpSubmission,
} from "#/api/education-service/teaching.api";
import {
  elimuproService,
  type EpCourse,
  type EpLevel,
  type EpSubject,
} from "#/api/education-service/elimupro.api";
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

type Section = "notes" | "assignments" | "groups" | "announcements";

const SECTIONS: { key: Section; label: string; icon: typeof BookOpen }[] = [
  { key: "notes", label: "Notes", icon: BookOpen },
  { key: "assignments", label: "Kazi", icon: ClipboardList },
  { key: "groups", label: "Vikundi & Chat", icon: MessagesSquare },
  { key: "announcements", label: "Matangazo", icon: Megaphone },
];

/**
 * ELIMUPRO — Awamu 2: ufundishaji.
 * Notes → wanafunzi, kazi (individual/kikundi) + AI-check, vikundi + chat,
 * matangazo.
 */
export function TeachingTab() {
  const [section, setSection] = useState<Section>("notes");
  const [status, setStatus] = useState<TpStatus | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await teachingService.status());
    } catch {
      /* takwimu si muhimu */
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  return (
    <div data-testid="teaching" className="flex flex-col gap-4">
      <EduCard testId="teaching-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Sparkles className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">
              ELIMUPRO — Ufundishaji (Awamu 2)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge>{status?.notes ?? 0} notes</EduBadge>
            <EduBadge>{status?.assignments ?? 0} kazi</EduBadge>
            <EduBadge>{status?.submissions ?? 0} majibu</EduBadge>
            <EduBadge>{status?.groups ?? 0} vikundi</EduBadge>
            <EduBadge>{status?.announcements ?? 0} matangazo</EduBadge>
            <EduButton variant="ghost" onClick={() => void refreshStatus()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      <nav className="flex flex-wrap gap-1 border-b border-border pb-1">
        {SECTIONS.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            type="button"
            data-testid={`teaching-section-${key}`}
            onClick={() => setSection(key)}
            className={
              "inline-flex items-center gap-1.5 rounded-t-md px-3 py-1.5 text-sm font-medium transition " +
              (section === key
                ? "border-b-2 border-primary text-primary"
                : "text-muted hover:text-content")
            }
          >
            <Icon className="size-4" aria-hidden />
            {label}
          </button>
        ))}
      </nav>

      {section === "notes" ? <NotesSection onChange={refreshStatus} /> : null}
      {section === "assignments" ? <AssignmentsSection onChange={refreshStatus} /> : null}
      {section === "groups" ? <GroupsSection onChange={refreshStatus} /> : null}
      {section === "announcements" ? <AnnouncementsSection onChange={refreshStatus} /> : null}
    </div>
  );
}

// ------------------------------------------------------------
// HOOK: levels + subjects (hutoka elimupro)
// ------------------------------------------------------------

function useLevelsAndSubjects() {
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [subjects, setSubjects] = useState<EpSubject[]>([]);

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.levels("");
        setLevels(r.levels);
        if (r.levels[0]) setLevelId(r.levels[0].id);
      } catch {
        /* hakuna levels bado */
      }
    })();
  }, []);

  useEffect(() => {
    if (!levelId) {
      setSubjects([]);
      return;
    }
    void (async () => {
      try {
        setSubjects((await elimuproService.subjects(levelId)).subjects);
      } catch {
        setSubjects([]);
      }
    })();
  }, [levelId]);

  return { levels, levelId, setLevelId, subjects };
}

// ------------------------------------------------------------
// 1. NOTES
// ------------------------------------------------------------

function NotesSection({ onChange }: { onChange: () => void }) {
  const { levels, levelId, setLevelId, subjects } = useLevelsAndSubjects();
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [subjectId, setSubjectId] = useState("");
  const [notes, setNotes] = useState<TpNote[]>([]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    if (subjects[0] && !subjectId) setSubjectId(subjects[0].id);
  }, [subjects, subjectId]);

  const load = useCallback(async () => {
    if (!levelId) return;
    try {
      setNotes((await teachingService.notes({ level_id: levelId })).notes);
    } catch (e) {
      setErr(String(e));
    }
  }, [levelId]);

  useEffect(() => {
    void load();
  }, [load]);

  const save = async () => {
    if (!subjectId || !title.trim()) {
      setErr("Chagua somo na uweke kichwa cha notes.");
      return;
    }
    setBusy(true);
    setErr("");
    setMsg("");
    try {
      const r = await teachingService.addNote({
        subject_id: subjectId,
        title: title.trim(),
        body,
        kind: "notes",
        level_ids: levelId ? [levelId] : [],
      });
      setMsg(`Notes imehifadhiwa na kushirikishwa level ${r.shared_levels}.`);
      setTitle("");
      setBody("");
      await load();
      onChange();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <EduCard>
        <div className="flex flex-col gap-3">
          <span className="text-sm font-semibold text-content">Notes mpya</span>
          <EduField label="Level">
            <EduSelect
              testId="tp-note-level"
              value={levelId}
              onChange={(e) => setLevelId(e.target.value)}
            >
              {levels.length === 0 ? <option value="">— hakuna level —</option> : null}
              {levels.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Somo">
            <EduSelect
              testId="tp-note-subject"
              value={subjectId}
              onChange={(e) => setSubjectId(e.target.value)}
            >
              {subjects.length === 0 ? <option value="">— hakuna somo —</option> : null}
              {subjects.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kichwa">
            <EduInput
              testId="tp-note-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Utangulizi wa Algoriti"
            />
          </EduField>
          <EduField label="Maudhui">
            <EduTextarea
              testId="tp-note-body"
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Andika notes hapa..."
            />
          </EduField>
          {err ? <EduError message={err} /> : null}
          {msg ? <p className="text-xs text-primary">{msg}</p> : null}
          <EduButton testId="tp-note-save" onClick={() => void save()} disabled={busy}>
            <Plus className="size-4" aria-hidden />
            {busy ? "Inahifadhi..." : "Hifadhi notes"}
          </EduButton>
        </div>
      </EduCard>

      <EduCard>
        <div className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-content">
            Notes za level hii ({notes.length})
          </span>
          {notes.length === 0 ? (
            <EduEmpty>Hakuna notes zilizoshirikishwa level hii bado.</EduEmpty>
          ) : (
            notes.map((n) => (
              <div key={n.id} className="rounded-md border border-border p-2">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium text-content">{n.title}</span>
                  <EduBadge>{n.kind}</EduBadge>
                </div>
                {n.body ? (
                  <p className="mt-1 line-clamp-3 text-xs text-muted">{n.body}</p>
                ) : null}
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}

// ------------------------------------------------------------
// 2. KAZI (ASSIGNMENTS)
// ------------------------------------------------------------

function AssignmentsSection({ onChange }: { onChange: () => void }) {
  const { levels, levelId, setLevelId, subjects } = useLevelsAndSubjects();
  const [subjectId, setSubjectId] = useState("");
  const [title, setTitle] = useState("");
  const [instructions, setInstructions] = useState("");
  const [mode, setMode] = useState<"individual" | "group">("individual");
  const [marks, setMarks] = useState("20");
  const [list, setList] = useState<TpAssignment[]>([]);
  const [selected, setSelected] = useState("");
  const [subs, setSubs] = useState<TpSubmission[]>([]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    if (subjects[0] && !subjectId) setSubjectId(subjects[0].id);
  }, [subjects, subjectId]);

  const load = useCallback(async () => {
    try {
      setList((await teachingService.assignments()).assignments);
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!selected) {
      setSubs([]);
      return;
    }
    void (async () => {
      try {
        setSubs((await teachingService.assignment(selected)).submissions);
      } catch {
        setSubs([]);
      }
    })();
  }, [selected]);

  const save = async () => {
    if (!subjectId || !title.trim()) {
      setErr("Chagua somo na uweke kichwa cha kazi.");
      return;
    }
    setBusy(true);
    setErr("");
    setMsg("");
    try {
      await teachingService.addAssignment({
        subject_id: subjectId,
        title: title.trim(),
        instructions,
        mode,
        marks: Number(marks) || 20,
      });
      setMsg("Kazi imehifadhiwa.");
      setTitle("");
      setInstructions("");
      await load();
      onChange();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <EduCard>
        <div className="flex flex-col gap-3">
          <span className="text-sm font-semibold text-content">Kazi mpya</span>
          <EduField label="Level">
            <EduSelect
              testId="tp-asg-level"
              value={levelId}
              onChange={(e) => setLevelId(e.target.value)}
            >
              {levels.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Somo">
            <EduSelect
              testId="tp-asg-subject"
              value={subjectId}
              onChange={(e) => setSubjectId(e.target.value)}
            >
              {subjects.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kichwa">
            <EduInput
              testId="tp-asg-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Zoezi la 1"
            />
          </EduField>
          <EduField label="Maelekezo">
            <EduTextarea
              testId="tp-asg-instructions"
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              placeholder="Eleza jinsi ya kuandika loop..."
            />
          </EduField>
          <div className="grid grid-cols-2 gap-3">
            <EduField label="Aina">
              <EduSelect
                testId="tp-asg-mode"
                value={mode}
                onChange={(e) => setMode(e.target.value as "individual" | "group")}
              >
                <option value="individual">Mtu mmoja</option>
                <option value="group">Kikundi</option>
              </EduSelect>
            </EduField>
            <EduField label="Alama za juu">
              <EduInput
                testId="tp-asg-marks"
                value={marks}
                onChange={(e) => setMarks(e.target.value)}
                type="number"
              />
            </EduField>
          </div>
          {err ? <EduError message={err} /> : null}
          {msg ? <p className="text-xs text-primary">{msg}</p> : null}
          <EduButton testId="tp-asg-save" onClick={() => void save()} disabled={busy}>
            <Plus className="size-4" aria-hidden />
            {busy ? "Inahifadhi..." : "Hifadhi kazi"}
          </EduButton>
        </div>
      </EduCard>

      <EduCard>
        <div className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-content">
            Kazi zilizopo ({list.length})
          </span>
          {list.length === 0 ? (
            <EduEmpty>Hakuna kazi bado.</EduEmpty>
          ) : (
            list.map((a) => (
              <button
                key={a.id}
                type="button"
                data-testid={`tp-asg-item-${a.id}`}
                onClick={() => setSelected(a.id)}
                className={
                  "rounded-md border p-2 text-left transition " +
                  (selected === a.id
                    ? "border-primary"
                    : "border-border hover:border-primary/60")
                }
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium text-content">{a.title}</span>
                  <EduBadge>{a.mode === "group" ? "kikundi" : "mtu mmoja"}</EduBadge>
                </div>
                <p className="mt-1 text-xs text-muted">
                  alama {a.marks} · majibu {a.submissions}
                </p>
              </button>
            ))
          )}
        </div>
      </EduCard>

      {selected ? (
        <EduCard testId="tp-asg-subs" className="lg:col-span-2">
          <div className="flex flex-col gap-2">
            <span className="text-sm font-semibold text-content">
              Majibu ({subs.length})
            </span>
            {subs.length === 0 ? (
              <EduEmpty>Hakuna majibu bado.</EduEmpty>
            ) : (
              subs.map((s) => (
                <div key={s.id} className="rounded-md border border-border p-2">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <span className="text-sm font-medium text-content">{s.student}</span>
                    <div className="flex gap-1">
                      <EduBadge>
                        AI: {s.ai_score >= 0 ? s.ai_score : "—"}
                      </EduBadge>
                      <EduBadge>
                        Mwalimu: {s.score >= 0 ? s.score : "—"}
                      </EduBadge>
                      <EduBadge>{s.status}</EduBadge>
                    </div>
                  </div>
                  {s.ai_feedback ? (
                    <p className="mt-1 text-xs text-muted">{s.ai_feedback}</p>
                  ) : null}
                </div>
              ))
            )}
          </div>
        </EduCard>
      ) : null}
    </div>
  );
}

// ------------------------------------------------------------
// 3. VIKUNDI & CHAT
// ------------------------------------------------------------

function GroupsSection({ onChange }: { onChange: () => void }) {
  const { levelId, subjects } = useLevelsAndSubjects();
  const [subjectId, setSubjectId] = useState("");
  const [name, setName] = useState("");
  const [groups, setGroups] = useState<TpGroup[]>([]);
  const [selected, setSelected] = useState("");
  const [messages, setMessages] = useState<TpMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [live, setLive] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (subjects[0] && !subjectId) setSubjectId(subjects[0].id);
  }, [subjects, subjectId]);

  const load = useCallback(async () => {
    try {
      setGroups((await teachingService.groups()).groups);
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // WebSocket ya chat: hujiunga na kikundi kilichochaguliwa.
  useEffect(() => {
    wsRef.current?.close();
    wsRef.current = null;
    setLive(false);
    if (!selected) {
      setMessages([]);
      return;
    }
    void (async () => {
      try {
        setMessages((await teachingService.messages(selected)).messages);
      } catch {
        setMessages([]);
      }
    })();
    try {
      const ws = new WebSocket(groupWsUrl(selected));
      wsRef.current = ws;
      ws.onopen = () => setLive(true);
      ws.onclose = () => setLive(false);
      ws.onmessage = (ev) => {
        try {
          const m = JSON.parse(String(ev.data)) as TpMessage & { type?: string };
          if (m.type === "message") {
            setMessages((prev) =>
              prev.some((x) => x.id === m.id) ? prev : [...prev, m],
            );
          }
        } catch {
          /* ujumbe mbovu */
        }
      };
    } catch {
      setLive(false);
    }
    return () => {
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [selected]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "nearest" });
  }, [messages]);

  const create = async () => {
    if (!subjectId || !name.trim()) {
      setErr("Chagua somo na uweke jina la kikundi.");
      return;
    }
    setBusy(true);
    setErr("");
    try {
      await teachingService.addGroup({ subject_id: subjectId, name: name.trim() });
      setName("");
      await load();
      onChange();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const send = async () => {
    if (!selected || !draft.trim()) return;
    const text = draft.trim();
    setDraft("");
    try {
      await teachingService.sendMessage(selected, {
        author: "Mwalimu",
        author_type: "teacher",
        body: text,
      });
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <EduCard>
        <div className="flex flex-col gap-3">
          <span className="text-sm font-semibold text-content">Kikundi kipya</span>
          <EduField label="Somo">
            <EduSelect
              testId="tp-grp-subject"
              value={subjectId}
              onChange={(e) => setSubjectId(e.target.value)}
            >
              {subjects.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Jina la kikundi">
            <EduInput
              testId="tp-grp-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Kikundi A"
            />
          </EduField>
          {err ? <EduError message={err} /> : null}
          <EduButton testId="tp-grp-save" onClick={() => void create()} disabled={busy}>
            <Plus className="size-4" aria-hidden />
            {busy ? "Inahifadhi..." : "Unda kikundi"}
          </EduButton>
          <p className="text-xs text-muted">Level: {levelId || "—"}</p>
          <div className="flex flex-col gap-1">
            {groups.map((g) => (
              <button
                key={g.id}
                type="button"
                data-testid={`tp-grp-item-${g.id}`}
                onClick={() => setSelected(g.id)}
                className={
                  "rounded-md border p-2 text-left text-sm transition " +
                  (selected === g.id
                    ? "border-primary text-primary"
                    : "border-border hover:border-primary/60")
                }
              >
                {g.name} · wanachama {g.members}
              </button>
            ))}
          </div>
        </div>
      </EduCard>

      <EduCard testId="tp-chat">
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-semibold text-content">Chat</span>
            <EduBadge>{live ? "live" : "imefungwa"}</EduBadge>
          </div>
          {!selected ? (
            <EduEmpty>Chagua kikundi kuanza chat.</EduEmpty>
          ) : (
            <>
              <div
                data-testid="tp-chat-messages"
                className="flex max-h-72 flex-col gap-1 overflow-y-auto rounded-md border border-border p-2"
              >
                {messages.length === 0 ? (
                  <span className="text-xs text-muted">Hakuna ujumbe bado.</span>
                ) : (
                  messages.map((m) => (
                    <div key={m.id} className="text-sm">
                      <span className="font-medium text-content">{m.author}: </span>
                      <span className="text-muted">{m.body}</span>
                    </div>
                  ))
                )}
                <div ref={bottomRef} />
              </div>
              <div className="flex gap-2">
                <EduInput
                  testId="tp-chat-input"
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void send();
                  }}
                  placeholder="Andika ujumbe..."
                />
                <EduButton testId="tp-chat-send" onClick={() => void send()}>
                  <Send className="size-4" aria-hidden />
                </EduButton>
              </div>
            </>
          )}
        </div>
      </EduCard>
    </div>
  );
}

// ------------------------------------------------------------
// 4. MATANGAZO
// ------------------------------------------------------------

function AnnouncementsSection({ onChange }: { onChange: () => void }) {
  const { levels, levelId, setLevelId } = useLevelsAndSubjects();
  const [courses, setCourses] = useState<EpCourse[]>([]);
  const [courseId, setCourseId] = useState("");
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [target, setTarget] = useState("all");
  const [list, setList] = useState<TpAnnouncement[]>([]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.courses();
        setCourses(r.courses);
        if (r.courses[0]) setCourseId(r.courses[0].id);
      } catch {
        /* hakuna kozi bado */
      }
    })();
  }, []);

  const load = useCallback(async () => {
    try {
      setList((await teachingService.announcements()).announcements);
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const save = async () => {
    if (!title.trim()) {
      setErr("Kichwa cha tangazo linahitajika.");
      return;
    }
    setBusy(true);
    setErr("");
    setMsg("");
    try {
      await teachingService.addAnnouncement({
        title: title.trim(),
        body,
        target,
        target_id: target === "level" ? levelId : target === "course" ? courseId : "",
        author: "MBILINYI",
      });
      setMsg("Tangazo limetumwa.");
      setTitle("");
      setBody("");
      await load();
      onChange();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <EduCard>
        <div className="flex flex-col gap-3">
          <span className="text-sm font-semibold text-content">Tangazo jipya</span>
          <EduField label="Kichwa">
            <EduInput
              testId="tp-ann-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Mtihani wa kati ya muhula"
            />
          </EduField>
          <EduField label="Maudhui">
            <EduTextarea
              testId="tp-ann-body"
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Andika tangazo..."
            />
          </EduField>
          <div className="grid grid-cols-2 gap-3">
            <EduField label="Walengwa">
              <EduSelect
                testId="tp-ann-target"
                value={target}
                onChange={(e) => setTarget(e.target.value)}
              >
                <option value="all">Wote</option>
                <option value="students">Wanafunzi</option>
                <option value="teachers">Walimu</option>
                <option value="level">Level</option>
                <option value="course">Kozi</option>
              </EduSelect>
            </EduField>
            {target === "level" ? (
              <EduField label="Level">
                <EduSelect
                  testId="tp-ann-level"
                  value={levelId}
                  onChange={(e) => setLevelId(e.target.value)}
                >
                  {levels.map((l) => (
                    <option key={l.id} value={l.id}>
                      {l.name}
                    </option>
                  ))}
                </EduSelect>
              </EduField>
            ) : null}
            {target === "course" ? (
              <EduField label="Kozi">
                <EduSelect
                  testId="tp-ann-course"
                  value={courseId}
                  onChange={(e) => setCourseId(e.target.value)}
                >
                  {courses.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </EduSelect>
              </EduField>
            ) : null}
          </div>
          {err ? <EduError message={err} /> : null}
          {msg ? <p className="text-xs text-primary">{msg}</p> : null}
          <EduButton testId="tp-ann-save" onClick={() => void save()} disabled={busy}>
            <Megaphone className="size-4" aria-hidden />
            {busy ? "Inatuma..." : "Tuma tangazo"}
          </EduButton>
        </div>
      </EduCard>

      <EduCard>
        <div className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-content">
            Matangazo ({list.length})
          </span>
          {list.length === 0 ? (
            <EduEmpty>Hakuna matangazo bado.</EduEmpty>
          ) : (
            list.map((a) => (
              <div key={a.id} className="rounded-md border border-border p-2">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium text-content">{a.title}</span>
                  <EduBadge>{a.target === "all" ? "wote" : a.target}</EduBadge>
                </div>
                {a.body ? (
                  <p className="mt-1 text-xs text-muted">{a.body}</p>
                ) : null}
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
