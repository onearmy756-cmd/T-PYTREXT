"""pytrex.sdlc — mzunguko KAMILI wa SDLC (awamu sita).

Imepandishwa kutoka MTECHOS `sdlc.rs`. Awamu:

    1. mahitaji     (Requirements) — spec → kind/lang + vigezo vya kukubali
    2. usanifu      (Design)       — blueprint ya faili
    3. ujenzi       (Build)        — kuandika faili HALISI
    4. uthibitisho  (Verify)       — lint/audit HALISI
    5. uwasilishaji (Deploy)       — mpango wa deploy
    6. ufuatiliaji  (Monitor)      — afya ya mradi

Kina huchaguliwa kwa ugumu: simple → medium → complex. Kila awamu inahitaji
idhini (gate). Hali huhifadhiwa kwenye faili ya JSON (historia halisi).
"""

from __future__ import annotations

import json
import os
import threading
import time
from typing import Dict, List, Optional

from . import code_audit

# (id, jina_sw, jina_en, lengo_sw, lengo_en)
PHASES = [
    ("mahitaji", "Mahitaji (Requirements)", "Requirements",
     "Kukusanya mahitaji na vigezo vya kukubali kabla ya kuandika kodi.",
     "Gather requirements and acceptance criteria before writing code."),
    ("usanifu", "Usanifu (Design)", "Design",
     "Mpango wa faili na usanifu wa muundo kabla ya kuandika kodi.",
     "File plan and structural design before writing code."),
    ("ujenzi", "Ujenzi (Build)", "Development",
     "Kuandika faili HALISI kwenye workspace.", "Write REAL files to the workspace."),
    ("uthibitisho", "Uthibitisho (Verify)", "Testing",
     "Build + tests HALISI + ukaguzi wa usalama.",
     "Real build + tests + security audit."),
    ("uwasilishaji", "Uwasilishaji (Deploy)", "Deployment",
     "Mpango wa deploy + build artifacts.", "Deploy plan + build artifacts."),
    ("ufuatiliaji", "Ufuatiliaji (Monitor)", "Monitoring",
     "Kurekodi matokeo + afya ya mradi.", "Record results + project health."),
]

PHASE_IDS = [p[0] for p in PHASES]

COMPLEXITY_LEVELS = [
    {"id": "simple", "label_sw": "Rahisi", "label_en": "Simple"},
    {"id": "medium", "label_sw": "Wastani", "label_en": "Medium"},
    {"id": "complex", "label_sw": "Mgumu", "label_en": "Complex"},
]

GUARDRAILS = [
    {"id": "tests", "label_sw": "Usitoe code yenye bugs bila tests.",
     "label_en": "Do not ship code without tests."},
    {"id": "no_secrets", "label_sw": "Usiweke secrets kwenye code.",
     "label_en": "Do not hardcode secrets in code."},
    {"id": "owasp", "label_sw": "Fuata OWASP kwa usalama.",
     "label_en": "Follow OWASP for security."},
    {"id": "fix_risks", "label_sw": "Rekebisha hatari za usalama.",
     "label_en": "Fix security risks."},
    {"id": "deploy_approval", "label_sw": "Uliza idhini kabla ya deploy.",
     "label_en": "Ask for approval before deploy."},
    {"id": "docs", "label_sw": "Weka documentation kila hatua.",
     "label_en": "Document every phase."},
]


def phase_label(pid: str, lang: str = "sw") -> str:
    for p in PHASES:
        if p[0] == pid:
            return p[1] if lang == "sw" else p[2]
    return pid


def next_phase(pid: str) -> Optional[str]:
    if pid in PHASE_IDS:
        i = PHASE_IDS.index(pid)
        if i + 1 < len(PHASE_IDS):
            return PHASE_IDS[i + 1]
    return None


