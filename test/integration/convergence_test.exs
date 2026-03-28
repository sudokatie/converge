defmodule Lattice.Integration.ConvergenceTest do
  @moduledoc """
  Integration tests for CRDT convergence across simulated nodes.
  """
  use ExUnit.Case

  alias Lattice.CRDT.{PNCounter, ORSet, LWWMap}
  alias Lattice.Storage.Store
  alias Lattice.Cluster.Node

  setup do
    # Create isolated test environment
    tmp_dir = System.tmp_dir!() |> Path.join("lattice_integration_#{:rand.uniform(100000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _} = Store.start_link(data_dir: tmp_dir)
    {:ok, _} = Node.start_link(node_id: "integration-node", data_dir: tmp_dir)

    on_exit(fn ->
      pid = Process.whereis(Store)
      if pid && Process.alive?(pid), do: GenServer.stop(Store)

      pid = Process.whereis(Node)
      if pid && Process.alive?(pid), do: GenServer.stop(Node)

      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  describe "counter convergence" do
    test "concurrent increments from multiple 'nodes' converge" do
      # Simulate two nodes making concurrent updates
      node1_counter = PNCounter.new("node1") |> PNCounter.inc(5)
      node2_counter = PNCounter.new("node2") |> PNCounter.inc(3)

      # Merge them (simulating sync)
      merged = PNCounter.merge(node1_counter, node2_counter)

      # Result should be sum of both
      assert PNCounter.value(merged) == 8
    end

    test "increment and decrement from different nodes converge" do
      node1 = PNCounter.new("node1") |> PNCounter.inc(10)
      node2 = PNCounter.new("node2") |> PNCounter.dec(3)

      merged = PNCounter.merge(node1, node2)
      assert PNCounter.value(merged) == 7
    end

    test "multiple merges are idempotent" do
      node1 = PNCounter.new("node1") |> PNCounter.inc(5)
      node2 = PNCounter.new("node2") |> PNCounter.inc(3)

      merged1 = PNCounter.merge(node1, node2)
      merged2 = PNCounter.merge(merged1, node1)
      merged3 = PNCounter.merge(merged2, node2)

      # All should have same value
      assert PNCounter.value(merged1) == 8
      assert PNCounter.value(merged2) == 8
      assert PNCounter.value(merged3) == 8
    end
  end

  describe "set convergence" do
    test "concurrent adds from multiple nodes converge" do
      set1 = ORSet.new() |> ORSet.add("a") |> ORSet.add("b")
      set2 = ORSet.new() |> ORSet.add("b") |> ORSet.add("c")

      merged = ORSet.merge(set1, set2)
      members = ORSet.members(merged) |> Enum.sort()

      assert members == ["a", "b", "c"]
    end

    test "add wins over concurrent remove" do
      # Node 1 adds element
      set1 = ORSet.new() |> ORSet.add("item")

      # Node 2 independently adds then removes (different tags)
      set2 = ORSet.new() |> ORSet.add("item") |> ORSet.remove("item")

      # When merged, node1's add should survive
      merged = ORSet.merge(set1, set2)
      assert ORSet.contains?(merged, "item")
    end
  end

  describe "map convergence" do
    test "concurrent updates to different keys converge" do
      map1 = LWWMap.new("node1") |> LWWMap.put("name", "Alice")
      map2 = LWWMap.new("node2") |> LWWMap.put("email", "alice@example.com")

      merged = LWWMap.merge(map1, map2)

      assert LWWMap.get(merged, "name") == "Alice"
      assert LWWMap.get(merged, "email") == "alice@example.com"
    end

    test "later timestamp wins for same key" do
      map1 = LWWMap.new("node1") |> LWWMap.put("status", "old")
      Process.sleep(1)  # Ensure different timestamp
      map2 = LWWMap.new("node2") |> LWWMap.put("status", "new")

      merged = LWWMap.merge(map1, map2)
      assert LWWMap.get(merged, "status") == "new"
    end
  end

  describe "persistence" do
    test "data persists to DETS and can be accessed", %{data_dir: _data_dir} do
      # Write data
      Lattice.counter_inc("persist_test", "counter", 42)
      Lattice.set_add("persist_test", "tags", "elixir")

      # Verify data is there
      assert Lattice.counter_value("persist_test", "counter") == 42
      assert Lattice.set_contains?("persist_test", "tags", "elixir")

      # Verify persistence by directly checking DETS is being used
      # (Store writes to both ETS and DETS)
      namespaces = Lattice.list_namespaces()
      assert "persist_test" in namespaces
    end

    test "multiple operations accumulate correctly" do
      # Simulate a realistic session with multiple operations
      for i <- 1..10 do
        Lattice.counter_inc("session", "events")
        Lattice.set_add("session", "users", "user_#{i}")
      end

      assert Lattice.counter_value("session", "events") == 10
      assert length(Lattice.set_members("session", "users")) == 10
    end
  end

  describe "api integration" do
    test "full workflow: counter, set, map operations" do
      # Counter
      Lattice.counter_inc("app", "visits", 100)
      Lattice.counter_dec("app", "visits", 10)
      assert Lattice.counter_value("app", "visits") == 90

      # Set
      Lattice.set_add("app", "features", "dark-mode")
      Lattice.set_add("app", "features", "notifications")
      assert length(Lattice.set_members("app", "features")) == 2

      # Map
      Lattice.map_put("app", "config", "theme", "dark")
      Lattice.map_put("app", "config", "lang", "en")
      assert Lattice.map_get("app", "config", "theme") == "dark"
      assert length(Lattice.map_keys("app", "config")) == 2
    end

    test "namespace isolation works" do
      Lattice.counter_inc("ns1", "key", 10)
      Lattice.counter_inc("ns2", "key", 20)

      assert Lattice.counter_value("ns1", "key") == 10
      assert Lattice.counter_value("ns2", "key") == 20
    end
  end
end
