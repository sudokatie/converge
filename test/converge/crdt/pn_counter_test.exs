defmodule Converge.CRDT.PNCounterTest do
  use ExUnit.Case, async: true

  alias Converge.CRDT.PNCounter

  test "new counter has value 0" do
    counter = PNCounter.new("node1")
    assert PNCounter.value(counter) == 0
  end

  test "inc increases value" do
    counter = PNCounter.new("node1") |> PNCounter.inc(5)
    assert PNCounter.value(counter) == 5
  end

  test "dec decreases value" do
    counter = PNCounter.new("node1") |> PNCounter.inc(5) |> PNCounter.dec(2)
    assert PNCounter.value(counter) == 3
  end

  test "negative values work" do
    counter = PNCounter.new("node1") |> PNCounter.dec(3)
    assert PNCounter.value(counter) == -3
  end

  test "concurrent inc/dec converges" do
    a = PNCounter.new("node1") |> PNCounter.inc(5)
    b = PNCounter.new("node2") |> PNCounter.dec(2)

    merged = PNCounter.merge(a, b)
    assert PNCounter.value(merged) == 3
  end

  test "merge is commutative" do
    a = PNCounter.new("node1") |> PNCounter.inc(5)
    b = PNCounter.new("node2") |> PNCounter.dec(2)

    ab = PNCounter.merge(a, b)
    ba = PNCounter.merge(b, a)

    assert PNCounter.value(ab) == PNCounter.value(ba)
  end

  test "serialization roundtrip" do
    counter = PNCounter.new("node1") |> PNCounter.inc(10) |> PNCounter.dec(3)
    binary = PNCounter.to_binary(counter)
    restored = PNCounter.from_binary(binary)

    assert PNCounter.value(restored) == 7
  end
end
