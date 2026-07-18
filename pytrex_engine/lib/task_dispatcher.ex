defmodule PytrexEngine.TaskDispatcher do
  @moduledoc """
  Dispatches incoming tasks from Python to supervised async tasks.
  Handles event processing and cluster-wide broadcasting.
  """

  @doc """
  Process an incoming task from the Python layer.
  Returns a response map that will be JSON-encoded and sent back.
  """
  def process_task(event, payload, broadcast? \\ false) do
    task = Task.Supervisor.async(PytrexEngine.TaskSupervisor, fn ->
      do_process(event, payload)
    end)

    result = Task.await(task, 30_000)

    if broadcast? do
      PytrexEngine.ClusterManager.broadcast(event, payload)
    end

    %{"status" => "ok", "event" => event, "result" => result}
  rescue
    e ->
      %{"status" => "error", "message" => inspect(e)}
  end

  defp do_process("sync_transaction", payload) do
    IO.puts("[PyTreX Elixir] Processing transaction sync: #{inspect(payload)}")
    "Transaction synced across cluster"
  end

  defp do_process("inventory_update", payload) do
    IO.puts("[PyTreX Elixir] Processing inventory update: #{inspect(payload)}")
    "Inventory updated across cluster"
  end

  defp do_process("mauzo_mapya", payload) do
    IO.puts("[PyTreX Elixir] Processing new sale: #{inspect(payload)}")
    "Sale distributed to all branches"
  end

  defp do_process("blockchain_broadcast", payload) do
    IO.puts("[PyTreX Elixir] Distributing blockchain block: #{inspect(payload)}")
    "Block distributed to all cluster nodes"
  end

  defp do_process("training_progress", payload) do
    IO.puts("[PyTreX Elixir] Broadcasting training progress: #{inspect(payload)}")
    "Progress broadcast to UI"
  end

  # === PyTreXT Extended: Search & Web ===
  defp do_process("search_web", payload) do
    IO.puts("[PyTreX Elixir] Web search requested: #{inspect(payload)}")
    "Web search dispatched to SearchEngine (SearXNG/DuckDuckGo)"
  end

  defp do_process("search_results", payload) do
    IO.puts("[PyTreX Elixir] Search results received: #{inspect(payload)}")
    "Search results processed"
  end

  # === PyTreXT Extended: Human-in-the-Loop ===
  defp do_process("human_approval", payload) do
    IO.puts("[PyTreX Elixir] Human approval requested: #{inspect(payload)}")
    "Human approval workflow initiated — awaiting response"
  end

  defp do_process("human_approved", payload) do
    IO.puts("[PyTreX Elixir] Human approved action: #{inspect(payload)}")
    "Action approved — executing now"
  end

  defp do_process("human_rejected", payload) do
    IO.puts("[PyTreX Elixir] Human rejected action: #{inspect(payload)}")
    "Action rejected"
  end

  # === PyTreXT Extended: Hermes Agent ===
  defp do_process("hermes_function_call", payload) do
    IO.puts("[PyTreX Elixir] Hermes function call: #{inspect(payload)}")
    "Hermes function executed"
  end

  defp do_process("hermes_response", payload) do
    IO.puts("[PyTreX Elixir] Hermes agent response: #{inspect(payload)}")
    "Hermes response processed"
  end

  # === PyTreXT Extended: MCP Protocol ===
  defp do_process("mcp_invoke", payload) do
    IO.puts("[PyTreX Elixir] MCP tool invocation: #{inspect(payload)}")
    "MCP tool invoked"
  end

  defp do_process("mcp_discover", _payload) do
    IO.puts("[PyTreX Elixir] MCP tool discovery requested")
    "MCP tools discovered"
  end

  # === PyTreXT Extended: LangChain ===
  defp do_process("langchain_agent", payload) do
    IO.puts("[PyTreX Elixir] LangChain agent execution: #{inspect(payload)}")
    "LangChain agent executed"
  end

  # === PyTreXT Extended: Axum Server ===
  defp do_process("axum_event", payload) do
    IO.puts("[PyTreX Elixir] Axum HTTP event received: #{inspect(payload)}")
    "Axum event routed"
  end

  defp do_process(event, payload) do
    IO.puts("[PyTreX Elixir] Generic event '#{event}': #{inspect(payload)}")
    "Event '#{event}' processed"
  end
end
