"""
PyTreXT MCP Client — Model Context Protocol Integration
=========================================================
Inawezesha PyTreXT kuungana na seva za MCP (Model Context Protocol).
- Connect to external MCP servers
- Discover tools, resources, and prompts
- Invoke MCP tools
- Read MCP resources
- Async communication via stdio or HTTP

Usage:
    from pytrex.mcp_client import MCPClient
    client = MCPClient()
    client.connect("http://localhost:8000/mcp")
    tools = client.list_tools()
    result = client.invoke_tool("blockchain_create_block", {"data": "test"})
"""

import json
import logging
import subprocess
import threading
import time
import uuid
from typing import Any, Callable, Dict, List, Optional, Union
from dataclasses import dataclass, field
from enum import Enum

logger = logging.getLogger("pytrex.mcp")


class MCPTransport(Enum):
    """Njia ya kuungana na MCP server"""
    HTTP = "http"
    STDIO = "stdio"
    WEBSOCKET = "websocket"


@dataclass
class MCPTool:
    """MCP tool definition"""
    name: str
    description: str
    input_schema: Dict[str, Any]
    server_name: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        }


@dataclass
class MCPResource:
    """MCP resource definition"""
    uri: str
    name: str
    description: str = ""
    mime_type: str = "text/plain"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "uri": self.uri,
            "name": self.name,
            "description": self.description,
            "mimeType": self.mime_type,
        }


@dataclass
class MCPPrompt:
    """MCP prompt definition"""
    name: str
    description: str = ""
    arguments: List[Dict[str, Any]] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "arguments": self.arguments,
        }


