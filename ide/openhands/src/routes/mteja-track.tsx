/**
 * `/ide/mteja` — ukurasa wa MTEA (umma, hana login).
 *
 * Mteja hufuatilia ombi lake kwa kumbukumbu (`REQ-2026-000001`) + namba ya
 * simu iliyotumika kuwasilisha. Uthibitisho huu wa pili ndio unaoruhusu
 * ukurasa kuwa wazi: kumbukumbu pekee haitoshi, kwa hivyo mtu asiyemjua
 * mteja hawezi kuona maombi yake.
 *
 * Backend hurudisha maendeleo, hatua, mfanyakazi aliyepo, mikataba na hali
 * ya malipo — LAKINI HAIRUDISHI simu/barua pepe za mfanyakazi wala maombi
 * ya wateja wengine (angalia `client_track` kwenye `biashara.rs`).
 */

import { useState } from "react";
import { ArrowLeft, CircleCheck, Clock, ShieldCheck, User } from "lucide-react";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
} from "#/components/features/education/edu-ui";
import { biasharaService, type ClientTrack } from "#/api/biashara/biashara.api";
import { buildAgentCanvasPath } from "#/utils/base-path";

const STATUS_SW: Record<string, string> = {
  submitted: "Imewasilishwa — inasubiri idhini",
  approved: "Imeidhinishwa",
  rejected: "Imekataliwa",
  assigned: "Imepewa mfanyakazi",
  in_progress: "Inaendelea",
  review: "Ukaguzi wa mwisho",
  delivered: "Imekamilika — inasubiri kufungwa",
  closed: "Imekamilika",
  cancelled: "Imesitishwa",
};

/** Hatua za mchakato, kwa mpangilio, ili mteja aone alipo. */
const STEPS = [
  { key: "submitted", label: "Ombi limewasilishwa" },
  { key: "approved", label: "Idhini ya admin" },
  { key: "assigned", label: "Imepewa mfanyakazi" },
  { key: "in_progress", label: "Kazi inaendelea" },
  { key: "review", label: "Ukaguzi" },
  { key: "delivered", label: "Kukabidhiwa mteja" },
  { key: "closed", label: "Imefungwa" },
];

function stepIndex(status: string): number {
  const i = STEPS.findIndex((s) => s.key === status);
  if (i >= 0) return i;
  if (status === "rejected" || status === "cancelled") return -1;
  return 0;
}

