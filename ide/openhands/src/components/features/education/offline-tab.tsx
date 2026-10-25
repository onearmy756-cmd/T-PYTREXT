import { useCallback, useEffect, useState } from "react";
import { CloudOff, RefreshCw, Server, Wifi, WifiOff } from "lucide-react";
import {
  offlineService,
  type LanNode,
  type OfflineStatus,
  type SyncItem,
} from "#/api/education-service/offline.api";
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
 * ELIMUPRO — Awamu 12: offline mode / LAN (moduli 12).
 * Chuo hufanya kazi bila internet: seva ya chuo, nodi za LAN, foleni ya
 * kusawazisha, na sync internet inapopatikana.
 */
export function OfflineTab() {
  const [status, setStatus] = useState<OfflineStatus | null>(null);
  const [mode, setMode] = useState("auto");
  const [cloudUrl, setCloudUrl] = useState("");
  const [queue, setQueue] = useState<SyncItem[]>([]);
  const [nodes, setNodes] = useState<LanNode[]>([]);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);
  const [nodeName, setNodeName] = useState("");
  const [nodeIp, setNodeIp] = useState("");
  const [nodeKind, setNodeKind] = useState("phone");

  const refresh = useCallback(async () => {
    try {
      setStatus(await offlineService.status());
      const st = await offlineService.settings();
      setMode(st.mode);
      setCloudUrl(st.cloud_url);
      setQueue((await offlineService.queue()).queue);
      setNodes((await offlineService.nodes()).nodes);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveSettings = async () => {
    setBusy(true);
    try {
      await offlineService.setSettings({ mode, cloud_url: cloudUrl });
      setMsg("Mipangilio imehifadhiwa.");
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doSync = async () => {
    setBusy(true);
    try {
      const r = await offlineService.sync();
      setMsg(
        `Sync: ${r.synced} zimesawazishwa, ${r.failed} zimeshindwa, ${r.local_commits} za ndani.`,
      );
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const registerNode = async () => {
    setBusy(true);
    try {
      const r = await offlineService.registerNode({
        name: nodeName || "Kifaa",
        ip: nodeIp,
        kind: nodeKind,
      });
      if (!r.ok) setErr(r.error ?? "imeshindikana");
      else {
        setMsg(`Nodi imesajiliwa (${r.ip}).`);
        setNodeName("");
        setNodeIp("");
        await refresh();
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const leave = async (id: string) => {
    try {
      await offlineService.leaveNode(id);
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  };

  const online = status?.state === "online";

  return (
    <div data-testid="offline" className="flex flex-col gap-4">
      <EduCard testId="off-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            {online ? (
              <Wifi className="size-5 text-success" aria-hidden />
            ) : (
              <WifiOff className="size-5 text-warning" aria-hidden />
            )}
            <span className="text-sm font-semibold text-content">
              Offline / LAN (Awamu 12)
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="off-state">{status?.state ?? "—"}</EduBadge>
            <EduBadge testId="off-mode">mode: {status?.mode ?? "—"}</EduBadge>
            <EduBadge testId="off-nodes">{status?.nodes ?? 0} nodi</EduBadge>
            <EduBadge testId="off-pending">{status?.queue_pending ?? 0} foleni</EduBadge>
            <EduButton variant="ghost" onClick={() => void refresh()}>
              <RefreshCw className="size-3.5" aria-hidden />
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-1">
          <Server className="size-3.5 text-muted" aria-hidden />
          <span className="text-xs text-muted">Seva ya chuo:</span>
          {(status?.lan_ips ?? []).map((ip) => (
            <EduBadge key={ip} testId={`off-ip-${ip}`}>
              {ip}
            </EduBadge>
          ))}
        </div>
      </EduCard>

      {err ? <EduError message={err} /> : null}
      {msg ? <p className="text-xs text-primary">{msg}</p> : null}

      <EduCard testId="off-settings">
        <span className="text-sm font-semibold text-content">Mipangilio ya mtandao</span>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Hali">
            <EduSelect testId="off-mode-select" value={mode} onChange={(e) => setMode(e.target.value)}>
              <option value="auto">Auto (gundua)</option>
              <option value="offline">Offline (LAN pekee)</option>
              <option value="online">Online</option>
            </EduSelect>
          </EduField>
          <EduField label="Cloud URL (hiari)">
            <EduInput
              testId="off-cloud-url"
              value={cloudUrl}
              onChange={(e) => setCloudUrl(e.target.value)}
              placeholder="https://cloud.chuo.ac.tz"
            />
          </EduField>
          <div className="flex items-end">
            <EduButton testId="off-save" disabled={busy} onClick={() => void saveSettings()}>
              Hifadhi
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex items-center gap-2">
          <EduButton testId="off-sync" disabled={busy} onClick={() => void doSync()}>
            <CloudOff className="size-4" aria-hidden /> Sawazisha sasa
          </EduButton>
          <span className="text-xs text-muted">
            {status?.internet ? "Internet inapatikana" : "Hakuna internet — local commit"}
          </span>
        </div>
      </EduCard>

      <EduCard testId="off-queue">
        <span className="text-sm font-semibold text-content">Foleni ya kusawazisha ({queue.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {queue.length === 0 ? (
            <EduEmpty>Foleni ni tupu.</EduEmpty>
          ) : (
            queue.slice(0, 15).map((q) => (
              <div
                key={q.id}
                data-testid={`off-q-${q.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {q.entity} · {q.op}
                </span>
                <div className="flex gap-1">
                  <EduBadge testId={`off-q-status-${q.id}`}>{q.status}</EduBadge>
                  {q.attempts > 0 ? <EduBadge>{q.attempts} majaribio</EduBadge> : null}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="off-nodes">
        <span className="text-sm font-semibold text-content">Nodi za LAN ({nodes.length})</span>
        <div className="mt-2 grid gap-2 md:grid-cols-4">
          <EduField label="Jina">
            <EduInput testId="off-node-name" value={nodeName} onChange={(e) => setNodeName(e.target.value)} placeholder="Simu ya Neema" />
          </EduField>
          <EduField label="IP ya LAN">
            <EduInput testId="off-node-ip" value={nodeIp} onChange={(e) => setNodeIp(e.target.value)} placeholder="192.168.1.20" />
          </EduField>
          <EduField label="Aina">
            <EduSelect testId="off-node-kind" value={nodeKind} onChange={(e) => setNodeKind(e.target.value)}>
              <option value="phone">Simu</option>
              <option value="desktop">Kompyuta</option>
              <option value="tablet">Tablet</option>
              <option value="web">Web</option>
            </EduSelect>
          </EduField>
          <div className="flex items-end">
            <EduButton testId="off-node-add" disabled={busy} onClick={() => void registerNode()}>
              Sajili nodi
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {nodes.length === 0 ? (
            <EduEmpty>Hakuna nodi bado.</EduEmpty>
          ) : (
            nodes.map((n) => (
              <div
                key={n.id}
                data-testid={`off-node-${n.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">
                  {n.name} <span className="text-xs text-muted">({n.ip} · {n.kind})</span>
                </span>
                <div className="flex items-center gap-1">
                  <EduBadge testId={`off-node-state-${n.id}`}>{n.state}</EduBadge>
                  <EduButton variant="ghost" testId={`off-node-leave-${n.id}`} onClick={() => void leave(n.id)}>
                    Ondoa
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
