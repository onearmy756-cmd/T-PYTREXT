defmodule PytrexEngine.Application do
  @moduledoc """
  PyTreX Elixir Concurrency Engine — BEAM VM application.
  Starts a WebSocket server on localhost to receive tasks from Python.
  """

  use Application

  @impl true
  def start(_type, _args) do
    port = Application.get_env(:pytrex_engine, :websocket_port, 42351)

    children = [
      {Task.Supervisor, name: PytrexEngine.TaskSupervisor},
      {PytrexEngine.WebSocketServer, port: port},
      {PytrexEngine.ClusterManager, []}
    ]

    opts = [strategy: :one_for_one, name: PytrexEngine.Supervisor]

    IO.puts("[PyTreX Elixir] Concurrency engine starting on port #{port}...")

    Supervisor.start_link(children, opts)
  end
end
