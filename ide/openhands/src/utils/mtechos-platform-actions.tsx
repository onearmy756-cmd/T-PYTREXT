import type { ReactNode } from "react";
import {
  Bug,
  Database,
  Gamepad2,
  Globe,
  MonitorPlay,
  PencilRuler,
  Radio,
  Rocket,
  ShieldCheck,
  Smartphone,
  Workflow,
} from "lucide-react";
import { I18nKey } from "#/i18n/declaration";

/**
 * MTECHOS platform actions — CHANZO KIMOJA (single source of truth).
 * Vitufe vyote vya platform (home launcher + chat suggestions) vinatumia
 * oraodha hii HII. Kila action inatuma PROMPT HALISI ndani ya chat ya home
 * (handleSubmitWithModelGuard → createConversation) — hakuna UI mpya,
 * hakuna iframe, hakuna endpoint tofauti. Kazi zote hufanywa na agent
 * kwenye conversation MOJA, akitumia API za MBILINYI IDE (server/agent):
 *   - /api/forge/* (Studio), /api/sdlc/* (mzunguko wa uhandisi),
 *   - /api/ide/agent/agentic/* (Autopilot), /api/swarm/v2|v3 (team),
 *   - /api/security/* + code_audit (ukaguzi wa usalama).
 * Ongeza action mpya HAPA PEKEE — zote sehemu mbili hupata kwa moja kwa moja.
 */
export interface MtechosPlatformAction {
  key: I18nKey;
  icon: ReactNode;
  prompt: string;
  testId: string;
}

const KANUNI_KUKODI =
  "TEKELEZA KWA KUKODI HALISI. Andika faili KWELI kwa zana write_file (workspace, si sandbox ya muda), kisha endesha tests kwa code_execute na urekebishe mpaka zipite. Tests LAZIMA zitumie node:test/node:assert PEKEE bila dependencies za nje (mock localStorage ndani ya test yenyewe) ili `node --test` ipite bila npm install. USITOE muhtasari, mpango, wala maelezo ya jumla BILA code halisi iliyoandikwa, na USISEME tests zimepita bila kuonyesha output HALISI ya node --test yenye pass/fail.";

