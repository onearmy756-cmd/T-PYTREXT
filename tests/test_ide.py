"""Tests for MBILINYI IDE integration — pytrex.ide, pytrex.ide_server, pytrex.agent_server.

Hizi hujaribu njia HALISI: zana zinaendesha, protocol inajibu, ReAct loop
inaendesha tool halisi. Hakuna mocks kwa zana; LLM pekee ndiyo inasimulika
(kwa kuwa haiwezi kuitwa kwenye CI bila key).
"""
import json
import os
import time
import urllib.request
import urllib.error

import pytest

from pytrex.ide import IDEEngine, IDEToolRegistry, IDETool, MBILINYIIDE, ide_home
from pytrex.ide_server import IDEServer, ToolBridge
from pytrex.agent_server import (
    AgentServer, AgentRuntime, ConversationStore, Conversation,
    message_event, action_event, observation_event, state_event, error_event,
)


# ═══════════════════════════════════════════════════════════
# pytrex.ide — registry + engine
# ═══════════════════════════════════════════════════════════

class TestIDEToolRegistry:
    def test_catalog_has_tools(self):
        reg = IDEToolRegistry()
        cat = reg.catalog()
        assert cat["total"] >= 17
        assert "core" in cat["categories"]

    def test_known_tools_present(self):
        reg = IDEToolRegistry()
        for name in ("api", "lsp", "dap", "build", "swarm", "canvas"):
            assert reg.get(name) is not None, name

    def test_by_category(self):
        reg = IDEToolRegistry()
        agent_tools = {t.name for t in reg.by_category("agent")}
        assert {"swarm", "engineer"} <= agent_tools

    def test_tool_to_dict(self):
        t = IDETool("x", "x.rs", "cat", "desc", "/e")
        d = t.to_dict()
        assert d["name"] == "x" and d["module"] == "x.rs"


class TestIDEEngine:
    def test_ide_home_found(self):
        # Repo hii ina ide/ vendored — lazima ipatikane.
        assert ide_home() is not None
        assert os.path.isdir(os.path.join(ide_home(), "openhands"))

    def test_status_probes_components(self):
        engine = IDEEngine()
        st = engine.status()
        assert st["available"] is True
        assert set(st["components"]) == {"canvas", "desktop", "extension-app", "backend", "skills"}

    def test_canvas_available_reflects_build(self):
        engine = IDEEngine()
        status = engine.canvas_status()
        build = os.path.join(engine.home, "openhands", "build", "index.html")
        assert status.available == os.path.isfile(build)

    def test_plan_lists_needed_steps(self):
        engine = IDEEngine()
        plan = engine.plan()
        assert isinstance(plan["steps"], list)

    def test_skills_found(self):
        engine = IDEEngine()
        st = engine.skills_status()
        assert st.present is True


class TestMBILINYIIDE:
    def test_app_status(self):
        app = MBILINYIIDE(name="Test IDE")
        assert app.status()["available"] is True

    def test_app_has_tool_registry(self):
        app = MBILINYIIDE(name="Test IDE")
        assert app.ide_tools.catalog()["total"] >= 17


# ═══════════════════════════════════════════════════════════
# ToolBridge — zana halisi za T-PYTREXT
# ═══════════════════════════════════════════════════════════

class TestToolBridge:
    @pytest.fixture
    def bridge(self, tmp_path):
        return ToolBridge(workspace=str(tmp_path))

    def test_catalog(self, bridge):
        cat = bridge.catalog()
        assert "terminal" in cat["pytrex_tools"]
        assert cat["ide_tools"]["total"] >= 17

    def test_terminal_runs_real_command(self, bridge):
        r = bridge.run("terminal", command="echo MBILINYI")
        assert r["status"] == "ok"
        assert "MBILINYI" in r["output"]

    def test_editor_write_then_read(self, bridge):
        r = bridge.run("editor", action="save", path=os.path.join(bridge.workspace, "a.py"),
                       content="print(1)\n")
        assert r["status"] == "ok"
        r = bridge.run("editor", action="open", path=os.path.join(bridge.workspace, "a.py"))
        assert r["status"] == "ok"
        assert r["language"] == "python"

    def test_linter_finds_issues(self, bridge):
        r = bridge.run("linter", code="import os\nprint('x')\n")
        assert r["status"] == "ok"
        assert r["total_issues"] >= 1

    def test_formatter_indents(self, bridge):
        r = bridge.run("formatter", code="def f():\nreturn 1", language="python")
        assert r["status"] == "ok"
        assert "    return 1" in r["formatted"]

    def test_security_scan(self, bridge):
        r = bridge.run("security", input_text="<script>alert(1)</script>")
        assert "status" in r

    def test_unknown_tool_is_honest(self, bridge):
        r = bridge.run("does_not_exist")
        assert r["status"] == "error"
        assert r["ok"] is False

    def test_vcs_commit_and_history(self, bridge):
        bridge.run("vcs", action="commit", resource_id="r1", data={"a": 1}, message="m")
        r = bridge.run("vcs", action="history", resource_id="r1")
        assert r["status"] == "ok"
        assert len(r["history"]) >= 1


# ═══════════════════════════════════════════════════════════
# IDEServer — HTTP
# ═══════════════════════════════════════════════════════════

def _http(port, path, data=None, method=None):
    url = f"http://127.0.0.1:{port}{path}"
    if data is not None:
        req = urllib.request.Request(url, data=json.dumps(data).encode(),
                                     headers={"Content-Type": "application/json"},
                                     method=method or "POST")
    else:
        req = urllib.request.Request(url, method=method or "GET")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw = resp.read().decode()
            try:
                return resp.status, json.loads(raw)
            except json.JSONDecodeError:
                return resp.status, {"_raw": raw}
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        try:
            return e.code, json.loads(raw)
        except json.JSONDecodeError:
            return e.code, {"_raw": raw}


