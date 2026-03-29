defmodule Lattice.CRDT.LWWMapTest do
  use ExUnit.Case, async: true

  alias Lattice.CRDT.LWWMap

  test "new map is empty" do
    map = LWWMap.new("node1")
    assert LWWMap.keys(map) == []
    assert LWWMap.size(map) == 0
  end

  test "put then get returns value" do
    map = LWWMap.new("node1") |> LWWMap.put("name", "Alice")
    assert LWWMap.get(map, "name") == "Alice"
  end

  test "put overwrites value" do
    map =
      LWWMap.new("node1")
      |> LWWMap.put("name", "Alice")
      |> LWWMap.put("name", "Bob")

    assert LWWMap.get(map, "name") == "Bob"
  end

  test "delete removes key" do
    map =
      LWWMap.new("node1")
      |> LWWMap.put("name", "Alice")
      |> LWWMap.delete("name")

    assert LWWMap.get(map, "name") == nil
    refute LWWMap.has_key?(map, "name")
  end

  test "re-put after delete works" do
    map =
      LWWMap.new("node1")
      |> LWWMap.put("name", "Alice")
      |> LWWMap.delete("name")
      |> LWWMap.put("name", "Bob")

    assert LWWMap.get(map, "name") == "Bob"
  end

  test "keys returns all keys" do
    map =
      LWWMap.new("node1")
      |> LWWMap.put("a", 1)
      |> LWWMap.put("b", 2)
      |> LWWMap.put("c", 3)

    keys = LWWMap.keys(map) |> Enum.sort()
    assert keys == ["a", "b", "c"]
    assert LWWMap.size(map) == 3
  end

  test "merge combines maps" do
    a = LWWMap.new("node1") |> LWWMap.put("x", 1)
    b = LWWMap.new("node2") |> LWWMap.put("y", 2)

    merged = LWWMap.merge(a, b)
    assert LWWMap.get(merged, "x") == 1
    assert LWWMap.get(merged, "y") == 2
  end

  test "merge takes newer value for same key" do
    # Create maps with controlled timestamps
    a = %LWWMap{
      node_id: "node1",
      entries: %{
        "name" => %Lattice.CRDT.LWWRegister{value: "old", timestamp: 100, node_id: "node1"}
      },
      tombstones: %{}
    }

    b = %LWWMap{
      node_id: "node2",
      entries: %{
        "name" => %Lattice.CRDT.LWWRegister{value: "new", timestamp: 200, node_id: "node2"}
      },
      tombstones: %{}
    }

    merged = LWWMap.merge(a, b)
    assert LWWMap.get(merged, "name") == "new"
  end

  test "serialization roundtrip" do
    map =
      LWWMap.new("node1")
      |> LWWMap.put("name", "Alice")
      |> LWWMap.put("age", 30)

    binary = LWWMap.to_binary(map)
    restored = LWWMap.from_binary(binary)

    assert LWWMap.get(restored, "name") == "Alice"
    assert LWWMap.get(restored, "age") == 30
  end
end
