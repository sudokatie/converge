defmodule Converge.Storage.StoreTest do
  use ExUnit.Case

  alias Converge.Storage.Store
  alias Converge.CRDT.GCounter

  setup do
    # Use a temp directory for tests
    tmp_dir = System.tmp_dir!() |> Path.join("converge_test_#{:rand.uniform(100_000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _pid} = Store.start_link(data_dir: tmp_dir)

    on_exit(fn ->
      # Clean up
      pid = Process.whereis(Store)
      if pid && Process.alive?(pid), do: GenServer.stop(Store)
      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  test "get returns nil for missing key" do
    assert Store.get("test", "missing") == nil
  end

  test "put then get returns value" do
    counter = GCounter.new("node1") |> GCounter.inc(5)
    Store.put("test", "counter1", counter)

    result = Store.get("test", "counter1")
    assert GCounter.value(result) == 5
  end

  test "delete removes key" do
    counter = GCounter.new("node1")
    Store.put("test", "counter1", counter)
    Store.delete("test", "counter1")

    assert Store.get("test", "counter1") == nil
  end

  test "list_keys returns all keys in namespace" do
    Store.put("myns", "a", :value_a)
    Store.put("myns", "b", :value_b)
    Store.put("other", "c", :value_c)

    keys = Store.list_keys("myns") |> Enum.sort()
    assert keys == ["a", "b"]
  end

  test "namespaces are isolated" do
    Store.put("ns1", "key", :value1)
    Store.put("ns2", "key", :value2)

    assert Store.get("ns1", "key") == :value1
    assert Store.get("ns2", "key") == :value2
  end

  test "create_namespace creates empty namespace" do
    Store.create_namespace("newns")
    assert Store.list_keys("newns") == []
  end

  test "delete_namespace removes all data" do
    Store.put("todelete", "a", 1)
    Store.put("todelete", "b", 2)
    Store.delete_namespace("todelete")

    assert Store.list_keys("todelete") == []
  end

  test "list_namespaces returns all namespaces" do
    Store.create_namespace("ns1")
    Store.create_namespace("ns2")

    namespaces = Store.list_namespaces() |> Enum.sort()
    assert "ns1" in namespaces
    assert "ns2" in namespaces
  end
end
