defmodule PytrexEngine.WebSocketHandler do
  @moduledoc """
  Handles WebSocket connections from the Python PyTreX core layer.
  Receives JSON messages with event name and payload, dispatches to TaskSupervisor.
  """

  @behaviour :cowboy_websocket

  @impl true
  def init(req, state) do
    {:cowboy_websocket, req, state}
  end

  @impl true
  def websocket_init(state) do
    IO.puts("[PyTreX Elixir] Python client connected via WebSocket")
    {:ok, state}
  end

  @impl true
  def websocket_handle({:text, message}, state) do
    case Jason.decode(message) do
      # === MCP JSON-RPC Protocol ===
      {:ok, %{"method" => method} = mcp_request} ->
        id = Map.get(mcp_request, "id")
        params = Map.get(mcp_request, "params", %{})
        result = PytrexEngine.MCPHandler.handle_request(method, params, id)
        {:reply, {:text, Jason.encode!(result)}, state}

      # === Standard PyTreX Events ===
      {:ok, %{"event" => event, "payload" => payload, "broadcast" => broadcast?}} ->
        response = PytrexEngine.TaskDispatcher.process_task(event, payload, broadcast?)
        {:reply, {:text, Jason.encode!(response)}, state}

      {:ok, %{"event" => event, "payload" => payload}} ->
        response = PytrexEngine.TaskDispatcher.process_task(event, payload, false)
        {:reply, {:text, Jason.encode!(response)}, state}

      # === MCP Tool Call (short form) ===
      {:ok, %{"tool" => tool_name, "arguments" => args}} ->
        mcp_params = %{"name" => tool_name, "arguments" => args}
        result = PytrexEngine.MCPHandler.handle_request("tools/call", mcp_params)
        {:reply, {:text, Jason.encode!(result)}, state}

      {:error, _} ->
        error_resp = %{"status" => "error", "message" => "Invalid JSON"}
        {:reply, {:text, Jason.encode!(error_resp)}, state}
    end
  end

  @impl true
  def websocket_handle(_frame, state) do
    {:ok, state}
  end

  @impl true
  def websocket_info({:broadcast, event, payload}, state) do
    msg = %{"event" => event, "payload" => payload, "type" => "broadcast"}
    {:reply, {:text, Jason.encode!(msg)}, state}
  end

  @impl true
  def websocket_terminate(_reason, _state) do
    IO.puts("[PyTreX Elixir] Python client disconnected")
    :ok
  end
end
