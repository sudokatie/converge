defmodule Converge.Sync.Merkle do
  @moduledoc """
  Merkle tree for efficient sync diff detection.

  Binary tree structure with SHA-256 hashing. Each leaf is a key-hash pair,
  internal nodes hash their children. Root hash changes when any leaf changes,
  enabling O(log n) diff detection.
  """

  @type key :: String.t()
  @type hash :: binary()
  @type t :: %__MODULE__{
          leaves: %{key => hash},
          root: hash | nil,
          dirty: boolean()
        }

  defstruct leaves: %{}, root: nil, dirty: false

  @doc """
  Creates an empty Merkle tree.
  """
  @spec new() :: t()
  def new do
    %__MODULE__{leaves: %{}, root: empty_hash(), dirty: false}
  end

  @doc """
  Inserts a key with its value hash.
  """
  @spec insert(t(), key(), binary()) :: t()
  def insert(%__MODULE__{} = tree, key, value) do
    hash = compute_hash(value)
    leaves = Map.put(tree.leaves, key, hash)
    %{tree | leaves: leaves, dirty: true}
  end

  @doc """
  Removes a key from the tree.
  """
  @spec remove(t(), key()) :: t()
  def remove(%__MODULE__{} = tree, key) do
    leaves = Map.delete(tree.leaves, key)
    %{tree | leaves: leaves, dirty: true}
  end

  @doc """
  Computes the root hash of the tree.

  Caches result until tree is modified.
  """
  @spec root_hash(t()) :: {hash(), t()}
  def root_hash(%__MODULE__{dirty: false, root: root} = tree) do
    {root, tree}
  end

  def root_hash(%__MODULE__{leaves: leaves} = tree) do
    root = compute_root(leaves)
    {root, %{tree | root: root, dirty: false}}
  end

  @doc """
  Finds keys that differ between two trees.

  Returns {only_in_a, only_in_b, different_hash}.
  """
  @spec diff(t(), t()) :: {[key()], [key()], [key()]}
  def diff(%__MODULE__{leaves: leaves_a}, %__MODULE__{leaves: leaves_b}) do
    keys_a = Map.keys(leaves_a) |> MapSet.new()
    keys_b = Map.keys(leaves_b) |> MapSet.new()

    only_in_a = MapSet.difference(keys_a, keys_b) |> MapSet.to_list()
    only_in_b = MapSet.difference(keys_b, keys_a) |> MapSet.to_list()

    common_keys = MapSet.intersection(keys_a, keys_b)

    different =
      common_keys
      |> Enum.filter(fn key ->
        Map.get(leaves_a, key) != Map.get(leaves_b, key)
      end)

    {Enum.sort(only_in_a), Enum.sort(only_in_b), Enum.sort(different)}
  end

  @doc """
  Returns all keys in the tree.
  """
  @spec keys(t()) :: [key()]
  def keys(%__MODULE__{leaves: leaves}) do
    Map.keys(leaves) |> Enum.sort()
  end

  @doc """
  Returns the hash for a specific key, or nil if not found.
  """
  @spec get_hash(t(), key()) :: hash() | nil
  def get_hash(%__MODULE__{leaves: leaves}, key) do
    Map.get(leaves, key)
  end

  @doc """
  Returns the number of entries in the tree.
  """
  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{leaves: leaves}) do
    map_size(leaves)
  end

  # Private functions

  defp empty_hash do
    :crypto.hash(:sha256, "")
  end

  defp compute_hash(value) when is_binary(value) do
    :crypto.hash(:sha256, value)
  end

  defp compute_hash(value) do
    :crypto.hash(:sha256, :erlang.term_to_binary(value))
  end

  defp compute_root(leaves) when map_size(leaves) == 0 do
    empty_hash()
  end

  defp compute_root(leaves) do
    # Sort keys for deterministic ordering
    sorted_pairs =
      leaves
      |> Enum.sort_by(fn {k, _v} -> k end)
      |> Enum.map(fn {k, v} -> {k, v} end)

    # Build tree bottom-up
    leaf_hashes = Enum.map(sorted_pairs, fn {k, v} -> hash_leaf(k, v) end)
    build_tree(leaf_hashes)
  end

  defp hash_leaf(key, value_hash) do
    :crypto.hash(:sha256, key <> value_hash)
  end

  defp build_tree([single]) do
    single
  end

  defp build_tree(hashes) do
    # Pair up hashes and combine
    paired =
      hashes
      |> Enum.chunk_every(2)
      |> Enum.map(fn
        [a, b] -> hash_pair(a, b)
        [a] -> a
      end)

    build_tree(paired)
  end

  defp hash_pair(a, b) do
    :crypto.hash(:sha256, a <> b)
  end
end
