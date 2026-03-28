defmodule Lattice.Sync.MerkleTest do
  use ExUnit.Case, async: true

  alias Lattice.Sync.Merkle

  describe "new/0" do
    test "creates empty tree" do
      tree = Merkle.new()
      assert Merkle.size(tree) == 0
      assert Merkle.keys(tree) == []
    end
  end

  describe "insert/3" do
    test "adds key with hash" do
      tree =
        Merkle.new()
        |> Merkle.insert("key1", "value1")

      assert Merkle.size(tree) == 1
      assert Merkle.keys(tree) == ["key1"]
      assert Merkle.get_hash(tree, "key1") != nil
    end

    test "updates existing key" do
      tree =
        Merkle.new()
        |> Merkle.insert("key1", "value1")

      hash1 = Merkle.get_hash(tree, "key1")

      tree = Merkle.insert(tree, "key1", "value2")
      hash2 = Merkle.get_hash(tree, "key1")

      assert hash1 != hash2
      assert Merkle.size(tree) == 1
    end
  end

  describe "remove/2" do
    test "removes existing key" do
      tree =
        Merkle.new()
        |> Merkle.insert("key1", "value1")
        |> Merkle.insert("key2", "value2")
        |> Merkle.remove("key1")

      assert Merkle.size(tree) == 1
      assert Merkle.keys(tree) == ["key2"]
      assert Merkle.get_hash(tree, "key1") == nil
    end

    test "removing non-existent key is no-op" do
      tree =
        Merkle.new()
        |> Merkle.insert("key1", "value1")
        |> Merkle.remove("nonexistent")

      assert Merkle.size(tree) == 1
    end
  end

  describe "root_hash/1" do
    test "empty trees have same root" do
      tree1 = Merkle.new()
      tree2 = Merkle.new()

      {root1, _} = Merkle.root_hash(tree1)
      {root2, _} = Merkle.root_hash(tree2)

      assert root1 == root2
    end

    test "insert changes root" do
      tree = Merkle.new()
      {root1, tree} = Merkle.root_hash(tree)

      tree = Merkle.insert(tree, "key1", "value1")
      {root2, _tree} = Merkle.root_hash(tree)

      assert root1 != root2
    end

    test "same content produces same root" do
      tree1 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "2")

      tree2 =
        Merkle.new()
        |> Merkle.insert("b", "2")
        |> Merkle.insert("a", "1")

      {root1, _} = Merkle.root_hash(tree1)
      {root2, _} = Merkle.root_hash(tree2)

      assert root1 == root2
    end

    test "caches result until modified" do
      tree =
        Merkle.new()
        |> Merkle.insert("key1", "value1")

      {root1, tree} = Merkle.root_hash(tree)
      {root2, _tree} = Merkle.root_hash(tree)

      # Same result, second call uses cache
      assert root1 == root2
    end
  end

  describe "diff/2" do
    test "identical trees have no diff" do
      tree1 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "2")

      tree2 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "2")

      {only_a, only_b, different} = Merkle.diff(tree1, tree2)

      assert only_a == []
      assert only_b == []
      assert different == []
    end

    test "finds added keys" do
      tree1 =
        Merkle.new()
        |> Merkle.insert("a", "1")

      tree2 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "2")

      {only_a, only_b, different} = Merkle.diff(tree1, tree2)

      assert only_a == []
      assert only_b == ["b"]
      assert different == []
    end

    test "finds changed keys" do
      tree1 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "2")

      tree2 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "changed")

      {only_a, only_b, different} = Merkle.diff(tree1, tree2)

      assert only_a == []
      assert only_b == []
      assert different == ["b"]
    end

    test "diff is symmetric for only_in fields" do
      tree1 =
        Merkle.new()
        |> Merkle.insert("a", "1")
        |> Merkle.insert("b", "2")

      tree2 =
        Merkle.new()
        |> Merkle.insert("b", "2")
        |> Merkle.insert("c", "3")

      {only_a1, only_b1, _} = Merkle.diff(tree1, tree2)
      {only_a2, only_b2, _} = Merkle.diff(tree2, tree1)

      # Swapped results when args swapped
      assert only_a1 == only_b2
      assert only_b1 == only_a2
    end

    test "complex diff scenario" do
      tree1 =
        Merkle.new()
        |> Merkle.insert("shared", "same")
        |> Merkle.insert("changed", "old")
        |> Merkle.insert("only_a", "value")

      tree2 =
        Merkle.new()
        |> Merkle.insert("shared", "same")
        |> Merkle.insert("changed", "new")
        |> Merkle.insert("only_b", "value")

      {only_a, only_b, different} = Merkle.diff(tree1, tree2)

      assert only_a == ["only_a"]
      assert only_b == ["only_b"]
      assert different == ["changed"]
    end
  end
end
