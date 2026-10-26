"""pytrex.code_audit — uchambuzi HALISI wa code (usalama, bugs, quality).

Imepandishwa kutoka MTECHOS `code_audit.rs` (chanzo kimoja cha kanuni).
Ni deterministic (substring/regex) — inagundua secrets, mifumo hatarishi,
bugs za kawaida, na matatizo ya quality. Matokeo yana severity + line +
snippet, na grade (100 = safi).
"""

from __future__ import annotations

import os
import re
from typing import Dict, List, Optional

# ---------------------------------------------------------------------------
# Kanuni — chanzo KIMOJA cha ukweli (kama MTECHOS RULES)
# ---------------------------------------------------------------------------

RULES: List[Dict[str, str]] = [
    # ---- SECRETS ----
    {"id": "SEC-001", "severity": "critical", "lang": "any", "category": "secret",
     "title": "API key ya OpenAI", "needle": "sk-",
     "fix": "Ondoa key kwenye code; tumia env var / secret manager."},
    {"id": "SEC-002", "severity": "critical", "lang": "any", "category": "secret",
     "title": "GitHub token (ghp_/gho_)", "needle": "ghp_",
     "fix": "Revoke token; weka kwenye env, si code."},
    {"id": "SEC-003", "severity": "high", "lang": "any", "category": "secret",
     "title": "AWS Access Key", "needle": "AKIA",
     "fix": "Tumia IAM role / env; usiweke key kwenye code."},
    {"id": "SEC-004", "severity": "high", "lang": "any", "category": "secret",
     "title": "Private key block", "needle": "PRIVATE KEY",
     "fix": "Hifadhi key kwenye secret store, si repo."},
    {"id": "SEC-005", "severity": "high", "lang": "any", "category": "secret",
     "title": "JWT / bearer token", "needle": "Bearer ",
     "fix": "Usiweke token kwenye code; tumia env."},
    # ---- DANGEROUS PATTERNS ----
    {"id": "SEC-013", "severity": "high", "lang": "any", "category": "rce",
     "title": "eval() / Function() kwenye JS", "needle": "eval(",
     "fix": "Epuka eval/Function; tumia JSON.parse."},
    {"id": "SEC-014", "severity": "high", "lang": "any", "category": "cmd",
     "title": "os.system / shell exec", "needle": "os.system",
     "fix": "Tumia subprocess args-array."},
    # ---- INJECTION ----
    {"id": "SEC-020", "severity": "high", "lang": "sql", "category": "sqli",
     "title": "SQL kwa concatenation", "needle": 'SELECT " +',
     "fix": "Tumia parameterized queries / prepared statements."},
    {"id": "SEC-021", "severity": "high", "lang": "js", "category": "xss",
     "title": "innerHTML kwa data", "needle": "innerHTML =",
     "fix": "Tumia textContent au sanitize (DOMPurify)."},
    {"id": "SEC-022", "severity": "high", "lang": "js", "category": "xss",
     "title": "document.write", "needle": "document.write(",
     "fix": "Tumia DOM API salama."},
    {"id": "SEC-023", "severity": "medium", "lang": "any", "category": "traversal",
     "title": "Path kwa '..'", "needle": "../",
     "fix": "Sanitize path; thibitisha ndani ya workspace."},
    # ---- WEAK CRYPTO / AUTH ----
    {"id": "SEC-030", "severity": "high", "lang": "any", "category": "crypto",
     "title": "MD5 hash", "needle": "md5", "fix": "Tumia SHA-256 / Argon2."},
    {"id": "SEC-031", "severity": "high", "lang": "any", "category": "crypto",
     "title": "DES / RC4", "needle": "DES", "fix": "Tumia AES-GCM."},
    {"id": "SEC-032", "severity": "medium", "lang": "any", "category": "auth",
     "title": "verify=False (TLS off)", "needle": "verify=False",
     "fix": "Washa uthibitisho wa TLS."},
    {"id": "SEC-033", "severity": "medium", "lang": "any", "category": "auth",
     "title": "CORS wildcard", "needle": "Access-Control-Allow-Origin: *",
     "fix": "Weka origin maalum."},
    # ---- BUGS ----
    {"id": "BUG-001", "severity": "medium", "lang": "rust", "category": "panic",
     "title": "unwrap() inaweza panic", "needle": ".unwrap()",
     "fix": "Tumia `?` au `unwrap_or_else`."},
    {"id": "BUG-002", "severity": "medium", "lang": "rust", "category": "panic",
     "title": "panic! kwenye path ya runtime", "needle": "panic!(",
     "fix": "Rudisha Result badala ya panic."},
    {"id": "BUG-005", "severity": "low", "lang": "js", "category": "bug",
     "title": "== badala ya ===", "needle": " == ", "fix": "Tumia `===`."},
    # ---- QUALITY ----
    {"id": "QLT-001", "severity": "low", "lang": "any", "category": "todo",
     "title": "TODO/FIXME", "needle": "TODO", "fix": "Kamilisha au fungua issue."},
    {"id": "QLT-002", "severity": "low", "lang": "any", "category": "debug",
     "title": "print() ya debug", "needle": "print(", "fix": "Tumia logger."},
    {"id": "QLT-003", "severity": "low", "lang": "js", "category": "debug",
     "title": "console.log", "needle": "console.log",
     "fix": "Ondoa debug kabla ya release."},
    # ---- ML / AI SAFETY ----
    {"id": "ML-001", "severity": "high", "lang": "rust", "category": "deserialization",
     "title": "Deserialization isiyo salama (bincode)", "needle": "bincode::deserialize",
     "fix": "Tumia safetensors au JSON yenye schema."},
    {"id": "ML-003", "severity": "high", "lang": "elixir", "category": "rce",
     "title": "Code.eval_string (RCE)", "needle": "Code.eval",
     "fix": "Epuka eval; tumia parser/dispatch ya wazi."},
    {"id": "ML-004", "severity": "medium", "lang": "elixir", "category": "dos",
     "title": "String.to_atom kwenye data (atom exhaustion)",
     "needle": "String.to_atom(", "fix": "Tumia String.to_existing_atom."},
    {"id": "ML-005", "severity": "high", "lang": "r", "category": "cmd",
     "title": "system() kwenye R (shell)", "needle": "system(",
     "fix": "Tumia system2() yenye args-array bila shell."},
    {"id": "ML-006", "severity": "medium", "lang": "r", "category": "rce",
     "title": "eval() kwenye R", "needle": "eval(",
     "fix": "Tumia do.call / parse+eval yenye udhibiti."},
]