@pytest.fixture
def server(tmp_path):
    srv = AgentServer(host="127.0.0.1", port=8137, workspace=str(tmp_path))
    srv.start(background=True)
    time.sleep(0.3)
    yield srv
    srv.stop()


class TestAgentServerProtocol:
    def test_server_info_version(self, server):
        st, body = _http(8137, "/server_info")
        assert st == 200
        assert body["version"] == "1.32.0"
        assert "workspaces" in body["capabilities"]

    def test_probes(self, server):
        assert _http(8137, "/alive")[1]["status"] == "ok"
        assert _http(8137, "/health")[1]["status"] == "ok"
        assert _http(8137, "/ready")[1]["status"] == "ok"

    def test_settings_shape(self, server):
        st, body = _http(8137, "/api/settings")
        assert "agent_settings" in body
        assert "llm_api_key_is_set" in body

    def test_settings_patch_sets_key(self, server):
        _http(8137, "/api/settings", {"agent_settings_diff": {"llm_api_key": "sk-test"}},
              method="PATCH")
        st, body = _http(8137, "/api/settings")
        assert body["llm_api_key_is_set"] is True

    def test_conversation_lifecycle(self, server):
        st, convo = _http(8137, "/api/conversations", {"title": "T"})
        assert st == 200
        cid = convo["id"]
        st, listing = _http(8137, f"/api/conversations/{cid}")
        assert listing["id"] == cid
        st, page = _http(8137, f"/api/conversations/{cid}/events")
        assert "items" in page
        _http(8137, f"/api/conversations/{cid}", method="DELETE")
        st, missing = _http(8137, f"/api/conversations/{cid}")
        assert st == 404

    def test_skills_from_canvas(self, server):
        st, body = _http(8137, "/api/v1/skills")
        assert body["total_count"] >= 1

    def test_unknown_route_404(self, server):
        st, _ = _http(8137, "/nope")
        assert st == 404


class TestCanvasServing:
    def test_ide_html(self, server):
        st, _ = _http(8137, "/ide")
        assert st == 200

    def test_ide_asset(self, server):
        # index.html references /ide/assets/... — huthibitisha assets zinafika.
        url = f"http://127.0.0.1:8137/ide"
        with urllib.request.urlopen(url, timeout=10) as resp:
            html = resp.read().decode()
        assert "/ide/assets/" in html


# ═══════════════════════════════════════════════════════════
# AgentRuntime — ReAct loop na zana HALISI
# ═══════════════════════════════════════════════════════════

class TestEventBuilders:
    def test_message_event(self):
        e = message_event("user", "hi")
        assert e["kind"] == "MessageEvent"
        assert e["llm_message"]["role"] == "user"

    def test_action_and_observation(self):
        a = action_event("run_bash", "c1", {"command": "ls"})
        o = observation_event("run_bash", "c1", a["id"], {"kind": "TerminalObservation", "content": "x"})
        assert a["tool_name"] == "run_bash"
        assert o["action_id"] == a["id"]

    def test_state_and_error(self):
        assert state_event("agent_status", "running")["key"] == "agent_status"
        assert error_event("boom")["detail"] == "boom"


class TestAgentRuntimeReAct:
    def test_tool_call_executes_real_command(self, tmp_path):
        store = ConversationStore()
        bridge = ToolBridge(workspace=str(tmp_path))
        rt = AgentRuntime(store, bridge, {"api_key": "fake"}, max_iterations=5)

        calls = {"n": 0}

        def fake_llm(messages):
            calls["n"] += 1
            if calls["n"] == 1:
                return {"role": "assistant", "content": None, "tool_calls": [{
                    "id": "c1", "type": "function",
                    "function": {"name": "run_bash",
                                 "arguments": json.dumps({"command": "echo REAL-OUT"})}}]}
            return {"role": "assistant", "content": "done"}

        rt._llm_chat = fake_llm
        convo = store.create("t")
        rt.run(convo, "run echo")

        kinds = [e["kind"] for e in convo.snapshot()]
        assert "ActionEvent" in kinds
        assert "ObservationEvent" in kinds
        obs = [e for e in convo.snapshot() if e["kind"] == "ObservationEvent"][0]
        assert "REAL-OUT" in obs["observation"]["content"]
        assert convo.agent_status == "idle"

    def test_no_llm_key_is_honest_error(self, tmp_path):
        store = ConversationStore()
        bridge = ToolBridge(workspace=str(tmp_path))
        rt = AgentRuntime(store, bridge, {"api_key": ""}, max_iterations=2)
        convo = store.create("t")
        rt.run(convo, "hello")
        kinds = [e["kind"] for e in convo.snapshot()]
        assert "ConversationErrorEvent" in kinds

    def test_write_file_tool(self, tmp_path):
        store = ConversationStore()
        bridge = ToolBridge(workspace=str(tmp_path))
        rt = AgentRuntime(store, bridge, {"api_key": "fake"}, max_iterations=5)
        calls = {"n": 0}

        def fake_llm(messages):
            calls["n"] += 1
            if calls["n"] == 1:
                return {"role": "assistant", "content": None, "tool_calls": [{
                    "id": "w1", "type": "function",
                    "function": {"name": "write_file",
                                 "arguments": json.dumps({"path": "out.txt", "content": "hello"})}}]}
            return {"role": "assistant", "content": "wrote"}

        rt._llm_chat = fake_llm
        convo = store.create("t")
        rt.run(convo, "write a file")
        assert os.path.isfile(os.path.join(str(tmp_path), "out.txt"))
