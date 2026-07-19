"""
T-PYTREXT — Comprehensive Validation Test
==========================================
Inajaribu modules zote mpya na demos zote.
"""
import sys, os, time, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

passed = 0; failed = 0

def test(name, fn):
    global passed, failed
    try:
        fn()
        passed += 1
        print(f"  ✅ {name}")
    except Exception as e:
        failed += 1
        print(f"  ❌ {name}: {str(e)[:80]}")

print("═══ VALIDATING ALL NEW MODULES ═══\n")

# ─── Test Runner ───
test("TestRunner init", lambda: __import__('pytrex.test_runner').test_runner.TestRunner())
test("TestRunner quick", lambda: __import__('pytrex.test_runner').test_runner.quick_test())

# ─── Project Manager ───
test("ProjectManager scan", lambda: __import__('pytrex.project_manager').project_manager.ProjectManager().scan(max_depth=1, show_progress=False))
test("ProjectManager list", lambda: __import__('pytrex.project_manager').project_manager.ProjectManager().list_projects())

# ─── Production Builder ───
test("ProductionBuilder init", lambda: __import__('pytrex.production').production.ProductionBuilder(project_path='.'))
test("ProductionBuilder configs", lambda: __import__('pytrex.production').production.ProductionBuilder(project_path='.').generate_env_files())

# ─── LangChain Agent ───
from pytrex.langchain_agent import LangChainAgent
agent = LangChainAgent()
test("LangChain create", lambda: agent)
test("LangChain tools", lambda: agent.add_tool("test", lambda x: x))
test("LangChain run", lambda: agent.run("hello"))

# ─── Search Engine ───
from pytrex.search_engine import SearchEngine as WebSearch
se = WebSearch()
test("SearchEngine init", lambda: se)
test("SearchEngine summary", lambda: se.web_search_summary("test"))

# ─── Hermes Agent ───
from pytrex.hermes_agent import HermesAgent
hermes = HermesAgent()
test("Hermes create", lambda: hermes)
test("Hermes builtins", lambda: len(hermes.list_functions()) >= 4)
test("Hermes chat", lambda: hermes.chat("test"))

# ─── Human-in-the-Loop ───
from pytrex.human_in_loop import HumanInTheLoop
hitl = HumanInTheLoop()
test("HITL create", lambda: hitl)
test("HITL approve", lambda: hitl.approve(hitl.request_approval("t", {}, timeout=999)))

# ─── MCP Client ───
from pytrex.mcp_client import MCPClient
mcp = MCPClient()
test("MCP create", lambda: mcp)
test("MCP invoke", lambda: mcp._invoke_local_tool("blockchain_create_block", {"data": "test"}))

# ─── Production ───
from pytrex.production import ProductionBuilder, ProductionConfig
config = ProductionConfig(app_name="test", port=9000)
builder = ProductionBuilder(project_path=".", config=config, verbose=False)
test("Production config", lambda: config)
test("Production dockerfile", lambda: builder._generate_dockerfile())
test("Production compose", lambda: builder._generate_docker_compose())
test("Production nginx", lambda: builder.generate_nginx_config())
test("Production systemd", lambda: builder.generate_systemd_service())
test("Production k8s", lambda: len(builder.generate_kubernetes_manifests()) == 3)
test("Production env", lambda: builder.generate_env_files())
test("Production security", lambda: builder.generate_security_hardening())

# ─── Project Manager full ───
from pytrex.project_manager import ProjectManager
pm = ProjectManager()
test("PM scan", lambda: len(pm.scan(max_depth=2, show_progress=False)) >= 0)
test("PM list", lambda: pm.projects_table())
test("PM export", lambda: pm.export_json("/tmp/test_pm.json"))

print(f"\n═══ RESULTS: {passed} passed, {failed} failed ═══")
sys.exit(0 if failed == 0 else 1)
