/**
 * ELIMU (education) service — hupiga endpoints halisi za `/api/education/*`
 * kwenye fundi-deploy (server/agent/src/education.rs).
 *
 * Hatumii `@openhands/typescript-client` kwa sababu endpoints hizi ni za
 * MTECHOS (si za OpenHands agent-server), hivyo hupigwa kwa `fetch` moja kwa
 * moja kwa base URL ile ile ya backend na session key ile ile.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

export interface EduStatus {
  ok: boolean;
  questions: number;
  exams: number;
  scribe: number;
  language_lessons: number;
  media: number;
  books: number;
  model: string;
  ffmpeg: boolean;
  audio_endpoint: string | null;
}

export interface EduQuestion {
  id: string;
  subject: string;
  level: string;
  topic: string;
  year: number | null;
  exam_type: string;
  question: string;
  options: string[];
  answer: string;
  explanation: string;
  marks: number;
}

export interface EduSubject {
  subject: string;
  level: string;
  count: number;
}

export interface EduExamMeta {
  id: string;
  title: string;
  subject: string;
  level: string;
  num_questions: number;
  duration_min: number;
  total_marks: number;
  created_at: string;
}

export interface EduExamAnswer {
  number: number | string;
  answer?: string;
  explanation?: string;
}

export interface EduExamSection {
  name: string;
  questions?: { number: number; question: string; marks: number }[];
  answers?: EduExamAnswer[];
}

export interface EduExamDetail {
  ok: boolean;
  exam: EduExamMeta & { model?: string };
  body: {
    title?: string;
    instructions?: string;
    sections?: EduExamSection[];
  };
  answers: { sections?: EduExamSection[] };
}

export interface EduScribe {
  id: string;
  title: string;
  source_type: string;
  language: string;
  created_at: string;
}

export interface EduScribeAnalysis {
  summary?: string;
  key_points?: string[];
  topics?: string[];
  glossary?: { term: string; meaning: string }[];
  questions?: string[];
  action_items?: string[];
  difficulty?: string;
  word_count?: number;
  raw?: string;
}

export interface EduLessonMeta {
  id: string;
  language: string;
  level: string;
  topic: string;
  created_at: string;
}

/** Sehemu ya somo la AI Mwalimu: maelezo + mfano + uchambuzi. */
export interface EduLessonSection {
  kichwa?: string;
  maelezo?: string;
  mfano?: string;
  uchambuzi?: string;
}

export interface EduLessonMwalimu {
  mada?: string;
  utangulizi?: string;
  sehemu?: EduLessonSection[];
  hitimisho?: string;
  maswali_ya_mazoezi?: string[];
  msamiati?: { neno?: string; maana?: string }[];
  raw?: string;
}

/** Sauti ya kipande kimoja cha somo. */
export interface EduAudioClip {
  file: string;
  url: string;
  bytes: number;
}

export interface EduLessonAudio {
  available: boolean;
  error?: string;
  utangulizi?: EduAudioClip | null;
  sehemu?: Record<string, EduAudioClip>;
  hitimisho?: EduAudioClip | null;
}

export interface EduLessonTools {
  pdftotext: boolean;
  pdftoppm: boolean;
  tesseract: boolean;
  espeak: boolean;
  ffmpeg: boolean;
  install: Record<string, string>;
}

export interface EduLessonSummary {
  id: string;
  title: string;
  language: string;
  level: string;
  minutes: number;
  source_type: string;
  chars: number;
  created_at: string;
}

export interface EduLessonFull extends EduLessonSummary {
  source_name: string;
  transcript: string;
  lesson: EduLessonMwalimu;
  audio: EduLessonAudio;
}

export interface EduLessonUpload {
  ok: boolean;
  error?: string;
  id?: string;
  title?: string;
  language?: string;
  level?: string;
  minutes?: number;
  source_type?: string;
  source_name?: string;
  chars?: number;
  lesson?: EduLessonMwalimu;
  audio?: EduLessonAudio;
  tools?: EduLessonTools;
}

