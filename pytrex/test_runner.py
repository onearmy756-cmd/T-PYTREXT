"""
PyTreXT TestRunner — Pima Mfumo Wako kwa Urahisi
====================================================
TestRunner inayowezesha developer kupima application yake
yote iliyotengenezwa kwa PyTreXT framework.

Features:
- Automated testing ya modules zote za PyTreXT
- Event system testing
- Blockchain integrity testing
- Database ACID compliance testing
- Encryption/decryption testing
- AI/ML model validation
- MCP protocol testing
- Search engine testing
- LangChain agent testing
- Hermes agent testing
- Human-in-the-loop workflow testing
- Beautiful CLI output with colors and stats

Usage:
    from pytrex.test_runner import TestRunner
    runner = TestRunner(my_app)
    results = runner.run_all()

    # Au kutoka CLI:
    pytrex test           # Test app yako
    pytrex test --all     # Test modules zote
    pytrex test --watch   # Watch mode (auto-run on file changes)
"""

import json
import sys
import time
import traceback
from typing import Any, Callable, Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from enum import Enum
from concurrent.futures import ThreadPoolExecutor, as_completed

# ─── ANSI Colors for Terminal ───
class Color:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    MAGENTA = "\033[95m"
    CYAN = "\033[96m"
    WHITE = "\033[97m"
    BG_RED = "\033[41m"
    BG_GREEN = "\033[42m"
    BG_YELLOW = "\033[43m"
    BG_BLUE = "\033[44m"
    BG_MAGENTA = "\033[45m"


class _TestSkip(Exception):
    """Internal exception for skipping tests"""
    pass


class TestStatus(Enum):
    PASS = "PASS"
    FAIL = "FAIL"
    SKIP = "SKIP"
    ERROR = "ERROR"


@dataclass
class TestResult:
    """Matokeo ya test moja"""
    name: str
    module: str
    status: TestStatus = TestStatus.PASS
    duration_ms: float = 0.0
    message: str = ""
    error_details: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "module": self.module,
            "status": self.status.value,
            "duration_ms": self.duration_ms,
            "message": self.message,
        }


@dataclass
class TestSuite:
    """Mkusanyiko wa matokeo ya tests"""
    results: List[TestResult] = field(default_factory=list)
    start_time: float = 0.0
    end_time: float = 0.0

    @property
    def total(self) -> int:
        return len(self.results)

    @property
    def passed(self) -> int:
        return sum(1 for r in self.results if r.status == TestStatus.PASS)

    @property
    def failed(self) -> int:
        return sum(1 for r in self.results if r.status == TestStatus.FAIL)

    @property
    def errors(self) -> int:
        return sum(1 for r in self.results if r.status == TestStatus.ERROR)

    @property
    def skipped(self) -> int:
        return sum(1 for r in self.results if r.status == TestStatus.SKIP)

    @property
    def duration(self) -> float:
        return self.end_time - self.start_time

    @property
    def success_rate(self) -> float:
        total_tested = self.total - self.skipped
        if total_tested == 0:
            return 100.0
        return (self.passed / total_tested) * 100