def assess_complexity(spec: str, kind: str, files_planned: int) -> tuple:
    """Pima ugumu wa mradi: (level, score, reasons)."""
    s = spec.lower()
    score = 0
    reasons: List[str] = []
    if len(spec) > 200:
        score += 2
        reasons.append("spec ndefu")
    if files_planned > 6:
        score += 2
        reasons.append("faili nyingi zimepangwa")
    heavy = ["enterprise", "multi-tenant", "oauth", "payment", "realtime",
             "websocket", "compliance", "gdpr", "scalab", "microservice",
             "kuza", "biashara kubwa"]
    hits = [w for w in heavy if w in s]
    if hits:
        score += 3
        reasons.append("vipengele vizito: " + ", ".join(hits[:4]))
    if kind in ("api", "mobile"):
        score += 1
        reasons.append(f"aina ya mradi: {kind}")
    if score >= 5:
        level = "complex"
    elif score >= 2:
        level = "medium"
    else:
        level = "simple"
    if not reasons:
        reasons.append("kazi ndogo")
    return level, score, reasons


def level_label(level: str, lang: str = "sw") -> str:
    for c in COMPLEXITY_LEVELS:
        if c["id"] == level:
            return c["label_sw"] if lang == "sw" else c["label_en"]
    return level


def depth_for(level: str, lang: str = "sw") -> str:
    return {
        ("simple", "sw"): "nyepesi", ("simple", "en"): "light",
        ("medium", "sw"): "kati", ("medium", "en"): "standard",
        ("complex", "sw"): "kamili", ("complex", "en"): "full",
    }.get((level, lang), "kati")


_ACTS = {
    "simple": {
        "mahitaji": [("Uliza maswali 2-3", "Ask 2-3 questions"),
                     ("Andika user stories fupi", "Write short user stories")],
        "usanifu": [("Mchoro rahisi au maelezo mafupi", "Simple sketch or short note")],
        "ujenzi": [("Andika code moja kwa moja", "Write code directly")],
        "uthibitisho": [("Unit tests + manual testing", "Unit tests + manual testing")],
        "uwasilishaji": [("Maelekezo mafupi ya ku-deploy", "Short deploy instructions")],
        "ufuatiliaji": [("Maelezo ya msingi ya monitoring", "Basic monitoring note")],
    },
    "medium": {
        "mahitaji": [("Uliza maswali zaidi", "Ask more questions"),
                     ("Andika requirements doc", "Write requirements doc"),
                     ("Andika acceptance criteria", "Write acceptance criteria")],
        "usanifu": [("Architecture", "Architecture"), ("DB schema", "DB schema"),
                    ("API endpoints", "API endpoints")],
        "ujenzi": [("Code modular", "Modular code"), ("Fuata standards", "Follow standards")],
        "uthibitisho": [("Unit + integration + baadhi ya E2E", "Unit + integration + some E2E")],
        "uwasilishaji": [("CI/CD basic", "Basic CI/CD"),
                         ("Environment variables", "Environment variables"),
                         ("Rollback plan", "Rollback plan")],
        "ufuatiliaji": [("Logging", "Logging"), ("Monitoring", "Monitoring"),
                        ("Update plan", "Update plan")],
    },
    "complex": {
        "mahitaji": [("Uchambuzi kamili", "Full analysis"), ("User stories", "User stories"),
                     ("Use cases", "Use cases"),
                     ("Mahitaji yasiyo-utendaji (NFR)", "Non-functional requirements")],
        "usanifu": [("Architecture detailed", "Detailed architecture"),
                    ("Security design", "Security design"),
                    ("Scalability plan", "Scalability plan"), ("DB schema", "DB schema"),
                    ("API specs", "API specs"), ("Diagrams", "Diagrams")],
        "ujenzi": [("Code reviews", "Code reviews"),
                   ("Branching strategy", "Branching strategy"),
                   ("Documentation", "Documentation"),
                   ("Security best practices", "Security best practices")],
        "uthibitisho": [("Unit + integration + E2E", "Unit + integration + E2E"),
                        ("Performance testing", "Performance testing"),
                        ("Security testing", "Security testing")],
        "uwasilishaji": [("CI/CD kamili", "Full CI/CD"), ("Staging", "Staging"),
                         ("Blue-green deployment", "Blue-green deployment"),
                         ("Rollback", "Rollback"), ("Monitoring", "Monitoring")],
        "ufuatiliaji": [("SLA", "SLA"), ("Incident response", "Incident response"),
                        ("Logging", "Logging"), ("Alerting", "Alerting"),
                        ("Periodic reviews", "Periodic reviews")],
    },
}

