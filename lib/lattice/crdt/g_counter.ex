defmodule Lattice.CRDT.GCounter do
  @moduledoc """
  Grow-only counter CRDT.

  A G-Counter can only be incremented. Each node maintains its own count,
  and the total value is the sum of all node counts. Merging takes the
  maximum value for each node.

  ## Example

      counter = GCounter.new("node1")
      counter = GCounter.inc(counter)
      counter = GCounter.inc(counter, 5)
      GCounter.value(counter)  # => 6
  """

  @type t :: %__MODULE__{
    node_id: String.t(),
    counts: %{String.t() => non_neg_integer()}
  }

  defstruct node_id: nil, counts: %{}

  @doc """
  Create a new G-Counter.
  """
  def new(node_id \\ nil) do
    %__MODULE__{
      node_id: node_id || Lattice.Config.node_id(),
      counts: %{}
    }
  end

  @doc """
  Increment the counter by 1.
  """
  def inc(%__MODULE__{} = counter) do
    inc(counter, 1)
  end

  @doc """
  Increment the counter by the given amount.
  """
  def inc(%__MODULE__{node_id: node_id, counts: counts} = counter, amount)
      when is_integer(amount) and amount > 0 do
    current = Map.get(counts, node_id, 0)
    %{counter | counts: Map.put(counts, node_id, current + amount)}
  end

  @doc """
  Get the current value (sum of all node counts).
  """
  def value(%__MODULE__{counts: counts}) do
    counts |> Map.values() |> Enum.sum()
  end

  @doc """
  Merge two G-Counters.

  Takes the maximum count for each node.
  """
  def merge(%__MODULE__{} = a, %__MODULE__{} = b) do
    merged_counts =
      Map.merge(a.counts, b.counts, fn _node, count_a, count_b ->
        max(count_a, count_b)
      end)

    %{a | counts: merged_counts}
  end

  @doc """
  Serialize to binary.
  """
  def to_binary(%__MODULE__{} = counter) do
    :erlang.term_to_binary({counter.node_id, counter.counts})
  end

  @doc """
  Deserialize from binary.
  """
  def from_binary(binary) when is_binary(binary) do
    {node_id, counts} = :erlang.binary_to_term(binary)
    %__MODULE__{node_id: node_id, counts: counts}
  end
end
