"""Tests za moduli mpya za IDE: code_audit, sdlc, forge, ide_tools, canvas_services.

Hizi hujaribu njia HALISI: kila zana inaendesha na kuthibitisha matokeo ya kweli
(faili zinaandikwa, kanuni zinagundua, SSE hutiririka). Hakuna mocks.
"""
import json
import os
import sys
import urllib.request

import pytest

from pytrex import code_audit, forge, sdlc, ide_tools, canvas_services
from pytrex.ide_server import IDEServer
from pytrex.agent_server import AgentServer


@pytest.fixture()
def tmp_ws(tmp_path):
    return str(tmp_path)


@pytest.fixture()
def engine(tmp_ws):
    store = sdlc.SDLCStore(os.path.join(tmp_ws, ".pytrex", "sdlc.json"))
    return sdlc.SDLCEngine(tmp_ws, store)


# ═══════════════════════════════════════════════════════════
# code_audit
# ═══════════════════════════════════════════════════════════

class TestCodeAudit:
    def test_rules_catalog(self):
        cat = code_audit.rules_catalog()
        assert cat["ok"] is True
        assert cat["total"] >= 20
        ids = {r["id"] for r in cat["rules"]}
        assert "SEC-001" in ids and "BUG-001" in ids

    def test_detects_secret_and_rce(self):
        r = code_audit.scan_text("k = 'sk-abc'\nos.system('x')\n", "python")
        assert r["total_findings"] >= 2
        assert r["by_severity"].get("critical", 0) >= 1
        assert r["grade"] < 100

    def test_clean_code_grades_100(self):
        r = code_audit.scan_text("x = 1\n", "python")
        assert r["grade"] == 100.0

    def test_lang_filter_skips_other_langs(self):
        # .unwrap() ni rule ya rust pekee
        r = code_audit.scan_text("let x = y.unwrap();", "rust")
        assert any(f["rule"] == "BUG-001" for f in r["findings"])
        r2 = code_audit.scan_text("x.unwrap()", "python")
        assert not any(f["rule"] == "BUG-001" for f in r2["findings"])

    def test_scan_workspace_writes_and_finds(self, tmp_ws):
        with open(os.path.join(tmp_ws, "a.py"), "w") as fh:
            fh.write("token = 'ghp_1234567890'\n")
        r = code_audit.scan_workspace(tmp_ws, "")
        assert r["ok"] is True
        assert r["files_scanned"] >= 1
        assert r["by_severity"].get("critical", 0) >= 1

    def test_scan_workspace_rejects_traversal(self, tmp_ws):
        r = code_audit.scan_workspace(tmp_ws, "../etc")
        assert r["ok"] is False

    def test_scan_missing_path_is_honest(self, tmp_ws):
        r = code_audit.scan_workspace(tmp_ws, "does-not-exist")
        assert r["ok"] is False
        assert "haipo" in r["error"]


# ═══════════════════════════════════════════════════════════
# sdlc
# ═══════════════════════════════════════════════════════════

