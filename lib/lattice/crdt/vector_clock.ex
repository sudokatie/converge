defmodule Lattice.CRDT.VectorClock do
  @moduledoc """
  Vector clock for causality tracking.

  A vector clock tracks logical time across multiple nodes, allowing
  us to determine if events happened before, after, or concurrently.

  ## Example

      clock = VectorClock.new()
      clock = VectorClock.increment(clock, "node1")
      clock = VectorClock.increment(clock, "node1")
      clock = VectorClock.increment(clock, "node2")

      VectorClock.get(clock, "node1")  # => 2
  """

  @type t :: %__MODULE__{
          clocks: %{String.t() => non_neg_integer()}
        }

  defstruct clocks: %{}

  @doc """
  Create a new vector clock.
  """
  def new do
    %__MODULE__{clocks: %{}}
  end

  @doc """
  Increment the logical time for a node.
  """
  def increment(%__MODULE__{clocks: clocks} = vc, node_id) do
    current = Map.get(clocks, node_id, 0)
    %{vc | clocks: Map.put(clocks, node_id, current + 1)}
  end

  @doc """
  Get the logical time for a node.
  """
  def get(%__MODULE__{clocks: clocks}, node_id) do
    Map.get(clocks, node_id, 0)
  end

  @doc """
  Merge two vector clocks (point-wise maximum).
  """
  def merge(%__MODULE__{} = a, %__MODULE__{} = b) do
    merged =
      Map.merge(a.clocks, b.clocks, fn _node, time_a, time_b ->
        max(time_a, time_b)
      end)

    %__MODULE__{clocks: merged}
  end

  @doc """
  Compare two vector clocks.

  Returns:
  - :equal if they are identical
  - :before if a happened before b
  - :after if a happened after b
  - :concurrent if neither dominates
  """
  def compare(%__MODULE__{} = a, %__MODULE__{} = b) do
    cond do
      a.clocks == b.clocks -> :equal
      dominates?(a, b) -> :after
      dominates?(b, a) -> :before
      true -> :concurrent
    end
  end

  @doc """
  Check if a dominates (happened after) b.

  A dominates B if:
  - For all nodes in B, A[node] >= B[node]
  - For at least one node, A[node] > B[node]
  """
  def dominates?(%__MODULE__{} = a, %__MODULE__{} = b) do
    all_nodes =
      MapSet.union(
        MapSet.new(Map.keys(a.clocks)),
        MapSet.new(Map.keys(b.clocks))
      )

    has_greater =
      Enum.any?(all_nodes, fn node ->
        get(a, node) > get(b, node)
      end)

    all_gte =
      Enum.all?(all_nodes, fn node ->
        get(a, node) >= get(b, node)
      end)

    has_greater and all_gte
  end

  @doc """
  Serialize to binary.
  """
  def to_binary(%__MODULE__{clocks: clocks}) do
    :erlang.term_to_binary(clocks)
  end

  @doc """
  Deserialize from binary.
  """
  def from_binary(binary) when is_binary(binary) do
    clocks = :erlang.binary_to_term(binary)
    %__MODULE__{clocks: clocks}
  end
end
