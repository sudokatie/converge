defmodule Converge.CRDT.ORSet do
  @moduledoc """
  Observed-Remove Set CRDT.

  An OR-Set allows both add and remove operations. Each add creates a
  unique tag. Remove only removes tags that have been observed. This
  means add wins over concurrent remove of the same element.

  ## Example

      set = ORSet.new("node1")
      set = ORSet.add(set, "apple")
      set = ORSet.add(set, "banana")
      ORSet.members(set)  # => ["apple", "banana"]

      set = ORSet.remove(set, "apple")
      ORSet.members(set)  # => ["banana"]
  """

  @type tag :: {String.t(), integer()}
  @type t :: %__MODULE__{
          node_id: String.t(),
          elements: %{any() => MapSet.t(tag)}
        }

  defstruct node_id: nil, elements: %{}

  @doc """
  Create a new empty OR-Set.
  """
  def new(node_id \\ nil) do
    %__MODULE__{
      node_id: node_id || Converge.Config.node_id(),
      elements: %{}
    }
  end

  @doc """
  Add an element to the set.

  Creates a unique tag for this add operation.
  """
  def add(%__MODULE__{node_id: node_id, elements: elements} = set, element) do
    tag = {node_id, System.unique_integer([:monotonic, :positive])}
    current_tags = Map.get(elements, element, MapSet.new())
    new_tags = MapSet.put(current_tags, tag)
    %{set | elements: Map.put(elements, element, new_tags)}
  end

  @doc """
  Remove an element from the set.

  Removes all observed tags for this element. If another node concurrently
  adds the element with a new tag, the element will remain in the set.
  """
  def remove(%__MODULE__{elements: elements} = set, element) do
    %{set | elements: Map.delete(elements, element)}
  end

  @doc """
  Check if an element is in the set.
  """
  def contains?(%__MODULE__{elements: elements}, element) do
    case Map.get(elements, element) do
      nil -> false
      tags -> MapSet.size(tags) > 0
    end
  end

  @doc """
  Get all elements in the set.
  """
  def members(%__MODULE__{elements: elements}) do
    elements
    |> Enum.filter(fn {_elem, tags} -> MapSet.size(tags) > 0 end)
    |> Enum.map(fn {elem, _tags} -> elem end)
  end

  @doc """
  Get the number of elements in the set.
  """
  def size(%__MODULE__{} = set) do
    length(members(set))
  end

  @doc """
  Merge two OR-Sets.

  Takes the union of tags for each element.
  """
  def merge(%__MODULE__{} = a, %__MODULE__{} = b) do
    merged_elements =
      Map.merge(a.elements, b.elements, fn _elem, tags_a, tags_b ->
        MapSet.union(tags_a, tags_b)
      end)

    %{a | elements: merged_elements}
  end

  @doc """
  Serialize to binary.
  """
  def to_binary(%__MODULE__{node_id: node_id, elements: elements}) do
    # Convert MapSets to lists for serialization
    serializable =
      Enum.map(elements, fn {elem, tags} ->
        {elem, MapSet.to_list(tags)}
      end)
      |> Map.new()

    :erlang.term_to_binary({node_id, serializable})
  end

  @doc """
  Deserialize from binary.
  """
  def from_binary(binary) when is_binary(binary) do
    {node_id, serialized} = :erlang.binary_to_term(binary)

    elements =
      Enum.map(serialized, fn {elem, tags_list} ->
        {elem, MapSet.new(tags_list)}
      end)
      |> Map.new()

    %__MODULE__{node_id: node_id, elements: elements}
  end
end
