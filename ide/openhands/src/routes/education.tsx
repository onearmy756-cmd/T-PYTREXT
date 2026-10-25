import { useEffect, useState } from "react";
import {
  GraduationCap,
  BookOpen,
  ClipboardList,
  Mic,
  Languages,
  Image as ImageIcon,
  Library,
  Radio,
  ShieldCheck,
  Grid3x3,
  CalendarDays,
  Wallet,
  BedDouble,
  Trophy,
  WifiOff,
  PhoneCall,
  FileCheck,
  Terminal,
  Brain,
  Smartphone,
  Building2,
  Sparkles,
  MapPin,
  Wrench,
  BookMarked,
} from "lucide-react";
import {
  educationService,
  type EduStatus,
} from "#/api/education-service/education-service.api";
import { cn } from "#/utils/utils";
import { EduBadge } from "#/components/features/education/edu-ui";
import { QuestionsTab } from "#/components/features/education/questions-tab";
import { ExamsTab } from "#/components/features/education/exams-tab";
import { ScribeTab } from "#/components/features/education/scribe-tab";
import { LessonTab } from "#/components/features/education/lesson-tab";
import { LanguageTab } from "#/components/features/education/language-tab";
import { MediaTab } from "#/components/features/education/media-tab";
import { TeteaTab } from "#/components/features/education/tetea-tab";
import { ExamLanTab } from "#/components/features/education/exam-lan-tab";
import { ElimuproTab } from "#/components/features/education/elimupro-tab";
import { TeachingTab } from "#/components/features/education/teaching-tab";
import { LiveTab } from "#/components/features/education/live-tab";
import { ExamTab } from "#/components/features/education/exam-tab";
import { MarksTab } from "#/components/features/education/marks-tab";
import { AttendanceTab } from "#/components/features/education/attendance-tab";
import { FeesTab } from "#/components/features/education/fees-tab";
import { LibraryTab } from "#/components/features/education/library-tab";
import { HostelTab } from "#/components/features/education/hostel-tab";
import { ResultsTab } from "#/components/features/education/results-tab";
import { OfflineTab } from "#/components/features/education/offline-tab";
import { VoipTab } from "#/components/features/education/voip-tab";
import { ClearanceTab } from "#/components/features/education/clearance-tab";
import { ItToolsTab } from "#/components/features/education/ittools-tab";
import { AiEngineTab } from "#/components/features/education/aiengine-tab";
import { MalipoTab } from "#/components/features/education/malipo-tab";
import { MapsTab } from "#/components/features/education/maps-tab";
import { ScribeProTab } from "#/components/features/education/scribepro-tab";
import { CompSolTab } from "#/components/features/education/compsol-tab";
import { AkademiaTab } from "#/components/features/education/akademia-tab";

type TabKey =
  | "lesson"
  | "questions"
  | "tetea"
  | "exams"
  | "scribe"
  | "language"
  | "media"
  | "lan"
  | "elimupro"
  | "ufundishaji"
  | "live"
  | "mitihani"
  | "alama"
  | "mahudhurio"
  | "ada"
  | "maktaba"
  | "hosteli"
  | "matokeo"
  | "offline"
  | "voip"
  | "clearance"
  | "ittools"
  | "aiengine"
  | "malipo"
  | "maps"
  | "scribepro"
  | "compsol"
  | "akademia";

const TABS: { key: TabKey; label: string; icon: typeof BookOpen }[] = [
  { key: "lesson", label: "AI Mwalimu", icon: GraduationCap },
  { key: "questions", label: "Benki ya maswali", icon: BookOpen },
  { key: "tetea", label: "NECTA (TETEA)", icon: Library },
  { key: "exams", label: "Mitihani", icon: ClipboardList },
  { key: "scribe", label: "AI Scribe", icon: Mic },
  { key: "language", label: "Lugha", icon: Languages },
  { key: "media", label: "Picha na video", icon: ImageIcon },
  { key: "lan", label: "Mtihani wa LAN", icon: Radio },
  { key: "elimupro", label: "ELIMUPRO — Chuo", icon: Building2 },
  { key: "ufundishaji", label: "Ufundishaji", icon: Sparkles },
  { key: "live", label: "Live", icon: Radio },
  { key: "mitihani", label: "Mitihani + Ulinzi", icon: ShieldCheck },
  { key: "alama", label: "Alama (Excel)", icon: Grid3x3 },
  { key: "mahudhurio", label: "Ratiba + Mahudhurio", icon: CalendarDays },
  { key: "ada", label: "Ada + Malipo", icon: Wallet },
  { key: "maktaba", label: "Maktaba", icon: BookOpen },
  { key: "hosteli", label: "Hosteli", icon: BedDouble },
  { key: "matokeo", label: "Matokeo + Vyeti", icon: Trophy },
  { key: "offline", label: "Offline / LAN", icon: WifiOff },
  { key: "voip", label: "VoIP ya LAN", icon: PhoneCall },
  { key: "clearance", label: "Clearance", icon: FileCheck },
  { key: "ittools", label: "Zana za IT", icon: Terminal },
  { key: "aiengine", label: "Injini ya AI", icon: Brain },
  { key: "malipo", label: "Malipo ya Simu", icon: Smartphone },
  { key: "maps", label: "Ramani + Uelekezaji", icon: MapPin },
  { key: "scribepro", label: "AI Scribe PRO", icon: Mic },
  { key: "compsol", label: "Computer Solutions", icon: Wrench },
  { key: "akademia", label: "Akademia ya AI", icon: BookMarked },
];

