import { useCallback, useEffect, useRef, useState } from "react";
import {
  Radio,
  Users,
  QrCode,
  Play,
  Square,
  RefreshCw,
  CheckCircle2,
  Clock,
} from "lucide-react";
import {
  educationService,
  type ExamLive,
  type ExamResults,
  type ExamRoomCreated,
  type ExamRoomSummary,
  type EduExamMeta,
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
} from "./edu-ui";

/**
 * MTIHANI WA LAN — mwalimu hufungua chumba cha mtihani kwenye laptop yake,
 * wanafunzi hujiunga kwa QR/code kupitia Wi-Fi ya ndani (bila intaneti).
 * Dashibodi hii huonyesha maendeleo ya kila mwanafunzi kwa wakati halisi
 * (WebSocket) na matokeo mara moja.
 */
export function ExamLanTab() {
  const [exams, setExams] = useState<EduExamMeta[]>([]);
  const [rooms, setRooms] = useState<ExamRoomSummary[]>([]);
  const [lanIp, setLanIp] = useState("");
  const [examId, setExamId] = useState("");
  const [duration, setDuration] = useState(60);
  const [passcode, setPasscode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [room, setRoom] = useState<ExamRoomCreated | null>(null);
  const [live, setLive] = useState<ExamLive | null>(null);
  const [results, setResults] = useState<ExamResults | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [e, r] = await Promise.all([
        educationService.exams(),
        educationService.examRooms(),
      ]);
      setExams(e.exams ?? []);
      setRooms(r.rooms ?? []);
      setLanIp(r.lan_ip ?? "");
      if (e.exams?.length && !examId) setExamId(e.exams[0].id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [examId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Masasisho ya wakati halisi kwa chumba kinachoendelea.
  useEffect(() => {
    const code = room?.code;
    if (!code) return;
    let closed = false;
    const poll = setInterval(() => {
      educationService
        .examRoomLive(code)
        .then(setLive)
        .catch(() => {});
    }, 4000);
    educationService
      .examRoomLive(code)
      .then(setLive)
      .catch(() => {});
    try {
      const ws = new WebSocket(educationService.examRoomWsUrl(code));
      wsRef.current = ws;
      ws.onmessage = () => {
        educationService
          .examRoomLive(code)
          .then(setLive)
          .catch(() => {});
      };
      ws.onerror = () => {};
    } catch {
      /* polling itatosha */
    }
    return () => {
      closed = true;
      clearInterval(poll);
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      void closed;
    };
  }, [room?.code]);

  const createRoom = async () => {
    if (!examId) {
      setError("Chagua mtihani kwanza.");
      return;
    }
    setBusy(true);
    setError("");
    setResults(null);
    setLive(null);
    try {
      const r = await educationService.examRoomCreate({
        exam_id: examId,
        duration_min: duration,
        passcode: passcode.trim() || undefined,
        host: lanIp ? `http://${lanIp}:8080` : undefined,
      });
      if (!r.ok) throw new Error(r.error ?? "kufungua chumba kulishindwa");
      setRoom(r);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const loadRoom = async (code: string) => {
    setBusy(true);
    setError("");
    try {
      const [r, l] = await Promise.all([
        educationService.examRoomGet(code),
        educationService.examRoomLive(code),
      ]);
      setRoom({
        ok: true,
        code,
        title: r.title,
        subject: r.subject,
        level: r.level,
        duration_min: r.duration_min,
        join_url: r.join_url,
      });
      setLive(l);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const closeRoom = async () => {
    if (!room?.code) return;
    await educationService.examRoomClose(room.code);
    await refresh();
  };

  const loadResults = async () => {
    if (!room?.code) return;
    try {
      setResults(await educationService.examRoomResults(room.code));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-1 flex items-center gap-1.5 text-sm font-semibold text-content">
          <Radio className="size-4 text-primary" />
          Mtihani wa LAN (offline)
        </h3>
        <p className="mb-3 text-xs text-muted">
          Fungua chumba kwenye laptop yako. Wanafunzi hujiunga kwa Wi-Fi ya
          ndani — hakuna intaneti inayohitajika. Wanaweza kutumia simu, tablet,
          au laptop yoyote yenye kivinjari.
        </p>

        <div className="grid gap-3 sm:grid-cols-2">
          <EduField label="Mtihani">
            <select
              data-testid="exam-room-exam"
              value={examId}
              onChange={(e) => setExamId(e.target.value)}
              className="w-full rounded-md border border-border bg-surface px-2 py-1.5 text-sm text-content"
            >
              {exams.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.title} ({x.subject})
                </option>
              ))}
            </select>
          </EduField>
          <EduField label="Muda (dakika)">
            <EduInput
              testId="exam-room-duration"
              type="number"
              min={5}
              max={360}
              value={duration}
              onChange={(e) => setDuration(Number(e.target.value) || 60)}
            />
          </EduField>
          <EduField label="Code ya siri (hiari)" hint="Wanafunzi wataingiza kabla ya kuanza">
            <EduInput
              testId="exam-room-passcode"
              value={passcode}
              onChange={(e) => setPasscode(e.target.value)}
              placeholder="mf. Chuo@2026"
            />
          </EduField>
          <EduField label="Anwani ya seva (LAN)" hint="IP ambayo simu za wanafunzi zinaona">
            <EduInput
              testId="exam-room-host"
              value={lanIp}
              onChange={(e) => setLanIp(e.target.value)}
              placeholder="192.168.1.100"
            />
          </EduField>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <EduButton testId="exam-room-create" onClick={createRoom} disabled={busy}>
            <Play className="size-4" />
            {busy ? "Inafungua…" : "Fungua Chumba"}
          </EduButton>
          {exams.length === 0 ? (
            <span className="text-xs text-feedback-warning">
              Hakuna mtihani bado — tengeneza kwenye kichupo cha "Mitihani".
            </span>
          ) : null}
        </div>
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {room?.code ? (
        <EduCard testId="exam-room-live">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <div className="text-xs text-muted uppercase">Code ya kuingia</div>
              <div
                data-testid="exam-room-code"
                className="font-mono text-3xl font-bold tracking-widest text-primary"
              >
                {room.code}
              </div>
              <div className="mt-1 text-xs text-muted">{room.title}</div>
            </div>
            <div className="flex flex-col items-center gap-1">
              {room.qr_svg ? (
                <div
                  data-testid="exam-room-qr"
                  className="rounded-md bg-white p-1"
                  // SVG inatengenezwa ndani ya seva (hakuna intaneti/CDN).
                  dangerouslySetInnerHTML={{ __html: room.qr_svg }}
                />
              ) : null}
              <span className="flex items-center gap-1 text-[10px] text-muted">
                <QrCode className="size-3" />
                Wanafunzi waichanue
              </span>
            </div>
            <div className="flex flex-col gap-1">
              <EduButton variant="ghost" onClick={loadResults}>
                <RefreshCw className="size-4" />
                Matokeo
              </EduButton>
              <EduButton variant="ghost" onClick={closeRoom}>
                <Square className="size-4" />
                Funga
              </EduButton>
            </div>
          </div>

          <div className="mt-3 rounded-md border border-border bg-surface-raised p-2">
            <div className="text-[11px] text-muted">Anwani ya kujiunga</div>
            <div className="font-mono text-xs break-all text-content">
              {room.join_url}
            </div>
          </div>

          <div className="mt-4">
            <div className="mb-2 flex items-center gap-3 text-sm">
              <span className="flex items-center gap-1 font-medium text-content">
                <Users className="size-4 text-primary" />
                Wanafunzi: {live?.participants.length ?? 0}
              </span>
              <EduBadge>wanaofanya: {live?.active ?? 0}</EduBadge>
              <EduBadge>waliowasilisha: {live?.submitted ?? 0}</EduBadge>
            </div>

            {!live || live.participants.length === 0 ? (
              <EduEmpty>
                Hakuna mwanafunzi bado. Onyesha QR/code ukutani.
              </EduEmpty>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-left text-xs">
                  <thead className="text-muted">
                    <tr>
                      <th className="py-1 pr-2">Jina</th>
                      <th className="py-1 pr-2">Namba</th>
                      <th className="py-1 pr-2">Maendeleo</th>
                      <th className="py-1 pr-2">Alama</th>
                      <th className="py-1">Hali</th>
                    </tr>
                  </thead>
                  <tbody>
                    {live.participants.map((p) => (
                      <tr key={p.name + p.student_no} className="border-t border-border">
                        <td className="py-1.5 pr-2 text-content">{p.name}</td>
                        <td className="py-1.5 pr-2 text-muted">{p.student_no || "—"}</td>
                        <td className="py-1.5 pr-2">
                          <span className="text-content">
                            {p.answered}/{p.of}
                          </span>
                          <span className="ml-2 inline-block h-1.5 w-16 overflow-hidden rounded-full bg-surface-raised align-middle">
                            <span
                              className="block h-full bg-primary"
                              style={{
                                width: `${p.of > 0 ? Math.round((p.answered / p.of) * 100) : 0}%`,
                              }}
                            />
                          </span>
                        </td>
                        <td className="py-1.5 pr-2 text-content">
                          {p.status === "submitted"
                            ? `${p.score}/${p.total}`
                            : "—"}
                        </td>
                        <td className="py-1.5">
                          {p.status === "submitted" ? (
                            <span className="flex items-center gap-1 text-feedback-success">
                              <CheckCircle2 className="size-3.5" />
                              amewasilisha
                            </span>
                          ) : (
                            <span className="flex items-center gap-1 text-muted">
                              <Clock className="size-3.5" />
                              anafanya
                            </span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          {results ? (
            <div data-testid="exam-room-results" className="mt-4 border-t border-border pt-3">
              <div className="mb-2 flex flex-wrap gap-2 text-sm">
                <EduBadge>wastani: {results.average}%</EduBadge>
                <EduBadge>waliofaulu: {results.passed}/{results.submitted}</EduBadge>
              </div>
              <ul className="flex flex-col text-xs">
                {results.students.map((s) => (
                  <li
                    key={s.name + s.student_no}
                    className="flex items-center justify-between border-t border-border py-1"
                  >
                    <span className="text-content">{s.name}</span>
                    <span className="text-muted">
                      {s.percent}% ({s.score}/{s.total}) · {s.status}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </EduCard>
      ) : null}

      <EduCard>
        <h3 className="mb-3 text-sm font-semibold text-content">Vyumba vilivyofunguliwa</h3>
        {rooms.length === 0 ? (
          <EduEmpty>Hakuna chumba bado.</EduEmpty>
        ) : (
          <ul className="flex flex-col divide-y divide-border">
            {rooms.map((r) => (
              <li key={r.code} className="flex items-center gap-2 py-2 text-sm">
                <button
                  type="button"
                  onClick={() => void loadRoom(r.code)}
                  className="font-mono text-left text-primary hover:underline"
                >
                  {r.code}
                </button>
                <span className="flex-1 truncate text-content">{r.title}</span>
                <EduBadge>{r.participants} wanafunzi</EduBadge>
                <EduBadge>{r.status}</EduBadge>
              </li>
            ))}
          </ul>
        )}
      </EduCard>
    </div>
  );
}
