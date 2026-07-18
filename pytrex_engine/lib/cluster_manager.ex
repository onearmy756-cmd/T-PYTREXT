defmodule PytrexEngine.ClusterManager do
  @moduledoc """
  Manages BEAM VM cluster connections for distributed task broadcasting.
  """

  use GenServer

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @impl true
  def init(_opts) do
    cookie = Application.get_env(:pytrex_engine, :cluster_cookie)
    if cookie == nil do
      IO.puts("[PyTreX Elixir] WARNING: cluster_cookie not configured — cluster networking disabled.")
      IO.puts("[PyTreX Elixir] Set cluster_cookie in config to enable cluster features.")
      {:ok, %{nodes: [], subscribers: [], cookie_set: false}}
    else
      Node.set_cookie(cookie)
      {:ok, %{nodes: [], subscribers: [], cookie_set: true}}
    end
  end

  @doc """
  Connect to remote BEAM nodes for cluster distribution.
  """
  def connect_cluster(nodes) when is_list(nodes) do
    GenServer.call(__MODULE__, {:connect, nodes})
  end

  @doc """
  Broadcast an event to all connected cluster nodes.
  """
  def broadcast(event, payload) do
    GenServer.cast(__MODULE__, {:broadcast, event, payload})
  end

  @impl true
  def handle_call({:connect, _nodes}, _from, %{cookie_set: false} = state) do
    {:reply, {:error, :cookie_not_configured}, state}
  end

  @impl true
  def handle_call({:connect, nodes}, _from, state) do
    results = Enum.map(nodes, fn node_name ->
      case Node.connect(String.to_atom(node_name)) do
        true ->
          IO.puts("[PyTreX Elixir] Connected to cluster node: #{node_name}")
          node_name
        false ->
          IO.puts("[PyTreX Elixir] Failed to connect to: #{node_name}")
          nil
      end
    end)

    connected = Enum.filter(results, &(&1 != nil))
    {:reply, {:ok, connected}, %{state | nodes: connected}}
  end

  @impl true
  def handle_cast({:broadcast, event, payload}, state) do
    Enum.each(state.nodes, fn node_name ->
      :rpc.call(String.to_atom(node_name), PytrexEngine.TaskDispatcher, :process_task, [event, payload, false])
    end)

    IO.puts("[PyTreX Elixir] Broadcast '#{event}' to #{length(state.nodes)} cluster nodes")
    {:noreply, state}
  end
end
