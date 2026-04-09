defmodule Converge.CRDT.RGA do
  @moduledoc """
  Replicated Growable Array (RGA) CRDT for text editing.

  An RGA is a sequence CRDT that supports insert and delete operations.
  Each element has a unique identifier consisting of a timestamp and node ID,
  allowing concurrent insertions to be ordered deterministically.

  ## Example

      rga = RGA.new("node1")
      rga = RGA.insert(rga, 0, "H")
      rga = RGA.insert(rga, 1, "i")
      RGA.value(rga)  # => "Hi"
      rga = RGA.delete(rga, 0)
      RGA.value(rga)  # => "i"
  """

  @type element_id :: {integer(), String.t()}
  @type element :: %{
          id: element_id(),
          value: any(),
          deleted: boolean()
        }
  @type t :: %__MODULE__{
          node_id: String.t(),
          clock: integer(),
          elements: [element()]
        }

  defstruct node_id: nil, clock: 0, elements: []

  @doc """
  Create a new empty RGA.
  """
  def new(node_id \\ nil) do
    id = node_id || Converge.Config.node_id()

    %__MODULE__{
      node_id: id,
      clock: 0,
      elements: []
    }
  end

  @doc """
  Insert a value at the given index.
  Returns the updated RGA.
  """
  def insert(%__MODULE__{} = rga, index, value) do
    new_clock = rga.clock + 1
    element_id = {new_clock, rga.node_id}

    new_element = %{
      id: element_id,
      value: value,
      deleted: false
    }

    # Find insertion position considering only visible elements
    visible_index = visible_to_actual_index(rga.elements, index)
    elements = List.insert_at(rga.elements, visible_index, new_element)

    %{rga | clock: new_clock, elements: elements}
  end

  @doc """
  Insert a value after a specific element ID (for merge operations).
  """
  def insert_after(%__MODULE__{} = rga, after_id, element_id, value) do
    new_element = %{
      id: element_id,
      value: value,
      deleted: false
    }

    elements =
      if after_id == nil do
        # Insert at beginning
        insert_ordered([new_element | rga.elements])
      else
        insert_element_after(rga.elements, after_id, new_element)
      end

    new_clock = max(rga.clock, elem(element_id, 0))
    %{rga | clock: new_clock, elements: elements}
  end

  @doc """
  Delete the element at the given index.
  The element is tombstoned, not removed, to support merging.
  """
  def delete(%__MODULE__{} = rga, index) do
    actual_index = visible_to_actual_index(rga.elements, index)

    if actual_index < length(rga.elements) do
      elements =
        List.update_at(rga.elements, actual_index, fn elem ->
          %{elem | deleted: true}
        end)

      %{rga | elements: elements}
    else
      rga
    end
  end

  @doc """
  Delete an element by its ID (for merge operations).
  """
  def delete_by_id(%__MODULE__{} = rga, element_id) do
    elements =
      Enum.map(rga.elements, fn elem ->
        if elem.id == element_id do
          %{elem | deleted: true}
        else
          elem
        end
      end)

    %{rga | elements: elements}
  end

  @doc """
  Get the current value as a string (for text) or list.
  """
  def value(%__MODULE__{elements: elements}) do
    elements
    |> Enum.reject(& &1.deleted)
    |> Enum.map(& &1.value)
    |> then(fn values ->
      if Enum.all?(values, &is_binary/1) do
        Enum.join(values)
      else
        values
      end
    end)
  end

  @doc """
  Get the current value as a list.
  """
  def to_list(%__MODULE__{elements: elements}) do
    elements
    |> Enum.reject(& &1.deleted)
    |> Enum.map(& &1.value)
  end

  @doc """
  Get the size (visible elements only).
  """
  def size(%__MODULE__{elements: elements}) do
    Enum.count(elements, &(not &1.deleted))
  end

  @doc """
  Merge two RGAs.
  Returns an RGA containing all elements from both, properly ordered.
  """
  def merge(%__MODULE__{} = rga1, %__MODULE__{} = rga2) do
    # Create a map of element IDs to their deletion state from both RGAs
    all_elements = merge_element_lists(rga1.elements, rga2.elements)
    new_clock = max(rga1.clock, rga2.clock)

    %__MODULE__{
      node_id: rga1.node_id,
      clock: new_clock,
      elements: all_elements
    }
  end

  @doc """
  Compare two element IDs for ordering.
  Higher timestamp wins; ties broken by node ID (lexicographic).
  """
  def compare_ids({ts1, node1}, {ts2, node2}) do
    cond do
      ts1 > ts2 -> :gt
      ts1 < ts2 -> :lt
      node1 > node2 -> :gt
      node1 < node2 -> :lt
      true -> :eq
    end
  end

  # Private functions

  defp visible_to_actual_index(elements, visible_index) do
    visible_to_actual_index(elements, visible_index, 0, 0)
  end

  defp visible_to_actual_index([], _visible_target, actual, _visible) do
    actual
  end

  defp visible_to_actual_index(_elements, visible_target, actual, visible)
       when visible == visible_target do
    actual
  end

  defp visible_to_actual_index([elem | rest], visible_target, actual, visible) do
    new_visible = if elem.deleted, do: visible, else: visible + 1
    visible_to_actual_index(rest, visible_target, actual + 1, new_visible)
  end

  defp insert_element_after([], _after_id, new_element) do
    [new_element]
  end

  defp insert_element_after([elem | rest], after_id, new_element) do
    if elem.id == after_id do
      [elem | insert_ordered([new_element | rest])]
    else
      [elem | insert_element_after(rest, after_id, new_element)]
    end
  end

  defp insert_ordered([]) do
    []
  end

  defp insert_ordered([single]) do
    [single]
  end

  defp insert_ordered([a, b | rest]) do
    case compare_ids(a.id, b.id) do
      :gt -> [a | insert_ordered([b | rest])]
      _ -> [b | insert_ordered([a | rest])]
    end
  end

  defp merge_element_lists(list1, list2) do
    # Build a map of all unique elements, preferring deleted state if either is deleted
    map1 = Map.new(list1, fn elem -> {elem.id, elem} end)
    map2 = Map.new(list2, fn elem -> {elem.id, elem} end)

    all_ids = MapSet.union(MapSet.new(Map.keys(map1)), MapSet.new(Map.keys(map2)))

    all_ids
    |> Enum.map(fn id ->
      elem1 = Map.get(map1, id)
      elem2 = Map.get(map2, id)

      case {elem1, elem2} do
        {nil, elem} -> elem
        {elem, nil} -> elem
        {e1, e2} -> %{e1 | deleted: e1.deleted or e2.deleted}
      end
    end)
    |> Enum.sort_by(& &1.id, fn {ts1, node1}, {ts2, node2} ->
      # Sort by timestamp descending, then node_id ascending for stable order
      if ts1 != ts2, do: ts1 > ts2, else: node1 <= node2
    end)
    |> Enum.reverse()
  end
end