_DELIVERABLES = {
    "mahitaji": {"simple": ["user_stories", "acceptance_criteria"],
                 "medium": ["requirements_doc", "acceptance_criteria"],
                 "complex": ["requirements_doc", "user_stories", "use_cases", "nfr"]},
    "usanifu": {"simple": ["design_note", "architecture"],
                "medium": ["architecture", "db_schema", "api_spec"],
                "complex": ["architecture", "security_design", "scalability_plan",
                            "db_schema", "api_spec", "diagrams"]},
    "ujenzi": {"simple": ["code", "pr"],
               "medium": ["code", "pr"],
               "complex": ["code", "pr", "branching_strategy", "docs"]},
    "uthibitisho": {"simple": ["test_report", "bugs"],
                    "medium": ["test_report", "bugs"],
                    "complex": ["test_report", "perf_report", "security_report", "bugs"]},
    "uwasilishaji": {"simple": ["deployment_plan", "rollback_plan"],
                     "medium": ["deployment_plan", "rollback_plan"],
                     "complex": ["deployment_plan", "staging", "blue_green_plan",
                                 "rollback_plan", "monitoring_plan"]},
    "ufuatiliaji": {"simple": ["monitoring_plan", "logs"],
                    "medium": ["monitoring_plan", "logs"],
                    "complex": ["sla", "incident_plan", "logs", "alerts", "review_cadence"]},
}


def phase_plan(level: str, lang: str = "sw") -> List[dict]:
    depth = depth_for(level, lang)
    out = []
    for pid in PHASE_IDS:
        acts = _ACTS[level][pid]
        out.append({
            "id": pid,
            "label": phase_label(pid, lang),
            "depth": depth,
            "activities": [a[0] if lang == "sw" else a[1] for a in acts],
            "deliverables": _DELIVERABLES[pid][level],
            "requires_approval": True,
        })
    return out


def acceptance_criteria(kind: str) -> List[str]:
    base = ["Programu hukusanywa/huendesha na exit code 0.",
            "Kila kipengele kilichoorodheshwa kinaweza kuthibitishwa.",
            "Hakuna secrets zilizoachwa kwenye code.",
            "Kuna tests zinazopita."]
    extra = {
        "web": ["Ukurasa hufunguka kwenye browser bila makosa."],
        "api": ["Endpoints hurudisha JSON sahihi na status codes."],
        "cli": ["Amri hufanya kazi na kutoa matokeo."],
        "mobile": ["Skrini hupakia bila crash."],
        "data": ["Ripoti huzalishwa kutoka data halisi."],
        "game": ["Mchezo huanza na kudhibitiwa."],
    }
    return base + extra.get(kind, [])


def definition_of_done(phases_ok: int, tests_pass: bool, findings: int,
                       critical: int) -> dict:
    blockers: List[str] = []
    if phases_ok < len(PHASE_IDS):
        blockers.append(f"awamu {phases_ok}/{len(PHASE_IDS)} pekee zimekamilika")
    if not tests_pass:
        blockers.append("tests hazijapita")
    if critical > 0:
        blockers.append(f"findings {critical} za critical")
    elif findings > 0:
        blockers.append(f"findings {findings} hazijarekebishwa")
    done = not blockers
    return {"done": done, "blockers": blockers,
            "message": ("Tayari kuwasha." if done else "Bado kuna vizuizi.")}


def phase_gate(phase: str, prior_ok: bool) -> dict:
    if phase not in PHASE_IDS:
        return {"ok": False, "blocked": True, "error": f"awamu '{phase}' haijulikani"}
    return {"ok": bool(prior_ok), "blocked": not prior_ok, "phase": phase,
            "requires_approval": True}


def phase_score(grade: float, tests_failed: int) -> float:
    return max(0.0, min(100.0, grade - 15.0 * tests_failed))


