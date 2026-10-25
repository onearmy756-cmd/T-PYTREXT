import { useCallback, useEffect, useState } from "react";
import {
  Building2,
  GraduationCap,
  Layers,
  UserCog,
  Users,
  Plus,
  CheckCircle2,
  RefreshCw,
  IdCard,
} from "lucide-react";
import {
  elimuproService,
  idCardUrl,
  type EpCourse,
  type EpInstitution,
  type EpLevel,
  type EpStatus,
  type EpStudent,
  type EpStudentDetail,
  type EpSubject,
  type EpTeacher,
  type EpTeacherDetail,
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
} from "./edu-ui";

type Section = "institution" | "courses" | "teachers" | "students";

const SECTIONS: { key: Section; label: string; icon: typeof Building2 }[] = [
  { key: "institution", label: "Taasisi", icon: Building2 },
  { key: "courses", label: "Masomo", icon: Layers },
  { key: "teachers", label: "Walimu", icon: UserCog },
  { key: "students", label: "Wanafunzi", icon: Users },
];

/**
 * ELIMUPRO — Awamu 1: msingi wa chuo.
 * Taasisi → courses/levels/subjects → walimu (na kugawa masomo) →
 * usajili wa wanafunzi (NECTA/RITA) → enrollment → ID card.
 */
export function ElimuproTab() {
  const [section, setSection] = useState<Section>("institution");
  const [status, setStatus] = useState<EpStatus | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await elimuproService.status());
    } catch {
      /* takwimu si muhimu */
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  return (
    <div data-testid="elimupro" className="flex flex-col gap-4">
      <EduCard testId="elimupro-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <GraduationCap className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">
              ELIMUPRO — Msingi wa Chuo (Awamu 1)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge>{status?.institutions ?? 0} taasisi</EduBadge>
            <EduBadge>{status?.courses ?? 0} course</EduBadge>
            <EduBadge>{status?.subjects ?? 0} masomo</EduBadge>
            <EduBadge>{status?.teachers ?? 0} walimu</EduBadge>
            <EduBadge>{status?.students ?? 0} wanafunzi</EduBadge>
            <EduBadge>NECTA: {status?.necta_api ? "API" : "manual"}</EduBadge>
            <EduBadge>RITA: {status?.rita_api ? "API" : "manual"}</EduBadge>
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
            data-testid={`elimupro-section-${key}`}
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

      {section === "institution" ? <InstitutionSection onChange={refreshStatus} /> : null}
      {section === "courses" ? <CoursesSection /> : null}
      {section === "teachers" ? <TeachersSection /> : null}
      {section === "students" ? <StudentsSection /> : null}
    </div>
  );
}

// ------------------------------------------------------------
// 1. TAASISI
// ------------------------------------------------------------