class TestSDLC:
    def test_phase_ids_order(self):
        assert sdlc.PHASE_IDS == ["mahitaji", "usanifu", "ujenzi", "uthibitisho",
                                  "uwasilishaji", "ufuatiliaji"]

    def test_capabilities_has_six_phases(self, engine):
        c = engine.capabilities()
        assert c["ok"] is True
        assert len(c["phases"]) == 6
        assert len(c["guardrails"]) >= 4

    def test_complexity_simple_vs_complex(self):
        s, _, _ = sdlc.assess_complexity("kikokotoo cha kuongeza", "web", 3)
        c, _, _ = sdlc.assess_complexity(
            "enterprise SaaS multi-tenant platform with OAuth, payment, realtime "
            "websocket, GDPR compliance, scalability", "api", 10)
        assert s == "simple"
        assert c == "complex"

    def test_assess_requires_spec(self, engine):
        assert engine.assess("")["ok"] is False

    def test_plan_returns_files_and_criteria(self, engine):
        p = engine.plan("tovuti ya kuuza bidhaa")
        assert p["ok"] is True
        assert p["files_planned"]
        assert len(p["acceptance_criteria"]) >= 4

    def test_gate_blocks_unknown_phase(self, engine):
        g = engine.gate("hakuna", prior_ok=True)
        assert g["ok"] is False
        assert g["blocked"] is True

    def test_run_writes_real_files(self, engine, tmp_ws):
        r = engine.run("kikokotoo rahisi cha kuongeza namba")
        assert r["ok"] is True
        ujenzi = next(p for p in r["phases"] if p["id"] == "ujenzi")
        assert ujenzi["status"] == "ok"
        assert ujenzi["written"] >= 3
        for rel in ujenzi["files_written"]:
            assert os.path.isfile(os.path.join(tmp_ws, r["project"], rel))

    def test_run_rejects_traversal_project(self, engine):
        r = engine.run("x", project="../evil")
        assert r["ok"] is True  # sanitize_name inasafisha
        assert ".." not in r["project"]

    def test_history_records_runs(self, engine):
        engine.run("kikokotoo")
        h = engine.history()
        assert h["count"] >= 1
        assert h["runs"][0]["project"]

    def test_continue_phase_approves(self, engine):
        r = engine.continue_phase("mradi", "mahitaji", approve=True)
        assert r["ok"] is True
        assert "mahitaji" in r["approved"]
        assert r["next"] == "usanifu"

    def test_definition_of_done_blocks(self):
        bad = sdlc.definition_of_done(2, False, 3, 1)
        assert bad["done"] is False
        assert len(bad["blockers"]) >= 3
        good = sdlc.definition_of_done(6, True, 0, 0)
        assert good["done"] is True


# ═══════════════════════════════════════════════════════════
# forge (Autopilot + Swarm)
# ═══════════════════════════════════════════════════════════

class TestForge:
    def test_detect_kind(self):
        assert forge.detect_kind("REST api ya malipo") == "api"
        assert forge.detect_kind("tovuti ya kuuza") == "web"
        assert forge.detect_kind("mobile app") == "mobile"

    def test_autopilot_generates_project(self, engine, tmp_ws):
        ap = forge.Autopilot(tmp_ws, engine)
        r = ap.run("tovuti ya kuuza bidhaa")
        assert r["ok"] is True
        assert len(r["files"]) >= 3
        assert os.path.isdir(os.path.join(tmp_ws, r["project"]))
        steps = {s["step"]: s["status"] for s in r["steps"]}
        assert steps["plan"] == "ok" and steps["scaffold"] == "ok"

    def test_autopilot_status_after_run(self, engine, tmp_ws):
        ap = forge.Autopilot(tmp_ws, engine)
        ap.run("kikokotoo")
        st = ap.status()
        assert st["running"] is False
        assert st["last"]

    def test_swarm_agents_and_run(self, engine, tmp_ws):
        sw = forge.Swarm(tmp_ws, engine)
        assert sw.agents()["total"] >= 4
        r = sw.run("api ya malipo")
        assert r["ok"] is True
        assert len(r["agents"]) >= 4
        assert all(a["contribution"] for a in r["agents"])


# ═══════════════════════════════════════════════════════════
# ide_tools (endpoints 14)
# ═══════════════════════════════════════════════════════════

