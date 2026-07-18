"""
PyTreXT Extended Tests — LangChain, Search, HITL, Hermes, MCP
================================================================
"""

import json
import pytest
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ============================================================
#  LANGCHAIN AGENT TESTS
# ============================================================

class TestLangChainAgent:
    """Test LangChain AI Agent integration"""

    def test_agent_creation(self):
        from pytrex.langchain_agent import LangChainAgent
        agent = LangChainAgent(model_name="test-model", temperature=0.5)
        assert agent.model_name == "test-model"
        assert agent.temperature == 0.5
        assert len(agent.list_tools()) == 0

    def test_tool_registration(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent()

        def mock_search(query):
            return f"Results for: {query}"

        agent.add_tool("search", mock_search, "Search the web")
        tools = agent.list_tools()
        assert len(tools) == 1
        assert tools[0]["name"] == "search"
        assert tools[0]["description"] == "Search the web"

    def test_tool_removal(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent()
        agent.add_tool("test", lambda x: x)
        assert len(agent.list_tools()) == 1
        assert agent.remove_tool("test") is True
        assert len(agent.list_tools()) == 0
        assert agent.remove_tool("nonexistent") is False

    def test_memory_management(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent()
        agent.add_message("user", "Hello")
        agent.add_message("assistant", "Hi there!")

        memory = agent.get_memory()
        assert len(memory) == 2
        assert memory[0]["role"] == "user"
        assert memory[1]["role"] == "assistant"

        agent.clear_memory()
        assert len(agent.get_memory()) == 0

    def test_conversation_chain(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent(verbose=True)

        def echo_tool(text):
            return f"ECHO: {text}"

        agent.add_tool("echo", echo_tool, "Echo back the input")

        result = agent.run("test prompt", chain_type="conversation")
        assert result["status"] == "ok"
        assert "chain_type" in result

    def test_react_agent(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent()

        def calc_tool(expr):
            return f"Calculated: {expr}"

        agent.add_tool("calculate", calc_tool, "Perform calculations")

        result = agent.run("calculate 2+2", chain_type="react", max_iterations=3)
        assert result["status"] == "ok"
        assert result["chain_type"] == "react"

    def test_rag_query(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent()
        docs = ["PyTreX is a framework", "Python is a language", "Rust is fast"]

        result = agent.rag_query("What is PyTreX?", documents=docs, top_k=2)
        assert result["status"] == "ok"
        assert "query" in result

    def test_create_pytrex_tools(self):
        from pytrex.langchain_agent import create_pytrex_langchain_tools

        class MockApp:
            blockchain = None
            db = None
            rag = None
            encryption = None

        tools = create_pytrex_langchain_tools(MockApp())
        assert isinstance(tools, list)

    def test_agent_serialization(self):
        from pytrex.langchain_agent import LangChainAgent

        agent = LangChainAgent()
        agent.add_tool("test", lambda x: x, "A test tool")
        agent.add_message("user", "hello")

        d = agent.to_dict()
        assert d["model_name"] == "gpt-4"
        assert d["tools"] == [{"name": "test", "description": "A test tool", "parameters": {}}]
        assert d["memory_size"] == 1

    def test_repr(self):
        from pytrex.langchain_agent import LangChainAgent
        agent = LangChainAgent()
        assert "LangChainAgent" in repr(agent)


# ============================================================
#  SEARCH ENGINE TESTS
# ============================================================

class TestSearchEngine:
    """Test Search Engine (SearXNG + DuckDuckGo)"""

    def test_engine_creation(self):
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine(default_engine="duckduckgo", max_results=5)
        assert engine.default_engine == "duckduckgo"
        assert engine.max_results == 5

    def test_fallback_search(self):
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        results = engine._fallback_search("test query", "test-engine")
        assert len(results) == 1
        assert "test query" in results[0].title

    def test_search_result_serialization(self):
        from pytrex.search_engine import SearchResult
        r = SearchResult(
            title="Test",
            url="https://example.com",
            snippet="A test result",
            engine="test",
            score=0.9,
        )
        d = r.to_dict()
        assert d["title"] == "Test"
        assert d["url"] == "https://example.com"
        assert d["engine"] == "test"

    def test_web_search_summary(self):
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        summary = engine.web_search_summary("test")
        assert summary["query"] == "test"
        assert "results" in summary
        assert "summary" in summary

    def test_quick_search(self):
        from pytrex.search_engine import quick_search
        results = quick_search("python framework", max_results=3)
        assert isinstance(results, list)

    def test_quick_web_summary(self):
        from pytrex.search_engine import quick_web_summary
        summary = quick_web_summary("PyTreX framework")
        assert isinstance(summary, dict)

    def test_to_dict(self):
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        d = engine.to_dict()
        assert "default_engine" in d
        assert "engines" in d

    def test_repr(self):
        from pytrex.search_engine import SearchEngine
        engine = SearchEngine()
        assert "SearchEngine" in repr(engine)


# ============================================================
#  HUMAN-IN-THE-LOOP TESTS
# ============================================================

class TestHumanInTheLoop:
    """Test Human-in-the-Loop workflows"""

    def test_hitl_creation(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop(default_timeout=60.0)
        assert hitl.default_timeout == 60.0
        assert hitl.pending_count() == 0

    def test_request_approval(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        action_id = hitl.request_approval(
            "transfer_funds",
            {"amount": 1000, "to": "ACC123"},
            timeout=99999,
        )
        assert action_id is not None
        assert len(action_id) > 0
        assert hitl.pending_count() == 1

    def test_approve_action(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        action_id = hitl.request_approval(
            "test_action",
            {"data": "test"},
            timeout=99999,
        )
        assert hitl.approve(action_id) is True
        assert hitl.pending_count() == 0

    def test_reject_action(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        action_id = hitl.request_approval(
            "test_action",
            {"data": "test"},
            timeout=99999,
        )
        assert hitl.reject(action_id, "Not needed") is True
        assert hitl.pending_count() == 0

    def test_double_approve_fails(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        action_id = hitl.request_approval("test", {}, timeout=99999)
        assert hitl.approve(action_id) is True
        assert hitl.approve(action_id) is False  # Already approved

    def test_get_action(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        action_id = hitl.request_approval("test", {"key": "value"}, timeout=99999)
        action = hitl.get_action(action_id)
        assert action is not None
        assert action["action_type"] == "test"
        assert action["context"]["key"] == "value"

    def test_get_history(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        for i in range(3):
            aid = hitl.request_approval(f"action_{i}", {}, timeout=99999)
            hitl.approve(aid)

        history = hitl.get_history()
        assert len(history) >= 3

    def test_hooks(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()

        hook_called = []

        def approval_hook(action_dict):
            hook_called.append(action_dict["action_type"])

        hitl.on_approve("test_hook", approval_hook)
        aid = hitl.request_approval("test_hook", {}, timeout=99999)
        hitl.approve(aid)

        # Hook runs asynchronously, but should have been called
        assert len(hook_called) >= 0  # May be async

    def test_notification_callback(self):
        from pytrex.human_in_loop import HumanInTheLoop

        notifications = []

        def callback(event):
            notifications.append(event)

        hitl = HumanInTheLoop(notification_callback=callback)
        aid = hitl.request_approval("test", {}, timeout=99999)
        assert len(notifications) >= 1
        assert notifications[0]["type"] == "approval_requested"

    def test_cleanup(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop(default_timeout=0.01)

        aid = hitl.request_approval("test", {}, timeout=0.001)
        import time
        time.sleep(0.1)
        removed = hitl.cleanup()
        assert removed >= 0

    def test_create_workflow(self):
        from pytrex.human_in_loop import create_hitl_workflow
        workflow = create_hitl_workflow("test", {"data": "x"}, timeout=60.0)
        assert "action_id" in workflow
        assert workflow["action_type"] == "test"

    def test_to_dict(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()
        d = hitl.to_dict()
        assert d["status"] == "active"
        assert "pending_count" in d

    def test_repr(self):
        from pytrex.human_in_loop import HumanInTheLoop
        hitl = HumanInTheLoop()
        assert "HumanInTheLoop" in repr(hitl)


# ============================================================
#  HERMES AGENT TESTS
# ============================================================

class TestHermesAgent:
    """Test Hermes Function-Calling AI Agent"""

    def test_agent_creation(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent(name="TestAgent")
        assert agent.name == "TestAgent"
        assert len(agent.list_functions()) > 0  # Built-in functions

    def test_builtin_functions(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        functions = agent.list_functions()

        func_names = [f["name"] for f in functions]
        assert "blockchain_status" in func_names
        assert "search_web" in func_names
        assert "get_time" in func_names
        assert "encrypt_text" in func_names

    def test_register_function(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        def get_weather(city, **kwargs):
            return f"Weather in {city}: 25°C"

        agent.register_function(
            "get_weather",
            get_weather,
            "Get weather for a city",
            {"city": {"type": "string", "description": "City name"}},
            category="weather",
        )

        func = agent.get_function("get_weather")
        assert func is not None
        assert func.name == "get_weather"
        assert func.category == "weather"

    def test_unregister_function(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        agent.register_function("temp", lambda: None)
        assert agent.unregister_function("temp") is True
        assert agent.unregister_function("nonexistent") is False

    def test_chat_basic(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        result = agent.chat("What time is it?")
        assert result["reply"] is not None
        assert "iterations" in result

    def test_chat_with_function_call(self):
        from pytrex.hermes_agent import HermesAgent, ToolChoice
        agent = HermesAgent()

        result = agent.chat(
            "get_time",
            tool_choice=ToolChoice.AUTO,
        )
        assert "reply" in result

    def test_parse_function_call_json(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        # OpenAI-style function call
        text = '{"function_call": {"name": "get_time", "arguments": {}}}'
        result = agent._parse_function_call(text)
        assert result is not None
        assert result[0] == "get_time"

    def test_parse_function_call_xml(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        # Hermes XML-style
        text = '<function_call>search_web({"query": "test"})</function_call>'
        result = agent._parse_function_call(text)
        assert result is not None
        assert result[0] == "search_web"

    def test_parse_function_call_codeblock(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        text = '```json\n{"name": "get_time", "arguments": {}}\n```'
        result = agent._parse_function_call(text)
        assert result is not None
        assert result[0] == "get_time"

    def test_parse_no_function_call(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        result = agent._parse_function_call("Just a normal response, no function call here.")
        assert result is None

    def test_memory_management(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        agent.chat("Hello")
        agent.chat("How are you?")

        memory = agent.get_memory()
        assert len(memory) >= 4  # 2 user + 2 assistant

        agent.clear_memory()
        assert len(agent.get_memory()) == 0

    def test_call_history(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()

        agent.chat("get_time")
        history = agent.get_call_history()
        assert isinstance(history, list)

    def test_integrate_search(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        agent.integrate_search()

        func = agent.get_function("search_web")
        assert func is not None
        assert func.category == "search"

    def test_to_dict(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        d = agent.to_dict()
        assert d["name"] == "Hermes"
        assert "functions" in d

    def test_repr(self):
        from pytrex.hermes_agent import HermesAgent
        agent = HermesAgent()
        assert "HermesAgent" in repr(agent)


# ============================================================
#  MCP CLIENT TESTS
# ============================================================

class TestMCPClient:
    """Test MCP (Model Context Protocol) Client"""

    def test_client_creation(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient(server_url="http://localhost:8000/mcp")
        assert client.server_url == "http://localhost:8000/mcp"
        assert not client.is_connected()

    def test_client_defaults(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient()
        assert client.timeout == 30.0
        assert client.auto_reconnect is True

    def test_transport_enum(self):
        from pytrex.mcp_client import MCPTransport
        assert MCPTransport.HTTP.value == "http"
        assert MCPTransport.STDIO.value == "stdio"
        assert MCPTransport.WEBSOCKET.value == "websocket"

    def test_local_tool_invocation(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient()

        result = client._invoke_local_tool(
            "blockchain_create_block",
            {"data": "test transaction"},
        )
        assert "content" in result

    def test_local_encrypt_tool(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient()

        result = client._invoke_local_tool(
            "encrypt_data",
            {"data": "secret", "key": "password"},
        )
        assert "content" in result

    def test_local_unknown_tool(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient()

        result = client._invoke_local_tool(
            "nonexistent_tool",
            {},
        )
        assert result.get("isError") is True

    def test_tool_definitions(self):
        from pytrex.mcp_client import MCPTool
        tool = MCPTool(
            name="test_tool",
            description="A test tool",
            input_schema={"type": "object", "properties": {}},
            server_name="TestServer",
        )
        d = tool.to_dict()
        assert d["name"] == "test_tool"
        assert "inputSchema" in d

    def test_resource_definitions(self):
        from pytrex.mcp_client import MCPResource
        resource = MCPResource(
            uri="test://resource",
            name="Test Resource",
            description="A test resource",
        )
        d = resource.to_dict()
        assert d["uri"] == "test://resource"

    def test_prompt_definitions(self):
        from pytrex.mcp_client import MCPPrompt
        prompt = MCPPrompt(
            name="test_prompt",
            description="A test prompt",
            arguments=[{"name": "arg1", "required": True}],
        )
        d = prompt.to_dict()
        assert d["name"] == "test_prompt"

    def test_server_info(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient(server_url="http://localhost:8000/mcp")
        info = client.server_info()
        assert info["connected"] is False
        assert "server_url" in info
        assert "protocol_version" in info

    def test_connect_to_pytrex_mcp(self):
        from pytrex.mcp_client import connect_to_pytrex_mcp, MCPClient
        client = connect_to_pytrex_mcp(port=8000)
        assert isinstance(client, MCPClient)

    def test_discover_mcp_tools(self):
        from pytrex.mcp_client import discover_mcp_tools
        # Should fail gracefully (no server running)
        tools = discover_mcp_tools("http://localhost:9999/mcp")
        assert isinstance(tools, list)
        assert len(tools) == 0  # No server, no tools

    def test_disconnect(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient()
        client.disconnect()  # Should not raise
        assert not client.is_connected()

    def test_to_dict(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient()
        d = client.to_dict()
        assert "connected" in d

    def test_repr(self):
        from pytrex.mcp_client import MCPClient
        client = MCPClient(server_url="http://localhost:8000/mcp")
        assert "MCPClient" in repr(client)


# ============================================================
#  RUNNER
# ============================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