function InstitutionSection({ onChange }: { onChange: () => void }) {
  const [form, setForm] = useState<EpInstitution>({
    id: "",
    name: "",
    code: "",
    motto: "",
    address: "",
    phone: "",
    email: "",
    website: "",
    description: "",
    logo: "",
  });
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState("");
  const [err, setErr] = useState("");

  useEffect(() => {
    void (async () => {
      try {
        const r = await elimuproService.institution();
        if (r.institution) setForm(r.institution);
      } catch {
        /* hakuna taasisi bado */
      }
    })();
  }, []);

  const set = (k: keyof EpInstitution, v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  const save = async () => {
    setBusy(true);
    setErr("");
    setMsg("");
    try {
      const r = await elimuproService.saveInstitution(form);
      setMsg(`Taasisi imehifadhiwa. Code: ${r.code}`);
      onChange();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    } finally {
      setBusy(false);
    }
  };

  return (
    <EduCard>
      <h3 className="mb-3 text-sm font-semibold text-content">
        Taasisi (msingi wa kila kitu)
      </h3>
      <div className="grid gap-3 md:grid-cols-2">
        <EduField label="Jina la chuo">
          <EduInput
            testId="ep-inst-name"
            value={form.name}
            onChange={(e) => set("name", e.target.value)}
            placeholder="Chuo cha Teknolojia MTECHOS"
          />
        </EduField>
        <EduField label="Code (prefix ya reg no)" hint="mf. CTM">
          <EduInput
            testId="ep-inst-code"
            value={form.code}
            onChange={(e) => set("code", e.target.value.toUpperCase())}
            placeholder="CTM"
          />
        </EduField>
        <EduField label="Motto">
          <EduInput value={form.motto} onChange={(e) => set("motto", e.target.value)} />
        </EduField>
        <EduField label="Anwani">
          <EduInput value={form.address} onChange={(e) => set("address", e.target.value)} />
        </EduField>
        <EduField label="Simu">
          <EduInput value={form.phone} onChange={(e) => set("phone", e.target.value)} />
        </EduField>
        <EduField label="Barua pepe">
          <EduInput value={form.email} onChange={(e) => set("email", e.target.value)} />
        </EduField>
        <EduField label="Website">
          <EduInput value={form.website} onChange={(e) => set("website", e.target.value)} />
        </EduField>
        <EduField label="Maelezo">
          <EduInput
            value={form.description}
            onChange={(e) => set("description", e.target.value)}
          />
        </EduField>
      </div>
      <div className="mt-4 flex items-center gap-3">
        <EduButton testId="ep-inst-save" onClick={() => void save()} disabled={busy}>
          {busy ? <EduSpinner /> : <CheckCircle2 className="size-4" aria-hidden />}
          Hifadhi Taasisi
        </EduButton>
        {msg ? (
          <span className="text-sm text-feedback-success" data-testid="ep-inst-msg">
            {msg}
          </span>
        ) : null}
      </div>
      {err ? <div className="mt-3"><EduError message={err} /></div> : null}
    </EduCard>
  );
}

// ------------------------------------------------------------
// 2. COURSES / LEVELS / SUBJECTS
// ------------------------------------------------------------

function CoursesSection() {
  const [courses, setCourses] = useState<EpCourse[]>([]);
  const [selectedCourse, setSelectedCourse] = useState("");
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [selectedLevel, setSelectedLevel] = useState("");
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [err, setErr] = useState("");

  const [course, setCourse] = useState({
    name: "",
    code: "",
    award: "Diploma",
    years: 2,
    auto_levels: true,
  });
  const [subject, setSubject] = useState({ name: "", code: "", credits: 10 });

  const loadCourses = useCallback(async () => {
    try {
      const r = await elimuproService.courses();
      setCourses(r.courses ?? []);
      if (r.courses?.length && !selectedCourse) setSelectedCourse(r.courses[0].id);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  }, [selectedCourse]);

  useEffect(() => {
    void loadCourses();
  }, [loadCourses]);

  useEffect(() => {
    if (!selectedCourse) return;
    void (async () => {
      try {
        const r = await elimuproService.levels(selectedCourse);
        setLevels(r.levels ?? []);
        if (r.levels?.length) setSelectedLevel(r.levels[0].id);
      } catch {
        /* */
      }
    })();
  }, [selectedCourse]);

  useEffect(() => {
    if (!selectedLevel) return;
    void (async () => {
      try {
        const r = await elimuproService.subjects(selectedLevel);
        setSubjects(r.subjects ?? []);
      } catch {
        /* */
      }
    })();
  }, [selectedLevel]);

  const addCourse = async () => {
    setErr("");
    try {
      await elimuproService.addCourse(course);
      setCourse({ name: "", code: "", award: "Diploma", years: 2, auto_levels: true });
      await loadCourses();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  };

  const addSubject = async () => {
    if (!selectedLevel) return;
    setErr("");
    try {
      await elimuproService.addSubject({ level_id: selectedLevel, ...subject });
      setSubject({ name: "", code: "", credits: 10 });
      const r = await elimuproService.subjects(selectedLevel);
      setSubjects(r.subjects ?? []);
      await loadCourses();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Sajili Course mpya
        </h3>
        <div className="grid gap-3 md:grid-cols-4">
          <EduField label="Jina la course">
            <EduInput
              testId="ep-course-name"
              value={course.name}
              onChange={(e) => setCourse({ ...course, name: e.target.value })}
              placeholder="Diploma in Computer Science"
            />
          </EduField>
          <EduField label="Code">
            <EduInput
              testId="ep-course-code"
              value={course.code}
              onChange={(e) => setCourse({ ...course, code: e.target.value })}
              placeholder="DCS"
            />
          </EduField>
          <EduField label="Award">
            <EduSelect
              value={course.award}
              onChange={(e) => setCourse({ ...course, award: e.target.value })}
            >
              <option>Certificate</option>
              <option>Diploma</option>
              <option>Degree</option>
              <option>Masters</option>
            </EduSelect>
          </EduField>
          <EduField label="Miaka">
            <EduInput
              type="number"
              min={1}
              max={6}
              value={course.years}
              onChange={(e) =>
                setCourse({ ...course, years: Number(e.target.value) || 1 })
              }
            />
          </EduField>
        </div>
        <label className="mt-3 flex items-center gap-2 text-sm text-content">
          <input
            type="checkbox"
            checked={course.auto_levels}
            onChange={(e) => setCourse({ ...course, auto_levels: e.target.checked })}
          />
          Tengeneza levels (Year/Semester) automatically
        </label>
        <div className="mt-3">
          <EduButton testId="ep-course-add" onClick={() => void addCourse()}>
            <Plus className="size-4" aria-hidden />
            Sajili Course
          </EduButton>
        </div>
      </EduCard>

      <EduCard testId="ep-courses-list">
        <h3 className="mb-3 text-sm font-semibold text-content">
          Courses ({courses.length})
        </h3>
        {courses.length === 0 ? (
          <EduEmpty>Sajili taasisi kwanza, kisha course.</EduEmpty>
        ) : (
          <div className="mb-4 flex flex-wrap gap-1">
            {courses.map((c) => (
              <button
                key={c.id}
                type="button"
                data-testid={`ep-course-${c.code}`}
                onClick={() => setSelectedCourse(c.id)}
                className={
                  "rounded-md border px-2.5 py-1 text-xs transition " +
                  (selectedCourse === c.id
                    ? "border-primary text-primary"
                    : "border-border text-muted hover:text-content")
                }
              >
                {c.name} · {c.award} · miaka {c.years} · levels {c.levels}
              </button>
            ))}
          </div>
        )}

        {selectedCourse ? (
          <>
            <h4 className="mb-2 text-xs font-semibold text-muted">
              Levels (bofya kuchagua)
            </h4>
            <div className="mb-4 flex flex-wrap gap-1" data-testid="ep-levels">
              {levels.map((l) => (
                <button
                  key={l.id}
                  type="button"
                  data-testid={`ep-level-${l.year}-${l.semester}`}
                  onClick={() => setSelectedLevel(l.id)}
                  className={
                    "rounded-md border px-2.5 py-1 text-xs transition " +
                    (selectedLevel === l.id
                      ? "border-primary text-primary"
                      : "border-border text-muted hover:text-content")
                  }
                >
                  {l.name} ({l.subjects})
                </button>
              ))}
            </div>

            <div className="mb-2 grid gap-3 md:grid-cols-4">
              <EduField label="Jina la somo">
                <EduInput
                  testId="ep-subject-name"
                  value={subject.name}
                  onChange={(e) => setSubject({ ...subject, name: e.target.value })}
                  placeholder="Programming Fundamentals"
                />
              </EduField>
              <EduField label="Code">
                <EduInput
                  value={subject.code}
                  onChange={(e) => setSubject({ ...subject, code: e.target.value })}
                  placeholder="CS101"
                />
              </EduField>
              <EduField label="Credits">
                <EduInput
                  type="number"
                  value={subject.credits}
                  onChange={(e) =>
                    setSubject({ ...subject, credits: Number(e.target.value) || 0 })
                  }
                />
              </EduField>
              <div className="flex items-end">
                <EduButton testId="ep-subject-add" onClick={() => void addSubject()}>
                  <Plus className="size-4" aria-hidden />
                  Ongeza Somo
                </EduButton>
              </div>
            </div>

            <div className="flex flex-wrap gap-1" data-testid="ep-subjects">
              {subjects.map((s) => (
                <EduBadge key={s.id}>
                  {s.name} ({s.code}) · credits {s.credits}
                </EduBadge>
              ))}
              {subjects.length === 0 ? (
                <span className="text-xs text-muted">Hakuna masomo bado.</span>
              ) : null}
            </div>
          </>
        ) : null}
      </EduCard>
      {err ? <EduError message={err} /> : null}
    </div>
  );
}

// ------------------------------------------------------------
// 3. WALIMU
// ------------------------------------------------------------

function TeachersSection() {
  const [teachers, setTeachers] = useState<EpTeacher[]>([]);
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [selected, setSelected] = useState("");
  const [detail, setDetail] = useState<EpTeacherDetail | null>(null);
  const [assignSubject, setAssignSubject] = useState("");
  const [err, setErr] = useState("");
  const [created, setCreated] = useState<{ username: string; password: string } | null>(
    null,
  );
  const [form, setForm] = useState({
    name: "",
    email: "",
    phone: "",
    staff_no: "",
    qualification: "",
    specialization: "",
  });

  const load = useCallback(async () => {
    try {
      const [t, s] = await Promise.all([
        elimuproService.teachers(),
        elimuproService.subjects(""),
      ]);
      setTeachers(t.teachers ?? []);
      setSubjects(s.subjects ?? []);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!selected) return;
    void (async () => {
      try {
        setDetail(await elimuproService.teacher(selected));
      } catch {
        /* */
      }
    })();
  }, [selected]);

  const add = async () => {
    setErr("");
    setCreated(null);
    try {
      const r = await elimuproService.addTeacher(form);
      setCreated({ username: r.username, password: r.password });
      setForm({
        name: "",
        email: "",
        phone: "",
        staff_no: "",
        qualification: "",
        specialization: "",
      });
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  };

  const doAssign = async () => {
    if (!selected || !assignSubject) return;
    setErr("");
    try {
      await elimuproService.assignSubject({
        teacher_id: selected,
        subject_id: assignSubject,
      });
      setDetail(await elimuproService.teacher(selected));
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">Sajili Mwalimu</h3>
        <div className="grid gap-3 md:grid-cols-3">
          <EduField label="Jina kamili">
            <EduInput
              testId="ep-teacher-name"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
            />
          </EduField>
          <EduField label="Barua pepe">
            <EduInput
              value={form.email}
              onChange={(e) => setForm({ ...form, email: e.target.value })}
            />
          </EduField>
          <EduField label="Simu">
            <EduInput
              value={form.phone}
              onChange={(e) => setForm({ ...form, phone: e.target.value })}
            />
          </EduField>
          <EduField label="Namba ya mfanyakazi">
            <EduInput
              value={form.staff_no}
              onChange={(e) => setForm({ ...form, staff_no: e.target.value })}
            />
          </EduField>
          <EduField label="Elimu">
            <EduInput
              value={form.qualification}
              onChange={(e) => setForm({ ...form, qualification: e.target.value })}
            />
          </EduField>
          <EduField label="Taaluma">
            <EduInput
              value={form.specialization}
              onChange={(e) => setForm({ ...form, specialization: e.target.value })}
            />
          </EduField>
        </div>
        <div className="mt-3">
          <EduButton testId="ep-teacher-add" onClick={() => void add()}>
            <Plus className="size-4" aria-hidden />
            Sajili Mwalimu
          </EduButton>
        </div>
        {created ? (
          <div
            data-testid="ep-teacher-created"
            className="mt-3 rounded-md border border-feedback-success/40 bg-feedback-success/10 px-3 py-2 text-sm"
          >
            Account imetengenezwa — <b>{created.username}</b> / password:{" "}
            <b>{created.password}</b>
          </div>
        ) : null}
      </EduCard>

      <EduCard testId="ep-teachers-list">
        <h3 className="mb-3 text-sm font-semibold text-content">
          Walimu ({teachers.length})
        </h3>
        {teachers.length === 0 ? (
          <EduEmpty>Hakuna walimu bado.</EduEmpty>
        ) : (
          <div className="flex flex-wrap gap-1">
            {teachers.map((t) => (
              <button
                key={t.id}
                type="button"
                data-testid={`ep-teacher-${t.username}`}
                onClick={() => setSelected(t.id)}
                className={
                  "rounded-md border px-2.5 py-1 text-xs transition " +
                  (selected === t.id
                    ? "border-primary text-primary"
                    : "border-border text-muted hover:text-content")
                }
              >
                {t.name} · masomo {t.subjects}
              </button>
            ))}
          </div>
        )}
      </EduCard>

      {selected && detail ? (
        <EduCard testId="ep-teacher-detail">
          <h3 className="mb-2 text-sm font-semibold text-content">
            {detail.teacher.name} — masomo yake
          </h3>
          <div className="mb-3 flex flex-wrap gap-1">
            {detail.subjects.map((s) => (
              <EduBadge key={s.id}>
                {s.name} ({s.code}) · {s.academic_year}
              </EduBadge>
            ))}
            {detail.subjects.length === 0 ? (
              <span className="text-xs text-muted">Hana somo bado.</span>
            ) : null}
          </div>
          <div className="flex flex-wrap items-end gap-3">
            <EduField label="Gawa somo">
              <EduSelect
                testId="ep-assign-subject"
                value={assignSubject}
                onChange={(e) => setAssignSubject(e.target.value)}
              >
                <option value="">— chagua somo —</option>
                {subjects.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name} ({s.code})
                  </option>
                ))}
              </EduSelect>
            </EduField>
            <EduButton testId="ep-assign-do" onClick={() => void doAssign()}>
              Gawa
            </EduButton>
          </div>
        </EduCard>
      ) : null}
      {err ? <EduError message={err} /> : null}
    </div>
  );
}

// ------------------------------------------------------------
// 4. WANAFUNZI (usajili + enrollment + ID card)
// ------------------------------------------------------------

function StudentsSection() {
  const [courses, setCourses] = useState<EpCourse[]>([]);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [students, setStudents] = useState<EpStudent[]>([]);
  const [err, setErr] = useState("");
  const [admitted, setAdmitted] = useState<{
    reg_no: string;
    username: string;
    password: string;
    id: string;
    enrolled: number;
  } | null>(null);
  const [necta, setNecta] = useState<{ mode: string; msg: string } | null>(null);
  const [rita, setRita] = useState<{ mode: string; msg: string } | null>(null);
  const [selected, setSelected] = useState("");
  const [detail, setDetail] = useState<EpStudentDetail | null>(null);

  const [form, setForm] = useState({
    full_name: "",
    course_id: "",
    level_id: "",
    gender: "Me",
    dob: "",
    phone: "",
    form4_index: "",
    birth_cert_no: "",
    necta_result: "",
    necta_verified: "none",
    rita_verified: "none",
  });

  const loadStudents = useCallback(async () => {
    try {
      const r = await elimuproService.students();
      setStudents(r.students ?? []);
    } catch {
      /* */
    }
  }, []);

  useEffect(() => {
    void (async () => {
      try {
        const c = await elimuproService.courses();
        setCourses(c.courses ?? []);
        if (c.courses?.length) {
          setForm((f) => ({ ...f, course_id: f.course_id || c.courses[0].id }));
        }
      } catch {
        /* */
      }
      await loadStudents();
    })();
  }, [loadStudents]);

  useEffect(() => {
    if (!form.course_id) return;
    void (async () => {
      const r = await elimuproService.levels(form.course_id);
      setLevels(r.levels ?? []);
      if (r.levels?.length) setForm((f) => ({ ...f, level_id: r.levels[0].id }));
    })();
  }, [form.course_id]);

  useEffect(() => {
    if (!form.level_id) return;
    void (async () => {
      const r = await elimuproService.subjects(form.level_id);
      setSubjects(r.subjects ?? []);
    })();
  }, [form.level_id]);

  useEffect(() => {
    if (!selected) return;
    void (async () => {
      try {
        setDetail(await elimuproService.student(selected));
      } catch {
        /* */
      }
    })();
  }, [selected]);

  const runVerify = async (kind: "necta" | "rita") => {
    setErr("");
    try {
      if (kind === "necta") {
        const r = await elimuproService.verifyNecta({
          form4_index: form.form4_index,
          full_name: form.full_name,
        });
        setNecta({ mode: r.mode, msg: r.message ?? (r.verified ? "imethibitishwa" : "") });
        if (r.verified) setForm((f) => ({ ...f, necta_verified: "api" }));
      } else {
        const r = await elimuproService.verifyRita({
          birth_cert_no: form.birth_cert_no,
          full_name: form.full_name,
        });
        setRita({ mode: r.mode, msg: r.message ?? (r.verified ? "imethibitishwa" : "") });
        if (r.verified) setForm((f) => ({ ...f, rita_verified: "api" }));
      }
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  };

  const admit = async () => {
    setErr("");
    setAdmitted(null);
    try {
      const r = await elimuproService.admit(form);
      if (!r.ok) throw new Error(r.error || "usajili umeshindikana");
      setAdmitted({
        reg_no: r.reg_no ?? "",
        username: r.username ?? "",
        password: r.password ?? "",
        id: r.id ?? "",
        enrolled: r.enrolled_subjects ?? 0,
      });
      setForm((f) => ({
        ...f,
        full_name: "",
        dob: "",
        phone: "",
        form4_index: "",
        birth_cert_no: "",
        necta_result: "",
        necta_verified: "none",
        rita_verified: "none",
      }));
      setNecta(null);
      setRita(null);
      await loadStudents();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "imeshindikana");
    }
  };

  const verified = form.necta_verified !== "none" || form.rita_verified !== "none";

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">
          Usajili wa Mwanafunzi (NECTA + RITA)
        </h3>
        <div className="grid gap-3 md:grid-cols-3">
          <EduField label="Jina kamili">
            <EduInput
              testId="ep-student-name"
              value={form.full_name}
              onChange={(e) => setForm({ ...form, full_name: e.target.value })}
            />
          </EduField>
          <EduField label="Course">
            <EduSelect
              testId="ep-student-course"
              value={form.course_id}
              onChange={(e) => setForm({ ...form, course_id: e.target.value })}
            >
              {courses.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Level">
            <EduSelect
              testId="ep-student-level"
              value={form.level_id}
              onChange={(e) => setForm({ ...form, level_id: e.target.value })}
            >
              {levels.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Jinsia">
            <EduSelect
              value={form.gender}
              onChange={(e) => setForm({ ...form, gender: e.target.value })}
            >
              <option>Me</option>
              <option>Mke</option>
            </EduSelect>
          </EduField>
          <EduField label="Tarehe ya kuzaliwa">
            <EduInput
              type="date"
              value={form.dob}
              onChange={(e) => setForm({ ...form, dob: e.target.value })}
            />
          </EduField>
          <EduField label="Simu">
            <EduInput
              value={form.phone}
              onChange={(e) => setForm({ ...form, phone: e.target.value })}
            />
          </EduField>
          <EduField label="Namba ya Form Four (NECTA)">
            <EduInput
              testId="ep-student-form4"
              value={form.form4_index}
              onChange={(e) => setForm({ ...form, form4_index: e.target.value })}
              placeholder="S0123/0021"
            />
          </EduField>
          <EduField label="Namba ya cheti (RITA)">
            <EduInput
              testId="ep-student-birth"
              value={form.birth_cert_no}
              onChange={(e) => setForm({ ...form, birth_cert_no: e.target.value })}
              placeholder="1995-1234567"
            />
          </EduField>
          <EduField label="Matokeo ya NECTA">
            <EduInput
              value={form.necta_result}
              onChange={(e) => setForm({ ...form, necta_result: e.target.value })}
              placeholder="DIV II, 12 points"
            />
          </EduField>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-3">
          <EduButton
            variant="ghost"
            testId="ep-verify-necta"
            onClick={() => void runVerify("necta")}
          >
            Thibitisha NECTA
          </EduButton>
          <EduButton
            variant="ghost"
            testId="ep-verify-rita"
            onClick={() => void runVerify("rita")}
          >
            Thibitisha RITA
          </EduButton>
          <EduSelect
            testId="ep-necta-mode"
            value={form.necta_verified}
            onChange={(e) => setForm({ ...form, necta_verified: e.target.value })}
          >
            <option value="none">NECTA: haijathibitishwa</option>
            <option value="api">NECTA: api</option>
            <option value="manual">NECTA: manual</option>
          </EduSelect>
          <EduSelect
            testId="ep-rita-mode"
            value={form.rita_verified}
            onChange={(e) => setForm({ ...form, rita_verified: e.target.value })}
          >
            <option value="none">RITA: haijathibitishwa</option>
            <option value="api">RITA: api</option>
            <option value="manual">RITA: manual</option>
          </EduSelect>
        </div>

        {necta ? (
          <p className="mt-2 text-xs text-muted" data-testid="ep-necta-result">
            NECTA [{necta.mode}]: {necta.msg}
          </p>
        ) : null}
        {rita ? (
          <p className="mt-1 text-xs text-muted" data-testid="ep-rita-result">
            RITA [{rita.mode}]: {rita.msg}
          </p>
        ) : null}

        <div className="mt-3">
          <EduButton
            testId="ep-student-admit"
            onClick={() => void admit()}
            disabled={!verified}
          >
            <Users className="size-4" aria-hidden />
            Sajili Mwanafunzi
          </EduButton>
          {!verified ? (
            <span className="ml-3 text-xs text-muted">
              Thibitisha NECTA au RITA kwanza (au chagua manual kama msajili).
            </span>
          ) : null}
        </div>
      </EduCard>

      {admitted ? (
        <EduCard testId="ep-admitted">
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex-1">
              <p className="text-sm text-content">
                ✅ Amesajiliwa: <b>{admitted.reg_no}</b> · masomo{" "}
                {admitted.enrolled} · account <b>{admitted.username}</b> /{" "}
                <b>{admitted.password}</b>
              </p>
            </div>
            <a
              data-testid="ep-idcard-link"
              href={idCardUrl(admitted.id)}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm text-content hover:bg-surface-raised"
            >
              <IdCard className="size-4" aria-hidden />
              Fungua ID Card
            </a>
          </div>
          <img
            data-testid="ep-idcard-img"
            src={idCardUrl(admitted.id)}
            alt="ID card"
            className="mt-3 w-full max-w-md rounded-md border border-border"
          />
        </EduCard>
      ) : null}

      <EduCard testId="ep-students-list">
        <h3 className="mb-3 text-sm font-semibold text-content">
          Wanafunzi ({students.length})
        </h3>
        {students.length === 0 ? (
          <EduEmpty>Hakuna wanafunzi bado.</EduEmpty>
        ) : (
          <div className="flex flex-col gap-1">
            {students.map((s) => (
              <button
                key={s.id}
                type="button"
                data-testid={`ep-student-${s.reg_no}`}
                onClick={() => setSelected(s.id)}
                className={
                  "flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2 text-left text-xs transition " +
                  (selected === s.id
                    ? "border-primary"
                    : "border-border hover:bg-surface-raised")
                }
              >
                <span className="font-medium text-content">{s.full_name}</span>
                <span className="text-muted">
                  {s.reg_no} · {s.level} · masomo {s.subjects}
                </span>
              </button>
            ))}
          </div>
        )}
      </EduCard>

      {selected && detail ? (
        <EduCard testId="ep-student-detail">
          <h3 className="mb-2 text-sm font-semibold text-content">
            {detail.student.full_name} — {detail.student.reg_no}
          </h3>
          <div className="mb-3 flex flex-wrap gap-1">
            {detail.subjects.map((s) => (
              <EduBadge key={s.code}>
                {s.name} ({s.code})
                {s.teacher ? ` · ${s.teacher}` : ""}
              </EduBadge>
            ))}
          </div>
          <img
            src={idCardUrl(detail.student.id)}
            alt="ID card"
            className="w-full max-w-md rounded-md border border-border"
          />
        </EduCard>
      ) : null}
      {err ? <EduError message={err} /> : null}
      {subjects.length ? (
        <p className="text-xs text-muted">
          Level hii ina masomo {subjects.length} — mwanafunzi hupata yote
          automatically.
        </p>
      ) : null}
    </div>
  );
}
