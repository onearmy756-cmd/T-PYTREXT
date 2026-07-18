"""
PyTreXT Hermes Agent — Function-Calling AI Agent
===================================================
Based on Nous Research's Hermes model patterns.
Inawezesha AI agents kuita functions kwa kutumia:
- Function call registry
- Tool orchestration
- Conversational memory
- Integration with MCP tools, Search, and LangChain

Usage:
    from pytrex.hermes_agent import HermesAgent
    agent = HermesAgent()
    agent.register_function("get_weather", get_weather_func)
    response = agent.chat("What's the weather in Dar es Salaam?")
"""

import json
import logging
import re
from typing import Any, Callable, Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from enum import Enum

logger = logging.getLogger("pytrex.hermes")


class ToolChoice(Enum):
    """Chaguo la jinsi agent atakavyotumia tools"""
    AUTO = "auto"       # Agent aamue mwenyewe
    REQUIRED = "required"  # Lazima atumie tool
    NONE = "none"       # Asitumie tool


@dataclass
class FunctionDefinition:
    """Definition ya function inayoweza kuitwa na agent"""
    name: str
    description: str
    parameters: Dict[str, Any]
    func: Callable
    category: str = "general"

    def to_openai_schema(self) -> Dict[str, Any]:
        """Convert to OpenAI function-calling schema"""
        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": self.parameters,
                    "required": list(self.parameters.keys()) if self.parameters else [],
                },
            },
        }


@dataclass
class FunctionCall:
    """Result of a function call"""
    name: str
    arguments: Dict[str, Any]
    result: Any
    success: bool
    error: Optional[str] = None
    duration_ms: float = 0.0