// ---- MTIHANI WA LAN (offline) ----

export interface ExamRoomSummary {
  code: string;
  title: string;
  subject: string;
  level: string;
  duration_min: number;
  status: string;
  join_url: string;
  created_at: string;
  participants: number;
}

export interface ExamRoomCreated {
  ok: boolean;
  error?: string;
  id?: string;
  code?: string;
  title?: string;
  subject?: string;
  level?: string;
  duration_min?: number;
  join_url?: string;
  qr_svg?: string;
  lan_ip?: string;
  questions?: number;
}

export interface ExamStudentQuestion {
  number?: number;
  section?: string;
  question?: string;
  marks?: number;
}

export interface ExamJoinResult {
  ok: boolean;
  error?: string;
  code?: string;
  title?: string;
  token?: string;
  questions?: ExamStudentQuestion[];
  current?: number;
  answers?: Record<string, string>;
  status?: string;
  score?: number;
  total?: number;
}

export interface ExamLiveParticipant {
  name: string;
  student_no: string;
  answered: number;
  of: number;
  score: number;
  total: number;
  status: string;
  joined_at: string;
  updated_at: string;
}

export interface ExamLive {
  ok: boolean;
  code: string;
  active: number;
  submitted: number;
  participants: ExamLiveParticipant[];
}

export interface ExamSubmitResult {
  ok: boolean;
  name?: string;
  score?: number;
  total?: number;
  percent?: number;
  needs_review?: number;
  detail?: {
    number?: number;
    marks?: number;
    awarded?: number;
    correct?: boolean;
    needs_review?: boolean;
  }[];
  wrong?: {
    number?: number;
    question?: string;
    answer?: string;
    expected?: string;
    explanation?: string;
  }[];
}

export interface ExamResults {
  ok: boolean;
  code: string;
  title: string;
  subject: string;
  duration_min: number;
  status: string;
  average: number;
  submitted: number;
  passed: number;
  students: {
    name: string;
    student_no: string;
    score: number;
    total: number;
    percent: number;
    status: string;
    joined_at: string;
  }[];
}

export interface EduLanguageLesson {
  title?: string;
  objectives?: string[];
  vocabulary?: {
    term: string;
    pronunciation?: string;
    meaning?: string;
    example?: string;
  }[];
  grammar?: { rule: string; explanation?: string; examples?: string[] }[];
  dialogue?: { speaker: string; line: string; translation?: string }[];
  exercises?: { question: string; answer?: string }[];
  pronunciation_tips?: string[];
  cultural_note?: string;
  raw?: string;
}

export interface EduMedia {
  ok: boolean;
  id: string;
  kind: string;
  url: string;
  path: string;
  bytes: number;
  width?: number;
  height?: number;
  frames?: number;
  fps?: number;
  error?: string;
}

export interface EduBook {
  id: string;
  title: string;
  subject: string;
  level: string;
  author: string;
  year: number | null;
}

export interface EduPrefs {
  ok: boolean;
  username: string | null;
  learn_language: string;
  native_language: string;
  level: string;
  saved: boolean;
}

export interface EduTeteaToolStatus {
  pdftotext: boolean;
  pdftoppm: boolean;
  tesseract: boolean;
}

export interface EduTeteaStatus {
  ok: boolean;
  root: string;
  source: string;
  indexed: number;
  papers_imported: number;
  questions_from_tetea: number;
  by_exam: { exam: string; count: number }[];
  by_kind: { kind: string; count: number }[];
  tools: EduTeteaToolStatus;
}

export interface EduTeteaIndex {
  ok: boolean;
  total?: number;
  error?: string;
}

export interface EduTeteaSubject {
  exam: string;
  subject: string;
  label: string;
  level: string;
  count: number;
}

export interface EduTeteaPaper {
  id: string;
  rel: string;
  exam: string;
  subject: string;
  label: string;
  level: string;
  year: number | null;
  paper: string;
  kind: string;
  bytes: number;
  imported: number;
  url: string;
}

