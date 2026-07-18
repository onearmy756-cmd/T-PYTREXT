defmodule PytrexEngine.WebSocketServer do
  @moduledoc """
  WebSocket server that accepts connections from the Python layer.
  Uses Plug/Cowboy for WebSocket handling.
  """

  use Plug.Router

  plug Plug.Logger
  plug :match
  plug Plug.Parsers, parsers: [:json], json_decoder: Jason
  plug :dispatch

  get "/" do
    send_resp(conn, 200, "PyTreX Elixir Engine — WebSocket endpoint at /ws")
  end

  get "/ws" do
    conn
    |> WebSockAdapter.upgrade(PytrexEngine.WebSocketHandler, [])
    |> halt()
  end

  match _ do
    send_resp(conn, 404, "Not Found")
  end

  def child_spec(opts) do
    port = Keyword.fetch!(opts, :port)

    %{
      id: __MODULE__,
      start: {Plug.Cowboy, :start_link, [scheme: :http, plug: __MODULE__, options: [port: port]]},
      type: :worker,
      restart: :permanent
    }
  end
end
