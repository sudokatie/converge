defmodule Lattice.ConfigTest do
  use ExUnit.Case, async: true

  alias Lattice.Config

  test "get returns default value" do
    assert Config.get(:sync_interval_ms) == 5000
  end

  test "all returns config map" do
    config = Config.all()
    assert is_map(config)
    assert Map.has_key?(config, :sync_interval_ms)
    assert Map.has_key?(config, :data_dir)
  end

  test "data_dir returns expanded path" do
    path = Config.data_dir()
    refute String.starts_with?(path, "~")
    assert String.contains?(path, "lattice")
  end

  test "node_id generates UUID if not set" do
    id = Config.node_id()
    assert is_binary(id)
    # UUID format
    assert String.length(id) == 36
  end
end