export interface EduTeteaExtract {
  ok: boolean;
  id: string;
  rel: string;
  exam: string;
  subject: string;
  label: string;
  level: string;
  year: number | null;
  paper: string;
  kind: string;
  chars: number;
  ocr_used: boolean;
  questions_detected: number;
  text?: string;
  truncated?: boolean;
  added?: number;
  skipped_duplicates?: number;
  sample?: { question: string; marks: number }[];
  error?: string;
}

export interface EduTeteaImportResult {
  id: string;
  rel: string;
  ok: boolean;
  detected?: number;
  added?: number;
  skipped?: number;
  ocr?: boolean;
  chars?: number;
  error?: string;
}

export interface EduTeteaImport {
  ok: boolean;
  papers: number;
  questions_added: number;
  skipped_duplicates: number;
  ocr_papers: number;
  chars_processed: number;
  failures: number;
  results: EduTeteaImportResult[];
  error?: string;
}

export interface EduTeteaFetchStatus {
  ok: boolean;
  running: boolean;
  phase?: string;
  done: number;
  total: number;
  ok_count?: number;
  fail?: number;
  started_at?: string;
  error?: string;
}

async function call<T>(
  path: string,
  init?: { method?: string; body?: unknown },
): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/education${path}`;
  const res = await fetch(url, {
    method: init?.method ?? "GET",
    headers: {
      ...getAgentServerHeaders(),
      ...(init?.body !== undefined
        ? { "Content-Type": "application/json" }
        : {}),
    },
    body: init?.body !== undefined ? JSON.stringify(init.body) : undefined,
  });
  const data = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) {
    throw new Error(data?.error || `HTTP ${res.status}`);
  }
  return data;
}

/** Kama `call`, lakini kwa endpoints za mtihani (`/api/exam/*`). */
async function examCall<T>(
  path: string,
  init?: { method?: string; body?: unknown },
): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/exam${path}`;
  const res = await fetch(url, {
    method: init?.method ?? "GET",
    headers: {
      ...getAgentServerHeaders(),
      ...(init?.body !== undefined
        ? { "Content-Type": "application/json" }
        : {}),
    },
    body: init?.body !== undefined ? JSON.stringify(init.body) : undefined,
  });
  const data = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

export const educationService = {
  status: () => call<EduStatus>("/status"),

  subjects: () => call<{ subjects: EduSubject[] }>("/subjects"),

  seed: () =>
    call<{ ok: boolean; added: number; total_seed: number }>("/seed", {
      method: "POST",
    }),

  questions: (params: {
    subject?: string;
    level?: string;
    year?: number;
    q?: string;
    limit?: number;
  }) => {
    const qs = new URLSearchParams();
    Object.entries(params).forEach(([k, v]) => {
      if (v !== undefined && v !== "") qs.set(k, String(v));
    });
    return call<{ ok: boolean; count: number; questions: EduQuestion[] }>(
      `/questions?${qs.toString()}`,
    );
  },

  generateQuestions: (params: {
    subject: string;
    level?: string;
    topic?: string;
    count?: number;
    language?: string;
  }) =>
    call<{
      ok: boolean;
      count: number;
      questions: EduQuestion[];
      error?: string;
    }>("/questions/generate", { method: "POST", body: params }),

  generateExam: (params: {
    subject: string;
    level?: string;
    topics?: string;
    num_questions?: number;
    duration_min?: number;
    language?: string;
  }) =>
    call<{
      ok: boolean;
      id: string;
      title: string;
      num_questions: number;
      total_marks: number;
      duration_min: number;
      exam: EduExamDetail["body"];
      answers: EduExamDetail["answers"];
      error?: string;
    }>("/exam/generate", { method: "POST", body: params }),

  exams: () => call<{ ok: boolean; exams: EduExamMeta[] }>("/exams"),

  exam: (id: string) => call<EduExamDetail>(`/exam/${id}`),

  scribeAnalyze: (params: {
    title?: string;
    transcript: string;
    language?: string;
    subject?: string;
  }) =>
    call<{
      ok: boolean;
      id: string;
      title: string;
      analysis: EduScribeAnalysis;
      error?: string;
    }>("/scribe/analyze", { method: "POST", body: params }),

  scribeList: () => call<{ ok: boolean; scribe: EduScribe[] }>("/scribe"),

  languageLesson: (params: {
    language: string;
    level?: string;
    topic?: string;
    native?: string;
  }) =>
    call<{
      ok: boolean;
      id: string;
      language: string;
      level: string;
      topic: string;
      lesson: EduLanguageLesson;
      error?: string;
    }>("/language/lesson", { method: "POST", body: params }),

  languageLessons: () =>
    call<{ ok: boolean; lessons: EduLessonMeta[] }>("/language/lessons"),

  prefs: () => call<EduPrefs>("/prefs"),

  savePrefs: (params: {
    learn_language?: string;
    native_language?: string;
    level?: string;
  }) => call<EduPrefs>("/prefs/set", { method: "POST", body: params }),

  generateMedia: (params: {
    kind?: "svg" | "video";
    prompt?: string;
    subject?: string;
    frames?: number;
    fps?: number;
    width?: number;
    height?: number;
  }) => call<EduMedia>("/media/generate", { method: "POST", body: params }),

  books: () => call<{ ok: boolean; books: EduBook[] }>("/books"),

  addBook: (params: {
    title: string;
    subject?: string;
    level?: string;
    author?: string;
    year?: number;
    content?: string;
    summary?: string;
  }) =>
    call<{ ok: boolean; id: string; error?: string }>("/books/add", {
      method: "POST",
      body: params,
    }),

  /** URL kamili ya media iliyozalishwa (SVG/MP4) — inaweza kuwekwa kwenye <img>/<video>. */
  mediaUrl: (path: string) => {
    const base = getAgentServerBaseUrl();
    if (!base) return path;
    return `${base.replace(/\/+$/, "")}${path}`;
  },

  // ---------- TETEA: karatasi za NECTA ----------

  teteaStatus: () => call<EduTeteaStatus>("/tetea/status"),

  teteaBuildIndex: () =>
    call<EduTeteaIndex>("/tetea/index/build", { method: "POST" }),

  teteaSubjects: (exam?: string) => {
    const qs = new URLSearchParams();
    if (exam) qs.set("exam", exam);
    return call<{ ok: boolean; subjects: EduTeteaSubject[] }>(
      `/tetea/subjects?${qs.toString()}`,
    );
  },

  teteaPapers: (params: {
    exam?: string;
    subject?: string;
    year?: number;
    kind?: string;
    level?: string;
    limit?: number;
  }) => {
    const qs = new URLSearchParams();
    Object.entries(params).forEach(([k, v]) => {
      if (v !== undefined && v !== "") qs.set(k, String(v));
    });
    return call<{ ok: boolean; count: number; papers: EduTeteaPaper[] }>(
      `/tetea/papers?${qs.toString()}`,
    );
  },

  teteaExtract: (
    id: string,
    params?: { ocr?: number; max_pages?: number; limit?: number },
  ) =>
    call<EduTeteaExtract>(`/tetea/paper/${id}/extract`, {
      method: "POST",
      body: params ?? {},
    }),

  teteaImport: (params: {
    exam?: string;
    subject?: string;
    year?: number;
    kind?: string;
    level?: string;
    limit?: number;
    ocr?: number;
    max_pages?: number;
    offset?: number;
    force?: number;
  }) => call<EduTeteaImport>("/tetea/import", { method: "POST", body: params }),

  teteaFetch: (params: { exam?: string; subject?: string; limit?: number }) =>
    call<{ ok: boolean; error?: string }>("/tetea/fetch", {
      method: "POST",
      body: params,
    }),

  teteaFetchStatus: () => call<EduTeteaFetchStatus>("/tetea/fetch/status"),

  // ---- AI MWALIMU: mwalimu ana-upload notes → somo + sauti ----

  lessonUpload: async (
    file: File | Blob,
    params: {
      name: string;
      title?: string;
      subject?: string;
      language?: string;
      level?: string;
      minutes?: number;
      voice?: string;
      speed?: number;
      audio?: number;
      max_pages?: number;
    },
  ): Promise<EduLessonUpload> => {
    const base = getAgentServerBaseUrl();
    if (!base) throw new Error("Backend base URL haijulikani");
    const qs = new URLSearchParams();
    qs.set("name", params.name);
    for (const [k, v] of Object.entries(params)) {
      if (k !== "name" && v !== undefined && v !== "") qs.set(k, String(v));
    }
    const url = `${base.replace(/\/+$/, "")}/api/education/lesson/upload?${qs.toString()}`;
    const res = await fetch(url, {
      method: "POST",
      headers: {
        ...getAgentServerHeaders(),
        "Content-Type": "application/octet-stream",
      },
      body: file,
    });
    const data = (await res.json().catch(() => ({}))) as EduLessonUpload;
    if (!res.ok && data.ok !== false) {
      throw new Error(data?.error || `HTTP ${res.status}`);
    }
    return data;
  },

  lessonList: () =>
    call<{
      ok: boolean;
      lessons: EduLessonSummary[];
      tools: EduLessonTools;
    }>("/lesson/lessons"),

  lessonGet: (id: string) => call<EduLessonFull>(`/lesson/${id}`),

  lessonAudio: (id: string, params?: { voice?: string; speed?: number }) =>
    call<{ ok: boolean; id: string; title: string; audio: EduLessonAudio }>(
      `/lesson/${id}/audio`,
      { method: "POST", body: params ?? {} },
    ),

  lessonAsk: (id: string, question: string) =>
    call<{ ok: boolean; answer?: string; error?: string }>(`/lesson/${id}/ask`, {
      method: "POST",
      body: { question },
    }),

  // ---- MTIHANI WA LAN (offline) ----

  examRooms: () =>
    examCall<{ ok: boolean; rooms: ExamRoomSummary[]; lan_ip: string }>("/rooms"),

  examRoomCreate: (body: {
    exam_id: string;
    title?: string;
    duration_min?: number;
    shuffle?: number;
    passcode?: string;
    host?: string;
  }) => examCall<ExamRoomCreated>("/room/create", { method: "POST", body }),

  examRoomGet: (code: string) =>
    examCall<{
      ok: boolean;
      code: string;
      title: string;
      subject: string;
      level: string;
      duration_min: number;
      shuffle: boolean;
      status: string;
      needs_passcode: boolean;
      join_url: string;
      error?: string;
    }>(`/room/${code}`),

  examRoomJoin: (code: string, body: { name: string; student_no?: string; passcode?: string }) =>
    examCall<ExamJoinResult>(`/room/${code}/join`, { method: "POST", body }),

  examRoomSave: (code: string, body: { token: string; index: number; answer: string; current?: number }) =>
    examCall<{ ok: boolean }>(`/room/${code}/save`, { method: "POST", body }),

  examRoomSubmit: (code: string, body: { token: string; answers?: Record<string, string> }) =>
    examCall<ExamSubmitResult>(`/room/${code}/submit`, { method: "POST", body }),

  examRoomLive: (code: string) => examCall<ExamLive>(`/room/${code}/live`),

  examRoomResults: (code: string) => examCall<ExamResults>(`/room/${code}/results`),

  examRoomClose: (code: string, status = "closed") =>
    examCall<{ ok: boolean; code: string; status: string }>(`/room/${code}/close`, {
      method: "POST",
      body: { status },
    }),

  /** URL kamili ya WebSocket ya chumba (kwa live updates). */
  examRoomWsUrl: (code: string): string => {
    const base = (getAgentServerBaseUrl() ?? "").replace(/\/+$/, "");
    const ws = base.replace(/^http/, "ws");
    return `${ws}/api/exam/room/${code}/ws`;
  },
};