class MCPClient:
    """
    MCP Client — inaunganisha PyTreXT na seva za MCP.
    Supports HTTP, stdio, and WebSocket transports.
    """

    PROTOCOL_VERSION = "2024-11-05"

    def __init__(
        self,
        server_url: str = "",
        transport: MCPTransport = MCPTransport.HTTP,
        timeout: float = 30.0,
        auto_reconnect: bool = True,
    ):
        self.server_url = server_url
        self.transport = transport
        self.timeout = timeout
        self.auto_reconnect = auto_reconnect

        self._session_id: Optional[str] = None
        self._connected = False
        self._server_info: Dict[str, Any] = {}
        self._capabilities: Dict[str, Any] = {}

        # Cached server info
        self._tools: Dict[str, MCPTool] = {}
        self._resources: Dict[str, MCPResource] = {}
        self._prompts: Dict[str, MCPPrompt] = {}

        # Stdio process management
        self._stdio_process: Optional[subprocess.Popen] = None
        self._stdio_thread: Optional[threading.Thread] = None

        # Event hooks
        self._on_connect_hooks: List[Callable] = []
        self._on_disconnect_hooks: List[Callable] = []
        self._on_tool_result_hooks: List[Callable] = []

        logger.info(f"MCPClient initialized: transport={transport.value}")

    # ─── Connection Management ────────────────────────────────

    def connect(self, server_url: Optional[str] = None) -> bool:
        """
        Ungana na MCP server.

        Args:
            server_url: URL ya MCP server (au process path kwa stdio)

        Returns:
            True kama connection imefanikiwa
        """
        if server_url:
            self.server_url = server_url

        if not self.server_url:
            logger.error("No server URL specified")
            return False

        self._session_id = str(uuid.uuid4())[:8]

        try:
            if self.transport == MCPTransport.HTTP:
                success = self._connect_http()
            elif self.transport == MCPTransport.STDIO:
                success = self._connect_stdio()
            elif self.transport == MCPTransport.WEBSOCKET:
                success = self._connect_websocket()
            else:
                logger.error(f"Unknown transport: {self.transport}")
                return False

            if success:
                self._connected = True
                self._discover_capabilities()
                logger.info(f"MCP connected: {self.server_url} (session={self._session_id})")

                # Fire connect hooks
                for hook in self._on_connect_hooks:
                    try:
                        hook(self._server_info)
                    except Exception as e:
                        logger.error(f"Connect hook failed: {e}")

            return success

        except Exception as e:
            logger.error(f"MCP connection failed: {e}")
            self._connected = False
            return False

    def disconnect(self) -> None:
        """Tenganisha na MCP server"""
        if self._stdio_process:
            self._stdio_process.terminate()
            self._stdio_process = None

        self._connected = False
        self._session_id = None
        logger.info("MCP disconnected")

        for hook in self._on_disconnect_hooks:
            try:
                hook()
            except Exception as e:
                logger.error(f"Disconnect hook failed: {e}")

    def is_connected(self) -> bool:
        """Angalia kama tumeunganishwa"""
        return self._connected

    # ─── Transport Implementations ────────────────────────────

    def _connect_http(self) -> bool:
        """Connect via HTTP POST to /mcp endpoint"""
        try:
            import urllib.request
            import urllib.error

            payload = json.dumps({
                "jsonrpc": "2.0",
                "id": self._session_id,
                "method": "initialize",
                "params": {
                    "protocolVersion": self.PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "PyTreXT MCP Client",
                        "version": "1.0.0",
                    },
                },
            }).encode()

            req = urllib.request.Request(
                self.server_url,
                data=payload,
                headers={
                    "Content-Type": "application/json",
                    "User-Agent": "PyTreXT-MCP/1.0",
                },
            )

            with urllib.request.urlopen(req, timeout=self.timeout) as response:
                data = json.loads(response.read().decode())

            if "result" in data:
                self._server_info = data["result"].get("serverInfo", {})
                self._capabilities = data["result"].get("capabilities", {})
                return True

            return False

        except urllib.error.HTTPError as e:
            logger.error(f"MCP HTTP error: {e.code} {e.reason}")
            return False
        except urllib.error.URLError as e:
            logger.error(f"MCP connection refused: {e.reason}")
            return False
        except Exception as e:
            logger.error(f"MCP HTTP connect failed: {e}")
            return False

    def _connect_stdio(self) -> bool:
        """Connect via stdio (subprocess)"""
        try:
            self._stdio_process = subprocess.Popen(
                self.server_url.split(),
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            # Send initialize request
            init_msg = json.dumps({
                "jsonrpc": "2.0",
                "id": self._session_id,
                "method": "initialize",
                "params": {
                    "protocolVersion": self.PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "PyTreXT", "version": "1.0.0"},
                },
            })

            stdout, stderr = self._stdio_process.communicate(
                input=init_msg + "\n",
                timeout=self.timeout,
            )

            if stdout:
                data = json.loads(stdout.strip())
                if "result" in data:
                    self._server_info = data["result"].get("serverInfo", {})
                    self._capabilities = data["result"].get("capabilities", {})
                    return True

            return False

        except Exception as e:
            logger.error(f"MCP stdio connect failed: {e}")
            return False

    def _connect_websocket(self) -> bool:
        """Connect via WebSocket"""
        try:
            # Using Python's built-in — for production, use websockets or aiohttp
            logger.info(f"WebSocket connect attempted to {self.server_url}")
            # WebSocket implementation requires additional dependencies
            logger.warning("WebSocket transport requires 'websockets' package")
            return False
        except Exception as e:
            logger.error(f"MCP WebSocket connect failed: {e}")
            return False

    def _discover_capabilities(self) -> None:
        """Discover server tools, resources, and prompts"""
        # Discover tools
        tools_data = self._send_request("tools/list")
        if tools_data and "tools" in tools_data:
            for tool in tools_data["tools"]:
                self._tools[tool["name"]] = MCPTool(
                    name=tool["name"],
                    description=tool.get("description", ""),
                    input_schema=tool.get("inputSchema", {}),
                    server_name=self._server_info.get("name", ""),
                )
            logger.info(f"Discovered {len(self._tools)} MCP tools")

        # Discover resources
        resources_data = self._send_request("resources/list")
        if resources_data and "resources" in resources_data:
            for resource in resources_data["resources"]:
                self._resources[resource["uri"]] = MCPResource(
                    uri=resource["uri"],
                    name=resource.get("name", ""),
                    description=resource.get("description", ""),
                    mime_type=resource.get("mimeType", "text/plain"),
                )
            logger.info(f"Discovered {len(self._resources)} MCP resources")

        # Discover prompts
        prompts_data = self._send_request("prompts/list")
        if prompts_data and "prompts" in prompts_data:
            for prompt in prompts_data["prompts"]:
                self._prompts[prompt["name"]] = MCPPrompt(
                    name=prompt["name"],
                    description=prompt.get("description", ""),
                    arguments=prompt.get("arguments", []),
                )
            logger.info(f"Discovered {len(self._prompts)} MCP prompts")

    def _send_request(self, method: str, params: Optional[Dict] = None) -> Optional[Dict]:
        """Send JSON-RPC request to MCP server"""
        try:
            import urllib.request

            payload = json.dumps({
                "jsonrpc": "2.0",
                "id": str(uuid.uuid4()),
                "method": method,
                "params": params or {},
            }).encode()

            req = urllib.request.Request(
                self.server_url,
                data=payload,
                headers={
                    "Content-Type": "application/json",
                    "User-Agent": "PyTreXT-MCP/1.0",
                },
            )

            with urllib.request.urlopen(req, timeout=self.timeout) as response:
                data = json.loads(response.read().decode())

            return data.get("result")

        except Exception as e:
            logger.error(f"MCP request '{method}' failed: {e}")
            return None

    # ─── Tool Operations ──────────────────────────────────────

    def list_tools(self, refresh: bool = False) -> List[Dict[str, Any]]:
        """Orodhesha MCP tools zote zinazopatikana"""
        if refresh or not self._tools:
            self._discover_capabilities()

        return [t.to_dict() for t in self._tools.values()]

    def get_tool(self, name: str) -> Optional[Dict[str, Any]]:
        """Pata tool kwa jina"""
        tool = self._tools.get(name)
        return tool.to_dict() if tool else None

    def invoke_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
        timeout: Optional[float] = None,
    ) -> Dict[str, Any]:
        """
        Ita MCP tool.

        Args:
            tool_name: Jina la tool
            arguments: Arguments za tool
            timeout: Muda wa kusubiri

        Returns:
            Matokeo ya tool
        """
        if not self._connected:
            # Try local/embedded invocation via Rust
            return self._invoke_local_tool(tool_name, arguments)

        # Try Rust MCP client first
        try:
            import my_framework
            params_json = json.dumps({"name": tool_name, "arguments": arguments})
            result = my_framework.mcp_client_tuma(
                self._session_id or "", "tools/call", params_json
            )
            result_data = json.loads(result)
            logger.info(f"MCP tool invoked: {tool_name}")
            return result_data
        except ImportError:
            pass

        # Fallback: HTTP request
        params = {
            "name": tool_name,
            "arguments": arguments,
        }

        result = self._send_request("tools/call", params)

        if result:
            # Fire tool result hooks
            for hook in self._on_tool_result_hooks:
                try:
                    hook(tool_name, result)
                except Exception as e:
                    logger.error(f"Tool result hook failed: {e}")

        return result or {"content": [], "isError": True, "error": "Tool invocation failed"}

    def _invoke_local_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Invoke a PyTreX built-in tool locally (no MCP server needed)"""
        try:
            import my_framework

            if tool_name == "blockchain_create_block":
                data = arguments.get("data", "")
                result = my_framework.fanya_block_ya_blockchain(data)
                return {"content": [{"type": "text", "text": result}]}

            elif tool_name == "blockchain_verify":
                chain = arguments.get("chain_json", "[]")
                valid = my_framework.hakiki_blockchain(chain)
                return {"content": [{"type": "text", "text": f"Chain valid: {valid}"}]}

            elif tool_name == "database_transaction":
                acc = arguments.get("acc_no", "")
                tx_type = arguments.get("type", "deposit")
                amount = arguments.get("amount", 0.0)
                result = my_framework.fanya_muamala_salama(acc, tx_type, amount)
                return {"content": [{"type": "text", "text": result}]}

            elif tool_name == "encrypt_data":
                data = arguments.get("data", "")
                key = arguments.get("key", "")
                result = my_framework.aes_encrypt(data, key)
                return {"content": [{"type": "text", "text": result}]}

            elif tool_name == "decrypt_data":
                encrypted = arguments.get("encrypted", "")
                key = arguments.get("key", "")
                result = my_framework.aes_decrypt(encrypted, key)
                return {"content": [{"type": "text", "text": result}]}

        except ImportError:
            pass
        except Exception as e:
            logger.error(f"Local tool '{tool_name}' failed: {e}")

        return {
            "content": [{"type": "text", "text": f"Tool '{tool_name}' not available locally"}],
            "isError": True,
        }

    # ─── Resource Operations ──────────────────────────────────

    def list_resources(self, refresh: bool = False) -> List[Dict[str, Any]]:
        """Orodhesha MCP resources zote"""
        if refresh or not self._resources:
            self._discover_capabilities()

        return [r.to_dict() for r in self._resources.values()]

    def read_resource(self, uri: str) -> Optional[Dict[str, Any]]:
        """
        Soma MCP resource.

        Args:
            uri: URI ya resource (mf: pytrex://blockchain/chain)

        Returns:
            Yaliyomo ya resource
        """
        result = self._send_request("resources/read", {"uri": uri})

        if result is None:
            # Try local resources
            if uri == "pytrex://blockchain/chain":
                try:
                    import my_framework
                    return {"contents": [{"text": "Blockchain active", "uri": uri}]}
                except ImportError:
                    pass
            elif uri == "pytrex://system/health":
                return {
                    "contents": [{
                        "text": json.dumps({"status": "healthy", "framework": "PyTreXT"}),
                        "uri": uri,
                    }]
                }

        return result

    # ─── Prompt Operations ────────────────────────────────────

    def list_prompts(self, refresh: bool = False) -> List[Dict[str, Any]]:
        """Orodhesha MCP prompts zote"""
        if refresh or not self._prompts:
            self._discover_capabilities()

        return [p.to_dict() for p in self._prompts.values()]

    def get_prompt(self, name: str, arguments: Optional[Dict] = None) -> Optional[Dict[str, Any]]:
        """
        Pata MCP prompt.

        Args:
            name: Jina la prompt
            arguments: Arguments za prompt

        Returns:
            Prompt messages
        """
        result = self._send_request("prompts/get", {
            "name": name,
            "arguments": arguments or {},
        })

        if result is None:
            # Try local prompts
            if name == "audit_blockchain":
                return {
                    "messages": [{
                        "role": "user",
                        "content": {"type": "text", "text": "Please audit the blockchain for any tampering or inconsistencies."}
                    }]
                }
            elif name == "secure_transaction":
                return {
                    "messages": [{
                        "role": "user",
                        "content": {"type": "text", "text": "Execute a secure ACID-compliant database transaction with proper validation."}
                    }]
                }

        return result

    # ─── Hooks ────────────────────────────────────────────────

    def on_connect(self, callback: Callable) -> None:
        """Register connect hook"""
        self._on_connect_hooks.append(callback)

    def on_disconnect(self, callback: Callable) -> None:
        """Register disconnect hook"""
        self._on_disconnect_hooks.append(callback)

    def on_tool_result(self, callback: Callable) -> None:
        """Register tool result hook"""
        self._on_tool_result_hooks.append(callback)

    # ─── Utility ──────────────────────────────────────────────

    def server_info(self) -> Dict[str, Any]:
        """Get MCP server information"""
        return {
            "connected": self._connected,
            "server_url": self.server_url,
            "session_id": self._session_id,
            "transport": self.transport.value,
            "server_name": self._server_info.get("name", "unknown"),
            "server_version": self._server_info.get("version", "unknown"),
            "protocol_version": self.PROTOCOL_VERSION,
            "tools_count": len(self._tools),
            "resources_count": len(self._resources),
            "prompts_count": len(self._prompts),
        }

    def to_dict(self) -> Dict[str, Any]:
        """Export client state"""
        return self.server_info()

    def __repr__(self) -> str:
        status = "connected" if self._connected else "disconnected"
        return f"MCPClient({self.server_url or 'no-url'}, {status}, tools={len(self._tools)})"


# ─── Convenience Functions ────────────────────────────────────

def connect_to_pytrex_mcp(port: int = 8000) -> MCPClient:
    """
    Ungana na MCP server ya PyTreXT inayoendeshwa ndani ya app.
    """
    client = MCPClient(
        server_url=f"http://127.0.0.1:{port}/mcp",
        transport=MCPTransport.HTTP,
    )
    client.connect()
    return client


def discover_mcp_tools(server_url: str) -> List[Dict[str, Any]]:
    """
    Gundua tools zote kutoka MCP server ya nje.
    """
    client = MCPClient(server_url=server_url)
    if client.connect():
        return client.list_tools()
    return []