class HermesAgent:
    """
    PyTreXT Hermes Agent — AI agent mwenye uwezo wa kuita functions.
    Imefuata muundo wa Nous Research Hermes model.

    Features:
    - Function calling (tool use) na structured output
    - Memory ya mazungumzo (conversational context)
    - Tool orchestration (auto-select tools based on prompt)
    - Integration na PyTreX MCP, Search, na LangChain
    """

    SYSTEM_PROMPT = """You are Hermes, an AI assistant integrated into the PyTreXT framework.
You have access to functions (tools) that you can call to help answer questions.

When you need to use a function, respond in this exact JSON format:
{{"function_call": {{"name": "function_name", "arguments": {{"arg1": "value1", ...}}}}}}

Available functions:
{function_list}

Current conversation:
{conversation}

Respond naturally. Only use function_call JSON when you need to use a tool."""

    def __init__(
        self,
        name: str = "Hermes",
        model: Optional[Any] = None,
        max_function_calls: int = 10,
        verbose: bool = False,
    ):
        self.name = name
        self.model = model
        self.max_function_calls = max_function_calls
        self.verbose = verbose

        self._functions: Dict[str, FunctionDefinition] = {}
        self._messages: List[Dict[str, Any]] = []
        self._tool_choice: ToolChoice = ToolChoice.AUTO
        self._call_history: List[FunctionCall] = []

        # Auto-register PyTreX built-in functions
        self._register_builtins()

        logger.info(f"HermesAgent '{name}' initialized with {len(self._functions)} functions")

    # ─── Function Registry ────────────────────────────────────

    def register_function(
        self,
        name: str,
        func: Callable,
        description: str = "",
        parameters: Optional[Dict[str, Any]] = None,
        category: str = "general",
    ) -> None:
        """
        Sajili function inayoweza kuitwa na Hermes agent.

        Args:
            name: Jina la function (unique)
            func: Function yenyewe
            description: Maelezo ya function (inatumika kwa AI kuelewa lini kuitumia)
            parameters: Schema ya parameters kwa OpenAI function-calling format
            category: Kategoria ya function
        """
        self._functions[name] = FunctionDefinition(
            name=name,
            description=description or func.__doc__ or f"Call {name}",
            parameters=parameters or {},
            func=func,
            category=category,
        )
        logger.info(f"Hermes function registered: {name} [{category}]")

    def unregister_function(self, name: str) -> bool:
        """Ondoa function kutoka registry"""
        if name in self._functions:
            del self._functions[name]
            logger.info(f"Hermes function removed: {name}")
            return True
        return False

    def list_functions(self) -> List[Dict[str, Any]]:
        """Orodhesha functions zote zilizosajiliwa"""
        return [
            {
                "name": f.name,
                "description": f.description,
                "category": f.category,
                "parameters": f.parameters,
            }
            for f in self._functions.values()
        ]

    def get_function(self, name: str) -> Optional[FunctionDefinition]:
        """Pata function kwa jina"""
        return self._functions.get(name)

    # ─── Built-in Functions ───────────────────────────────────

    def _register_builtins(self) -> None:
        """Sajili functions za msingi za PyTreXT"""

        self.register_function(
            name="blockchain_status",
            func=self._builtin_blockchain_status,
            description="Get current blockchain status: last block hash, block count, and chain integrity",
            parameters={},
            category="blockchain",
        )

        self.register_function(
            name="search_web",
            func=self._builtin_search_web,
            description="Search the web for information using DuckDuckGo or SearXNG",
            parameters={
                "query": {"type": "string", "description": "The search query"},
                "engine": {"type": "string", "enum": ["duckduckgo", "searxng"], "description": "Search engine to use"},
            },
            category="search",
        )

        self.register_function(
            name="get_time",
            func=self._builtin_get_time,
            description="Get the current date and time",
            parameters={},
            category="utility",
        )

        self.register_function(
            name="encrypt_text",
            func=self._builtin_encrypt,
            description="Encrypt text using AES-256 encryption",
            parameters={
                "text": {"type": "string", "description": "Text to encrypt"},
                "password": {"type": "string", "description": "Encryption password"},
            },
            category="security",
        )

        self.register_function(
            name="decrypt_text",
            func=self._builtin_decrypt,
            description="Decrypt AES-256 encrypted text",
            parameters={
                "encrypted_text": {"type": "string", "description": "Base64-encoded encrypted text"},
                "password": {"type": "string", "description": "Decryption password"},
            },
            category="security",
        )

    def _builtin_blockchain_status(self, **kwargs) -> str:
        """Get blockchain status"""
        try:
            import my_framework
            return my_framework.hakiki_blockchain("[]")
        except ImportError:
            return json.dumps({"status": "Python-only mode", "blocks": 0, "message": "Rust core not built"})

    def _builtin_search_web(self, query: str, engine: str = "duckduckgo", **kwargs) -> str:
        """Search the web"""
        try:
            from pytrex.search_engine import SearchEngine
            se = SearchEngine()
            results = se.search(query, engine=engine, max_results=3)
            return json.dumps([r.to_dict() for r in results])
        except Exception as e:
            return json.dumps({"error": str(e)})

    def _builtin_get_time(self, **kwargs) -> str:
        """Get current time"""
        import datetime
        now = datetime.datetime.now()
        return json.dumps({
            "datetime": now.isoformat(),
            "date": now.strftime("%Y-%m-%d"),
            "time": now.strftime("%H:%M:%S"),
            "timezone": str(datetime.timezone.utc),
        })

    def _builtin_encrypt(self, text: str, password: str, **kwargs) -> str:
        """Encrypt text"""
        try:
            import my_framework
            return my_framework.aes_encrypt(text, password)
        except ImportError:
            import base64
            # Simple fallback
            return base64.b64encode(text.encode()).decode()

    def _builtin_decrypt(self, encrypted_text: str, password: str, **kwargs) -> str:
        """Decrypt text"""
        try:
            import my_framework
            return my_framework.aes_decrypt(encrypted_text, password)
        except ImportError:
            import base64
            try:
                return base64.b64decode(encrypted_text).decode()
            except Exception:
                return json.dumps({"error": "Decryption failed"})

    # ─── Chat Interface ───────────────────────────────────────

    def chat(
        self,
        user_message: str,
        system_prompt: Optional[str] = None,
        tool_choice: ToolChoice = ToolChoice.AUTO,
    ) -> Dict[str, Any]:
        """
        Ongea na Hermes agent.

        Args:
            user_message: Ujumbe wa mtumiaji
            system_prompt: System prompt maalum
            tool_choice: Jinsi agent atakavyotumia tools

        Returns:
            Dict yenye reply na metadata
        """
        self._tool_choice = tool_choice
        self._messages.append({"role": "user", "content": user_message})

        if self.verbose:
            logger.info(f"Hermes chat: '{user_message[:100]}...'")

        # Build system prompt with available functions
        function_list = self._build_function_list()
        conversation = self._format_conversation()

        full_prompt = (system_prompt or self.SYSTEM_PROMPT).format(
            function_list=function_list,
            conversation=conversation,
        )

        # Main reasoning loop
        result = self._reasoning_loop(full_prompt, user_message)

        self._messages.append({"role": "assistant", "content": result.get("reply", "")})

        return result

    def _reasoning_loop(
        self, system_prompt: str, user_message: str
    ) -> Dict[str, Any]:
        """
        Hermes reasoning loop — inaruhusu function calling na multi-step reasoning.
        """
        function_calls = []
        current_response = user_message
        iterations = 0

        while iterations < self.max_function_calls:
            iterations += 1

            # Try to parse function calls from the response
            func_call = self._parse_function_call(current_response)

            if func_call is None:
                # No function call — return the natural response
                break

            func_name, func_args = func_call

            if func_name not in self._functions:
                if self.verbose:
                    logger.warning(f"Unknown function: {func_name}")
                break

            # Execute the function
            import time
            start = time.time()
            try:
                func_def = self._functions[func_name]
                result = func_def.func(**func_args)
                success = True
                error = None
            except Exception as e:
                result = f"Error: {e}"
                success = False
                error = str(e)
                logger.error(f"Function '{func_name}' failed: {e}")

            duration_ms = (time.time() - start) * 1000

            fc = FunctionCall(
                name=func_name,
                arguments=func_args,
                result=result,
                success=success,
                error=error,
                duration_ms=duration_ms,
            )
            function_calls.append(fc)
            self._call_history.append(fc)

            if self.verbose:
                status = "✓" if success else "✗"
                logger.info(f"  {status} {func_name}({json.dumps(func_args)[:50]}) → {str(result)[:100]}")

            # Use result as next context
            current_response = str(result)

            # If tool_choice is NONE, don't chain
            if self._tool_choice == ToolChoice.NONE:
                break

        return {
            "reply": current_response if not function_calls else self._format_final_response(function_calls),
            "function_calls": [
                {
                    "name": fc.name,
                    "arguments": fc.arguments,
                    "success": fc.success,
                    "error": fc.error,
                    "duration_ms": fc.duration_ms,
                }
                for fc in function_calls
            ],
            "iterations": iterations,
            "model_used": self.name,
        }

    # ─── Function Call Parsing ────────────────────────────────

    def _parse_function_call(self, text: str) -> Optional[Tuple[str, Dict[str, Any]]]:
        """
        Parse function call from text.
        Supports multiple formats:
        1. OpenAI JSON: {"function_call": {"name": "...", "arguments": {...}}}
        2. Hermes XML: <function_call>name(args)</function_call>
        3. Markdown code block with JSON
        """
        # Format 1: OpenAI-style JSON (match complete JSON object)
        try:
            # Find the first complete JSON object with function_call
            depth = 0
            start = -1
            for i, ch in enumerate(text):
                if ch == '{':
                    if depth == 0:
                        start = i
                    depth += 1
                elif ch == '}':
                    depth -= 1
                    if depth == 0 and start >= 0:
                        try:
                            data = json.loads(text[start:i+1])
                            if isinstance(data, dict):
                                fc = data.get("function_call", {})
                                if isinstance(fc, dict):
                                    name = fc.get("name", "")
                                    args = fc.get("arguments", {})
                                    if name:
                                        return (name, args)
                                # Direct format: {"name": "...", "arguments": {...}}
                                name = data.get("name", "")
                                if name and "arguments" in data:
                                    return (name, data.get("arguments", {}))
                        except (json.JSONDecodeError, ValueError):
                            pass
                        start = -1
        except Exception:
            pass

        # Format 2: Hermes XML-style
        match = re.search(r'<function_call>(.*?)</function_call>', text, re.DOTALL)
        if match:
            content = match.group(1).strip()
            # Parse name(args)
            m = re.match(r'(\w+)\((.*)\)', content, re.DOTALL)
            if m:
                name = m.group(1)
                args_str = m.group(2)
                try:
                    args = json.loads(args_str)
                except json.JSONDecodeError:
                    args = {"input": args_str}
                return (name, args)

        # Format 3: JSON code block
        match = re.search(r'```json\s*(.*?)\s*```', text, re.DOTALL)
        if match:
            try:
                data = json.loads(match.group(1))
                if "function_call" in data or "name" in data:
                    fc = data.get("function_call", data)
                    name = fc.get("name", "")
                    args = fc.get("arguments", {})
                    if name:
                        return (name, args)
            except (json.JSONDecodeError, AttributeError):
                pass

        return None

    # ─── Prompt Building ──────────────────────────────────────

    def _build_function_list(self) -> str:
        """Build the function list for the system prompt"""
        if not self._functions:
            return "No functions available."

        lines = []
        for func in self._functions.values():
            params_str = json.dumps(func.parameters) if func.parameters else "no parameters"
            lines.append(f"- {func.name}({params_str}): {func.description}")

        return "\n".join(lines)

    def _format_conversation(self) -> str:
        """Format conversation history"""
        if not self._messages:
            return "No conversation history."

        lines = []
        for msg in self._messages[-10:]:  # Last 10 messages
            role = msg.get("role", "unknown").capitalize()
            content = str(msg.get("content", ""))[:200]
            lines.append(f"{role}: {content}")

        return "\n".join(lines)

    def _format_final_response(self, function_calls: List[FunctionCall]) -> str:
        """Format final response with function call results"""
        parts = []
        for fc in function_calls:
            status = "✓" if fc.success else "✗"
            result_str = str(fc.result)[:200]
            parts.append(f"[{status} {fc.name}]: {result_str}")

        return "\n".join(parts)

    # ─── Memory Management ────────────────────────────────────

    def clear_memory(self) -> None:
        """Futa kumbukumbu yote ya mazungumzo"""
        self._messages.clear()
        self._call_history.clear()
        logger.info("Hermes memory cleared")

    def get_memory(self) -> List[Dict[str, Any]]:
        """Pata kumbukumbu ya mazungumzo"""
        return self._messages.copy()

    def get_call_history(self) -> List[Dict[str, Any]]:
        """Pata historia ya function calls"""
        return [
            {
                "name": fc.name,
                "arguments": fc.arguments,
                "success": fc.success,
                "error": fc.error,
                "duration_ms": fc.duration_ms,
            }
            for fc in self._call_history
        ]

    # ─── Integration ──────────────────────────────────────────

    def integrate_search(self, search_engine=None) -> None:
        """Integrate with PyTreX SearchEngine"""
        if search_engine is None:
            from pytrex.search_engine import SearchEngine
            search_engine = SearchEngine()

        def search_func(query: str, engine: str = "duckduckgo", **kwargs):
            results = search_engine.search(query, engine=engine)
            return json.dumps([r.to_dict() for r in results])

        self.register_function(
            name="search_web",
            func=search_func,
            description="Search the web using DuckDuckGo or SearXNG",
            parameters={
                "query": {"type": "string", "description": "Search query"},
                "engine": {"type": "string", "enum": ["duckduckgo", "searxng"]},
            },
            category="search",
        )

    def integrate_mcp(self, mcp_client=None) -> None:
        """Integrate with PyTreX MCP Client"""
        if mcp_client is None:
            from pytrex.mcp_client import MCPClient
            mcp_client = MCPClient()

        def mcp_func(tool_name: str, arguments: str = "{}", **kwargs):
            result = mcp_client.invoke_tool(tool_name, json.loads(arguments))
            return json.dumps(result)

        self.register_function(
            name="mcp_invoke",
            func=mcp_func,
            description="Invoke an MCP tool on the connected MCP server",
            parameters={
                "tool_name": {"type": "string", "description": "Name of the MCP tool to invoke"},
                "arguments": {"type": "string", "description": "JSON string of tool arguments"},
            },
            category="mcp",
        )

    # ─── Utility ──────────────────────────────────────────────

    def set_tool_choice(self, choice: ToolChoice) -> None:
        """Set default tool choice behavior"""
        self._tool_choice = choice

    def to_dict(self) -> Dict[str, Any]:
        """Export agent state as dict"""
        return {
            "name": self.name,
            "functions": self.list_functions(),
            "memory_size": len(self._messages),
            "call_history_size": len(self._call_history),
            "tool_choice": self._tool_choice.value,
        }

    def __repr__(self) -> str:
        return f"HermesAgent(name={self.name}, functions={len(self._functions)}, memory={len(self._messages)})"
