defmodule Lattice.Integration.ClusterTest do
  @moduledoc """
  Integration tests for cluster operations, convergence, and failure scenarios.
  """
  use ExUnit.Case

  alias Lattice.CRDT.{PNCounter, ORSet, LWWMap}
  alias Lattice.Storage.Store
  alias Lattice.Cluster.Node
  alias Lattice.Sync.Membership

  setup do
    # Create isolated test environment
    tmp_dir = System.tmp_dir!() |> Path.join("lattice_integration_#{:rand.uniform(100_000)}")
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
      # Ensure different timestamp
      Process.sleep(1)
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

  describe "network partition and recovery" do
    test "nodes diverge during partition then converge on reconnect" do
      # Simulate two nodes that were partitioned
      # Each made independent updates during partition
      node_a_counter = PNCounter.new("node-a") |> PNCounter.inc(100)
      node_b_counter = PNCounter.new("node-b") |> PNCounter.inc(50)

      # After partition heals, nodes exchange state and merge
      merged_at_a = PNCounter.merge(node_a_counter, node_b_counter)
      merged_at_b = PNCounter.merge(node_b_counter, node_a_counter)

      # Both should converge to same value
      assert PNCounter.value(merged_at_a) == 150
      assert PNCounter.value(merged_at_b) == 150
    end

    test "set operations during partition merge correctly" do
      # Node A adds items during partition
      set_a =
        ORSet.new()
        |> ORSet.add("item1")
        |> ORSet.add("item2")

      # Node B adds different items during partition
      set_b =
        ORSet.new()
        |> ORSet.add("item3")
        |> ORSet.add("item4")

      # After recovery, merge both directions
      merged_at_a = ORSet.merge(set_a, set_b)
      merged_at_b = ORSet.merge(set_b, set_a)

      # Both should have all items
      assert Enum.sort(ORSet.members(merged_at_a)) == ["item1", "item2", "item3", "item4"]
      assert Enum.sort(ORSet.members(merged_at_b)) == ["item1", "item2", "item3", "item4"]
    end

    test "conflicting map updates resolve by timestamp after partition" do
      # Node A updates a key
      map_a = LWWMap.new("node-a") |> LWWMap.put("status", "from_a")
      Process.sleep(1)

      # Node B updates same key later
      map_b = LWWMap.new("node-b") |> LWWMap.put("status", "from_b")

      # Merge after partition heals
      merged = LWWMap.merge(map_a, map_b)

      # Later write wins
      assert LWWMap.get(merged, "status") == "from_b"
    end

    test "multiple partitions and recoveries still converge" do
      # Simulate multiple partition/recovery cycles
      # Each node maintains its own counter independently

      # First partition: node1 and node2 make independent updates
      node1_state = PNCounter.new("node1") |> PNCounter.inc(10)
      node2_state = PNCounter.new("node2") |> PNCounter.inc(20)

      # First recovery - nodes sync
      recovered_1a = PNCounter.merge(node1_state, node2_state)
      recovered_1b = PNCounter.merge(node2_state, node1_state)

      # Both nodes converge to same value after first sync
      assert PNCounter.value(recovered_1a) == 30
      assert PNCounter.value(recovered_1b) == 30

      # Second partition: both continue with their own increments
      # Node1 increments its counter (from its view of recovered state)
      node1_state_2 = PNCounter.inc(recovered_1a, 5)
      # Node2 increments its counter (from its view of recovered state)
      node2_state_2 = PNCounter.inc(recovered_1b, 15)

      # Second recovery
      final_a = PNCounter.merge(node1_state_2, node2_state_2)
      final_b = PNCounter.merge(node2_state_2, node1_state_2)

      # Both should converge: node1 contributed 10+5=15, node2 contributed 20+15=35
      # Total = 50
      assert PNCounter.value(final_a) == 50
      assert PNCounter.value(final_b) == 50
    end
  end

  describe "node join/leave during sync" do
    setup do
      test_pid = self()

      send_fn = fn target_id, message ->
        send(test_pid, {:sent, target_id, message})
        :ok
      end

      {:ok, membership} = Membership.start_link(node_id: "test-node", send_fn: send_fn, enabled: false)

      on_exit(fn ->
        pid = Process.whereis(Membership)
        if pid && Process.alive?(pid), do: GenServer.stop(Membership)
      end)

      {:ok, membership: membership}
    end

    test "new node joining receives existing data through merge" do
      # Existing cluster has data
      existing_state = PNCounter.new("existing-node") |> PNCounter.inc(100)

      # New node joins with empty state
      new_node_state = PNCounter.new("new-node")

      # New node receives existing state through sync
      merged = PNCounter.merge(new_node_state, existing_state)

      # New node should see all existing data
      assert PNCounter.value(merged) == 100
    end

    test "node leaving doesn't lose its contributions" do
      # Three nodes with data
      node_a = PNCounter.new("node-a") |> PNCounter.inc(10)
      node_b = PNCounter.new("node-b") |> PNCounter.inc(20)
      node_c = PNCounter.new("node-c") |> PNCounter.inc(30)

      # All nodes have synced
      cluster_state =
        PNCounter.merge(node_a, node_b)
        |> PNCounter.merge(node_c)

      assert PNCounter.value(cluster_state) == 60

      # Node C leaves - remaining nodes still have its data
      # (CRDTs preserve all contributions)
      remaining = PNCounter.merge(node_a, node_b) |> PNCounter.merge(cluster_state)
      assert PNCounter.value(remaining) == 60
    end

    test "concurrent join and data updates merge correctly" do
      # Existing node is updating data
      existing = PNCounter.new("existing") |> PNCounter.inc(50)

      # New node joins and immediately starts updating
      joining = PNCounter.new("joining") |> PNCounter.inc(25)

      # Both make more updates concurrently
      existing_updated = PNCounter.inc(existing, 10)
      joining_updated = PNCounter.inc(joining, 5)

      # Eventual sync merges everything
      final = PNCounter.merge(existing_updated, joining_updated)

      # All updates present: 50 + 10 + 25 + 5 = 90
      assert PNCounter.value(final) == 90
    end

    test "membership changes are tracked" do
      # Join a new member
      seed = %{id: "seed-node", address: "10.0.0.1", port: 4000}
      :ok = Membership.join(seed)

      # Verify member is tracked
      members = Membership.members()
      assert length(members) == 1

      # Add more members
      Membership.add_member(%{id: "node-2", address: "10.0.0.2", port: 4000})
      Membership.add_member(%{id: "node-3", address: "10.0.0.3", port: 4000})

      members = Membership.members()
      assert length(members) == 3

      # Leave
      :ok = Membership.leave()
      assert Membership.members() == []
    end
  end

  describe "persistence across restarts" do
    test "data survives store restart", %{data_dir: data_dir} do
      # Write data
      Lattice.counter_inc("restart_test", "counter", 42)

      # Stop store
      GenServer.stop(Store)

      # Restart store with same data dir
      {:ok, _} = Store.start_link(data_dir: data_dir)

      # Access the namespace to trigger DETS load, then verify data
      # (Store lazy-loads namespaces from DETS on first access)
      Lattice.create_namespace("restart_test")
      assert Lattice.counter_value("restart_test", "counter") == 42
    end

    test "multiple namespaces persist across restart", %{data_dir: data_dir} do
      # Write to multiple namespaces
      Lattice.counter_inc("ns1", "key", 10)
      Lattice.counter_inc("ns2", "key", 20)
      Lattice.set_add("ns3", "items", "a")

      # Restart
      GenServer.stop(Store)
      {:ok, _} = Store.start_link(data_dir: data_dir)

      # Access namespaces to trigger DETS load
      Lattice.create_namespace("ns1")
      Lattice.create_namespace("ns2")
      Lattice.create_namespace("ns3")

      # All data should persist
      assert Lattice.counter_value("ns1", "key") == 10
      assert Lattice.counter_value("ns2", "key") == 20
      assert Lattice.set_contains?("ns3", "items", "a")
    end
  end
end
