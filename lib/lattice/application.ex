defmodule Lattice.Application do
  @moduledoc """
  OTP Application for Lattice CRDT database.

  Starts all services in dependency order:
  1. Config - configuration management
  2. Metrics - metrics collection
  3. Node - node identity
  4. Store - data storage
  5. WAL - write-ahead log
  6. Snapshot - periodic snapshots
  7. AntiEntropy - sync coordination
  8. Discovery - peer discovery
  9. Membership - cluster membership
  10. Health - health check HTTP server
  """
  use Application

  @impl true
  def start(_type, _args) do
    # Don't start services in test mode - tests manage their own
    if Mix.env() == :test do
      opts = [strategy: :one_for_one, name: Lattice.Supervisor]
      Supervisor.start_link([], opts)
    else
      start_services()
    end
  rescue
    # Mix not available (escript) - start normally
    _ -> start_services()
  end

  defp start_services do
    data_dir = get_config(:data_dir, default_data_dir())
    node_id = get_config(:node_id, nil)
    sync_interval = get_config(:sync_interval_ms, 5_000)
    enable_discovery = get_config(:enable_discovery, true)
    enable_monitoring = get_config(:enable_monitoring, true)
    health_port = get_config(:health_port, 8080)

    # Ensure data directory exists
    File.mkdir_p!(data_dir)

    children =
      [
        # Monitoring (start early)
        monitoring_child(enable_monitoring),
        health_child(enable_monitoring, health_port),

        # Consistency manager
        {Lattice.Consistency, []},

        # Node identity (must start first)
        {Lattice.Cluster.Node,
         [
           data_dir: data_dir,
           node_id: node_id
         ]},

        # Storage layer
        {Lattice.Storage.Store,
         [
           data_dir: data_dir
         ]},
        {Lattice.Storage.WAL,
         [
           data_dir: data_dir
         ]},
        {Lattice.Storage.Snapshot,
         [
           data_dir: data_dir
         ]},

        # Sync layer
        {Lattice.Sync.AntiEntropy,
         [
           interval_ms: sync_interval
         ]},

        # Cluster layer (optional based on config)
        discovery_child(data_dir, enable_discovery),
        membership_child()
      ]
      |> List.flatten()
      |> Enum.reject(&is_nil/1)

    opts = [strategy: :one_for_one, name: Lattice.Supervisor]
    Supervisor.start_link(children, opts)
  end

  @impl true
  def stop(_state) do
    :ok
  end

  defp monitoring_child(true) do
    {Lattice.Monitoring.Metrics, []}
  end

  defp monitoring_child(false), do: nil

  defp health_child(true, port) do
    {Lattice.Monitoring.Health, [port: port, enabled: true]}
  end

  defp health_child(false, _port), do: nil

  defp discovery_child(data_dir, true) do
    {Lattice.Cluster.Discovery,
     [
       data_dir: data_dir,
       enabled: true
     ]}
  end

  defp discovery_child(_data_dir, false), do: nil

  defp membership_child do
    {Lattice.Sync.Membership,
     [
       enabled: true
     ]}
  end

  defp get_config(key, default) do
    Application.get_env(:lattice, key, default)
  end

  defp default_data_dir do
    # Use home directory for development, /var/lib/lattice for production
    case Mix.env() do
      :prod ->
        case :os.type() do
          {:unix, _} -> "/var/lib/lattice"
          {:win32, _} -> Path.join(System.user_home!(), "AppData/Local/Lattice")
        end

      _ ->
        Path.join(System.tmp_dir!(), "lattice_dev")
    end
  rescue
    # Mix not available (escript)
    _ -> Path.join(System.user_home!(), ".lattice")
  end
end