export const MTECHOS_PLATFORM_ACTIONS: MtechosPlatformAction[] = [
  {
    key: I18nKey["MTECHOS$WEBSITE"],
    icon: <Globe size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: jenga tovuti ya TODO list. Unda faili hizi KWELI: index.html (form + list + kitufe cha kufuta), style.css (responsive, flexbox), src/todo.js (addTodo/toggleTodo/removeTodo zinazotumia localStorage), package.json yenye script ya test, na src/todo.test.js (tests 4+ zinazothibitisha add/toggle/remove). Endesha tests kwa node --test na uonyeshe matokeo.`,
    testId: "mtechos-quick-website",
  },
  {
    key: I18nKey["MTECHOS$MOBILE"],
    icon: <Smartphone size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: jenga app ya simu ya noti kwa React Native (Expo). Unda faili KWELI: package.json, app.json, App.js (FlatList ya notes + TextInput + kitufe cha kuongeza), src/storage.js (load/save kwa AsyncStorage), src/storage.test.js (tests 4+ kwa logic ya add/remove/list). Endesha tests kwa node --test na uonyeshe matokeo.`,
    testId: "mtechos-quick-mobile",
  },
  {
    key: I18nKey["MTECHOS$IOT"],
    icon: <Radio size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: jenga IoT telemetry backend kwa Node. Unda faili KWELI: package.json, src/telemetry.js (ingestReading yenye validation + rolling average), src/server.js (HTTP server inayopokea POST /telemetry na kurudisha GET /stats), src/telemetry.test.js (tests 4+ kwa validation + average). Endesha tests kwa node --test na uonyeshe matokeo.`,
    testId: "mtechos-quick-iot",
  },
  {
    key: I18nKey["MTECHOS$GAME"],
    icon: <Gamepad2 size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: jenga mchezo wa HTML5 Canvas uitwao "Neon Dodger". Unda faili KWELI: index.html (canvas + HUD + overlay ya start/game-over), src/game.js (core loop: input, update, render kwa requestAnimationFrame; collision kwa Math.hypot), src/logic.js (spawnEnemy, checkCollision, nextLevel kama functions safi), package.json, src/logic.test.js (tests 4+ kwa collision + level progression). Endesha tests kwa node --test na uonyeshe matokeo.`,
    testId: "mtechos-quick-game",
  },
  {
    key: I18nKey["MTECHOS$AUTOPILOT"],
    icon: <Rocket size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: endesha mzunguko KAMILI wa ulturr (plan -> implement -> test -> fix) bila kuuliza. Chagua mradi mdogo wenye maana (mf. CLI ya kusimamia kazi/notes), unda faili zote KWELI kwa write_file (package.json + src/*.js + src/*.test.js + README.md), endesha tests, rekebisha mpaka zipite, kisha ripoti faili zilizoandikwa na matokeo halisi ya tests.`,
    testId: "mtechos-quick-autopilot",
  },
  {
    key: I18nKey["MTECHOS$EMULATOR"],
    icon: <MonitorPlay size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: endesha project iliyopo kwenye workspace sasa hivi. Kwanza tumia list_files kuona faili zilizopo, tumia read_file kusoma package.json, kisha endesha amri halisi (npm test / node src/*.js / python -m http.server) kwa code_execute. Uonyeshe output HALISI ya amri (exit_code, stdout, stderr) — usibuni.`,
    testId: "mtechos-quick-emulator",
  },
  {
    key: I18nKey["MTECHOS$SECURITY_AUDIT"],
    icon: <ShieldCheck size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: fanya security audit ya workspace. Tumia list_files + read_file kusoma code YOTE iliyopo, kisha tafuta: hard-coded secrets/API keys, SQL injection, XSS, eval, path traversal, dependencies zenye CVE. Andika faili KWELI SECURITY_AUDIT.md yenye findings kwa severity (critical/high/medium/low) na mrekebisho halisi wa code (andika patch kwa write_file) — si mapendekezo ya jumla.`,
    testId: "mtechos-quick-security-audit",
  },
  {
    key: I18nKey["MTECHOS$BUG_HUNT"],
    icon: <Bug size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: tafuta na rekebisha bug za logic kwenye workspace. Tumia list_files + read_file kusoma code, andika test mpya inayoonyesha bug (write_file), endesha test kwa code_execute kuonyesha inafeli, kisha rekebisha code (write_file) na endesha test tena kuonyesha inapita. Uonyeshe test inayofeli kabla na inapita baada.`,
    testId: "mtechos-quick-bug-hunt",
  },
  {
    key: I18nKey["MTECHOS$SCHEMA"],
    icon: <Database size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: buni data layer halisi. Unda faili KWELI: src/schema.sql (tables + indexes), src/db.js (initDb + queries kwa node:sqlite), src/db.test.js (tests 4+ kwa insert/select/update). Endesha tests kwa node --test na uonyeshe matokeo halisi.`,
    testId: "mtechos-quick-schema",
  },
  {
    key: I18nKey["MTECHOS$REFACTOR"],
    icon: <Workflow size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: refactor code iliyopo kwenye workspace. Tumia list_files + read_file kusoma code, vunja functions ndefu na ondoa duplication (andika faili mpya kwa write_file), kisha endesha tests zilizopo kwa code_execute kuthibitisha tabia haijabadilika. Uonyeshe matokeo halisi ya tests kabla na baada.`,
    testId: "mtechos-quick-refactor",
  },
  {
    key: I18nKey["MTECHOS$MONOCODE"],
    icon: <PencilRuler size={16} />,
    prompt:
      `${KANUNI_KUKODI}\n\nKazi: MONOCODE — hariri faili NYINGI kwa mabadiliko MOJA yanayoshirikiana. Tumia list_files + read_file kuchanganua workspace, tambua faili zote zinazohusika, andika kila faili iliyobadilishwa kwa write_file, kisha endesha tests kwa code_execute. Uonyeshe diff (+/-) kwa kila faili na matokeo halisi ya tests. Usivunje API zilizopo.`,
    testId: "mtechos-quick-monocode",
  },
];