/**
 * ELIMU — kituo kimoja cha kufundisha na kutayarisha mitihani:
 * benki ya maswali, mitihani kamili (majibu + maelezo), AI Scribe,
 * lugha, na media halisi. Vyote hupiga `/api/education/*`.
 */
export default function EducationRoute() {
  const [tab, setTab] = useState<TabKey>("lesson");
  const [status, setStatus] = useState<EduStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const s = await educationService.status();
        if (!cancelled) setStatus(s);
      } catch {
        /* takwimu si muhimu kwa matumizi ya tabs */
      }
    };
    void load();
    const timer = window.setInterval(load, 15000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [tab]);

  return (
    <div
      data-testid="education-page"
      className="mx-auto flex h-full w-full max-w-5xl flex-col gap-4 overflow-y-auto p-4"
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <GraduationCap className="size-6 text-primary" aria-hidden />
          <div>
            <h1 className="text-lg font-semibold text-content">
              Elimu — Mwalimu wa AI
            </h1>
            <p className="text-xs text-muted">
              Maswali, mitihani, AI Scribe, lugha, na media halisi
            </p>
          </div>
        </div>
        {status ? (
          <div className="flex flex-wrap gap-1" data-testid="edu-status">
            <EduBadge>{status.questions} maswali</EduBadge>
            <EduBadge>{status.exams} mitihani</EduBadge>
            <EduBadge>{status.language_lessons} masomo</EduBadge>
            <EduBadge>{status.media} media</EduBadge>
            <EduBadge>model: {status.model}</EduBadge>
            <EduBadge>ffmpeg: {status.ffmpeg ? "ipo" : "haipo"}</EduBadge>
          </div>
        ) : null}
      </header>

      <nav className="flex flex-wrap gap-1 border-b border-border pb-1">
        {TABS.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            type="button"
            data-testid={`edu-tab-${key}`}
            onClick={() => setTab(key)}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-t-md px-3 py-1.5 text-sm font-medium transition",
              tab === key
                ? "border-b-2 border-primary text-primary"
                : "text-muted hover:text-content",
            )}
          >
            <Icon className="size-4" aria-hidden />
            {label}
          </button>
        ))}
      </nav>

      <div className="flex-1 pb-8">
        {tab === "lesson" ? <LessonTab /> : null}
        {tab === "questions" ? <QuestionsTab /> : null}
        {tab === "tetea" ? <TeteaTab /> : null}
        {tab === "exams" ? <ExamsTab /> : null}
        {tab === "scribe" ? <ScribeTab /> : null}
        {tab === "language" ? <LanguageTab /> : null}
        {tab === "media" ? <MediaTab /> : null}
        {tab === "lan" ? <ExamLanTab /> : null}
        {tab === "elimupro" ? <ElimuproTab /> : null}
        {tab === "ufundishaji" ? <TeachingTab /> : null}
        {tab === "live" ? <LiveTab /> : null}
        {tab === "mitihani" ? <ExamTab /> : null}
        {tab === "alama" ? <MarksTab /> : null}
        {tab === "mahudhurio" ? <AttendanceTab /> : null}
        {tab === "ada" ? <FeesTab /> : null}
        {tab === "maktaba" ? <LibraryTab /> : null}
        {tab === "hosteli" ? <HostelTab /> : null}
        {tab === "matokeo" ? <ResultsTab /> : null}
        {tab === "offline" ? <OfflineTab /> : null}
        {tab === "voip" ? <VoipTab /> : null}
        {tab === "clearance" ? <ClearanceTab /> : null}
        {tab === "ittools" ? <ItToolsTab /> : null}
        {tab === "aiengine" ? <AiEngineTab /> : null}
        {tab === "malipo" ? <MalipoTab /> : null}
        {tab === "maps" ? <MapsTab /> : null}
        {tab === "scribepro" ? <ScribeProTab /> : null}
        {tab === "compsol" ? <CompSolTab /> : null}
        {tab === "akademia" ? <AkademiaTab /> : null}
      </div>
    </div>
  );
}
