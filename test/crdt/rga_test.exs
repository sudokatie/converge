defmodule Converge.CRDT.RGATest do
  use ExUnit.Case, async: true

  alias Converge.CRDT.RGA

  describe "new/1" do
    test "creates empty RGA" do
      rga = RGA.new("node1")
      assert rga.node_id == "node1"
      assert rga.clock == 0
      assert rga.elements == []
      assert RGA.value(rga) == ""
    end
  end

  describe "insert/3" do
    test "inserts at beginning" do
      rga = RGA.new("node1")
      rga = RGA.insert(rga, 0, "a")
      assert RGA.value(rga) == "a"
    end

    test "inserts multiple characters" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "H")
        |> RGA.insert(1, "i")

      assert RGA.value(rga) == "Hi"
    end

    test "inserts in middle" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "H")
        |> RGA.insert(1, "o")
        |> RGA.insert(1, "i")

      assert RGA.value(rga) == "Hio"
    end

    test "increments clock" do
      rga = RGA.new("node1")
      assert rga.clock == 0

      rga = RGA.insert(rga, 0, "a")
      assert rga.clock == 1

      rga = RGA.insert(rga, 1, "b")
      assert rga.clock == 2
    end
  end

  describe "delete/2" do
    test "deletes element" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "a")
        |> RGA.insert(1, "b")
        |> RGA.insert(2, "c")

      assert RGA.value(rga) == "abc"

      rga = RGA.delete(rga, 1)
      assert RGA.value(rga) == "ac"
    end

    test "deletes first element" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "a")
        |> RGA.insert(1, "b")
        |> RGA.delete(0)

      assert RGA.value(rga) == "b"
    end

    test "deletes last element" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "a")
        |> RGA.insert(1, "b")
        |> RGA.delete(1)

      assert RGA.value(rga) == "a"
    end

    test "preserves tombstones for merge" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "a")
        |> RGA.delete(0)

      # Internal elements list still has the deleted element
      assert length(rga.elements) == 1
      assert Enum.at(rga.elements, 0).deleted == true
    end
  end

  describe "length/1" do
    test "counts visible elements" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, "a")
        |> RGA.insert(1, "b")
        |> RGA.insert(2, "c")

      assert RGA.size(rga) == 3

      rga = RGA.delete(rga, 1)
      assert RGA.size(rga) == 2
    end
  end

  describe "to_list/1" do
    test "returns values as list" do
      rga =
        RGA.new("node1")
        |> RGA.insert(0, 1)
        |> RGA.insert(1, 2)
        |> RGA.insert(2, 3)

      assert RGA.to_list(rga) == [1, 2, 3]
    end
  end

  describe "merge/2" do
    test "merges concurrent inserts" do
      rga1 =
        RGA.new("node1")
        |> RGA.insert(0, "a")

      rga2 =
        RGA.new("node2")
        |> RGA.insert(0, "b")

      merged = RGA.merge(rga1, rga2)

      # Both characters should be present
      value = RGA.value(merged)
      assert String.length(value) == 2
      assert String.contains?(value, "a")
      assert String.contains?(value, "b")
    end

    test "respects deleted state" do
      rga1 =
        RGA.new("node1")
        |> RGA.insert(0, "a")

      # Simulate same element being inserted on both nodes
      element_id = Enum.at(rga1.elements, 0).id

      rga2 = %{rga1 | node_id: "node2"}
      rga2 = RGA.delete_by_id(rga2, element_id)

      merged = RGA.merge(rga1, rga2)

      # Element should be deleted after merge
      assert RGA.value(merged) == ""
    end

    test "merges disjoint sequences" do
      rga1 =
        RGA.new("node1")
        |> RGA.insert(0, "a")
        |> RGA.insert(1, "b")

      rga2 =
        RGA.new("node2")
        |> RGA.insert(0, "x")
        |> RGA.insert(1, "y")

      merged = RGA.merge(rga1, rga2)
      assert RGA.size(merged) == 4
    end

    test "convergence - merging in different order yields same result" do
      base = RGA.new("base")

      rga1 =
        %{base | node_id: "node1"}
        |> RGA.insert(0, "a")

      rga2 =
        %{base | node_id: "node2"}
        |> RGA.insert(0, "b")

      merged1 = RGA.merge(rga1, rga2)
      merged2 = RGA.merge(rga2, rga1)

      # Order should be the same regardless of merge order
      assert RGA.value(merged1) == RGA.value(merged2)
    end
  end

  describe "compare_ids/2" do
    test "higher timestamp wins" do
      assert RGA.compare_ids({2, "a"}, {1, "b"}) == :gt
      assert RGA.compare_ids({1, "a"}, {2, "b"}) == :lt
    end

    test "ties broken by node_id" do
      assert RGA.compare_ids({1, "b"}, {1, "a"}) == :gt
      assert RGA.compare_ids({1, "a"}, {1, "b"}) == :lt
    end

    test "equal ids" do
      assert RGA.compare_ids({1, "a"}, {1, "a"}) == :eq
    end
  end
end