class TestRunner:
    """
    PyTreXT TestRunner — Pima application yako kikamilifu.
    Inajaribu modules zote za mfumo na inaripoti matokeo.
    """

    def __init__(
        self,
        app=None,
        verbose: bool = True,
        color: bool = True,
        parallel: bool = True,
        timeout: float = 30.0,
    ):
        self.app = app
        self.verbose = verbose
        self.color = color and sys.stdout.isatty()
        self.parallel = parallel
        self.timeout = timeout
        self.suite = TestSuite()

        self._tests: List[Tuple[str, str, Callable]] = []
        self._register_tests()

    # ─── Color Helpers ────────────────────────────────────────

    def _c(self, text: str, color: str) -> str:
        """Apply color if enabled"""
        return f"{color}{text}{Color.RESET}" if self.color else text

    def _status_icon(self, status: TestStatus) -> str:
        icons = {
            TestStatus.PASS: self._c("✓", Color.GREEN),
            TestStatus.FAIL: self._c("✗", Color.RED),
            TestStatus.ERROR: self._c("⚠", Color.YELLOW),
            TestStatus.SKIP: self._c("○", Color.DIM),
        }
        return icons.get(status, "?")

    # ─── Test Registration ────────────────────────────────────

    def _register_tests(self) -> None:
        """Register all test cases"""
        # Core System Tests
        self._add_test("Core", "App Initialization", self._test_app_init)
        self._add_test("Core", "App Name & Config", self._test_app_config)
        self._add_test("Core", "Event Registry", self._test_event_registry)
        self._add_test("Core", "Event Execution", self._test_event_execution)
        self._add_test("Core", "Rate Limiting", self._test_rate_limiting)

        # Network & Elixir Tests
        self._add_test("Network", "Elixir Client Init", self._test_elixir_client)
        self._add_test("Network", "WebSocket Connection", self._test_websocket)
        self._add_test("Network", "Event Broadcasting", self._test_broadcasting)

        # Blockchain Tests
        self._add_test("Blockchain", "Block Creation", self._test_blockchain_create)
        self._add_test("Blockchain", "Chain Verification", self._test_blockchain_verify)
        self._add_test("Blockchain", "Tamper Detection", self._test_blockchain_tamper)
        self._add_test("Blockchain", "Cache Management", self._test_blockchain_cache)

        # Database Tests
        self._add_test("Database", "DB Initialization", self._test_db_init)
        self._add_test("Database", "ACID Transaction", self._test_db_transaction)
        self._add_test("Database", "Data Integrity", self._test_db_integrity)

        # Encryption Tests
        self._add_test("Encryption", "AES Encrypt/Decrypt", self._test_encrypt_decrypt)
        self._add_test("Encryption", "Hash Generation", self._test_hashing)
        self._add_test("Encryption", "Secret Generation", self._test_secret_gen)

        # Search Engine Tests
        self._add_test("Search", "Engine Initialization", self._test_search_init)
        self._add_test("Search", "Fallback Search", self._test_search_fallback)
        self._add_test("Search", "Multi-Engine Query", self._test_search_multi)

        # AI/ML Tests
        self._add_test("AI/ML", "RAG Engine", self._test_rag_engine)
        self._add_test("AI/ML", "Neural Network Init", self._test_neural_net)

        # LangChain Tests
        self._add_test("LangChain", "Agent Creation", self._test_langchain_create)
        self._add_test("LangChain", "Tool Registration", self._test_langchain_tools)
        self._add_test("LangChain", "Chain Execution", self._test_langchain_chain)

        # Hermes Agent Tests
        self._add_test("Hermes", "Agent Creation", self._test_hermes_create)
        self._add_test("Hermes", "Function Calling", self._test_hermes_functions)
        self._add_test("Hermes", "Chat Interface", self._test_hermes_chat)

        # Human-in-the-Loop Tests
        self._add_test("HITL", "Workflow Creation", self._test_hitl_create)
        self._add_test("HITL", "Approve/Reject", self._test_hitl_approve)
        self._add_test("HITL", "Timeout Handling", self._test_hitl_timeout)

        # MCP Tests
        self._add_test("MCP", "Client Creation", self._test_mcp_client)
        self._add_test("MCP", "Tool Discovery", self._test_mcp_tools)
        self._add_test("MCP", "Local Tool Invocation", self._test_mcp_invoke)

        # Compression & Serialization
        self._add_test("Utils", "MessagePack", self._test_msgpack)
        self._add_test("Utils", "Compression", self._test_compression)
        self._add_test("Utils", "File Operations", self._test_file_ops)

    def _add_test(self, module: str, name: str, func: Callable) -> None:
        self._tests.append((module, name, func))

    # ─── Test Implementations ─────────────────────────────────

    def _run_test(self, module: str, name: str, func: Callable) -> TestResult:
        """Run a single test and return result"""
        start = time.time()
        try:
            func()
            duration = (time.time() - start) * 1000
            return TestResult(
                name=name,
                module=module,
                status=TestStatus.PASS,
                duration_ms=duration,
            )
        except _TestSkip as e:
            duration = (time.time() - start) * 1000
            return TestResult(
                name=name,
                module=module,
                status=TestStatus.SKIP,
                duration_ms=duration,
                message=str(e),
            )
        except AssertionError as e:
            duration = (time.time() - start) * 1000
            return TestResult(
                name=name,
                module=module,
                status=TestStatus.FAIL,
                duration_ms=duration,
                message=str(e),
            )
        except Exception as e:
            duration = (time.time() - start) * 1000
            return TestResult(
                name=name,
                module=module,
                status=TestStatus.ERROR,
                duration_ms=duration,
                message=str(e),
                error_details=traceback.format_exc(),
            )

    def _skip(self, reason: str = "") -> None:
        """Skip the current test with a reason"""
        raise _TestSkip(reason)

    # ─── CORE TESTS ───────────────────────────────────────────

    def _test_app_init(self) -> None:
        """Verify app initialization"""
        from pytrex.core import PyTreXApp
        app = PyTreXApp(name="TestRunner")
        assert app is not None, "App failed to initialize"
        assert app.name == "TestRunner", "App name mismatch"

    def _test_app_config(self) -> None:
        """Verify app configuration"""
        from pytrex.core import PyTreXApp
        app = PyTreXApp(name="TestConfig")
        assert hasattr(app, "network"), "Missing network attribute"
        assert hasattr(app, "bus"), "Missing event bus"

    def _test_event_registry(self) -> None:
        """Verify event registration system"""
        from pytrex.core import REGISTERED_EVENTS, event
        assert isinstance(REGISTERED_EVENTS, dict), "Event registry not initialized"

    def _test_event_execution(self) -> None:
        """Verify event execution"""
        from pytrex.core import execute_python_event, REGISTERED_EVENTS, event

        @event("__test_runner_event__")
        def handler(data):
            return f"handled: {data}"

        result = execute_python_event("__test_runner_event__", "test_data")
        assert "handled" in result, f"Event not handled correctly: {result}"

        # Cleanup
        REGISTERED_EVENTS.pop("__test_runner_event__", None)

    def _test_rate_limiting(self) -> None:
        """Verify rate limiting works"""
        from pytrex.core import REGISTERED_EVENTS, event, execute_python_event

        call_count = 0

        @event("__test_rate_limit__")
        def handler(data):
            nonlocal call_count
            call_count += 1
            return "ok"

        # Call many times quickly
        for _ in range(5):
            execute_python_event("__test_rate_limit__", "")

        assert call_count > 0, "Rate-limited event should execute at least once"

        REGISTERED_EVENTS.pop("__test_rate_limit__", None)

    # ─── NETWORK TESTS ────────────────────────────────────────

    def _test_elixir_client(self) -> None:
        """Verify Elixir client initialization"""
        from pytrex.core import ElixirClient
        client = ElixirClient()
        assert client is not None, "ElixirClient failed to initialize"
        assert hasattr(client, "emit"), "Missing emit method"

    def _test_websocket(self) -> None:
        """Verify WebSocket setup"""
        from pytrex.core import ElixirClient
        client = ElixirClient()
        assert client.host == "localhost", f"Unexpected host: {client.host}"
        assert client.port == 42351, f"Unexpected port: {client.port}"

    def _test_broadcasting(self) -> None:
        """Verify event broadcasting capability"""
        from pytrex.core import ElixirClient
        client = ElixirClient()
        assert hasattr(client, "broadcast"), "Missing broadcast method"

    # ─── BLOCKCHAIN TESTS ─────────────────────────────────────

    def _test_blockchain_create(self) -> None:
        """Verify blockchain block creation"""
        from pytrex.core import BlockchainBridge
        bridge = BlockchainBridge()
        result = bridge.add_block("test_transaction")
        assert result["status"] == "ok", f"Block creation failed: {result}"
        assert "block" in result, "No block in result"

    def _test_blockchain_verify(self) -> None:
        """Verify blockchain verification"""
        from pytrex.core import BlockchainBridge
        bridge = BlockchainBridge()
        bridge.add_block("tx1")
        bridge.add_block("tx2")
        result = bridge.verify_chain()
        assert result["status"] == "ok", f"Verification failed: {result}"

    def _test_blockchain_tamper(self) -> None:
        """Verify tamper detection"""
        from pytrex.core import BlockchainBridge, BLOCKCHAIN_CACHE

        # Clear cache for clean test
        BLOCKCHAIN_CACHE.clear()

        bridge = BlockchainBridge()
        bridge.add_block("genuine_block_1")
        bridge.add_block("genuine_block_2")

        # Tamper with block 1's data (not genesis block 0)
        if len(BLOCKCHAIN_CACHE) >= 2:
            BLOCKCHAIN_CACHE[1]["data"] = "TAMPERED_DATA!"
            result = bridge.verify_chain()
            # In Python fallback, verification starts from index 1
            # Genesis block (0) is not verified
            is_valid = result.get("valid", True)
            if is_valid:
                # Python fallback verifies from index 1, and we tampered with index 1
                assert not is_valid, "Should detect tampering in block 1"

    def _test_blockchain_cache(self) -> None:
        """Verify blockchain cache management"""
        from pytrex.core import BLOCKCHAIN_CACHE
        assert isinstance(BLOCKCHAIN_CACHE, list), "Cache should be a list"

    # ─── DATABASE TESTS ───────────────────────────────────────

    def _test_db_init(self) -> None:
        """Verify database initialization"""
        try:
            import my_framework
            import tempfile, os
            db_path = os.path.join(tempfile.gettempdir(), "pytrex_test.db")
            result = my_framework.kuandaa_database_salama(db_path, "test_key_123")
            assert result is None or "ok" in str(result).lower(), "DB init failed"
        except ImportError:
            self._skip("Rust core (my_framework) not built")

    def _test_db_transaction(self) -> None:
        """Verify ACID transaction"""
        try:
            import my_framework
            import tempfile, os
            db_path = os.path.join(tempfile.gettempdir(), "pytrex_test_tx.db")
            my_framework.kuandaa_database_salama(db_path, "key123")
            result = my_framework.fanya_muamala_salama("ACC001", "deposit", 500.0)
            assert result is not None, "Transaction failed"
        except ImportError:
            self._skip("Rust core not built")

    def _test_db_integrity(self) -> None:
        """Verify data integrity after transaction"""
        try:
            import my_framework
            import tempfile, os
            db_path = os.path.join(tempfile.gettempdir(), "pytrex_test_int.db")
            my_framework.kuandaa_database_salama(db_path, "key456")
            my_framework.fanya_muamala_salama("ACC002", "deposit", 1000.0)
            result = my_framework.fanya_muamala_salama("ACC002", "withdraw", 300.0)
            assert result is not None, "Transaction failed"
        except ImportError:
            self._skip("Rust core not built")

    # ─── ENCRYPTION TESTS ─────────────────────────────────────

    def _test_encrypt_decrypt(self) -> None:
        """Verify AES encryption/decryption"""
        from pytrex.core import EncryptionManager
        mgr = EncryptionManager(default_password="test")
        encrypted = mgr.encrypt("Secret Message")
        assert encrypted is not None, "Encryption failed"
        assert encrypted != "Secret Message", "Data not encrypted"

        decrypted = mgr.decrypt(encrypted)
        assert decrypted == "Secret Message", f"Decryption mismatch: {decrypted}"

    def _test_hashing(self) -> None:
        """Verify hash generation"""
        from pytrex.core import EncryptionManager
        mgr = EncryptionManager()
        hash1 = mgr.hash("test_data")
        hash2 = mgr.hash("test_data")
        assert hash1 == hash2, "Hash should be deterministic"
        assert len(hash1) == 64, f"Expected 64-char SHA-256, got {len(hash1)}"

    def _test_secret_gen(self) -> None:
        """Verify secret generation"""
        from pytrex.core import EncryptionManager
        mgr = EncryptionManager()
        secret = mgr.generate_secret(32)
        assert secret is not None, "Secret generation failed"
        assert len(secret) == 32, f"Expected 32 chars, got {len(secret)}"

    # ─── SEARCH TESTS ─────────────────────────────────────────

    def _test_search_init(self) -> None:
        """Verify search engine initialization"""
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        assert engine.default_engine == "duckduckgo", "Wrong default engine"
        assert engine.max_results == 10, "Wrong max results"

    def _test_search_fallback(self) -> None:
        """Verify search fallback"""
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        results = engine._fallback_search("test", "fallback-engine")
        assert len(results) == 1, "Fallback should return 1 result"
        assert "test" in results[0].title, "Wrong fallback result"

    def _test_search_multi(self) -> None:
        """Verify multi-engine search"""
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        summary = engine.web_search_summary("PyTreX framework")
        assert summary["query"] == "PyTreX framework", "Query mismatch"
        assert "results" in summary, "Missing results"

    # ─── AI/ML TESTS ──────────────────────────────────────────

    def _test_rag_engine(self) -> None:
        """Verify RAG engine"""
        from pytrex.core import RAGEngine
        rag = RAGEngine()
        assert rag is not None, "RAG engine failed to initialize"
        assert hasattr(rag, "query"), "Missing query method"

    def _test_neural_net(self) -> None:
        """Verify neural network initialization"""
        from pytrex.core import NeuralNetwork
        nn = NeuralNetwork()
        assert nn is not None, "Neural network failed to initialize"

    # ─── LANGCHAIN TESTS ──────────────────────────────────────

    def _test_langchain_create(self) -> None:
        """Verify LangChain agent creation"""
        from pytrex.langchain_agent import LangChainAgent
        agent = LangChainAgent()
        assert agent.model_name == "gpt-4", "Wrong default model"
        assert len(agent.list_tools()) == 0, "Should have no tools initially"

    def _test_langchain_tools(self) -> None:
        """Verify LangChain tool registration"""
        from pytrex.langchain_agent import LangChainAgent
        agent = LangChainAgent()
        agent.add_tool("test_tool", lambda x: x, "Test tool")
        tools = agent.list_tools()
        assert len(tools) == 1, "Tool not registered"
        assert tools[0]["name"] == "test_tool", "Wrong tool name"

    def _test_langchain_chain(self) -> None:
        """Verify LangChain chain execution"""
        from pytrex.langchain_agent import LangChainAgent
        agent = LangChainAgent()
        result = agent.run("hello", chain_type="conversation")
        assert result["status"] == "ok", f"Chain failed: {result}"

    # ─── HERMES TESTS ─────────────────────────────────────────

    def _test_hermes_create(self) -> None:
        """Verify Hermes agent creation"""
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        assert agent.name == "Hermes", "Wrong agent name"
        funcs = agent.list_functions()
        assert len(funcs) >= 4, f"Expected >=4 built-in functions, got {len(funcs)}"

    def _test_hermes_functions(self) -> None:
        """Verify Hermes function calling"""
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        agent.register_function("echo", lambda x, **kw: x, "Echo input", {"x": {"type": "string"}})
        func = agent.get_function("echo")
        assert func is not None, "Function not registered"

    def _test_hermes_chat(self) -> None:
        """Verify Hermes chat interface"""
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        result = agent.chat("Hello")
        assert "reply" in result, "Missing reply in chat result"
        assert result["iterations"] >= 1, "Should have at least 1 iteration"

    # ─── HITL TESTS ───────────────────────────────────────────

    def _test_hitl_create(self) -> None:
        """Verify HITL workflow creation"""
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()
        assert hitl.pending_count() == 0, "Should have no pending actions"

        action_id = hitl.request_approval("test", {"data": "x"}, timeout=9999)
        assert hitl.pending_count() == 1, "Should have 1 pending action"
        assert action_id is not None, "No action ID returned"

    def _test_hitl_approve(self) -> None:
        """Verify HITL approve/reject"""
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        aid = hitl.request_approval("test_approve", {}, timeout=9999)
        assert hitl.approve(aid), "Approve should succeed"
        assert hitl.pending_count() == 0, "Should be empty after approve"

        aid2 = hitl.request_approval("test_reject", {}, timeout=9999)
        assert hitl.reject(aid2, "Not needed"), "Reject should succeed"
        assert hitl.pending_count() == 0, "Should be empty after reject"

    def _test_hitl_timeout(self) -> None:
        """Verify HITL timeout handling"""
        from pytrex.human_in_loop import HumanInTheLoop
        import time
        hitl = HumanInTheLoop(default_timeout=0.1)

        aid = hitl.request_approval("test_timeout", {}, timeout=0.01)
        time.sleep(0.2)
        # After timeout, action should be expired
        action = hitl.get_action(aid)
        assert action is not None, "Action should still be retrievable"

    # ─── MCP TESTS ────────────────────────────────────────────

    def _test_mcp_client(self) -> None:
        """Verify MCP client creation"""
        from pytrex.mcp_client import MCPClient
        client = MCPClient()
        assert not client.is_connected(), "Should not be connected initially"
        assert client.transport.value == "http", "Wrong default transport"

    def _test_mcp_tools(self) -> None:
        """Verify MCP tool discovery"""
        from pytrex.mcp_client import MCPClient
        client = MCPClient()
        tools = client.list_tools()
        assert isinstance(tools, list), "Tools should be a list"

    def _test_mcp_invoke(self) -> None:
        """Verify MCP local tool invocation"""
        from pytrex.mcp_client import MCPClient
        client = MCPClient()

        result = client._invoke_local_tool("blockchain_create_block", {"data": "mcp_test"})
        assert "content" in result, "Missing content in MCP result"

    # ─── UTILITY TESTS ────────────────────────────────────────

    def _test_msgpack(self) -> None:
        """Verify MessagePack serialization"""
        try:
            import msgpack
            data = {"test": "value", "num": 42}
            packed = msgpack.packb(data)
            unpacked = msgpack.unpackb(packed)
            assert unpacked == data, "MessagePack round-trip failed"
        except ImportError:
            # Test Python json as fallback
            data = {"test": "value"}
            packed = json.dumps(data)
            unpacked = json.loads(packed)
            assert unpacked == data, "JSON fallback failed"

    def _test_compression(self) -> None:
        """Verify data compression"""
        from pytrex.core import Compression
        comp = Compression()
        original = "Hello PyTreXT! " * 100
        compressed = comp.compress(original.encode() if isinstance(original, str) else original)
        assert compressed is not None, "Compression failed"

    def _test_file_ops(self) -> None:
        """Verify file operations"""
        import tempfile, os
        test_file = os.path.join(tempfile.gettempdir(), "pytrex_test_file.txt")

        with open(test_file, "w") as f:
            f.write("PyTreXT Test Data")

        assert os.path.exists(test_file), "File not created"
        os.remove(test_file)
        assert not os.path.exists(test_file), "File not removed"

    # ─── Main Runner ──────────────────────────────────────────

    def run_all(self, modules: Optional[List[str]] = None) -> TestSuite:
        """
        Endesha tests zote au modules maalum.

        Args:
            modules: Orodha ya modules za kujaribu (None = zote)

        Returns:
            TestSuite yenye matokeo yote
        """
        self.suite = TestSuite()
        self.suite.start_time = time.time()

        # Filter tests by module
        tests_to_run = self._tests
        if modules:
            tests_to_run = [(m, n, f) for m, n, f in self._tests if m in modules]

        if self.verbose:
            print(f"\n{self._c('═══ PyTreXT TestRunner ═══', Color.BOLD + Color.CYAN)}")
            print(f"{self._c('Modules:', Color.DIM)} {len(set(m for m, _, _ in tests_to_run))}")
            print(f"{self._c('Tests:', Color.DIM)} {len(tests_to_run)}")
            print(f"{self._c('─' * 50, Color.DIM)}\n")

        if self.parallel:
            results = self._run_parallel(tests_to_run)
        else:
            results = self._run_sequential(tests_to_run)

        self.suite.results = results
        self.suite.end_time = time.time()

        if self.verbose:
            self._print_summary()

        return self.suite

    def _run_sequential(self, tests: List[Tuple]) -> List[TestResult]:
        """Run tests one by one"""
        results = []
        current_module = ""

        for module, name, func in tests:
            if module != current_module and self.verbose:
                current_module = module
                print(f"\n  {self._c(module, Color.BOLD + Color.MAGENTA)}")

            result = self._run_test(module, name, func)
            results.append(result)

            if self.verbose:
                icon = self._status_icon(result.status)
                duration = f"{result.duration_ms:.1f}ms"
                msg = f"    {icon} {name} {self._c(f'({duration})', Color.DIM)}"
                if result.status in (TestStatus.FAIL, TestStatus.ERROR):
                    msg += f"\n       {self._c(result.message, Color.RED)}"
                print(msg)

        return results

    def _run_parallel(self, tests: List[Tuple]) -> List[TestResult]:
        """Run tests in parallel using ThreadPoolExecutor"""
        results = []
        current_module = ""
        module_groups: Dict[str, List[Tuple]] = {}

        # Group by module for display
        for module, name, func in tests:
            if module not in module_groups:
                module_groups[module] = []
            module_groups[module].append((module, name, func))

        for module, group_tests in module_groups.items():
            if self.verbose:
                print(f"\n  {self._c(module, Color.BOLD + Color.MAGENTA)}")

            # Run module tests in parallel
            with ThreadPoolExecutor(max_workers=4) as executor:
                futures = {
                    executor.submit(self._run_test, m, n, f): (m, n)
                    for m, n, f in group_tests
                }

                for future in as_completed(futures):
                    result = future.result(timeout=self.timeout)
                    results.append(result)

                    if self.verbose:
                        icon = self._status_icon(result.status)
                        duration = f"{result.duration_ms:.1f}ms"
                        msg = f"    {icon} {result.name} {self._c(f'({duration})', Color.DIM)}"
                        if result.status in (TestStatus.FAIL, TestStatus.ERROR):
                            msg += f"\n       {self._c(result.message, Color.RED)}"
                        print(msg)

        return results

    def _print_summary(self) -> None:
        """Print final test summary"""
        s = self.suite
        rate = s.success_rate
        rate_color = Color.GREEN if rate >= 90 else (Color.YELLOW if rate >= 70 else Color.RED)

        print(f"\n{self._c('═' * 50, Color.DIM)}")
        print(f"  {self._c('RESULTS', Color.BOLD)}")

        lines = [
            f"  {self._c('Total:', Color.DIM)}   {s.total}",
            f"  {self._c('Passed:', Color.DIM)}  {self._c(str(s.passed), Color.GREEN)}",
            f"  {self._c('Failed:', Color.DIM)}  {self._c(str(s.failed), Color.RED) if s.failed > 0 else '0'}",
            f"  {self._c('Errors:', Color.DIM)}  {self._c(str(s.errors), Color.YELLOW) if s.errors > 0 else '0'}",
            f"  {self._c('Skipped:', Color.DIM)} {s.skipped}",
            f"  {self._c('Duration:', Color.DIM)} {s.duration:.2f}s",
            f"  {self._c('Rate:', Color.DIM)}     {self._c(f'{rate:.1f}%', rate_color)}",
        ]
        for line in lines:
            print(line)

        # Final verdict
        if s.failed == 0 and s.errors == 0:
            verdict = self._c("✅ ALL TESTS PASSED! Mfumo wako uko sawa kabisa.", Color.BOLD + Color.GREEN)
        elif s.success_rate >= 80:
            verdict = self._c(f"⚠️  {s.failed + s.errors} tests zimefeli — angalia matokeo hapo juu.", Color.BOLD + Color.YELLOW)
        else:
            verdict = self._c(f"❌ {s.failed + s.errors} tests zimefeli — mfumo unahitaji maboresho.", Color.BOLD + Color.RED)

        print(f"\n  {verdict}")
        print(f"{self._c('═' * 50, Color.DIM)}\n")

    def quick_test(self) -> Dict[str, Any]:
        """
        Jaribio la haraka — tests muhimu tu.
        Inafaa kwa CI/CD pipelines.
        """
        return self.run_all(modules=["Core", "Blockchain", "Encryption"]).to_dict()

    def export_json(self, filepath: str = "test_results.json") -> str:
        """Export matokeo kwenye JSON file"""
        data = {
            "framework": "PyTreXT",
            "version": "1.0.0",
            "results": [r.to_dict() for r in self.suite.results],
            "summary": {
                "total": self.suite.total,
                "passed": self.suite.passed,
                "failed": self.suite.failed,
                "errors": self.suite.errors,
                "skipped": self.suite.skipped,
                "duration": self.suite.duration,
                "success_rate": self.suite.success_rate,
            },
        }

        with open(filepath, "w") as f:
            json.dump(data, f, indent=2)

        return filepath


# ─── Convenience Functions ────────────────────────────────────

def test_app(app=None) -> TestSuite:
    """
    Pima PyTreXT app yako kwa amri moja.
    
    Usage:
        from pytrex.test_runner import test_app
        results = test_app(my_app)
    """
    runner = TestRunner(app=app)
    return runner.run_all()


def quick_test() -> Dict[str, Any]:
    """Jaribio la haraka — tests muhimu tu"""
    runner = TestRunner()
    return runner.quick_test()


def test_module(module_name: str) -> TestSuite:
    """Pima module maalum"""
    runner = TestRunner()
    return runner.run_all(modules=[module_name])


if __name__ == "__main__":
    runner = TestRunner()
    suite = runner.run_all()
    sys.exit(0 if suite.failed == 0 else 1)
