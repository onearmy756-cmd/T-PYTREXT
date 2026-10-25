import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, BookOpen, Plus, RefreshCw, RotateCcw, Search } from "lucide-react";
import {
  libraryService,
  type LibBook,
  type LibLoan,
  type LibOverdue,
  type LibStatus,
} from "#/api/education-service/library.api";
import { elimuproService, type EpLevel, type EpStudent } from "#/api/education-service/elimupro.api";
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
 * ELIMUPRO — Awamu 9: maktaba.
 * Vitabu + nakala (barcode), kukopa (kikomo cha mikopo), kurudisha (faini kwa
 * siku zilizochelewa), na orodha ya walioshindwa kurejesha kwa wakati.
 */
export function LibraryTab() {
  const [status, setStatus] = useState<LibStatus | null>(null);
  const [books, setBooks] = useState<LibBook[]>([]);
  const [loans, setLoans] = useState<LibLoan[]>([]);
  const [overdue, setOverdue] = useState<LibOverdue[]>([]);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [students, setStudents] = useState<EpStudent[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [search, setSearch] = useState("");
  const [title, setTitle] = useState("");
  const [author, setAuthor] = useState("");
  const [copies, setCopies] = useState("1");

  const refresh = useCallback(async () => {
    try {
      setStatus(await libraryService.status());
      setBooks((await libraryService.books(search)).books);
      setLoans((await libraryService.loans({ active: true })).loans);
      setOverdue((await libraryService.overdue()).overdue);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [search]);

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
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!levelId) return;
    void elimuproService
      .students(levelId)
      .then((r) => setStudents(r.students))
      .catch(() => setStudents([]));
  }, [levelId]);

  const addBook = async () => {
    setBusy(true);
    try {
      const r = await libraryService.addBook({
        title,
        author,
        copies: Math.max(1, Number(copies) || 1),
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Kitabu "${title}" kimeongezwa na nakala ${r.copies?.length ?? 0}.`);
        setTitle("");
        setAuthor("");
        setCopies("1");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const borrow = async (studentId: string, bookId: string) => {
    setBusy(true);
    try {
      const r = await libraryService.borrow({ student_id: studentId, book_id: bookId });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Kimekopwa. Kirejeshwe kabla ya ${r.due_at}.`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const returnLoan = async (loanId: string) => {
    setBusy(true);
    try {
      const r = await libraryService.returnBook({ loan_id: loanId });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(
          r.fine_due
            ? `Kimerejeshwa. Faini ya siku ${r.late_days}: ${r.fine}`
            : "Kimerejeshwa kwa wakati — hakuna faini.",
        );
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const payFine = async (loanId: string) => {
    try {
      const r = await libraryService.payFine(loanId);
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Faini imelipwa: ${r.paid}`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div data-testid="library" className="flex flex-col gap-4">
      <EduCard testId="lib-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <BookOpen className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">Maktaba (Awamu 9)</span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="lib-books">{status?.books ?? 0} vitabu</EduBadge>
            <EduBadge>{status?.copies ?? 0} nakala</EduBadge>
            <EduBadge testId="lib-onloan">{status?.on_loan ?? 0} zilizokopwa</EduBadge>
            <EduBadge testId="lib-fines">faini {status?.outstanding_fines ?? "0.00"}</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="lib-add">
        <span className="text-sm font-semibold text-content">Ongeza kitabu</span>
        <div className="mt-2 grid gap-2 md:grid-cols-4">
          <EduField label="Jina">
            <EduInput testId="lib-title" value={title} onChange={(e) => setTitle(e.target.value)} />
          </EduField>
          <EduField label="Mwandishi">
            <EduInput testId="lib-author" value={author} onChange={(e) => setAuthor(e.target.value)} />
          </EduField>
          <EduField label="Nakala">
            <EduInput testId="lib-copies" value={copies} onChange={(e) => setCopies(e.target.value)} />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="lib-add-book" disabled={busy || !title} onClick={() => void addBook()}>
              <Plus className="size-4" aria-hidden /> Ongeza
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard testId="lib-books">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">Katalogi ({books.length})</span>
          <div className="flex items-center gap-1">
            <Search className="size-3.5 text-muted" aria-hidden />
            <EduInput testId="lib-search" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Tafuta..." />
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {books.length === 0 ? (
            <EduEmpty>Hakuna vitabu bado.</EduEmpty>
          ) : (
            books.map((b) => (
              <div key={b.id} data-testid={`lib-book-${b.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <div>
                  <p className="text-sm text-content">{b.title}</p>
                  <p className="text-xs text-muted">
                    {b.author || "—"} · {b.category || "—"}
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-1">
                  <EduBadge testId={`lib-avail-${b.id}`}>
                    {b.available}/{b.copies} inapatikana
                  </EduBadge>
                  {students.length > 0 ? (
                    <EduSelect
                      testId={`lib-borrow-student-${b.id}`}
                      value=""
                      onChange={(e) => e.target.value && void borrow(e.target.value, b.id)}
                    >
                      <option value="">Kopesha kwa...</option>
                      {students.map((st) => (
                        <option key={st.id} value={st.id}>
                          {st.full_name}
                        </option>
                      ))}
                    </EduSelect>
                  ) : null}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="lib-loans">
        <span className="text-sm font-semibold text-content">Mikopo inayoendelea ({loans.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {loans.length === 0 ? (
            <EduEmpty>Hakuna mikopo inayoendelea.</EduEmpty>
          ) : (
            loans.map((l) => (
              <div key={l.id} data-testid={`lib-loan-${l.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {l.book} — {l.student} <span className="text-xs text-muted">({l.reg_no})</span>
                </span>
                <div className="flex flex-wrap items-center gap-1">
                  <EduBadge>kirejeshwe {l.due_at}</EduBadge>
                  {l.late_days > 0 ? <EduBadge>siku {l.late_days} zimepita</EduBadge> : null}
                  <EduButton variant="ghost" testId={`lib-return-${l.id}`} disabled={busy} onClick={() => void returnLoan(l.id)}>
                    <RotateCcw className="size-3.5" aria-hidden /> Rejesha
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="lib-overdue">
        <div className="flex items-center gap-2">
          <AlertTriangle className="size-4 text-warning" aria-hidden />
          <span className="text-sm font-semibold text-content">
            Waliochelewa ({overdue.length})
          </span>
          <EduBadge testId="lib-overdue-total">faini {status?.outstanding_fines ?? "0.00"}</EduBadge>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {overdue.length === 0 ? (
            <EduEmpty>Hakuna waliochelewa.</EduEmpty>
          ) : (
            overdue.map((o) => (
              <div key={o.loan_id} data-testid={`lib-overdue-${o.loan_id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {o.book} — {o.student}
                </span>
                <div className="flex items-center gap-1">
                  <EduBadge>siku {o.late_days}</EduBadge>
                  <EduBadge>faini {o.fine}</EduBadge>
                  <EduButton variant="ghost" testId={`lib-pay-${o.loan_id}`} onClick={() => void payFine(o.loan_id)}>
                    Lipa faini
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard>
        <EduField label="Level (kwa kukopesha)">
          <EduSelect testId="lib-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
            {levels.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
      </EduCard>
    </div>
  );
}
