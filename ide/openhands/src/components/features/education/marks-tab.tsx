import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Download,
  Grid3x3,
  Pencil,
  Plus,
  RefreshCw,
  Save,
  Trash2,
} from "lucide-react";
import {
  marksService,
  type MkColumn,
  type MkRow,
  type MkStatus,
} from "#/api/education-service/marks.api";
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

/**
 * ELIMUPRO — Awamu 6: alama za mtindo wa Excel.
 * Jedwali: wanafunzi kwa safu, nguzo (T1/T2/A1/A2/Final) kwa juu.
 * Kila nguzo ina uzito; jumla + daraja la NECTA huhesabiwa kiotomatiki.
 * Alama hukatwa kwenye max ya nguzo; -1 = haijafanywa.
 */
export function MarksTab() {
  const [status, setStatus] = useState<MkStatus | null>(null);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [subjects, setSubjects] = useState<EpSubject[]>([]);
  const [subjectId, setSubjectId] = useState("");
  const [columns, setColumns] = useState<MkColumn[]>([]);
  const [rows, setRows] = useState<MkRow[]>([]);
  const [draft, setDraft] = useState<Record<string, string>>({});
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  // Fomu ya nguzo mpya
  const [kind, setKind] = useState("t1");
  const [label, setLabel] = useState("");
  const [maxMarks, setMaxMarks] = useState("100");
  const [weight, setWeight] = useState("1");

  useEffect(() => {
    void (async () => {
      try {
        setStatus(await marksService.status());
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
    })();
  }, [levelId]);

  const load = useCallback(async () => {
    if (!levelId) return;
    try {
      const g = await marksService.gradebook({ level_id: levelId, subject_id: subjectId });
      setColumns(g.columns);
      setRows(g.rows);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [levelId, subjectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const addColumn = async () => {
    setBusy(true);
    try {
      await marksService.addColumn({
        level_id: levelId,
        subject_id: subjectId,
        kind,
        label,
        max_marks: Number(maxMarks) || 100,
        weight: Number(weight) || 1,
      });
      setLabel("");
      setMsg(`Nguzo ${kind.toUpperCase()} imeongezwa.`);
      await load();
      setStatus(await marksService.status());
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const removeColumn = async (id: string) => {
    try {
      await marksService.deleteColumn(id);
      await load();
    } catch (e) {
      setErr(String(e));
    }
  };

  const renameColumn = async (c: MkColumn) => {
    const label = window.prompt("Jina jipya la nguzo:", c.label);
    if (label === null) return;
    const raw = window.prompt("Kiwango cha juu (max_marks):", String(c.max_marks));
    if (raw === null) return;
    try {
      await marksService.editColumn({
        id: c.id,
        label: label.trim(),
        max_marks: Number(raw) || undefined,
      });
      await load();
    } catch (e) {
      setErr(String(e));
    }
  };

  const cellKey = (studentId: string, columnId: string) => `${studentId}::${columnId}`;

  const saveCell = async (studentId: string, columnId: string) => {
    const raw = draft[cellKey(studentId, columnId)];
    if (raw === undefined) return;
    try {
      await marksService.setMark({ column_id: columnId, student_id: studentId, score: raw === "" ? -1 : Number(raw) });
      setDraft((p) => {
        const n = { ...p };
        delete n[cellKey(studentId, columnId)];
        return n;
      });
      await load();
      setStatus(await marksService.status());
    } catch (e) {
      setErr(String(e));
    }
  };

  const saveAll = async () => {
    const marks = Object.entries(draft).map(([k, v]) => {
      const [student_id, column_id] = k.split("::");
      return { column_id, student_id, score: v === "" ? -1 : Number(v) };
    });
    if (marks.length === 0) {
      setMsg("Hakuna mabadiliko ya kuhifadhi.");
      return;
    }
    setBusy(true);
    try {
      const r = await marksService.bulk({ level_id: levelId, subject_id: subjectId, marks });
      setMsg(`Alama ${r.saved} zimehifadhiwa.`);
      setDraft({});
      await load();
      setStatus(await marksService.status());
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const dirty = useMemo(() => Object.keys(draft).length, [draft]);
  const classAvg = useMemo(() => {
    const done = rows.filter((r) => r.percent >= 0);
    if (done.length === 0) return null;
    return done.reduce((a, r) => a + r.percent, 0) / done.length;
  }, [rows]);

  return (
    <div data-testid="marks" className="flex flex-col gap-4">
      <EduCard testId="marks-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Grid3x3 className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">
              Alama (Awamu 6 — mtindo wa Excel)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="marks-col-count">{columns.length} nguzo</EduBadge>
            <EduBadge testId="marks-row-count">{rows.length} wanafunzi</EduBadge>
            <EduBadge>{status?.marks ?? 0} alama</EduBadge>
            {classAvg !== null ? <EduBadge>wastani {classAvg.toFixed(1)}%</EduBadge> : null}
            {dirty > 0 ? <EduBadge testId="marks-dirty">{dirty} hazijahifadhiwa</EduBadge> : null}
            <EduButton variant="ghost" onClick={() => void load()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard>
        <div className="grid gap-3 md:grid-cols-4">
          <EduField label="Level">
            <EduSelect testId="marks-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
              {levels.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Somo">
            <EduSelect testId="marks-subject" value={subjectId} onChange={(e) => setSubjectId(e.target.value)}>
              {subjects.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </EduSelect>
          </EduField>
          <EduField label="Aina ya nguzo">
            <EduSelect testId="marks-kind" value={kind} onChange={(e) => setKind(e.target.value)}>
              <option value="t1">T1 (Muhula 1)</option>
              <option value="t2">T2 (Muhula 2)</option>
              <option value="a1">A1 (Mwaka 1)</option>
              <option value="a2">A2 (Mwaka 2)</option>
              <option value="final">Final</option>
              <option value="other">Nyingine</option>
            </EduSelect>
          </EduField>
          <EduField label="Jina la nguzo (si lazima)">
            <EduInput testId="marks-label" value={label} onChange={(e) => setLabel(e.target.value)} placeholder="T1" />
          </EduField>
          <EduField label="Alama za juu">
            <EduInput testId="marks-max" value={maxMarks} onChange={(e) => setMaxMarks(e.target.value)} inputMode="numeric" />
          </EduField>
          <EduField label="Uzito">
            <EduInput testId="marks-weight" value={weight} onChange={(e) => setWeight(e.target.value)} inputMode="numeric" />
          </EduField>
          <div className="md:col-span-2 flex items-end">
            <EduButton testId="marks-add-column" onClick={() => void addColumn()} disabled={busy || !levelId}>
              <Plus className="size-4" aria-hidden /> Ongeza nguzo
            </EduButton>
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="marks-grid">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">Jedwali la alama</span>
          <div className="flex gap-1">
            <EduButton variant="ghost" testId="marks-save" onClick={() => void saveAll()} disabled={busy || dirty === 0}>
              <Save className="size-4" aria-hidden /> Hifadhi zote
            </EduButton>
            <a
              data-testid="marks-export"
              href={levelId ? marksService.exportUrl({ level_id: levelId, subject_id: subjectId }) : "#"}
              className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm text-content hover:bg-surface-raised"
            >
              <Download className="size-4" aria-hidden /> CSV (Excel)
            </a>
          </div>
        </div>

        {columns.length === 0 ? (
          <EduEmpty>Hakuna nguzo bado — ongeza T1 kuanza.</EduEmpty>
        ) : (
          <div className="mt-3 overflow-x-auto">
            <table className="w-full border-collapse text-sm" data-testid="marks-table">
              <thead>
                <tr className="border-b border-border text-left">
                  <th className="p-2 text-muted">Reg No</th>
                  <th className="p-2 text-muted">Jina</th>
                  {columns.map((c) => (
                    <th key={c.id} className="p-2 text-center text-muted" data-testid={`marks-col-${c.id}`}>
                      <div className="flex flex-col items-center gap-0.5">
                        <span className="text-content">{c.label}</span>
                        <span className="text-xs">
                          /{c.max_marks} ×{c.weight}
                        </span>
                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            aria-label={`badilisha ${c.label}`}
                            data-testid={`marks-col-edit-${c.id}`}
                            onClick={() => void renameColumn(c)}
                            className="text-muted hover:text-primary"
                          >
                            <Pencil className="size-3" aria-hidden />
                          </button>
                          <button
                            type="button"
                            aria-label={`futa ${c.label}`}
                            onClick={() => void removeColumn(c.id)}
                            className="text-muted hover:text-feedback-error"
                          >
                            <Trash2 className="size-3" aria-hidden />
                          </button>
                        </div>
                      </div>
                    </th>
                  ))}
                  <th className="p-2 text-center text-muted">Jumla</th>
                  <th className="p-2 text-center text-muted">Daraja</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((r) => (
                  <tr key={r.student_id} className="border-b border-border/60" data-testid={`marks-row-${r.student_id}`}>
                    <td className="p-2 text-muted">{r.reg_no}</td>
                    <td className="p-2 text-content">{r.student}</td>
                    {columns.map((c) => {
                      const cell = r.marks.find((m) => m.column_id === c.id);
                      const k = cellKey(r.student_id, c.id);
                      const val = draft[k] ?? (cell && cell.score >= 0 ? String(cell.score) : "");
                      const isDraft = draft[k] !== undefined;
                      return (
                        <td key={c.id} className="p-1 text-center">
                          <input
                            data-testid={`marks-cell-${r.student_id}-${c.id}`}
                            value={val}
                            onChange={(e) => setDraft((p) => ({ ...p, [k]: e.target.value }))}
                            onBlur={() => void saveCell(r.student_id, c.id)}
                            inputMode="decimal"
                            className={
                              "w-16 rounded-md border bg-surface-raised px-1.5 py-1 text-center text-sm text-content outline-none focus:border-focus " +
                              (isDraft ? "border-primary" : "border-border")
                            }
                          />
                        </td>
                      );
                    })}
                    <td className="p-2 text-center text-content" data-testid={`marks-total-${r.student_id}`}>
                      {r.percent >= 0 ? `${r.percent.toFixed(1)}%` : "—"}
                    </td>
                    <td className="p-2 text-center">
                      <EduBadge>{r.grade}</EduBadge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {rows.length === 0 ? <EduEmpty>Hakuna wanafunzi kwenye level hii.</EduEmpty> : null}
          </div>
        )}
      </EduCard>
    </div>
  );
}
