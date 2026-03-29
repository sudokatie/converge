defmodule Lattice.Monitoring.Health do
  @moduledoc """
  Health check HTTP server for Lattice.

  Provides endpoints:
  - /health - basic liveness check
  - /ready - readiness check (cluster connected, storage healthy)
  - /metrics - Prometheus-style metrics
  """

  use GenServer

  require Logger

  @default_port 8080

  @type state :: %{
          port: pos_integer(),
          listen_socket: :gen_tcp.socket() | nil,
          acceptor_pid: pid() | nil
        }

  # Client API

  @doc """
  Starts the health check server.

  Options:
  - :port - HTTP port (default 8080)
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Returns current health status.
  """
  @spec health() :: :ok | {:error, term()}
  def health do
    :ok
  end

  @doc """
  Returns readiness status.
  Checks if storage and cluster are operational.
  """
  @spec ready() :: :ok | {:error, term()}
  def ready do
    checks = [
      {:storage, check_storage()},
      {:cluster, check_cluster()}
    ]

    failed = Enum.filter(checks, fn {_, result} -> result != :ok end)

    if length(failed) == 0 do
      :ok
    else
      {:error, failed}
    end
  end

  @doc """
  Stops the health server.
  """
  @spec stop() :: :ok
  def stop do
    GenServer.stop(__MODULE__)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    port = Keyword.get(opts, :port, @default_port)
    enabled = Keyword.get(opts, :enabled, true)

    state = %{
      port: port,
      listen_socket: nil,
      acceptor_pid: nil
    }

    if enabled do
      case start_listener(port) do
        {:ok, socket} ->
          Logger.info("Health server listening on port #{port}")
          pid = spawn_link(fn -> accept_loop(socket) end)
          {:ok, %{state | listen_socket: socket, acceptor_pid: pid}}

        {:error, reason} ->
          Logger.warning("Failed to start health server: #{inspect(reason)}")
          {:ok, state}
      end
    else
      {:ok, state}
    end
  end

  @impl true
  def terminate(_reason, state) do
    if state.listen_socket do
      :gen_tcp.close(state.listen_socket)
    end

    :ok
  end

  # Private functions

  defp start_listener(port) do
    opts = [
      :binary,
      packet: :line,
      active: false,
      reuseaddr: true
    ]

    :gen_tcp.listen(port, opts)
  end

  defp accept_loop(listen_socket) do
    case :gen_tcp.accept(listen_socket, 5000) do
      {:ok, client_socket} ->
        spawn(fn -> handle_client(client_socket) end)
        accept_loop(listen_socket)

      {:error, :timeout} ->
        accept_loop(listen_socket)

      {:error, :closed} ->
        :ok

      {:error, _reason} ->
        accept_loop(listen_socket)
    end
  end

  defp handle_client(socket) do
    case :gen_tcp.recv(socket, 0, 5000) do
      {:ok, data} ->
        path = extract_path(data)
        response = build_response(path)
        :gen_tcp.send(socket, response)
        :gen_tcp.close(socket)

      {:error, _} ->
        :gen_tcp.close(socket)
    end
  end

  defp extract_path(data) do
    case String.split(data, " ") do
      [_method, path | _] -> String.trim(path)
      _ -> "/"
    end
  end

  defp build_response(path) do
    case path do
      "/health" ->
        # health() currently always returns :ok
        json_response(200, %{status: "ok"})

      "/ready" ->
        case ready() do
          :ok ->
            json_response(200, %{status: "ready", storage: "ok", cluster: "ok"})

          {:error, failed} ->
            failed_map =
              Enum.into(failed, %{}, fn {name, {:error, reason}} ->
                {name, inspect(reason)}
              end)

            json_response(503, %{status: "not_ready", failed: failed_map})
        end

      "/metrics" ->
        metrics_response()

      _ ->
        json_response(404, %{error: "not_found"})
    end
  end

  defp json_response(status, body) do
    json = Jason.encode!(body)
    status_text = status_text(status)

    "HTTP/1.1 #{status} #{status_text}\r\n" <>
      "Content-Type: application/json\r\n" <>
      "Content-Length: #{byte_size(json)}\r\n" <>
      "Connection: close\r\n" <>
      "\r\n" <>
      json
  end

  defp metrics_response do
    body = Lattice.Monitoring.Metrics.report()

    "HTTP/1.1 200 OK\r\n" <>
      "Content-Type: text/plain; charset=utf-8\r\n" <>
      "Content-Length: #{byte_size(body)}\r\n" <>
      "Connection: close\r\n" <>
      "\r\n" <>
      body
  end

  defp status_text(200), do: "OK"
  defp status_text(503), do: "Service Unavailable"
  defp status_text(404), do: "Not Found"
  defp status_text(_), do: "Unknown"

  defp check_storage do
    if Process.whereis(Lattice.Storage.Store) do
      :ok
    else
      {:error, :not_running}
    end
  end

  defp check_cluster do
    cond do
      Process.whereis(Lattice.Cluster.Node) == nil ->
        {:error, :node_not_running}

      Process.whereis(Lattice.Cluster.Membership) == nil ->
        {:error, :membership_not_running}

      true ->
        :ok
    end
  end
end
