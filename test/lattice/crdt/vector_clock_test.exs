defmodule Lattice.CRDT.VectorClockTest do
  use ExUnit.Case, async: true

  alias Lattice.CRDT.VectorClock

  test "new clock has all zeros" do
    clock = VectorClock.new()
    assert VectorClock.get(clock, "node1") == 0
    assert VectorClock.get(clock, "node2") == 0
  end

  test "increment advances time for node" do
    clock =
      VectorClock.new()
      |> VectorClock.increment("node1")
      |> VectorClock.increment("node1")

    assert VectorClock.get(clock, "node1") == 2
    assert VectorClock.get(clock, "node2") == 0
  end

  test "merge takes component-wise max" do
    a =
      VectorClock.new()
      |> VectorClock.increment("node1")
      |> VectorClock.increment("node1")

    b =
      VectorClock.new()
      |> VectorClock.increment("node1")
      |> VectorClock.increment("node2")
      |> VectorClock.increment("node2")

    merged = VectorClock.merge(a, b)
    assert VectorClock.get(merged, "node1") == 2
    assert VectorClock.get(merged, "node2") == 2
  end

  test "compare equal clocks" do
    a = VectorClock.new() |> VectorClock.increment("node1")
    b = VectorClock.new() |> VectorClock.increment("node1")

    assert VectorClock.compare(a, b) == :equal
  end

  test "compare causally ordered events" do
    a = VectorClock.new() |> VectorClock.increment("node1")
    b = a |> VectorClock.increment("node1")

    assert VectorClock.compare(a, b) == :before
    assert VectorClock.compare(b, a) == :after
  end

  test "compare concurrent events" do
    a = VectorClock.new() |> VectorClock.increment("node1")
    b = VectorClock.new() |> VectorClock.increment("node2")

    assert VectorClock.compare(a, b) == :concurrent
    assert VectorClock.compare(b, a) == :concurrent
  end

  test "serialization roundtrip" do
    clock =
      VectorClock.new()
      |> VectorClock.increment("node1")
      |> VectorClock.increment("node2")

    binary = VectorClock.to_binary(clock)
    restored = VectorClock.from_binary(binary)

    assert VectorClock.get(restored, "node1") == 1
    assert VectorClock.get(restored, "node2") == 1
  end
end