class TestIDETools:
    @pytest.fixture()
    def tools(self, engine, tmp_ws):
        return ide_tools.IDETools(tmp_ws, engine)

    def test_lsp_servers(self, tools):
        r = tools.dispatch("lsp", {"action": "servers"})
        assert r["ok"] is True and r["servers"]

    def test_lsp_python_diagnostics(self, tools):
        r = tools.dispatch("lsp", {"code": "def f(:\n  pass", "language": "python"})
        assert r["ok"] is True
        assert r["diagnostics"]  # syntax error detected

    def test_dap_adapters(self, tools):
        assert tools.dispatch("dap", {"action": "adapters"})["adapters"]

    def test_complete_python(self, tools):
        r = tools.dispatch("complete", {"code": "import os\nos.", "line": 2, "col": 3})
        assert r["ok"] is True
        assert any(i["label"] for i in r["items"])

    def test_composer_plan_apply(self, tools, tmp_ws):
        p = tools.dispatch("composer", {"files": [{"path": "n.txt", "content": "hi"}]})
        assert p["ok"] is True
        a = tools.dispatch("composer", {"action": "apply", "plan_id": p["plan_id"]})
        assert a["applied"] == ["n.txt"]
        assert os.path.isfile(os.path.join(tmp_ws, "n.txt"))

    def test_build_targets(self, tools):
        assert "targets" in tools.dispatch("build", {"action": "targets"})

    def test_pipeline_run(self, tools):
        r = tools.dispatch("pipeline", {"action": "run"})
        assert "stages" in r

    def test_testkit_suites(self, tools):
        assert "suites" in tools.dispatch("testkit", {"action": "suites"})

    def test_e2e_run_real_html(self, tools, tmp_ws):
        with open(os.path.join(tmp_ws, "index.html"), "w") as fh:
            fh.write("<!doctype html><html><head><title>t</title></head>"
                     "<body><script src='a.js'></script></body></html>")
        r = tools.dispatch("e2e", {"path": "index.html"})
        assert r["ok"] is True
        assert all(c["pass"] for c in r["checks"])

    def test_lab_env(self, tools):
        r = tools.dispatch("lab", {"action": "env"})
        assert r["ok"] is True and "python" in r

    def test_lab_test_executes(self, tools):
        r = tools.dispatch("lab", {"action": "test", "code": "y = 40 + 2"})
        assert r["ok"] is True and "y" in r["names"]

    def test_devices_catalog(self, tools):
        assert tools.dispatch("devices", {})["devices"]

    def test_emulators_web(self, tools):
        assert tools.dispatch("emulators", {"kind": "web"})["ok"] is True

    def test_swarm_agents(self, tools):
        assert tools.dispatch("swarm", {"action": "agents"})["total"] >= 4

    def test_engineer_runs_sdlc(self, tools):
        r = tools.dispatch("engineer", {"spec": "kikokotoo", "write": False})
        assert r["ok"] is True
        assert r["sdlc"]["ok"] is True

    def test_integrations_status(self, tools):
        assert "git" in tools.dispatch("integrations", {"action": "status"})

    def test_audit_rules(self, tools):
        assert tools.dispatch("audit", {"action": "rules"})["total"] >= 20

    def test_sdlc_via_tools(self, tools):
        assert tools.dispatch("sdlc", {"action": "capabilities"})["ok"] is True

    def test_unknown_tool_is_honest(self, tools):
        r = tools.dispatch("nope", {})
        assert r["ok"] is False and "haijulikani" in r["error"]


# ═══════════════════════════════════════════════════════════
# canvas_services
# ═══════════════════════════════════════════════════════════

class TestCanvasServices:
    @pytest.fixture()
    def svc(self, tmp_ws):
        return canvas_services.CanvasServices(
            tmp_ws, os.path.join(tmp_ws, ".pytrex"),
            os.path.join(tmp_ws, "skills"))

    def test_provider_connection_roundtrip(self, svc):
        c = svc.connections.create("My OAI", "openai", "sk-secret")
        assert c["api_key_set"] is True
        assert svc.connections.api_key_for("openai") == "sk-secret"
        assert len(svc.connections.list()) == 1
        svc.connections.delete(c["id"])
        assert svc.connections.list() == []

    def test_provider_connection_requires_key(self, svc):
        with pytest.raises(ValueError):
            svc.connections.create("x", "openai", "")

    def test_public_view_hides_key(self, svc):
        svc.connections.create("x", "openai", "sk-abc")
        pub = svc.connections.list()[0]
        assert "api_key" not in pub and "api_key_enc" not in pub

    def test_secrets_crud(self, svc):
        svc.secrets.upsert("K", "V", "d")
        assert svc.secrets.get("K") == "V"
        assert svc.secrets.list()[0]["name"] == "K"
        assert svc.secrets.delete("K") is True
        assert svc.secrets.get("K") is None

    def test_secrets_search(self, svc):
        svc.secrets.upsert("GITHUB_TOKEN", "x")
        svc.secrets.upsert("AWS_KEY", "y")
        r = svc.secrets.search("github")
        assert r["total_count"] == 1

    def test_mcp_add_and_health(self, svc):
        s = svc.mcp.add("m", "http://127.0.0.1:1")
        assert s["name"] == "m"
        h = svc.mcp.health()
        assert h["total"] == 1 and h["servers"][0]["healthy"] is False

    def test_sandbox_lifecycle(self, svc):
        sb = svc.sandboxes.create("sb")
        assert sb["status"] == "running"
        assert svc.sandboxes.pause(sb["id"])["status"] == "paused"
        assert svc.sandboxes.resume(sb["id"])["status"] == "running"

    def test_config_search(self, svc):
        assert svc.config.search_models("claude")["total_count"] >= 1
        assert svc.config.search_providers()["total_count"] >= 4

    def test_file_upload_download(self, svc):
        import base64
        data = base64.b64encode(b"hello").decode()
        assert svc.files.upload("f.txt", data)["ok"] is True
        r = svc.files.download("f.txt")
        assert base64.b64decode(r["content_base64"]) == b"hello"

    def test_file_rejects_traversal(self, svc):
        assert svc.files.download("../../etc/passwd")["ok"] is False

    def test_organizations(self, svc):
        o = svc.organizations.create("Acme")
        assert svc.organizations.list()["total_count"] == 1
        assert svc.organizations.profiles(o["id"])["organization"] == o["id"]