_WEIGHTS = {"critical": 10.0, "high": 7.0, "medium": 4.0, "low": 2.0}

_EXT_LANG = {
    ".py": "python", ".rs": "rust", ".js": "js", ".jsx": "js", ".ts": "js",
    ".tsx": "js", ".ex": "elixir", ".exs": "elixir", ".r": "r",
    ".sql": "sql", ".go": "go", ".rb": "ruby", ".java": "java",
    ".html": "html", ".css": "css", ".json": "json", ".md": "markdown",
    ".toml": "toml", ".yaml": "yaml", ".yml": "yaml",
}

_SKIP_DIRS = {".git", "node_modules", "target", "__pycache__", ".venv",
              "venv", "dist", "build", ".mypy_cache", ".pytest_cache"}

_MAX_BYTES = 400_000


def lang_for(path: str) -> str:
    _, ext = os.path.splitext(path.lower())
    return _EXT_LANG.get(ext, "")


def severity_weight(sev: str) -> float:
    return _WEIGHTS.get(sev, 2.0)


def analyze_content(lang: str, content: str) -> List[dict]:
    """Changanua maudhui ya faili moja kulingana na kanuni (lang-aware)."""
    findings: List[dict] = []
    lines = content.splitlines()
    for rule in RULES:
        if lang and rule["lang"] not in ("any", lang):
            continue
        needle = rule["needle"]
        for i, line in enumerate(lines):
            if needle in line:
                findings.append({
                    "rule": rule["id"],
                    "severity": rule["severity"],
                    "category": rule["category"],
                    "title": rule["title"],
                    "lang": rule["lang"],
                    "line": i + 1,
                    "snippet": line.strip()[:200],
                    "fix": rule["fix"],
                })
    return findings


def scan_text(content: str, lang: str = "") -> dict:
    """Changanua maudhui yaliyotumwa (bila kuandika). Hurejesha ripoti kamili."""
    findings = analyze_content(lang, content)
    return _summarize(findings, files=[{"path": "<text>", "findings": len(findings),
                                        "lang": lang or "text"}], scanned=1)


def scan_workspace(root: str, rel_path: str = "") -> dict:
    """Changanua folda/faili ndani ya workspace. Hukataa traversal."""
    root = os.path.abspath(root)
    if rel_path.strip():
        rel = rel_path.strip().lstrip("/")
        if ".." in rel.split("/"):
            return {"ok": False, "error": "path nje ya workspace"}
        target = os.path.join(root, rel)
    else:
        target = root
    if not os.path.exists(target):
        return {"ok": False, "error": f"path haipo ndani ya workspace: {rel_path}",
                "target": target, "workspace": root}

    findings: List[dict] = []
    files: List[dict] = []
    budget = 500

    def scan_file(full: str) -> None:
        nonlocal budget
        if budget <= 0:
            return
        budget -= 1
        try:
            if os.path.getsize(full) > _MAX_BYTES:
                return
            with open(full, "r", encoding="utf-8", errors="ignore") as fh:
                content = fh.read()
        except OSError:
            return
        rel = os.path.relpath(full, root)
        f = analyze_content(lang_for(rel), content)
        files.append({"path": rel, "findings": len(f), "lang": lang_for(rel)})
        for item in f:
            item["path"] = rel
            findings.append(item)

    if os.path.isfile(target):
        scan_file(target)
    else:
        for dirpath, dirnames, filenames in os.walk(target):
            dirnames[:] = [d for d in dirnames if d not in _SKIP_DIRS]
            for name in filenames:
                if budget <= 0:
                    break
                scan_file(os.path.join(dirpath, name))

    return _summarize(findings, files=files, scanned=len(files))


def _summarize(findings: List[dict], files: List[dict], scanned: int) -> dict:
    by_sev: Dict[str, int] = {}
    by_cat: Dict[str, int] = {}
    score = 0.0
    for f in findings:
        sev = f.get("severity", "low")
        cat = f.get("category", "other")
        by_sev[sev] = by_sev.get(sev, 0) + 1
        by_cat[cat] = by_cat.get(cat, 0) + 1
        score += severity_weight(sev)
    grade = max(0.0, 100.0 - score)
    return {
        "ok": True,
        "files_scanned": scanned,
        "files_with_findings": sum(1 for f in files if f.get("findings", 0) > 0),
        "total_findings": len(findings),
        "by_severity": by_sev,
        "by_category": by_cat,
        "grade": round(grade, 1),
        "files": files,
        "findings": findings[:400],
    }


def rules_catalog() -> dict:
    return {"ok": True, "total": len(RULES),
            "rules": [{"id": r["id"], "severity": r["severity"], "lang": r["lang"],
                       "category": r["category"], "title": r["title"], "fix": r["fix"]}
                      for r in RULES]}
