import { useCallback, useEffect, useMemo, useState } from "react";
import {
  CalendarDays,
  CheckCircle2,
  Clock,
  MapPin,
  Plus,
  RefreshCw,
  Trash2,
  UserCheck,
  XCircle,
} from "lucide-react";
import {
  attendanceService,
  type AtPeriod,
  type AtSession,
  type AtStatus,
  type AtStudentMark,
} from "#/api/education-service/attendance.api";
import { elimuproService, type EpLevel, type EpSubject } from "#/api/education-service/elimupro.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
} from "./edu-ui";

const DAYS = ["", "Jumatatu", "Jumanne", "Jumatano", "Alhamisi", "Ijumaa", "Jumamosi", "Jumapili"];
const STATUSES = [
  { key: "present", label: "Yupo", icon: CheckCircle2 },
  { key: "late", label: "Amechelewa", icon: Clock },
  { key: "excused", label: "Ruhusa", icon: UserCheck },
  { key: "absent", label: "Hayupo", icon: XCircle },
];

/**
 * ELIMUPRO — Awamu 7: ratiba (timetable) + mahudhurio (attendance).
 * Ratiba huzuia migongano HALISI (mwalimu/level/chumba). Kikao hufunguliwa
 * kwa kipindi + tarehe; wanafunzi wa level hujazwa. Mwalimu huweka hali au
 * mwanafunzi hujisajili kwa kodi ya kikao.
 */
