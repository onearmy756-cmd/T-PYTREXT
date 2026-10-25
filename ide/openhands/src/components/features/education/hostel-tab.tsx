import { useCallback, useEffect, useState } from "react";
import { BedDouble, Home, Plus, RefreshCw, UserMinus } from "lucide-react";
import {
  hostelService,
  type Hostel,
  type HostelAllocation,
  type HostelRoom,
  type HostelStatus,
} from "#/api/education-service/hostel.api";
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
 * ELIMUPRO — Awamu 10: hosteli (malazi).
 * Hosteli → vyumba → vitanda; ugawaji huzingatia jinsia na vitanda huru.
 */
export function HostelTab() {
  const [status, setStatus] = useState<HostelStatus | null>(null);
  const [hostels, setHostels] = useState<Hostel[]>([]);
  const [selected, setSelected] = useState("");
  const [rooms, setRooms] = useState<HostelRoom[]>([]);
  const [allocations, setAllocations] = useState<HostelAllocation[]>([]);
  const [levels, setLevels] = useState<EpLevel[]>([]);
  const [levelId, setLevelId] = useState("");
  const [students, setStudents] = useState<EpStudent[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [hName, setHName] = useState("");
  const [hGender, setHGender] = useState("male");
  const [roomNo, setRoomNo] = useState("");
  const [roomBeds, setRoomBeds] = useState("4");

  const refresh = useCallback(async () => {
    try {
      setStatus(await hostelService.status());
      const hs = (await hostelService.hostels()).hostels;
      setHostels(hs);
      const cur = selected || hs[0]?.id || "";
      if (cur !== selected) setSelected(cur);
      if (cur) {
        setRooms((await hostelService.hostel(cur)).rooms);
        setAllocations((await hostelService.allocations({ hostel_id: cur })).allocations);
      }
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, [selected]);

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

  const addHostel = async () => {
    setBusy(true);
    try {
      const r = await hostelService.addHostel({ name: hName, gender: hGender });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Hosteli "${hName}" imeongezwa (${r.gender}).`);
        setHName("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addRoom = async () => {
    if (!selected) return;
    setBusy(true);
    try {
      const r = await hostelService.addRoom({
        hostel_id: selected,
        room_no: roomNo,
        beds: Math.max(1, Number(roomBeds) || 1),
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Chumba ${roomNo} kimeongezwa na vitanda ${r.beds}.`);
        setRoomNo("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const allocate = async (studentId: string) => {
    if (!selected) return;
    setBusy(true);
    try {
      const r = await hostelService.allocate({ student_id: studentId, hostel_id: selected });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Nafasi imetolewa.");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const vacate = async (allocationId: string) => {
    setBusy(true);
    try {
      const r = await hostelService.vacate({ allocation_id: allocationId });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Kitanda kimeachiliwa.");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="hostel" className="flex flex-col gap-4">
      <EduCard testId="hostel-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Home className="size-5 text-primary" aria-hidden />
            <span className="text-sm font-semibold text-content">Hosteli (Awamu 10)</span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="hostel-count">{status?.hostels ?? 0} hosteli</EduBadge>
            <EduBadge>{status?.rooms ?? 0} vyumba</EduBadge>
            <EduBadge testId="hostel-beds">{status?.beds ?? 0} vitanda</EduBadge>
            <EduBadge testId="hostel-occupied">{status?.occupied ?? 0} vimechukuliwa</EduBadge>
            <EduBadge testId="hostel-free">{status?.free ?? 0} huru</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="hostel-add">
        <span className="text-sm font-semibold text-content">Ongeza hosteli</span>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Jina">
            <EduInput testId="hostel-name" value={hName} onChange={(e) => setHName(e.target.value)} placeholder="Kilimanjaro" />
          </EduField>
          <EduField label="Jinsia">
            <EduSelect testId="hostel-gender" value={hGender} onChange={(e) => setHGender(e.target.value)}>
              <option value="male">Kiume</option>
              <option value="female">Kike</option>
              <option value="mixed">Mchanganyiko</option>
            </EduSelect>
          </EduField>
          <div className="flex items-end">
            <EduButton testId="hostel-add-btn" disabled={busy || !hName} onClick={() => void addHostel()}>
              <Plus className="size-4" aria-hidden /> Ongeza
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard>
        <EduField label="Hosteli">
          <EduSelect testId="hostel-select" value={selected} onChange={(e) => setSelected(e.target.value)}>
            {hostels.map((h) => (
              <option key={h.id} value={h.id}>
                {h.name} ({h.gender}) — {h.free}/{h.beds} huru
              </option>
            ))}
          </EduSelect>
        </EduField>
      </EduCard>

      <EduCard testId="hostel-rooms">
        <span className="text-sm font-semibold text-content">Vyumba ({rooms.length})</span>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Namba ya chumba">
            <EduInput testId="hostel-room-no" value={roomNo} onChange={(e) => setRoomNo(e.target.value)} placeholder="A101" />
          </EduField>
          <EduField label="Vitanda">
            <EduInput testId="hostel-room-beds" value={roomBeds} onChange={(e) => setRoomBeds(e.target.value)} />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="hostel-room-add" disabled={busy || !roomNo || !selected} onClick={() => void addRoom()}>
              <Plus className="size-4" aria-hidden /> Ongeza chumba
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-2">
          {rooms.length === 0 ? (
            <EduEmpty>Hakuna vyumba bado.</EduEmpty>
          ) : (
            rooms.map((r) => (
              <div key={r.id} data-testid={`hostel-room-${r.id}`} className="rounded-md border border-border p-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-content">Chumba {r.room_no}</span>
                  <EduBadge testId={`hostel-room-occ-${r.id}`}>
                    {r.occupied}/{r.beds} vimechukuliwa
                  </EduBadge>
                </div>
                <div className="mt-1 flex flex-wrap gap-1">
                  {r.beds_detail.map((bd) => (
                    <span
                      key={bd.id}
                      data-testid={`hostel-bed-${bd.id}`}
                      className={`rounded-md border px-2 py-0.5 text-xs ${
                        bd.status === 'free' ? 'border-success/30 text-success' : 'border-warning/30 text-warning'
                      }`}
                    >
                      {bd.label}: {bd.status === 'free' ? 'huru' : bd.student || 'imechukuliwa'}
                    </span>
                  ))}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="hostel-allocations">
        <div className="flex items-center gap-2">
          <BedDouble className="size-4 text-primary" aria-hidden />
          <span className="text-sm font-semibold text-content">Ugawaji ({allocations.length})</span>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {allocations.length === 0 ? (
            <EduEmpty>Hakuna ugawaji.</EduEmpty>
          ) : (
            allocations.map((a) => (
              <div key={a.id} data-testid={`hostel-alloc-${a.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {a.student} — {a.room_no}/{a.bed}
                </span>
                <EduButton variant="ghost" testId={`hostel-vacate-${a.id}`} disabled={busy} onClick={() => void vacate(a.id)}>
                  <UserMinus className="size-3.5" aria-hidden /> Ondoa
                </EduButton>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="hostel-students">
        <EduField label="Level (kwa ugawaji)">
          <EduSelect testId="hostel-level" value={levelId} onChange={(e) => setLevelId(e.target.value)}>
            {levels.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name}
              </option>
            ))}
          </EduSelect>
        </EduField>
        <div className="mt-2 flex flex-col gap-1">
          {students.length === 0 ? (
            <EduEmpty>Hakuna wanafunzi kwenye level hii.</EduEmpty>
          ) : (
            students.map((st) => (
              <div key={st.id} className="flex items-center justify-between rounded-md border border-border p-2">
                <span className="text-sm text-content">
                  {st.full_name} <span className="text-xs text-muted">({st.gender})</span>
                </span>
                <EduButton variant="ghost" testId={`hostel-alloc-${st.id}`} disabled={busy} onClick={() => void allocate(st.id)}>
                  Gawa kitanda
                </EduButton>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
