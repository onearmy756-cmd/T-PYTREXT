import { useState } from "react";
import {
  educationService,
  type EduMedia,
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
 * Media HALISI: SVG na MP4 (kwa ffmpeg) zinazozalishwa na backend, kisha
 * kuhudumiwa kwa `/api/education/media/:name`. Hakuna CDN, hakuna picha ya
 * kubuni — grafu inachorwa kwa hesabu halisi.
 */
export function MediaTab() {
  const [prompt, setPrompt] = useState("");
  const [subject, setSubject] = useState("");
  const [width, setWidth] = useState(640);
  const [height, setHeight] = useState(360);
  const [frames, setFrames] = useState(48);
  const [fps, setFps] = useState(12);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [items, setItems] = useState<EduMedia[]>([]);

  const generate = async (kind: "svg" | "video") => {
    setBusy(true);
    setError("");
    try {
      const r = await educationService.generateMedia({
        kind,
        prompt: prompt.trim() || undefined,
        subject: subject.trim() || undefined,
        width,
        height,
        frames,
        fps,
      });
      if (!r.ok) throw new Error(r.error ?? "kizazi kilishindwa");
      setItems((prev) => [r, ...prev]);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <EduCard>
        <h3 className="mb-1 text-sm font-semibold text-content">
          Media halisi — picha (SVG) na video (MP4)
        </h3>
        <p className="mb-3 text-xs text-muted">
          Grafu huchorwa kwa hesabu halisi (y = sin x). Video hutengenezwa kwa
          ffmpeg kwenye server.
        </p>
        <div className="grid gap-3 sm:grid-cols-4">
          <EduField label="Kichwa / maelezo">
            <EduInput
              testId="edu-media-prompt"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="mf. Grafu ya sin(x)"
            />
          </EduField>
          <EduField label="Somo">
            <EduInput
              testId="edu-media-subject"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="mf. Physics"
            />
          </EduField>
          <EduField label="Upana × urefu">
            <div className="flex gap-2">
              <EduInput
                type="number"
                value={width}
                onChange={(e) => setWidth(Number(e.target.value))}
                className="w-20"
              />
              <EduInput
                type="number"
                value={height}
                onChange={(e) => setHeight(Number(e.target.value))}
                className="w-20"
              />
            </div>
          </EduField>
          <EduField label="Fremu / fps (video)">
            <div className="flex gap-2">
              <EduInput
                type="number"
                value={frames}
                onChange={(e) => setFrames(Number(e.target.value))}
                className="w-20"
              />
              <EduInput
                type="number"
                value={fps}
                onChange={(e) => setFps(Number(e.target.value))}
                className="w-20"
              />
            </div>
          </EduField>
        </div>
        <div className="mt-3 flex gap-2">
          <EduButton
            testId="edu-media-svg"
            onClick={() => generate("svg")}
            disabled={busy}
          >
            Tengeneza SVG
          </EduButton>
          <EduButton
            variant="ghost"
            testId="edu-media-video"
            onClick={() => generate("video")}
            disabled={busy}
          >
            Tengeneza video (MP4)
          </EduButton>
        </div>
        {busy ? (
          <div className="mt-3">
            <EduSpinner label="Inazalisha media…" />
          </div>
        ) : null}
      </EduCard>

      {error ? <EduError message={error} /> : null}

      {items.length === 0 ? (
        <EduCard>
          <EduEmpty>Hakuna media iliyozalishwa bado.</EduEmpty>
        </EduCard>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          {items.map((m) => (
            <EduCard key={m.id} testId="edu-media-item">
              <div className="mb-2 flex items-center gap-2">
                <EduBadge>{m.kind}</EduBadge>
                {m.width && m.height ? (
                  <EduBadge>
                    {m.width}×{m.height}
                  </EduBadge>
                ) : null}
                {m.frames ? <EduBadge>{m.frames} fremu</EduBadge> : null}
                <EduBadge>{Math.round(m.bytes / 1024)} KB</EduBadge>
              </div>
              <MediaPreview media={m} />
              <a
                href={educationService.mediaUrl(m.url)}
                download
                className="mt-2 inline-block text-xs text-primary underline"
              >
                Pakua
              </a>
            </EduCard>
          ))}
        </div>
      )}
    </div>
  );
}

function MediaPreview({ media }: { media: EduMedia }) {
  const src = educationService.mediaUrl(media.url);
  if (media.kind === "video") {
    return (
      <video
        data-testid="edu-media-video-preview"
        src={src}
        controls
        className="w-full rounded-md border border-border"
      />
    );
  }
  return (
    <img
      data-testid="edu-media-img-preview"
      src={src}
      alt="media iliyozalishwa"
      className="w-full rounded-md border border-border bg-white"
    />
  );
}