export function AttendanceTab() {
  const [status, setStatus] = useState<AtStatus | null>(null);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [periods, setPeriods] = useState<AtPeriod[]>([]);
  const [sessions, setSessions] = useState<AtSession[]>([]);
  const [openSession, setOpenSession] = useState<{ id: string; code: string; students: AtStudentMark[]; subject: string; percent: number } | null>(null);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");

  // Fomu ya kipindi
  const [subjectId, setSubjectId] = useState("");
  const [day, setDay] = useState("1");
  const [start, setStart] = useState("08:00");
  const [end, setEnd] = useState("10:00");
  const [room, setRoom] = useState("");
  const [date, setDate] = useState(() => new Date().toISOString().slice(0, 10));

  const refresh = useCallback(async () => {
    try {
      setStatus(await attendanceService.status());
      setPeriods((await attendanceService.periods({ level_id: levelId })).periods);
      setSessions((await attendanceService.sessions({ level_id: levelId })).sessions);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [levelId]);

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.levels("");
        setLevels(r.levels);
        if (r.levels[0]) setLevelId(r.levels[0].id);
      } catch (e) {
        setErr(String(e));
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
        setSubjectId("");
      }
      await refresh();
    })();
  }, [levelId, refresh]);

  const addPeriod = async () => {
    try {
      const r = await attendanceService.addPeriod({
        level_id: levelId,
        subject_id: subjectId,
        day: Number(day),
        start,
        end,
        room,
      });
      if (!r.ok) {
        setErr(`Mgongano: ${r.error} (${r.kind ?? "?"})`);
        return;
      }
      setErr("");
      setMsg(`Kipindi kimeongezwa (${start}-${end}).`);
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  };

  const openSess = async (p: AtPeriod) => {
    try {
      const r = await attendanceService.openSession({ timetable_id: p.id, date });
      const full = await attendanceService.session(r.id);
      setOpenSession({
        id: r.id,
        code: r.code,
        students: full.students,
        subject: p.subject,
        percent: full.session.percent,
      });
      setMsg(`Kikao kimefunguliwa. Kodi ya kujisajili: ${r.code}`);
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  };

  const setStatusFor = async (studentId: string, st: string) => {
    if (!openSession) return;
    try {
      await attendanceService.mark({ session_id: openSession.id, student_id: studentId, status: st });
      const full = await attendanceService.session(openSession.id);
      setOpenSession({ ...openSession, students: full.students, percent: full.session.percent });
    } catch (e) {
      setErr(String(e));
    }
  };

  const markAllPresent = async () => {
    if (!openSession) return;
    try {
      await attendanceService.markBulk({
        session_id: openSession.id,
        marks: openSession.students.map((s) => ({ student_id: s.student_id, status: "present" })),
      });
      const full = await attendanceService.session(openSession.id);
      setOpenSession({ ...openSession, students: full.students, percent: full.session.percent });
    } catch (e) {
      setErr(String(e));
    }
  };

  const closeSess = async () => {
    if (!openSession) return;
    try {
      await attendanceService.closeSession(openSession.id);
      setOpenSession(null);
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  };

  const grouped = useMemo(() => {
    const m: Record<number, AtPeriod[]> = {};
    periods.forEach((p) => {
      (m[p.day] ??= []).push(p);
    });
    return m;
  }, [periods]);

  return (
    <div data-testid="attendance" className="flex flex-col gap-4">
      <EduCard testId="att-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <CalendarDays className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">
              Ratiba + Mahudhurio (Awamu 7)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="att-period-count">{status?.periods ?? 0} vipindi</EduBadge>
            <EduBadge>{status?.sessions ?? 0} vikao</EduBadge>
            <EduBadge>{status?.open ?? 0} wazi</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard>
        <div className="grid gap-3 md:grid-cols-3">
          <EduField label="Level">
            <EduSelect testId="att-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
              {levels.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Somo">
            <EduSelect testId="att-subject" value={subjectId} onChange={(e) => setSubjectId(e.target.value)}>
              {subjects.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Siku">
            <EduSelect testId="att-day" value={day} onChange={(e) => setDay(e.target.value)}>
              {DAYS.slice(1).map((d, i) => (
                <option key={d} value={i + 1}>
                  {d}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Kuanza (HH:MM)">
            <EduInput testId="att-start" value={start} onChange={(e) => setStart(e.target.value)} />
          </EduField>
          <EduField label="Kumaliza (HH:MM)">
            <EduInput testId="att-end" value={end} onChange={(e) => setEnd(e.target.value)} />
          </EduField>
          <EduField label="Chumba">
            <EduInput testId="att-room" value={room} onChange={(e) => setRoom(e.target.value)} placeholder="A101" />
          </EduField>
          <div className="md:col-span-3">
            <EduButton testId="att-add-period" onClick={() => void addPeriod()} disabled={!levelId || !subjectId}>
              <Plus className="size-4" aria-hidden /> Ongeza kipindi
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard testId="att-timetable">
        <span className="text-sm font-semibold text-content">Ratiba ya wiki</span>
        <div className="mt-2 flex flex-col gap-2">
          {periods.length === 0 ? (
            <EduEmpty>Hakuna vipindi bado.</EduEmpty>
          ) : (
            DAYS.slice(1).map((d, i) => {
              const list = grouped[i + 1] ?? [];
              if (list.length === 0) return null;
              return (
                <div key={d}>
                  <p className="text-xs font-semibold text-muted">{d}</p>
                  <div className="flex flex-col gap-1">
                    {list.map((p) => (
                      <div
                        key={p.id}
                        data-testid={`att-period-${p.id}`}
                        className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
                      >
                        <div className="flex items-center gap-2">
                          <EduBadge>
                            <Clock className="size-3" aria-hidden /> {p.start}-{p.end}
                          </EduBadge>
                          <span className="text-sm text-content">{p.subject}</span>
                          {p.room ? (
                            <EduBadge>
                              <MapPin className="size-3" aria-hidden /> {p.room}
                            </EduBadge>
                          ) : null}
                        </div>
                        <div className="flex gap-1">
                          <EduButton variant="ghost" testId={`att-open-${p.id}`} onClick={() => void openSess(p)}>
                            Fungua kikao
                          </EduButton>
                          <EduButton variant="ghost" onClick={() => void attendanceService.deletePeriod(p.id).then(refresh)}>
                            <Trash2 className="size-3.5" aria-hidden />
                          </EduButton>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </EduCard>

      {openSession ? (
        <EduCard testId="att-session">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm font-semibold text-content">
                Kikao: {openSession.subject}
              </span>
              <EduBadge testId="att-code">kodi {openSession.code}</EduBadge>
              <EduBadge testId="att-percent">mahudhurio {openSession.percent >= 0 ? `${openSession.percent}%` : "—"}</EduBadge>
            </div>
            <div className="flex gap-1">
              <EduButton variant="ghost" testId="att-all-present" onClick={() => void markAllPresent()}>
                Wote wapo
              </EduButton>
              <EduButton variant="ghost" testId="att-close" onClick={() => void closeSess()}>
                Funga kikao
              </EduButton>
            </div>
          </div>
          <div className="mt-3 flex flex-col gap-1">
            {openSession.students.map((s) => (
              <div
                key={s.student_id}
                data-testid={`att-student-${s.student_id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {s.student} <span className="text-xs text-muted">({s.reg_no})</span>
                </span>
                <div className="flex gap-1">
                  {STATUSES.map((st) => {
                    const Icon = st.icon;
                    const active = s.status === st.key;
                    return (
                      <button
                        key={st.key}
                        type="button"
                        data-testid={`att-set-${s.student_id}-${st.key}`}
                        onClick={() => void setStatusFor(s.student_id, st.key)}
                        className={
                          "inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs transition " +
                          (active
                            ? "border-primary bg-primary/10 text-primary"
                            : "border-border text-muted hover:border-primary/50")
                        }
                      >
                        <Icon className="size-3" aria-hidden /> {st.label}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </EduCard>
      ) : null}

      <EduCard testId="att-sessions">
        <span className="text-sm font-semibold text-content">Vikao ({sessions.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {sessions.length === 0 ? (
            <EduEmpty>Hakuna vikao bado.</EduEmpty>
          ) : (
            sessions.map((s) => (
              <div key={s.id} className="flex items-center justify-between rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {s.date} — {s.subject}
                </span>
                <div className="flex gap-1">
                  <EduBadge>{s.present}/{s.total} wapo</EduBadge>
                  <EduBadge>{s.open ? "wazi" : "imefungwa"}</EduBadge>
                  <EduBadge>{s.code}</EduBadge>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