function ProgressSteps({ status }: { status: string }) {
  const at = stepIndex(status);
  const dead = at < 0;
  return (
    <ol className="flex flex-col gap-2" data-testid="mteja-steps">
      {STEPS.map((s, i) => {
        const done = !dead && i < at;
        const current = !dead && i === at;
        return (
          <li key={s.key} className="flex items-center gap-2 text-sm">
            <span
              className={
                done
                  ? "flex size-5 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground"
                  : current
                    ? "flex size-5 shrink-0 items-center justify-center rounded-full border-2 border-primary text-primary"
                    : "flex size-5 shrink-0 items-center justify-center rounded-full border border-border text-muted"
              }
            >
              {done ? <CircleCheck className="size-3.5" /> : i + 1}
            </span>
            <span
              className={
                current
                  ? "font-medium text-content"
                  : done
                    ? "text-content"
                    : "text-muted"
              }
            >
              {s.label}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

export default function MtejaTrack() {
  const [ref, setRef] = useState("");
  const [phone, setPhone] = useState("");
  const [data, setData] = useState<ClientTrack | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = () => {
    setBusy(true);
    setError(null);
    setData(null);
    biasharaService
      .clientTrack(ref, phone)
      .then((r) => {
        if (!r.ok) {
          setError(r.error ?? "Ombi halikupatikana");
          return;
        }
        setData(r);
      })
      .catch((e: unknown) =>
        setError(e instanceof Error ? e.message : String(e)),
      )
      .finally(() => setBusy(false));
  };

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 p-4">
      <div className="flex items-center justify-between gap-2">
        <div>
          <h1 className="text-xl font-semibold text-content">
            Kufuatilia Maendeleo ya Ombi
          </h1>
          <p className="text-sm text-muted">
            Weka kumbukumbu ya ombi na namba ya simu uliyotumia kuwasilisha.
          </p>
        </div>
        <a
          href={buildAgentCanvasPath("/")}
          className="text-sm text-primary hover:underline"
        >
          <ArrowLeft className="mr-1 inline size-4" />
          Nyumbani
        </a>
      </div>

      <EduCard testId="mteja-form">
        <div className="grid gap-2 md:grid-cols-2">
          <EduField
            label="Kumbukumbu ya ombi"
            hint="Mfano: REQ-2026-000001"
          >
            <EduInput
              value={ref}
              onChange={(e) => setRef(e.target.value)}
              placeholder="REQ-2026-000001"
              testId="mteja-ref"
            />
          </EduField>
          <EduField label="Namba ya simu" hint="Ile uliyotumia kuwasilisha ombi">
            <EduInput
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="+2557XXXXXXXX"
              testId="mteja-phone"
            />
          </EduField>
        </div>
        <div className="mt-3">
          <EduButton
            testId="mteja-submit"
            onClick={submit}
            disabled={busy || !ref.trim() || !phone.trim()}
          >
            {busy ? "Inatafuta…" : "Onyesha maendeleo"}
          </EduButton>
        </div>
        <p className="mt-2 flex items-center gap-1 text-xs text-muted">
          <ShieldCheck className="size-3.5" />
          Kumbukumbu pekee haitoshi — tunathibitisha kwa namba ya simu.
        </p>
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {data ? (
        <div className="flex flex-col gap-3">
          <EduCard testId="mteja-result">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="font-semibold text-content">{data.ref}</div>
                <div className="text-sm text-muted">{data.title}</div>
                {data.service ? (
                  <div className="text-xs text-muted">{data.service}</div>
                ) : null}
              </div>
              <div className="flex flex-col items-end gap-1">
                <EduBadge>{STATUS_SW[data.status] ?? data.status}</EduBadge>
                <span className="text-xs text-muted">
                  {data.amount} {data.currency}
                </span>
              </div>
            </div>
            <div className="mt-3">
              <div className="mb-1 flex items-center justify-between text-xs text-muted">
                <span>{data.stage}</span>
                <span>{data.progress}%</span>
              </div>
              <div className="h-2 w-full overflow-hidden rounded-full bg-surface-raised">
                <div
                  className="h-full bg-primary transition-all"
                  style={{ width: `${data.progress}%` }}
                />
              </div>
            </div>
            {data.assignee ? (
              <div
                className="mt-3 flex items-center gap-2 rounded-md border border-border bg-surface-raised px-3 py-2 text-sm"
                data-testid="mteja-assignee"
              >
                <User className="size-4 text-primary" />
                <div>
                  <div className="text-content">{data.assignee.full_name}</div>
                  <div className="text-xs text-muted">
                    {data.assignee.profession}
                  </div>
                </div>
              </div>
            ) : (
              <p className="mt-3 text-sm text-muted">
                Mfanyakazi bado hajapangwa.
              </p>
            )}
          </EduCard>

          <EduCard>
            <h2 className="mb-2 font-semibold text-content">Hatua za kazi</h2>
            <ProgressSteps status={data.status} />
          </EduCard>

          <EduCard testId="mteja-timeline">
            <h2 className="mb-2 font-semibold text-content">
              Kumbukumbu za maendeleo
            </h2>
            {data.timeline.length === 0 ? (
              <EduEmpty>Hakuna kumbukumbu bado.</EduEmpty>
            ) : (
              <ol className="flex flex-col gap-2">
                {data.timeline.map((ev, i) => (
                  <li key={i} className="flex gap-2 text-sm">
                    <span className="mt-1 size-2 shrink-0 rounded-full bg-primary" />
                    <div>
                      <div className="text-content">{ev.note}</div>
                      <div className="flex items-center gap-1 text-xs text-muted">
                        <Clock className="size-3" />
                        {ev.at.slice(0, 16)} · {ev.progress}%
                      </div>
                    </div>
                  </li>
                ))}
              </ol>
            )}
          </EduCard>

          <EduCard>
            <h2 className="mb-2 font-semibold text-content">
              Mikataba na malipo
            </h2>
            <div className="flex flex-wrap gap-2 text-sm">
              <EduBadge>
                {data.paid ? "Malipo yamefanyika" : "Malipo hayajafanyika"}
              </EduBadge>
              {data.contracts.length === 0 ? (
                <span className="text-muted">Hakuna mkataba bado.</span>
              ) : (
                data.contracts.map((c, i) => (
                  <EduBadge key={i}>
                    {c.title} · {c.status}
                  </EduBadge>
                ))
              )}
            </div>
          </EduCard>
        </div>
      ) : null}
    </div>
  );
}
