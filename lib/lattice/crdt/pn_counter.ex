defmodule Lattice.CRDT.PNCounter do
  @moduledoc """
  Positive-Negative counter CRDT.

  A PN-Counter can be incremented and decremented. Internally it uses
  two G-Counters: one for positive (P) and one for negative (N) values.
  The actual value is P - N.

  ## Example

      counter = PNCounter.new("node1")
      counter = PNCounter.inc(counter, 5)
      counter = PNCounter.dec(counter, 2)
      PNCounter.value(counter)  # => 3
  """

  alias Lattice.CRDT.GCounter

  @type t :: %__MODULE__{
          node_id: String.t(),
          p: GCounter.t(),
          n: GCounter.t()
        }

  defstruct node_id: nil, p: nil, n: nil

  @doc """
  Create a new PN-Counter.
  """
  def new(node_id \\ nil) do
    id = node_id || Lattice.Config.node_id()

    %__MODULE__{
      node_id: id,
      p: GCounter.new(id),
      n: GCounter.new(id)
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
  def inc(%__MODULE__{p: p} = counter, amount) when is_integer(amount) and amount > 0 do
    %{counter | p: GCounter.inc(p, amount)}
  end

  @doc """
  Decrement the counter by 1.
  """
  def dec(%__MODULE__{} = counter) do
    dec(counter, 1)
  end

  @doc """
  Decrement the counter by the given amount.
  """
  def dec(%__MODULE__{n: n} = counter, amount) when is_integer(amount) and amount > 0 do
    %{counter | n: GCounter.inc(n, amount)}
  end

  @doc """
  Get the current value (P - N).
  """
  def value(%__MODULE__{p: p, n: n}) do
    GCounter.value(p) - GCounter.value(n)
  end

  @doc """
  Merge two PN-Counters.
  """
  def merge(%__MODULE__{} = a, %__MODULE__{} = b) do
    %{a | p: GCounter.merge(a.p, b.p), n: GCounter.merge(a.n, b.n)}
  end

  @doc """
  Serialize to binary.
  """
  def to_binary(%__MODULE__{node_id: node_id, p: p, n: n}) do
    :erlang.term_to_binary({node_id, GCounter.to_binary(p), GCounter.to_binary(n)})
  end

  @doc """
  Deserialize from binary.
  """
  def from_binary(binary) when is_binary(binary) do
    {node_id, p_bin, n_bin} = :erlang.binary_to_term(binary)

    %__MODULE__{
      node_id: node_id,
      p: GCounter.from_binary(p_bin),
      n: GCounter.from_binary(n_bin)
    }
  end
end
