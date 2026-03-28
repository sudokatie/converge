defmodule Lattice.CRDT.ORSetTest do
  use ExUnit.Case, async: true

  alias Lattice.CRDT.ORSet

  test "new set is empty" do
    set = ORSet.new("node1")
    assert ORSet.members(set) == []
    assert ORSet.size(set) == 0
  end

  test "add makes element present" do
    set = ORSet.new("node1") |> ORSet.add("apple")
    assert ORSet.contains?(set, "apple")
    assert "apple" in ORSet.members(set)
  end

  test "remove makes element absent" do
    set = ORSet.new("node1")
    |> ORSet.add("apple")
    |> ORSet.remove("apple")

    refute ORSet.contains?(set, "apple")
    assert ORSet.members(set) == []
  end

  test "add after remove re-adds element" do
    set = ORSet.new("node1")
    |> ORSet.add("apple")
    |> ORSet.remove("apple")
    |> ORSet.add("apple")

    assert ORSet.contains?(set, "apple")
  end

  test "add wins over concurrent remove" do
    # Node1 adds apple, Node2 removes apple concurrently
    base = ORSet.new("node1") |> ORSet.add("apple")

    # Node1 adds again (different tag)
    a = ORSet.add(base, "apple")

    # Node2 removes (only sees original tag)
    b = ORSet.remove(base, "apple")

    # Merge: the new tag from a survives
    merged = ORSet.merge(a, b)
    assert ORSet.contains?(merged, "apple")
  end

  test "multiple elements" do
    set = ORSet.new("node1")
    |> ORSet.add("apple")
    |> ORSet.add("banana")
    |> ORSet.add("cherry")

    members = ORSet.members(set) |> Enum.sort()
    assert members == ["apple", "banana", "cherry"]
    assert ORSet.size(set) == 3
  end

  test "merge combines elements from both sets" do
    a = ORSet.new("node1") |> ORSet.add("apple")
    b = ORSet.new("node2") |> ORSet.add("banana")

    merged = ORSet.merge(a, b)
    members = ORSet.members(merged) |> Enum.sort()
    assert members == ["apple", "banana"]
  end

  test "serialization roundtrip" do
    set = ORSet.new("node1")
    |> ORSet.add("apple")
    |> ORSet.add("banana")

    binary = ORSet.to_binary(set)
    restored = ORSet.from_binary(binary)

    assert ORSet.contains?(restored, "apple")
    assert ORSet.contains?(restored, "banana")
    assert ORSet.size(restored) == 2
  end
end
