defmodule Lattice.CRDT.LWWRegister do
  @moduledoc """
  Last-Writer-Wins Register CRDT.

  A register holds a single value. When merging, the value with the
  highest timestamp wins. Ties are broken by comparing node IDs.

  ## Example

      reg = LWWRegister.new("node1")
      reg = LWWRegister.set(reg, "hello")
      LWWRegister.get(reg)  # => "hello"
  """

  @type t :: %__MODULE__{
          value: any(),
          timestamp: integer(),
          node_id: String.t()
        }

  defstruct value: nil, timestamp: 0, node_id: nil

  @doc """
  Create a new empty register.
  """
  def new(node_id \\ nil) do
    %__MODULE__{
      value: nil,
      timestamp: 0,
      node_id: node_id || Lattice.Config.node_id()
    }
  end

  @doc """
  Create a register with an initial value.
  """
  def new(node_id, value) do
    %__MODULE__{
      value: value,
      timestamp: System.os_time(:nanosecond),
      node_id: node_id || Lattice.Config.node_id()
    }
  end

  @doc """
  Set the register value (auto-timestamps).
  """
  def set(%__MODULE__{node_id: node_id} = reg, value) do
    %{reg | value: value, timestamp: System.os_time(:nanosecond), node_id: node_id}
  end

  @doc """
  Set the register value with explicit timestamp.
  """
  def set(%__MODULE__{} = reg, value, timestamp, node_id) do
    %{reg | value: value, timestamp: timestamp, node_id: node_id}
  end

  @doc """
  Get the current value.
  """
  def get(%__MODULE__{value: value}) do
    value
  end

  @doc """
  Merge two registers (highest timestamp wins).

  Ties are broken by comparing node IDs lexicographically.
  """
  def merge(%__MODULE__{} = a, %__MODULE__{} = b) do
    cond do
      a.timestamp > b.timestamp -> a
      b.timestamp > a.timestamp -> b
      a.node_id >= b.node_id -> a
      true -> b
    end
  end

  @doc """
  Serialize to binary.
  """
  def to_binary(%__MODULE__{value: value, timestamp: ts, node_id: node_id}) do
    :erlang.term_to_binary({value, ts, node_id})
  end

  @doc """
  Deserialize from binary.
  """
  def from_binary(binary) when is_binary(binary) do
    {value, timestamp, node_id} = :erlang.binary_to_term(binary)
    %__MODULE__{value: value, timestamp: timestamp, node_id: node_id}
  end
end
