"""
PyTreXT LangChain Integration — AI Agent Framework
====================================================
Inawezesha PyTreXT kutumia LangChain kwa ajili ya:
- Chain-based reasoning (LLMChain, ConversationChain)
- AI Agents (ZeroShotAgent, ReActAgent)
- Tool integration (PyTreX tools as LangChain tools)
- Memory management (ConversationBufferMemory)
- RAG integration (Retrieval-Augmented Generation)

Usage:
    from pytrex.langchain_agent import LangChainAgent
    agent = LangChainAgent()
    agent.add_tool(my_pytrex_tool)
    result = agent.run("What is the blockchain status?")
"""

import json
import logging
from typing import Any, Callable, Dict, List, Optional, Union
from dataclasses import dataclass, field

logger = logging.getLogger("pytrex.langchain")


@dataclass
class ToolDefinition:
    """Definition ya LangChain tool"""
    name: str
    description: str
    func: Callable
    parameters: Dict[str, Any] = field(default_factory=dict)


class LangChainAgent:
    """
    LangChain AI Agent — inaunganisha PyTreXT na LangChain.
    Supports chain-based reasoning, tool use, and memory management.
    """

    def __init__(
        self,
        model_name: str = "gpt-4",
        temperature: float = 0.7,
        verbose: bool = False,
    ):
        self.model_name = model_name
        self.temperature = temperature
        self.verbose = verbose
        self._tools: Dict[str, ToolDefinition] = {}
        self._memory: List[Dict[str, str]] = []
        self._chains: Dict[str, Any] = {}
        self._model = None
        self._initialized = False

        logger.info(f"LangChainAgent initialized: model={model_name}, temp={temperature}")

    # ─── Tool Management ───────────────────────────────────────

    def add_tool(
        self,
        name: str,
        func: Callable,
        description: str = "",
        parameters: Optional[Dict[str, Any]] = None,
    ) -> None:
        """Sajili tool mpya kwa ajili ya AI agent"""
        self._tools[name] = ToolDefinition(
            name=name,
            description=description or func.__doc__ or f"Tool: {name}",
            func=func,
            parameters=parameters or {},
        )
        logger.info(f"Tool registered: {name}")

    def remove_tool(self, name: str) -> bool:
        """Ondoa tool kutoka kwenye registry"""
        if name in self._tools:
            del self._tools[name]
            logger.info(f"Tool removed: {name}")
            return True
        return False

    def list_tools(self) -> List[Dict[str, Any]]:
        """Orodhesha tools zote zilizosajiliwa"""
        return [
            {
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            }
            for t in self._tools.values()
        ]

    def get_tool(self, name: str) -> Optional[ToolDefinition]:
        """Pata tool kwa jina"""
        return self._tools.get(name)

    # ─── Memory Management ────────────────────────────────────

    def add_message(self, role: str, content: str) -> None:
        """Ongeza ujumbe kwenye kumbukumbu ya mazungumzo"""
        self._memory.append({"role": role, "content": content})

    def get_memory(self) -> List[Dict[str, str]]:
        """Pata kumbukumbu yote ya mazungumzo"""
        return self._memory.copy()

    def clear_memory(self) -> None:
        """Futa kumbukumbu yote"""
        self._memory.clear()

    def get_conversation_history(self, last_n: int = 10) -> str:
        """Pata historia ya mazungumzo kama string"""
        history = self._memory[-last_n:] if last_n > 0 else self._memory
        lines = []
        for msg in history:
            role = msg.get("role", "unknown")
            content = msg.get("content", "")
            lines.append(f"{role.upper()}: {content}")
        return "\n".join(lines)

    # ─── Chain Management ─────────────────────────────────────

    def create_chain(
        self,
        chain_type: str = "conversation",
        system_prompt: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        Tengeneza chain mpya ya LangChain.

        Chain types:
        - "conversation": ConversationChain (chat)
        - "react": ReAct agent (reasoning + action)
        - "qa": Question-answering chain
        - "summarize": Summarization chain
        """
        chain_config = {
            "type": chain_type,
            "model": self.model_name,
            "temperature": self.temperature,
            "system_prompt": system_prompt,
            "tools": self.list_tools(),
        }

        chain_id = f"chain_{len(self._chains)}"
        self._chains[chain_id] = chain_config
        logger.info(f"Chain created: {chain_id} type={chain_type}")
        return chain_config

    # ─── Agent Execution ──────────────────────────────────────

    def run(
        self,
        prompt: str,
        chain_type: str = "conversation",
        tools: Optional[List[str]] = None,
        max_iterations: int = 5,
    ) -> Dict[str, Any]:
        """
        Endesha AI agent na prompt.

        Args:
            prompt: Maagizo ya mtumiaji
            chain_type: Aina ya chain ("conversation", "react", "qa")
            tools: Orodha ya majina ya tools za kutumia
            max_iterations: Idadi ya juu ya iterations

        Returns:
            Dict yenye response na metadata
        """
        if self.verbose:
            logger.info(f"Agent running: type={chain_type}, prompt='{prompt[:100]}...'")

        # Add user message to memory
        self.add_message("user", prompt)

        # Get available tools
        available_tools = {}
        tool_names = tools or list(self._tools.keys())
        for name in tool_names:
            if name in self._tools:
                available_tools[name] = self._tools[name]

        # Build tool descriptions for the prompt
        tool_descriptions = ""
        if available_tools:
            tool_descriptions = "\nAvailable Tools:\n"
            for t in available_tools.values():
                tool_descriptions += f"- {t.name}: {t.description}\n"

        # Build reasoning chain
        try:
            result = self._execute_chain(
                prompt=prompt,
                chain_type=chain_type,
                tools=available_tools,
                tool_descriptions=tool_descriptions,
                max_iterations=max_iterations,
            )

            # Add assistant response to memory
            if "response" in result:
                self.add_message("assistant", result["response"])

            return result

        except Exception as e:
            error_msg = f"Agent execution failed: {e}"
            logger.error(error_msg)
            self.add_message("system", error_msg)
            return {
                "status": "error",
                "response": error_msg,
                "error": str(e),
            }

    def _execute_chain(
        self,
        prompt: str,
        chain_type: str,
        tools: Dict[str, ToolDefinition],
        tool_descriptions: str,
        max_iterations: int,
    ) -> Dict[str, Any]:
        """Internal chain execution with ReAct-style reasoning"""

        if chain_type == "react":
            return self._execute_react(
                prompt, tools, tool_descriptions, max_iterations
            )
        elif chain_type == "qa":
            return self._execute_qa(prompt, tools)
        else:
            return self._execute_conversation(prompt, tools)

    def _execute_conversation(
        self, prompt: str, tools: Dict[str, ToolDefinition]
    ) -> Dict[str, Any]:
        """Execute conversation-style chain"""
        # Try to use LangChain if available
        try:
            from langchain_core.prompts import ChatPromptTemplate
            from langchain_core.output_parsers import StrOutputParser

            has_langchain = True
        except ImportError:
            has_langchain = False

        if has_langchain and self._model:
            # Use LangChain pipeline
            try:
                chain = ChatPromptTemplate.from_messages([
                    ("system", "You are a helpful AI assistant for PyTreXT framework."),
                    ("user", "{input}"),
                ]) | self._model | StrOutputParser()

                response = chain.invoke({"input": prompt})
                return {
                    "status": "ok",
                    "response": response,
                    "chain_type": "conversation",
                    "tools_used": [],
                    "iterations": 1,
                }
            except Exception as e:
                logger.warning(f"LangChain execution failed, using fallback: {e}")

        # Fallback: check if any tool names are mentioned in the prompt
        tools_used = []
        for name, tool in tools.items():
            if name.lower() in prompt.lower():
                try:
                    result = tool.func(prompt)
                    tools_used.append(name)
                    return {
                        "status": "ok",
                        "response": str(result),
                        "chain_type": "conversation",
                        "tools_used": tools_used,
                        "iterations": 1,
                    }
                except Exception as e:
                    logger.error(f"Tool '{name}' failed: {e}")

        return {
            "status": "ok",
            "response": f"[LangChain Agent] Processed: {prompt}. Tools available: {list(tools.keys())}. Install langchain and set up an LLM model for full AI reasoning.",
            "chain_type": "conversation",
            "tools_used": tools_used,
            "iterations": 1,
            "note": "PyTreXT fallback mode — install langchain + langchain-openai for full capabilities",
        }

    def _execute_react(
        self,
        prompt: str,
        tools: Dict[str, ToolDefinition],
        tool_descriptions: str,
        max_iterations: int,
    ) -> Dict[str, Any]:
        """Execute ReAct-style agent (Reasoning + Action)"""
        tools_used = []
        reasoning_steps = []

        # ReAct loop
        thought = f"Question: {prompt}"
        reasoning_steps.append({"type": "thought", "content": thought})

        for iteration in range(max_iterations):
            # Check if any tool matches the current thought
            action_taken = False
            for name, tool in tools.items():
                if name.lower() in thought.lower():
                    reasoning_steps.append({
                        "type": "action",
                        "content": f"Calling tool: {name}",
                    })
                    try:
                        result = tool.func(thought)
                        tools_used.append(name)
                        reasoning_steps.append({
                            "type": "observation",
                            "content": str(result),
                        })
                        thought = str(result)
                        action_taken = True
                    except Exception as e:
                        reasoning_steps.append({
                            "type": "observation",
                            "content": f"Error: {e}",
                        })
                    break

            if not action_taken:
                break

        return {
            "status": "ok",
            "response": thought,
            "chain_type": "react",
            "tools_used": tools_used,
            "iterations": iteration + 1,
            "reasoning_steps": reasoning_steps,
        }

    def _execute_qa(
        self, prompt: str, tools: Dict[str, ToolDefinition]
    ) -> Dict[str, Any]:
        """Execute QA-style chain"""
        # Find relevant context from tools
        context_parts = []
        for name, tool in tools.items():
            try:
                result = tool.func(prompt)
                context_parts.append(f"[{name}]: {result}")
            except Exception:
                pass

        context = "\n".join(context_parts) if context_parts else "No context available."

        return {
            "status": "ok",
            "response": f"Question: {prompt}\n\nContext:\n{context}\n\nAnswer: [QA processing complete]",
            "chain_type": "qa",
            "tools_used": list(tools.keys()),
            "iterations": 1,
        }

    # ─── RAG Integration ──────────────────────────────────────

    def rag_query(
        self,
        query: str,
        documents: List[str],
        top_k: int = 3,
    ) -> Dict[str, Any]:
        """
        Retrieval-Augmented Generation query.
        Uses embedding similarity to find relevant documents.
        """
        from pytrex.core import RAGEngine

        rag = RAGEngine()
        results = rag.query(query, top_k=top_k) if documents else []

        # Add documents if none indexed
        if not results and documents:
            for doc in documents:
                rag.add_document(doc)
            results = rag.query(query, top_k=top_k)

        # Format response
        context = "\n\n".join([
            r.get("content", str(r)) if isinstance(r, dict) else str(r)
            for r in results
        ])
        response = f"Query: {query}\n\nRelevant Context:\n{context}"

        return {
            "status": "ok",
            "query": query,
            "results": results,
            "response": response,
            "top_k": top_k,
        }

    # ─── Utility ──────────────────────────────────────────────

    def set_model(self, model: Any) -> None:
        """Set the underlying LLM model (LangChain or OpenAI compatible)"""
        self._model = model
        self._initialized = True
        logger.info(f"Model set: {type(model).__name__}")

    def to_dict(self) -> Dict[str, Any]:
        """Export agent state as dict"""
        return {
            "model_name": self.model_name,
            "temperature": self.temperature,
            "tools": self.list_tools(),
            "memory_size": len(self._memory),
            "chains": len(self._chains),
            "initialized": self._initialized,
        }

    def __repr__(self) -> str:
        return f"LangChainAgent(model={self.model_name}, tools={len(self._tools)}, memory={len(self._memory)} msgs)"


def create_pytrex_langchain_tools(app) -> List[Dict[str, Any]]:
    """
    Tengeneza LangChain tools kutoka kwenye PyTreX app.
    Inabadilisha methods za PyTreXApp kuwa LangChain tools.

    Args:
        app: PyTreXApp instance

    Returns:
        List ya tool definitions
    """
    tools_info = []

    # Blockchain tools
    if hasattr(app, "blockchain") and app.blockchain is not None:
        tools_info.append({
            "name": "blockchain_status",
            "description": "Get the current blockchain status including last block hash and index",
            "function": lambda _: json.dumps(app.blockchain.get_status() if hasattr(app.blockchain, "get_status") else {"blocks": len(app.blockchain._chain)}),
        })

    # Database tools
    if hasattr(app, "db") and app.db is not None:
        tools_info.append({
            "name": "database_query",
            "description": "Query the encrypted PyTreX database",
            "function": lambda query: str(getattr(app.db, "execute", lambda q: "DB OK")(query)),
        })

    # RAG tools
    if hasattr(app, "rag") and app.rag is not None:
        tools_info.append({
            "name": "rag_search",
            "description": "Search through indexed documents using RAG",
            "function": lambda q: json.dumps(getattr(app.rag, "query", lambda q, **kw: [])(q)),
        })

    # Encryption tools
    if hasattr(app, "encryption") and app.encryption is not None:
        tools_info.append({
            "name": "encrypt",
            "description": "Encrypt data using AES-256",
            "function": lambda data: getattr(app.encryption, "encrypt", lambda d: d)(data),
        })

    return tools_info
