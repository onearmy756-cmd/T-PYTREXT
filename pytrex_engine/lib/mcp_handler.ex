defmodule PytrexEngine.MCPHandler do
  @moduledoc """
  PyTreXT MCP (Model Context Protocol) Handler — Elixir Engine
  
  Inasimamia MCP protocol kwenye Elixir BEAM VM:
  - Tool execution na result broadcasting
  - Resource management
  - Prompt handling
  - Integration na Python MCP client
  """

  require Logger

  @protocol_version "2024-11-05"
  @server_name "PyTreXT Elixir MCP"
  @server_version "1.0.0"

  # ============================================================
  #  TOOL DEFINITIONS
  # ============================================================

  @tools [
    %{
      name: "elixir_cluster_status",
      description: "Get the current BEAM cluster status including connected nodes",
      inputSchema: %{
        type: "object",
        properties: %{},
        required: []
      }
    },
    %{
      name: "elixir_broadcast_event",
      description: "Broadcast an event to all connected BEAM cluster nodes",
      inputSchema: %{
        type: "object",
        properties: %{
          event: %{type: "string", description: "Event name to broadcast"},
          payload: %{type: "string", description: "JSON payload to broadcast"}
        },
        required: ["event"]
      }
    },
    %{
      name: "elixir_task_dispatch",
      description: "Dispatch an async task to the Elixir TaskSupervisor",
      inputSchema: %{
        type: "object",
        properties: %{
          event: %{type: "string", description: "Task event name"},
          payload: %{type: "string", description: "JSON payload"},
          broadcast: %{type: "boolean", description: "Whether to broadcast to cluster"}
        },
        required: ["event"]
      }
    },
    %{
      name: "search_web",
      description: "Search the web via SearXNG or DuckDuckGo integration",
      inputSchema: %{
        type: "object",
        properties: %{
          query: %{type: "string", description: "Search query"},
          engine: %{type: "string", enum: ["duckduckgo", "searxng"], description: "Search engine"}
        },
        required: ["query"]
      }
    },
    %{
      name: "human_approval_request",
      description: "Request human approval for an AI action",
      inputSchema: %{
        type: "object",
        properties: %{
          action_type: %{type: "string", description: "Type of action requiring approval"},
          context: %{type: "string", description: "JSON context for the approval"},
          timeout: %{type: "number", description: "Timeout in seconds"}
        },
        required: ["action_type"]
      }
    },
    %{
      name: "hermes_function_call",
      description: "Execute a Hermes AI agent function call",
      inputSchema: %{
        type: "object",
        properties: %{
          function_name: %{type: "string", description: "Name of function to call"},
          arguments: %{type: "string", description: "JSON arguments for the function"}
        },
        required: ["function_name"]
      }
    }
  ]

  @resources [
    %{
      uri: "pytrex://elixir/cluster",
      name: "BEAM Cluster",
      description: "Current state of the BEAM VM cluster",
      mimeType: "application/json"
    },
    %{
      uri: "pytrex://elixir/nodes",
      name: "Connected Nodes",
      description: "List of connected BEAM nodes",
      mimeType: "application/json"
    },
    %{
      uri: "pytrex://elixir/tasks",
      name: "Task Status",
      description: "Status of running Elixir tasks",
      mimeType: "application/json"
    }
  ]

  @prompts [
    %{
      name: "distributed_task",
      description: "Execute a task across the BEAM cluster",
      arguments: [
        %{name: "task_name", description: "Name of the task to execute", required: true},
        %{name: "nodes", description: "Target nodes (comma-separated, or 'all')", required: false}
      ]
    },
    %{
      name: "cluster_health_check",
      description: "Check health of all connected BEAM nodes",
      arguments: []
    },
    %{
      name: "realtime_sync",
      description: "Sync data in real-time across all cluster nodes",
      arguments: [
        %{name: "data_type", description: "Type of data to sync", required: true},
        %{name: "payload", description: "Data payload as JSON", required: true}
      ]
    }
  ]

  # ============================================================
  #  PUBLIC API
  # ============================================================

  @doc """
  Process an MCP JSON-RPC request and return the appropriate response.
  
  ## Parameters
    - method: MCP method name (initialize, tools/list, tools/call, etc.)
    - params: Method parameters (map)
    - id: Request ID
  
  ## Returns
    - Map with :jsonrpc, :id, and :result (or :error)
  """
  def handle_request(method, params \\ %{}, id \\ nil) do
    case method do
      "initialize" -> handle_initialize(params)
      "tools/list" -> handle_tools_list()
      "tools/call" -> handle_tool_call(params)
      "resources/list" -> handle_resources_list()
      "resources/read" -> handle_resource_read(params)
      "prompts/list" -> handle_prompts_list()
      "prompts/get" -> handle_prompt_get(params)
      _ -> error_response(id, -32601, "Method not found: #{method}")
    end
    |> wrap_response(id)
  end

  @doc """
  Get all registered MCP tools.
  """
  def get_tools, do: @tools

  @doc """
  Get all registered MCP resources.
  """
  def get_resources, do: @resources

  @doc """
  Get all registered MCP prompts.
  """
  def get_prompts, do: @prompts

  # ============================================================
  #  METHOD HANDLERS
  # ============================================================

  defp handle_initialize(_params) do
    %{
      protocolVersion: @protocol_version,
      capabilities: %{
        tools: %{},
        resources: %{},
        prompts: %{}
      },
      serverInfo: %{
        name: @server_name,
        version: @server_version
      }
    }
  end

  defp handle_tools_list do
    %{tools: @tools}
  end

  defp handle_tool_call(%{"name" => tool_name} = params) do
    arguments = Map.get(params, "arguments", %{})

    case tool_name do
      "elixir_cluster_status" ->
        nodes = Node.list()
        %{
          content: [
            %{
              type: "text",
              text: "BEAM Cluster Status:\n- Self: #{Node.self()}\n- Connected: #{length(nodes)} nodes\n- Nodes: #{inspect(nodes)}"
            }
          ]
        }

      "elixir_broadcast_event" ->
        event = Map.get(arguments, "event", "unknown")
        payload = Map.get(arguments, "payload", "{}")

        # Broadcast to cluster
        PytrexEngine.ClusterManager.broadcast(event, payload)

        %{
          content: [
            %{
              type: "text",
              text: "Broadcast event '#{event}' sent to cluster"
            }
          ]
        }

      "elixir_task_dispatch" ->
        event = Map.get(arguments, "event", "unknown")
        payload = Map.get(arguments, "payload", "{}")
        broadcast? = Map.get(arguments, "broadcast", false)

        task_result = PytrexEngine.TaskDispatcher.process_task(event, payload, broadcast?)

        %{
          content: [
            %{
              type: "text",
              text: "Task '#{event}' dispatched: #{task_result}"
            }
          ]
        }

      "search_web" ->
        query = Map.get(arguments, "query", "")
        engine = Map.get(arguments, "engine", "duckduckgo")

        # Search via Python bridge (WebSocket callback)
        result = PytrexEngine.TaskDispatcher.process_task(
          "search_web",
          Jason.encode!(%{query: query, engine: engine}),
          false
        )

        %{
          content: [
            %{
              type: "text",
              text: "Search results for '#{query}' via #{engine}: #{result}"
            }
          ]
        }

      "human_approval_request" ->
        action_type = Map.get(arguments, "action_type", "unknown")
        context = Map.get(arguments, "context", "{}")
        timeout = Map.get(arguments, "timeout", 300)

        # Forward to Python human-in-the-loop
        PytrexEngine.TaskDispatcher.process_task(
          "human_approval",
          Jason.encode!(%{action_type: action_type, context: context, timeout: timeout}),
          true
        )

        %{
          content: [
            %{
              type: "text",
              text: "Human approval requested for action '#{action_type}' (timeout: #{timeout}s)"
            }
          ]
        }

      "hermes_function_call" ->
        func_name = Map.get(arguments, "function_name", "")
        func_args = Map.get(arguments, "arguments", "{}")

        PytrexEngine.TaskDispatcher.process_task(
          "hermes_function_call",
          Jason.encode!(%{function_name: func_name, arguments: func_args}),
          false
        )

        %{
          content: [
            %{
              type: "text",
              text: "Hermes function '#{func_name}' executed with args: #{func_args}"
            }
          ]
        }

      _ ->
        %{
          content: [
            %{
              type: "text",
              text: "Unknown tool: #{tool_name}"
            }
          ],
          isError: true
        }
    end
  end

  defp handle_resources_list do
    %{resources: @resources}
  end

  defp handle_resource_read(%{"uri" => uri}) do
    case uri do
      "pytrex://elixir/cluster" ->
        nodes = Node.list()
        %{
          contents: [
            %{
              uri: uri,
              mimeType: "application/json",
              text: Jason.encode!(%{
                self: Node.self(),
                connected_nodes: nodes,
                node_count: length(nodes),
                alive: Node.alive?()
              })
            }
          ]
        }

      "pytrex://elixir/nodes" ->
        %{
          contents: [
            %{
              uri: uri,
              mimeType: "application/json",
              text: Jason.encode!(%{
                nodes: [Node.self() | Node.list()],
                visible: Node.list(:visible),
                hidden: Node.list(:hidden)
              })
            }
          ]
        }

      "pytrex://elixir/tasks" ->
        %{
          contents: [
            %{
              uri: uri,
              mimeType: "application/json",
              text: Jason.encode!(%{
                status: "operational",
                supervisor: "PytrexEngine.TaskSupervisor",
                message: "Task system is running"
              })
            }
          ]
        }

      _ ->
        %{
          contents: [],
          isError: true,
          error: "Resource not found: #{uri}"
        }
    end
  end

  defp handle_prompts_list do
    %{prompts: @prompts}
  end

  defp handle_prompt_get(%{"name" => prompt_name} = params) do
    arguments = Map.get(params, "arguments", %{})

    case prompt_name do
      "distributed_task" ->
        task_name = Map.get(arguments, "task_name", "unknown")
        nodes = Map.get(arguments, "nodes", "all")

        %{
          messages: [
            %{
              role: "user",
              content: %{
                type: "text",
                text: "Execute the '#{task_name}' task across the BEAM cluster. Target nodes: #{nodes}. Ensure proper error handling and collect results from all nodes."
              }
            }
          ]
        }

      "cluster_health_check" ->
        %{
          messages: [
            %{
              role: "user",
              content: %{
                type: "text",
                text: "Perform a health check of all connected BEAM nodes. Check memory usage, process count, message queue lengths, and node responsiveness. Report any anomalies."
              }
            }
          ]
        }

      "realtime_sync" ->
        data_type = Map.get(arguments, "data_type", "unknown")
        payload = Map.get(arguments, "payload", "{}")

        %{
          messages: [
            %{
              role: "user",
              content: %{
                type: "text",
                text: "Sync #{data_type} data across all cluster nodes in real-time. Payload: #{payload}. Ensure consistency and handle conflicts appropriately."
              }
            }
          ]
        }

      _ ->
        %{
          messages: [],
          isError: true,
          error: "Prompt not found: #{prompt_name}"
        }
    end
  end

  # ============================================================
  #  HELPERS
  # ============================================================

  defp wrap_response(result, id) do
    case result do
      %{error: error_data} ->
        %{
          jsonrpc: "2.0",
          id: id,
          error: error_data
        }

      _ ->
        %{
          jsonrpc: "2.0",
          id: id,
          result: result
        }
    end
  end

  defp error_response(id, code, message) do
    %{
      error: %{
        code: code,
        message: message
      }
    }
  end
end