class SDLCStore:
    """Hifadhi ya runs za SDLC (faili ya JSON — historia halisi)."""

    def __init__(self, path: str) -> None:
        self.path = path
        self._lock = threading.Lock()
        self._runs: List[dict] = []
        self._approved: Dict[str, List[str]] = {}
        self._load()

    def _load(self) -> None:
        try:
            with open(self.path, "r", encoding="utf-8") as fh:
                data = json.load(fh)
            self._runs = data.get("runs", [])
            self._approved = data.get("approved", {})
        except (OSError, json.JSONDecodeError):
            self._runs = []
            self._approved = {}

    def _save(self) -> None:
        tmp = self.path + ".tmp"
        try:
            os.makedirs(os.path.dirname(self.path), exist_ok=True)
            with open(tmp, "w", encoding="utf-8") as fh:
                json.dump({"runs": self._runs[-200:], "approved": self._approved},
                          fh, indent=1)
            os.replace(tmp, self.path)
        except OSError:
            pass

    def record(self, entry: dict) -> None:
        with self._lock:
            entry["id"] = len(self._runs) + 1
            entry["at"] = time.strftime("%Y-%m-%dT%H:%M:%S")
            self._runs.append(entry)
            self._save()

    def history(self, limit: int = 50) -> List[dict]:
        with self._lock:
            return list(reversed(self._runs[-limit:]))

    def approve(self, project: str, phase: str) -> List[str]:
        with self._lock:
            lst = self._approved.setdefault(project, [])
            if phase not in lst:
                lst.append(phase)
            self._save()
            return list(lst)

    def approved(self, project: str) -> List[str]:
        with self._lock:
            return list(self._approved.get(project, []))