# ═══════════════════════════════════════════════════════════
# HTTP routes (server-level)
# ═══════════════════════════════════════════════════════════

class TestServerRoutes:
    @pytest.fixture()
    def srv(self, tmp_ws):
        return AgentServer(host="127.0.0.1", port=0, workspace=tmp_ws)

    def _call(self, srv, method, path, body=None, query=None):
        return srv._route(method, path, query or {}, json.dumps(body or {}))

    def test_ide_tool_routes(self, srv):
        for path in ("/api/ide/lsp/servers", "/api/ide/dap/adapters",
                     "/api/ide/build/targets", "/api/ide/swarm/agents",
                     "/api/ide/code/rules"):
            r = self._call(srv, "GET", path)
            assert r.get("ok") is not False, path

    def test_sdlc_routes(self, srv):
        assert self._call(srv, "GET", "/api/sdlc/capabilities")["ok"] is True
        assert self._call(srv, "POST", "/api/sdlc/assess",
                          {"spec": "tovuti"})["ok"] is True

    def test_autopilot_status(self, srv):
        assert self._call(srv, "GET", "/api/autopilot/status")["ok"] is True

    def test_provider_connections_route(self, srv):
        r = self._call(srv, "POST", "/api/llm/provider-connections",
                       {"display_name": "O", "provider": "openai", "api_key": "sk-x"})
        assert r["provider"] == "openai"
        lst = self._call(srv, "GET", "/api/llm/provider-connections")
        assert lst["total_count"] == 1

    def test_secrets_route(self, srv):
        assert self._call(srv, "POST", "/api/v1/secrets",
                          {"name": "K", "value": "V"})["name"] == "K"
        assert self._call(srv, "GET", "/api/v1/secrets")["total_count"] == 1

    def test_sandbox_route(self, srv):
        r = self._call(srv, "POST", "/api/v1/sandboxes", {"name": "s"})
        assert r["status"] == "running"

    def test_app_conversations_route(self, srv):
        c = self._call(srv, "POST", "/api/conversations", {"title": "T"})
        r = self._call(srv, "GET", f"/api/v1/app-conversations/{c['id']}")
        assert r["app_conversation_id"] == c["id"]

    def test_sse_sdlc_plan(self, srv):
        r = self._call(srv, "POST", "/api/sdlc/stream",
                       {"action": "plan", "spec": "tovuti"})
        assert r["_sse"] is True
        assert any(e["event"] == "done" for e in r["_sse_events"])

    def test_sse_audit(self, srv):
        r = self._call(srv, "POST", "/api/ide/code/audit/stream",
                       {"code": "sk-x\nos.system(1)", "language": "python"})
        assert r["_sse"] is True
        assert any(e["event"] == "summary" for e in r["_sse_events"])

    def test_bash_route(self, srv):
        r = self._call(srv, "POST", "/api/bash/execute_bash_command",
                       {"command": "echo hi"})
        assert "hi" in r.get("output", "")


class TestSSEOverHTTP:
    def test_sse_real_http(self, tmp_ws):
        srv = AgentServer(host="127.0.0.1", port=0, workspace=tmp_ws)
        srv.start(background=True)
        try:
            port = srv._httpd.server_address[1]
            req = urllib.request.Request(
                f"http://127.0.0.1:{port}/api/sdlc/stream",
                data=json.dumps({"action": "plan", "spec": "tovuti"}).encode(),
                headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=30) as r:
                assert r.headers.get("Content-Type") == "text/event-stream"
                body = r.read().decode()
            assert "event: done" in body
            assert "event: plan" in body
        finally:
            srv.stop()
