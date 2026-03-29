defmodule LatticeTest do
  use ExUnit.Case

  alias Lattice.Storage.Store
  alias Lattice.Cluster.Node

  setup do
    tmp_dir = System.tmp_dir!() |> Path.join("lattice_api_test_#{:rand.uniform(100_000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _} = Store.start_link(data_dir: tmp_dir)
    {:ok, _} = Node.start_link(node_id: "test-node", data_dir: tmp_dir)

    on_exit(fn ->
      pid = Process.whereis(Store)
      if pid && Process.alive?(pid), do: GenServer.stop(Store)

      pid = Process.whereis(Node)
      if pid && Process.alive?(pid), do: GenServer.stop(Node)

      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  describe "counter operations" do
    test "counter_inc increments by 1" do
      Lattice.counter_inc("test", "counter1")
      assert Lattice.counter_value("test", "counter1") == 1

      Lattice.counter_inc("test", "counter1")
      assert Lattice.counter_value("test", "counter1") == 2
    end

    test "counter_inc with amount" do
      Lattice.counter_inc("test", "counter1", 5)
      assert Lattice.counter_value("test", "counter1") == 5

      Lattice.counter_inc("test", "counter1", 10)
      assert Lattice.counter_value("test", "counter1") == 15
    end

    test "counter_dec decrements" do
      Lattice.counter_inc("test", "counter1", 10)
      Lattice.counter_dec("test", "counter1")
      assert Lattice.counter_value("test", "counter1") == 9

      Lattice.counter_dec("test", "counter1", 5)
      assert Lattice.counter_value("test", "counter1") == 4
    end

    test "counter_value returns 0 for missing key" do
      assert Lattice.counter_value("test", "nonexistent") == 0
    end
  end

  describe "register operations" do
    test "register_set and register_get" do
      Lattice.register_set("test", "config", "value1")
      assert Lattice.register_get("test", "config") == "value1"

      Lattice.register_set("test", "config", "value2")
      assert Lattice.register_get("test", "config") == "value2"
    end

    test "register_get returns nil for missing key" do
      assert Lattice.register_get("test", "nonexistent") == nil
    end

    test "register supports complex values" do
      Lattice.register_set("test", "data", %{name: "Alice", age: 30})
      result = Lattice.register_get("test", "data")
      assert result == %{name: "Alice", age: 30}
    end
  end

  describe "set operations" do
    test "set_add and set_members" do
      Lattice.set_add("test", "tags", "elixir")
      Lattice.set_add("test", "tags", "erlang")
      Lattice.set_add("test", "tags", "otp")

      members = Lattice.set_members("test", "tags")
      assert length(members) == 3
      assert "elixir" in members
      assert "erlang" in members
      assert "otp" in members
    end

    test "set_remove removes element" do
      Lattice.set_add("test", "tags", "a")
      Lattice.set_add("test", "tags", "b")
      Lattice.set_remove("test", "tags", "a")

      members = Lattice.set_members("test", "tags")
      assert members == ["b"]
    end

    test "set_contains?" do
      Lattice.set_add("test", "tags", "present")

      assert Lattice.set_contains?("test", "tags", "present") == true
      assert Lattice.set_contains?("test", "tags", "absent") == false
    end

    test "set_members returns empty for missing key" do
      assert Lattice.set_members("test", "nonexistent") == []
    end
  end

  describe "map operations" do
    test "map_put and map_get" do
      Lattice.map_put("test", "user:1", "name", "Alice")
      Lattice.map_put("test", "user:1", "email", "alice@example.com")

      assert Lattice.map_get("test", "user:1", "name") == "Alice"
      assert Lattice.map_get("test", "user:1", "email") == "alice@example.com"
    end

    test "map_delete removes field" do
      Lattice.map_put("test", "user:1", "name", "Alice")
      Lattice.map_put("test", "user:1", "temp", "value")
      Lattice.map_delete("test", "user:1", "temp")

      assert Lattice.map_get("test", "user:1", "name") == "Alice"
      assert Lattice.map_get("test", "user:1", "temp") == nil
    end

    test "map_keys returns all keys" do
      Lattice.map_put("test", "user:1", "a", 1)
      Lattice.map_put("test", "user:1", "b", 2)
      Lattice.map_put("test", "user:1", "c", 3)

      keys = Lattice.map_keys("test", "user:1") |> Enum.sort()
      assert keys == ["a", "b", "c"]
    end

    test "map_get returns nil for missing field" do
      Lattice.map_put("test", "user:1", "name", "Alice")
      assert Lattice.map_get("test", "user:1", "nonexistent") == nil
    end
  end

  describe "namespace management" do
    test "create and list namespaces" do
      Lattice.create_namespace("ns1")
      Lattice.create_namespace("ns2")

      namespaces = Lattice.list_namespaces()
      assert "ns1" in namespaces
      assert "ns2" in namespaces
    end

    test "delete_namespace removes namespace" do
      Lattice.create_namespace("todelete")
      Lattice.counter_inc("todelete", "counter")

      Lattice.delete_namespace("todelete")

      # After deletion, counter should be gone
      assert Lattice.counter_value("todelete", "counter") == 0
    end
  end

  describe "namespace isolation" do
    test "same key in different namespaces are independent" do
      Lattice.counter_inc("ns1", "counter", 10)
      Lattice.counter_inc("ns2", "counter", 20)

      assert Lattice.counter_value("ns1", "counter") == 10
      assert Lattice.counter_value("ns2", "counter") == 20
    end
  end

  describe "cluster_status" do
    test "returns node info" do
      status = Lattice.cluster_status()

      assert status.node.id == "test-node"
      assert is_map(status.node.metadata)
      assert is_list(status.members)
      assert is_list(status.namespaces)
    end
  end

  describe "sync operations" do
    test "sync_now does not crash without anti-entropy running" do
      assert :ok = Lattice.sync_now()
      assert :ok = Lattice.sync_now("test")
    end
  end
end
