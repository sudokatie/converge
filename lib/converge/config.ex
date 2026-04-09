defmodule Converge.Config do
  @moduledoc """
  Configuration management for Converge.
  """

  @default_config %{
    node_id: nil,
    data_dir: "~/.local/share/converge",
    sync_interval_ms: 5000,
    seed_nodes: [],
    listen_port: 4000,
    enable_mdns: true,
    snapshot_interval_ms: 60_000,
    enable_monitoring: true,
    health_port: 8080,
    default_consistency: :eventual
  }

  @doc """
  Get configuration value with default fallback.
  """
  def get(key) do
    Application.get_env(:converge, key, Map.get(@default_config, key))
  end

  @doc """
  Get all configuration as a map.
  """
  def all do
    Enum.reduce(Map.keys(@default_config), %{}, fn key, acc ->
      Map.put(acc, key, get(key))
    end)
  end

  @doc """
  Get the data directory path, expanded.
  """
  def data_dir do
    get(:data_dir) |> Path.expand()
  end

  @doc """
  Get or generate node ID.
  """
  def node_id do
    case get(:node_id) do
      nil -> generate_node_id()
      id -> id
    end
  end

  defp generate_node_id do
    UUID.uuid4()
  end
end
