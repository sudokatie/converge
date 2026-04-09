defmodule Converge.CRDT.GCounterTest do
  use ExUnit.Case, async: true

  alias Converge.CRDT.GCounter

  test "new counter has value 0" do
    counter = GCounter.new("node1")
    assert GCounter.value(counter) == 0
  end

  test "inc increases value by 1" do
    counter = GCounter.new("node1") |> GCounter.inc()
    assert GCounter.value(counter) == 1
  end

  test "inc with amount increases by that amount" do
    counter = GCounter.new("node1") |> GCounter.inc(5)
    assert GCounter.value(counter) == 5
  end

  test "multiple incs accumulate" do
    counter =
      GCounter.new("node1")
      |> GCounter.inc()
      |> GCounter.inc(3)
      |> GCounter.inc(2)

    assert GCounter.value(counter) == 6
  end

  test "merge takes max per node" do
    a = GCounter.new("node1") |> GCounter.inc(5)
    b = %{GCounter.new("node1") | counts: %{"node1" => 3, "node2" => 4}}

    merged = GCounter.merge(a, b)
    # max(5,3) + 4
    assert GCounter.value(merged) == 9
  end

  test "merge with concurrent updates converges" do
    # Node1 sees 5, Node2 sees 3
    a = %{GCounter.new("node1") | counts: %{"node1" => 5}}
    b = %{GCounter.new("node2") | counts: %{"node2" => 3}}

    # Merge in either order gives same result
    ab = GCounter.merge(a, b)
    ba = GCounter.merge(b, a)

    assert GCounter.value(ab) == 8
    assert GCounter.value(ba) == 8
  end

  test "serialization roundtrip" do
    counter = GCounter.new("node1") |> GCounter.inc(42)
    binary = GCounter.to_binary(counter)
    restored = GCounter.from_binary(binary)

    assert GCounter.value(restored) == 42
    assert restored.node_id == "node1"
  end
end