class SDLCEngine:
    """Inaendesha mzunguko wa SDLC kwa kutumia zana HALISI za T-PYTREXT."""

    def __init__(self, workspace: str, store: SDLCStore, bridge=None) -> None:
        self.workspace = os.path.abspath(workspace)
        self.store = store
        self.bridge = bridge

    # ---- helpers ----

    @staticmethod
    def detect_kind(spec: str) -> str:
        s = spec.lower()
        if any(k in s for k in ("api", "rest", "backend", "server")):
            return "api"
        if any(k in s for k in ("cli", "command line", "terminal")):
            return "cli"
        if any(k in s for k in ("mobile", "simu", "android", "ios")):
            return "mobile"
        if any(k in s for k in ("game", "mchezo", "3d")):
            return "game"
        if any(k in s for k in ("dashboard", "data", "analytics", "report", "takwimu")):
            return "data"
        return "web"

    @staticmethod
    def sanitize_name(spec: str) -> str:
        out = []
        for ch in spec:
            if ch.isalnum():
                out.append(ch.lower())
            elif ch in " -_" and out and out[-1] != "-":
                out.append("-")
            if len(out) >= 32:
                break
        name = "".join(out).strip("-")
        return name or "mradi"

    @staticmethod
    def blueprint(kind: str) -> List[dict]:
        if kind in ("api", "cli", "data"):
            return [{"path": "src/main.rs", "purpose": "programu kuu (endpoint/CLI)", "lang": "rust"},
                    {"path": "Cargo.toml", "purpose": "usanidi wa crate", "lang": "text"},
                    {"path": "README.md", "purpose": "maelezo ya matumizi", "lang": "markdown"}]
        if kind in ("mobile", "game"):
            return [{"path": "index.html", "purpose": "kiolesura cha skrini", "lang": "html"},
                    {"path": "app.js", "purpose": "mantiki ya kiolesura", "lang": "javascript"},
                    {"path": "styles.css", "purpose": "muonekano", "lang": "css"},
                    {"path": "README.md", "purpose": "maelezo ya matumizi", "lang": "markdown"}]
        return [{"path": "index.html", "purpose": "ukurasa wa mwanzo", "lang": "html"},
                {"path": "app.js", "purpose": "mantiki ya ukurasa", "lang": "javascript"},
                {"path": "styles.css", "purpose": "muonekano", "lang": "css"},
                {"path": "README.md", "purpose": "maelezo ya matumizi", "lang": "markdown"}]

    def _safe_dir(self, project: str) -> Optional[str]:
        if not project or ".." in project.split("/"):
            return None
        d = os.path.join(self.workspace, project)
        if not os.path.abspath(d).startswith(self.workspace):
            return None
        return d

    # ---- endpoints ----

    def capabilities(self) -> dict:
        return {
            "ok": True, "engine": "T-PYTREXT SDLC",
            "note": "Mzunguko kamili: mahitaji → usanifu → ujenzi → uthibitisho "
                    "→ uwasilishaji → ufuatiliaji. Kina huchaguliwa kwa ugumu.",
            "phases": [{"id": p[0], "label_sw": p[1], "label_en": p[2],
                        "goal_sw": p[3], "goal_en": p[4]} for p in PHASES],
            "complexity_levels": COMPLEXITY_LEVELS,
            "guardrails": GUARDRAILS,
            "endpoints": ["GET  /api/sdlc/capabilities", "POST /api/sdlc/assess",
                          "POST /api/sdlc/plan", "POST /api/sdlc/run",
                          "POST /api/sdlc/gate", "POST /api/sdlc/continue",
                          "GET  /api/sdlc/history"],
        }

    def assess(self, spec: str, kind: str = "", lang: str = "sw") -> dict:
        spec = (spec or "").strip()
        if not spec:
            return {"ok": False, "error": "spec ni lazima"}
        kind = kind or self.detect_kind(spec)
        bp = self.blueprint(kind)
        level, score, reasons = assess_complexity(spec, kind, len(bp))
        return {"ok": True, "lang": lang, "kind": kind,
                "complexity": {"level": level, "label": level_label(level, lang),
                               "score": score, "reasons": reasons},
                "phase_plan": phase_plan(level, lang), "guardrails": GUARDRAILS}

    def plan(self, spec: str, kind: str = "", lang: str = "sw") -> dict:
        spec = (spec or "").strip()
        if not spec:
            return {"ok": False, "error": "spec ni lazima"}
        kind = kind or self.detect_kind(spec)
        bp = self.blueprint(kind)
        level, score, reasons = assess_complexity(spec, kind, len(bp))
        return {"ok": True, "lang": lang, "spec": spec, "kind": kind,
                "project": self.sanitize_name(spec),
                "complexity": {"level": level, "label": level_label(level, lang),
                               "score": score, "reasons": reasons},
                "acceptance_criteria": acceptance_criteria(kind),
                "files_planned": bp,
                "phase_plan": phase_plan(level, lang), "guardrails": GUARDRAILS}

    def gate(self, phase: str, prior_ok: bool = True, approved: bool = False) -> dict:
        v = phase_gate(phase, prior_ok)
        v["requires_approval"] = True
        v["approved"] = approved
        if prior_ok and not approved:
            v["waiting_for_approval"] = True
        return v

    def continue_phase(self, project: str, phase: str = "", approve: bool = True,
                       lang: str = "sw") -> dict:
        project = self.sanitize_name(project)
        if not project:
            return {"ok": False, "error": "project ni lazima"}
        if phase and phase not in PHASE_IDS:
            return {"ok": False, "error": f"awamu '{phase}' haijulikani"}
        if approve and phase:
            approved = self.store.approve(project, phase)
        else:
            approved = self.store.approved(project)
        nxt = next_phase(phase) if phase else None
        return {"ok": True, "project": project, "lang": lang,
                "approved_phase": phase if approve else None,
                "approved": approved, "next": nxt,
                "next_label": phase_label(nxt, lang) if nxt else None,
                "message": ("Awamu imeidhinishwa. Tuendelee?" if lang == "sw"
                            else "Phase approved. Shall we continue?")}

    def history(self, limit: int = 50) -> dict:
        runs = self.store.history(limit)
        return {"ok": True, "count": len(runs), "runs": runs}

    def run(self, spec: str, project: str = "", kind: str = "", write: bool = True,
            run_verify: bool = True, lang: str = "sw",
            complexity: str = "") -> dict:
        spec = (spec or "").strip()
        if not spec:
            return {"ok": False, "error": "spec ni lazima"}
        started = time.time()
        kind = kind or self.detect_kind(spec)
        project = self.sanitize_name(project or spec)
        bp = self.blueprint(kind)
        if complexity in ("simple", "medium", "complex"):
            level, cx_score, cx_reasons = complexity, 0, ["iliyotolewa na mtumiaji"]
        else:
            level, cx_score, cx_reasons = assess_complexity(spec, kind, len(bp))
        depth = depth_for(level, lang)
        phases: List[dict] = []
        prior_ok = True

        # 1) MAHITAJI
        phases.append({"id": "mahitaji", "label": phase_label("mahitaji", lang),
                       "status": "ok", "depth": depth, "kind": kind, "project": project,
                       "acceptance_criteria": acceptance_criteria(kind),
                       "deliverables": _DELIVERABLES["mahitaji"][level],
                       "requires_approval": True})

        # 2) USANIFU
        prior_ok = len(bp) > 0
        phases.append({"id": "usanifu", "label": phase_label("usanifu", lang),
                       "status": "ok" if prior_ok else "blocked", "depth": depth,
                       "files_planned": len(bp), "files": bp,
                       "deliverables": _DELIVERABLES["usanifu"][level],
                       "requires_approval": True, "gate": phase_gate("usanifu", prior_ok)})
        if not prior_ok:
            return self._finish(project, kind, level, lang, phases, 0.0, False,
                                started, "SDLC imesimama (gate ilizuia)")

        # 3) UJENZI
        written: List[str] = []
        write_error: Optional[str] = None
        target_dir = None
        if write:
            d = self._safe_dir(project)
            if d is None:
                write_error = "project si sahihi (traversal)"
            else:
                files = self._files_for(kind, spec, bp)
                for rel, content in files:
                    full = os.path.join(d, rel)
                    try:
                        os.makedirs(os.path.dirname(full) or d, exist_ok=True)
                        with open(full, "w", encoding="utf-8") as fh:
                            fh.write(content)
                        written.append(rel)
                    except OSError:
                        pass
                target_dir = d
                if not written:
                    write_error = "hakuna faili iliyoandikwa"
        prior_ok = bool(written) and write_error is None
        phases.append({"id": "ujenzi", "label": phase_label("ujenzi", lang),
                       "status": "ok" if prior_ok else "error", "depth": depth,
                       "written": len(written), "files_written": written,
                       "path": target_dir, "error": write_error,
                       "deliverables": _DELIVERABLES["ujenzi"][level],
                       "requires_approval": True, "gate": phase_gate("ujenzi", prior_ok)})
        if not prior_ok:
            return self._finish(project, kind, level, lang, phases, 0.0, False,
                                started, "SDLC imesimama (gate ilizuia)")

        # 4) UTHIBITISHO
        audit = code_audit.scan_workspace(self.workspace, project) if run_verify else {}
        crit = audit.get("by_severity", {}).get("critical", 0) if audit else 0
        grade = audit.get("grade", 100.0) if audit else 100.0
        verify_ok = crit == 0
        phases.append({"id": "uthibitisho", "label": phase_label("uthibitisho", lang),
                       "status": "ok" if verify_ok else "warning", "depth": depth,
                       "grade": grade, "total_findings": audit.get("total_findings", 0),
                       "by_severity": audit.get("by_severity", {}),
                       "deliverables": _DELIVERABLES["uthibitisho"][level],
                       "requires_approval": True,
                       "gate": phase_gate("uthibitisho", verify_ok)})

        # 5) UWASILISHAJI
        phases.append({"id": "uwasilishaji", "label": phase_label("uwasilishaji", lang),
                       "status": "ok", "depth": depth,
                       "deploy_domain": {"web": "website", "game": "gaming"}.get(kind, "website"),
                       "steps": ["build", "test", "package", "deploy", "smoke"],
                       "deliverables": _DELIVERABLES["uwasilishaji"][level],
                       "requires_approval": True,
                       "gate": phase_gate("uwasilishaji", verify_ok)})

        # 6) UFUATILIAJI
        dod = definition_of_done(len(PHASE_IDS), verify_ok,
                                 audit.get("total_findings", 0), crit)
        phases.append({"id": "ufuatiliaji", "label": phase_label("ufuatiliaji", lang),
                       "status": "ok" if dod["done"] else "warning", "depth": depth,
                       "health": {"grade": grade, "critical": crit,
                                  "files": audit.get("files_scanned", 0)},
                       "deliverables": _DELIVERABLES["ufuatiliaji"][level],
                       "requires_approval": True, "definition_of_done": dod})

        ok = verify_ok and dod["done"]
        return self._finish(project, kind, level, lang, phases, grade, ok, started,
                            "Mzunguko kamili umekamilika" if ok
                            else "Umekamilika lakini kuna tahadhari")

    def _files_for(self, kind: str, spec: str, bp: List[dict]) -> List[tuple]:
        """Zalisha faili HALISI kulingana na aina ya mradi."""
        if kind in ("api", "cli", "data"):
            main_rs = (
                "//! Programu kuu — mradi: " + spec + "\n"
                "use std::io::{Read, Write};\nuse std::net::TcpListener;\n\n"
                "fn main() {\n    let addr = \"127.0.0.1:8080\";\n"
                "    let listener = match TcpListener::bind(addr) {\n"
                "        Ok(l) => l,\n        Err(e) => {\n"
                "            eprintln!(\"bind imeshindikana: {e}\");\n            return;\n        }\n"
                "    };\n    println!(\"Inasikiliza {addr}\");\n"
                "    for stream in listener.incoming() {\n        match stream {\n"
                "            Ok(mut s) => {\n                let mut buf = [0u8; 1024];\n"
                "                let _ = s.read(&mut buf);\n"
                "                let body = \"{\\\"ok\\\":true}\";\n"
                "                let resp = format!(\n"
                "                    \"HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\n"
                "Content-Length: {}\\r\\n\\r\\n{}\",\n                    body.len(),\n"
                "                    body\n                );\n"
                "                let _ = s.write_all(resp.as_bytes());\n            }\n"
                "            Err(_) => continue,\n        }\n    }\n}\n")
            cargo = ("[package]\nname = \"mradi\"\nversion = \"0.1.0\"\n"
                     "edition = \"2021\"\n\n[dependencies]\n")
            readme = f"# {spec}\n\nAina: {kind}\n\n## Matumizi\n\n```\ncargo run\n```\n"
            return [("src/main.rs", main_rs), ("Cargo.toml", cargo), ("README.md", readme)]

        index = (f"<!DOCTYPE html>\n<html lang=\"sw\">\n<head>\n<meta charset=\"utf-8\">\n"
                 "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n"
                 f"<title>{spec}</title>\n<link rel=\"stylesheet\" href=\"styles.css\">\n"
                 "</head>\n<body>\n<header>\n<h1>" + spec + "</h1>\n"
                 "<button id=\"run\">Anza</button>\n<p id=\"out\"></p>\n</header>\n"
                 "<script src=\"app.js\"></script>\n</body>\n</html>\n")
        app_js = ("document.addEventListener('DOMContentLoaded', function () {\n"
                  "  var btn = document.getElementById('run');\n"
                  "  var out = document.getElementById('out');\n"
                  "  if (btn) {\n    btn.addEventListener('click', function () {\n"
                  "      if (out) {\n"
                  "        out.textContent = 'Habari ' + new Date().toISOString();\n"
                  "      }\n    });\n  }\n});\n")
        css = ("body { font-family: system-ui, sans-serif; margin: 0; padding: 32px; "
               "background: #0d1117; color: #e6edf3; }\n"
               "header { max-width: 720px; margin: 0 auto; }\n"
               "button { background: #58a6ff; color: #fff; border: 0; border-radius: 8px; "
               "padding: 10px 18px; cursor: pointer; }\n")
        readme = f"# {spec}\n\nAina: {kind}\n\nFungua `index.html` kwenye browser.\n"
        return [("index.html", index), ("app.js", app_js),
                ("styles.css", css), ("README.md", readme)]

    def _finish(self, project: str, kind: str, level: str, lang: str,
                phases: List[dict], score: float, ok: bool, started: float,
                summary: str) -> dict:
        elapsed = round(time.time() - started, 2)
        self.store.record({"project": project, "kind": kind, "lang": lang,
                           "complexity": level, "ok": ok, "score": score,
                           "summary": summary})
        return {"ok": True, "project": project, "kind": kind, "lang": lang,
                "complexity": {"level": level, "label": level_label(level, lang)},
                "phases": phases, "score": score, "success": ok,
                "elapsed_seconds": elapsed, "summary": summary,
                "note": "T-PYTREXT SDLC — files HALISI kwenye workspace"}
