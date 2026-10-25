import { useCallback, useEffect, useState } from "react";
import { FileUp, MessageSquare, Phone, PhoneCall, RefreshCw, Video } from "lucide-react";
import {
  voipService,
  type VoipCall,
  type VoipFile,
  type VoipMessage,
  type VoipStatus,
  type VoipUser,
} from "#/api/education-service/voip.api";
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
 * ELIMUPRO — Awamu 13: mawasiliano ya LAN (moduli 13).
 * Simu, video, SMS na kutuma faili ndani ya LAN ya chuo — bila internet.
 */
export function VoipTab() {
  const [status, setStatus] = useState<VoipStatus | null>(null);
  const [users, setUsers] = useState<VoipUser[]>([]);
  const [calls, setCalls] = useState<VoipCall[]>([]);
  const [messages, setMessages] = useState<VoipMessage[]>([]);
  const [files, setFiles] = useState<VoipFile[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);

  const [me, setMe] = useState("");
  const [myName, setMyName] = useState("");
  const [myIp, setMyIp] = useState("");
  const [peer, setPeer] = useState("");
  const [callKind, setCallKind] = useState("audio");
  const [smsBody, setSmsBody] = useState("");
  const [fileName, setFileName] = useState("");
  const [fileSize, setFileSize] = useState("1024");

  const refresh = useCallback(async () => {
    try {
      setStatus(await voipService.status());
      setUsers((await voipService.users()).users);
      setCalls((await voipService.calls()).calls);
      setMessages((await voipService.messages()).messages);
      setFiles((await voipService.files()).files);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const goOnline = async () => {
    if (!me.trim()) {
      setErr("Weka kitambulisho chako kwanza");
      return;
    }
    setBusy(true);
    try {
      await voipService.presence({ user_id: me, name: myName, ip: myIp, status: "online" });
      setMsg(`Uko mtandaoni kama ${me}.`);
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const dial = async () => {
    if (!me.trim() || !peer.trim()) {
      setErr("Weka kitambulisho chako na cha mpokeaji");
      return;
    }
    setBusy(true);
    try {
      const r = await voipService.startCall({ caller_id: me, callee_id: peer, kind: callKind });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Simu ya ${callKind} inaita (${r.status}).`);
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const callAction = async (id: string, action: "answer" | "reject" | "end") => {
    setBusy(true);
    try {
      const r =
        action === "answer"
          ? await voipService.answer(id)
          : action === "reject"
            ? await voipService.reject(id)
            : await voipService.end(id);
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else setMsg(`Simu: ${r.status}`);
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const sendSms = async () => {
    if (!me.trim() || !peer.trim()) {
      setErr("Weka kitambulisho chako na cha mpokeaji");
      return;
    }
    setBusy(true);
    try {
      const r = await voipService.sendMessage({ from_id: me, to_id: peer, body: smsBody });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Ujumbe umetumwa.");
        setSmsBody("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const offerFile = async () => {
    if (!me.trim() || !peer.trim()) {
      setErr("Weka kitambulisho chako na cha mpokeaji");
      return;
    }
    setBusy(true);
    try {
      const r = await voipService.offerFile({
        from_id: me,
        to_id: peer,
        name: fileName || "faili",
        size: Number(fileSize) || 0,
        lan_url: `http://${myIp || "192.168.1.5"}/f/${Date.now()}`,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg("Faili imetolewa (offered).");
        setFileName("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="voip" className="flex flex-col gap-4">
      <EduCard testId="voip-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">VoIP ya LAN (Awamu 13)</span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="voip-online">{status?.online ?? 0} mtandaoni</EduBadge>
            <EduBadge testId="voip-ringing">{status?.ringing ?? 0} inaita</EduBadge>
            <EduBadge testId="voip-active">{status?.active ?? 0} hai</EduBadge>
            <EduBadge testId="voip-unread">{status?.unread ?? 0} hazijasomwa</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        <p className="mt-1 text-xs text-muted">
          Sauti/video husafiri moja kwa moja kati ya vifaa (WebRTC) — seva hushughulikia ishara pekee.
        </p>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="voip-me">
        <span className="text-sm font-semibold text-content">Mimi</span>
        <div className="mt-2 grid gap-2 md:grid-cols-4">
          <EduField label="Kitambulisho">
            <EduInput testId="voip-me-id" value={me} onChange={(e) => setMe(e.target.value)} placeholder="neema" />
          </EduField>
          <EduField label="Jina">
            <EduInput testId="voip-me-name" value={myName} onChange={(e) => setMyName(e.target.value)} placeholder="Neema Juma" />
          </EduField>
          <EduField label="IP ya LAN">
            <EduInput testId="voip-me-ip" value={myIp} onChange={(e) => setMyIp(e.target.value)} placeholder="192.168.1.20" />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="voip-online-btn" disabled={busy} onClick={() => void goOnline()}>
              Kuwa mtandaoni
            </EduButton>
          </div>
        </div>
      </EduCard>

      <EduCard testId="voip-dial">
        <span className="text-sm font-semibold text-content">Piga simu / Tuma</span>
        <div className="mt-2 grid gap-2 md:grid-cols-4">
          <EduField label="Mpokeaji">
            <EduInput testId="voip-peer" value={peer} onChange={(e) => setPeer(e.target.value)} placeholder="juma" />
          </EduField>
          <EduField label="Aina">
            <EduSelect testId="voip-kind" value={callKind} onChange={(e) => setCallKind(e.target.value)}>
              <option value="audio">Sauti</option>
              <option value="video">Video</option>
            </EduSelect>
          </EduField>
          <div className="flex items-end gap-1">
            <EduButton testId="voip-dial-btn" disabled={busy} onClick={() => void dial()}>
              <PhoneCall className="size-4" aria-hidden /> Piga
            </EduButton>
          </div>
          <div className="flex items-end">
            <EduBadge testId="voip-max-file">
              Max {(status?.max_file_bytes ?? 0) / (1024 * 1024)} MB
            </EduBadge>
          </div>
        </div>

        <div className="mt-3 grid gap-2 md:grid-cols-2">
          <div className="flex flex-col gap-1">
            <EduField label="SMS">
              <EduInput testId="voip-sms-body" value={smsBody} onChange={(e) => setSmsBody(e.target.value)} placeholder="Habari, tuko darasa?" />
            </EduField>
            <div>
              <EduButton testId="voip-sms-btn" disabled={busy} onClick={() => void sendSms()}>
                <MessageSquare className="size-4" aria-hidden /> Tuma SMS
              </EduButton>
            </div>
          </div>
          <div className="flex flex-col gap-1">
            <div className="grid grid-cols-2 gap-2">
              <EduField label="Jina la faili">
                <EduInput testId="voip-file-name" value={fileName} onChange={(e) => setFileName(e.target.value)} placeholder="notes.pdf" />
              </EduField>
              <EduField label="Ukubwa (bytes)">
                <EduInput testId="voip-file-size" value={fileSize} onChange={(e) => setFileSize(e.target.value)} />
              </EduField>
            </div>
            <div>
              <EduButton testId="voip-file-btn" disabled={busy} onClick={() => void offerFile()}>
                <FileUp className="size-4" aria-hidden /> Tuma faili
              </EduButton>
            </div>
          </div>
        </div>
      </EduCard>

      <EduCard testId="voip-calls">
        <span className="text-sm font-semibold text-content">Simu ({calls.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {calls.length === 0 ? (
            <EduEmpty>Hakuna simu bado.</EduEmpty>
          ) : (
            calls.slice(0, 10).map((c) => (
              <div
                key={c.id}
                data-testid={`voip-call-${c.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="flex items-center gap-2 text-sm text-content">
                  {c.kind === "video" ? (
                    <Video className="size-4" aria-hidden />
                  ) : (
                    <Phone className="size-4" aria-hidden />
                  )}
                  {c.caller_id} → {c.callee_id}
                  <span className="text-xs text-muted">{c.duration_s}s</span>
                </span>
                <div className="flex items-center gap-1">
                  <EduBadge testId={`voip-call-status-${c.id}`}>{c.status}</EduBadge>
                  {c.status === "ringing" ? (
                    <>
                      <EduButton testId={`voip-answer-${c.id}`} disabled={busy} onClick={() => void callAction(c.id, "answer")}>
                        Pokea
                      </EduButton>
                      <EduButton variant="ghost" testId={`voip-reject-${c.id}`} disabled={busy} onClick={() => void callAction(c.id, "reject")}>
                        Kataa
                      </EduButton>
                    </>
                  ) : null}
                  {c.status === "active" ? (
                    <EduButton variant="ghost" testId={`voip-end-${c.id}`} disabled={busy} onClick={() => void callAction(c.id, "end")}>
                      Kata
                    </EduButton>
                  ) : null}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <div className="grid gap-4 md:grid-cols-2">
        <EduCard testId="voip-messages">
          <span className="text-sm font-semibold text-content">SMS ({messages.length})</span>
          <div className="mt-2 flex flex-col gap-1">
            {messages.length === 0 ? (
              <EduEmpty>Hakuna ujumbe.</EduEmpty>
            ) : (
              messages.slice(0, 10).map((m) => (
                <div
                  key={m.id}
                  data-testid={`voip-msg-${m.id}`}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
                >
                  <span className="text-sm text-content">
                    <span className="text-xs text-muted">{m.from_id}→{m.to_id}:</span> {m.body}
                  </span>
                  <div className="flex items-center gap-1">
                    <EduBadge testId={`voip-msg-status-${m.id}`}>{m.status}</EduBadge>
                    {m.status !== "read" ? (
                      <EduButton variant="ghost" testId={`voip-msg-read-${m.id}`} onClick={() => void voipService.markRead(m.id).then(refresh)}>
                        Soma
                      </EduButton>
                    ) : null}
                  </div>
                </div>
              ))
            )}
          </div>
        </EduCard>

        <EduCard testId="voip-files">
          <span className="text-sm font-semibold text-content">Faili ({files.length})</span>
          <div className="mt-2 flex flex-col gap-1">
            {files.length === 0 ? (
              <EduEmpty>Hakuna faili.</EduEmpty>
            ) : (
              files.slice(0, 10).map((f) => (
                <div
                  key={f.id}
                  data-testid={`voip-file-${f.id}`}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
                >
                  <span className="text-sm text-content">
                    {f.name} <span className="text-xs text-muted">({f.size} B)</span>
                  </span>
                  <div className="flex items-center gap-1">
                    <EduBadge testId={`voip-file-status-${f.id}`}>{f.status}</EduBadge>
                    {f.status === "offered" ? (
                      <EduButton variant="ghost" testId={`voip-file-accept-${f.id}`} onClick={() => void voipService.acceptFile(f.id).then(refresh)}>
                        Kubali
                      </EduButton>
                    ) : null}
                    {f.status === "accepted" ? (
                      <EduButton variant="ghost" testId={`voip-file-done-${f.id}`} onClick={() => void voipService.doneFile(f.id).then(refresh)}>
                        Maliza
                      </EduButton>
                    ) : null}
                  </div>
                </div>
              ))
            )}
          </div>
        </EduCard>
      </div>

      <EduCard testId="voip-users">
        <span className="text-sm font-semibold text-content">Watumiaji wa LAN ({users.length})</span>
        <div className="mt-2 flex flex-wrap gap-1">
          {users.length === 0 ? (
            <EduEmpty>Hakuna mtumiaji bado.</EduEmpty>
          ) : (
            users.map((u) => (
              <EduBadge key={u.id} testId={`voip-user-${u.user_id}`}>
                {u.name || u.user_id} · {u.status}
              </EduBadge>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
