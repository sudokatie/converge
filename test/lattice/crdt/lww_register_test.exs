defmodule Lattice.CRDT.LWWRegisterTest do
  use ExUnit.Case, async: true

  alias Lattice.CRDT.LWWRegister

  test "new register has nil value" do
    reg = LWWRegister.new("node1")
    assert LWWRegister.get(reg) == nil
  end

  test "set then get returns value" do
    reg = LWWRegister.new("node1") |> LWWRegister.set("hello")
    assert LWWRegister.get(reg) == "hello"
  end

  test "set updates value" do
    reg = LWWRegister.new("node1")
    |> LWWRegister.set("first")
    |> LWWRegister.set("second")

    assert LWWRegister.get(reg) == "second"
  end

  test "higher timestamp wins on merge" do
    a = %LWWRegister{value: "old", timestamp: 100, node_id: "node1"}
    b = %LWWRegister{value: "new", timestamp: 200, node_id: "node2"}

    merged = LWWRegister.merge(a, b)
    assert LWWRegister.get(merged) == "new"
  end

  test "tie-breaking uses node_id" do
    a = %LWWRegister{value: "from_a", timestamp: 100, node_id: "node_a"}
    b = %LWWRegister{value: "from_b", timestamp: 100, node_id: "node_b"}

    merged = LWWRegister.merge(a, b)
    # node_b > node_a lexicographically, so b wins? No, a.node_id >= b.node_id is checked
    # "node_b" > "node_a", so b.node_id > a.node_id, meaning a.node_id >= b.node_id is false
    # So b wins
    assert LWWRegister.get(merged) == "from_b"
  end

  test "nil is a valid value" do
    reg = LWWRegister.new("node1") |> LWWRegister.set(nil)
    assert LWWRegister.get(reg) == nil
    assert reg.timestamp > 0
  end

  test "serialization roundtrip" do
    reg = LWWRegister.new("node1") |> LWWRegister.set("test value")
    binary = LWWRegister.to_binary(reg)
    restored = LWWRegister.from_binary(binary)

    assert LWWRegister.get(restored) == "test value"
  end
end
